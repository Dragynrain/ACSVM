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

impl Runner<'_> {
    /// One `mem` run: restore, set `sets` and (if `state`) the random state,
    /// then look every 20,000 instructions for `limit` instructions, or until
    /// 600,000 instructions after the movement bar refills (`until_refill`:
    /// the top of the next turn), logging each watched byte's changes.
    pub fn mem_run(
        &mut self,
        run: usize,
        state: Option<u8>,
        sets: &[(u16, u8)],
        watch: &[u16],
        limit: u64,
        until_refill: bool,
    ) -> Result<(), String> {
        self.reset(0)?;
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
            self.step(LOOK)?;
            t += LOOK;
            let now = read(self)?;
            for (i, (&a, &b)) in last.iter().zip(&now).enumerate() {
                if a != b {
                    changes.push(json!({"t": t, "addr": format!("{:04X}", watch[i]), "from": a, "to": b}));
                }
            }
            last = now;
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
        let names: Vec<String> = watch.iter().map(|a| format!("{a:04X}")).collect();
        self.write(json!({
            "kind": "mem", "run": run, "state": state, "watch": names, "start": start,
            "end": last, "changes": changes, "refilled_at": refilled, "ran": t,
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
