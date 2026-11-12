// SPDX-License-Identifier: GPL-3.0-or-later

//! The `turns` experiment (docs/capture.md 10.10.1): who moves when, with
//! players and creatures of ours in one room. The name strip under the view
//! names whoever is moving; each individual's room square comes from its
//! picture's box shape (players 16 x 16, residents `RESIDENT_BOXES`).

use crate::experiment::{Runner, CELL, VIEW_X, VIEW_Y};
use crate::glyphs as g;
use crate::observe::blobs;
use crate::scenario::{RESIDENT_BOXES, RESIDENT_INSET};
use crate::screen::Frame;
use serde::Serialize;
use serde_json::json;

/// The inner rows read for the name strip (it sits at about 159..167).
const STRIP_Y: (usize, usize) = (152, 24);
/// Characters (palette index 5) inside the view.
const CHARACTER: u8 = 5;
const VIEW_AREA: (usize, usize, usize, usize) = (28, 0, 240, 158);
/// Instructions between looks.
const LOOK: u64 = 20_000;
/// Bar growth (pixels) that marks a new turn.
const REFILL: usize = 4;

/// One look at the screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Look {
    /// The name strip's first word: who is moving ("" between turns).
    pub mover: String,
    /// "room" or "map" from the strip's place name, if it shows.
    pub place: Option<&'static str>,
    /// Squares of the players' 16 x 16 boxes (two players on one square
    /// show as one).
    pub players: Vec<(i32, i32)>,
    /// (resident slot, square) for each resident box seen.
    pub creatures: Vec<(usize, (i32, i32))>,
}

/// A room square, counted from the room's top-left wall square.
pub type Square = (i32, i32);

fn square(x0: usize, y0: usize) -> Square {
    ((x0 as i32 - VIEW_X) / CELL, (y0 as i32 - VIEW_Y) / CELL)
}

/// Reads the name strip (white name, yellow place name): inner rows
/// `STRIP_Y`, on whichever character grid matches the table best.
pub fn strip(f: &Frame, t: &g::Table) -> (String, Option<&'static str>) {
    let yellow = f.palette.get(7).copied().unwrap_or([255, 255, 0]);
    let s = g::Screen::from_frame(f, &[g::INK, yellow], &[]).rows(STRIP_Y.0, STRIP_Y.1);
    let text = g::read(&s, t);
    // The line with the place name; failing that, the fullest one.
    let line = text
        .lines
        .iter()
        .find(|l| l.contains("TEST"))
        .or_else(|| text.lines.iter().max_by_key(|l| l.trim().len()))
        .cloned()
        .unwrap_or_default();
    strip_line(&line)
}

/// The mover and place from one line of the strip.
fn strip_line(line: &str) -> (String, Option<&'static str>) {
    let place = if line.contains("ROOM") {
        Some("room")
    } else if line.contains("FIELD") {
        Some("map")
    } else {
        None
    };
    // The first word, unless it is the place name (the strip names nobody).
    let mover = match line.split_whitespace().next() {
        Some(w) if !line.trim_start().starts_with("TEST") => w.to_string(),
        _ => String::new(),
    };
    (mover, place)
}

/// The players' and residents' squares from the character boxes in the
/// view. A box filling whole squares on the grid is one player per square
/// (adjacent players' boxes join); a resident's box is its shape, inset.
pub fn individuals(f: &Frame) -> (Vec<Square>, Vec<(usize, Square)>) {
    let mut players = Vec::new();
    let mut creatures = Vec::new();
    for b in blobs(f, CHARACTER, VIEW_AREA) {
        let (w, h) = (b.w(), b.h());
        if let Some(slot) = RESIDENT_BOXES.iter().position(|&s| s == (w, h)) {
            creatures.push((slot, square(b.x0 - RESIDENT_INSET, b.y0 - RESIDENT_INSET)));
        } else if w % 16 == 0 && h % 16 == 0 {
            for dy in (0..h).step_by(16) {
                for dx in (0..w).step_by(16) {
                    let full =
                        (0..16).all(|y| (0..16).all(|x| f.at(b.x0 + dx + x, b.y0 + dy + y) == CHARACTER));
                    if full {
                        players.push(square(b.x0 + dx, b.y0 + dy));
                    }
                }
            }
        }
    }
    players.sort();
    creatures.sort();
    (players, creatures)
}

impl Runner<'_> {
    fn turns_look(&mut self) -> Result<(Look, usize), String> {
        let f = self.s.frame().map_err(|e| e.to_string())?;
        let t = self
            .glyphs
            .as_ref()
            .ok_or("no glyph table (--glyphs or ACSVM_CAPTURE_GLYPHS)")?;
        let (mover, place) = strip(&f, t);
        let (players, creatures) = if place == Some("room") {
            individuals(&f)
        } else {
            (Vec::new(), Vec::new())
        };
        let bar = crate::experiment::observe(&f, &self.scen, crate::experiment::Place::Map).bar;
        Ok((
            Look {
                mover,
                place,
                players,
                creatures,
            },
            bar,
        ))
    }

    /// `turns` (10.10.1): from the run's snapshot, after its jitter, looks
    /// every 20,000 instructions and writes a timeline row whenever the
    /// mover, the place or a square changes. A player of `players` that has
    /// not been in the room yet holds south on its turn until the strip
    /// shows the room (or another mover); other turns are idled out. Ends
    /// at the start of the first player's turn number `rounds` + 1.
    /// `enter`: the players that walk in (the rest stay on the map).
    /// The first 16 changes of mover are saved as screenshots
    /// (`turnRUN-I-MOVER.png`).
    pub fn turns_run(
        &mut self,
        run: usize,
        jitter: u64,
        players: &[&str],
        enter: &[&str],
        rounds: usize,
    ) -> Result<(), String> {
        const LIMIT: u64 = 2_000_000_000;
        self.reset(jitter)?;
        let mut rows = Vec::new();
        let mut movers: Vec<String> = Vec::new();
        // Each turn: who, when, and the fullest bar seen in it.
        let mut turns: Vec<serde_json::Value> = Vec::new();
        let mut entered = vec![false; players.len()];
        let mut holding: Option<usize> = None;
        let mut last: Option<Look> = None;
        let mut current = String::new();
        let (mut t, mut frames, mut firsts, mut last_bar) = (0u64, 0u64, 0usize, 0usize);
        let mut last_place: Option<&'static str> = None;
        while t < LIMIT {
            let (look, bar) = self.turns_look()?;
            // A new turn: another mover, or the same one's bar refilled
            // (the strip keeps the last mover's name between turns). A
            // door hides the bar while the new place loads, and its showing
            // again is not a new turn (10.8): so not when the place changed.
            // The place is the one last seen with a mover named.
            let same_place = look.place.is_none() || look.place == last_place;
            // Only for players: a creature's bar is drawn in steps, and a
            // creature has one turn a phase.
            let player = players.contains(&look.mover.as_str());
            let refilled = player && bar > last_bar + REFILL && !movers.is_empty() && same_place;
            last_bar = bar;
            if look.place.is_some() && !look.mover.is_empty() {
                last_place = look.place;
            }
            if !look.mover.is_empty() && (look.mover != current || refilled) {
                if holding.take().is_some() {
                    self.joy("none")?;
                }
                current = look.mover.clone();
                movers.push(current.clone());
                if movers.len() <= 16 {
                    let safe: String = current.chars().filter(char::is_ascii_alphanumeric).collect();
                    let name = format!("turn{run}-{:02}-{safe}", movers.len());
                    self.s.shot(&name).map_err(|e| e.to_string())?;
                }
                turns.push(json!({"mover": current, "t": t, "frames": frames, "full_bar": bar}));
                if Some(current.as_str()) == players.first().copied() {
                    firsts += 1;
                    if firsts > rounds {
                        break;
                    }
                }
                if let Some(i) = players.iter().position(|p| *p == current) {
                    if !entered[i] && enter.contains(&current.as_str()) && look.place != Some("room") {
                        self.joy("s")?;
                        holding = Some(i);
                    }
                }
            }
            if let Some(last) = turns.last_mut() {
                if bar as u64 > last["full_bar"].as_u64().unwrap_or(0) {
                    last["full_bar"] = bar.into();
                }
            }
            if let Some(i) = holding {
                if look.place == Some("room") {
                    self.joy("none")?;
                    entered[i] = true;
                    holding = None;
                }
            }
            if last.as_ref() != Some(&look) {
                rows.push(json!({
                    "t": t, "frames": frames, "bar": bar, "mover": look.mover,
                    "place": look.place, "players": look.players, "creatures": look.creatures,
                }));
                last = Some(look);
            }
            frames += self.s.run_counting_frames(LOOK).map_err(|e| e.to_string())?;
            t += LOOK;
        }
        self.joy("none")?;
        self.write(json!({
            "kind": "turns", "run": run, "jitter": jitter, "players": players, "enter": enter,
            "movers": movers, "turns": turns, "ended": t < LIMIT, "rows": rows,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitor::Display;

    /// A blank frame with these (x, y, w, h) boxes of the character colour.
    fn frame(boxes: &[(usize, usize, usize, usize)]) -> Frame {
        let (w, h) = (320, 200);
        let mut px = vec![0u8; w * h];
        for &(x0, y0, bw, bh) in boxes {
            for y in y0..y0 + bh {
                for x in x0..x0 + bw {
                    px[y * w + x] = CHARACTER;
                }
            }
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

    /// The top-left pixel of room square (x, y).
    fn at(x: i32, y: i32) -> (usize, usize) {
        ((VIEW_X + x * CELL) as usize, (VIEW_Y + y * CELL) as usize)
    }

    #[test]
    fn strip_names_the_mover_and_the_place() {
        assert_eq!(
            strip_line("          T          TEST ROOM"),
            ("T".into(), Some("room"))
        );
        assert_eq!(
            strip_line("     AA        TEST FIELD"),
            ("AA".into(), Some("map"))
        );
        assert_eq!(
            strip_line("               TEST ROOM"),
            (String::new(), Some("room"))
        );
        assert_eq!(strip_line(""), (String::new(), None));
    }

    #[test]
    fn players_and_residents_by_box() {
        let (px, py) = at(4, 4);
        let (qx, qy) = at(5, 4);
        let (ax, ay) = at(6, 4);
        let (bx, by) = at(12, 2);
        let (aw, ah) = RESIDENT_BOXES[0];
        let (bw, bh) = RESIDENT_BOXES[1];
        let i = RESIDENT_INSET;
        // Two players side by side join into one 32 x 16 box; resident 0
        // stands right of them without touching.
        let f = frame(&[
            (px, py, 16, 16),
            (qx, qy, 16, 16),
            (ax + i, ay + i, aw, ah),
            (bx + i, by + i, bw, bh),
        ]);
        let (players, creatures) = individuals(&f);
        assert_eq!(players, vec![(4, 4), (5, 4)]);
        assert_eq!(creatures, vec![(0, (6, 4)), (1, (12, 2))]);
    }
}
