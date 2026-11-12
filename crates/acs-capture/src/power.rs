// SPDX-License-Identifier: GPL-3.0-or-later

//! Experiments that read the profile screen (docs/capture.md 10.6): what
//! opening it costs, the Power cost of casting and of using a magic item,
//! and Power, Life Force and healing over many turns.

use crate::experiment::{Runner, TURN_LIMIT};
use serde_json::json;

impl Runner<'_> {
    /// `profile-check`: is reading the profile free? From the first turn,
    /// after the run's jitter: the bar, the profile, the bar again and the
    /// rest of the turn; then the same without the profile. The menu pauses
    /// the drain, so the bar should not move and the turn should be longer
    /// by just the instructions spent in the menu.
    pub fn profile_check(&mut self, run: usize, jitter: u64) -> Result<(), String> {
        self.reset(jitter)?;
        let before = self.look()?;
        let c0 = self.s.clock;
        let p = self.profile(None)?;
        let spent = self.s.clock - c0;
        let after = self.look()?;
        let rest = self.wait_refill(TURN_LIMIT)?;
        self.reset(jitter)?;
        let control = self.wait_refill(TURN_LIMIT)?;
        self.write(json!({
            "kind": "profile_check", "run": run, "jitter": jitter,
            "before": before, "after": after, "profile": p, "menu_instructions": spent,
            "rest_of_turn": rest, "control_rest_of_turn": control,
        }))
    }
}

/// What a drift run casts on its first turns (10.6.5), one entry a turn:
/// the taps that reach the spell from NONE in SELECT SPELL.
pub type Plan = Vec<Vec<&'static str>>;

/// The drift plans by name. Spell 1 (thing 80) is below NONE in SELECT
/// SPELL (s), spell 2 (thing 79) beside it (e).
pub fn plan(name: &str) -> Result<Plan, String> {
    Ok(match name {
        "none" => vec![],
        "spell1" => vec![vec!["s"]],
        "spell1-spell2" => vec![vec!["s"], vec!["e"]],
        other => return Err(format!("{other}: plan none, spell1 or spell1-spell2")),
    })
}

impl Runner<'_> {
    /// A hash of the screen and, once per different screen, a screenshot.
    pub(crate) fn note_screen(&mut self, prefix: &str) -> Result<String, String> {
        let f = self.s.frame().map_err(|e| e.to_string())?;
        let h = format!("{:016x}", f.hash());
        let name = format!("{prefix}-{h}");
        if !self.s.out.join(format!("{name}.png")).exists() {
            self.s.shot(&name).map_err(|e| e.to_string())?;
        }
        Ok(h)
    }

    /// Use Object: menu e and a tap of the button; SELECT OBJECT (NONE,
    /// then our item): s and a tap; the item's long message; the button.
    /// `tap`: answer the message with a 25,000-instruction tap (as the
    /// profile is left) rather than `button`'s 4 million held (10.8.1).
    pub(crate) fn use_item(&mut self, tap: bool) -> Result<String, String> {
        self.open_menu()?;
        for d in ["e", "fire", "s", "fire"] {
            self.menu_tap(d)?;
        }
        self.settle(10_000_000, 1_000_000)?;
        let screen = self.note_screen("item")?;
        if tap {
            self.joy("fire")?;
            self.step(25_000)?;
            self.joy("none")?;
        } else {
            self.button()?;
        }
        self.step(1_000_000)?;
        Ok(screen)
    }

    /// Use Power, the spell `taps` reaches from NONE, and the button at the
    /// target prompt (its cursor starts on the caster).
    fn cast_on_self(&mut self, taps: &[&str]) -> Result<String, String> {
        self.open_menu()?;
        for d in ["e", "s", "s", "fire"] {
            self.menu_tap(d)?;
        }
        for d in taps {
            self.menu_tap(d)?;
        }
        self.menu_tap("fire")?;
        self.settle(10_000_000, 1_000_000)?;
        let screen = self.note_screen("target")?;
        self.button()?;
        self.step(1_000_000)?;
        Ok(screen)
    }

    /// `item` (10.6.2): the profile; use the item; the bar and the profile;
    /// the next turn's profile; use it again; the profile; the next turn's.
    pub fn item_run(&mut self, run: usize, jitter: u64) -> Result<(), String> {
        self.reset(jitter)?;
        let p0 = self.profile(None)?;
        let b0 = self.look()?;
        let screen1 = self.use_item(false)?;
        let b1 = self.look()?;
        let p1 = self.profile(None)?;
        let refill1 = self.wait_refill(TURN_LIMIT)?;
        let p2 = self.profile(None)?;
        let screen2 = self.use_item(false)?;
        let b2 = self.look()?;
        let p3 = self.profile(None)?;
        let refill2 = self.wait_refill(TURN_LIMIT)?;
        let p4 = self.profile(None)?;
        let ps = [p0, p1, p2, p3, p4];
        let power: Vec<Option<u16>> = ps.iter().map(|p| p.get("pow")).collect();
        self.write(json!({
            "kind": "item", "run": run, "jitter": jitter, "power": power,
            "bar_before": b0.bar, "bar_after_use": [b1.bar, b2.bar],
            "refill_after": [refill1, refill2], "screens": [screen1, screen2],
            "profiles": ps,
        }))
    }

    fn drift_row(&mut self, turn: usize, cast: Option<String>) -> Result<serde_json::Value, String> {
        let p = self.profile(None)?;
        Ok(json!({
            "turn": turn, "pow": p.get("pow"), "lf": p.get("lf"), "wis": p.get("wis"),
            "con": p.get("con"), "clock": self.s.clock, "cast_screen": cast,
        }))
    }

    /// Walks the turn out: east on odd turns (the first after the cast is
    /// turn 1), west on even ones, until the next turn's bar shows (the
    /// stick let go at once, so the new turn keeps its steps). A Speed 15
    /// player's 15 squares fit the field east of the start and back.
    fn walk_turn(&mut self, turn: usize) -> Result<Option<u64>, String> {
        let dir = if turn % 2 == 1 { "e" } else { "w" };
        let mut last = self.look()?.bar;
        self.joy(dir)?;
        let mut t = 0;
        while t < TURN_LIMIT {
            self.step(20_000)?;
            t += 20_000;
            let b = self.look()?.bar;
            if b > last + 4 {
                self.joy("none")?;
                // Let the new bar finish drawing.
                self.step(200_000)?;
                return Ok(Some(t));
            }
            last = b;
        }
        self.joy("none")?;
        Ok(None)
    }

    /// `drift` (10.6.5): at the top of a fresh turn, the profile; the plan's
    /// casts, one a turn; then the profile at the top of each of `turns`
    /// turns, each left to run out, or with `walk` walked out (10.8.7).
    pub fn drift_run(
        &mut self,
        run: usize,
        jitter: u64,
        plan: &Plan,
        turns: usize,
        walk: bool,
    ) -> Result<(), String> {
        self.reset(jitter)?;
        if self.wait_refill(TURN_LIMIT)?.is_none() {
            return Err("no new turn".into());
        }
        let mut rows = vec![self.drift_row(0, None)?];
        let mut turn = 0;
        let casts = plan.iter().map(Some).chain(std::iter::repeat_n(None, turns));
        for taps in casts {
            let screen = match taps {
                Some(t) => Some(self.cast_on_self(t)?),
                None => None,
            };
            let next = if walk && taps.is_none() {
                self.walk_turn(turn)?
            } else {
                self.wait_refill(TURN_LIMIT)?
            };
            if next.is_none() {
                return Err(format!("no new turn after turn {turn}"));
            }
            turn += 1;
            rows.push(self.drift_row(turn, screen)?);
        }
        self.write(json!({ "kind": "drift", "run": run, "jitter": jitter, "walk": walk, "turns": rows }))
    }
}
