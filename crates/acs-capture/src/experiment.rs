// SPDX-License-Identifier: GPL-3.0-or-later

//! Experiments: input scripts plus observers, run many times from a
//! scenario snapshot, written as JSON lines and distilled into a summary
//! (docs/capture.md sections 4 and 7).

use crate::observe::{blobs, Box2};
use crate::scenario::MoveBasic;
use crate::screen::Frame;
use crate::script::{joy_bits, Session};
use serde::Serialize;
use serde_json::json;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

/// C64 colour codes of the scenario palette (`scenario::PALETTE`).
const PLAYER: u8 = 5;
const MARKER: u8 = 2;

/// World map view geometry, in inner-screen pixels: cells are 16 x 16, the
/// view's left edge is at x = 28 and its top at y = -1 (measured from the
/// move-basic frames). Everything below y = 158 is the status area.
const MAP_AREA: (usize, usize, usize, usize) = (28, 0, 240, 158);
/// The movement bar: a vertical colour-3 bar in the bottom-left corner,
/// 8 pixels per point of Speed when full (48 for Speed 6), bottom at
/// y = 158; the area reaches the top so a Speed 15 bar (120) fits.
const BAR_AREA: (usize, usize, usize, usize) = (0, 0, 28, 159);
const BAR_BOTTOM: usize = 158;
/// A bar that grows by more than this is a new turn's bar. (The bar only
/// shrinks within a turn; a Speed 1 bar is 8 pixels, so the step can't be 8.)
const REFILL: usize = 4;
/// Instructions to wait for a turn to pass: an idle Speed 15 turn lasts
/// about 5,400 frames, some 40 million instructions.
const TURN_LIMIT: u64 = 80_000_000;
const CELL: i32 = 16;
const VIEW_X: i32 = 28;
const VIEW_Y: i32 = -1;

pub const DIRS: [&str; 8] = ["n", "ne", "e", "se", "s", "sw", "w", "nw"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Obs {
    /// The player's world map cell, from the marker cell's known position.
    pub cell: Option<(i32, i32)>,
    /// Movement bar height in pixels (0 = empty or hidden).
    pub bar: usize,
}

fn square(b: &Box2) -> bool {
    b.w() == 16 && b.h() == 16
}

/// Where the observer is: on the world map (the view scrolls, so cells
/// are read relative to the marker) or in a room (the room is drawn from
/// the view's top-left corner and doesn't scroll, so the player's box gives
/// its square directly, counted from the room's top-left wall square).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    Map,
    Room,
}

pub fn observe(f: &Frame, s: &MoveBasic, place: Place) -> Obs {
    let players: Vec<Box2> = blobs(f, PLAYER, MAP_AREA).into_iter().filter(square).collect();
    let cell = match place {
        Place::Map => {
            // The full marker is a 16 x 16 box, the second a 16 x 8 one
            // (its cell's top half); either gives the player's cell.
            let marker = blobs(f, MARKER, MAP_AREA)
                .into_iter()
                .filter(|b| b.w() == 16)
                .find_map(|b| match b.h() {
                    16 => Some((b, s.marker)),
                    8 => Some((b, s.marker2)),
                    _ => None,
                });
            match (players.as_slice(), marker) {
                ([p], Some((m, at))) => Some((
                    at.0 as i32 + (p.x0 as i32 - m.x0 as i32) / CELL,
                    at.1 as i32 + (p.y0 as i32 - m.y0 as i32) / CELL,
                )),
                _ => None,
            }
        }
        Place::Room => match players.as_slice() {
            [p] => Some(((p.x0 as i32 - VIEW_X) / CELL, (p.y0 as i32 - VIEW_Y) / CELL)),
            _ => None,
        },
    };
    let bar = blobs(f, PLAYER, BAR_AREA)
        .into_iter()
        .find(|b| b.y1 == BAR_BOTTOM && b.w() >= 8)
        .map(|b| b.h())
        .unwrap_or(0);
    Obs { cell, bar }
}

pub struct Runner<'a> {
    pub s: &'a mut Session,
    pub snapshot: String,
    pub scen: MoveBasic,
    pub place: Place,
    pub out: std::fs::File,
}

impl Runner<'_> {
    fn look(&mut self) -> Result<Obs, String> {
        let f = self.s.frame().map_err(|e| e.to_string())?;
        Ok(observe(&f, &self.scen, self.place))
    }

    fn reset(&mut self, jitter: u64) -> Result<(), String> {
        let line = format!("undump {}", self.snapshot);
        self.s.line(&line)?;
        if jitter > 0 {
            self.s.run(jitter).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    fn joy(&mut self, dir: &str) -> Result<(), String> {
        let bits = joy_bits(dir).ok_or("bad direction")?;
        self.s.vice.mon.joystick(1, bits).map_err(|e| e.to_string())
    }

    fn step(&mut self, n: u64) -> Result<(), String> {
        self.s.run(n).map_err(|e| e.to_string())
    }

    fn write(&mut self, v: serde_json::Value) -> Result<(), String> {
        writeln!(self.out, "{v}").map_err(|e| e.to_string())
    }

    /// Runs until the bar refills (a new turn), up to `limit` instructions.
    fn wait_refill(&mut self, limit: u64) -> Result<Option<u64>, String> {
        let mut last = self.look()?.bar;
        let mut t = 0;
        while t < limit {
            self.step(25_000)?;
            t += 25_000;
            let b = self.look()?.bar;
            if b > last + REFILL {
                // The new bar is drawn over a few frames: wait until it
                // stops growing.
                let mut prev = b;
                loop {
                    self.step(10_000)?;
                    t += 10_000;
                    let now = self.look()?.bar;
                    if now <= prev {
                        break;
                    }
                    prev = now;
                }
                return Ok(Some(t));
            }
            last = b;
        }
        Ok(None)
    }

    /// One push in `dir` from a fresh snapshot: did the player move, where,
    /// and what did the bar do?
    pub fn direction(&mut self, run: usize, jitter: u64, dir: &str) -> Result<(), String> {
        self.reset(jitter)?;
        // Start each push at the top of a fresh turn.
        self.wait_refill(TURN_LIMIT)?;
        self.step(100_000)?;
        let before = self.look()?;
        self.joy(dir)?;
        let mut t = 0;
        let mut moved = None;
        let mut bar_at_move = before.bar;
        let mut bar_after_move = None;
        while t < 1_500_000 {
            self.step(20_000)?;
            t += 20_000;
            let o = self.look()?;
            if o.cell.is_some() && o.cell != before.cell {
                moved = Some(t);
                bar_after_move = Some(o.bar);
                break;
            }
            bar_at_move = o.bar;
        }
        self.joy("none")?;
        self.step(300_000)?;
        let after = self.look()?;
        let delta = match (before.cell, after.cell) {
            (Some(a), Some(b)) if moved.is_some() => Some((b.0 - a.0, b.1 - a.1)),
            _ => None,
        };
        self.write(json!({
            "kind": "direction", "place": format!("{:?}", self.place), "run": run,
            "jitter": jitter, "dir": dir, "before": before, "after": after,
            "moved_after": moved, "delta": delta,
            // Bar just before and at the first frame showing the move (the
            // frames are 20,000 instructions apart, so the real-time drain
            // can't take a pixel in between).
            "bar_before_move": bar_at_move, "bar_after_move": bar_after_move,
        }))
    }

    /// Holds east through one whole turn and counts the steps taken.
    pub fn steps_per_turn(&mut self, run: usize, jitter: u64) -> Result<(), String> {
        self.reset(jitter)?;
        self.wait_refill(TURN_LIMIT)?;
        let start = self.look()?;
        self.joy("e")?;
        let mut cells = vec![start.cell];
        let mut bars = vec![start.bar];
        let mut last_bar = start.bar;
        let mut t = 0;
        let mut ended = false;
        while t < TURN_LIMIT {
            self.step(20_000)?;
            t += 20_000;
            let o = self.look()?;
            if o.bar > last_bar + REFILL {
                ended = true;
                break;
            }
            last_bar = o.bar;
            if o.cell.is_some() && Some(o.cell) != cells.last().copied() {
                // Let the scroll settle before reading the cell.
                self.step(60_000)?;
                t += 60_000;
                let o2 = self.look()?;
                if o2.cell != *cells.last().unwrap() {
                    cells.push(o2.cell);
                    bars.push(o2.bar);
                }
                last_bar = o2.bar;
            }
        }
        self.joy("none")?;
        // Count squares, not observed changes: a fast step can fall between
        // two looks.
        let steps = match (cells.first().copied().flatten(), cells.last().copied().flatten()) {
            (Some(a), Some(b)) => (b.0 - a.0).abs(),
            _ => -1,
        };
        self.write(json!({
            "kind": "steps_per_turn", "run": run, "jitter": jitter,
            "steps": steps, "turn_ended": ended,
            "cells": cells, "bars": bars,
        }))
    }

    /// Leaves the joystick alone for one whole turn: how fast does the bar
    /// drain in real time? Counted in frames.
    pub fn idle_drain(&mut self, run: usize, jitter: u64) -> Result<(), String> {
        self.reset(jitter)?;
        self.wait_refill(TURN_LIMIT)?;
        let full = self.look()?.bar;
        let mut frames = 0;
        let mut samples = vec![(0u64, full)];
        let mut last = full;
        let mut ended = false;
        for _ in 0..(TURN_LIMIT / 25_000) {
            frames += self.s.run_counting_frames(25_000).map_err(|e| e.to_string())?;
            let b = self.look()?.bar;
            if b > last + REFILL {
                ended = true;
                break;
            }
            if b != last {
                samples.push((frames, b));
                last = b;
            }
        }
        self.write(json!({
            "kind": "idle_drain", "run": run, "jitter": jitter, "full_bar": full,
            "frames_to_empty": samples.iter().find(|(_, b)| *b == 0).map(|(f, _)| *f),
            "turn_ended": ended, "samples": samples,
        }))
    }
}

/// Reads `runs.jsonl` and distils it (docs/capture.md section 4.4).
pub fn summarize(path: &Path, speed: u8) -> std::io::Result<serde_json::Value> {
    let text = std::fs::read_to_string(path)?;
    let mut dirs: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    let mut bar_cost: BTreeMap<String, usize> = BTreeMap::new();
    let mut steps: BTreeMap<String, usize> = BTreeMap::new();
    let mut drain: Vec<u64> = Vec::new();
    let mut full: BTreeMap<String, usize> = BTreeMap::new();
    for line in text.lines() {
        let v: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        match v["kind"].as_str() {
            Some("direction") => {
                let d = v["dir"].as_str().unwrap_or("?").to_string();
                let outcome = match v["delta"].as_array() {
                    Some(a) => format!("moved {},{}", a[0], a[1]),
                    None => "no move".into(),
                };
                *dirs.entry(d).or_default().entry(outcome).or_default() += 1;
            }
            Some("steps_per_turn") => {
                *steps.entry(v["steps"].to_string()).or_default() += 1;
                // Bar cost per square: the full bar over the squares one
                // turn held. (Single readings right after a step can catch
                // the bar before it is redrawn, in rooms especially.)
                if let (Some(full), Some(n)) = (v["bars"][0].as_f64(), v["steps"].as_f64()) {
                    if n > 0.0 {
                        *bar_cost.entry(format!("{}", full / n)).or_default() += 1;
                    }
                }
                *full.entry(v["bars"][0].to_string()).or_default() += 1;
            }
            Some("idle_drain") => {
                if let Some(f) = v["frames_to_empty"].as_u64() {
                    drain.push(f);
                }
            }
            _ => {}
        }
    }
    drain.sort_unstable();
    Ok(json!({
        "speed": speed,
        "engine_default_rules": engine_predictions(acs_engine::Rules::default(), speed),
        "directions": dirs,
        "bar_pixels_per_step_per_turn": bar_cost,
        "steps_per_turn": steps,
        "full_bar_pixels": full,
        "idle_frames_to_empty": drain,
        "idle_seconds_to_empty_pal": drain.iter().map(|f| *f as f64 / 50.125).collect::<Vec<_>>(),
    }))
}

/// What our engine does in the same scenario with the given rules: the
/// outcome of one push in each direction from the start, and the steps in
/// one turn of pushing east, for a player of `speed`.
pub fn engine_predictions(rules: acs_engine::Rules, speed: u8) -> serde_json::Value {
    use acs_engine::{Command, Dir, Event, Game, PlayerSetup};
    let s = MoveBasic::default();
    let adv = acs_core::decode(&crate::scenario::move_basic(&s, None))
        .expect("scenario decodes")
        .adventure;
    let setup = PlayerSetup {
        speed,
        ..PlayerSetup::default()
    };
    let names = ["n", "ne", "e", "se", "s", "sw", "w", "nw"];
    let mut dirs = BTreeMap::new();
    for (name, dir) in names.iter().zip(Dir::ALL) {
        let (mut g, _) = Game::new(adv.clone(), 1, rules, setup.clone());
        let outcome = g
            .apply(Command::Move { dir })
            .iter()
            .find_map(|e| match e {
                Event::Moved {
                    from: acs_engine::Location::World { x: x0, y: y0 },
                    to: acs_engine::Location::World { x, y },
                    ..
                } => Some(format!(
                    "moved {},{}",
                    *x as i32 - *x0 as i32,
                    *y as i32 - *y0 as i32
                )),
                _ => None,
            })
            .unwrap_or_else(|| "no move".into());
        dirs.insert(name.to_string(), outcome);
    }
    let (mut g, _) = Game::new(adv, 1, rules, setup);
    let mut steps = 0;
    'turn: for _ in 0..32 {
        for e in g.apply(Command::Move { dir: Dir::E }) {
            match e {
                Event::Moved { .. } => steps += 1,
                Event::TurnEnded { .. } => break 'turn,
                _ => {}
            }
        }
    }
    json!({ "speed": speed, "directions": dirs, "steps_per_turn": steps })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitor::Display;

    /// A blank 320 x 200 frame with the player at map cell (12, 12), the
    /// marker at (15, 14) and a bar of `bar` pixels, drawn the way the
    /// move-basic frames show them.
    fn frame(bar: usize) -> Frame {
        let (w, h) = (320, 200);
        let mut px = vec![0u8; w * h];
        let mut fill = |x0: usize, y0: usize, ww: usize, hh: usize, c: u8| {
            for y in y0..y0 + hh {
                for x in x0..x0 + ww {
                    px[y * w + x] = c;
                }
            }
        };
        fill(140, 63, 16, 16, PLAYER);
        fill(188, 95, 16, 16, MARKER);
        if bar > 0 {
            fill(4, BAR_BOTTOM + 1 - bar, 16, bar, PLAYER);
        }
        Frame {
            display: Display {
                width: w,
                height: h,
                inner: (0, 0, w, h),
                pixels: px,
            },
            palette: vec![[0, 0, 0]; 16],
        }
    }

    #[test]
    fn reads_cell_and_bar() {
        let s = MoveBasic::default();
        let o = observe(&frame(40), &s, Place::Map);
        assert_eq!(o.cell, Some((12, 12)));
        assert_eq!(o.bar, 40);
        assert_eq!(observe(&frame(0), &s, Place::Map).bar, 0);
        // In a room the same box is square (7, 4) of the room.
        assert_eq!(observe(&frame(40), &s, Place::Room).cell, Some((7, 4)));
    }

    #[test]
    fn engine_predictions_follow_the_diagonal_rule() {
        let on = engine_predictions(acs_engine::Rules { diagonals: true }, 6);
        assert_eq!(on["directions"]["ne"], "moved 1,-1");
        assert_eq!(on["steps_per_turn"], 6);
        // The default now matches the capture: no diagonal moves.
        let off = engine_predictions(acs_engine::Rules::default(), 6);
        assert_eq!(off["directions"]["ne"], "no move");
        assert_eq!(off["directions"]["e"], "moved 1,0");
    }
}
