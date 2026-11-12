// SPDX-License-Identifier: GPL-3.0-or-later

//! `acs-capture snap`: snapshots taken just before a roll, for the
//! clean-room researcher's inputs and for state sweeps (docs/capture.md
//! 10.9, 10.11). Each restores a scenario snapshot (a first turn), plays a
//! player's input up to the moment named, and saves a snapshot there.

use crate::experiment::{Runner, Track, TURN_LIMIT};
use crate::power::Plan;
use std::path::Path;

/// What to stop just before.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Moment {
    /// Use Object with our item chosen in SELECT OBJECT: one button tap
    /// from the use.
    ItemMenu,
    /// The turn's bar at `bar_left` pixels or fewer, idling: the end of
    /// the turn (drift, decay, healing) is a few frames away.
    TurnEndIdle,
    /// Walked `walk` squares east first, then as `TurnEndIdle`.
    TurnEndWalked,
    /// The top of the next turn: the bar just refilled after an idle turn
    /// (whatever the end of a turn rolls has been rolled).
    NextTurn,
}

impl Moment {
    pub fn parse(s: &str) -> Result<Moment, String> {
        Ok(match s {
            "item-menu" => Moment::ItemMenu,
            "turn-end" => Moment::TurnEndIdle,
            "turn-end-walked" => Moment::TurnEndWalked,
            "next-turn" => Moment::NextTurn,
            other => {
                return Err(format!(
                    "{other}: item-menu, turn-end, turn-end-walked or next-turn"
                ))
            }
        })
    }
}

impl Runner<'_> {
    /// Plays up to `moment` from the run's snapshot and saves `path` there
    /// (with disks, so the file stands alone) and a screenshot beside it.
    /// Returns a line on what was seen.
    /// `plan`: casts first, one a turn as in `drift` (10.6.5), then waits
    /// for the top of the next turn.
    pub fn snap(
        &mut self,
        moment: Moment,
        plan: &Plan,
        walk: i32,
        bar_left: usize,
        path: &Path,
    ) -> Result<String, String> {
        self.reset(0)?;
        let mut note = String::new();
        if !plan.is_empty() {
            self.wait_refill(TURN_LIMIT)?.ok_or("no new turn")?;
            for taps in plan {
                self.cast_on_self(taps)?;
                self.wait_refill(TURN_LIMIT)?.ok_or("no new turn after a cast")?;
            }
            note = format!("cast {}; ", plan.len());
        }
        match moment {
            Moment::ItemMenu => {
                self.open_menu()?;
                // Use Object: one column right, the button; NONE, then the item.
                for d in ["e", "fire", "s"] {
                    self.menu_tap(d)?;
                }
            }
            Moment::TurnEndIdle | Moment::TurnEndWalked => {
                if moment == Moment::TurnEndWalked {
                    let mut k = Track {
                        frames: 0,
                        bar: self.look()?.bar,
                        refilled: false,
                        count_frames: false,
                    };
                    let (moved, cell) = self.walk(&mut k, "e", walk)?;
                    if k.refilled {
                        return Err(format!("the turn ended after {moved} squares"));
                    }
                    note.push_str(&format!("walked {moved} to {cell:?}; "));
                }
                let mut t = 0;
                let mut bar = self.look()?.bar;
                while bar > bar_left {
                    if t > TURN_LIMIT {
                        return Err(format!("bar still {bar} pixels"));
                    }
                    self.step(5_000)?;
                    t += 5_000;
                    bar = self.look()?.bar;
                }
                note.push_str(&format!("bar {bar} after {t}; "));
            }
            Moment::NextTurn => {
                let t = self.wait_refill(TURN_LIMIT)?.ok_or("no new turn")?;
                note.push_str(&format!("new turn after {t}; "));
            }
        }
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("snap");
        self.s.shot(stem).map_err(|e| e.to_string())?;
        self.s.line(&format!("dump {}", path.display()))?;
        Ok(format!("{note}clock {}", self.s.clock))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moments_by_name() {
        assert_eq!(Moment::parse("item-menu"), Ok(Moment::ItemMenu));
        assert_eq!(Moment::parse("turn-end"), Ok(Moment::TurnEndIdle));
        assert_eq!(Moment::parse("turn-end-walked"), Ok(Moment::TurnEndWalked));
        assert_eq!(Moment::parse("next-turn"), Ok(Moment::NextTurn));
        assert!(Moment::parse("cast").is_err());
    }
}
