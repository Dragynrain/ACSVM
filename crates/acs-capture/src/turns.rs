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
    /// The strip's place name (TEST FIELD, TEST ROOM, SIDE ROOM), if it
    /// shows.
    pub place: Option<String>,
    /// Squares of the players' 16 x 16 boxes (two players on one square
    /// show as one).
    pub players: Vec<(i32, i32)>,
    /// (resident slot, square) for each resident box seen.
    pub creatures: Vec<(usize, (i32, i32))>,
}

/// A room square, counted from the room's top-left wall square.
pub type Square = (i32, i32);

/// The place names of our scenarios (docs/capture.md 7, 10.10.3).
const PLACES: [&str; 3] = ["TEST FIELD", "TEST ROOM", "SIDE ROOM"];

fn is_room(place: &Option<String>) -> bool {
    place.as_deref().is_some_and(|p| p.ends_with("ROOM"))
}

/// Who plays a `turns` run, and how.
pub struct TurnsSetup<'a> {
    /// The players' names, first player first.
    pub players: Vec<&'a str>,
    /// The players that walk into the room (the rest stay on the map).
    pub enter: Vec<&'a str>,
    /// Routes after the map door, by player.
    pub routes: Vec<(String, Route)>,
    /// Bytes read at each turn's start (researcher addresses).
    pub peek: Vec<u16>,
}

/// A route: (direction, squares) moves, walked in order.
pub type Route = Vec<(String, u32)>;

/// `NAME=e3,s1` to the player's name and route. `NAME@K=..` is a route
/// for the start of the player's K-th turn (counted from the snapshot,
/// from 1) instead of right after the map door; the name comes back as
/// `NAME@K`.
pub fn parse_route(spec: &str) -> Result<(String, Route), String> {
    let (name, moves) = spec.split_once('=').ok_or(format!("{spec}: want NAME=MOVES"))?;
    if let Some((_, k)) = name.split_once('@') {
        k.parse::<usize>().map_err(|_| format!("{name}: want NAME@K"))?;
    }
    let mut route = Vec::new();
    for m in moves.split(',').map(str::trim).filter(|m| !m.is_empty()) {
        let split = m
            .find(|c: char| c.is_ascii_digit())
            .ok_or(format!("{m}: want e3"))?;
        let (dir, n) = m.split_at(split);
        if crate::script::joy_bits(dir).is_none() {
            return Err(format!("{m}: bad direction"));
        }
        let n: u32 = n.parse().map_err(|_| format!("{m}: bad count"))?;
        route.push((dir.to_string(), n));
    }
    Ok((name.to_string(), route))
}

fn square(x0: usize, y0: usize) -> Square {
    ((x0 as i32 - VIEW_X) / CELL, (y0 as i32 - VIEW_Y) / CELL)
}

/// Reads the name strip (white name, yellow place name): inner rows
/// `STRIP_Y`, on whichever character grid matches the table best.
pub fn strip(f: &Frame, t: &g::Table) -> (String, Option<String>) {
    let yellow = f.palette.get(7).copied().unwrap_or([255, 255, 0]);
    let s = g::Screen::from_frame(f, &[g::INK, yellow], &[]).rows(STRIP_Y.0, STRIP_Y.1);
    let text = g::read(&s, t);
    // The line with a place name; failing that, the fullest one.
    let line = text
        .lines
        .iter()
        .find(|l| PLACES.iter().any(|p| l.contains(p)))
        .or_else(|| text.lines.iter().max_by_key(|l| l.trim().len()))
        .cloned()
        .unwrap_or_default();
    strip_line(&line)
}

/// The mover and place from one line of the strip.
fn strip_line(line: &str) -> (String, Option<String>) {
    let place = PLACES.iter().find(|p| line.contains(*p)).map(|p| p.to_string());
    // The first word, unless the line starts with the place name (the
    // strip names nobody).
    let starts_with_place = PLACES.iter().any(|p| line.trim_start().starts_with(p));
    let mover = match line.split_whitespace().next() {
        Some(w) if !starts_with_place => w.to_string(),
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
        let (players, creatures) = if is_room(&place) {
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

    /// Walks `route` from the current square, letting go of the stick as
    /// soon as the player's square changes (a held stick queues another
    /// step, 10.8) and ending a move early when the place changes (a door).
    /// Returns each move's (direction, squares walked, place after), the
    /// instructions and the frames spent.
    fn walk_route(&mut self, route: &Route) -> Result<(Vec<serde_json::Value>, u64, u64), String> {
        const MOVE_LIMIT: u64 = 40_000_000;
        let (mut t, mut frames) = (0u64, 0u64);
        let mut done = Vec::new();
        for (dir, n) in route {
            let (start, _) = self.turns_look()?;
            let mut at = start.players.first().copied();
            let mut moved = 0;
            let mut spent = 0;
            let mut place = start.place.clone();
            self.joy(dir)?;
            while moved < *n && spent < MOVE_LIMIT {
                frames += self.s.run_counting_frames(LOOK).map_err(|e| e.to_string())?;
                spent += LOOK;
                let (look, _) = self.turns_look()?;
                if look.place.is_some() && look.place != start.place {
                    // Through a door: the move ends.
                    self.joy("none")?;
                    moved += 1;
                    place = look.place;
                    break;
                }
                let now = look.players.first().copied();
                if now.is_some() && now != at {
                    self.joy("none")?;
                    moved += 1;
                    at = now;
                    frames += self.s.run_counting_frames(60_000).map_err(|e| e.to_string())?;
                    spent += 60_000;
                    if moved < *n {
                        self.joy(dir)?;
                    }
                }
            }
            self.joy("none")?;
            t += spent;
            done.push(json!({"dir": dir, "squares": moved, "place": place}));
        }
        Ok((done, t, frames))
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
        setup: &TurnsSetup,
        rounds: usize,
    ) -> Result<(), String> {
        const LIMIT: u64 = 2_000_000_000;
        let TurnsSetup {
            players,
            enter,
            routes,
            peek,
        } = setup;
        let (players, enter, peek) = (&players[..], &enter[..], &peek[..]);
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
        let mut last_place: Option<String> = None;
        let mut walked = Vec::new();
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
                last_place = look.place.clone();
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
                let mut peeked = serde_json::Map::new();
                for &a in peek {
                    let b = self.s.vice.mon.mem_read(a, a, false).map_err(|e| e.to_string())?;
                    peeked.insert(format!("{a:04X}"), b.first().copied().unwrap_or(0).into());
                }
                turns.push(
                    json!({"mover": current, "t": t, "frames": frames, "full_bar": bar, "peek": peeked}),
                );
                if Some(current.as_str()) == players.first().copied() {
                    firsts += 1;
                    if firsts > rounds {
                        break;
                    }
                }
                if let Some(i) = players.iter().position(|p| *p == current) {
                    // A route for this turn of the player's (NAME@K).
                    let k = movers.iter().filter(|m| **m == current).count();
                    let key = format!("{current}@{k}");
                    if let Some((_, route)) = routes.iter().find(|(n, _)| *n == key) {
                        let (done, spent, f) = self.walk_route(route)?;
                        walked.push(json!({"player": key, "t": t, "moves": done}));
                        t += spent;
                        frames += f;
                    }
                    if !entered[i] && enter.contains(&current.as_str()) && !is_room(&look.place) {
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
                if is_room(&look.place) {
                    self.joy("none")?;
                    entered[i] = true;
                    holding = None;
                    // The player's route, on the same turn.
                    if let Some((_, route)) = routes.iter().find(|(n, _)| n == players[i]) {
                        let (done, spent, f) = self.walk_route(route)?;
                        walked.push(json!({"player": players[i], "t": t, "moves": done}));
                        t += spent;
                        frames += f;
                    }
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
            "movers": movers, "turns": turns, "routes": walked, "ended": t < LIMIT, "rows": rows,
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
        let p = |s: &str| Some(s.to_string());
        assert_eq!(
            strip_line("          T          TEST ROOM"),
            ("T".into(), p("TEST ROOM"))
        );
        assert_eq!(
            strip_line("     AA        TEST FIELD"),
            ("AA".into(), p("TEST FIELD"))
        );
        assert_eq!(
            strip_line("     AA        SIDE ROOM"),
            ("AA".into(), p("SIDE ROOM"))
        );
        assert_eq!(
            strip_line("               TEST ROOM"),
            (String::new(), p("TEST ROOM"))
        );
        assert_eq!(strip_line(""), (String::new(), None));
        assert!(is_room(&p("SIDE ROOM")) && !is_room(&p("TEST FIELD")));
    }

    #[test]
    fn routes_parse() {
        assert_eq!(
            parse_route("T=e3,s1").unwrap(),
            ("T".to_string(), vec![("e".to_string(), 3), ("s".to_string(), 1)])
        );
        assert!(parse_route("T=x3").is_err());
        assert_eq!(parse_route("T@3=e3").unwrap().0, "T@3");
        assert!(parse_route("T@x=e3").is_err());
        assert!(parse_route("e3").is_err());
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
