// SPDX-License-Identifier: GPL-3.0-or-later

//! Casting (magic 4): the cast chance, a failed cast's cost; and the spell
//! actions (magic 2.4, 3): Kill All, summons, Add to Room, Activate All and
//! the "when dropped here" space (inventory 5.7); the cast's target, a
//! summons with no target press or into a full region, and the death drop
//! (magic 7.2); magic defense a level a cast, Give not readying, a spell
//! that disappears, drops refused at the walls, creatures casting and
//! using items (magic 6), and what a death loses.

use crate::stats::{same_rate, Sweep, SEEDS};
use crate::world::*;
use acs_core::model::{
    Access, Alignment, Carry, Creature, CustomTrigger, Destination, MagicDefense, PlacementChoice,
    SpellAction, SpellActionKind as K, ThingKind,
};
use acs_core::Adventure;
use acs_core::Platform;
use acs_engine::{CastRefusal, Command, Dir, DropRefusal, Event, Game, Location, PlayerSetup, Stat, Who};

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
const GOLD: u16 = 81;
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

// ---- the cast's target, the summons, Add to Room, the death drop -----------
//
// capture.md 10.42 (rr38): T on 4,4 at its first room turn, as rr36's.
// Every case is one deterministic run, but the full region's summons with
// the random state set.

const DOALL_SPELL: u16 = 37; // 0:12:0, Activate All
const SUMMON20: u16 = 38; // 20:2:5, summon ZED at cost 20
const KILL62: u16 = 39; // 0:1:62, kill all but owners of 62
const SPELL80: u16 = 80; // a spell, lost at a death
const ITEM: u16 = 70; // magic item, used: Add to Room 61
const LURE: u16 = 71; // magic item, picked up or dropped: summon ZED
const SEED: u16 = 72; // magic item, picked up: Add to Room 60
const GIVE61: u16 = 111; // space: Give 61
const CALLER: u16 = 112; // space: summon ZED
const GONG: u16 = 113; // obstacle, bump: summon ZED
const GATE: u16 = 114; // portal to SIDE (2,4), invokes summon ZED
const ARCH: u16 = 115; // portal to SIDE (2,4), invokes Add to Room 60

fn magic_item(action: SpellAction, on_use: bool, on_drop: bool, on_pickup: bool) -> ThingKind {
    ThingKind::MagicItem {
        picture: 3,
        disappears_after_use: false,
        carry: Carry { weight: 1, value: 1 },
        action,
        on_use,
        on_drop,
        on_pickup,
    }
}

fn invoking_portal(action: SpellAction) -> ThingKind {
    ThingKind::Portal {
        picture: 5,
        two_way: false,
        access: Access::InvokeSpell { action },
        destroy_needed_thing: false,
    }
}

/// rr36's kit with rr38's things.
fn rr38() -> Adventure {
    let mut a = magic_kit();
    let summon = act(K::SummonOrBanish, ZED);
    thing(&mut a, DOALL_SPELL, "DOALL", spell(0, act(K::ActivateAll, 0)));
    thing(&mut a, SUMMON20, "TESTSPELL1", spell(20, summon));
    thing(&mut a, KILL62, "KILL62", spell(0, act(K::KillAllButOwnerOf, T62)));
    thing(&mut a, SPELL80, "SPELL80", spell(0, nothing()));
    thing(
        &mut a,
        ITEM,
        "ITEM",
        magic_item(act(K::AddToRoom, T61), true, false, false),
    );
    thing(&mut a, LURE, "LURE", magic_item(summon, false, true, true));
    thing(
        &mut a,
        SEED,
        "SEED",
        magic_item(act(K::AddToRoom, PUTCI), false, false, true),
    );
    thing(&mut a, GIVE61, "GIVE61", invokes(K::Give, T61));
    thing(&mut a, CALLER, "CALLER", invokes(K::SummonOrBanish, ZED));
    thing(
        &mut a,
        GONG,
        "GONG",
        ThingKind::Obstacle {
            picture: 7,
            bump_invokes_spell: true,
            action: summon,
        },
    );
    thing(&mut a, GATE, "GATE", invoking_portal(summon));
    thing(&mut a, ARCH, "ARCH", invoking_portal(act(K::AddToRoom, PUTCI)));
    for n in [13, 14, 50, 51] {
        thing(&mut a, n, "TREASURE", treasure(1, 1));
    }
    thing(&mut a, GOLD, "GOLD", treasure(1, 1));
    a
}

/// Puts the portal `thing` on ROOM (x, y), leading to SIDE (2,4).
fn portal(a: &mut Adventure, x: u8, y: u8, thing: u16) {
    put(a, x, y, thing, 0);
    a.regions[0].rooms[ROOM as usize].items[0].destination = Some(Destination::Room {
        region: 1,
        room: SIDE,
        x: 2,
        y: 4,
    });
}

fn side(x: u8, y: u8) -> Location {
    Location::Room {
        region: 1,
        room: SIDE,
        x,
        y,
    }
}

fn summoned(ev: &[Event]) -> Option<(usize, Location)> {
    ev.iter().find_map(|e| match e {
        Event::CreatureSummoned {
            slot,
            creature: ZED,
            at,
            ..
        } => Some((*slot, *at)),
        _ => None,
    })
}

fn added(ev: &[Event], thing: u16) -> Option<Location> {
    ev.iter().find_map(|e| match e {
        Event::ThingAdded { thing: t, at } if *t == thing => Some(*at),
        _ => None,
    })
}

/// capture.md 10.42 item 1, reach (rr38 `tc-e9`, `tc-e12`, `tc-s4`,
/// `tc-s6`, `tc-n4`, `tc-n6`, `tc-w4`, `tc-w6`; magic 2.4): Add to Room 60
/// cast with the cursor pushed 9 or 12 east, 4 or 6 each other way from
/// 4,4 in the 15 x 10 room: the thing on 13,4, 13,4, 4,8, 4,8, 4,1, 4,1,
/// 1,4, 1,4. Any inside square, 9 away included; the cursor stops at the
/// inside edge, never on a wall square, which the engine refuses at no
/// cost.
#[test]
fn the_cursor_reaches_any_inside_square_and_no_wall() {
    let a = magic_kit();
    for (dx, dy, edge) in [(1, 0, (13, 4)), (0, 1, (4, 8)), (0, -1, (4, 1)), (-1, 0, (1, 4))] {
        let mut g = tee(&a, &[ADD]);
        let ev = at(&mut g, ADD, Some(edge));
        assert_eq!(added(&ev, PUTCI), Some(room(edge.0, edge.1)), "{edge:?}: {ev:?}");
        assert_eq!(stack(&g, edge.0, edge.1), [PUTCI], "{edge:?}");
        // One square further: the wall.
        let wall = ((edge.0 as i8 + dx) as u8, (edge.1 as i8 + dy) as u8);
        let mut g = tee(&a, &[ADD]);
        let ev = at(&mut g, ADD, Some(wall));
        assert!(
            matches!(
                ev[..],
                [Event::CastRefused {
                    reason: CastRefusal::BadTarget,
                    ..
                }]
            ),
            "{wall:?}: {ev:?}"
        );
        assert_eq!((g.players()[0].moves_left, g.players()[0].power), (6, 13));
    }
}

/// capture.md 10.42 item 1, Use Object (rr38 `ta-use`, `ta-use2`; magic
/// 2.4): T owning a magic item doing Add to Room 61; the use asked "move
/// cursor to target" as a cast does: no tap put 61 on T's 4,4, two taps
/// east on 6,4.
#[test]
fn use_object_picks_its_target_as_a_cast_does() {
    let a = rr38();
    for (square, on) in [(None, (4, 4)), (Some((6, 4)), (6, 4))] {
        let mut g = tee(&a, &[ITEM]);
        let ev = g.apply(Command::Use {
            thing: ITEM,
            at: square,
            target: None,
        });
        assert_eq!(added(&ev, T61), Some(room(on.0, on.1)), "{square:?}: {ev:?}");
        assert_eq!(stack(&g, on.0, on.1), [T61], "{square:?}");
    }
}

/// capture.md 10.42 item 1, Activate All cast (rr38 `tb-c`, `ta-sel`,
/// `ta-actc`; magic 2.5, 3.14): no target prompt; cast by T standing on a
/// stack it had arrived on and not set off (a space giving 61, then a space
/// changing Power by +5): 61 given and Power 13 to 17, the turn over. On an
/// empty square nothing happened.
#[test]
fn an_activate_all_cast_runs_the_casters_own_square() {
    let mut a = rr38();
    put(&mut a, 4, 4, POWER5, 0);
    put(&mut a, 4, 4, GIVE61, 0);
    // The target, were one given, is ignored.
    for square in [None, Some((6, 6))] {
        let mut g = tee(&a, &[DOALL_SPELL]);
        let ev = at(&mut g, DOALL_SPELL, square);
        assert_eq!(given(&ev), [T61], "tb-c {square:?}: {ev:?}");
        assert!(
            ev.contains(&Event::TurnEnded { player: 0 }),
            "tb-c: the turn over"
        );
        assert_eq!(g.players()[0].power, 17, "tb-c {square:?}");
    }
    let a = rr38();
    let mut g = tee(&a, &[DOALL_SPELL]);
    let ev = at(&mut g, DOALL_SPELL, None);
    assert!(given(&ev).is_empty(), "ta-sel: {ev:?}");
    // Only the cast's own trigger: nothing under T to run.
    assert!(
        !ev.iter().any(|e| matches!(
            e,
            Event::SpellTriggered { thing, .. } if *thing != DOALL_SPELL
        ) || matches!(e, Event::StatChanged { .. })),
        "ta-sel: {ev:?}"
    );
    assert_eq!(g.players()[0].power, 13, "ta-sel");
}

/// capture.md 10.42 item 1, a summons with no target press (rr38 `sn-sp`,
/// `sn-ob`, `sn-po`, `sn-pk2`, `sn-dro`, `sn-drs`; magic 3.2): ZED on 5,4,
/// the space stepped onto (T's square too); on 3,4, the bumped obstacle's
/// own square, T staying on 4,4; through the portal on 4,3, ZED on 4,3 of
/// the first room, T on the side room's 2,4; the item picked up from 4,5
/// (placed with 0, T not owning it), ZED on 4,5, T on 4,4; the item dropped
/// on T's 4,4 or on 4,5, ZED on the drop square. Each time the lowest free
/// slot, made at once.
#[test]
fn a_summons_with_no_target_press_lands_on_the_setting_off_things_square() {
    let mut a = rr38();
    put(&mut a, 5, 4, CALLER, 0);
    put(&mut a, 3, 4, GONG, 0);
    portal(&mut a, 4, 3, GATE);
    let check = |name: &str, g: &Game, ev: &[Event], on: Location, t: Location| {
        assert_eq!(summoned(ev), Some((0, on)), "{name}: {ev:?}");
        assert_eq!(g.players()[0].at, t, "{name}");
    };
    for (name, dir, on, t) in [
        ("sn-sp", Dir::E, room(5, 4), room(5, 4)),
        ("sn-ob", Dir::W, room(3, 4), room(4, 4)),
        ("sn-po", Dir::N, room(4, 3), side(2, 4)),
    ] {
        let mut g = tee(&a, &[]);
        let ev = g.apply(Command::Move { dir });
        check(name, &g, &ev, on, t);
    }

    let mut b = a.clone();
    put(&mut b, 4, 5, LURE, 0);
    let mut g = tee(&b, &[]);
    let ev = g.apply(Command::Move { dir: Dir::S });
    assert!(ev.contains(&Event::PickedUp {
        player: 0,
        thing: LURE
    }));
    check("sn-pk2", &g, &ev, room(4, 5), room(4, 4));

    for (name, dir, on) in [("sn-dro", None, room(4, 4)), ("sn-drs", Some(Dir::S), room(4, 5))] {
        let mut g = tee(&a, &[LURE]);
        let ev = g.apply(Command::Drop { thing: LURE, dir });
        check(name, &g, &ev, on, room(4, 4));
    }
}

/// capture.md 10.42 item 1, a summons of cost 20 into a full region (rr38
/// `sf-r0`, `sf-r85`, `sf-r170`, and `sg-*` with 15 slots in use; magic
/// 3.2): T of Power 31, the region's 16 slots in use: "a few sparks fizzle
/// but TESTSPELL1 fails", no prompt, no slot changed, Power 31 to 11, at
/// every random state. With 15 in use the same states gave two successes
/// (the creature in slot 15 on T's 4,4) and one failed roll, Power 11 each
/// time. Exact per seed; both outcomes of the free region's roll seen.
#[test]
fn a_summons_into_a_full_region_spends_its_cost_and_fizzles() {
    let region = |full: bool| {
        let mut a = rr38();
        for n in 0..15 {
            let (x, y) = if n < 13 {
                (n as u8 + 1, 1)
            } else {
                (n as u8 - 12, 7)
            };
            still(&mut a, n, x, y);
        }
        if full {
            still(&mut a, 15, 3, 7);
        }
        a
    };
    let names = |g: &Game| {
        (0..16)
            .map(|n| g.creature(1, n).map(|c| c.name.clone()))
            .collect::<Vec<_>>()
    };
    let mut seen = [false; 2];
    for full in [true, false] {
        let a = region(full);
        for seed in 0..32 {
            let mut g = game(
                &a,
                seed,
                PlayerSetup {
                    name: "T".into(),
                    speed: 6,
                    // Wisdom 11, so no drift moves the 11 left.
                    wisdom: 11,
                    possessions: vec![SUMMON20],
                    ..Default::default()
                },
                room(4, 4),
            );
            g.set_power(0, 31);
            let before = names(&g);
            let ev = at(&mut g, SUMMON20, None);
            let success = ev
                .iter()
                .find_map(|e| match e {
                    Event::SpellCast {
                        cost: 20, success, ..
                    } => Some(*success),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("full {full} seed {seed}: no cast: {ev:?}"));
            assert_eq!(g.players()[0].power, 11, "full {full} seed {seed}");
            assert!(ev.contains(&Event::TurnEnded { player: 0 }));
            if full {
                assert!(!success, "sf seed {seed}: {ev:?}");
                assert!(
                    ev.contains(&Event::SummonUnanswered { creature: ZED }),
                    "sf seed {seed}"
                );
                assert_eq!(summoned(&ev), None);
                assert_eq!(names(&g), before, "sf seed {seed}: no slot changed");
            } else {
                seen[success as usize] = true;
                let want = success.then_some((15, room(4, 4)));
                assert_eq!(summoned(&ev), want, "sg seed {seed}: {ev:?}");
            }
        }
    }
    assert_eq!(seen, [true, true], "sg: a success and a failed roll");
}

/// capture.md 10.42 item 4 (rr38 `ar-wall`, `ar-po`, `ar-pk2`; magic 3.13):
/// Add to Room 60 run by an obstacle on the wall line 0,4 bumped from 1,4:
/// 60 on 1,4, T's own square; by a portal on 4,5 to the side room: T on
/// the side room's 2,4, 60 on 4,5, the portal's square in the room left;
/// by our item on 5,4 when picked up (placed with 0, T not owning it): 60
/// on 5,4, where the item lay.
#[test]
fn add_to_room_from_a_wall_obstacle_a_portal_and_a_pick_up() {
    let mut a = rr38();
    put(&mut a, 0, 4, CHEST, 0);
    let mut g = tee(&a, &[]);
    g.place(0, room(1, 4));
    let ev = g.apply(Command::Move { dir: Dir::W });
    assert_eq!(added(&ev, PUTCI), Some(room(1, 4)), "ar-wall: {ev:?}");
    assert_eq!(stack(&g, 1, 4), [PUTCI], "ar-wall");
    assert_eq!(stack(&g, 0, 4), [CHEST], "ar-wall");
    assert_eq!(g.players()[0].at, room(1, 4), "ar-wall");

    let mut a = rr38();
    portal(&mut a, 4, 5, ARCH);
    let mut g = tee(&a, &[]);
    let ev = g.apply(Command::Move { dir: Dir::S });
    assert_eq!(g.players()[0].at, side(2, 4), "ar-po");
    assert_eq!(added(&ev, PUTCI), Some(room(4, 5)), "ar-po: {ev:?}");
    assert_eq!(stack(&g, 4, 5), [PUTCI, ARCH], "ar-po");

    let mut a = rr38();
    put(&mut a, 5, 4, SEED, 0);
    let mut g = tee(&a, &[]);
    let ev = g.apply(Command::Move { dir: Dir::E });
    assert!(ev.contains(&Event::PickedUp {
        player: 0,
        thing: SEED
    }));
    assert_eq!(added(&ev, PUTCI), Some(room(5, 4)), "ar-pk2: {ev:?}");
    assert_eq!(stack(&g, 5, 4), [PUTCI], "ar-pk2");
    assert_eq!(g.players()[0].at, room(4, 4), "ar-pk2");
    assert!(!g.players()[0].owns(SEED), "ar-pk2: placed with 0");
}

/// capture.md 10.42 item 2 (rr38 `dd-aa`, `dd-t`; magic 7.2): Kill All but
/// owners of 62 cast by T, who owns the blade 59, treasures 13 and 14, our
/// magic item 70, the spell 80 and 50 gold; AA on 6,4 owns the blade,
/// treasures 50 and 51, item 70, the spell 80 and 50 gold. Cast at AA: 6,4
/// held, top first, 70, 59, 51, 50, the gold; cast at T's own square, T
/// dies and 4,4 held 70, 59, 14, 13, the gold. The magic item by thing
/// number with the rest, the spell lost, the gold under all.
#[test]
fn a_deaths_drop_puts_magic_items_by_number_and_the_gold_under_all() {
    let mut a = rr38();
    still(&mut a, 0, 6, 4);
    let aa = &mut a.regions[0].residents[0];
    for t in [BLADE, 50, 51, ITEM, SPELL80] {
        owns(aa, t);
    }
    aa.wealth = 50;
    for (name, square, on, want) in [
        ("dd-aa", Some((6, 4)), (6, 4), [ITEM, BLADE, 51, 50, GOLD]),
        ("dd-t", None, (4, 4), [ITEM, BLADE, 14, 13, GOLD]),
    ] {
        let mut g = game(
            &a,
            1,
            PlayerSetup {
                name: "T".into(),
                speed: 6,
                wisdom: 13,
                power: Some(13),
                gold: 50,
                possessions: vec![KILL62, BLADE, 13, 14, ITEM, SPELL80],
                ..Default::default()
            },
            room(4, 4),
        );
        let ev = at(&mut g, KILL62, square);
        assert!(
            ev.iter().any(|e| matches!(e, Event::Died { .. })),
            "{name}: {ev:?}"
        );
        assert_eq!(stack(&g, on.0, on.1), want, "{name}");
        let pile = g.adventure().regions[0].rooms[ROOM as usize]
            .items
            .iter()
            .find(|it| (it.x, it.y, it.thing) == (on.0, on.1, GOLD))
            .and_then(|it| it.gold);
        assert_eq!(pile, Some(50), "{name}: the gold as one pile");
    }
}

// ---- magic defense, Give, disappearing, the walls, creatures casting ------
//
// capture.md 10.43 (rr39): T on 4,4 at its first room turn, as rr36's; a
// retired player joining has its Life Force set to its Constitution (40)
// and its Power to its Wisdom (13).

const MAIL: u16 = 40;
const BOW: u16 = 41;
const INC_MD: u16 = 42; // 0:3:3, increase magic defense up to 3
const GIVE_BLADE: u16 = 43; // 0:9:59, give the blade
const WALL: u16 = 116; // a custom obstacle
/// rr39 `cc`'s spells of cost 0 and item.
const HURT: u16 = 80; // Life Force -5
const CALL: u16 = 79; // summon ZED
const HELP: u16 = 78; // Life Force +5
const ROD: u16 = 70; // magic item, used: Life Force -3

/// rr39's T: Constitution and Life Force 40, Speed 6, Wisdom and Power 13.
fn t40(things: &[u16]) -> PlayerSetup {
    PlayerSetup {
        name: "T".into(),
        speed: 6,
        constitution: 40,
        life_force: 40,
        wisdom: 13,
        power: Some(13),
        possessions: things.to_vec(),
        ..Default::default()
    }
}

/// capture.md 10.43 item 3 (rr39 `md-c`, `md-c1`, `gv-c`, `gv-cb`; magic
/// 3.3, 3.9): spells of cost 0 cast at T. Increase Magic Defense to 3: from
/// 0 it gave 1, from 1 (poked) 2: one level a cast. Give the blade: T
/// empty-handed owned it and held nothing; T holding the bow owned it and
/// held the bow still. One run each.
#[test]
fn a_magic_defense_cast_raises_one_level_and_a_given_weapon_is_not_readied() {
    use MagicDefense as M;
    let mut a = magic_kit();
    thing(&mut a, INC_MD, "INCMD", spell(0, act(K::IncreaseMagicDefense, 3)));
    thing(&mut a, GIVE_BLADE, "GIVEBLADE", spell(0, act(K::Give, BLADE)));
    thing(&mut a, BOW, "BOW", bow(5, 9));
    for (name, from, to) in [
        ("md-c", M::None, M::NonMagicHalfDamage),
        ("md-c1", M::NonMagicHalfDamage, M::OnlyMagicDamages),
    ] {
        let mut g = game(
            &a,
            1,
            PlayerSetup {
                magic_defense: from,
                ..t40(&[INC_MD])
            },
            room(4, 4),
        );
        let ev = at(&mut g, INC_MD, None);
        assert_eq!(g.players()[0].magic_defense, to, "{name}: {ev:?}");
    }
    for (name, held) in [("gv-c", None), ("gv-cb", Some(BOW))] {
        let mut g = game(&a, 1, t40(&[GIVE_BLADE, BOW]), room(4, 4));
        if let Some(w) = held {
            g.apply(Command::Ready { thing: w });
        }
        let ev = at(&mut g, GIVE_BLADE, None);
        assert_eq!(given(&ev), [BLADE], "{name}: {ev:?}");
        let p = &g.players()[0];
        assert!(p.owns(BLADE), "{name}");
        assert_eq!(p.weapon, held, "{name}: the given blade not readied");
    }
}

/// capture.md 10.43 item 4 (rr39 `dsp-r0` .. `dsp-r224`; magic 3): our
/// spell 80, Life Force +1 at cost 60 with the "disappears" bit, T of Power
/// 13: a 19% cast. Of eight runs, the random state set to 0, 32, .., 224
/// before the menu, one worked (Life Force 40 to 41, spell 80 gone) and
/// seven failed, the Power spent and spell 80 kept: 8 of 8 fit. The rule
/// is exact on every seed; the one in eight, eight spaced states, a sample.
#[test]
fn a_spell_that_disappears_goes_only_after_a_working_cast() {
    let mut a = magic_kit();
    thing(
        &mut a,
        SPELL80,
        "SPELL80",
        ThingKind::MagicSpell {
            disappears_after_use: true,
            power: 60,
            action: act(K::ChangeLifeForce, 1),
        },
    );
    let worked = crate::world::count(SEEDS, |seed| {
        let mut g = game(&a, seed, t40(&[SPELL80]), room(4, 4));
        let ev = at(&mut g, SPELL80, None);
        let success = ev
            .iter()
            .find_map(|e| match e {
                Event::SpellCast { success, .. } => Some(*success),
                _ => None,
            })
            .unwrap_or_else(|| panic!("seed {seed}: no cast: {ev:?}"));
        let gone = ev.contains(&Event::SpellGone {
            who: ME,
            thing: SPELL80,
        });
        assert_eq!(gone, success, "seed {seed}: {ev:?}");
        assert_eq!(g.players()[0].owns(SPELL80), !success, "seed {seed}");
        assert_eq!(life(&g), 40 + u8::from(success), "seed {seed}");
        // The 13 spent either way, then the next turn's drift.
        assert_eq!(g.players()[0].power, 1, "seed {seed}");
        success
    });
    same_rate("dsp: casts that worked", Sweep::Sample, 1, 8, worked, SEEDS);
}

/// capture.md 10.43 item 6 (rr39 `wp-po`, `wp-dco`, `wp-dpo`, `dh2-e`;
/// magic 3.13, inventory 5.4, 5.7): T owning the blade; on the wall square
/// 0,4 a portal of ours to the side room doing Add to Room 60, our custom
/// obstacle on 4,5. T walked to 1,4 and west through the wall portal: on
/// the side room's 2,4, 60 on 1,4, its square before the door. The drop
/// cursor pushed toward the custom obstacle, or from 1,4 toward the wall
/// portal, did not move, and the blade landed on T's own square: the
/// engine refuses those squares at no cost, and the drop then goes on T's
/// square. `dh2`: T owning the blade and our mail, on 5,4 a custom space
/// "when dropped here" for the blade giving 61: the mail dropped there lay
/// on top of the space, 61 not given. One run each.
#[test]
fn drops_refused_at_a_custom_obstacle_and_a_wall_portal_and_its_add_to_room() {
    let mut a = rr38();
    portal(&mut a, 0, 4, ARCH);
    thing(
        &mut a,
        WALL,
        "WALL",
        ThingKind::CustomObstacle {
            picture: 7,
            action: nothing(),
        },
    );
    put(&mut a, 4, 5, WALL, 0);

    // `wp-po`
    let mut g = tee(&a, &[BLADE]);
    for _ in 0..4 {
        g.apply(Command::Move { dir: Dir::W });
    }
    assert_eq!(g.players()[0].at, side(2, 4), "wp-po");
    assert_eq!(stack(&g, 1, 4), [PUTCI], "wp-po");
    assert_eq!(stack(&g, 0, 4), [ARCH], "wp-po");

    // `wp-dco`, `wp-dpo`
    for (name, from, dir, why) in [
        ("wp-dco", (4, 4), Dir::S, DropRefusal::Thing { thing: WALL }),
        ("wp-dpo", (1, 4), Dir::W, DropRefusal::Wall),
    ] {
        let mut g = tee(&a, &[BLADE]);
        g.place(0, room(from.0, from.1));
        let ev = g.apply(Command::Drop {
            thing: BLADE,
            dir: Some(dir),
        });
        assert_eq!(
            ev,
            [Event::DropRefused {
                player: 0,
                thing: BLADE,
                reason: why
            }],
            "{name}"
        );
        assert_eq!(g.players()[0].moves_left, 6, "{name}: no cost");
        g.apply(Command::Drop {
            thing: BLADE,
            dir: None,
        });
        assert_eq!(stack(&g, from.0, from.1), [BLADE], "{name}: on T's square");
    }

    // `dh2-e`
    let mut a = magic_kit();
    thing(&mut a, MAIL, "MAIL", mail(2, false));
    put(&mut a, 5, 4, SLOT61, BLADE as u8);
    let mut g = tee(&a, &[BLADE, MAIL]);
    let ev = g.apply(Command::Drop {
        thing: MAIL,
        dir: Some(Dir::E),
    });
    assert!(given(&ev).is_empty(), "dh2-e: {ev:?}");
    assert_eq!(stack(&g, 5, 4), [MAIL, SLOT61], "dh2-e");
}

fn mail(power: u8, once: bool) -> ThingKind {
    ThingKind::Armor {
        picture: 8,
        disappears_after_use: once,
        carry: Carry { weight: 5, value: 10 },
        stats: acs_core::model::WeaponStats {
            power,
            range: None,
            break_chance: 0,
            attack_adjustment: 0,
            usable_by_anyone: true,
            magic: false,
        },
    }
}

/// Each turn of the creature in slot `n`: from its `CreatureTurn` to the
/// next individual's turn.
fn turns_of(ev: &[Event], n: usize) -> Vec<&[Event]> {
    ev.iter()
        .enumerate()
        .filter(|(_, e)| matches!(e, Event::CreatureTurn { region: 1, slot, .. } if *slot == n))
        .map(|(s, _)| {
            let end = ev[s + 1..]
                .iter()
                .position(|e| matches!(e, Event::CreatureTurn { .. } | Event::TurnStarted { .. }))
                .map_or(ev.len(), |k| s + 1 + k);
            &ev[s..end]
        })
        .collect()
}

/// rr39 `cc`: EE in slot 0 on 10,4, six squares from T, an aggressive
/// enemy of Wisdom 20, Power 29 (at the snapshot) and Speed 3, owning
/// `owned`; then `f`. The spells cost 0.
fn cc(owned: &[u16], f: impl FnOnce(&mut Creature)) -> Adventure {
    let mut a = magic_kit();
    thing(&mut a, HURT, "HURT", spell(0, act(K::ChangeLifeForce, 0x85)));
    thing(&mut a, CALL, "CALL", spell(0, act(K::SummonOrBanish, ZED)));
    thing(&mut a, HELP, "HELP", spell(0, act(K::ChangeLifeForce, 5)));
    thing(
        &mut a,
        ROD,
        "ROD",
        magic_item(act(K::ChangeLifeForce, 0x83), true, false, false),
    );
    creature(&mut a, 0, 10, 4, |c| {
        c.name = "EE".into();
        c.speed = 3;
        c.wisdom = 20;
        c.power = 29;
        c.weapon = 0;
        c.possessions.clear();
        for &t in owned {
            owns(c, t);
        }
        f(c);
    });
    a
}

/// EE's first three turns, T waiting on 4,4.
fn cc_turns(a: &Adventure, seed: u64) -> (Game, Vec<Vec<Event>>) {
    let mut g = game(a, seed, t40(&[]), room(4, 4));
    let mut ev = Vec::new();
    for _ in 0..8 {
        if turns_of(&ev, 0).len() >= 3 {
            break;
        }
        ev.extend(wait(&mut g));
    }
    let turns: Vec<Vec<Event>> = turns_of(&ev, 0).into_iter().take(3).map(|t| t.to_vec()).collect();
    assert_eq!(turns.len(), 3, "seed {seed}: EE's turns");
    (g, turns)
}

fn steps_of(turn: &[Event]) -> usize {
    turn.iter()
        .filter(|e| matches!(e, Event::CreatureMoved { slot: 0, .. }))
        .count()
}

/// T's Life Force changes in `turn`: (from, to).
fn t_life(turn: &[Event]) -> Vec<(u8, u8)> {
    turn.iter()
        .filter_map(|e| match *e {
            Event::StatChanged {
                who: ME,
                stat: Stat::LifeForce,
                from,
                to,
            } => Some((from, to)),
            _ => None,
        })
        .collect()
}

/// capture.md 10.43 item 7, the spell (rr39 `cc-s5`, `cc-p19`, `cc-p10`,
/// `cc-p15`, `run mem`, 2 states, three EE turns each; magic 6.3): EE
/// owning the Life Force -5 spell cast it at T on each of its turns from
/// six squares off, never moving, its moves 3 to 0 at the cast. Its Power
/// is the one after the turn's drift toward Wisdom 20: from 29 and 19
/// (drifted to 20) it cast; from 10 and 15 (drifted to 11, 16) it did not,
/// walking up to T. Exact.
#[test]
fn a_smart_creature_casts_at_its_goal_anywhere_in_the_room() {
    for (name, power, casts) in [
        ("cc-s5", 29, true),
        ("cc-p19", 19, true),
        ("cc-p10", 10, false),
        ("cc-p15", 15, false),
    ] {
        let a = cc(&[HURT], |c| c.power = power);
        for seed in 0..4 {
            let (_, turns) = cc_turns(&a, seed);
            let cast = |t: &[Event]| {
                t.iter().any(
                    |e| matches!(e, Event::SpellCast { by, thing: HURT, target: ME, .. } if *by == slot(0)),
                )
            };
            if casts {
                for (n, turn) in turns.iter().enumerate() {
                    assert!(cast(turn), "{name} seed {seed} turn {n}: {turn:?}");
                    assert_eq!(steps_of(turn), 0, "{name} seed {seed} turn {n}");
                    let fell = t_life(turn);
                    assert!(
                        matches!(fell[..], [(from, to)] if to + 5 == from),
                        "{name} seed {seed} turn {n}: {fell:?}"
                    );
                }
            } else {
                let turn = &turns[0];
                assert!(!cast(turn), "{name} seed {seed}: {turn:?}");
                assert!(steps_of(turn) > 0, "{name} seed {seed}: walked up");
            }
        }
    }
}

/// capture.md 10.43 item 7, fitting and the summons (rr39 `cc-fr`,
/// `cc-sum`, `cc-fsum`; magic 6.2, 6.4): EE a peaceful friend owning only
/// the Life Force +5: T's Life Force 40, 45, 50, 55 on EE's three turns.
/// The enemy owning only the summons: ZED made in slot 1 on EE's turn, EE
/// not moving; the friend owning only the summons: ZED made as well.
/// (Where ZED lands, beside T in the one run read, is not settled: not
/// compared.) Exact.
#[test]
fn creatures_cast_helpful_spells_and_summons_at_a_friend_and_summons_at_a_foe() {
    let friend = |c: &mut Creature| {
        c.strategy.alignment = Alignment::Friend;
        c.strategy.aggressive = false;
    };
    for seed in 0..4 {
        let a = cc(&[HELP], friend);
        let (g, turns) = cc_turns(&a, seed);
        let lives: Vec<_> = turns.iter().flat_map(|t| t_life(t)).collect();
        assert_eq!(lives, [(40, 45), (45, 50), (50, 55)], "cc-fr seed {seed}");
        assert_eq!(life(&g), 55, "cc-fr seed {seed}");

        for (name, f) in [("cc-sum", None), ("cc-fsum", Some(friend))] {
            let a = cc(&[CALL], |c| {
                if let Some(f) = f {
                    f(c)
                }
            });
            let (_, turns) = cc_turns(&a, seed);
            let turn = &turns[0];
            assert!(
                turn.iter().any(|e| matches!(
                    e,
                    Event::SpellCast { by, thing: CALL, success: true, .. } if *by == slot(0)
                )),
                "{name} seed {seed}: {turn:?}"
            );
            assert_eq!(summoned(turn).map(|s| s.0), Some(1), "{name} seed {seed}");
            assert_eq!(steps_of(turn), 0, "{name} seed {seed}: EE stayed");
        }
    }
}

/// capture.md 10.43 item 7, magic items and "half power" (rr39 `cc-i30`,
/// `cc-i10`, `cc-i9`; magic 6.5): EE owning only our item, used, Life
/// Force -3. From Power 29 (28 after drift): T -3, EE's Power -2, its
/// moves 3 to 1 at the use and one step with the last, at its next turns
/// the same. From 10 (Power 11, 10, 11 at its three turns) and from 9
/// (10, 11, 10): used at every 11, never at 10, walking instead. So with
/// Wisdom 20 an item is used above half the Wisdom, the use costs two
/// moves and the turn goes on. Exact.
#[test]
fn a_creature_uses_an_item_above_half_its_wisdom_for_two_moves() {
    for (name, power, uses) in [
        ("cc-i30", 29, [true, true, true]),
        ("cc-i10", 10, [true, false, true]),
        ("cc-i9", 9, [false, true, false]),
    ] {
        let a = cc(&[ROD], |c| c.power = power);
        for seed in 0..4 {
            let (_, turns) = cc_turns(&a, seed);
            for (n, (turn, &used)) in turns.iter().zip(&uses).enumerate() {
                let use_ = turn
                    .iter()
                    .any(|e| matches!(e, Event::ItemUsed { by, thing: ROD, target: ME } if *by == slot(0)));
                assert_eq!(use_, used, "{name} seed {seed} turn {n}: {turn:?}");
                if used {
                    assert!(
                        matches!(t_life(turn)[..], [(from, to)] if to + 3 == from),
                        "{name} seed {seed} turn {n}: {turn:?}"
                    );
                    assert_eq!(steps_of(turn), 1, "{name} seed {seed} turn {n}: the last move");
                } else {
                    assert!(steps_of(turn) > 0, "{name} seed {seed} turn {n}: walked");
                }
            }
        }
    }
}

/// capture.md 10.43 item 10 (rr39 `bl0-aa`, `bln-aa`, `blb-aa`, `bl0-t`;
/// magic 7.2, characters 7.4): AA, a Speed 0 friend on 6,4, owning and
/// holding our blade and owning our item 70, which acts when dropped: Add
/// to Room 61. T casts Kill All but owners of 62 at AA. The blade of weight
/// 0 usable by anyone: 6,4 held 70, 59; of weight 5, owner only: 70 only;
/// both marks: 70 only. Cast at T's own square, T owning item 70: T died
/// and 4,4 held 70 only. No 61 in any run: drop spells do not go off at a
/// death. One run each.
#[test]
fn at_a_death_owner_only_weapons_are_lost_and_drop_spells_stay_quiet() {
    let kit = |weight: i32, anyone: bool| {
        let mut a = rr38();
        thing(
            &mut a,
            BLADE,
            "BLADE",
            ThingKind::MeleeWeapon {
                picture: 6,
                disappears_after_use: false,
                carry: Carry { weight, value: 10 },
                stats: acs_core::model::WeaponStats {
                    power: 10,
                    range: None,
                    break_chance: 0,
                    attack_adjustment: 0,
                    usable_by_anyone: anyone,
                    magic: false,
                },
            },
        );
        thing(
            &mut a,
            ITEM,
            "ITEM",
            magic_item(act(K::AddToRoom, T61), false, true, false),
        );
        still(&mut a, 0, 6, 4);
        let aa = &mut a.regions[0].residents[0];
        owns(aa, BLADE);
        owns(aa, ITEM);
        aa.weapon = BLADE;
        a
    };
    for (name, weight, anyone, want) in [
        ("bl0-aa", 0, true, vec![ITEM, BLADE]),
        ("bln-aa", 5, false, vec![ITEM]),
        ("blb-aa", 0, false, vec![ITEM]),
    ] {
        let a = kit(weight, anyone);
        let mut g = game(&a, 1, t40(&[KILL62]), room(4, 4));
        let ev = at(&mut g, KILL62, Some((6, 4)));
        assert!(
            ev.iter()
                .any(|e| matches!(e, Event::Died { who, .. } if *who == slot(0))),
            "{name}: {ev:?}"
        );
        assert_eq!(stack(&g, 6, 4), want, "{name}");
        assert_eq!(added(&ev, T61), None, "{name}: no drop spell");
    }
    let a = kit(0, true);
    let mut g = game(&a, 1, t40(&[KILL62, ITEM]), room(4, 4));
    let ev = at(&mut g, KILL62, None);
    assert!(
        ev.iter().any(|e| matches!(e, Event::Died { who: ME, .. })),
        "bl0-t: {ev:?}"
    );
    assert_eq!(stack(&g, 4, 4), [ITEM], "bl0-t");
    assert_eq!(added(&ev, T61), None, "bl0-t: no drop spell");
}
