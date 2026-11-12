// SPDX-License-Identifier: GPL-3.0-or-later

//! The `acts` experiment (docs/capture.md 10.2 and 10.8): named tables of
//! cases, each a list of acts played from the top of a fresh turn and
//! compared with a control case in the same run, so the real-time drain
//! cancels. `refuse` is the first table.

use crate::experiment::{observe, room_entry, Place, Runner, Track, TURN_LIMIT};
use serde_json::json;

/// One step of a case.
#[derive(Debug, Clone, Copy)]
pub enum Act {
    /// Hold the stick until the player has moved this many squares.
    Walk(&'static str, i32),
    /// Hold the stick for this many instructions; the player should not move.
    Push(&'static str, u64),
    /// Hold the stick until the room behind the map door shows.
    IntoRoom(&'static str),
    /// Hold the stick until the player shows on this square of that place
    /// (a door; at most `TURN_LIMIT`, then the case goes on, noting it).
    Through(&'static str, Place, (i32, i32)),
    /// Push into a thing to take it, then answer its message.
    Take(&'static str),
    /// Use the player's magic item (menu, Use Object) and answer its message.
    UseItem,
    /// Read the profile: the control for a use (it costs nothing, 10.6.1).
    Profile,
    /// Let the turn run out and wait for the next one's bar.
    WaitTurn,
    /// Do nothing for this many instructions.
    Idle(u64),
    /// This many pushes of 300,000 instructions, each followed by as long
    /// with the stick let go.
    Bump(&'static str, u32),
    /// Hold the stick until the next turn's bar shows, counting the squares
    /// moved: the steps the turn had left.
    CountSteps(&'static str),
}

use Act::*;

type Table = &'static [(&'static str, &'static [Act])];

/// `refuse` (10.2), all from the map: the room cases go through the door
/// first, so they start on the entry square; `door` is their control.
pub const REFUSE: Table = &[
    ("map-wall", &[Walk("w", 6), Push("w", 2_000_000)]),
    ("map-walk", &[Walk("w", 6)]),
    ("door", &[IntoRoom("s")]),
    ("map-walk-s3", &[Walk("s", 3)]),
    ("room-wall", &[IntoRoom("s"), Walk("w", 3), Push("w", 2_000_000)]),
    ("room-walk", &[IntoRoom("s"), Walk("w", 3)]),
    ("room-obstacle", &[IntoRoom("s"), Push("e", 2_000_000)]),
];

/// 10.8.1 (`loose`, map): a use's step cost against the profile, and the
/// profile against nothing; three of each (a Speed 15 player has room).
pub const USE: Table = &[
    ("use", &[UseItem, CountSteps("e")]),
    ("profile", &[Profile, CountSteps("e")]),
    ("walk", &[CountSteps("e")]),
    ("use-3", &[UseItem, UseItem, UseItem, CountSteps("e")]),
    ("profile-3", &[Profile, Profile, Profile, CountSteps("e")]),
];

/// 10.8.2 (`loose`, room): taking west of the entry square, then counting
/// east; each against the same without the take.
pub const TAKE: Table = &[
    (
        "take-next-turn",
        &[IntoRoom("s"), WaitTurn, Take("w"), CountSteps("e")],
    ),
    ("next-turn", &[IntoRoom("s"), WaitTurn, CountSteps("e")]),
    ("take-at-once", &[IntoRoom("s"), Take("w"), CountSteps("e")]),
    ("at-once", &[IntoRoom("s"), CountSteps("e")]),
    (
        "take-after-step",
        &[
            IntoRoom("s"),
            Walk("e", 1),
            Walk("w", 1),
            Take("w"),
            CountSteps("e"),
        ],
    ),
    (
        "after-step",
        &[IntoRoom("s"), Walk("e", 1), Walk("w", 1), CountSteps("e")],
    ),
    (
        "take-twice",
        &[
            IntoRoom("s"),
            WaitTurn,
            Take("w"),
            Walk("w", 1),
            Take("w"),
            CountSteps("e"),
        ],
    ),
];

/// 10.8.3 (the `refuse` disk): the obstacle east of the entry against the
/// room wall, for three push lengths, five bumps, and a push after a step
/// away and back.
pub const OBSTACLE: Table = &[
    ("ob-0.5", &[IntoRoom("s"), Push("e", 500_000)]),
    ("ob-2", &[IntoRoom("s"), Push("e", 2_000_000)]),
    ("ob-8", &[IntoRoom("s"), Push("e", 8_000_000)]),
    ("door", &[IntoRoom("s")]),
    ("wall-0.5", &[IntoRoom("s"), Walk("w", 3), Push("w", 500_000)]),
    ("wall-2", &[IntoRoom("s"), Walk("w", 3), Push("w", 2_000_000)]),
    ("wall-8", &[IntoRoom("s"), Walk("w", 3), Push("w", 8_000_000)]),
    ("room-walk", &[IntoRoom("s"), Walk("w", 3)]),
    ("ob-bump5", &[IntoRoom("s"), Bump("e", 5)]),
    ("wall-bump5", &[IntoRoom("s"), Walk("w", 3), Bump("w", 5)]),
    (
        "ob-after-step",
        &[IntoRoom("s"), Walk("w", 1), Walk("e", 1), Push("e", 2_000_000)],
    ),
    ("after-step", &[IntoRoom("s"), Walk("w", 1), Walk("e", 1)]),
];

/// 10.8.4 (`loose`): south onto the door out to map point 32 (20,12), then
/// counting east on the map.
pub const EXIT: Table = &[
    (
        "exit",
        &[IntoRoom("s"), Through("s", Place::Map, (20, 12)), CountSteps("e")],
    ),
    ("at-once", &[IntoRoom("s"), CountSteps("e")]),
];

/// 10.8.5 (`loose`, each of the three disks): the locked door north of
/// the entry, to 6,7; `again` comes back round to 5,3 and pushes into it
/// from the east.
pub const LOCK: Table = &[
    (
        "lock-push",
        &[IntoRoom("s"), Push("n", 2_000_000), CountSteps("e")],
    ),
    ("at-once", &[IntoRoom("s"), CountSteps("e")]),
    (
        "lock-through",
        &[IntoRoom("s"), Through("n", Place::Room, (6, 7)), CountSteps("e")],
    ),
    (
        "lock-again",
        &[
            IntoRoom("s"),
            Through("n", Place::Room, (6, 7)),
            WaitTurn,
            Walk("n", 4),
            Walk("w", 1),
            Push("w", 2_000_000),
            CountSteps("e"),
        ],
    ),
];

pub fn table(name: &str) -> Result<Table, String> {
    Ok(match name {
        "refuse" => REFUSE,
        "use" => USE,
        "take" => TAKE,
        "obstacle" => OBSTACLE,
        "exit" => EXIT,
        "lock" => LOCK,
        other => {
            return Err(format!(
                "{other}: table refuse, use, take, obstacle, exit or lock"
            ))
        }
    })
}

/// Push (and bump) lengths.
const BUMP: u64 = 300_000;
/// Instructions with no move and no new bar after which a count gives up
/// (held against a wall the bar stops draining).
const STUCK: u64 = 20_000_000;

impl Runner<'_> {
    /// Holds `dir` until the player has moved `n` squares (or `TURN_LIMIT`).
    fn walk(&mut self, k: &mut Track, dir: &str, n: i32) -> Result<(i32, Option<(i32, i32)>), String> {
        let mut cell = self.look()?.cell;
        self.joy(dir)?;
        let mut moved = 0;
        let mut t = 0;
        while moved < n && t < TURN_LIMIT {
            let o = self.tick(k, 20_000)?;
            t += 20_000;
            if o.cell.is_some() && o.cell != cell {
                // Let go at once: a stick still held while the view settles
                // queues one more step, taken later (seen on single-square
                // walks). Press again for the next square.
                self.joy("none")?;
                let o2 = self.tick(k, 60_000)?;
                t += 60_000;
                if moved + 1 < n {
                    self.joy(dir)?;
                }
                match (cell, o2.cell) {
                    (Some(a), Some(b)) if a != b => {
                        moved += (b.0 - a.0).abs() + (b.1 - a.1).abs();
                        cell = o2.cell;
                    }
                    (None, Some(_)) => cell = o2.cell,
                    _ => {}
                }
            }
        }
        self.joy("none")?;
        Ok((moved, cell))
    }

    /// The bar and refill flag after a menu act, which steps without
    /// counting frames.
    fn resync(&mut self, k: &mut Track) -> Result<(), String> {
        k.bar = self.look()?.bar;
        Ok(())
    }

    fn act(&mut self, k: &mut Track, act: Act) -> Result<serde_json::Value, String> {
        let before = self.look()?;
        let (bar0, frames0) = (k.bar, k.frames);
        Ok(match act {
            Walk(dir, n) => {
                let (moved, cell) = self.walk(k, dir, n)?;
                json!({"act": "walk", "dir": dir, "squares": moved, "from": before.cell, "to": cell,
                    "frames": k.frames, "bar": k.bar})
            }
            Push(dir, n) => {
                self.joy(dir)?;
                let o = self.tick(k, n)?;
                self.joy("none")?;
                json!({"act": "push", "dir": dir, "for": n, "from": before.cell, "to": o.cell,
                    "bar_before": bar0, "frames": k.frames, "bar": k.bar})
            }
            Bump(dir, n) => {
                let mut bars = Vec::new();
                let mut cells = Vec::new();
                for _ in 0..n {
                    self.joy(dir)?;
                    let o = self.tick(k, BUMP)?;
                    self.joy("none")?;
                    cells.push(o.cell);
                    self.tick(k, BUMP)?;
                    bars.push(k.bar);
                }
                json!({"act": "bump", "dir": dir, "times": n, "from": before.cell, "cells": cells,
                    "bar_before": bar0, "bars": bars, "frames": k.frames, "bar": k.bar})
            }
            Idle(n) => {
                self.tick(k, n)?;
                json!({"act": "idle", "for": n, "bar_before": bar0, "frames": k.frames, "bar": k.bar})
            }
            IntoRoom(dir) => {
                let (reached, bars) = self.through(k, dir, Place::Room, room_entry(&self.scen))?;
                json!({"act": "into_room", "dir": dir, "from": before.cell,
                    "entered_at_frame": reached, "bars": bars,
                    "refilled_before_entry": k.refilled, "frames": k.frames, "bar": k.bar})
            }
            Through(dir, place, to) => {
                let (reached, bars) = self.through(k, dir, place, to)?;
                json!({"act": "through", "dir": dir, "place": format!("{place:?}"), "to": to,
                    "from": before.cell, "reached_at_frame": reached, "bars": bars,
                    "refilled": k.refilled, "frames": k.frames, "bar": k.bar})
            }
            Take(dir) => {
                // As in `weight`: the program shows "T TAKES ..." and waits
                // for the button, the player on its own square.
                self.joy(dir)?;
                self.tick(k, 1_000_000)?;
                self.joy("none")?;
                let at_message = self.tick(k, 5_000_000)?;
                let screen = self.note_screen("take")?;
                let moved = at_message.cell != before.cell;
                if !moved {
                    self.joy("fire")?;
                    self.tick(k, 4_000_000)?;
                    self.joy("none")?;
                }
                let after = self.tick(k, 300_000)?;
                json!({"act": "take", "dir": dir, "from": before.cell, "moved": moved,
                    "screen": screen, "bar_before": bar0, "bar_at_message": at_message.bar,
                    "cell_after": after.cell, "refilled": k.refilled,
                    "frames": k.frames, "bar": k.bar})
            }
            UseItem => {
                let screen = self.use_item(true)?;
                self.resync(k)?;
                json!({"act": "use", "screen": screen, "bar_before": bar0, "bar": k.bar,
                    "frames_counted": k.frames - frames0})
            }
            Profile => {
                let p = self.profile(None)?;
                self.resync(k)?;
                json!({"act": "profile", "pow": p.get("pow"), "spd": p.get("spd"),
                    "bar_before": bar0, "bar": k.bar})
            }
            WaitTurn => {
                let refill = self.wait_refill(TURN_LIMIT)?;
                let o = self.tick(k, 200_000)?;
                k.refilled = false;
                json!({"act": "wait_turn", "after": refill, "bar_before": bar0, "bar": o.bar})
            }
            CountSteps(dir) => self.count_steps(k, dir)?,
        })
    }

    /// Holds `dir` until the player shows on `to` in `place`. Returns the
    /// frame it got there (none if it never did) and the bar's readings.
    fn through(
        &mut self,
        k: &mut Track,
        dir: &str,
        place: Place,
        to: (i32, i32),
    ) -> Result<(Option<u64>, Vec<usize>), String> {
        let mut bars = vec![k.bar];
        self.joy(dir)?;
        let mut t = 0;
        let mut reached = None;
        while t < TURN_LIMIT {
            self.tick(k, 20_000)?;
            t += 20_000;
            if bars.last() != Some(&k.bar) {
                bars.push(k.bar);
            }
            let f = self.s.frame().map_err(|e| e.to_string())?;
            let here = observe(&f, &self.scen, place).cell == Some(to);
            // In a room the map observer finds no marker; on the map the
            // room observer would read any lone player box.
            let ok = match place {
                Place::Room => here && observe(&f, &self.scen, Place::Map).cell.is_none(),
                Place::Map => here,
            };
            if ok {
                reached = Some(k.frames);
                break;
            }
        }
        self.joy("none")?;
        if reached.is_some() {
            self.place = place;
            // The bar is hidden while a region (or the room again) loads;
            // its showing again is not a new turn. Wait for it (at most 10
            // million instructions: a player out of moves shows none).
            let mut t = 0;
            while k.bar == 0 && t < 10_000_000 {
                self.tick(k, 20_000)?;
                t += 20_000;
            }
            self.tick(k, 100_000)?;
            bars.push(k.bar);
            k.refilled = false;
        }
        Ok((reached, bars))
    }

    /// Holds `dir` until the next turn's bar, counting squares; gives up
    /// when nothing moves for `STUCK` instructions.
    fn count_steps(&mut self, k: &mut Track, dir: &str) -> Result<serde_json::Value, String> {
        let start = self.look()?;
        let mut cell = start.cell;
        let mut squares = 0;
        // The bar after each square.
        let mut bars = vec![k.bar];
        k.refilled = false;
        self.joy(dir)?;
        let (mut t, mut still) = (0, 0);
        while !k.refilled && t < TURN_LIMIT && still < STUCK {
            let o = self.tick(k, 20_000)?;
            t += 20_000;
            still += 20_000;
            if k.refilled {
                break;
            }
            if o.cell.is_some() && o.cell != cell {
                let o2 = self.tick(k, 60_000)?;
                t += 60_000;
                if k.refilled {
                    break;
                }
                if let (Some(a), Some(b)) = (cell, o2.cell) {
                    if a != b {
                        squares += (b.0 - a.0).abs() + (b.1 - a.1).abs();
                        bars.push(k.bar);
                        still = 0;
                    }
                }
                if o2.cell.is_some() {
                    cell = o2.cell;
                }
            }
        }
        self.joy("none")?;
        let last_bar = bars.last().copied();
        Ok(
            json!({"act": "count_steps", "dir": dir, "from": start.cell, "to": cell,
            "squares": squares, "bars": bars, "bar_before_refill": last_bar,
            "turn_ended": k.refilled, "stuck": still >= STUCK, "frames": k.frames}),
        )
    }

    /// One case from the top of a fresh turn: plays its acts; unless the
    /// last act counted steps to the turn's end, reads the bar once settled
    /// and counts frames until the turn ends.
    pub fn case_run(
        &mut self,
        run: usize,
        jitter: u64,
        table: &str,
        name: &str,
        acts: &[Act],
    ) -> Result<(), String> {
        self.place = Place::Map;
        self.reset(jitter)?;
        self.wait_refill(TURN_LIMIT)?;
        let first = self.look()?;
        let mut k = Track {
            frames: 0,
            bar: first.bar,
            refilled: false,
            count_frames: true,
        };
        let mut log = Vec::new();
        for &a in acts {
            log.push(self.act(&mut k, a)?);
        }
        let mut v = json!({
            "kind": "acts", "table": table, "case": name, "run": run, "jitter": jitter,
            "full_bar": first.bar, "start_cell": first.cell, "acts": log,
        });
        if !matches!(acts.last(), Some(CountSteps(_))) {
            // Settle: the bar is redrawn a few frames after a move.
            let settled = self.tick(&mut k, 300_000)?;
            let refilled_during = k.refilled;
            let at_settle = k.frames;
            // Count on until the bar refills: the end of the turn.
            k.refilled = false;
            while !k.refilled && k.frames < at_settle + 20_000 {
                self.tick(&mut k, 25_000)?;
            }
            v["settled_frame"] = at_settle.into();
            v["settled_bar"] = settled.bar.into();
            v["settled_cell"] = json!(settled.cell);
            v["refilled_before_settle"] = refilled_during.into();
            v["turn_frames"] = json!(k.refilled.then_some(k.frames));
        }
        self.write(v)
    }
}
