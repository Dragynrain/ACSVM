// SPDX-License-Identifier: GPL-3.0-or-later

//! Movement and weight (movement 2-3, 5-6, inventory 2-5): the Speed survey,
//! the weight slowdown and capacity, refused moves and the door's cost,
//! spaces on arrival, drops, readying armor, gold piles and the success
//! stack (winning 1). All deterministic.

use crate::world::*;
use acs_core::model::{
    Access, Alignment, Carry, CustomTrigger, Destination, PlacementChoice, RoomItem, SpellAction,
    SpellActionKind, ThingKind, WeaponStats,
};
use acs_core::{Adventure, Platform};
use acs_engine::{Command, Dir, Event, Game, Location, PlayerSetup, Rules, Slot};

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

/// capture.md 10.39 item 4 (rr35 `c4`, `capcheck.py`; inventory 3.2): T of
/// Speed 15 in the room owning thing 71 of weight 500, its moves read at
/// each turn's refill with the Life Force at that moment. `c-s9l20` ..
/// `c-s9l30` (Strength 9, Constitution 20, Life Force 20, 21, 22, 25, 30;
/// the Life Force never moved): moves 13, 15, 15, 15, 15. `c-s12c40`,
/// `c-s12c41` (Strength 12, Constitution 40 or 41, healing from wounds of
/// 13 down to 0): moves 5, 5, 8, 8, 11, 11, 13, 13, 15 .. for wounds 13,
/// 12, .., each odd wound as the even one below it; 169 turn starts, the
/// wound term halved rounding down fitting all. Deterministic: every turn
/// start here must read so. This test: the `c-s9` series.
#[test]
fn capacity_with_life_force_above_constitution() {
    let (a, t) = capacity_kit();
    for (lf, moves) in [(20, 13), (21, 15), (22, 15), (25, 15), (30, 15)] {
        let mut g = game(&a, 1, t(9, 20, lf), room(4, 4));
        g.set_life_force(0, lf);
        for turn in 0..3 {
            assert_eq!(next_turn(&mut g), moves, "c-s9l{lf} turn {turn}");
            assert_eq!(life(&g), lf, "c-s9l{lf}: the Life Force never moved");
        }
    }
}

/// `c4`'s T: Speed 15 owning the load of 500, Strength, Constitution and
/// Life Force as given.
fn capacity_kit() -> (Adventure, impl Fn(u8, u8, u8) -> PlayerSetup) {
    let mut a = base(Platform::C64);
    let load = loaded(&mut a, 500);
    let t = move |strength: u8, con: u8, lf: u8| PlayerSetup {
        speed: 15,
        strength,
        constitution: con,
        life_force: lf,
        possessions: vec![load],
        ..Default::default()
    };
    (a, t)
}

/// capture.md 10.39 item 4, the `c-s12c40` and `c-s12c41` series (above):
/// the moves at each turn start by the wound then, the Life Force at the
/// refill, after the top-of-turn die's heal (read before it, at the turn
/// byte's change, 12 of 63 fit), as inventory 3.4 has it.
#[test]
fn capacity_at_the_refill_by_odd_wounds() {
    let (a, t) = capacity_kit();
    const SERIES: [u8; 14] = [15, 15, 15, 15, 15, 15, 13, 13, 11, 11, 8, 8, 5, 5];
    let mut seen = [false; 14];
    for con in [40u8, 41] {
        for wound in 0..=13u8 {
            for seed in 0..4 {
                let mut g = game(&a, seed, t(12, con, con - wound), room(4, 4));
                g.set_life_force(0, con - wound);
                for turn in 0..4 {
                    let moves = next_turn(&mut g);
                    let w = con - life(&g);
                    assert_eq!(
                        moves, SERIES[w as usize],
                        "c-s12c{con} from wound {wound}, seed {seed}, turn {turn}: wound {w}"
                    );
                    seen[w as usize] = true;
                }
            }
        }
    }
    assert_eq!(seen, [true; 14]);
}

// ---- spaces, drops and the success stack (capture.md 10.40) -----------------

const BLADE: u16 = 59;
const MAIL: u16 = 40;
const GOLD: u16 = 81;
/// Our treasures: 60 the success stack's goal, 61 what the spaces give.
const PUTCI: u16 = 60;
const GIFT: u16 = 61;
/// The fixtures of ours.
const GIVER: u16 = 101; // space: Give 61
const BUMPER: u16 = 102; // obstacle, bump: Give 61
const BARRIER: u16 = 103; // custom space: open to owners of (placement), Give 61
const PORTAL: u16 = 104; // portal to SIDE (1,4), anyone
const SHOP: u16 = 105;
const DOALL: u16 = 106; // space: Activate All
const THEME: u16 = 107; // space: Play Music 13
const LETTER: u16 = 108; // space: long message 1
const FANFARE: u16 = 109; // space: Play Music 1

fn act(kind: SpellActionKind, parameter: u16) -> SpellAction {
    SpellAction { kind, parameter }
}

fn invokes(kind: SpellActionKind, parameter: u16) -> ThingKind {
    ThingKind::Space {
        picture: 4,
        access: Access::InvokeSpell {
            action: act(kind, parameter),
        },
    }
}

/// The base adventure with the blade, the mail, gold, our treasures and
/// fixtures, and long message 1.
fn kit() -> Adventure {
    use SpellActionKind as K;
    let mut a = base(Platform::C64);
    thing(&mut a, BLADE, "BLADE", blade(10));
    thing(
        &mut a,
        MAIL,
        "MAIL",
        ThingKind::Armor {
            picture: 8,
            disappears_after_use: false,
            carry: Carry { weight: 5, value: 10 },
            stats: WeaponStats {
                power: 2,
                range: None,
                break_chance: 0,
                attack_adjustment: 0,
                usable_by_anyone: true,
                magic: false,
            },
        },
    );
    thing(&mut a, GOLD, "GOLD", treasure(1, 1));
    thing(&mut a, PUTCI, "PUTCI", treasure(1, 1));
    thing(&mut a, GIFT, "PUTGI", treasure(1, 1));
    thing(&mut a, GIVER, "GIVER", invokes(K::Give, GIFT));
    thing(
        &mut a,
        BUMPER,
        "BUMPER",
        ThingKind::Obstacle {
            picture: 7,
            bump_invokes_spell: true,
            action: act(K::Give, GIFT),
        },
    );
    thing(
        &mut a,
        BARRIER,
        "BARRIER",
        ThingKind::CustomSpace {
            picture: 4,
            placement_choice: PlacementChoice::Thing,
            trigger: CustomTrigger::OpenToOwnersOf,
            action: act(K::Give, GIFT),
            destroy_needed_thing: false,
        },
    );
    thing(&mut a, PORTAL, "PORTAL", portal_kind());
    thing(&mut a, SHOP, "SHOP", ThingKind::Store { picture: 5 });
    thing(&mut a, DOALL, "DOALL", invokes(K::ActivateAll, 0));
    thing(&mut a, THEME, "THEME", invokes(K::PlayMusic, 12));
    thing(&mut a, LETTER, "LETTER", invokes(K::LongMessage, 1));
    thing(&mut a, FANFARE, "FANFARE", invokes(K::PlayMusic, 0));
    a.long_messages[0] = "WELL DONE".into();
    a
}

/// Puts `thing` with `parameter` on top of ROOM (x, y).
fn put(a: &mut Adventure, x: u8, y: u8, thing: u16, parameter: u8) {
    item(a, ROOM, x, y, thing);
    a.regions[0].rooms[ROOM as usize].items[0].parameter = parameter;
}

fn to_side() -> Destination {
    Destination::Room {
        region: 1,
        room: SIDE,
        x: 1,
        y: 4,
    }
}

/// The harness's T (Speed 6, Wisdom and Power 13) owning `things`.
fn tee(things: &[u16]) -> PlayerSetup {
    PlayerSetup {
        name: "T".into(),
        speed: 6,
        wisdom: 13,
        power: Some(13),
        possessions: things.to_vec(),
        ..Default::default()
    }
}

/// T from the map's start (12,12) south through the map door onto ROOM
/// (4,4): the events of the walk.
fn walk_in(a: &Adventure, seed: u64, things: &[u16]) -> (Game, Vec<Event>) {
    let (mut g, _) = Game::new(a.clone(), seed, Rules::default(), tee(things));
    let mut ev = Vec::new();
    for _ in 0..8 {
        let e = g.apply(Command::Move { dir: Dir::S });
        let done = e.iter().any(|e| matches!(e, Event::EnteredRoom { .. }));
        ev.extend(e);
        if done {
            assert_eq!(g.players()[0].at, room(4, 4));
            return (g, ev);
        }
    }
    panic!("T never reached the room: {ev:?}");
}

fn given(ev: &[Event]) -> Vec<u16> {
    ev.iter()
        .filter_map(|e| match e {
            Event::Given { who: ME, thing, .. } => Some(*thing),
            _ => None,
        })
        .collect()
}

fn stack(g: &Game, x: u8, y: u8) -> Vec<u16> {
    g.adventure().regions[0].rooms[ROOM as usize]
        .items
        .iter()
        .filter(|it| (it.x, it.y) == (x, y))
        .map(|it| it.thing)
        .collect()
}

/// capture.md 10.40 item 5 (rr36, `peek.txt` at T's first room turn;
/// movement 6.3, 5.5): the map door leads to 4,4; on it a space giving 61
/// (`arsp`), an obstacle giving 61 when bumped (`arob`), a portal (`ardo`)
/// or a custom space open only to owners of the blade, T not one (`arcs`):
/// T stood on 4,4 each time, nothing given, no door taken. Stepping off and
/// back (`arsp-ew`) gave 61; from 5,4 a push west into the obstacle
/// (`arob-ew`) gave 61, T staying on 5,4. One run each, nothing random.
#[test]
fn arriving_by_the_map_door_on_a_space_triggers_nothing() {
    for (name, thing, parameter) in [
        ("arsp", GIVER, 0),
        ("arob", BUMPER, 0),
        ("ardo", PORTAL, 0),
        ("arcs", BARRIER, BLADE as u8),
    ] {
        let mut a = kit();
        put(&mut a, 4, 4, thing, parameter);
        if thing == PORTAL {
            a.regions[0].rooms[ROOM as usize].items[0].destination = Some(to_side());
        }
        let (mut g, ev) = walk_in(&a, 1, &[]);
        assert!(given(&ev).is_empty(), "{name}: {ev:?}");
        assert!(
            !ev.iter().any(|e| matches!(
                e,
                Event::SpellTriggered { .. } | Event::TookDoor { thing: Some(_), .. }
            )),
            "{name}: {ev:?}"
        );
        assert_eq!(g.players()[0].at, room(4, 4), "{name}");
        match name {
            "arsp" => {
                let mut ev = g.apply(Command::Move { dir: Dir::E });
                ev.extend(g.apply(Command::Move { dir: Dir::W }));
                assert_eq!(given(&ev), [GIFT], "arsp-ew: {ev:?}");
                assert_eq!(g.players()[0].at, room(4, 4));
            }
            "arob" => {
                g.apply(Command::Move { dir: Dir::E });
                let ev = g.apply(Command::Move { dir: Dir::W });
                assert_eq!(given(&ev), [GIFT], "arob-ew: {ev:?}");
                assert_eq!(g.players()[0].at, room(5, 4), "arob-ew");
            }
            _ => {}
        }
    }
}

/// capture.md 10.40 item 5 (`bar`, against the idle control `bar-c`;
/// movement 5.5): a custom space open only to owners of the blade on 5,4,
/// T not one: the push east left T on 4,4 and took no move. Deterministic.
#[test]
fn a_barring_space_costs_no_move() {
    let mut a = kit();
    put(&mut a, 5, 4, BARRIER, BLADE as u8);
    let mut g = game(&a, 1, tee(&[]), room(4, 4));
    for _ in 0..3 {
        let ev = g.apply(Command::Move { dir: Dir::E });
        assert!(ev.iter().any(|e| matches!(e, Event::Blocked { .. })), "{ev:?}");
    }
    assert_eq!(g.players()[0].at, room(4, 4));
    assert_eq!(g.players()[0].moves_left, 6);
}

/// capture.md 10.40 item 6 (`dh-own`, `dk2`; inventory 5.1, 5.4, 5.9):
/// the own square takes a drop (the blade on 4,4 under T); T owning the
/// blade and the mail on 4,4, our obstacle on 5,4, a portal on 3,4, the
/// store on 4,5, AA on 4,3: the mail landed on the portal, the store and
/// AA's square, on top each time; the cursor pushed toward the obstacle
/// stayed put (here: refused), and the press there put the mail on T's
/// own square. No net move for a drop. One run each.
#[test]
fn drops_land_on_the_own_square_portals_stores_and_creatures() {
    let mut a = kit();
    put(&mut a, 5, 4, OBSTACLE, 0);
    put(&mut a, 3, 4, PORTAL, 0);
    a.regions[0].rooms[ROOM as usize].items[0].destination = Some(to_side());
    put(&mut a, 4, 5, SHOP, 0);
    creature(&mut a, 0, 4, 3, |c| {
        c.name = "AA".into();
        c.speed = 0;
        c.strategy.alignment = Alignment::Friend;
        c.strategy.aggressive = false;
    });
    // `dh-own`: the blade under T.
    let mut g = game(&a, 1, tee(&[BLADE]), room(4, 4));
    let ev = g.apply(Command::Drop {
        thing: BLADE,
        dir: None,
    });
    assert!(ev.contains(&Event::Dropped {
        player: 0,
        thing: BLADE,
        at: room(4, 4)
    }));
    assert_eq!(stack(&g, 4, 4), [BLADE]);
    assert_eq!(g.players()[0].moves_left, 6, "dh-own: no net move");

    for (dir, at) in [(Dir::W, (3, 4)), (Dir::S, (4, 5)), (Dir::N, (4, 3))] {
        let mut g = game(&a, 1, tee(&[BLADE, MAIL]), room(4, 4));
        let under = stack(&g, at.0, at.1);
        let ev = g.apply(Command::Drop {
            thing: MAIL,
            dir: Some(dir),
        });
        assert!(
            ev.contains(&Event::Dropped {
                player: 0,
                thing: MAIL,
                at: room(at.0, at.1)
            }),
            "dk2 {dir:?}: {ev:?}"
        );
        let mut want = vec![MAIL];
        want.extend(under);
        assert_eq!(stack(&g, at.0, at.1), want, "dk2 {dir:?}: on top");
        assert_eq!(g.players()[0].moves_left, 6, "dk2 {dir:?}: no net move");
    }
    // Toward the obstacle: the cursor stays; the drop goes on T's square.
    let mut g = game(&a, 1, tee(&[BLADE, MAIL]), room(4, 4));
    let ev = g.apply(Command::Drop {
        thing: MAIL,
        dir: Some(Dir::E),
    });
    assert!(
        ev.iter().any(|e| matches!(e, Event::DropRefused { .. })),
        "{ev:?}"
    );
    assert!(g.players()[0].owns(MAIL));
    g.apply(Command::Drop {
        thing: MAIL,
        dir: None,
    });
    assert_eq!(stack(&g, 4, 4), [MAIL]);
    assert_eq!(stack(&g, 5, 4), [OBSTACLE]);
    assert_eq!(g.players()[0].moves_left, 6);
}

/// capture.md 10.40 item 6 (`dk2-armor`; inventory 4.3): NEW ARMOR, the
/// mail: the moves went 5 to 0 at the choice, the mail worn and the next
/// mover at once: readying armor ends the turn.
#[test]
fn new_armor_ends_the_turn() {
    let a = kit();
    let mut g = game(&a, 1, tee(&[BLADE, MAIL]), room(4, 4));
    g.apply(Command::Move { dir: Dir::E });
    let ev = g.apply(Command::Ready { thing: MAIL });
    assert!(
        ev.iter().any(|e| matches!(
            e,
            Event::Readied {
                thing: MAIL,
                slot: Slot::Armor,
                ..
            }
        )),
        "{ev:?}"
    );
    assert!(ev.contains(&Event::TurnEnded { player: 0 }), "{ev:?}");
    assert_eq!(g.players()[0].armor, Some(MAIL));
}

/// capture.md 10.40 (`gp2`, `gp`; inventory 2.5): a gold pile of ours on
/// 5,4 with parameter 35 and size record 10: T took it, gold 0 to 10; with
/// the size record's bytes 81 and 10, 2,641. The amount is the size
/// record's alone; the parameter changes nothing. One run each.
#[test]
fn a_gold_piles_amount_is_its_size_record_alone() {
    for (name, size) in [("gp2", 10u16), ("gp", 81 + 10 * 256)] {
        let mut a = kit();
        a.regions[0].rooms[ROOM as usize].items.insert(
            0,
            RoomItem {
                x: 5,
                y: 4,
                thing: GOLD,
                parameter: 35,
                flag: false,
                destination: None,
                gold: Some(size),
            },
        );
        let mut g = game(&a, 1, tee(&[]), room(4, 4));
        let ev = g.apply(Command::Move { dir: Dir::E });
        assert!(
            ev.contains(&Event::GoldPickedUp {
                player: 0,
                amount: size
            }),
            "{name}: {ev:?}"
        );
        assert_eq!(g.players()[0].gold, size, "{name}");
    }
}

/// capture.md 10.40 item 7 (`win-4`, `win-5`; winning 1): on 6,4, top
/// first, our treasure 60, a space running Activate All, music 13, long
/// message 1, music 1. T stepped to 5,4, took the treasure, then stepped
/// onto 6,4: the long message filled the screen, and after the button play
/// went on, T walking back west. No win screen and no end.
#[test]
fn the_success_stack_shows_its_message_and_play_goes_on() {
    let mut a = kit();
    for t in [FANFARE, LETTER, THEME, DOALL, PUTCI] {
        put(&mut a, 6, 4, t, 0);
    }
    let mut g = game(&a, 1, tee(&[]), room(4, 4));
    let mut ev = Vec::new();
    for _ in 0..4 {
        if ev.iter().any(|e| matches!(e, Event::LongMessage { .. })) {
            break;
        }
        ev.extend(g.apply(Command::Move { dir: Dir::E }));
    }
    let took = ev
        .iter()
        .position(|e| matches!(e, Event::PickedUp { thing: PUTCI, .. }))
        .unwrap_or_else(|| panic!("no take: {ev:?}"));
    let message = ev
        .iter()
        .position(|e| matches!(e, Event::LongMessage { number: 1, .. }))
        .unwrap_or_else(|| panic!("no message: {ev:?}"));
    assert!(took < message);
    assert_eq!(g.players()[0].at, room(6, 4));
    assert!(!g.is_over());
    assert!(!ev.iter().any(|e| matches!(e, Event::AllPlayersDead)));
    // Play goes on: T walks back west, now and in later rounds.
    let mut walked = 0;
    for _ in 0..12 {
        let ev = g.apply(Command::Move { dir: Dir::W });
        walked += ev.iter().filter(|e| matches!(e, Event::Moved { .. })).count();
    }
    assert!(walked >= 3, "{walked}");
    assert!(!g.is_over());
}
