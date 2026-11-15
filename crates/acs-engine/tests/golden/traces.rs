// SPDX-License-Identifier: GPL-3.0-or-later

//! Fixed-seed traces: scripted games whose every event, and the game's
//! fingerprint after each command, are kept in `traces/NAME.txt`. A trace
//! that changes means the rules changed: if on purpose, rerun with
//! `ACSVM_BLESS=1` and commit the new trace with the rule change, so the
//! diff shows what the change did to play. Each game must also come back
//! identically from its save file (a replay of seed and command log,
//! `docs/saves.md`).

use crate::world::*;
use acs_core::model::{Possession, SpellAction, SpellActionKind};
use acs_core::{Adventure, Platform};
use acs_engine::save::{fingerprint, SaveFile};
use acs_engine::{Command, Dir, Game, Location, PlayerSetup};
use std::fmt::Write as _;
use std::path::PathBuf;

const BLADE: u16 = 3;
const BOW: u16 = 4;
const GEM: u16 = 1;
const HURT: u16 = 33;
const MEND: u16 = 34;

fn adventure() -> Adventure {
    let mut a = base(Platform::C64);
    thing(&mut a, GEM, "GEM", treasure(1, 50));
    thing(&mut a, BLADE, "BLADE", blade(10));
    thing(&mut a, BOW, "BOW", bow(10, 7));
    let life = |kind, parameter| SpellAction { kind, parameter };
    thing(
        &mut a,
        HURT,
        "HURT",
        spell(10, life(SpellActionKind::ChangeLifeForce, 0x80 | 6)),
    );
    thing(
        &mut a,
        MEND,
        "MEND",
        spell(5, life(SpellActionKind::ChangeLifeForce, 4)),
    );
    item(&mut a, ROOM, 5, 6, GEM);
    a
}

struct Script {
    name: &'static str,
    seed: u64,
    player: PlayerSetup,
    at: Location,
    setup: fn(&mut Adventure),
    commands: Vec<Command>,
}

fn mv(dir: Dir, n: usize) -> impl Iterator<Item = Command> {
    std::iter::repeat_n(Command::Move { dir }, n)
}

fn scripts() -> Vec<Script> {
    let waits = |n| std::iter::repeat_n(Command::Wait, n);
    vec![
        // The map door into ROOM, a pick-up, then a fight with an armed
        // enemy that walks up, and an archer behind it.
        Script {
            name: "duel",
            seed: 7,
            player: PlayerSetup {
                name: "T".into(),
                speed: 6,
                strength: 12,
                constitution: 30,
                life_force: 30,
                possessions: vec![BLADE],
                melee: 40,
                ..Default::default()
            },
            at: Location::World { x: 12, y: 12 },
            setup: |a| {
                creature(a, 1, 10, 4, |c| {
                    c.speed = 3;
                    c.strength = 10;
                    owns(c, BLADE);
                });
                creature(a, 2, 12, 7, |c| {
                    c.speed = 4;
                    c.dexterity = 13;
                    c.possessions = vec![Possession { thing: BOW, count: 1 }];
                });
            },
            commands: mv(Dir::S, 4)
                .chain(mv(Dir::S, 2))
                .chain([Command::Move { dir: Dir::E }])
                .chain(waits(2))
                .chain(mv(Dir::E, 3))
                .chain(waits(10))
                .collect(),
        },
        // Casting on itself and on a creature, through the portal to SIDE
        // and back, idling out turns (drift and healing).
        Script {
            name: "magic",
            seed: 11,
            player: PlayerSetup {
                name: "M".into(),
                speed: 6,
                wisdom: 25,
                power: Some(25),
                constitution: 20,
                life_force: 12,
                possessions: vec![HURT, MEND],
                ..Default::default()
            },
            at: room(7, 4),
            setup: |a| {
                creature(a, 1, 3, 2, |c| {
                    c.speed = 0;
                    c.strategy.aggressive = false;
                });
            },
            commands: [
                Command::Cast {
                    thing: MEND,
                    target: None,
                },
                Command::Cast {
                    thing: HURT,
                    target: Some(slot(1)),
                },
            ]
            .into_iter()
            .chain(mv(Dir::N, 4))
            .chain(waits(2))
            .chain(mv(Dir::W, 1))
            .chain(std::iter::repeat_n(Command::Idle, 18))
            .chain([Command::Cast {
                thing: HURT,
                target: Some(slot(1)),
            }])
            .chain(waits(6))
            .collect(),
        },
    ]
}

fn trace_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/traces")
}

/// Plays `s`, writing each command, its events and the game's fingerprint.
fn play(a: &Adventure, s: &Script) -> (String, Game) {
    let mut g = game(a, s.seed, s.player.clone(), s.at);
    let mut out = String::new();
    writeln!(
        out,
        "# {} seed {} (crates/acs-engine/tests/golden/traces.rs)",
        s.name, s.seed
    )
    .unwrap();
    writeln!(out, "start {}", fingerprint(&g)).unwrap();
    for c in &s.commands {
        writeln!(out, "> {}", serde_json::to_string(c).unwrap()).unwrap();
        for e in g.apply(c.clone()) {
            writeln!(out, "  {}", serde_json::to_string(&e).unwrap()).unwrap();
        }
        writeln!(out, "state {}", fingerprint(&g)).unwrap();
    }
    (out, g)
}

#[test]
fn fixed_seed_traces_replay_identically() {
    let bless = std::env::var_os("ACSVM_BLESS").is_some();
    for s in scripts() {
        let mut a = adventure();
        (s.setup)(&mut a);
        let (text, g) = play(&a, &s);
        // The same script again: the same trace.
        assert_eq!(play(&a, &s).0, text, "{}: two plays differ", s.name);
        // From its save: the same game.
        let save = SaveFile::from_json(&SaveFile::new(&g, &a, &g).to_json()).expect("reads");
        let back = save.restore(&a).expect("restores");
        assert!(!back.diverged, "{}: the save's replay diverged", s.name);
        assert_eq!(fingerprint(&back.game), fingerprint(&g), "{}", s.name);
        let path = trace_dir().join(format!("{}.txt", s.name));
        if bless {
            std::fs::create_dir_all(trace_dir()).unwrap();
            std::fs::write(&path, &text).unwrap();
            continue;
        }
        let kept = std::fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("{}: no trace; run with ACSVM_BLESS=1", path.display()))
            .replace("\r\n", "\n");
        if kept != text {
            let first = kept
                .lines()
                .zip(text.lines())
                .position(|(k, t)| k != t)
                .unwrap_or(kept.lines().count().min(text.lines().count()));
            panic!(
                "{}: the trace changed at line {} (kept / now):\n{}\n{}\nIf the rules changed on purpose, rerun with ACSVM_BLESS=1.",
                s.name,
                first + 1,
                kept.lines().nth(first).unwrap_or("<end>"),
                text.lines().nth(first).unwrap_or("<end>"),
            );
        }
    }
}
