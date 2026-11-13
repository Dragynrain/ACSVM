// SPDX-License-Identifier: GPL-3.0-or-later

//! The `mem` experiment (docs/capture.md 10.12): from a snapshot just before
//! a roll, optionally with the random state set to the run's index, runs on
//! and logs every change of the watched bytes. The addresses come only from
//! the clean-room researcher (capture.md section 9); the outcome is then
//! checked against the reported rule with `scripts/linefit.py`.

use crate::experiment::{Runner, RANDOM_STATE, TURN_LIMIT};
use serde_json::json;

/// Instructions between looks at the watched bytes.
const LOOK: u64 = 20_000;
/// Instructions run on after the refill with `until_refill`.
const TAIL: u64 = 600_000;

/// Hex addresses, comma-separated (`F5A2,1CC9`).
pub fn parse_addrs(s: &str) -> Result<Vec<u16>, String> {
    s.split(',')
        .map(str::trim)
        .filter(|a| !a.is_empty())
        .map(|a| u16::from_str_radix(a.trim_start_matches('$'), 16).map_err(|_| format!("{a}: want hex")))
        .collect()
}

/// `ADDR=VALUE` (hex address, decimal value) for bytes set before the run.
pub fn parse_set(s: &str) -> Result<(u16, u8), String> {
    let (a, v) = s.split_once('=').ok_or(format!("{s}: want ADDR=VALUE"))?;
    let a = parse_addrs(a)?
        .first()
        .copied()
        .ok_or(format!("{s}: no address"))?;
    let v: u8 = v.parse().map_err(|_| format!("{s}: value 0..255"))?;
    Ok((a, v))
}

/// What a `mem` run does (`mem_run`).
pub struct MemSetup {
    /// Bytes set before the run.
    pub sets: Vec<(u16, u8)>,
    /// Bytes watched.
    pub watch: Vec<u16>,
    /// Instructions to run.
    pub limit: u64,
    /// Stop shortly after the movement bar refills.
    pub until_refill: bool,
    /// Log every draw of the random state.
    pub draws: bool,
    /// Answer messages shown on a creature's turn (a creature picking up
    /// gold, 10.14.4) with the button: the players' names, so a mover not
    /// among them is a creature. Needs the glyph table.
    pub answer: Option<Vec<String>>,
}

/// Rows of the inner screen below the name strip, where messages show.
const MESSAGE_ROWS: std::ops::Range<usize> = 170..200;
/// The message area's background, blue.
const MESSAGE_BG: u8 = 6;

/// Whether the message area holds any text.
fn message_shown(f: &crate::screen::Frame) -> bool {
    let (_, _, w, h) = f.inner();
    let lit = MESSAGE_ROWS
        .filter(|&y| y < h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .filter(|&(x, y)| f.at(x, y) != MESSAGE_BG)
        .count();
    lit > 50
}

impl Runner<'_> {
    /// One `mem` run: restore, set `sets` and (if `state`) the random state,
    /// then look every 20,000 instructions for `limit` instructions, or until
    /// 600,000 instructions after the movement bar refills (`until_refill`:
    /// the top of the next turn), logging each watched byte's changes.
    pub fn mem_run(&mut self, run: usize, state: Option<u8>, setup: &MemSetup) -> Result<(), String> {
        let MemSetup {
            sets,
            watch,
            limit,
            until_refill,
            draws,
            answer,
        } = setup;
        let (sets, watch, limit, until_refill, draws) =
            (&sets[..], &watch[..], *limit, *until_refill, *draws);
        self.reset(0)?;
        // With `draws`, every store to the random state stops the machine:
        // the new state (the draw's raw number) and the raster line are
        // logged, so each roll can be checked on its own (10.14).
        let lin = if draws {
            let names = self.s.vice.mon.registers_available().map_err(|e| e.to_string())?;
            names.iter().find(|(_, n)| n == "LIN").map(|(i, _)| *i)
        } else {
            None
        };
        let checkpoint = if draws {
            Some(
                self.s
                    .vice
                    .mon
                    .watch_store(RANDOM_STATE)
                    .map_err(|e| e.to_string())?,
            )
        } else {
            None
        };
        let mut draw_log = Vec::new();
        let mut answered = Vec::new();
        let mut pressed = false;
        for &(a, v) in sets {
            self.s.vice.mon.mem_set(a, &[v]).map_err(|e| e.to_string())?;
        }
        if let Some(s) = state {
            self.s
                .vice
                .mon
                .mem_set(RANDOM_STATE, &[s])
                .map_err(|e| e.to_string())?;
        }
        let read = |r: &mut Self| -> Result<Vec<u8>, String> {
            watch
                .iter()
                .map(|&a| {
                    r.s.vice
                        .mon
                        .mem_read(a, a, false)
                        .map_err(|e| e.to_string())
                        .map(|b| b.first().copied().unwrap_or(0))
                })
                .collect()
        };
        let start = read(self)?;
        let mut last = start.clone();
        let mut changes = Vec::new();
        let mut bar = self.look()?.bar;
        let (mut t, mut refilled) = (0u64, None);
        let mut limit = if until_refill {
            limit.max(TURN_LIMIT)
        } else {
            limit
        };
        while t < limit {
            if checkpoint.is_some() {
                // Chunks short enough that a hit ends one early, then read.
                let mut done = 0;
                while done < LOOK {
                    let before = self.s.vice.mon.checkpoint_hits;
                    self.s.vice.mon.advance(2_000).map_err(|e| e.to_string())?;
                    done += 2_000;
                    if self.s.vice.mon.checkpoint_hits != before {
                        let raw = self
                            .s
                            .vice
                            .mon
                            .mem_read(RANDOM_STATE, RANDOM_STATE, false)
                            .map_err(|e| e.to_string())?;
                        let line = match lin {
                            Some(id) => self
                                .s
                                .vice
                                .mon
                                .registers()
                                .map_err(|e| e.to_string())?
                                .iter()
                                .find(|(i, _)| *i == id)
                                .map(|(_, v)| *v),
                            None => None,
                        };
                        draw_log.push(json!({"t": t + done, "raw": raw.first(), "line": line}));
                    }
                }
            } else {
                self.step(LOOK)?;
            }
            t += LOOK;
            let now = read(self)?;
            for (i, (&a, &b)) in last.iter().zip(&now).enumerate() {
                if a != b {
                    changes.push(json!({"t": t, "addr": format!("{:04X}", watch[i]), "from": a, "to": b}));
                }
            }
            last = now;
            if let Some(players) = answer {
                if pressed {
                    self.joy("none")?;
                    pressed = false;
                } else {
                    let f = self.s.frame().map_err(|e| e.to_string())?;
                    let t_ = self
                        .glyphs
                        .as_ref()
                        .ok_or("no glyph table (--glyphs or ACSVM_CAPTURE_GLYPHS)")?;
                    let (mover, _) = crate::turns::strip(&f, t_);
                    if !mover.is_empty() && !players.contains(&mover) && message_shown(&f) {
                        if answered.len() < 8 {
                            let png = self
                                .s
                                .out
                                .join(format!("answer{run}-{:02}.png", answered.len() + 1));
                            f.save_png(&png, false, 2).map_err(|e| e.to_string())?;
                        }
                        self.joy("fire")?;
                        pressed = true;
                        answered.push(json!({"t": t, "mover": mover}));
                    }
                }
            }
            let b = self.look()?.bar;
            if b > bar + 4 && refilled.is_none() {
                refilled = Some(t);
                if until_refill {
                    // A little longer: the top-of-turn draws come just after
                    // the refill, the next idle move's much later.
                    limit = t + TAIL;
                }
            }
            bar = b;
        }
        if let Some(n) = checkpoint {
            self.s.vice.mon.delete_checkpoint(n).map_err(|e| e.to_string())?;
        }
        let names: Vec<String> = watch.iter().map(|a| format!("{a:04X}")).collect();
        self.write(json!({
            "kind": "mem", "run": run, "state": state, "watch": names, "start": start,
            "end": last, "changes": changes, "draws": draw_log, "answered": answered, "refilled_at": refilled, "ran": t,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses_and_sets() {
        assert_eq!(parse_addrs("F5A2, $1CC9").unwrap(), vec![0xF5A2, 0x1CC9]);
        assert!(parse_addrs("G1").is_err());
        assert_eq!(parse_set("F65F=0").unwrap(), (0xF65F, 0));
        assert!(parse_set("F65F").is_err());
    }
}
