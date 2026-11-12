// SPDX-License-Identifier: GPL-3.0-or-later

//! Creature rules (`docs/rules/creatures.md`) on a synthetic adventure built
//! byte by byte with acs-core's fixture builder. No real adventure data.

#[path = "../../acs-core/tests/common/mod.rs"]
mod common;

use acs_core::layout::world_map as wm;
use acs_core::model::{Alignment, Possession, RoomItem};
use acs_core::{decode, Adventure, Platform};
use acs_engine::inventory::GOLD;
use acs_engine::{Command, Dir, Event, Game, Location, PlayerSetup, RemovalReason, Rules, Who};
use common::{creature_record, Fixture, RegionSpec, RoomSpec};

const GEM: u16 = 1; // treasure, weight 1
const ROCK: u16 = 2; // treasure, weight 200
const SWORD: u16 = 3; // melee weapon
const BOW: u16 = 4; // missile weapon
const WAND: u16 = 40; // magic item
const DOOR: u16 = 82; // portal, anyone
const RIDDOOR: u16 = 83; // portal, invokes Rid Room
const PILLAR: u16 = 84; // obstacle

/// One region "DEN":
///
/// - WEST 10 x 7 (inside 1..8 x 1..5): east door (9,3) -> EAST (1,3); west
///   door (0,3) -> the deleted room LOST; a Rid Room door (4,6) -> EAST (5,5);
///   a door (4,0) to the world map.
/// - EAST 10 x 7: west door (0,3) -> WEST (8,3).
/// - Slot 0 holds a template creature (not in use until a test puts it).
fn fixture(platform: Platform) -> Fixture {
    let mut f = Fixture::new(platform);
    f.header("DEN", "", "");
    f.thing(GEM as usize, "GEM", [0x01, 1, 50, 5]);
    f.thing(ROCK as usize, "ROCK", [0x01, 200, 1, 4]);
    f.thing(SWORD as usize, "SWORD", [0x04, 5, 10, 0]);
    f.extended(None, 0, [5, 0, 20 | 0x40]);
    f.thing(BOW as usize, "BOW", [0x03, 3, 10, 1]);
    f.extended(None, 1, [3, 0, 21 | 0x40]);
    f.thing(WAND as usize, "WAND", [0x02, 1, 1, 0]);
    f.extended(Some(12), 0, [10, 7, 3]);
    f.thing(GOLD as usize, "GOLD", [0x01, 1, 1, 9]);
    f.thing(DOOR as usize, "DOOR", [0x27, 5, 0, 0]);
    f.thing(RIDDOOR as usize, "RID DOOR", [0x07, 5 | 1 << 6, 12, 0]);
    f.thing(PILLAR as usize, "PILLAR", [0x0A, 11, 0, 0]);
    f.thing(127, "EMPTY FLOOR", [0x00, 1, 0, 0]);
    f.thing(128, "ROOM BORDER", [0x00, 12, 0, 0]);
    let mut template = creature_record(10, 0x60, 70);
    template[0x10] = 0x10;
    f.region(RegionSpec {
        name: "DEN",
        rooms: vec![
            RoomSpec {
                name: "WEST",
                w: 10,
                h: 7,
                wall: 12,
                items: vec![
                    (9, 3, (DOOR as u8), 1),
                    (0, 3, (DOOR as u8), 3),
                    (4, 6, (RIDDOOR as u8), 4),
                    (4, 0, (DOOR as u8), 5),
                ],
                ..Default::default()
            },
            RoomSpec {
                name: "EAST",
                w: 10,
                h: 7,
                x: 9,
                wall: 12,
                items: vec![(0, 3, (DOOR as u8), 2)],
                ..Default::default()
            },
            RoomSpec {
                name: "LOST",
                w: 1,
                h: 1,
                x: 30,
                wall: 12,
                ..Default::default()
            },
        ],
        portals: vec![
            [0, 0],
            [0x11, 0x13],
            [0x10, 0x83],
            [0x12, 0x33],
            [0x11, 0x55],
            [0x00, 0x23],
        ],
        residents: vec![(0, "TEMPLATE", template)],
        ..Default::default()
    });
    let w = f.l.world_map;
    f.put(w + wm::TERRAIN, &[1 | 1 << 6, 0]);
    f.put(w + wm::TERRAIN + 12 * 2, &[5 | 1 << 6, 0]); // door terrain, open
    f.put(w + wm::START_X, &[20, 20]);
    f.put(w + wm::POINTS + (32 + 3) * 2, &[5, 6]);
    // Map door 0 at (20,19) -> WEST (2,2).
    let cell = w + wm::MAP + 19 * 20 + 10;
    f.data[cell] = (f.data[cell] & 0x0F) | 12 << 4;
    f.put(w + wm::POINTS, &[20, 19]);
    f.put(w + wm::DOOR_DESTINATIONS, &[0x10, 0x22]);
    f
}

fn adventure(platform: Platform) -> Adventure {
    let mut a = decode(&fixture(platform).build()).expect("decodes").adventure;
    a.regions[0].residents[0].in_use = false;
    a
}

/// What a test puts in a slot.
#[derive(Clone, Copy)]
struct Spec {
    room: u8,
    x: u8,
    y: u8,
    alignment: Alignment,
    aggressive: bool,
    brave: bool,
    speed: u8,
}

fn spec(room: u8, x: u8, y: u8, alignment: Alignment, aggressive: bool) -> Spec {
    Spec {
        room,
        x,
        y,
        alignment,
        aggressive,
        brave: true,
        speed: 15,
    }
}

fn put(a: &mut Adventure, slot: usize, s: Spec) {
    let mut c = a.regions[0].residents[0].clone();
    c.in_use = true;
    c.name = format!("C{slot}");
    c.number = u16::from(slot as u8 + 1);
    c.region = 1;
    c.room = s.room;
    c.x = s.x;
    c.y = s.y;
    c.strategy.alignment = s.alignment;
    c.strategy.aggressive = s.aggressive;
    c.strategy.brave = s.brave;
    c.speed = s.speed;
    a.regions[0].residents[slot] = c;
}

fn item(a: &mut Adventure, room: usize, x: u8, y: u8, thing: u16, parameter: u8) {
    a.regions[0].rooms[room].items.insert(
        0,
        RoomItem {
            x,
            y,
            thing,
            parameter,
            flag: false,
            destination: None,
        },
    );
}

fn start(a: Adventure, room: u8, x: u8, y: u8) -> Game {
    start_seeded(a, 3, room, x, y)
}

fn start_seeded(a: Adventure, seed: u64, room: u8, x: u8, y: u8) -> Game {
    let (mut g, _) = Game::new(
        a,
        seed,
        Rules { diagonals: true },
        PlayerSetup {
            speed: 15,
            ..Default::default()
        },
    );
    g.place(0, at(room, x, y));
    g
}

fn at(room: u8, x: u8, y: u8) -> Location {
    Location::Room {
        region: 1,
        room,
        x,
        y,
    }
}

fn wait(g: &mut Game) -> Vec<Event> {
    g.apply(Command::Wait)
}

fn xy(g: &Game, slot: usize) -> (u8, u8, u8) {
    let c = g.creature(1, slot).expect("in use");
    (c.room, c.x, c.y)
}

#[allow(dead_code)]
fn adjacent(a: (u8, u8), b: (u8, u8)) -> bool {
    a != b && (a.0 as i16 - b.0 as i16).abs() <= 1 && (a.1 as i16 - b.1 as i16).abs() <= 1
}

fn top(g: &Game, room: usize, x: u8, y: u8) -> Option<u16> {
    g.adventure().regions[0].rooms[room]
        .items
        .iter()
        .find(|it| it.x == x && it.y == y)
        .map(|it| it.thing)
}

fn both(test: impl Fn(Platform)) {
    test(Platform::Pc);
    test(Platform::C64);
}

#[test]
fn residents_wait_until_a_player_is_in_their_room() {
    both(|p| {
        let mut a = adventure(p);
        put(&mut a, 1, spec(1, 5, 3, Alignment::Enemy, true));
        let mut g = start(a, 0, 2, 2);
        for _ in 0..5 {
            let ev = wait(&mut g);
            assert!(
                !ev.iter().any(|e| matches!(e, Event::CreatureTurn { .. })),
                "{ev:?}"
            );
        }
        assert_eq!(xy(&g, 1), (1, 5, 3));
    });
}

#[test]
fn fighter_enemy_walks_up_to_the_player_and_squares_up() {
    both(|p| {
        let mut a = adventure(p);
        put(&mut a, 1, spec(0, 7, 5, Alignment::Enemy, true));
        let mut g = start(a, 0, 2, 2);
        let ev = wait(&mut g);
        assert!(ev.contains(&Event::CreatureTurn {
            region: 1,
            slot: 1,
            moves: 15
        }));
        // 6.5: steps only to the 4 squares beside; 7.1: beside its foe, it
        // squares up and its turn ends.
        let (_, x, y) = xy(&g, 1);
        let beside = (x as i16 - 2).abs() + (y as i16 - 2).abs() == 1;
        assert!(beside, "ended at {x},{y}");
        assert!(ev.contains(&Event::CreatureConfronts {
            region: 1,
            slot: 1,
            target: Who::Player { player: 0 }
        }));
        // 2.6: beside an aggressive enemy, the bar is halved.
        assert!(ev.contains(&Event::TurnStarted {
            player: 0,
            moves: 7,
            hindered: true
        }));
        for e in &ev {
            if let Event::CreatureMoved { from, to, .. } = e {
                let (a, b) = (from.xy(), to.xy());
                assert_eq!(a.0.abs_diff(b.0) + a.1.abs_diff(b.1), 1, "{e:?}");
            }
        }
    });
}

#[test]
fn friends_do_not_hinder_and_do_not_confront_players() {
    both(|p| {
        let mut a = adventure(p);
        put(&mut a, 1, spec(0, 3, 3, Alignment::Friend, true));
        let mut g = start(a, 0, 2, 2);
        let ev = wait(&mut g);
        assert!(!ev.iter().any(|e| matches!(e, Event::CreatureConfronts { .. })));
        assert!(ev.contains(&Event::TurnStarted {
            player: 0,
            moves: 15,
            hindered: false
        }));
    });
}

#[test]
fn creatures_follow_a_player_through_the_door_they_took() {
    both(|p| {
        // On 6,3: the east door (3 steps) is its nearest exit, not the
        // west one into the deleted room (6).
        let mut a = adventure(p);
        put(&mut a, 1, spec(0, 6, 3, Alignment::Enemy, true));
        let mut g = start(a, 0, 8, 2);
        // Through the east door onto EAST (1,3): a door into another room
        // ends the turn (movement 2.3), so the creature's turn follows.
        let ev = g.apply(Command::Move { dir: Dir::SE });
        assert_eq!(g.current_player().at, at(1, 1, 3));
        assert!(ev.contains(&Event::TurnEnded { player: 0 }), "{ev:?}");
        let took = ev.iter().find_map(|e| match e {
            Event::CreatureTookDoor {
                slot: 1, from, to, ..
            } => Some((*from, *to)),
            _ => None,
        });
        // 6.2: the player has left, so its goal is the exit (rank 96); 5.4:
        // it lands on the destination, sharing the player's square.
        let (from, to) = took.expect("followed");
        assert_eq!(from, at(0, 9, 3));
        assert_eq!(to, at(1, 1, 3));
        // Next round it acts in EAST and squares up.
        let ev = wait(&mut g);
        assert!(ev
            .iter()
            .any(|e| matches!(e, Event::CreatureConfronts { slot: 1, .. })));
    });
}

#[test]
fn slinker_neutrals_head_for_the_nearest_exit_they_may_take() {
    both(|p| {
        // Next to the Rid Room door (closed to creatures) and two squares
        // from the door into the deleted room: banished (5.3, 5.5).
        let mut a = adventure(p);
        put(&mut a, 1, spec(0, 2, 5, Alignment::Neutral, false));
        let mut g = start(a.clone(), 0, 7, 1);
        let ev = wait(&mut g);
        assert!(ev.contains(&Event::CreatureRemoved {
            region: 1,
            slot: 1,
            reason: RemovalReason::Banished
        }));
        assert!(g.creature(1, 1).is_none());

        // Next to the world map door and three from the east door: it takes
        // the east door (5.3: same region only) and stops in EAST.
        let mut a = adventure(p);
        put(&mut a, 1, spec(0, 6, 1, Alignment::Neutral, false));
        let mut g = start(a, 0, 1, 5);
        let ev = wait(&mut g);
        assert!(ev
            .iter()
            .any(|e| matches!(e, Event::CreatureTookDoor { slot: 1, .. })));
        assert_eq!(xy(&g, 1), (1, 1, 3));
        // EAST has no player: it no longer acts.
        let ev = wait(&mut g);
        assert!(!ev.iter().any(|e| matches!(e, Event::CreatureTurn { .. })));
    });
}

#[test]
fn nearer_beats_better_by_rank_and_steps() {
    both(|p| {
        // 6.2-6.4: a slinker friend scores the player 26 + 9 steps = 35 and
        // the east door 32 + 1 = 33: it takes the door.
        let mut a = adventure(p);
        put(&mut a, 1, spec(0, 8, 3, Alignment::Friend, false));
        let mut g = start(a, 0, 1, 1);
        let ev = wait(&mut g);
        assert!(
            ev.iter()
                .any(|e| matches!(e, Event::CreatureTookDoor { slot: 1, .. })),
            "{ev:?}"
        );
    });
}

#[test]
fn thieves_take_treasure_and_gold_but_never_step_on_magic_items() {
    both(|p| {
        let mut a = adventure(p);
        put(&mut a, 1, spec(0, 2, 3, Alignment::Thief, false));
        item(&mut a, 0, 4, 3, WAND, 1);
        item(&mut a, 0, 6, 3, GEM, 1);
        item(&mut a, 0, 6, 5, GOLD, 5);
        // Walls of pillars force the path round the wand.
        item(&mut a, 0, 4, 2, PILLAR, 0);
        let mut g = start(a, 0, 1, 1);
        let mut stood = Vec::new();
        for _ in 0..3 {
            for e in wait(&mut g) {
                if let Event::CreatureMoved { slot: 1, to, .. } = e {
                    stood.push(to);
                }
            }
        }
        let c = g.creature(1, 1).cloned();
        let gem = c.as_ref().map(|c| c.possessions.iter().any(|p| p.thing == GEM));
        assert_eq!(gem, Some(true), "{c:?}");
        assert_eq!(c.map(|c| c.wealth), Some(5));
        assert!(!stood.contains(&at(0, 4, 3)), "stepped on the wand");
        assert_eq!(top(&g, 0, 4, 3), Some(WAND));
    });
}

#[test]
fn friends_leave_gold_lying() {
    both(|p| {
        let mut a = adventure(p);
        put(&mut a, 1, spec(0, 7, 4, Alignment::Friend, true));
        item(&mut a, 0, 6, 4, GOLD, 3);
        let mut g = start(a, 0, 2, 4);
        for _ in 0..3 {
            let ev = wait(&mut g);
            assert!(!ev.iter().any(|e| matches!(e, Event::CreatureGoldPickedUp { .. })));
        }
        // 5.2: gold is closed to friends, so it walks round it.
        assert_eq!(top(&g, 0, 6, 4), Some(GOLD));
    });
}

#[test]
fn overburdened_creatures_drop_a_treasure_and_leave_it() {
    both(|p| {
        let mut a = adventure(p);
        let mut s = spec(0, 5, 3, Alignment::Enemy, false);
        s.speed = 0;
        put(&mut a, 1, s);
        // Capacity (12 - 0) x 100 = 1200; three rocks (600) and 800 gold.
        let c = &mut a.regions[0].residents[1];
        c.possessions = vec![Possession {
            thing: ROCK,
            count: 3,
        }];
        c.wealth = 800;
        let mut g = start(a, 0, 1, 1);
        let ev = wait(&mut g);
        assert!(ev.contains(&Event::CreatureDropped {
            region: 1,
            slot: 1,
            thing: ROCK,
            amount: 1,
            at: Some(at(0, 5, 3))
        }));
        let ev = wait(&mut g);
        assert!(!ev.iter().any(|e| matches!(e, Event::CreatureDropped { .. })));
        assert_eq!(top(&g, 0, 5, 3), Some(ROCK));
    });
}

#[test]
fn overburdened_creatures_drop_just_enough_gold() {
    both(|p| {
        let mut a = adventure(p);
        let mut s = spec(0, 5, 3, Alignment::Enemy, false);
        s.speed = 0;
        put(&mut a, 1, s);
        a.regions[0].residents[1].wealth = 1300;
        let mut g = start(a, 0, 1, 1);
        let ev = wait(&mut g);
        assert!(ev.contains(&Event::CreatureDropped {
            region: 1,
            slot: 1,
            thing: GOLD,
            amount: 100,
            at: Some(at(0, 5, 3))
        }));
        assert_eq!(g.creature(1, 1).unwrap().wealth, 1200);
    });
}

#[test]
fn creatures_ready_a_missile_weapon_when_safe_and_melee_when_not() {
    both(|p| {
        let mut a = adventure(p);
        let mut s = spec(0, 5, 3, Alignment::Enemy, true);
        s.speed = 0;
        put(&mut a, 1, s);
        a.regions[0].residents[1].possessions = vec![
            Possession {
                thing: SWORD,
                count: 1,
            },
            Possession { thing: BOW, count: 1 },
        ];
        let mut g = start(a.clone(), 0, 1, 1);
        let ev = wait(&mut g);
        assert!(ev.contains(&Event::CreatureReadied {
            region: 1,
            slot: 1,
            thing: BOW
        }));
        let mut g = start(a, 0, 4, 3);
        let ev = wait(&mut g);
        assert!(ev.contains(&Event::CreatureReadied {
            region: 1,
            slot: 1,
            thing: SWORD
        }));
    });
}

#[test]
fn a_friend_next_to_the_player_drops_a_magic_item_beside_them() {
    both(|p| {
        let mut a = adventure(p);
        put(&mut a, 1, spec(0, 3, 3, Alignment::Friend, false));
        a.regions[0].residents[1].possessions = vec![Possession {
            thing: WAND,
            count: 1,
        }];
        let mut g = start(a, 0, 3, 2);
        let ev = wait(&mut g);
        // First free square round the player from N: (3,1).
        assert!(ev.contains(&Event::CreatureDropped {
            region: 1,
            slot: 1,
            thing: WAND,
            amount: 1,
            at: Some(at(0, 3, 1))
        }));
        assert_eq!(top(&g, 0, 3, 1), Some(WAND));
    });
}

#[test]
fn random_creatures_appear_on_an_empty_square_and_go_when_left_behind() {
    both(|p| {
        let mut a = adventure(p);
        let mut bat = a.regions[0].residents[0].clone();
        bat.in_use = true;
        bat.name = "BAT".into();
        bat.speed = 0;
        a.regions[0].random_creatures[2] = bat;
        a.regions[0].rooms[1].random_chance = 100;
        a.regions[0].rooms[1].random_creature = 2;
        item(&mut a, 1, 5, 5, PILLAR, 0);
        let mut g = start(a, 0, 8, 3);
        let ev = g.apply(Command::Move { dir: Dir::E });
        let (slot, square) = ev
            .iter()
            .find_map(|e| match e {
                Event::RoomEncounter {
                    creature: 2,
                    slot: Some(s),
                    at: Some(at),
                    ..
                } => Some((*s, *at)),
                _ => None,
            })
            .expect("appeared");
        assert_eq!(slot, 0, "lowest free slot");
        assert!(g.is_random(1, 0));
        let Location::Room { room: 1, x, y, .. } = square else {
            panic!("{square:?}")
        };
        assert!((1..9).contains(&x) && (1..6).contains(&y));
        assert_ne!((x, y), (5, 5));
        assert_ne!((x, y), (1, 3));
        // Back west (the door into another room ends the turn, movement
        // 2.3): the bat cannot move and is left behind. 2.4: each round a
        // slot roll (1 in 16 hits its slot, the only one in use), then one
        // percentile roll under 20.
        let mut ev = g.apply(Command::Move { dir: Dir::W });
        let gone = |ev: &[Event]| {
            ev.contains(&Event::CreatureRemoved {
                region: 1,
                slot: 0,
                reason: RemovalReason::LeftBehind,
            })
        };
        let mut rounds = 0;
        while !gone(&ev) {
            assert!(rounds < 3000, "never removed");
            ev = wait(&mut g);
            rounds += 1;
        }
        assert!(g.creature(1, 0).is_none());
        assert!(!g.is_random(1, 0));
    });
}

#[test]
fn a_random_creature_can_go_at_the_end_of_the_round_its_room_was_left() {
    // 2.4: what counts is no player in its room at the round's end, not
    // its room being off the round's list (capture.md 10.14.3).
    both(|p| {
        let mut a = adventure(p);
        let mut bat = a.regions[0].residents[0].clone();
        bat.in_use = true;
        bat.name = "BAT".into();
        bat.speed = 0;
        a.regions[0].random_creatures[2] = bat;
        a.regions[0].rooms[1].random_chance = 100;
        a.regions[0].rooms[1].random_creature = 2;
        let mut first_round = 0;
        for seed in 0..400 {
            let mut g = start_seeded(a.clone(), seed, 0, 8, 3);
            g.apply(Command::Move { dir: Dir::E });
            assert!(g.is_random(1, 0), "seed {seed}: appeared");
            let ev = g.apply(Command::Move { dir: Dir::W });
            if ev.iter().any(|e| {
                matches!(
                    e,
                    Event::CreatureRemoved {
                        reason: RemovalReason::LeftBehind,
                        ..
                    }
                )
            }) {
                first_round += 1;
            }
        }
        // 1 in 16 for the slot, then 1 in 5: about 5 of 400.
        assert!((1..20).contains(&first_round), "{first_round}");
    });
}

#[test]
fn a_full_region_gets_no_random_creatures() {
    both(|p| {
        let mut a = adventure(p);
        for slot in 0..16 {
            let mut s = spec(
                0,
                1 + (slot % 8) as u8,
                1 + (slot / 8) as u8,
                Alignment::Friend,
                false,
            );
            s.speed = 0;
            put(&mut a, slot, s);
        }
        a.regions[0].random_creatures[0].in_use = true;
        a.regions[0].rooms[1].random_chance = 100;
        let mut g = start(a, 0, 8, 3);
        let ev = g.apply(Command::Move { dir: Dir::E });
        assert!(
            !ev.iter().any(|e| matches!(e, Event::RoomEncounter { .. })),
            "{ev:?}"
        );
    });
}

#[test]
fn games_with_creatures_replay_exactly() {
    both(|p| {
        let mut a = adventure(p);
        put(&mut a, 1, spec(0, 6, 4, Alignment::Enemy, true));
        put(&mut a, 2, spec(0, 2, 5, Alignment::Thief, true));
        put(&mut a, 3, spec(1, 6, 4, Alignment::Neutral, true));
        put(&mut a, 4, spec(0, 7, 5, Alignment::Friend, true));
        item(&mut a, 0, 7, 1, GOLD, 9);
        item(&mut a, 0, 5, 1, GEM, 1);
        a.regions[0].random_creatures[0] = a.regions[0].residents[1].clone();
        a.regions[0].rooms[1].random_chance = 50;
        let setup = PlayerSetup {
            speed: 15,
            ..Default::default()
        };
        let (mut g, _) = Game::new(a.clone(), 11, Rules { diagonals: true }, setup.clone());
        let mut kinds = std::collections::BTreeSet::new();
        let script = [
            Dir::N,
            Dir::E,
            Dir::E,
            Dir::E,
            Dir::SE,
            Dir::E,
            Dir::E,
            Dir::E,
            Dir::W,
            Dir::W,
        ];
        for (i, d) in script.into_iter().cycle().take(60).enumerate() {
            let cmd = if i % 3 == 2 {
                Command::Wait
            } else {
                Command::Move { dir: d }
            };
            for e in g.apply(cmd) {
                kinds.insert(format!("{e:?}").split([' ', '{']).next().unwrap().to_string());
            }
        }
        assert!(kinds.contains("CreatureTurn"), "{kinds:?}");
        assert!(kinds.contains("CreatureMoved"), "{kinds:?}");
        let r = Game::replay(a, 11, Rules { diagonals: true }, setup, g.log());
        assert_eq!(r, g);
    });
}

/// 2.6: an aggressive enemy that never moves (Speed 0) hinders a player
/// beside it, not one diagonally next to it or two squares away.
#[test]
fn only_the_four_squares_beside_the_player_hinder() {
    both(|p| {
        for (x, y, hindered) in [(3, 2, true), (2, 3, true), (3, 3, false), (4, 2, false)] {
            let mut a = adventure(p);
            put(
                &mut a,
                1,
                Spec {
                    speed: 0,
                    ..spec(0, x, y, Alignment::Enemy, true)
                },
            );
            let mut g = start(a, 0, 2, 2);
            let ev = wait(&mut g);
            let moves = if hindered { 7 } else { 15 };
            assert!(
                ev.contains(&Event::TurnStarted {
                    player: 0,
                    moves,
                    hindered
                }),
                "enemy at {x},{y}: {ev:?}"
            );
        }
    });
}

/// 2.2: two players in a room give its creatures one phase a round, after
/// the last player's turn, not one after each player's.
#[test]
fn two_players_one_creature_phase_a_round() {
    both(|p| {
        // Speed 0: it takes a turn but never walks out of the room.
        let mut a = adventure(p);
        put(
            &mut a,
            1,
            Spec {
                speed: 0,
                ..spec(0, 7, 5, Alignment::Friend, false)
            },
        );
        let mut g = start(a, 0, 2, 2);
        g.apply(Command::AddPlayer {
            setup: PlayerSetup {
                speed: 15,
                ..Default::default()
            },
        });
        g.place(1, at(0, 2, 6));
        let mut phases = Vec::new();
        for _ in 0..6 {
            let who = g.current();
            let ev = wait(&mut g);
            phases.push((who, ev.iter().any(|e| matches!(e, Event::CreatureTurn { .. }))));
        }
        assert_eq!(
            phases,
            vec![
                (0, false),
                (1, true),
                (0, false),
                (1, true),
                (0, false),
                (1, true)
            ]
        );
    });
}

/// 2.2: players in different regions get the scheduler's visits in turn,
/// each up to 10 rounds, the world map first in every cycle.
#[test]
fn players_apart_take_ten_round_visits() {
    both(|p| {
        let mut a = adventure(p);
        put(&mut a, 1, spec(0, 7, 5, Alignment::Friend, false));
        let mut g = start(a, 0, 2, 2);
        g.apply(Command::AddPlayer {
            setup: PlayerSetup {
                speed: 15,
                ..Default::default()
            },
        });
        // Player 0 in the room (region 1), player 1 on the map: the room's
        // visit goes on from here.
        g.place(0, at(0, 2, 2));
        let mut who = Vec::new();
        for _ in 0..31 {
            g.apply(Command::Wait);
            who.push(g.current());
        }
        // The room's visit: 9 more rounds of player 0. Then the rest of the
        // cycle (the map: 10 rounds of player 1) and a new cycle, map first
        // (10 more), then the room again.
        let mut want = vec![0; 9];
        want.extend([1; 20]);
        want.extend([0; 2]);
        assert_eq!(who, want);
    });
}
