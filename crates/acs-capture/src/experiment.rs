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
/// The Power and Life Force bars at the status area's right edge, 4 pixels
/// a point, bottom at y = 158 like the movement bar (docs/capture.md 10.5).
/// Each is read in one column, so its colour (Power turns grey while a
/// message waits) and the red strip below y = 158 don't matter.
const POWER_X: usize = 283;
const LIFE_X: usize = 313;
/// A bar that grows by more than this is a new turn's bar. (The bar only
/// shrinks within a turn; a Speed 1 bar is 8 pixels, so the step can't be 8.)
const REFILL: usize = 4;
/// Instructions to wait for a turn to pass: an idle Speed 15 turn lasts
/// about 5,400 frames, some 40 million instructions.
pub(crate) const TURN_LIMIT: u64 = 80_000_000;
/// A pixel inside the command menu's cursor on MOVE (cyan), which shows
/// only while the menu is open.
const MENU_CURSOR: (usize, usize) = (15, 170);
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
    /// Power and Life Force bar heights in pixels (4 a point).
    pub power: usize,
    pub life: usize,
}

/// Non-black pixels in column `x`, counted up from `BAR_BOTTOM`.
fn column(f: &Frame, x: usize) -> usize {
    (0..=BAR_BOTTOM).rev().take_while(|&y| f.at(x, y) != 0).count()
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
            // (its cell's top half), the third 8 x 16 (the left half); any
            // of them gives the player's cell.
            let marker = blobs(f, MARKER, MAP_AREA)
                .into_iter()
                .find_map(|b| match (b.w(), b.h()) {
                    (16, 16) => Some((b, s.marker)),
                    (16, 8) => Some((b, s.marker2)),
                    (8, 16) => Some((b, s.marker3)),
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
    Obs {
        cell,
        bar,
        power: column(f, POWER_X),
        life: column(f, LIFE_X),
    }
}

pub struct Runner<'a> {
    pub s: &'a mut Session,
    pub snapshot: String,
    pub scen: MoveBasic,
    pub place: Place,
    pub out: std::fs::File,
    /// The glyph table, for reading the profile screen (5.3).
    pub glyphs: Option<crate::glyphs::Table>,
}

/// The profile screen read back (docs/capture.md 5.3.5).
#[derive(Debug, Clone, Serialize)]
pub struct ProfileRead {
    pub stats: BTreeMap<String, crate::glyphs::Stat>,
    /// Instructions from the button to the profile showing.
    pub shown_after: u64,
}

impl ProfileRead {
    pub fn get(&self, key: &str) -> Option<u16> {
        self.stats.get(key).and_then(|s| s.value)
    }
}

impl Runner<'_> {
    pub(crate) fn look(&mut self) -> Result<Obs, String> {
        let f = self.s.frame().map_err(|e| e.to_string())?;
        Ok(observe(&f, &self.scen, self.place))
    }

    pub(crate) fn reset(&mut self, jitter: u64) -> Result<(), String> {
        let line = format!("undump {}", self.snapshot);
        self.s.line(&line)?;
        if jitter > 0 {
            self.s.run(jitter).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub(crate) fn joy(&mut self, dir: &str) -> Result<(), String> {
        let bits = joy_bits(dir).ok_or("bad direction")?;
        self.s.vice.mon.joystick(1, bits).map_err(|e| e.to_string())
    }

    pub(crate) fn step(&mut self, n: u64) -> Result<(), String> {
        self.s.run(n).map_err(|e| e.to_string())
    }

    pub(crate) fn write(&mut self, v: serde_json::Value) -> Result<(), String> {
        writeln!(self.out, "{v}").map_err(|e| e.to_string())
    }

    /// Runs until the bar refills (a new turn), up to `limit` instructions.
    pub(crate) fn wait_refill(&mut self, limit: u64) -> Result<Option<u64>, String> {
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

/// One step of a `refuse` case (docs/capture.md 10.2).
#[derive(Debug, Clone, Copy)]
pub enum Act {
    /// Hold the stick until the player has moved this many squares.
    Walk(&'static str, i32),
    /// Hold the stick for this many instructions; the player should not move.
    Push(&'static str, u64),
    /// Hold the stick until the room behind the map door shows.
    IntoRoom(&'static str),
}

/// The `refuse` cases, all from the map's first turn: name and acts. The
/// room cases go through the door first, so they start on the entry
/// square; `door` is their control.
pub const REFUSE: [(&str, &[Act]); 7] = [
    ("map-wall", &[Act::Walk("w", 6), Act::Push("w", 2_000_000)]),
    ("map-walk", &[Act::Walk("w", 6)]),
    ("door", &[Act::IntoRoom("s")]),
    ("map-walk-s3", &[Act::Walk("s", 3)]),
    (
        "room-wall",
        &[Act::IntoRoom("s"), Act::Walk("w", 3), Act::Push("w", 2_000_000)],
    ),
    ("room-walk", &[Act::IntoRoom("s"), Act::Walk("w", 3)]),
    ("room-obstacle", &[Act::IntoRoom("s"), Act::Push("e", 2_000_000)]),
];

/// A case's running state: frames since the turn started, and what was seen.
struct Track {
    frames: u64,
    bar: usize,
    refilled: bool,
}

impl Runner<'_> {
    /// Runs `n` instructions in 20,000-instruction looks, counting frames and
    /// noting a bar refill (a new turn).
    fn tick(&mut self, k: &mut Track, n: u64) -> Result<Obs, String> {
        let mut done = 0;
        let mut o = self.look()?;
        while done < n {
            let c = (n - done).min(20_000);
            k.frames += self.s.run_counting_frames(c).map_err(|e| e.to_string())?;
            done += c;
            o = self.look()?;
            if o.bar > k.bar + REFILL {
                k.refilled = true;
            }
            k.bar = o.bar;
        }
        Ok(o)
    }

    /// One `refuse` case from the top of a fresh turn: plays its acts, reads
    /// the bar once settled, then counts frames until the turn ends.
    pub fn refuse_case(
        &mut self,
        run: usize,
        jitter: u64,
        name: &str,
        acts: &[Act],
        start_place: Place,
    ) -> Result<(), String> {
        self.place = start_place;
        self.reset(jitter)?;
        self.wait_refill(TURN_LIMIT)?;
        let first = self.look()?;
        let mut k = Track {
            frames: 0,
            bar: first.bar,
            refilled: false,
        };
        let mut log = Vec::new();
        for act in acts {
            let before = self.look()?.cell;
            match *act {
                Act::Walk(dir, n) => {
                    self.joy(dir)?;
                    let mut cell = before;
                    let mut moved = 0;
                    let mut t = 0;
                    while moved < n && t < TURN_LIMIT {
                        let o = self.tick(&mut k, 20_000)?;
                        t += 20_000;
                        if o.cell.is_some() && o.cell != cell {
                            // Let the view settle before counting the square.
                            let o2 = self.tick(&mut k, 60_000)?;
                            t += 60_000;
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
                    log.push(json!({"act": "walk", "dir": dir, "squares": moved,
                        "from": before, "to": cell, "frames": k.frames, "bar": k.bar}));
                }
                Act::Push(dir, n) => {
                    let bar0 = k.bar;
                    self.joy(dir)?;
                    let o = self.tick(&mut k, n)?;
                    self.joy("none")?;
                    log.push(json!({"act": "push", "dir": dir, "from": before, "to": o.cell,
                        "bar_before": bar0, "frames": k.frames, "bar": k.bar}));
                }
                Act::IntoRoom(dir) => {
                    let bar0 = k.bar;
                    let mut bars = vec![bar0];
                    self.joy(dir)?;
                    let mut t = 0;
                    let mut entered = None;
                    while t < TURN_LIMIT {
                        self.tick(&mut k, 20_000)?;
                        t += 20_000;
                        if bars.last() != Some(&k.bar) {
                            bars.push(k.bar);
                        }
                        let f = self.s.frame().map_err(|e| e.to_string())?;
                        let on_map = observe(&f, &self.scen, Place::Map).cell;
                        let in_room = observe(&f, &self.scen, Place::Room).cell;
                        if on_map.is_none() && in_room == Some(room_entry(&self.scen)) {
                            entered = Some(k.frames);
                            break;
                        }
                    }
                    self.joy("none")?;
                    self.place = Place::Room;
                    log.push(json!({"act": "into_room", "dir": dir, "from": before,
                        "entered_at_frame": entered, "bars": bars,
                        "refilled_before_entry": k.refilled, "frames": k.frames, "bar": k.bar}));
                }
            }
        }
        // Settle: the bar is redrawn a few frames after a move.
        let settled = self.tick(&mut k, 300_000)?;
        let refilled_during = k.refilled;
        let at_settle = k.frames;
        // Count on until the bar refills: the end of the turn.
        k.refilled = false;
        while !k.refilled && k.frames < at_settle + 20_000 {
            self.tick(&mut k, 25_000)?;
        }
        self.write(json!({
            "kind": "refuse", "case": name, "run": run, "jitter": jitter,
            "full_bar": first.bar, "start_cell": first.cell, "acts": log,
            "settled_frame": at_settle, "settled_bar": settled.bar, "settled_cell": settled.cell,
            "refilled_before_settle": refilled_during,
            "turn_frames": if k.refilled { Some(k.frames) } else { None },
        }))
    }
}

impl Runner<'_> {
    /// The `weight` run (docs/capture.md 10.3): through the door, then one
    /// step east onto each thing of the row; after each pick-up, the bar
    /// right away and the next turn's full bar.
    pub fn weight_run(&mut self, run: usize, jitter: u64, items: usize) -> Result<(), String> {
        self.place = Place::Map;
        self.reset(jitter)?;
        self.wait_refill(TURN_LIMIT)?;
        let first = self.look()?;
        let mut k = Track {
            frames: 0,
            bar: first.bar,
            refilled: false,
        };
        let mut steps = Vec::new();
        // Through the door (as in the refuse cases).
        self.joy("s")?;
        let mut t = 0;
        while t < TURN_LIMIT {
            self.tick(&mut k, 20_000)?;
            t += 20_000;
            let f = self.s.frame().map_err(|e| e.to_string())?;
            if observe(&f, &self.scen, Place::Map).cell.is_none()
                && observe(&f, &self.scen, Place::Room).cell == Some(room_entry(&self.scen))
            {
                break;
            }
        }
        self.joy("none")?;
        self.place = Place::Room;
        let entered = self.tick(&mut k, 300_000)?;
        for i in 0..items {
            // From the second thing on, first step onto the square the last
            // one was taken from (the player stays put when taking).
            let mut walked = None;
            if i > 0 {
                let from = self.look()?.cell;
                self.joy("e")?;
                let mut t = 0;
                while t < TURN_LIMIT {
                    let o = self.tick(&mut k, 20_000)?;
                    t += 20_000;
                    if o.cell.is_some() && o.cell != from {
                        walked = o.cell;
                        break;
                    }
                }
                self.joy("none")?;
                self.tick(&mut k, 300_000)?;
            }
            let before = self.look()?;
            // Push east into the thing: the program shows "T takes ..." and
            // waits for the button.
            self.joy("e")?;
            self.tick(&mut k, 1_000_000)?;
            self.joy("none")?;
            let at_message = self.tick(&mut k, 5_000_000)?;
            let moved = at_message.cell != before.cell;
            if !moved {
                self.joy("fire")?;
                self.tick(&mut k, 4_000_000)?;
                self.joy("none")?;
            }
            let after = self.tick(&mut k, 2_000_000)?;
            // Let the turn run out, then read the new turn's full bar.
            let refill = self.wait_refill(TURN_LIMIT)?;
            let next = self.tick(&mut k, 200_000)?;
            steps.push(json!({
                "item": i + 1, "walked_to": walked, "from": before.cell,
                "moved_on_push": moved, "cell_after": after.cell,
                "bar_before": before.bar, "bar_at_message": at_message.bar,
                "bar_after_button": after.bar,
                "new_turn": refill.is_some(), "next_full_bar": next.bar,
            }));
            if refill.is_none() {
                // No new turn: the player can no longer move (Speed 0).
                break;
            }
        }
        self.write(json!({
            "kind": "weight", "run": run, "jitter": jitter, "full_bar": first.bar,
            "bar_in_room": entered.bar, "steps": steps,
        }))
    }
}

impl Runner<'_> {
    /// Taps the stick for 25,000 instructions (the menu cursor repeats
    /// after about 40,000) and lets the menu settle.
    pub(crate) fn menu_tap(&mut self, dir: &str) -> Result<(), String> {
        self.joy(dir)?;
        self.step(25_000)?;
        self.joy("none")?;
        self.step(10_000_000)
    }

    /// Opens the command menu from the map: the button held for a million
    /// instructions (holding it stops the drain; the menu opens when it is
    /// let go), then waits for the menu's cursor on MOVE. A short tap can be
    /// missed, and the menu taps that follow would then walk the player.
    pub(crate) fn open_menu(&mut self) -> Result<(), String> {
        // First the turn's prompt ("move with the joystick ..."): between
        // turns the status area is blank and a press opens nothing.
        let mut t = 0;
        while !self.prompt_shows()? {
            if t > 40_000_000 {
                self.s.shot("no-prompt").map_err(|e| e.to_string())?;
                return Err("no turn prompt (no-prompt.png)".into());
            }
            self.step(100_000)?;
            t += 100_000;
        }
        self.joy("fire")?;
        self.step(1_000_000)?;
        self.joy("none")?;
        for _ in 0..60 {
            self.step(500_000)?;
            let f = self.s.frame().map_err(|e| e.to_string())?;
            let c = f.palette.get(f.at(MENU_CURSOR.0, MENU_CURSOR.1) as usize);
            if c == Some(&crate::glyphs::MASK[0]) {
                return self.step(5_000_000);
            }
        }
        self.s.shot("menu-unopened").map_err(|e| e.to_string())?;
        Err("the command menu did not open (menu-unopened.png)".into())
    }

    /// Whether the status area shows text in the prompt colour (cyan), as
    /// it does for the turn's prompt and the open menu.
    fn prompt_shows(&mut self) -> Result<bool, String> {
        let f = self.s.frame().map_err(|e| e.to_string())?;
        for y in 167..195 {
            for x in 4..316 {
                if f.palette.get(f.at(x, y) as usize) == Some(&crate::glyphs::MASK[0]) {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    pub(crate) fn button(&mut self) -> Result<(), String> {
        self.joy("fire")?;
        self.step(4_000_000)?;
        self.joy("none")
    }

    /// The `cast` run (docs/capture.md 10.4 and 10.5): from the first turn
    /// on the map, after the run's jitter, reads the three bars, casts the
    /// player's spell (menu, Use Power) and looks 10 million instructions
    /// later: the spell's long message means it worked, the map still
    /// showing means it failed. Then the button, the bars again, and the
    /// bars once more at the start of the next turn.
    pub fn cast_run(&mut self, run: usize, jitter: u64) -> Result<(), String> {
        self.place = Place::Map;
        self.reset(jitter)?;
        let start = self.look()?;
        self.button()?;
        self.step(30_000_000)?;
        // Use Power: one column right, two rows down. Then short taps: a
        // held button would pick the list's first entry, NONE.
        self.menu_tap("e")?;
        self.menu_tap("s")?;
        self.menu_tap("s")?;
        self.menu_tap("fire")?;
        // "SELECT SPELL": NONE, then the spell.
        self.menu_tap("s")?;
        let chosen = self.look()?;
        self.menu_tap("fire")?;
        let f = self.s.frame().map_err(|e| e.to_string())?;
        let outcome_hash = f.hash();
        let outcome = observe(&f, &self.scen, Place::Map);
        let worked = outcome.cell.is_none();
        let kind = if worked { "worked" } else { "failed" };
        // One screenshot of each different outcome screen, named by its
        // hash (a cost above Power might be refused some other way).
        let name = format!("cast-{kind}-{outcome_hash:016x}");
        if !self.s.out.join(format!("{name}.png")).exists() {
            self.s.shot(&name).map_err(|e| e.to_string())?;
        }
        self.button()?;
        self.step(10_000_000)?;
        let after = self.look()?;
        // With a glyph table, Power read back (10.6.3). The cast ended the
        // turn, so this is the next turn's top.
        let profile = match self.glyphs {
            Some(_) => Some(self.profile(None)?),
            None => None,
        };
        let next_turn = self.wait_refill(TURN_LIMIT)?;
        let next = self.look()?;
        self.write(json!({
            "kind": "cast", "run": run, "jitter": jitter, "start": start,
            "chosen": chosen, "worked": worked, "outcome": outcome,
            "outcome_hash": format!("{outcome_hash:016x}"),
            "after": after, "profile": profile, "next_turn_after": next_turn, "next": next,
        }))
    }
}

impl Runner<'_> {
    /// Opens the profile (a tap of the button, then e, e, s, s and a tap
    /// in the command menu, whose cursor starts on MOVE each time), reads
    /// it once every stat shows, and leaves it with a tap of the button;
    /// the map shows again. `shot` saves the profile screen under that name.
    pub fn profile(&mut self, shot: Option<&str>) -> Result<ProfileRead, String> {
        use crate::glyphs as g;
        if self.glyphs.is_none() {
            return Err("no glyph table (--glyphs or ACSVM_CAPTURE_GLYPHS)".into());
        }
        self.open_menu()?;
        for d in ["e", "e", "s", "s"] {
            self.menu_tap(d)?;
        }
        self.joy("fire")?;
        self.step(25_000)?;
        self.joy("none")?;
        let mut t = 25_000;
        // Two equal readings a million instructions apart, so a number
        // still being drawn is not taken.
        let mut prev = None;
        let stats = loop {
            self.step(1_000_000)?;
            t += 1_000_000;
            let f = self.s.frame().map_err(|e| e.to_string())?;
            if f.palette.get(1) != Some(&g::INK) || f.palette.get(3) != Some(&g::MASK[0]) {
                return Err("VICE's palette is not the one the glyph colours assume".into());
            }
            let text = g::read(
                &g::Screen::from_frame(&f, &[g::INK], &g::MASK),
                self.glyphs.as_ref().unwrap(),
            );
            let stats = g::profile(&text);
            let full = stats.len() == g::PROFILE_STATS.len() && stats.values().all(|s| s.value.is_some());
            if full && prev.as_ref() == Some(&stats) {
                break stats;
            }
            prev = full.then_some(stats);
            if t > 60_000_000 {
                self.s.shot("profile-unread").map_err(|e| e.to_string())?;
                return Err("the profile did not read (profile-unread.png)".into());
            }
        };
        if let Some(name) = shot {
            self.s.shot(name).map_err(|e| e.to_string())?;
        }
        // Leave: a 25,000-instruction tap of the button, then a million
        // instructions for the map to show (the turn goes on from here).
        self.joy("fire")?;
        self.step(25_000)?;
        self.joy("none")?;
        self.step(1_000_000)?;
        Ok(ProfileRead {
            stats,
            shown_after: t,
        })
    }
}

fn room_entry(s: &MoveBasic) -> (i32, i32) {
    (s.room_entry.0 as i32, s.room_entry.1 as i32)
}

/// Reads `runs.jsonl` and distils it (docs/capture.md section 4.4).
pub fn summarize(path: &Path, speed: u8) -> std::io::Result<serde_json::Value> {
    let text = std::fs::read_to_string(path)?;
    let mut dirs: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    let mut bar_cost: BTreeMap<String, usize> = BTreeMap::new();
    let mut steps: BTreeMap<String, usize> = BTreeMap::new();
    let mut drain: Vec<u64> = Vec::new();
    let mut full: BTreeMap<String, usize> = BTreeMap::new();
    // refuse cases: settled bars (counted) and turn lengths, per case.
    let mut refuse: BTreeMap<String, (BTreeMap<String, usize>, Vec<u64>)> = BTreeMap::new();
    // cast: per outcome, counts of each reading.
    let mut cast: BTreeMap<String, BTreeMap<String, BTreeMap<String, usize>>> = BTreeMap::new();
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
            Some("refuse") => {
                let e = refuse
                    .entry(v["case"].as_str().unwrap_or("?").to_string())
                    .or_default();
                *e.0.entry(v["settled_bar"].to_string()).or_default() += 1;
                if let Some(f) = v["turn_frames"].as_u64() {
                    e.1.push(f);
                }
            }
            Some("cast") => {
                let outcome = if v["worked"].as_bool() == Some(true) {
                    "worked"
                } else {
                    "failed"
                };
                let e = cast.entry(outcome.into()).or_default();
                let mut count = |k: &str, val: String| {
                    *e.entry(k.into()).or_default().entry(val).or_default() += 1;
                };
                count("runs", "all".into());
                for (k, at) in [("start", "start"), ("after", "after"), ("next", "next")] {
                    count(&format!("{k}_power_px"), v[at]["power"].to_string());
                    count(&format!("{k}_life_px"), v[at]["life"].to_string());
                    count(&format!("{k}_bar_px"), v[at]["bar"].to_string());
                }
                count("chosen_power_px", v["chosen"]["power"].to_string());
                count("outcome_hash", v["outcome_hash"].to_string());
                // The wait for the next turn, in millions of instructions.
                let wait = v["next_turn_after"]
                    .as_u64()
                    .map(|t| format!("{}M", t / 1_000_000))
                    .unwrap_or_else(|| "none".into());
                count("next_turn_after", wait);
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
    let refuse: BTreeMap<String, serde_json::Value> = refuse
        .into_iter()
        .map(|(case, (bars, mut turns))| {
            turns.sort_unstable();
            let range = (turns.first().copied(), turns.last().copied());
            (
                case,
                json!({"settled_bar": bars, "turn_frames": [range.0, range.1], "turns": turns.len()}),
            )
        })
        .collect();
    Ok(json!({
        "cast": cast,
        "refuse": refuse,
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
    fn reads_power_and_life_columns() {
        let s = MoveBasic::default();
        let mut f = frame(0);
        let w = f.display.width;
        // Power 20 (80 px) in grey, Life Force 11 (44 px) in red, with the
        // red strip below the bars' bottom.
        for y in BAR_BOTTOM + 1 - 80..=BAR_BOTTOM {
            f.display.pixels[y * w + POWER_X] = 12;
        }
        for y in BAR_BOTTOM + 1 - 44..BAR_BOTTOM + 9 {
            f.display.pixels[y * w + LIFE_X] = MARKER;
        }
        let o = observe(&f, &s, Place::Map);
        assert_eq!((o.power, o.life), (80, 44));
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
