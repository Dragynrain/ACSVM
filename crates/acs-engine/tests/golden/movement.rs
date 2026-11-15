// SPDX-License-Identifier: GPL-3.0-or-later

//! Movement and weight (movement 2-3, inventory 3): the Speed survey, the
//! weight slowdown, refused moves and the door's cost. All deterministic.

use crate::world::*;
use acs_core::{Adventure, Platform};
use acs_engine::{Command, Dir, Event, Game, Location, PlayerSetup};

fn map(x: u8, y: u8) -> Location {
    Location::World { x, y }
}

/// Pushes `dir` until the turn ends; the squares moved.
fn hold(g: &mut Game, dir: Dir) -> u32 {
    let mut steps = 0;
    for _ in 0..64 {
        let ev = g.apply(Command::Move { dir });
        steps += ev.iter().filter(|e| matches!(e, Event::Moved { .. })).count() as u32;
        if ev.iter().any(|e| matches!(e, Event::TurnEnded { .. })) {
            return steps;
        }
    }
    panic!("the turn never ended");
}

/// The moves the next turn starts with, after waiting out this one.
fn next_turn(g: &mut Game) -> u8 {
    wait(g)
        .iter()
        .rev()
        .find_map(|e| match e {
            Event::TurnStarted { moves, .. } => Some(*moves),
            _ => None,
        })
        .expect("a turn")
}

/// capture.md 7 (Speed survey): holding east for a whole turn on the map
/// takes Speed steps, at Speed 1, 3, 6, 9 and 15 (20 runs each, all alike).
#[test]
fn speed_survey_steps_per_turn() {
    let a = base(Platform::C64);
    for speed in [1u8, 3, 6, 9, 15] {
        let mut g = game(
            &a,
            1,
            PlayerSetup {
                speed,
                ..Default::default()
            },
            map(12, 12),
        );
        assert_eq!(hold(&mut g, Dir::E), u32::from(speed), "Speed {speed}");
    }
}

/// capture.md 7 (move-basic): a diagonal push is refused at no cost, on the
/// map and in a room (80 of 80 each).
#[test]
fn diagonals_are_refused_at_no_cost() {
    let a = base(Platform::C64);
    for at in [map(12, 12), room(4, 4)] {
        let mut g = game(
            &a,
            1,
            PlayerSetup {
                speed: 6,
                ..Default::default()
            },
            at,
        );
        for dir in [Dir::NE, Dir::NW, Dir::SE, Dir::SW] {
            let ev = g.apply(Command::Move { dir });
            assert!(
                !ev.iter().any(|e| matches!(e, Event::Moved { .. })),
                "{dir:?}: {ev:?}"
            );
        }
        assert_eq!(g.players()[0].moves_left, 6, "{at:?}");
    }
}

/// capture.md 10.2 (`refuse`): pushing into a wall on the map or in a room,
/// or into an obstacle, costs nothing (0 px in 20 runs each).
#[test]
fn refused_moves_are_free() {
    let mut a = base(Platform::C64);
    item(&mut a, ROOM, 5, 4, OBSTACLE);
    let speed = PlayerSetup {
        speed: 15,
        ..Default::default()
    };
    // Into the room's west wall from (1,4), and into the obstacle from (4,4).
    for (at, dir) in [(room(1, 4), Dir::W), (room(4, 4), Dir::E)] {
        let mut g = game(&a, 1, speed.clone(), at);
        for _ in 0..3 {
            g.apply(Command::Move { dir });
        }
        assert_eq!(g.players()[0].moves_left, 15, "{at:?} {dir:?}");
    }
}

fn loaded(a: &mut Adventure, load: i32) -> u16 {
    const LOAD: u16 = 71;
    thing(a, LOAD, "LOAD", treasure(load, 1));
    LOAD
}

/// capture.md 7 (`weight`) and 10.3: the next turn's Speed by load, above
/// half the capacity Speed x 2 x (capacity - load) / capacity, rounded
/// down (inventory 3.3). Every reading was the same in all 20 runs.
#[test]
fn weight_slowdown_table() {
    // (Speed, Strength, [(load, next turn's moves)])
    type Row = (u8, u8, &'static [(i32, u8)]);
    let rows: &[Row] = &[
        (
            15,
            10,
            &[
                (400, 15),
                (500, 15),
                (501, 14),
                (600, 12),
                (700, 9),
                (800, 6),
                (1000, 0),
            ],
        ),
        (
            10,
            15,
            &[
                (700, 10),
                (750, 10),
                (751, 9),
                (900, 8),
                (1050, 6),
                (1200, 4),
                (1350, 2),
                (1500, 0),
            ],
        ),
    ];
    for &(speed, strength, table) in rows {
        for &(load, moves) in table {
            let mut a = base(Platform::C64);
            let t = loaded(&mut a, load);
            let mut g = game(
                &a,
                1,
                PlayerSetup {
                    speed,
                    strength,
                    constitution: 10,
                    life_force: 10,
                    possessions: vec![t],
                    ..Default::default()
                },
                map(12, 12),
            );
            assert_eq!(
                next_turn(&mut g),
                moves,
                "Speed {speed} Strength {strength} load {load}"
            );
        }
    }
}
