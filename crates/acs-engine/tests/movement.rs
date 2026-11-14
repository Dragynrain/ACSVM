// SPDX-License-Identifier: GPL-3.0-or-later

//! Movement rules (`docs/rules/movement.md`) on synthetic adventures built
//! byte by byte with acs-core's fixture builder. No real adventure data.

#[path = "../../acs-core/tests/common/mod.rs"]
mod common;

use acs_core::layout::{adventure as adv, region as rg, world_map as wm};
use acs_core::{decode, Adventure, Platform};
use acs_engine::{BlockReason, Command, Dir, Event, Game, Location, PlayerSetup, Rules, SpellSource};
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

/// Builds an adventure with one region of three rooms (one deleted):
///
/// - room 0 "HALL" 8x6 at grid (0,0); room 1 "CELL" 5x6 at grid (7,0), sharing
///   HALL's east wall; room 2 deleted.
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
        ],
        residents: vec![(0, "GUARD", guard)],
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
    let ev = g.apply(Command::AddPlayer {
        setup: PlayerSetup {
            name: "TWO".into(),
            speed: 1,
            ..Default::default()
        },
    });
    assert!(matches!(ev[0], Event::PlayerAdded { player: 1, .. }));
    // Creatures 2.2: a player added during a round joins the next one,
    // after the players before it.
    g.apply(Command::Wait);
    assert_eq!(g.current(), 0);
    g.apply(Command::Wait);
    assert_eq!(g.current(), 1);
    mv(&mut g, Dir::S); // one step uses TWO's whole bar
    assert_eq!(g.current(), 0);
}

/// Movement 1.4, 1.5: four living characters at most; names cut to 15.
#[test]
fn at_most_four_players() {
    let mut g = game(Platform::Pc);
    let add = |name: &str| Command::AddPlayer {
        setup: PlayerSetup {
            name: name.into(),
            ..Default::default()
        },
    };
    for n in ["TWO", "THREE", "A NAME FAR TOO LONG"] {
        let ev = g.apply(add(n));
        assert!(matches!(ev[0], Event::PlayerAdded { .. }));
    }
    assert_eq!(g.players().len(), 4);
    assert_eq!(g.players()[3].name, "A NAME FAR TOO ");
    let start = g.players()[3].at;
    assert_eq!(g.players()[1].at, start);
    assert_eq!(g.apply(add("FIVE")), vec![Event::AddRefused]);
    assert_eq!(g.players().len(), 4);
    // Every one of the four gets a turn, in joining order.
    let mut seen = Vec::new();
    for _ in 0..8 {
        seen.push(g.current());
        g.apply(Command::Wait);
    }
    assert_eq!(seen, [0, 0, 1, 2, 3, 0, 1, 2]);
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
    g.apply(Command::CreatePlayer { name: "two".into() });
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

#[test]
fn map_encounters_roll_for_the_terrain_stood_on() {
    let mut f = fixture(Platform::Pc);
    let w = f.l.world_map;
    f.put(w + wm::CREATURES, &creature_record(8, 0x50, 70));
    f.packed(w + wm::CREATURE_NAMES, "WOLF");
    f.put(w + wm::CREATURE_CHANCES, &[15]); // grass, (15-3)*5 = 60%
    let a = build(&mut f);
    assert_eq!(a.world.random_creatures[0].chance, 60);
    let mut hits = 0;
    for seed in 0..500 {
        let (mut g, _) = Game::new(a.clone(), seed, Rules { diagonals: true }, PlayerSetup::default());
        let ev = mv(&mut g, Dir::SW);
        if ev.iter().any(|e| {
            matches!(
                e,
                Event::Encounter {
                    creature: 0,
                    terrain: 0,
                    ..
                }
            )
        }) {
            hits += 1;
        }
        // The move goes ahead either way (roll only for now).
        assert_eq!(g.current_player().at, Location::World { x: 19, y: 21 });
    }
    assert!((250..350).contains(&hits), "{hits}");
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
