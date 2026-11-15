// SPDX-License-Identifier: GPL-3.0-or-later

//! Casting (magic 4): the cast chance, a failed cast's cost.

use crate::stats::{same_rate, Sweep, SEEDS};
use crate::world::*;
use acs_core::Adventure;
use acs_core::Platform;
use acs_engine::{Command, Event, Game, PlayerSetup};

const SPELL: u16 = 33;

fn adventure(cost: u8) -> Adventure {
    let mut a = base(Platform::C64);
    thing(&mut a, SPELL, "SPELL", spell(cost, nothing()));
    a
}

/// A player of Speed 6 on ROOM (4,4) with Power `power` (Wisdom the same)
/// owning one spell; whether its cast at itself worked.
fn cast(a: &Adventure, seed: u64, power: u8) -> (bool, Vec<Event>, Game) {
    let mut g = game(
        a,
        seed,
        PlayerSetup {
            speed: 6,
            wisdom: power,
            power: Some(power),
            possessions: vec![SPELL],
            ..Default::default()
        },
        room(4, 4),
    );
    let ev = g.apply(Command::Cast {
        thing: SPELL,
        target: None,
    });
    let worked = ev
        .iter()
        .find_map(|e| match e {
            Event::SpellCast { success, .. } => Some(*success),
            _ => None,
        })
        .expect("a cast");
    (worked, ev, g)
}

/// capture.md 10.9: the random state set to each of its 256 values just
/// before the cast, from the menu snapshot (magic 4.1).
#[test]
fn cast_chance_sweep() {
    for (power, cost, worked) in [(20, 10, 231), (20, 60, 74), (20, 90, 50), (10, 20, 117)] {
        let a = adventure(cost);
        let eng = crate::world::count(SEEDS, |s| cast(&a, s, power).0);
        same_rate(
            &format!("cast Power {power} cost {cost}"),
            Sweep::Exhaustive,
            worked,
            256,
            eng,
            SEEDS,
        );
    }
}

/// capture.md 10.6.3: Power 20 against cost 30, a failure spends the full
/// cost as a success does: all 60 runs read Power 1 on the next turn, 0
/// plus one turn's drift toward Wisdom.
#[test]
fn a_failed_cast_spends_the_cost() {
    let a = adventure(30);
    let mut seen = [false, false];
    for seed in 0..64 {
        let (worked, ev, g) = cast(&a, seed, 20);
        seen[worked as usize] = true;
        assert!(matches!(ev.last(), Some(Event::TurnStarted { .. })), "{ev:?}");
        assert_eq!(g.players()[0].power, 1, "seed {seed}, worked {worked}: {ev:?}");
    }
    assert_eq!(seen, [true, true]);
}

/// capture.md 10.4: a spell of cost 0 always works (20 of 20).
#[test]
fn a_cost_0_spell_always_works() {
    let a = adventure(0);
    assert!((0..256).all(|s| cast(&a, s, 20).0));
}
