// SPDX-License-Identifier: GPL-3.0-or-later

//! Natural healing (magic 5.3): the idle move's roll and the top-of-turn die.

use crate::stats::{same_histogram, SEEDS};
use crate::world::*;
use acs_core::Platform;
use acs_engine::{Command, Dir, Event, Location, PlayerSetup};

/// capture.md 10.12 (`heal15-turn-end-idle`): Speed 15, Life Force about 6
/// under Constitution 20, one idle move left, to the next turn's top. Draw 1
/// is the idle move's roll of 0..14 (heals on 0), draw 2 the top-of-turn
/// die (heals on 2). Over the 256 states: gained 0 in 198 runs, 1 in 55,
/// 2 in 3. Two draws, so compared as a sample.
#[test]
fn healing_at_the_last_idle_move() {
    let a = base(Platform::C64);
    let gained = crate::world::histogram(SEEDS, 3, |seed| {
        let mut g = game(
            &a,
            seed,
            PlayerSetup {
                speed: 15,
                constitution: 20,
                life_force: 14,
                ..Default::default()
            },
            Location::World { x: 12, y: 12 },
        );
        // 14 steps east (walking heals nothing), then the last move idles.
        for _ in 0..14 {
            let ev = g.apply(Command::Move { dir: Dir::E });
            assert!(ev.iter().any(|e| matches!(e, Event::Moved { .. })), "{ev:?}");
        }
        let before = life(&g);
        let ev = g.apply(Command::Idle);
        assert!(matches!(ev.last(), Some(Event::TurnStarted { .. })), "{ev:?}");
        usize::from(life(&g) - before)
    });
    same_histogram("healing, last idle move to the next top", &[198, 55, 3], &gained);
}
