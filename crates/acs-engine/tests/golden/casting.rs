// SPDX-License-Identifier: GPL-3.0-or-later

//! Casting (magic 4): the cast chance, a failed cast's cost; and the spell
//! actions (magic 2.4, 3): Kill All, summons, Add to Room, Activate All and
//! the "when dropped here" space (inventory 5.7).

use crate::stats::{same_rate, Sweep, SEEDS};
use crate::world::*;
use acs_core::model::{
    Access, Alignment, CustomTrigger, PlacementChoice, SpellAction, SpellActionKind as K, ThingKind,
};
use acs_core::Adventure;
use acs_core::Platform;
use acs_engine::{Command, Dir, Event, Game, PlayerSetup, Who};

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
        at: None,
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

// ---- spell actions (capture.md 10.40; magic 2.4, 3) -------------------------
//
// rr36: T on 4,4 at its first room turn; every spell of ours costs 0 (no
// roll) and every cast asked for a target square, the cursor starting on
// the caster. Each case is one deterministic run.

const BLADE: u16 = 59;
const PUTCI: u16 = 60;
/// Our treasures 61 to 64, what the spaces give.
const T61: u16 = 61;
const T62: u16 = 62;
const T63: u16 = 63;
const T64: u16 = 64;
const KILL_ALL: u16 = 34; // 0:1:59, kill all but owners of the blade
const SUMMON: u16 = 35; // 0:2:5, summon master creature 5
const ADD: u16 = 36; // 0:13:60, add 60 to the room
/// Master creature 5, ZED: an enemy of Speed 3, Constitution 12.
const ZED: u16 = 5;
const PLAIN: u16 = 100; // a space anyone passes
const ADDER: u16 = 101; // space: Add to Room 60
const CHEST: u16 = 102; // obstacle, bump: Add to Room 60
const DOALL: u16 = 103; // space: Activate All
const ON_MOVE: u16 = 104; // custom space, when moved onto: Give 61
const OWNERS: u16 = 105; // custom space, open to owners of (placement): Give 62
const ON_DROP: u16 = 106; // custom space, when dropped here (placement): Give 63
const GIVE64: u16 = 107; // space: Give 64
const POWER5: u16 = 108; // space: Power +5
const SLOT61: u16 = 109; // custom space, when dropped here (placement): Give 61
const SLOT61D: u16 = 110; // the same, destroying the thing

fn act(kind: K, parameter: u16) -> SpellAction {
    SpellAction { kind, parameter }
}

fn invokes(kind: K, parameter: u16) -> ThingKind {
    ThingKind::Space {
        picture: 4,
        access: Access::InvokeSpell {
            action: act(kind, parameter),
        },
    }
}

fn custom(trigger: CustomTrigger, action: SpellAction, destroy: bool) -> ThingKind {
    ThingKind::CustomSpace {
        picture: 4,
        placement_choice: PlacementChoice::Thing,
        trigger,
        action,
        destroy_needed_thing: destroy,
    }
}

/// The base adventure with rr36's things and ZED.
fn magic_kit() -> Adventure {
    let mut a = base(Platform::C64);
    thing(&mut a, BLADE, "BLADE", blade(10));
    for (n, name) in [
        (PUTCI, "PUTCI"),
        (T61, "PUTGI"),
        (T62, "PUTGJ"),
        (T63, "PUTGK"),
        (T64, "PUTGL"),
    ] {
        thing(&mut a, n, name, treasure(1, 1));
    }
    thing(
        &mut a,
        KILL_ALL,
        "KILLALL",
        spell(0, act(K::KillAllButOwnerOf, BLADE)),
    );
    thing(&mut a, SUMMON, "SUMMON", spell(0, act(K::SummonOrBanish, ZED)));
    thing(&mut a, ADD, "ADD", spell(0, act(K::AddToRoom, PUTCI)));
    thing(
        &mut a,
        PLAIN,
        "PLAIN",
        ThingKind::Space {
            picture: 4,
            access: Access::Anyone,
        },
    );
    thing(&mut a, ADDER, "ADDER", invokes(K::AddToRoom, PUTCI));
    thing(
        &mut a,
        CHEST,
        "CHEST",
        ThingKind::Obstacle {
            picture: 7,
            bump_invokes_spell: true,
            action: act(K::AddToRoom, PUTCI),
        },
    );
    thing(&mut a, DOALL, "DOALL", invokes(K::ActivateAll, 0));
    thing(
        &mut a,
        ON_MOVE,
        "ONMOVE",
        custom(CustomTrigger::WhenMovedOnto, act(K::Give, T61), false),
    );
    thing(
        &mut a,
        OWNERS,
        "OWNERS",
        custom(CustomTrigger::OpenToOwnersOf, act(K::Give, T62), false),
    );
    thing(
        &mut a,
        ON_DROP,
        "ONDROP",
        custom(CustomTrigger::WhenDroppedHere, act(K::Give, T63), false),
    );
    thing(&mut a, GIVE64, "GIVE64", invokes(K::Give, T64));
    thing(&mut a, POWER5, "POWER5", invokes(K::ChangePower, 5));
    thing(
        &mut a,
        SLOT61,
        "SLOT",
        custom(CustomTrigger::WhenDroppedHere, act(K::Give, T61), false),
    );
    thing(
        &mut a,
        SLOT61D,
        "SLOTD",
        custom(CustomTrigger::WhenDroppedHere, act(K::Give, T61), true),
    );
    let mut zed = a.regions[0].residents[0].clone();
    zed.in_use = true;
    zed.name = "ZED".into();
    zed.number = ZED;
    zed.strategy.alignment = Alignment::Enemy;
    zed.strategy.aggressive = false;
    zed.speed = 3;
    zed.constitution = 12;
    zed.life_force = 12;
    zed.x = 8;
    zed.y = 2;
    a.creatures[ZED as usize - 1] = zed;
    a
}

/// Puts `thing` with `parameter` on top of ROOM (x, y).
fn put(a: &mut Adventure, x: u8, y: u8, thing: u16, parameter: u8) {
    item(a, ROOM, x, y, thing);
    a.regions[0].rooms[ROOM as usize].items[0].parameter = parameter;
}

/// A Speed 0 peaceful friend of ours in `n` on ROOM (x, y).
fn still(a: &mut Adventure, n: usize, x: u8, y: u8) {
    creature(a, n, x, y, |c| {
        c.speed = 0;
        c.strategy.alignment = Alignment::Friend;
        c.strategy.aggressive = false;
    });
}

/// The harness's T (Speed 6, Wisdom and Power 13) on 4,4 owning `things`.
fn tee(a: &Adventure, things: &[u16]) -> Game {
    game(
        a,
        1,
        PlayerSetup {
            name: "T".into(),
            speed: 6,
            wisdom: 13,
            power: Some(13),
            possessions: things.to_vec(),
            ..Default::default()
        },
        room(4, 4),
    )
}

fn at(g: &mut Game, thing: u16, square: Option<(u8, u8)>) -> Vec<Event> {
    g.apply(Command::Cast {
        thing,
        at: square,
        target: None,
    })
}

fn stack(g: &Game, x: u8, y: u8) -> Vec<u16> {
    g.adventure().regions[0].rooms[ROOM as usize]
        .items
        .iter()
        .filter(|it| (it.x, it.y) == (x, y))
        .map(|it| it.thing)
        .collect()
}

fn given(ev: &[Event]) -> Vec<u16> {
    ev.iter()
        .filter_map(|e| match e {
            Event::Given { who: ME, thing, .. } => Some(*thing),
            _ => None,
        })
        .collect()
}

/// capture.md 10.40 item 1 (`ka`, `kb`; magic 3.1): Kill All but owners of
/// the blade, T owning only the spell; AA (owning the blade) on 8,2, BB on
/// 10,6, CC in the side room, all Speed 0 friends. Cast at T (`ka-c`): T
/// dies, AA, BB and CC untouched; at BB (`ka-bb`): BB alone dies; at AA
/// (`ka-aa`), or by T owning the blade at itself (`kb-c`): nobody dies.
#[test]
fn kill_all_kills_its_target_alone_unless_it_owns_the_thing() {
    let mut a = magic_kit();
    still(&mut a, 0, 8, 2);
    still(&mut a, 1, 10, 6);
    still(&mut a, 2, 3, 3);
    a.regions[0].residents[2].room = SIDE;
    owns(&mut a.regions[0].residents[0], BLADE);
    let died = |ev: &[Event]| {
        ev.iter()
            .filter_map(|e| match e {
                Event::Died { who, .. } => Some(*who),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let lives = |g: &Game| {
        (0..3)
            .map(|n| g.creature(1, n).map(|c| c.life_force))
            .collect::<Vec<_>>()
    };
    let full = vec![Some(20); 3];
    for (name, owner, square, dead) in [
        ("ka-c", false, None, vec![ME]),
        ("ka-bb", false, Some((10, 6)), vec![slot(1)]),
        ("ka-aa", false, Some((8, 2)), vec![]),
        ("kb-c", true, None, vec![]),
    ] {
        let things = if owner {
            vec![KILL_ALL, BLADE]
        } else {
            vec![KILL_ALL]
        };
        let mut g = tee(&a, &things);
        let ev = at(&mut g, KILL_ALL, square);
        assert_eq!(died(&ev), dead, "{name}: {ev:?}");
        let mut want = full.clone();
        if let Some(Who::Creature { slot, .. }) = dead.first() {
            want[*slot] = None;
        }
        assert_eq!(lives(&g), want, "{name}");
        assert_eq!(g.players()[0].alive, dead != [ME], "{name}");
    }
}

/// capture.md 10.40 item 2 (`sa`, `sb15`, `sb16`; magic 3.2): summons of
/// ZED (master creature 5, an enemy of Speed 3, Constitution 12). The
/// creature is made at the cast, in the lowest free slot, a copy of the
/// master record (its alignment, Speed and Constitution), on the target
/// square in the same round: an empty square (`sa-p66`: 5,6), T's own
/// (`sa-p2`: 4,4), a square with a creature on it (`sb15-27`: SB's 2,7,
/// shared); with slots 0 to 14 in use, slot 15 (`sb15-c`: 6,6). It took
/// its first turn in the next round. With all 16 in use (`sb16-c`): the
/// spell fails, no slot changed.
#[test]
fn a_summons_appears_at_once_on_the_target_square() {
    let summoned = |ev: &[Event]| {
        ev.iter().find_map(|e| match e {
            Event::CreatureSummoned {
                slot, creature, at, ..
            } => Some((*slot, *creature, *at)),
            _ => None,
        })
    };
    let first_turn = |ev: &[Event], n: usize| {
        ev.iter()
            .any(|e| matches!(e, Event::CreatureTurn { region: 1, slot, .. } if *slot == n))
    };
    let check = |name: &str, a: &Adventure, square: Option<(u8, u8)>, n: usize, on: (u8, u8)| {
        let mut g = tee(a, &[SUMMON]);
        let ev = at(&mut g, SUMMON, square);
        assert_eq!(summoned(&ev), Some((n, ZED, room(on.0, on.1))), "{name}: {ev:?}");
        let made = ev
            .iter()
            .position(|e| matches!(e, Event::CreatureSummoned { .. }))
            .unwrap();
        let ended = ev
            .iter()
            .position(|e| *e == Event::TurnEnded { player: 0 })
            .expect("the cast ends the turn");
        assert!(made < ended, "{name}: made at the cast");
        assert!(!first_turn(&ev, n), "{name}: no turn in its round: {ev:?}");
        let c = g.creature(1, n).expect("made");
        assert_eq!(
            (c.strategy.alignment, c.speed, c.constitution, c.room),
            (Alignment::Enemy, 3, 12, ROOM),
            "{name}"
        );
        assert!(first_turn(&wait(&mut g), n), "{name}: its turn next round");
    };
    let a = magic_kit();
    check("sa-p66", &a, Some((5, 6)), 0, (5, 6));
    check("sa-p2", &a, None, 0, (4, 4));

    // Slots 0 to 14 in use, SB (slot 14) on 2,7.
    let mut a = magic_kit();
    for n in 0..15 {
        let (x, y) = if n < 13 {
            (n as u8 + 1, 1)
        } else {
            (n as u8 - 12, 7)
        };
        still(&mut a, n, x, y);
    }
    assert_eq!(a.regions[0].residents[14].x, 2);
    check("sb15-c", &a, Some((6, 6)), 15, (6, 6));
    check("sb15-27", &a, Some((2, 7)), 15, (2, 7));

    still(&mut a, 15, 3, 7);
    let mut g = tee(&a, &[SUMMON]);
    let before: Vec<_> = (0..16)
        .map(|n| g.creature(1, n).map(|c| c.name.clone()))
        .collect();
    let ev = at(&mut g, SUMMON, Some((6, 6)));
    assert!(ev.contains(&Event::SummonUnanswered { creature: ZED }), "{ev:?}");
    assert_eq!(summoned(&ev), None);
    let after: Vec<_> = (0..16)
        .map(|n| g.creature(1, n).map(|c| c.name.clone()))
        .collect();
    assert_eq!(before, after, "sb16-c: no slot changed");
    assert_eq!(g.players()[0].power, 13, "sb16-c: cost 0");
}

/// capture.md 10.40 item 3 (`aa`, `ab`, `ac`, `ad`; magic 3.13): Add to
/// Room (thing 60) lands on top of the square where the spell takes
/// effect, one of it: cast at T on 4,4 (`aa-c`), on 4,4; T standing on a
/// space (`ab-c`), on top of the space; cast at the empty 6,6 (`aa-66`),
/// on 6,6; a space adding it when moved onto, on 5,4 (`ac-s2`): T on it,
/// the thing on top there; an obstacle adding it when bumped, on 5,4
/// (`ad-s`): T stayed on 4,4, the thing on top of the obstacle's square.
#[test]
fn add_to_room_lands_on_top_of_the_target_square() {
    let a = magic_kit();
    let count = |g: &Game| {
        g.adventure().regions[0].rooms[ROOM as usize]
            .items
            .iter()
            .filter(|it| it.thing == PUTCI)
            .count()
    };
    let mut g = tee(&a, &[ADD]);
    at(&mut g, ADD, None);
    assert_eq!(stack(&g, 4, 4), [PUTCI], "aa-c");
    assert_eq!(count(&g), 1);

    let mut b = a.clone();
    put(&mut b, 4, 4, PLAIN, 0);
    let mut g = tee(&b, &[ADD]);
    at(&mut g, ADD, None);
    assert_eq!(stack(&g, 4, 4), [PUTCI, PLAIN], "ab-c");

    let mut g = tee(&a, &[ADD]);
    at(&mut g, ADD, Some((6, 6)));
    assert_eq!(stack(&g, 6, 6), [PUTCI], "aa-66");
    assert!(stack(&g, 4, 4).is_empty(), "aa-66");
    assert_eq!(count(&g), 1);

    let mut b = a.clone();
    put(&mut b, 5, 4, ADDER, 0);
    let mut g = tee(&b, &[]);
    g.apply(Command::Move { dir: Dir::E });
    assert_eq!(g.players()[0].at, room(5, 4), "ac-s2");
    assert_eq!(stack(&g, 5, 4), [PUTCI, ADDER], "ac-s2");

    let mut b = a.clone();
    put(&mut b, 5, 4, CHEST, 0);
    let mut g = tee(&b, &[]);
    g.apply(Command::Move { dir: Dir::E });
    assert_eq!(g.players()[0].at, room(4, 4), "ad-s");
    assert_eq!(stack(&g, 5, 4), [PUTCI, CHEST], "ad-s");
    assert_eq!(count(&g), 1);
}

/// capture.md 10.40 item 4 (`acto`, `actn`, `actp`; magic 3.14): on 5,4,
/// top first, a space running Activate All; a custom space "when moved
/// onto" giving 61; one "open to owners of" the blade giving 62; one "when
/// dropped here" (the blade) giving 63; a second Activate All space; a
/// space giving 64. T stepping onto it from 4,4: owning the blade, 61, 62
/// and 64 given; not owning, 61 and 64; 63 never; T on 5,4 either way.
/// `actp` (the last space Power +5 instead): Power 13 to 17, the +5 once
/// with the turn's drift, not twice.
#[test]
fn activate_all_runs_the_stack_below_once() {
    let stacked = |last: u16| {
        let mut a = magic_kit();
        for (t, p) in [
            (last, 0),
            (DOALL, 0),
            (ON_DROP, BLADE as u8),
            (OWNERS, BLADE as u8),
            (ON_MOVE, 0),
            (DOALL, 0),
        ] {
            put(&mut a, 5, 4, t, p);
        }
        a
    };
    let a = stacked(GIVE64);
    for (name, things, want) in [
        ("acto", vec![BLADE], vec![T61, T62, T64]),
        ("actn", vec![], vec![T61, T64]),
    ] {
        let mut g = tee(&a, &things);
        let ev = g.apply(Command::Move { dir: Dir::E });
        assert_eq!(given(&ev), want, "{name}: {ev:?}");
        assert_eq!(g.players()[0].at, room(5, 4), "{name}");
    }

    let a = stacked(POWER5);
    let mut g = tee(&a, &[]);
    let ev = g.apply(Command::Move { dir: Dir::E });
    let changes = ev
        .iter()
        .filter(|e| matches!(e, Event::StatChanged { who: ME, .. }))
        .count();
    assert_eq!(changes, 1, "actp: {ev:?}");
    g.apply(Command::Wait);
    assert_eq!(g.players()[0].power, 17, "actp");
}

/// capture.md 10.40 (`dh`, `dhd`; inventory 5.7): a custom space "when
/// dropped here" for the blade on 5,4, giving 61, T owning the blade; DROP
/// OBJECT, the blade, the cursor east: 61 given and the blade on top of the
/// space (`dh-e`); with the destroy bit (`dhd-e`), 61 given and the blade
/// gone.
#[test]
fn when_dropped_here_gives_and_keeps_the_thing_unless_destroyed() {
    for (name, space, kept) in [("dh-e", SLOT61, true), ("dhd-e", SLOT61D, false)] {
        let mut a = magic_kit();
        put(&mut a, 5, 4, space, BLADE as u8);
        let mut g = tee(&a, &[BLADE]);
        let ev = g.apply(Command::Drop {
            thing: BLADE,
            dir: Some(Dir::E),
        });
        assert_eq!(given(&ev), [T61], "{name}: {ev:?}");
        let want = if kept { vec![BLADE, space] } else { vec![space] };
        assert_eq!(stack(&g, 5, 4), want, "{name}");
        assert!(!g.players()[0].owns(BLADE), "{name}");
    }
}
