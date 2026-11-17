// SPDX-License-Identifier: GPL-3.0-or-later

//! Movement and weight (movement 2-3, 5-6, inventory 2-5): the Speed survey,
//! the weight slowdown and capacity, refused moves and the door's cost,
//! spaces on arrival, drops, readying armor, gold piles and the success
//! stack (winning 1), gold's weight per piece and what a take gives, the
//! carry limit's refusal, readying by a pick-up, the map drop's move, a
//! part-drained step and a player of Speed 0. All deterministic.

use crate::world::*;
use acs_core::model::{
    Access, Alignment, Carry, CustomTrigger, Destination, PlacementChoice, RoomItem, SpellAction,
    SpellActionKind, ThingKind, WeaponStats,
};
use acs_core::{Adventure, Platform};
use acs_engine::{BlockReason, Command, Dir, Event, Game, Location, PlayerSetup, Rules, Slot};

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

// ---- gold's weight and what a take gives (capture.md 10.42) ----------------

/// capture.md 10.42 item 2 (rr38 `gw1`, `gw4`, `gw0`, `run mem` watching
/// the moves 60M; inventory 1.3, 3.1): T of Speed 15, Strength 10,
/// Constitution and Life Force 20 (capacity 1,000), carrying nothing but
/// gold. Gold's weight 1 with 300, 600, 800 and 1,000 pieces: refills of
/// 15, 12, 6 and none (Speed 0); weight 4 with 100, 150, 200: 15, 12, 6;
/// weight 0 with 900 and 3,000: 15. 18 refills in all, each 3.3's slowdown
/// for pieces x Gold's weight. Here two refills a case, 18.
#[test]
fn gold_weighs_per_piece() {
    let mut refills = 0;
    for (weight, gold, moves) in [
        (1, 300, 15),
        (1, 600, 12),
        (1, 800, 6),
        (1, 1000, 0),
        (4, 100, 15),
        (4, 150, 12),
        (4, 200, 6),
        (0, 900, 15),
        (0, 3000, 15),
    ] {
        let mut a = kit();
        thing(&mut a, GOLD, "GOLD", treasure(weight, 1));
        let mut g = game(
            &a,
            1,
            PlayerSetup {
                speed: 15,
                strength: 10,
                constitution: 20,
                life_force: 20,
                gold,
                ..Default::default()
            },
            room(4, 4),
        );
        for _ in 0..2 {
            assert_eq!(next_turn(&mut g), moves, "weight {weight}, {gold} gold");
            refills += 1;
        }
    }
    assert_eq!(refills, 18);
}

/// capture.md 10.42 item 6 (rr38 `pk-e2`, `pk-s2`, `pk-w2`, each take
/// answered; inventory 2.7, winning 1.4): our treasures 60 on 5,4 placed
/// with parameter 0, 61 on 4,5 with 1, 62 on 3,4 with 2. 60: "T takes", the
/// thing gone from the floor, no possession set; 61 and 62: the possession
/// set (one bit, things 33..80). One run each.
#[test]
fn a_take_gives_the_placed_parameter() {
    const PUTGJ: u16 = 62;
    let mut a = kit();
    thing(&mut a, PUTGJ, "PUTGJ", treasure(1, 1));
    put(&mut a, 5, 4, PUTCI, 0);
    put(&mut a, 4, 5, GIFT, 1);
    put(&mut a, 3, 4, PUTGJ, 2);
    for (name, dir, t, (x, y), owned) in [
        ("pk-e2", Dir::E, PUTCI, (5, 4), 0),
        ("pk-s2", Dir::S, GIFT, (4, 5), 1),
        ("pk-w2", Dir::W, PUTGJ, (3, 4), 1),
    ] {
        let mut g = game(&a, 1, tee(&[]), room(4, 4));
        let ev = g.apply(Command::Move { dir });
        assert!(
            ev.contains(&Event::PickedUp { player: 0, thing: t }),
            "{name}: {ev:?}"
        );
        assert!(stack(&g, x, y).is_empty(), "{name}: gone from the floor");
        assert_eq!(g.players()[0].count(t), owned, "{name}");
    }
}

// ---- the carry limit, readying by a pick-up, the map drop (capture.md 10.43)

/// rr39's bow (thing 41).
const BOW: u16 = 41;

/// capture.md 10.43 item 1 (rr39 `pc-e`, `pc-s`, `pc-e0`; inventory 2.2):
/// our treasure 60 on 5,4 and thing 1 on 4,5, each placed with 1. T owning
/// 60, a push east: "T cannot carry more PUTCI", T on 4,4, its moves the
/// same at the push, after the button and on to the drain; T owning three
/// of thing 1, a push south: the same. Owning nothing, the push east took
/// 60 and one move. One run each.
#[test]
fn a_move_refused_at_the_carry_limit_costs_nothing() {
    const ONE: u16 = 1;
    let mut a = kit();
    thing(&mut a, ONE, "ONE", treasure(1, 1));
    put(&mut a, 5, 4, PUTCI, 1);
    put(&mut a, 4, 5, ONE, 1);
    for (name, owned, dir, t) in [
        ("pc-e", vec![PUTCI], Dir::E, PUTCI),
        ("pc-s", vec![ONE; 3], Dir::S, ONE),
    ] {
        let mut g = game(&a, 1, tee(&owned), room(4, 4));
        let ev = g.apply(Command::Move { dir });
        assert!(
            matches!(
                ev[..],
                [Event::Blocked {
                    reason: BlockReason::CarryLimit { thing },
                    ..
                }] if thing == t
            ),
            "{name}: {ev:?}"
        );
        assert_eq!(g.players()[0].at, room(4, 4), "{name}");
        assert_eq!(g.players()[0].moves_left, 6, "{name}: no move spent");
        assert_eq!(g.players()[0].count(t), owned.len() as u16, "{name}");
    }
    let mut g = game(&a, 1, tee(&[]), room(4, 4));
    let ev = g.apply(Command::Move { dir: Dir::E });
    assert!(
        ev.contains(&Event::PickedUp {
            player: 0,
            thing: PUTCI
        }),
        "pc-e0: {ev:?}"
    );
    assert_eq!(g.players()[0].at, room(4, 4), "pc-e0");
    assert_eq!(g.players()[0].moves_left, 5, "pc-e0: the take's move");
}

/// capture.md 10.43 item 2 (rr39 `rd-e`, `rdb-e`, `rd-s`, `pr-e`, `pr-w`,
/// `rd-drop`; inventory 2.4, 4.3, 4.4): T owning and holding the blade,
/// our bow on 5,4, our mail on 4,5, treasure 60 on 3,4 (`rd`); or holding
/// the bow, the blade on 5,4 (`rdb`). The bow taken: held 41, the blade
/// replaced; the blade taken: held 59, the bow replaced; the mail taken:
/// owned, nothing worn, the blade still held. The bow's take and the
/// treasure's each left 4 of 5 moves: the readying costs nothing over the
/// step. The blade dropped on T's own square: nothing held. One run each.
#[test]
fn a_picked_up_weapon_is_readied_for_the_steps_cost_alone() {
    let mut rd = kit();
    thing(&mut rd, BOW, "BOW", bow(5, 9));
    let mut rdb = rd.clone();
    put(&mut rd, 5, 4, BOW, 1);
    put(&mut rd, 4, 5, MAIL, 1);
    put(&mut rd, 3, 4, PUTCI, 1);
    put(&mut rdb, 5, 4, BLADE, 1);
    // T holding `held` with 5 moves left.
    let holding = |a: &Adventure, held: u16| {
        let mut g = game(a, 1, tee(&[held]), room(4, 4));
        let ev = g.apply(Command::Ready { thing: held });
        assert!(
            ev.iter()
                .any(|e| matches!(e, Event::Readied { thing, .. } if *thing == held)),
            "{ev:?}"
        );
        assert_eq!(g.players()[0].moves_left, 5);
        g
    };
    for (name, a, held, dir, taken, weapon) in [
        ("rd-e", &rd, BLADE, Dir::E, BOW, BOW),
        ("rdb-e", &rdb, BOW, Dir::E, BLADE, BLADE),
        ("rd-s", &rd, BLADE, Dir::S, MAIL, BLADE),
        ("pr-w", &rd, BLADE, Dir::W, PUTCI, BLADE),
    ] {
        let mut g = holding(a, held);
        let ev = g.apply(Command::Move { dir });
        assert!(
            ev.contains(&Event::PickedUp {
                player: 0,
                thing: taken
            }),
            "{name}: {ev:?}"
        );
        let p = &g.players()[0];
        assert!(p.owns(taken), "{name}");
        assert_eq!((p.weapon, p.armor), (Some(weapon), None), "{name}");
        assert_eq!(p.moves_left, 4, "{name}: the step alone");
    }
    let mut g = holding(&rd, BLADE);
    let ev = g.apply(Command::Drop {
        thing: BLADE,
        dir: None,
    });
    assert!(ev.contains(&Event::Dropped {
        player: 0,
        thing: BLADE,
        at: room(4, 4)
    }));
    assert_eq!(stack(&g, 4, 4), [BLADE], "rd-drop");
    assert_eq!(g.players()[0].weapon, None, "rd-drop: nothing held");
}

/// capture.md 10.43 item 5 (rr39 `mp-d`, `mp-d2`, `mp2.txt`; inventory
/// 5.3, 5.9): T on the map's start owning our magic item, acting when
/// dropped, Life Force +5. DROP OBJECT, the item: "the thing dropped is
/// lost forever!", the item gone, Life Force 40 to 45; the moves 4 to 3 as
/// the thing was chosen and 3 on to the drain: a drop on the world map
/// costs one move. Two runs alike.
#[test]
fn a_drop_on_the_world_map_costs_one_move_and_its_spell_goes_off() {
    const CHARM: u16 = 70;
    let mut a = kit();
    thing(
        &mut a,
        CHARM,
        "CHARM",
        ThingKind::MagicItem {
            picture: 3,
            disappears_after_use: false,
            carry: Carry { weight: 1, value: 1 },
            action: act(SpellActionKind::ChangeLifeForce, 5),
            on_use: false,
            on_drop: true,
            on_pickup: false,
        },
    );
    let mut g = game(
        &a,
        1,
        PlayerSetup {
            constitution: 40,
            life_force: 40,
            ..tee(&[CHARM])
        },
        map(12, 12),
    );
    let before = g.players()[0].moves_left;
    let ev = g.apply(Command::Drop {
        thing: CHARM,
        dir: None,
    });
    assert_eq!(
        ev[0],
        Event::LostOnMap {
            player: 0,
            thing: CHARM
        },
        "{ev:?}"
    );
    assert!(!g.players()[0].owns(CHARM));
    assert_eq!(life(&g), 45, "the drop spell on the dropper: {ev:?}");
    assert_eq!(g.players()[0].moves_left, before - 1, "one move");
}

// ---- a part-drained step, a player of Speed 0 (capture.md 10.44) ----------

/// capture.md 10.44 item 7 (rr40 `pd`, `pd.txt`; movement 2.4): T alone,
/// Speed 6, left idle until the bar's first drain to 5, then a further
/// 100k, 900k, 1,600k or 1,750k instructions (part of the next step
/// drained), then holding east until the moves reach 0: 5 squares each
/// time; after 1,850k, past the second drain, 4 moves and 4 squares. Only
/// a whole step's drain takes it: here an `Idle` a drained step. 5 of 5
/// runs.
#[test]
fn a_part_drained_step_can_still_be_walked() {
    let a = base(Platform::C64);
    for drains in [1, 1, 1, 1, 2] {
        let mut g = game(&a, 1, tee(&[]), room(4, 4));
        for _ in 0..drains {
            g.apply(Command::Idle);
        }
        let left = g.players()[0].moves_left;
        assert_eq!(left, 6 - drains, "moves at the push");
        assert_eq!(hold(&mut g, Dir::E), u32::from(left), "{drains} drained");
    }
}

/// capture.md 10.44 item 8 (rr40 `sp0.txt`, `sp0b.txt`, from rr37's
/// `hp-room.vsf`; movement 2.8): T's Speed set to 0; at its next turn the
/// moves read 0 and the menu opened by itself; the stick moved T nowhere
/// and the turn did not end in 20M instructions; MOVE from the menu said
/// it can't move, and the turn still went on. Engine: moves refused
/// (`NoMovesLeft`) at no cost, only `Wait` ending the turn. One run each.
#[test]
fn a_speed_0_player_cannot_move_and_only_wait_ends_its_turn() {
    let a = base(Platform::C64);
    let mut g = game(&a, 1, PlayerSetup { speed: 0, ..tee(&[]) }, room(4, 4));
    assert_eq!(next_turn(&mut g), 0, "the next turn's moves");
    for dir in [Dir::E, Dir::S, Dir::W, Dir::N] {
        let ev = g.apply(Command::Move { dir });
        assert!(
            matches!(
                ev[..],
                [Event::Blocked {
                    reason: BlockReason::NoMovesLeft,
                    ..
                }]
            ),
            "{dir:?}: {ev:?}"
        );
        assert_eq!(g.players()[0].at, room(4, 4));
    }
    let ev = wait(&mut g);
    assert!(ev.contains(&Event::TurnEnded { player: 0 }), "{ev:?}");
    assert_eq!(g.players()[0].at, room(4, 4));
}
