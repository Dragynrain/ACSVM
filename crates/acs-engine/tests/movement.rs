// SPDX-License-Identifier: GPL-3.0-or-later

//! Movement rules (`docs/rules/movement.md`) on synthetic adventures built
//! byte by byte with acs-core's fixture builder. No real adventure data.

#[path = "../../acs-core/tests/common/mod.rs"]
mod common;

use acs_core::layout::{adventure as adv, region as rg, world_map as wm};
use acs_core::{decode, Adventure, Platform};
use acs_engine::{
    encounters, BlockReason, Command, Dir, Event, Game, Location, Newcomer, PlayerSetup, Rules, SaveError,
    SaveFile, SpellSource,
};
use common::{creature_record, Fixture, RegionSpec, RoomSpec};

// Master things used below (1-based numbers).
const KEY: u16 = 1; // treasure
const ROPE: u16 = 2; // treasure
const SWIM: u16 = 3; // magic spell (key for terrain)
const LOCKED: u16 = 4; // portal, owners of KEY, destroys it, blocked message
const DOOR: u16 = 5; // portal, anyone, two-way
const MAGIC_DOOR: u16 = 6; // portal, invokes spell
const BED: u16 = 7; // space, invokes spell
const GATE: u16 = 8; // space, owners of ROPE
const LIGHT: u16 = 9; // custom space, open to owners of (thing at placement)
const VEND: u16 = 10; // custom space, when moved onto, spell parameter at placement
const WALL: u16 = 11; // obstacle, no spell
const CHEST: u16 = 12; // obstacle, bump spell, vanishes
const ALTAR: u16 = 13; // custom obstacle
const SHOP: u16 = 14; // store
const CURSE: u16 = 15; // space, non-owners of KEY
const ONE_WAY: u16 = 16; // portal, anyone, one-way

/// Builds an adventure with one region of three rooms (one deleted):
///
/// - room 0 "HALL" 8x6 at grid (0,0); room 1 "CELL" 5x6 at grid (7,0), sharing
///   HALL's east wall; room 2 deleted. One-way doors on HALL (2,0) -> CELL
///   (2,2) and (0,1) -> region 2's YARD (2,2).
/// - World map: all open grass, a mountain at (21,20), a river (owners of
///   SWIM) at (19,20), a spell field at (20,21), a map door at (20,19) into
///   HALL (2,2). Start (20,20).
fn fixture(platform: Platform) -> Fixture {
    let mut f = Fixture::new(platform);
    f.header("WALKS", "", "");
    f.thing(KEY as usize, "KEY", [0x01, 1, 1, 3]);
    f.thing(ROPE as usize, "ROPE", [0x01, 1, 1, 4]);
    f.thing(SWIM as usize, "SWIM", [0x06, 0, 0, 0]);
    // portal: p0 type 7, p1 picture | access << 6, p2 spell | destroy, p3 thing
    f.thing(
        LOCKED as usize,
        "LOCKED DOOR",
        [0x07, 5 | 2 << 6, 0x80, (KEY as u8)],
    );
    f.thing_message(LOCKED as usize, 0, 1);
    f.short_message(1, "THE DOOR IS LOCKED.");
    f.thing(DOOR as usize, "DOOR", [0x27, 5, 0, 0]);
    f.thing(MAGIC_DOOR as usize, "MAGIC DOOR", [0x07, 5 | 1 << 6, 8, 3]); // change life force 3
    f.thing(BED as usize, "BED", [0x08, 6 | 1 << 6, 8, 5]);
    f.thing_message(BED as usize, 1, 2);
    f.short_message(2, "YOU FEEL RESTED.");
    f.thing(GATE as usize, "GATE", [0x08, 6 | 2 << 6, 0, (ROPE as u8)]);
    // custom space: thing chosen at placement (bit 7 clear), trigger 2 =
    // open to owners of, spell 12 (rid room) with fixed parameter 9.
    f.thing(LIGHT as usize, "LIGHT", [0x09, 7 | 2 << 6, 12, 9]);
    // custom space: spell parameter at placement (bit 7 set), trigger 1 =
    // when moved onto, spell 13 (add to room).
    f.thing(VEND as usize, "VEND", [0x89, 7 | 1 << 6, 13, 0]);
    f.thing(WALL as usize, "PILLAR", [0x0A, 8, 0, 0]);
    f.thing(CHEST as usize, "CHEST", [0x1A, 8 | 0x40, 13, (KEY as u8)]); // vanishes, add KEY
    f.thing(ALTAR as usize, "ALTAR", [0x0B, 8, 10, 0]); // long message, per placement
    f.thing(SHOP as usize, "SHOP", [0x0C, 9, 0, 0]);
    f.thing(CURSE as usize, "CURSE", [0x08, 6 | 3 << 6, 0, (KEY as u8)]);
    f.thing(ONE_WAY as usize, "ONE WAY", [0x07, 5, 0, 0]);
    f.thing(127, "EMPTY FLOOR", [0x00, 1, 0, 0]);
    f.thing(128, "ROOM BORDER", [0x00, 9, 0, 0]);

    let mut guard = creature_record(10, 0x50, 70);
    guard[0x0D] = 6; // x
    guard[0x0E] = 4; // y
    guard[0x10] = 0x10; // region 1, room 0

    f.region(RegionSpec {
        name: "KEEP",
        rooms: vec![
            RoomSpec {
                name: "HALL",
                w: 8,
                h: 6,
                wall: 9,
                random: 0, // set per test
                items: vec![
                    (7, 2, (DOOR as u8), 1),       // east wall -> CELL (1,2)
                    (7, 3, (LOCKED as u8), 2),     // east wall -> CELL (1,3), key needed
                    (0, 2, (MAGIC_DOOR as u8), 3), // west wall -> world map exit point
                    (0, 3, (DOOR as u8), 4),       // west wall -> deleted room 2
                    (2, 1, (KEY as u8), 0),
                    (3, 1, (BED as u8), 0),
                    (4, 1, (GATE as u8), 0),
                    (5, 1, (LIGHT as u8), (ROPE as u8)),
                    (6, 1, (VEND as u8), 77),
                    (3, 4, (WALL as u8), 0),
                    (4, 4, (CHEST as u8), 0),
                    (5, 4, (ALTAR as u8), 42),
                    (1, 4, (SHOP as u8), 0),
                    (2, 4, (CURSE as u8), 0),
                    (2, 3, (WALL as u8), 0), // a pillar on top of a key: the pillar decides
                    (2, 3, (KEY as u8), 0),
                    (2, 0, (ONE_WAY as u8), 6), // north wall -> CELL (2,2)
                    (0, 1, (ONE_WAY as u8), 7), // west wall -> region 2 YARD (2,2)
                ],
                ..Default::default()
            },
            RoomSpec {
                name: "CELL",
                w: 5,
                h: 6,
                x: 7,
                wall: 9,
                items: vec![(0, 2, (DOOR as u8), 5)], // back to HALL (6,2)
                ..Default::default()
            },
            RoomSpec {
                name: "GONE",
                w: 1, // stored size 0: deleted
                h: 1,
                x: 20,
                wall: 9,
                ..Default::default()
            },
        ],
        // Entry 0 unused (C64 numbering), then the doors above.
        portals: vec![
            [0, 0],
            [0x11, 0x12],
            [0x11, 0x13],
            [0x00, 0x23], // region 0: world map point 35 (exit point 3)
            [0x12, 0x22],
            [0x10, 0x62],
            [0x11, 0x22],
            [0x20, 0x22],
        ],
        residents: vec![(0, "GUARD", guard)],
        ..Default::default()
    });
    f.region(RegionSpec {
        name: "FARM",
        rooms: vec![RoomSpec {
            name: "YARD",
            w: 6,
            h: 6,
            wall: 9,
            ..Default::default()
        }],
        portals: vec![[0, 0]],
        ..Default::default()
    });
    world(&mut f);
    f
}

fn terrain(f: &mut Fixture, i: usize, picture: u8, pass: u8, param: u8) {
    let w = f.l.world_map;
    f.put(w + wm::TERRAIN + i * 2, &[picture | pass << 6, param]);
}

fn set_tile(f: &mut Fixture, x: usize, y: usize, t: u8) {
    let a = f.l.world_map + wm::MAP + y * 20 + x / 2;
    let b = f.data[a];
    f.data[a] = if x % 2 == 0 {
        (b & 0x0F) | t << 4
    } else {
        (b & 0xF0) | t
    };
}

fn world(f: &mut Fixture) {
    let w = f.l.world_map;
    terrain(f, 0, 1, 1, 0); // grass, open
    terrain(f, 1, 2, 0, 0); // mountain, impassable
    terrain(f, 2, 3, 2, SWIM as u8); // river, owners of SWIM
    terrain(f, 3, 4, 3, SWIM as u8); // field, triggers SWIM's spell
    terrain(f, 12, 5, 1, 0); // door terrain
    set_tile(f, 21, 20, 1);
    set_tile(f, 19, 20, 2);
    set_tile(f, 20, 21, 3);
    set_tile(f, 20, 19, 12);
    f.put(w + wm::START_X, &[20, 20]);
    // Map door 0 at (20,19) -> region 1 room 0 (2,2).
    f.put(w + wm::POINTS, &[20, 19]);
    f.put(w + wm::DOOR_DESTINATIONS, &[0x10, 0x22]);
    // Exit point 32 + 3 = (5,6), used by HALL's west door (table entry 3).
    f.put(w + wm::POINTS + (32 + 3) * 2, &[5, 6]);
}

fn build(f: &mut Fixture) -> Adventure {
    decode(&f.build()).expect("decodes").adventure
}

fn adventure(platform: Platform) -> Adventure {
    build(&mut fixture(platform))
}

fn game(platform: Platform) -> Game {
    Game::new(
        adventure(platform),
        1,
        Rules { diagonals: true },
        PlayerSetup {
            speed: 15,
            ..Default::default()
        },
    )
    .0
}

fn hall(x: u8, y: u8) -> Location {
    Location::Room {
        region: 1,
        room: 0,
        x,
        y,
    }
}

fn mv(g: &mut Game, dir: Dir) -> Vec<Event> {
    g.apply(Command::Move { dir })
}

fn blocked(ev: &[Event]) -> Option<BlockReason> {
    ev.iter().find_map(|e| match e {
        Event::Blocked { reason, .. } => Some(*reason),
        _ => None,
    })
}

fn both(test: impl Fn(Platform)) {
    test(Platform::Pc);
    test(Platform::C64);
}

#[test]
fn start_and_first_turn() {
    both(|p| {
        let (g, ev) = Game::new(adventure(p), 9, Rules { diagonals: true }, PlayerSetup::default());
        assert_eq!(g.current_player().at, Location::World { x: 20, y: 20 });
        assert_eq!(
            ev,
            vec![
                Event::PlayerAdded {
                    player: 0,
                    at: Location::World { x: 20, y: 20 }
                },
                Event::TurnStarted {
                    player: 0,
                    moves: 7,
                    hindered: false
                },
            ]
        );
    });
}

#[test]
fn walls_block_without_cost_and_floor_costs_one() {
    both(|p| {
        let mut g = game(p);
        g.place(0, hall(1, 1));
        let ev = mv(&mut g, Dir::N);
        assert_eq!(blocked(&ev), Some(BlockReason::Wall));
        assert_eq!(g.current_player().moves_left, 15);
        let ev = mv(&mut g, Dir::S);
        assert!(matches!(ev[0], Event::Moved { to, .. } if to == hall(1, 2)));
        assert_eq!(g.current_player().moves_left, 14);
    });
}

#[test]
fn the_bar_runs_out_and_refills() {
    let (mut g, _) = Game::new(
        adventure(Platform::C64),
        1,
        Rules { diagonals: true },
        PlayerSetup {
            speed: 2,
            ..Default::default()
        },
    );
    mv(&mut g, Dir::S); // 20,21 field: spell, still a step
    let ev = mv(&mut g, Dir::S);
    assert!(ev.contains(&Event::TurnEnded { player: 0 }));
    assert!(ev.contains(&Event::TurnStarted {
        player: 0,
        moves: 2,
        hindered: false
    }));
    assert_eq!(g.current_player().moves_left, 2);
    // Wait ends the turn at once.
    let ev = g.apply(Command::Wait);
    assert_eq!(ev[0], Event::TurnEnded { player: 0 });
}

#[test]
fn players_take_turns() {
    let mut g = game(Platform::Pc);
    let full = g.current_player().moves_left;
    let ev = g.apply(Command::AddPlayer {
        setup: PlayerSetup {
            name: "TWO".into(),
            speed: 1,
            ..Default::default()
        },
    });
    // Characters 6.2: the request is taken at once, no move used; the
    // newcomer is added at the round's end and joins the next round, after
    // the players before it.
    assert_eq!(ev, vec![Event::AddRequested]);
    assert_eq!(g.players().len(), 1);
    assert_eq!(g.current_player().moves_left, full);
    let ev = g.apply(Command::Wait);
    assert!(ev
        .iter()
        .any(|e| matches!(e, Event::PlayerAdded { player: 1, .. })));
    assert_eq!(g.current(), 0);
    g.apply(Command::Wait);
    assert_eq!(g.current(), 1);
    mv(&mut g, Dir::S); // one step uses TWO's whole bar
    assert_eq!(g.current(), 0);
}

/// Waits until the round's end has handled the waiting add (characters
/// 6.2): the events up to it.
fn until_added(g: &mut Game) -> Vec<Event> {
    let mut ev = Vec::new();
    while !ev
        .iter()
        .any(|e| matches!(e, Event::PlayerAdded { .. } | Event::AddRefused))
    {
        ev.extend(g.apply(Command::Wait));
    }
    ev
}

fn added(ev: &[Event]) -> Vec<usize> {
    ev.iter()
        .filter_map(|e| match e {
            Event::PlayerAdded { player, .. } => Some(*player),
            _ => None,
        })
        .collect()
}

/// Movement 1.4, 1.5, characters 5.4, 6.1, 6.2, 6.4: four living characters
/// at most, a fifth refused at the round's end; one add a round, the first
/// asked; names wrap past 15.
#[test]
fn at_most_four_players() {
    let mut g = game(Platform::Pc);
    let add = |name: &str| Command::AddPlayer {
        setup: PlayerSetup {
            name: name.into(),
            ..Default::default()
        },
    };
    // Every request is answered alike; the round holds only the first.
    for n in ["TWO", "THREE", "A NAME FAR TOO LONG", "FIVE"] {
        assert_eq!(g.apply(add(n)), vec![Event::AddRequested]);
    }
    assert_eq!(g.players().len(), 1);
    let ev = g.apply(Command::Wait);
    assert_eq!(added(&ev), [1]);
    assert_eq!(g.players().len(), 2);
    assert_eq!(g.players()[1].name, "TWO");
    // Adds bring no round-end message: the count at the next round's
    // start includes them.
    assert!(!ev.iter().any(|e| matches!(e, Event::LivingChanged { .. })));
    // Nothing more comes of the other three.
    let mut later = Vec::new();
    for _ in 0..4 {
        later.extend(g.apply(Command::Wait));
    }
    assert!(added(&later).is_empty());
    for (n, slot) in [("THREE", 2), ("A NAME FAR TOO LONG", 3)] {
        g.apply(add(n));
        assert_eq!(added(&until_added(&mut g)), [slot]);
    }
    assert_eq!(g.players().len(), 4);
    assert_eq!(g.players()[3].name, "LONGME FAR TOO");
    let start = g.players()[3].at;
    assert_eq!(g.players()[1].at, start);
    // Two requests with all four living: one refusal.
    g.apply(add("FIVE"));
    g.apply(add("SIX"));
    let ev = until_added(&mut g);
    assert_eq!(ev.iter().filter(|e| **e == Event::AddRefused).count(), 1);
    assert!(added(&ev).is_empty());
    assert_eq!(g.players().len(), 4);
    // Every one of the four gets a turn, in joining order.
    while g.current() != 0 {
        g.apply(Command::Wait);
    }
    let mut seen = Vec::new();
    for _ in 0..8 {
        seen.push(g.current());
        g.apply(Command::Wait);
    }
    assert_eq!(seen, [0, 1, 2, 3, 0, 1, 2, 3]);
}

/// Characters 6.2, 6.4: two players asking once each in a round add one
/// newcomer, the first asker's, into the lowest free slot.
#[test]
fn two_players_asking_in_a_round_add_the_first_ones_newcomer() {
    let mut g = game(Platform::Pc);
    let add = |name: &str| Command::AddPlayer {
        setup: PlayerSetup {
            name: name.into(),
            ..Default::default()
        },
    };
    g.apply(add("TWO"));
    g.apply(Command::Wait);
    assert_eq!(g.players().len(), 2);
    assert_eq!(g.current(), 0);
    assert_eq!(g.apply(add("FIRST")), vec![Event::AddRequested]);
    g.apply(Command::Wait);
    assert_eq!(g.current(), 1);
    assert_eq!(g.apply(add("SECOND")), vec![Event::AddRequested]);
    let ev = until_added(&mut g);
    assert_eq!(added(&ev), [2]);
    assert_eq!(g.players()[2].name, "FIRST");
    assert_eq!(g.players().len(), 3);
}

/// Characters 6.2: a request with 4 living is taken at once and refused at
/// the round's end with no draw; a refused name (5.6) is no request.
#[test]
fn a_full_game_refuses_at_the_round_end_without_a_roll() {
    let mut g = game(Platform::Pc);
    for n in ["TWO", "THREE", "FOUR"] {
        g.apply(Command::AddPlayer {
            setup: PlayerSetup {
                name: n.into(),
                ..Default::default()
            },
        });
        until_added(&mut g);
    }
    assert_eq!(g.players().len(), 4);
    // Same game, with and without the refused request: same draws.
    let mut h = g.clone();
    assert_eq!(
        g.apply(Command::CreatePlayer { name: "FIVE".into() }),
        vec![Event::AddRequested]
    );
    assert_eq!(
        h.apply(Command::CreatePlayer { name: "   NO".into() }),
        vec![Event::NameRefused]
    );
    let mut eg = Vec::new();
    let mut eh = Vec::new();
    for _ in 0..4 {
        eg.extend(g.apply(Command::Wait));
        eh.extend(h.apply(Command::Wait));
    }
    assert!(eg.contains(&Event::AddRefused));
    eg.retain(|e| *e != Event::AddRefused);
    assert_eq!(eg, eh);
    assert_eq!(g.players(), h.players());
    assert_eq!(g.players().len(), 4);
}

/// Characters 5.6, 5.7: a name whose first three characters fold to spaces
/// is refused and nothing is asked for; the game waits as it was.
#[test]
fn a_name_of_three_leading_spaces_is_asked_again() {
    let mut g = game(Platform::Pc);
    for name in ["", "   ", "        ABCDEFG", "@[]X"] {
        assert_eq!(
            g.apply(Command::CreatePlayer { name: name.into() }),
            vec![Event::NameRefused],
            "{name:?}"
        );
    }
    assert_eq!(
        g.apply(Command::AddPlayer {
            setup: PlayerSetup {
                name: "  ".into(),
                ..Default::default()
            }
        }),
        vec![Event::NameRefused]
    );
    let ev = g.apply(Command::Wait);
    assert!(!ev.iter().any(|e| matches!(e, Event::PlayerAdded { .. })));
    assert_eq!(g.players().len(), 1);
    // Two leading spaces are a name.
    assert_eq!(
        g.apply(Command::CreatePlayer { name: "  X".into() }),
        vec![Event::AddRequested]
    );
    g.apply(Command::Wait);
    assert_eq!(g.players()[1].name, "  X");
}

/// Characters 1, 3.1: a created character is rolled from the game's
/// generator (so a replay rolls it again the same); a joining one keeps its
/// Power and Life Force as given.
#[test]
fn created_and_joining_characters() {
    let adv = adventure(Platform::Pc);
    let first = acs_engine::Newcomer::Create {
        create: "Rolled 0ne!".into(),
    };
    let (mut g, _) = Game::new(adv.clone(), 5, Rules::default(), first.clone());
    let p = g.players()[0].clone();
    assert_eq!(p.name, "ROLLED ONE.");
    assert_eq!(
        (p.life_force, p.power, p.armor_skill, p.gold),
        (p.constitution, p.wisdom, 0, 0)
    );
    assert_eq!(p.speed, 5 + p.dexterity / 7);
    // Each joins at its round's end, one a round (characters 6.2, 6.4).
    g.apply(Command::CreatePlayer { name: "two".into() });
    until_added(&mut g);
    g.apply(Command::AddPlayer {
        setup: PlayerSetup {
            name: "saved".into(),
            wisdom: 20,
            power: Some(5),
            constitution: 15,
            life_force: 3,
            ..Default::default()
        },
    });
    until_added(&mut g);
    let saved = &g.players()[2];
    assert_eq!((saved.power, saved.life_force), (5, 3));
    let r = Game::replay(adv, 5, Rules::default(), first, g.log());
    assert_eq!(r.players(), g.players());
}

/// The synthetic tests were written with diagonal steps allowed and keep
/// them on explicitly; the default follows the C64 capture (movement 3.2).
#[test]
fn default_rules_refuse_diagonals() {
    let (mut g, _) = Game::new(
        adventure(Platform::Pc),
        1,
        Rules::default(),
        PlayerSetup::default(),
    );
    assert_eq!(blocked(&mv(&mut g, Dir::NE)), Some(BlockReason::Diagonal));
    assert_ne!(blocked(&mv(&mut g, Dir::E)), Some(BlockReason::Diagonal));
}

#[test]
fn diagonals_can_be_turned_off() {
    let (mut g, _) = Game::new(
        adventure(Platform::Pc),
        1,
        Rules { diagonals: false },
        PlayerSetup::default(),
    );
    assert_eq!(blocked(&mv(&mut g, Dir::SE)), Some(BlockReason::Diagonal));
    let mut g = game(Platform::Pc);
    let ev = mv(&mut g, Dir::SW);
    assert!(matches!(
        ev[0],
        Event::Moved {
            to: Location::World { x: 19, y: 21 },
            ..
        }
    ));
}

#[test]
fn world_map_terrain() {
    both(|p| {
        let mut g = game(p);
        // Mountain: impassable.
        assert_eq!(
            blocked(&mv(&mut g, Dir::E)),
            Some(BlockReason::Terrain { terrain: 1 })
        );
        // River: owners of SWIM only.
        assert_eq!(
            blocked(&mv(&mut g, Dir::W)),
            Some(BlockReason::NeedsThing { thing: SWIM })
        );
        g.give(0, SWIM);
        assert!(matches!(mv(&mut g, Dir::W)[0], Event::Moved { .. }));
        mv(&mut g, Dir::E);
        // Field: moves, then SWIM's spell runs.
        let ev = mv(&mut g, Dir::S);
        assert!(matches!(ev[0], Event::Moved { .. }));
        assert!(matches!(
            ev[1],
            Event::SpellTriggered {
                source: SpellSource::Terrain,
                thing: SWIM,
                ..
            }
        ));
    });
}

#[test]
fn extended_world_map_is_as_big_as_its_tiles() {
    // Classic stops at column 39; an Extended map 60 wide goes on.
    let mut a = build(&mut fixture(Platform::C64));
    a.profile = acs_core::Profile::Extended;
    a.limits = acs_core::Limits::EXTENDED;
    for row in &mut a.world.tiles {
        row.resize(60, 0);
    }
    a.world.tiles.resize(60, vec![0; 60]);
    assert!(acs_core::limits::check(&a).is_empty());
    let mut g = Game::new(a, 1, Rules { diagonals: true }, PlayerSetup::default()).0;
    g.place(0, Location::World { x: 39, y: 50 });
    let ev = mv(&mut g, Dir::E);
    assert!(matches!(
        ev[0],
        Event::Moved {
            to: Location::World { x: 40, y: 50 },
            ..
        }
    ));
    g.place(0, Location::World { x: 59, y: 59 });
    assert_eq!(blocked(&mv(&mut g, Dir::E)), Some(BlockReason::MapEdge));
}

#[test]
fn map_edges_stop_unless_the_map_wraps() {
    let mut f = fixture(Platform::C64);
    let mut g = Game::new(
        build(&mut f),
        1,
        Rules { diagonals: true },
        PlayerSetup::default(),
    )
    .0;
    g.place(0, Location::World { x: 0, y: 39 });
    assert_eq!(blocked(&mv(&mut g, Dir::W)), Some(BlockReason::MapEdge));
    assert_eq!(blocked(&mv(&mut g, Dir::S)), Some(BlockReason::MapEdge));

    let a = f.l.adventure + adv::MAP_WRAP;
    f.put(a, &[1]);
    let mut g = Game::new(
        build(&mut f),
        1,
        Rules { diagonals: true },
        PlayerSetup::default(),
    )
    .0;
    g.place(0, Location::World { x: 0, y: 39 });
    let ev = mv(&mut g, Dir::SW);
    assert!(matches!(
        ev[0],
        Event::Moved {
            to: Location::World { x: 39, y: 0 },
            ..
        }
    ));
}

#[test]
fn map_door_into_a_room() {
    both(|p| {
        let mut g = game(p);
        let ev = mv(&mut g, Dir::N);
        assert_eq!(
            &ev[..3],
            &[
                Event::TookDoor {
                    player: 0,
                    from: Location::World { x: 20, y: 20 },
                    to: hall(2, 2),
                    thing: None,
                },
                Event::EnteredRegion { player: 0, region: 1 },
                Event::EnteredRoom {
                    player: 0,
                    region: 1,
                    room: 0
                },
            ]
        );
        assert_eq!(g.current_player().at, hall(2, 2));
        assert_eq!(g.current_player().moves_left, 14);
    });
}

#[test]
fn room_doors_lead_through_shared_walls_and_back() {
    both(|p| {
        let mut g = game(p);
        g.place(0, hall(6, 2));
        let ev = mv(&mut g, Dir::E);
        assert!(matches!(
            ev[0],
            Event::TookDoor {
                thing: Some(DOOR),
                ..
            }
        ));
        assert!(ev.contains(&Event::EnteredRoom {
            player: 0,
            region: 1,
            room: 1
        }));
        assert!(!ev.iter().any(|e| matches!(e, Event::EnteredRegion { .. })));
        let cell = Location::Room {
            region: 1,
            room: 1,
            x: 1,
            y: 2,
        };
        assert_eq!(g.current_player().at, cell);
        mv(&mut g, Dir::W);
        assert_eq!(g.current_player().at, hall(6, 2));
    });
}

fn moves_at_start(ev: &[Event], player: usize) -> Option<u8> {
    ev.iter().find_map(|e| match *e {
        Event::TurnStarted { player: q, moves, .. } if q == player => Some(moves),
        _ => None,
    })
}

/// 2.3, creatures 2.2.4: a door into another room ends the turn, the
/// counter goes up, and the next turn starts with the moves left after the
/// door's step; through it on the last move, the next turn is a full one.
#[test]
fn a_room_door_into_another_room_carries_the_moves_left() {
    both(|p| {
        for (from, dir, door) in [((6, 2), Dir::E, (1, 2)), ((1, 1), Dir::NE, (2, 2))] {
            // Two-way DOOR into CELL (1,2), one-way into CELL (2,2).
            let cell = Location::Room {
                region: 1,
                room: 1,
                x: door.0,
                y: door.1,
            };
            let mut g = game(p);
            g.place(0, hall(from.0, from.1 + 1));
            mv(&mut g, Dir::N);
            let round = g.round();
            let ev = mv(&mut g, dir);
            assert!(ev.contains(&Event::TurnEnded { player: 0 }), "{ev:?}");
            assert_eq!(g.current_player().at, cell);
            assert_eq!(moves_at_start(&ev, 0), Some(13), "{ev:?}");
            assert!(g.round() > round);
            assert_eq!(g.current_player().moves_left, 13);
            let ev = g.apply(Command::Wait);
            assert_eq!(moves_at_start(&ev, 0), Some(15));
            // On the last move: a full next turn.
            let mut g = game(p);
            g.place(0, hall(from.0, from.1));
            for _ in 0..14 {
                g.apply(Command::Idle);
            }
            assert_eq!(g.current_player().moves_left, 1);
            let ev = mv(&mut g, dir);
            assert_eq!(g.current_player().at, cell);
            assert_eq!(moves_at_start(&ev, 0), Some(15), "{ev:?}");
        }
    });
}

/// 2.6, creatures 2.2.4: a door into a room of another region carries the
/// moves left, kept while the player waits for that region's visit.
#[test]
fn a_door_into_another_region_keeps_the_moves_until_its_visit() {
    both(|p| {
        let mut g = game(p);
        g.apply(Command::AddPlayer {
            setup: PlayerSetup {
                speed: 15,
                ..Default::default()
            },
        });
        g.apply(Command::Wait);
        g.place(0, hall(1, 3));
        g.place(1, hall(5, 3));
        assert_eq!(g.current(), 0);
        mv(&mut g, Dir::N);
        mv(&mut g, Dir::N);
        let ev = mv(&mut g, Dir::W);
        assert!(ev.contains(&Event::TurnEnded { player: 0 }), "{ev:?}");
        let yard = Location::Room {
            region: 2,
            room: 0,
            x: 2,
            y: 2,
        };
        assert_eq!(g.players()[0].at, yard);
        // The second player plays out region 1's visit meanwhile.
        let mut seen = moves_at_start(&ev, 0);
        let mut others = usize::from(moves_at_start(&ev, 1).is_some());
        while seen.is_none() {
            assert!(others < 40, "the first player never came back");
            let ev = g.apply(Command::Wait);
            others += usize::from(moves_at_start(&ev, 1).is_some());
            seen = moves_at_start(&ev, 0);
        }
        assert!(others > 1, "{others}");
        assert_eq!(seen, Some(12));
        assert_eq!(g.current_player().at, yard);
    });
}

#[test]
fn locked_door_needs_and_destroys_the_key() {
    both(|p| {
        let mut g = game(p);
        g.place(0, hall(6, 3));
        let ev = mv(&mut g, Dir::E);
        assert_eq!(
            ev,
            vec![Event::Blocked {
                player: 0,
                target: hall(7, 3),
                reason: BlockReason::NeedsThing { thing: KEY },
                message: Some("THE DOOR IS LOCKED.".into()),
            }]
        );
        assert_eq!(g.current_player().moves_left, 15);
        g.give(0, KEY);
        let ev = mv(&mut g, Dir::E);
        assert_eq!(
            ev[0],
            Event::ThingUsedUp {
                player: 0,
                thing: KEY
            }
        );
        assert!(matches!(ev[1], Event::TookDoor { .. }));
        assert!(!g.current_player().owns(KEY));
    });
}

#[test]
fn one_way_door_to_the_map_and_door_to_a_deleted_room() {
    both(|p| {
        let mut g = game(p);
        g.place(0, hall(1, 2));
        let ev = mv(&mut g, Dir::W);
        assert!(matches!(
            ev[0],
            Event::SpellTriggered {
                source: SpellSource::Portal,
                thing: MAGIC_DOOR,
                ..
            }
        ));
        // The spell's own effects come next, then the door.
        assert!(
            ev.iter().any(|e| matches!(
                e,
                Event::TookDoor {
                    to: Location::World { x: 5, y: 6 },
                    ..
                }
            )),
            "{ev:?}"
        );
        assert!(ev.contains(&Event::LeftRegion { player: 0, region: 1 }));
        assert!(ev.contains(&Event::EnteredWorldMap { player: 0 }));

        g.place(0, hall(1, 3));
        let ev = mv(&mut g, Dir::W);
        assert_eq!(ev[0], Event::SentToStart { player: 0 });
        assert_eq!(g.current_player().at, Location::World { x: 20, y: 20 });
    });
}

#[test]
fn spaces_custom_spaces_and_pickups() {
    both(|p| {
        let mut g = game(p);
        // Key: picked up; the player stays put (inventory 2.1).
        g.place(0, hall(2, 2));
        let ev = mv(&mut g, Dir::N);
        assert_eq!(
            ev[0],
            Event::PickedUp {
                player: 0,
                thing: KEY
            }
        );
        assert!(g.current_player().owns(KEY));
        let items = &g.adventure().regions[0].rooms[0].items;
        assert!(!items.iter().any(|i| (i.x, i.y) == (2, 1)));
        assert_eq!(g.current_player().at, hall(2, 2));
        assert!(matches!(mv(&mut g, Dir::N)[0], Event::Moved { .. }));

        // Bed: invoke-spell space, with its spell message.
        let ev = mv(&mut g, Dir::E);
        assert!(matches!(ev[0], Event::Moved { .. }));
        assert!(matches!(
            &ev[1],
            Event::SpellTriggered { source: SpellSource::Space, thing: BED, message: Some(m), .. }
                if m == "YOU FEEL RESTED."
        ));

        // Gate: owners of ROPE.
        assert_eq!(
            blocked(&mv(&mut g, Dir::E)),
            Some(BlockReason::NeedsThing { thing: ROPE })
        );
        g.give(0, ROPE);
        mv(&mut g, Dir::E);
        assert_eq!(g.current_player().at, hall(4, 1));

        // Light: owners of the thing named at placement (ROPE); spell
        // parameter fixed in the master list (9).
        let ev = mv(&mut g, Dir::E);
        assert!(matches!(
            ev[1],
            Event::SpellTriggered { source: SpellSource::CustomSpace, thing: LIGHT, action, .. }
                if action.parameter == 9
        ));

        // Vend: the placement parameter is the spell's parameter.
        let ev = mv(&mut g, Dir::E);
        assert!(matches!(
            ev[1],
            Event::SpellTriggered { thing: VEND, action, .. } if action.parameter == 77
        ));
    });
}

#[test]
fn non_owner_space_obstacles_store_and_residents() {
    both(|p| {
        let mut g = game(p);
        // Curse: closed to owners of KEY.
        g.place(0, hall(1, 3));
        g.give(0, KEY);
        assert_eq!(
            blocked(&mv(&mut g, Dir::SE)),
            Some(BlockReason::ClosedToOwnersOf { thing: KEY })
        );
        // Pillar on top of a key: the pillar decides.
        assert_eq!(
            blocked(&mv(&mut g, Dir::E)),
            Some(BlockReason::Obstacle { thing: WALL })
        );
        assert_eq!(g.current_player().moves_left, 15);
        // Store (stores 3.2): a step onto its square, and it opens.
        let ev = mv(&mut g, Dir::S);
        assert_eq!(blocked(&ev), None);
        assert!(ev.iter().any(|e| matches!(e, Event::StoreOpened { .. })));
        assert_eq!(g.current_player().at, hall(1, 4));
        assert_eq!(g.current_player().moves_left, 14);
        g.apply(Command::LeaveStore);

        // Chest: bump spell, then it vanishes.
        g.place(0, hall(4, 3));
        let ev = mv(&mut g, Dir::S);
        assert_eq!(blocked(&ev), Some(BlockReason::Obstacle { thing: CHEST }));
        assert!(matches!(
            ev[1],
            Event::SpellTriggered {
                source: SpellSource::Obstacle,
                ..
            }
        ));
        assert!(ev
            .iter()
            .any(|e| matches!(e, Event::ObstacleVanished { thing: CHEST, .. })));
        // Its Add to Room left a KEY where it stood (magic 3.13).
        let ev = mv(&mut g, Dir::S);
        assert_eq!(
            ev[0],
            Event::PickedUp {
                player: 0,
                thing: KEY
            }
        );
        // The taker stays put (inventory 2.1).
        assert!(!ev.iter().any(|e| matches!(e, Event::Moved { .. })));

        // Altar: custom obstacle with the placement parameter.
        g.place(0, hall(5, 3));
        let ev = mv(&mut g, Dir::S);
        assert!(matches!(
            ev[1],
            Event::SpellTriggered { source: SpellSource::CustomObstacle, action, .. }
                if action.parameter == 42
        ));

        // The guard at (6,4).
        g.place(0, hall(6, 3));
        assert_eq!(
            mv(&mut g, Dir::S),
            vec![Event::BumpedCreature {
                player: 0,
                region: 1,
                room: 0,
                resident: 0
            }]
        );
    });
}

// ---- world map encounters (docs/rules/encounters.md) -----------------------

/// The patch square SW of the start: terrain 4, open, picture 6.
const PATCH: (u8, u8) = (19, 21);
/// A second patch square, SE of the start.
const PATCH2: (u8, u8) = (21, 21);

fn area(room: u8, x: u8, y: u8) -> Location {
    Location::Room {
        region: 0,
        room,
        x,
        y,
    }
}

/// The walking fixture with a patch of terrain 4 on `PATCH` and map
/// records `(terrain, chance code, speed)` in slots 0, 1, ..; the rest
/// stay empty (terrain 0, code 0).
fn encounter_adventure(records: &[(u8, u8, u8)]) -> Adventure {
    encounter_adventure_with(records, &[])
}

/// As [`encounter_adventure`], with the world map tail's lists `lists`
/// (area 1..4, items x, y, thing, parameter) in place of the editor's,
/// and then a second patch square SE of the start.
/// Area lists: (area, items (x, y, thing, parameter)).
type AreaLists = [(usize, Vec<(u8, u8, u8, u8)>)];

fn encounter_adventure_with(records: &[(u8, u8, u8)], lists: &AreaLists) -> Adventure {
    let mut f = fixture(Platform::Pc);
    for (n, items) in lists {
        f.area_list(*n, items.clone());
    }
    if !lists.is_empty() {
        set_tile(&mut f, PATCH2.0 as usize, PATCH2.1 as usize, 4);
    }
    terrain(&mut f, 4, 6, 1, 0);
    set_tile(&mut f, PATCH.0 as usize, PATCH.1 as usize, 4);
    let w = f.l.world_map;
    for (i, &(t, code, speed)) in records.iter().enumerate() {
        // Speed is the low four bits of the strategy byte.
        f.put(w + wm::CREATURES + i * 37, &creature_record(8, 0x50 | speed, 70));
        f.packed(
            w + wm::CREATURE_NAMES + i * 10,
            if i == 0 { "WOLF" } else { "HARE" },
        );
        f.put(w + wm::CREATURE_CHANCES + i, &[t << 4 | code]);
    }
    build(&mut f)
}

fn encounters(ev: &[Event]) -> Vec<(usize, Option<usize>)> {
    ev.iter()
        .filter_map(|e| match e {
            Event::Encounter { creature, slot, .. } => Some((*creature, *slot)),
            _ => None,
        })
        .collect()
}

/// A game whose first step SW onto the patch brought an encounter.
fn pulled_in(a: &Adventure) -> (Game, Vec<Event>) {
    for seed in 0..200 {
        let (mut g, _) = Game::new(a.clone(), seed, Rules { diagonals: true }, PlayerSetup::default());
        let ev = mv(&mut g, Dir::SW);
        if !encounters(&ev).is_empty() {
            return (g, ev);
        }
    }
    panic!("no encounter in 200 seeds");
}

#[test]
fn map_encounters_roll_after_the_step_for_the_terrain_entered() {
    // WOLF on the start's grass (terrain 0), none on the patch: the step
    // onto the patch never rolls for it (encounters 2.1).
    let a = encounter_adventure(&[(0, 15, 4)]);
    assert_eq!(a.world.random_creatures[0].chance, 60);
    for seed in 0..100 {
        let (mut g, _) = Game::new(a.clone(), seed, Rules { diagonals: true }, PlayerSetup::default());
        assert!(encounters(&mv(&mut g, Dir::SW)).is_empty());
        assert_eq!(g.current_player().at, Location::World { x: 19, y: 21 });
    }
    // WOLF on the patch at 60%: about 60% of first steps onto it.
    let a = encounter_adventure(&[(4, 15, 4)]);
    let mut hits = 0;
    for seed in 0..500 {
        let (mut g, _) = Game::new(a.clone(), seed, Rules { diagonals: true }, PlayerSetup::default());
        if !encounters(&mv(&mut g, Dir::SW)).is_empty() {
            hits += 1;
        }
    }
    assert!((250..350).contains(&hits), "{hits}");
}

#[test]
fn refused_moves_and_map_doors_take_no_roll() {
    // Records on the mountain (1) and the door terrain (12) at 60%.
    let a = encounter_adventure(&[(1, 15, 4), (12, 15, 4)]);
    for seed in 0..100 {
        let (mut g, _) = Game::new(a.clone(), seed, Rules { diagonals: true }, PlayerSetup::default());
        assert!(encounters(&mv(&mut g, Dir::E)).is_empty()); // the mountain refuses
        let ev = mv(&mut g, Dir::N); // the map door into HALL
        assert!(encounters(&ev).is_empty());
        assert_eq!(g.current_player().at, hall(2, 2));
    }
}

#[test]
fn every_hit_appears_in_record_order() {
    let a = encounter_adventure(&[(4, 15, 4), (4, 15, 3)]);
    let (mut both, mut one) = (0, 0);
    for seed in 0..400 {
        let (mut g, _) = Game::new(a.clone(), seed, Rules { diagonals: true }, PlayerSetup::default());
        let hits = encounters(&mv(&mut g, Dir::SW));
        match hits.as_slice() {
            [(0, Some(8)), (1, Some(9))] => {
                both += 1;
                assert_eq!(g.creature(0, 8).map(|c| (c.x, c.y)), Some((2, 3)));
                assert_eq!(g.creature(0, 9).map(|c| (c.x, c.y)), Some((3, 3)));
            }
            [(r, Some(8))] => {
                one += 1;
                // Record r's own square, whatever slot it takes (3.6).
                let sq = encounters::creature_square(*r);
                assert_eq!(g.creature(0, 8).map(|c| (c.x, c.y)), Some(sq));
            }
            [] => {}
            other => panic!("{other:?}"),
        }
    }
    // 36% both, 48% one.
    assert!((110..180).contains(&both), "{both}");
    assert!((150..235).contains(&one), "{one}");
}

#[test]
fn the_area_is_a_room_of_the_map_region() {
    let a = encounter_adventure(&[(4, 15, 4)]);
    let (g, ev) = pulled_in(&a);
    // 5.1: the player is on 12,4 of area 1 and its turn has ended.
    assert_eq!(g.players()[0].at, area(1, 12, 4));
    assert!(ev.iter().any(|e| matches!(e, Event::EnteredArea { area: 1, .. })));
    assert!(ev.iter().any(|e| matches!(e, Event::TurnEnded { player: 0 })));
    assert_eq!(g.area_entrance(1), Some(PATCH));
    // 3.2-3.5: 15 x 10, walled with the patch's picture, holding only
    // its list's door (thing 82 on 0,5, parameter 0).
    let room = &g.region(0).unwrap().rooms[1];
    assert!(!room.deleted);
    assert_eq!((room.width, room.height), (15, 10));
    assert_eq!(room.wall_picture, a.world.terrain[4].picture);
    let items: Vec<_> = room
        .items
        .iter()
        .map(|it| (it.x, it.y, it.thing, it.parameter))
        .collect();
    assert_eq!(items, [(0, 5, 82, 0)]);
    assert_eq!(g.area_door(area(1, 0, 5)), Some(0));
    assert_eq!(g.area_door(area(1, 1, 5)), None);
    // 4.2: the copy is the record as stored, marked random.
    let wolf = g.creature(0, 8).expect("a copy in slot 8");
    let rec = &a.world.random_creatures[0].creature;
    assert_eq!(
        (wolf.name.as_str(), wolf.life_force, wolf.speed),
        (rec.name.as_str(), rec.life_force, rec.speed)
    );
    assert_eq!((wolf.room, wolf.x, wolf.y), (1, 2, 3));
    assert!(g.is_random(0, 8));
    assert!(g.adventure().world.random_creatures[0] == a.world.random_creatures[0]);
}

#[test]
fn walking_out_returns_to_the_entrance_and_closes_the_area() {
    let a = encounter_adventure(&[(4, 15, 0)]); // a WOLF that stands still
    let (mut g, _) = pulled_in(&a);
    g.place(0, area(1, 1, 5));
    let before = g.current_player().moves_left;
    assert!(before > 0);
    let ev = mv(&mut g, Dir::W);
    // 7.1: back on the entrance, one move used, no roll.
    assert_eq!(
        g.players()[0].at,
        Location::World {
            x: PATCH.0,
            y: PATCH.1
        }
    );
    assert_eq!(g.current_player().moves_left, before - 1);
    assert!(encounters(&ev).is_empty());
    // 7.2: closed, the copy gone.
    assert!(ev.iter().any(|e| matches!(e, Event::AreaClosed { area: 1 })));
    assert!(g.creature(0, 8).is_none());
    assert!(g.region(0).unwrap().rooms[1].deleted);
    assert_eq!(g.area_entrance(1), None);
}

#[test]
fn a_second_player_joins_through_the_entrance() {
    let a = encounter_adventure(&[(4, 15, 0)]);
    for seed in 0..200 {
        let (mut g, _) = Game::new(a.clone(), seed, Rules { diagonals: true }, PlayerSetup::default());
        g.apply(Command::AddPlayer {
            setup: PlayerSetup {
                name: "U".into(),
                ..Default::default()
            },
        });
        if encounters(&mv(&mut g, Dir::SW)).is_empty() {
            continue;
        }
        // 6.1: the next round lists U (open map) first, then T and WOLF.
        assert_eq!(g.current(), 1);
        let ev = mv(&mut g, Dir::SW);
        assert!(encounters(&ev).is_empty());
        assert_eq!(g.players()[1].at, area(1, 12, 4));
        assert!(ev.iter().any(|e| matches!(
            e,
            Event::EnteredArea {
                player: 1,
                area: 1,
                ..
            }
        )));
        // T walks out: the area stays open for U (7.2).
        assert_eq!(g.current(), 0);
        g.place(0, area(1, 1, 5));
        let ev = mv(&mut g, Dir::W);
        assert!(!ev.iter().any(|e| matches!(e, Event::AreaClosed { .. })));
        assert!(g.creature(0, 8).is_some());
        return;
    }
    panic!("no encounter");
}

#[test]
fn a_creature_through_the_door_is_removed() {
    let a = encounter_adventure(&[(4, 15, 3)]);
    let (mut g, _) = pulled_in(&a);
    // A peaceful, cautious friend beside the door: the door is its goal
    // (creatures 6.4) and taking it removes it (encounters 6.4).
    let c = g.creature_mut(0, 8).unwrap();
    c.strategy.alignment = acs_core::model::Alignment::Friend;
    c.strategy.aggressive = false;
    c.strategy.brave = false;
    c.wisdom = 20;
    c.x = 1;
    c.y = 5;
    let mut removed = false;
    for _ in 0..4 {
        let ev = g.apply(Command::Wait);
        if ev.iter().any(|e| {
            matches!(
                e,
                Event::CreatureRemoved {
                    region: 0,
                    slot: 8,
                    reason: acs_engine::RemovalReason::LeftArea
                }
            )
        }) {
            removed = true;
            break;
        }
    }
    assert!(removed);
    // The area stays open while T is in it (7.3).
    assert!(!g.region(0).unwrap().rooms[1].deleted);
}

/// Encounters 3.4, 3.5, 7.1, 7.2: the area's items are its list's; the
/// door is the list's thing 82, wherever it stands; closing keeps it and
/// deletes the rest.
#[test]
fn an_area_holds_its_list_and_its_door_is_the_list_item() {
    let a = encounter_adventure_with(&[(4, 15, 0)], &[(1, vec![(0, 3, 82, 0), (6, 6, KEY as u8, 0)])]);
    let (mut g, _) = pulled_in(&a);
    let things = |g: &Game| -> Vec<(u8, u8, u16)> {
        g.region(0).unwrap().rooms[1]
            .items
            .iter()
            .map(|it| (it.x, it.y, it.thing))
            .collect()
    };
    assert_eq!(things(&g), [(0, 3, 82), (6, 6, KEY)]);
    // 0,5 is wall now: refused, no move used.
    g.place(0, area(1, 1, 5));
    let before = g.current_player().moves_left;
    let ev = mv(&mut g, Dir::W);
    assert_eq!(blocked(&ev), Some(BlockReason::Wall));
    assert_eq!(g.current_player().moves_left, before);
    assert_eq!(g.players()[0].at, area(1, 1, 5));
    // Through 0,3: back on the entrance, the area closed, the KEY gone.
    g.place(0, area(1, 1, 3));
    let ev = mv(&mut g, Dir::W);
    assert_eq!(
        g.players()[0].at,
        Location::World {
            x: PATCH.0,
            y: PATCH.1
        }
    );
    assert!(ev.iter().any(|e| matches!(e, Event::AreaClosed { area: 1 })));
    assert_eq!(things(&g), [(0, 3, 82)]);
    // The other areas keep the editor's door.
    let room2 = &g.region(0).unwrap().rooms[2];
    let doors: Vec<_> = room2.items.iter().map(|it| (it.x, it.y, it.parameter)).collect();
    assert_eq!(doors, [(0, 5, 1)]);
}

/// Encounters 3.5: with an empty list an area has no way out.
#[test]
fn an_area_with_an_empty_list_has_no_way_out() {
    let a = encounter_adventure_with(&[(4, 15, 0)], &[(1, vec![])]);
    let (mut g, _) = pulled_in(&a);
    assert!(g.region(0).unwrap().rooms[1].items.is_empty());
    g.place(0, area(1, 1, 5));
    let ev = mv(&mut g, Dir::W);
    assert_eq!(blocked(&ev), Some(BlockReason::Wall));
    assert_eq!(g.players()[0].at, area(1, 1, 5));
}

/// Encounters 3.5, 7.1: with the editor's area door table a door of
/// parameter n leads to area n + 1's entrance, or the start when that area
/// is free or n is above 4.
#[test]
fn the_door_parameter_names_the_area_it_leads_to() {
    for n in [1u8, 9] {
        let a = encounter_adventure_with(&[(4, 15, 0)], &[(1, vec![(0, 5, 82, n)])]);
        let (mut g, _) = pulled_in(&a);
        g.place(0, area(1, 1, 5));
        let ev = mv(&mut g, Dir::W);
        assert_eq!(g.players()[0].at, Location::World { x: 20, y: 20 }, "n {n}");
        assert!(ev.iter().any(|e| matches!(
            e,
            Event::LeftArea {
                player: 0,
                area: 1,
                ..
            }
        )));
        assert!(g.region(0).unwrap().rooms[1].deleted);
    }
    // Area 2 open: area 1's door with n 1 leads to area 2's entrance.
    let a = encounter_adventure_with(&[(4, 15, 0)], &[(1, vec![(0, 5, 82, 1)])]);
    for seed in 0..400 {
        let (mut g, _) = Game::new(a.clone(), seed, Rules { diagonals: true }, PlayerSetup::default());
        g.apply(Command::AddPlayer {
            setup: PlayerSetup {
                name: "U".into(),
                ..Default::default()
            },
        });
        if encounters(&mv(&mut g, Dir::SW)).is_empty() || g.current() != 1 {
            continue;
        }
        if encounters(&mv(&mut g, Dir::SE)).is_empty() {
            continue;
        }
        assert_eq!(g.players()[1].at, area(2, 12, 4));
        assert_eq!(g.area_entrance(2), Some(PATCH2));
        for _ in 0..20 {
            if g.current() == 0 {
                break;
            }
            g.apply(Command::Wait);
        }
        assert_eq!(g.current(), 0);
        g.place(0, area(1, 1, 5));
        mv(&mut g, Dir::W);
        assert_eq!(
            g.players()[0].at,
            Location::World {
                x: PATCH2.0,
                y: PATCH2.1
            }
        );
        // Area 1 closed behind it; area 2 still open.
        assert!(g.region(0).unwrap().rooms[1].deleted);
        assert!(!g.region(0).unwrap().rooms[2].deleted);
        return;
    }
    panic!("no two encounters in 400 seeds");
}

/// Stores 2.7: a store in an area opens on shelves of every stockable
/// thing 1..80 with a value above 0, as many as the stock limit allows.
#[test]
fn a_store_in_an_area_offers_every_stockable_thing_of_value() {
    let a = encounter_adventure_with(&[(4, 15, 0)], &[(1, vec![(0, 5, 82, 0), (7, 3, SHOP as u8, 0)])]);
    let (mut g, _) = pulled_in(&a);
    g.place(0, area(1, 8, 3));
    let ev = mv(&mut g, Dir::W);
    assert!(ev.iter().any(|e| matches!(
        e,
        Event::StoreOpened {
            player: 0,
            region: 0,
            ..
        }
    )));
    // KEY and ROPE (treasures of value 1, things 1..32: 3 each); not SWIM
    // (a spell), nor the portals, spaces, obstacles or the store.
    let stock: Vec<_> = g.stock(0).iter().map(|s| (s.thing, s.count)).collect();
    assert_eq!(stock, [(KEY, 3), (ROPE, 3)]);
}

/// Stores 2.7: only stockable things 1..80 of value above 0; things 33..80
/// at most one each.
#[test]
fn the_map_regions_stock_follows_type_value_and_limit() {
    let mut a = encounter_adventure(&[(4, 15, 0)]);
    assert!(a.things.len() > 80);
    let treasure = a.things[KEY as usize - 1].clone();
    let mut zero = treasure.clone();
    if let acs_core::model::ThingKind::Treasure { carry, .. } = &mut zero.kind {
        carry.value = 0;
    }
    // KEY copied as thing 40 (limit 1) and 81 (past 80), one of value 0
    // as thing 41.
    a.things[39] = treasure.clone();
    a.things[40] = zero;
    a.things[80] = treasure;
    let (g, _) = Game::new(a, 1, Rules { diagonals: true }, PlayerSetup::default());
    let stock: Vec<_> = g.stock(0).iter().map(|s| (s.thing, s.count)).collect();
    assert_eq!(stock, [(KEY, 3), (ROPE, 3), (40, 1)]);
}

/// Encounters 3.5: a door on an inside square is a way out too.
#[test]
fn a_door_on_an_inside_square_is_a_way_out() {
    let a = encounter_adventure_with(&[(4, 15, 0)], &[(1, vec![(7, 5, 82, 0)])]);
    let (mut g, _) = pulled_in(&a);
    g.place(0, area(1, 8, 5));
    let before = g.current_player().moves_left;
    let ev = mv(&mut g, Dir::W);
    assert_eq!(
        g.players()[0].at,
        Location::World {
            x: PATCH.0,
            y: PATCH.1
        }
    );
    assert_eq!(g.current_player().moves_left, before - 1);
    assert!(ev.iter().any(|e| matches!(e, Event::AreaClosed { area: 1 })));
    // Closing keeps it (7.2).
    let doors: Vec<_> = g.region(0).unwrap().rooms[1]
        .items
        .iter()
        .map(|it| (it.x, it.y, it.thing))
        .collect();
    assert_eq!(doors, [(7, 5, 82)]);
}

/// Area 1 with doors of parameters `doors` (in list order) on 0,5, the
/// area door table `table` and + 0x80E `e`; the player on 1,5.
fn area_door_game(doors: &[u8], table: [u8; 8], e: u16) -> Game {
    let items = doors.iter().map(|&n| (0, 5, 82, n)).collect();
    let mut a = encounter_adventure_with(&[(4, 15, 0)], &[(1, items)]);
    a.world.unknown_806 = table.to_vec();
    a.world.unknown_80e = e;
    let (mut g, _) = pulled_in(&a);
    g.place(0, area(1, 1, 5));
    g
}

/// The editor's area door table with entry `n` set to `entry`.
fn with_entry(n: usize, entry: [u8; 2]) -> [u8; 8] {
    let mut t = wm::STOCK_AREA_DOORS;
    t[n * 2] = entry[0];
    t[n * 2 + 1] = entry[1];
    t
}

/// Encounters 3.5: the parameter picks an entry of the area door table,
/// read as a room door's destination with codes of its own for the areas.
#[test]
fn the_area_door_table_says_where_a_door_leads() {
    let start = Location::World { x: 20, y: 20 };
    let cases: [([u8; 2], Location); 7] = [
        // Map point 0: map door 0's square, not taken through it.
        ([0x00, 0x00], Location::World { x: 20, y: 19 }),
        // Map point 35: exit point 3.
        ([0x00, 35], Location::World { x: 5, y: 6 }),
        // An unused point, a code that is neither, a free area: the start.
        ([0x00, 0x01], start),
        ([0x00, 0x50], start),
        ([0x00, 0x42], start),
        // Region 1's deleted room 2, a region the adventure doesn't have.
        ([0x12, 0x22], start),
        ([0x32, 0x08], start),
    ];
    for (entry, to) in cases {
        let mut g = area_door_game(&[0], with_entry(0, entry), wm::STOCK_80E);
        let before = g.current_player().moves_left;
        let ev = mv(&mut g, Dir::W);
        assert_eq!(g.players()[0].at, to, "{entry:02x?}");
        // Not a step: no roll, no door taken; one move used.
        assert!(encounters(&ev).is_empty());
        assert!(!ev
            .iter()
            .any(|e| matches!(e, Event::TookDoor { .. } | Event::EnteredRoom { .. })));
        assert_eq!(g.current_player().moves_left, before - 1, "{entry:02x?}");
        assert!(ev.iter().any(|e| matches!(e, Event::AreaClosed { area: 1 })));
    }
    // Area 1's own code: its entrance (area 1 is still open when read).
    let mut g = area_door_game(&[0], with_entry(0, [0x00, 0x41]), wm::STOCK_80E);
    mv(&mut g, Dir::W);
    assert_eq!(
        g.players()[0].at,
        Location::World {
            x: PATCH.0,
            y: PATCH.1
        }
    );
}

/// Encounters 3.5: region 1 or above is a region change, as a room door
/// into another region: the turn ends and the moves left are carried.
#[test]
fn an_area_door_into_a_region_is_a_region_change() {
    let mut g = area_door_game(&[0], with_entry(0, [0x10, 0x23]), wm::STOCK_80E);
    let ev = mv(&mut g, Dir::W);
    assert_eq!(g.players()[0].at, hall(2, 3));
    assert!(ev
        .iter()
        .any(|e| matches!(e, Event::EnteredRegion { player: 0, region: 1 })));
    assert!(ev.iter().any(|e| matches!(
        e,
        Event::EnteredRoom {
            player: 0,
            region: 1,
            room: 0
        }
    )));
    assert!(ev.iter().any(|e| matches!(e, Event::TurnEnded { player: 0 })));
    assert!(ev.iter().any(|e| matches!(e, Event::AreaClosed { area: 1 })));
    assert!(g.region(0).unwrap().rooms[1].deleted);
}

/// Encounters 3.5: parameter 4 reads + 0x80E (low byte first); above 4,
/// the start; + 0x80E changes nothing for parameters 0..3.
#[test]
fn parameter_4_reads_the_word_at_0x80e() {
    let start = Location::World { x: 20, y: 20 };
    // 00 23 (map point 35) as the word 0x2300.
    let mut g = area_door_game(&[4], wm::STOCK_AREA_DOORS, 0x2300);
    mv(&mut g, Dir::W);
    assert_eq!(g.players()[0].at, Location::World { x: 5, y: 6 });
    // The corpus's 32 08: region 3, room 2, which the fixture lacks.
    let mut g = area_door_game(&[4], wm::STOCK_AREA_DOORS, wm::STOCK_80E);
    mv(&mut g, Dir::W);
    assert_eq!(g.players()[0].at, start);
    let mut g = area_door_game(&[5], wm::STOCK_AREA_DOORS, 0x2300);
    mv(&mut g, Dir::W);
    assert_eq!(g.players()[0].at, start);
    let mut g = area_door_game(&[0], with_entry(0, [0x00, 0x00]), 0x2300);
    mv(&mut g, Dir::W);
    assert_eq!(g.players()[0].at, Location::World { x: 20, y: 19 });
}

/// Encounters 3.5: with several doors on one square the first in the list
/// gives the parameter.
#[test]
fn the_first_door_on_a_square_gives_the_parameter() {
    let mut table = with_entry(1, [0x00, 35]);
    table[0..2].copy_from_slice(&[0x00, 0x00]);
    let mut g = area_door_game(&[0, 1], table, wm::STOCK_80E);
    mv(&mut g, Dir::W);
    assert_eq!(g.players()[0].at, Location::World { x: 20, y: 19 });
    let mut g = area_door_game(&[1, 0], table, wm::STOCK_80E);
    mv(&mut g, Dir::W);
    assert_eq!(g.players()[0].at, Location::World { x: 5, y: 6 });
}

/// Encounters 6.4: a creature leaves by the door item, wherever it is.
#[test]
fn a_creature_leaves_by_a_moved_door() {
    let a = encounter_adventure_with(&[(4, 15, 3)], &[(1, vec![(0, 3, 82, 0)])]);
    let (mut g, _) = pulled_in(&a);
    let c = g.creature_mut(0, 8).unwrap();
    c.strategy.alignment = acs_core::model::Alignment::Friend;
    c.strategy.aggressive = false;
    c.strategy.brave = false;
    c.wisdom = 20;
    c.x = 1;
    c.y = 3;
    let mut removed = false;
    for _ in 0..4 {
        let ev = g.apply(Command::Wait);
        if ev.iter().any(|e| {
            matches!(
                e,
                Event::CreatureRemoved {
                    region: 0,
                    slot: 8,
                    reason: acs_engine::RemovalReason::LeftArea
                }
            )
        }) {
            removed = true;
            break;
        }
    }
    assert!(removed);
}

#[test]
fn play_in_an_area_replays_exactly() {
    let a = encounter_adventure(&[(4, 15, 4)]);
    let (mut g, _) = pulled_in(&a);
    for _ in 0..30 {
        if g.is_over() {
            break;
        }
        g.apply(Command::Wait);
    }
    let r = Game::replay(
        a,
        g.seed(),
        Rules { diagonals: true },
        PlayerSetup::default(),
        g.log(),
    );
    assert_eq!(r, g);
}

#[test]
fn room_encounters_roll_on_entry_and_respect_the_region_limit() {
    let mut f = fixture(Platform::C64);
    f.build(); // lays out the rooms; patch the built bytes afterwards
    let base = f.l.regions;
    // HALL: random creature slot 1, 75%.
    f.put(base + rg::ROOMS + 4, &[1 << 4 | 15]);
    f.put(base + rg::RANDOM_CREATURES + 37, &creature_record(8, 0x50, 70));
    let image = acs_core::Image::from_bytes(Platform::C64, f.data.clone()).unwrap();
    let a = decode(&image).expect("decodes").adventure;
    assert_eq!(a.regions[0].rooms[0].random_chance, 75);
    assert_eq!(a.regions[0].rooms[0].random_creature, 1);
    let rolls = |a: &Adventure| {
        (0..400)
            .filter(|&seed| {
                let (mut g, _) =
                    Game::new(a.clone(), seed, Rules { diagonals: true }, PlayerSetup::default());
                mv(&mut g, Dir::N)
                    .iter()
                    .any(|e| matches!(e, Event::RoomEncounter { creature: 1, .. }))
            })
            .count()
    };
    let hits = rolls(&a);
    assert!((270..330).contains(&hits), "{hits}");

    // 16 creatures already in the region: no roll.
    let mut full = a.clone();
    let guard = full.regions[0].residents[0].clone();
    for c in full.regions[0].residents.iter_mut() {
        *c = guard.clone();
    }
    assert_eq!(rolls(&full), 0);
}

#[test]
fn replay_from_seed_and_log_is_exact() {
    let mut f = fixture(Platform::Pc);
    let w = f.l.world_map;
    f.put(w + wm::CREATURES, &creature_record(8, 0x50, 70));
    f.put(w + wm::CREATURE_CHANCES, &[15]);
    let a = build(&mut f);
    let setup = PlayerSetup::default();
    let (mut g, _) = Game::new(a.clone(), 1234, Rules { diagonals: true }, setup.clone());
    // A fixed walk with some refusals, door trips and waits.
    let mut events = Vec::new();
    for (i, d) in [
        Dir::N,
        Dir::E,
        Dir::E,
        Dir::N,
        Dir::E,
        Dir::E,
        Dir::E,
        Dir::E,
        Dir::W,
        Dir::S,
        Dir::W,
        Dir::W,
        Dir::W,
        Dir::W,
        Dir::W,
        Dir::SW,
    ]
    .into_iter()
    .enumerate()
    {
        events.extend(mv(&mut g, d));
        if i % 5 == 4 {
            events.extend(g.apply(Command::Wait));
        }
    }
    let r = Game::replay(a, 1234, Rules { diagonals: true }, setup, g.log());
    assert_eq!(r, g);
    // The log round-trips through JSON.
    let json = serde_json::to_string(g.log()).unwrap();
    let back: Vec<Command> = serde_json::from_str(&json).unwrap();
    assert_eq!(back, g.log());
    assert!(!events.is_empty());
}

/// A walk through the map door into HALL and about, with a rolled first
/// character; returns the game, the autosave (the game whenever the
/// engine's save point moved to it, as walk mode keeps it) and the
/// adventure.
fn saved_walk() -> (Game, Game, Adventure) {
    let a = adventure(Platform::C64);
    let first = Newcomer::Create {
        create: "SAVER".into(),
    };
    let (mut g, _) = Game::new(a.clone(), 77, Rules { diagonals: true }, first);
    g.place(0, Location::World { x: 20, y: 20 });
    assert_eq!(g.save_point(), 0);
    let mut autosave = g.clone();
    for d in [Dir::E, Dir::W, Dir::N, Dir::S, Dir::E, Dir::E] {
        mv(&mut g, d);
        if g.save_point() == g.log().len() {
            autosave = g.clone();
        }
    }
    g.apply(Command::Wait);
    mv(&mut g, Dir::W);
    (g, autosave, a)
}

#[test]
fn saves_replay_to_the_same_game_and_autosave() {
    let (g, autosave, a) = saved_walk();
    assert!(!autosave.log().is_empty() && autosave.log().len() < g.log().len());
    let save = SaveFile::new(&g, &a, &autosave);
    assert_eq!(save.placements.len(), 1);
    let back = SaveFile::from_json(&save.to_json()).unwrap();
    assert_eq!(back, save);
    let r = back.restore(&a).unwrap();
    assert_eq!(r.game, g);
    assert_eq!(r.autosave, autosave);
    assert!(!r.adventure_changed && !r.diverged);
    // A save made after resuming from the autosave holds the shorter line.
    let resumed = SaveFile::new(&autosave, &a, &autosave);
    assert_eq!(resumed.restore(&a).unwrap().game, autosave);
}

#[test]
fn saves_check_the_adventure_and_the_state() {
    let (g, autosave, a) = saved_walk();
    let save = SaveFile::new(&g, &a, &autosave);
    // Another adventure: refused.
    let mut other = a.clone();
    other.name = "ELSEWHERE".into();
    assert!(matches!(
        save.restore(&other),
        Err(SaveError::WrongAdventure { .. })
    ));
    // Same name, different data: loaded with a warning.
    let mut changed = a.clone();
    changed.byline = "A NEW BYLINE".into();
    let r = save.restore(&changed).unwrap();
    assert!(r.adventure_changed);
    // A different command line: the replay diverges, with a warning.
    let mut edited = save.clone();
    edited.commands.pop();
    let r = edited.restore(&a).unwrap();
    assert!(r.diverged && !r.adventure_changed);
    // Damaged and foreign files.
    let mut damaged = save.clone();
    damaged.autosave = save.commands.len() + 1;
    assert!(matches!(damaged.restore(&a), Err(SaveError::Damaged(_))));
    let mut nobody = save.clone();
    nobody.placements[0].player = 3;
    assert!(matches!(nobody.restore(&a), Err(SaveError::Damaged(_))));
    assert!(matches!(SaveFile::from_json("{}"), Err(SaveError::NotASave(_))));
    assert!(matches!(
        SaveFile::from_json(r#"{"format":"acsvm-save","version":99}"#),
        Err(SaveError::Version(99))
    ));
    assert!(matches!(
        SaveFile::from_json(r#"{"format":"other","version":1}"#),
        Err(SaveError::NotASave(_))
    ));
}

/// Saves 2.1: version 3 is written; a version 1 save still loads, its old
/// `fire` (refused, no move, no draw) read as `leave_store`, which plays
/// the same, so the command count, autosave and placements keep their
/// places and the game comes out the same.
#[test]
fn version_1_saves_load_with_fire_mapped() {
    let (g, autosave, a) = saved_walk();
    let save = SaveFile::new(&g, &a, &autosave);
    assert_eq!(save.version, 3);
    let mut v: serde_json::Value = serde_json::from_str(&save.to_json()).unwrap();
    v["version"] = 1.into();
    let commands = v["commands"].as_array_mut().unwrap();
    let at = commands.len() - 1;
    commands.insert(at, serde_json::json!({ "command": "fire" }));
    let old = SaveFile::from_json(&v.to_string()).unwrap();
    assert_eq!(old.commands[at], Command::LeaveStore);
    assert_eq!(old.commands.len(), g.log().len() + 1);
    let r = old.restore(&a).unwrap();
    assert_eq!(r.game.players(), g.players());
    assert_eq!(r.autosave, autosave);
    // The log holds the mapped command, so the state fingerprint differs:
    // loaded with the diverged warning.
    assert!(r.diverged);
    // A version 2 `fire` is the new command.
    let mut v2 = v.clone();
    v2["version"] = 2.into();
    let new = SaveFile::from_json(&v2.to_string()).unwrap();
    assert_eq!(
        new.commands[at],
        Command::Fire {
            target: None,
            weapon: None
        }
    );
}

/// Saves 2.1: a version 2 save loads as it is, its autosave (made at the
/// door step then) kept as written, and replays to the same game; a
/// version 3 save carries SAVE GAME and the autosave of creatures 2.2.7.
#[test]
fn version_2_saves_load_unchanged_and_save_game_replays() {
    let (mut g, autosave, a) = saved_walk();
    let save = SaveFile::new(&g, &a, &autosave);
    let mut v: serde_json::Value = serde_json::from_str(&save.to_json()).unwrap();
    v["version"] = 2.into();
    v["autosave"] = 2.into();
    let old = SaveFile::from_json(&v.to_string()).unwrap();
    let r = old.restore(&a).unwrap();
    assert_eq!(r.game, g);
    assert_eq!(r.autosave.log().len(), 2);
    assert!(!r.diverged && !r.adventure_changed);
    // SAVE GAME is a logged command; the save point comes back on replay.
    g.apply(Command::SaveGame);
    while g.save_point() != g.log().len() {
        g.apply(Command::Wait);
    }
    let at_save = g.clone();
    g.apply(Command::Wait);
    let saved = SaveFile::new(&g, &a, &at_save);
    assert_eq!(saved.autosave, g.save_point());
    let back = SaveFile::from_json(&saved.to_json())
        .unwrap()
        .restore(&a)
        .unwrap();
    assert_eq!(back.game, g);
    assert_eq!(back.autosave, at_save);
    assert_eq!(back.game.save_point(), g.save_point());
    assert!(back.game.log().contains(&Command::SaveGame));
}

#[test]
fn idle_drain_is_a_wait_one_step_at_a_time() {
    // Movement 2.4, magic 5.3: each drained step is an idle move with its
    // healing roll, so draining a turn away draws what one Wait draws.
    let a = adventure(Platform::C64);
    let setup = PlayerSetup {
        speed: 6,
        constitution: 20,
        life_force: 4,
        ..Default::default()
    };
    let new = || Game::new(a.clone(), 31, Rules::default(), setup.clone()).0;
    let (mut waited, mut idled) = (new(), new());
    for turn in 0..12 {
        mv(&mut waited, Dir::E);
        mv(&mut idled, Dir::E);
        let ev = waited.apply(Command::Wait);
        let left = idled.current_player().moves_left;
        let mut got = Vec::new();
        for _ in 0..left {
            got.extend(idled.apply(Command::Idle));
        }
        assert_eq!(got, ev, "turn {turn}");
        assert_eq!(idled.players(), waited.players(), "turn {turn}");
    }
    assert!(idled.current_player().life_force > 4, "healed while idling");
    // Idle halfway, then walk: the steps drained are gone.
    let full = idled.current_player().moves_left;
    idled.apply(Command::Idle);
    assert_eq!(idled.current_player().moves_left, full - 1);
}

// ---- encounters as captured (capture.md 10.20) -----------------------------
//
// Exact generator states are compared only when nothing hit: a hit ends the
// turn, and the next round starts (with its own draws) in the same apply.

/// The generator's state, read from the game's serialized form.
fn rng_of(g: &Game) -> acs_engine::Rng {
    serde_json::from_value(serde_json::to_value(g).expect("serializes")["rng"].clone()).expect("an Rng")
}

fn fast_game(a: &Adventure, seed: u64) -> Game {
    Game::new(
        a.clone(),
        seed,
        Rules { diagonals: true },
        PlayerSetup {
            speed: 15,
            ..Default::default()
        },
    )
    .0
}

#[test]
fn a_step_rolls_once_per_record_on_the_terrain_and_nothing_else() {
    // WOLF on the patch (terrain 4); records 1..7 empty on terrain 0.
    let a = encounter_adventure(&[(4, 15, 4)]);
    let (mut hit, mut miss) = (false, false);
    for seed in 0..60 {
        // Onto the patch: WOLF's one roll, nothing else, hit or miss (2.5,
        // 2.7).
        let mut g = fast_game(&a, seed);
        let mut want = rng_of(&g);
        let hits = want.percent(60);
        let ev = mv(&mut g, Dir::SW);
        assert_eq!(encounters(&ev) == [(0, Some(8))], hits, "seed {seed}");
        if hits {
            hit = true;
            assert_eq!(g.players()[0].at, area(1, 12, 4));
        } else {
            // A miss leaves the turn going: the one roll was all.
            miss = true;
            assert_eq!(rng_of(&g), want, "seed {seed}");
        }
        // Onto grass (terrain 0): the 7 empty records roll against 0 and
        // never hit.
        let mut g = fast_game(&a, seed);
        let mut want = rng_of(&g);
        assert!(!(0..7).any(|_| want.percent(0)));
        let ev = mv(&mut g, Dir::SE);
        assert!(encounters(&ev).is_empty());
        assert_eq!(rng_of(&g), want);
    }
    assert!(hit && miss);
    // WOLF on the grass: 8 rolls, WOLF's first.
    let a = encounter_adventure(&[(0, 15, 4)]);
    for seed in 0..60 {
        let mut g = fast_game(&a, seed);
        let mut want = rng_of(&g);
        let hits = want.percent(60);
        assert!(!(0..7).any(|_| want.percent(0)));
        let ev = mv(&mut g, Dir::SE);
        assert_eq!(!encounters(&ev).is_empty(), hits, "seed {seed}");
        if !hits {
            assert_eq!(rng_of(&g), want, "seed {seed}");
        }
    }
}

#[test]
fn code_15_hits_on_59_and_misses_on_60() {
    let a = encounter_adventure(&[(4, 15, 4)]);
    let (mut saw59, mut saw60) = (false, false);
    for seed in 0..20_000 {
        let g = fast_game(&a, seed);
        let roll = rng_of(&g).below(100);
        if roll != 59 && roll != 60 {
            continue;
        }
        let mut g = g;
        let ev = mv(&mut g, Dir::SW);
        assert_eq!(!encounters(&ev).is_empty(), roll == 59, "seed {seed}");
        saw59 |= roll == 59;
        saw60 |= roll == 60;
        if saw59 && saw60 {
            return;
        }
    }
    panic!("no seed rolled 59 and 60");
}

#[test]
fn the_player_lands_on_12_4_entering_east_or_north() {
    let a = encounter_adventure(&[(4, 15, 4)]);
    for (from, dir) in [((18, 21), Dir::E), ((19, 22), Dir::N)] {
        let mut found = false;
        for seed in 0..200 {
            let mut g = fast_game(&a, seed);
            g.place(0, Location::World { x: from.0, y: from.1 });
            let ev = mv(&mut g, dir);
            if encounters(&ev).is_empty() {
                continue;
            }
            // 3.1, 3.6, 5.1: area 1, 12,4, the entrance the square entered,
            // the turn over.
            assert_eq!(g.players()[0].at, area(1, 12, 4));
            assert_eq!(g.area_entrance(1), Some(PATCH));
            assert!(ev.iter().any(|e| matches!(e, Event::TurnEnded { player: 0 })));
            found = true;
            break;
        }
        assert!(found, "{dir:?}");
    }
}

#[test]
fn one_copy_per_hit_in_slots_8_and_9_by_record() {
    let a = encounter_adventure(&[(4, 15, 4), (4, 15, 3)]);
    let (mut both, mut hare_alone) = (false, false);
    for seed in 0..400 {
        let mut g = fast_game(&a, seed);
        let mut want = rng_of(&g);
        let (wolf, hare) = (want.percent(60), want.percent(60));
        let hits = encounters(&mv(&mut g, Dir::SW));
        match (wolf, hare) {
            (true, true) => {
                both = true;
                assert_eq!(hits, [(0, Some(8)), (1, Some(9))]);
                assert_eq!(g.creature(0, 9).map(|c| (c.x, c.y)), Some((3, 3)));
            }
            (false, true) => {
                // 3.6: record 1 on 3,3 even alone in slot 8.
                hare_alone = true;
                assert_eq!(hits, [(1, Some(8))]);
                assert_eq!(g.creature(0, 8).map(|c| (c.x, c.y, c.room)), Some((3, 3, 1)));
                assert!(g.creature(0, 9).is_none());
            }
            (true, false) => assert_eq!(hits, [(0, Some(8))]),
            (false, false) => {
                // The two rolls and nothing more.
                assert!(hits.is_empty());
                assert_eq!(rng_of(&g), want);
            }
        }
    }
    assert!(both && hare_alone);
}

#[test]
fn walking_out_keeps_the_moves_left_and_the_next_step_rolls() {
    let a = encounter_adventure(&[(4, 15, 0)]);
    let (mut g, _) = pulled_in(&a);
    // The next round: T in area 1 with a fresh turn.
    assert_eq!(g.current(), 0);
    g.place(0, area(1, 1, 5));
    let before = g.current_player().moves_left;
    let mut want = rng_of(&g);
    let ev = mv(&mut g, Dir::W);
    // 7.1: on the entrance with one move used, no roll for the return.
    assert_eq!(
        g.players()[0].at,
        Location::World {
            x: PATCH.0,
            y: PATCH.1
        }
    );
    assert_eq!(g.current_player().moves_left, before - 1);
    // 7.2: closed and free, the entrance forgotten, slot 8 empty.
    assert!(ev.iter().any(|e| matches!(
        e,
        Event::CreatureRemoved {
            region: 0,
            slot: 8,
            reason: acs_engine::RemovalReason::AreaClosed
        }
    )));
    assert!(g.region(0).unwrap().rooms[1].deleted);
    assert_eq!(g.area_entrance(1), None);
    assert!(g.creature(0, 8).is_none());
    assert_eq!(rng_of(&g), want);
    // 2.3: no cool-down, the next step onto the patch's terrain rolls. Step
    // off west onto grass first (7 empty records), then back on (WOLF's
    // roll).
    assert!(g.current_player().moves_left >= 2);
    assert!(!(0..7).any(|_| want.percent(0)));
    mv(&mut g, Dir::W);
    assert_eq!(rng_of(&g), want);
    let hits = want.percent(60);
    let ev = mv(&mut g, Dir::E);
    assert_eq!(!encounters(&ev).is_empty(), hits);
    if !hits {
        assert_eq!(rng_of(&g), want);
    }
}
