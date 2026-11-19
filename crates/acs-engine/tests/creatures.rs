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
            gold: None,
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

/// Resident `slot` owns the sword (it readies it at its turn's start).
fn armed(a: &mut Adventure, slot: usize) {
    a.regions[0].residents[slot].possessions = vec![Possession {
        thing: SWORD,
        count: 1,
    }];
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
fn arrivals_land_on_a_door_destination_someone_holds() {
    // Creatures 5.4, movement 6.3 (capture.md 10.10.5): the player and a
    // creature following both land on the destination, sharing it.
    both(|p| {
        let mut a = adventure(p);
        let mut s = spec(1, 1, 3, Alignment::Friend, false);
        s.speed = 0;
        put(&mut a, 2, s);
        put(&mut a, 1, spec(0, 6, 3, Alignment::Enemy, true));
        let mut g = start(a, 0, 8, 2);
        let ev = g.apply(Command::Move { dir: Dir::SE });
        assert_eq!(g.current_player().at, at(1, 1, 3), "{ev:?}");
        let took = ev.iter().find_map(|e| match e {
            Event::CreatureTookDoor { slot: 1, to, .. } => Some(*to),
            _ => None,
        });
        assert_eq!(took, Some(at(1, 1, 3)));
        assert_eq!(xy(&g, 2), (1, 1, 3));
    });
}

#[test]
fn an_unarmed_fighter_keeps_stepping_round_its_foe() {
    // Combat 2.3 (capture.md 10.14.6): owning no melee weapon, it never
    // attacks.
    both(|p| {
        let mut a = adventure(p);
        put(&mut a, 1, spec(0, 7, 5, Alignment::Enemy, true));
        let mut g = start(a, 0, 2, 2);
        for _ in 0..3 {
            let ev = wait(&mut g);
            assert!(!ev.iter().any(|e| matches!(e, Event::Attacked { .. })));
        }
    });
}

#[test]
fn fighter_enemy_walks_up_to_the_player_and_squares_up() {
    both(|p| {
        let mut a = adventure(p);
        put(&mut a, 1, spec(0, 7, 5, Alignment::Enemy, true));
        armed(&mut a, 1);
        let mut g = start(a, 0, 2, 2);
        let ev = wait(&mut g);
        assert!(ev.contains(&Event::CreatureTurn {
            region: 1,
            slot: 1,
            moves: 15
        }));
        // 6.5: steps only to the 4 squares beside; combat 2.1: it attacks
        // by stepping into the player's square, readying the sword it owns
        // on contact (2.6), and its turn ends (2.5).
        let (_, x, y) = xy(&g, 1);
        let beside = (x as i16 - 2).abs() + (y as i16 - 2).abs() == 1;
        assert!(beside, "ended at {x},{y}");
        assert!(ev.contains(&Event::CreatureReadied {
            region: 1,
            slot: 1,
            thing: SWORD
        }));
        assert!(
            ev.iter().any(|e| matches!(
                e,
                Event::Attacked {
                    by: Who::Creature { region: 1, slot: 1 },
                    target: Who::Player { player: 0 },
                    weapon: SWORD,
                    ..
                }
            )),
            "{ev:?}"
        );
        // 2.6: beside an aggressive enemy, the bar is halved, rounding up
        // (15 to 8).
        assert!(ev.contains(&Event::TurnStarted {
            player: 0,
            moves: 8,
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
fn friends_do_not_hinder_and_do_not_attack_players() {
    both(|p| {
        let mut a = adventure(p);
        put(&mut a, 1, spec(0, 3, 3, Alignment::Friend, true));
        let mut g = start(a, 0, 2, 2);
        let ev = wait(&mut g);
        assert!(!ev.iter().any(|e| matches!(e, Event::Attacked { .. })));
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
        armed(&mut a, 1);
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
        // Next round it acts in EAST and attacks.
        let ev = wait(&mut g);
        assert!(ev.iter().any(|e| matches!(
            e,
            Event::Attacked {
                by: Who::Creature { slot: 1, .. },
                ..
            }
        )));
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
        // 6.2-6.4: a slinker friend on (8,4) scores the player 26 + 10
        // steps = 36 and the east door (9,3) 32 + 2 = 34: it heads for the
        // door and takes it. (Started beside the door, it would rarely take
        // it at once: 6.5's exit cost of 4 on the turn's first step choice;
        // here the door is beside it from its second, cost 1.)
        let mut a = adventure(p);
        put(&mut a, 1, spec(0, 8, 4, Alignment::Friend, false));
        let mut g = start(a, 0, 1, 1);
        let mut ev = Vec::new();
        for _ in 0..4 {
            ev.extend(wait(&mut g));
        }
        assert!(
            ev.iter()
                .any(|e| matches!(e, Event::CreatureTookDoor { slot: 1, .. })),
            "{ev:?}"
        );
    });
}

/// 6.5: an exit costs 4 on the turn's first step choice, goal or not; 1
/// for a slinker on every later one. A Speed 0 slinker neutral beside the
/// east door (9,3), its goal, takes it only on the rare rolls (2 in 81:
/// the door's 0, south's and north's 2, west's at least 1), standing
/// otherwise (2.7.3: the ordinary step is cancelled). Started at (8,4) with
/// Speed 15, the door comes beside it at its second step choice, cost 1,
/// and it nearly always goes through in its first turn.
#[test]
fn an_exit_costs_4_on_the_turns_first_step_choice() {
    let went = |speed: u8, x: u8, y: u8, seeds: u64| {
        let mut n = 0;
        for seed in 0..seeds {
            let mut a = adventure(Platform::C64);
            put(
                &mut a,
                1,
                Spec {
                    speed,
                    ..spec(0, x, y, Alignment::Neutral, false)
                },
            );
            let mut g = start_seeded(a, seed, 0, 1, 1);
            let ev = wait(&mut g);
            let took = ev
                .iter()
                .any(|e| matches!(e, Event::CreatureTookDoor { slot: 1, .. }));
            if speed == 0 && !took {
                assert!(
                    !ev.iter()
                        .any(|e| matches!(e, Event::CreatureMoved { slot: 1, .. })),
                    "seed {seed}: a Speed 0 creature walked: {ev:?}"
                );
            }
            n += took as u32;
        }
        n
    };
    // 256 x 2 / 81 is about 6.
    let still = went(0, 8, 3, 256);
    assert!(
        (1..=16).contains(&still),
        "Speed 0 beside the door: {still} of 256"
    );
    let walking = went(15, 8, 4, 64);
    assert!(walking >= 48, "Speed 15 from (8,4): {walking} of 64");
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
        // 7.5: where it stood at each pick-up, and whether its turn went on.
        let mut takes = Vec::new();
        let mut here = at(0, 2, 3);
        for _ in 0..3 {
            let ev = wait(&mut g);
            for (i, e) in ev.iter().enumerate() {
                match e {
                    Event::CreatureMoved { slot: 1, to, .. } => {
                        stood.push(*to);
                        here = *to;
                    }
                    Event::CreaturePickedUp { slot: 1, .. } | Event::CreatureGoldPickedUp { slot: 1, .. } => {
                        let moved_after = ev[i..]
                            .iter()
                            .take_while(|e| {
                                !matches!(e, Event::TurnStarted { .. } | Event::CreatureTurn { .. })
                            })
                            .any(|e| matches!(e, Event::CreatureMoved { slot: 1, .. }));
                        takes.push((here, moved_after));
                    }
                    _ => {}
                }
            }
        }
        let c = g.creature(1, 1).cloned();
        let gem = c.as_ref().map(|c| c.possessions.iter().any(|p| p.thing == GEM));
        assert_eq!(gem, Some(true), "{c:?}");
        assert_eq!(c.map(|c| c.wealth), Some(5));
        assert!(!stood.contains(&at(0, 4, 3)), "stepped on the wand");
        // 7.5: it takes from a square beside, and the take ends its turn.
        assert_eq!(takes.len(), 2, "{takes:?}");
        for (from, moved_after) in &takes {
            let Location::Room { x, y, .. } = *from else {
                panic!("{from:?}")
            };
            let beside = |(tx, ty): (u8, u8)| x.abs_diff(tx) + y.abs_diff(ty) == 1;
            assert!(beside((6, 3)) || beside((6, 5)), "{takes:?}");
            assert!(!moved_after, "{takes:?}");
        }
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
fn overburdened_creatures_drop_all_their_gold_then_a_treasure_one_a_turn() {
    // 8.1 (capture.md 10.14.5): all the gold first, then the
    // lowest-numbered treasure; here the slowdown leaves 0 moves, so a
    // turn with a drop takes no steps.
    both(|p| {
        let mut a = adventure(p);
        let mut s = spec(0, 5, 3, Alignment::Enemy, false);
        s.speed = 2;
        put(&mut a, 1, s);
        // Capacity (3 - 0) x 100 = 300; three rocks (600) and 800 gold.
        let c = &mut a.regions[0].residents[1];
        c.strength = 3;
        c.possessions = vec![Possession {
            thing: ROCK,
            count: 3,
        }];
        c.wealth = 800;
        let mut g = start(a, 0, 1, 1);
        let moved = |ev: &[Event]| {
            ev.iter()
                .any(|e| matches!(e, Event::CreatureMoved { slot: 1, .. }))
        };
        let ev = wait(&mut g);
        assert!(ev.iter().any(|e| matches!(
            e,
            Event::CreatureDropped {
                slot: 1,
                thing: GOLD,
                amount: 800,
                ..
            }
        )));
        // 8.1.1: its own square or one beside it.
        let gold = dropped_at(&ev, 1, GOLD).expect("drops its gold");
        assert!(steps(gold, (5, 3)) <= 1);
        assert!(!moved(&ev));
        assert_eq!(g.creature(1, 1).unwrap().wealth, 0);
        assert_eq!(top(&g, 0, gold.0, gold.1), Some(GOLD));
        let ev = wait(&mut g);
        let rock = dropped_at(&ev, 1, ROCK).expect("then a rock");
        assert!(steps(rock, (5, 3)) <= 1);
        assert!(!moved(&ev));
        assert_eq!(top(&g, 0, rock.0, rock.1), Some(ROCK));
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

/// Where `slot` dropped `thing` in `ev`, if it did.
fn dropped_at(ev: &[Event], slot: usize, thing: u16) -> Option<(u8, u8)> {
    ev.iter().find_map(|e| match e {
        Event::CreatureDropped {
            slot: s,
            thing: t,
            at: Some(Location::Room { x, y, .. }),
            ..
        } if *s == slot && *t == thing => Some((*x, *y)),
        _ => None,
    })
}

#[test]
fn a_friend_next_to_the_player_drops_a_magic_item_around_itself() {
    // 7.3: the gift's square is 8.1.1's, around the giver: its own square
    // or one beside it, the player's included.
    both(|p| {
        let mut on_player = false;
        for seed in 0..40 {
            let mut a = adventure(p);
            put(&mut a, 1, spec(0, 3, 3, Alignment::Friend, false));
            a.regions[0].residents[1].possessions = vec![Possession {
                thing: WAND,
                count: 1,
            }];
            let mut g = start_seeded(a, seed, 0, 3, 2);
            let ev = wait(&mut g);
            let at = dropped_at(&ev, 1, WAND).expect("drops the wand");
            assert!(steps(at, (3, 3)) <= 1, "seed {seed}: {at:?}");
            assert_eq!(top(&g, 0, at.0, at.1), Some(WAND));
            on_player |= at == (3, 2);
        }
        assert!(on_player);
    });
}

#[test]
fn a_friend_with_no_magic_item_gives_a_treasure_and_keeps_its_gold() {
    // 7.3 (capture.md 10.14.5): a treasure when it has no magic item.
    both(|p| {
        let mut a = adventure(p);
        put(&mut a, 1, spec(0, 3, 3, Alignment::Friend, false));
        let c = &mut a.regions[0].residents[1];
        c.possessions = vec![Possession { thing: GEM, count: 1 }];
        c.wealth = 40;
        let mut g = start(a, 0, 3, 2);
        let ev = wait(&mut g);
        let at = dropped_at(&ev, 1, GEM).expect("gives the gem");
        assert!(steps(at, (3, 3)) <= 1);
        assert_eq!(g.creature(1, 1).unwrap().wealth, 40);
    });
}

fn steps(a: (u8, u8), b: (u8, u8)) -> u8 {
    a.0.abs_diff(b.0) + a.1.abs_diff(b.1)
}

/// An overburdened Speed 2 peaceful enemy on WEST (x, y) carrying one
/// rock over a capacity of 100 (Strength 1): it drops the rock.
fn dropper(p: Platform, x: u8, y: u8) -> Adventure {
    let mut a = adventure(p);
    let mut s = spec(0, x, y, Alignment::Enemy, false);
    s.speed = 2;
    put(&mut a, 1, s);
    let c = &mut a.regions[0].residents[1];
    c.strength = 1;
    c.possessions = vec![Possession {
        thing: ROCK,
        count: 1,
    }];
    a
}

#[test]
fn a_dropped_thing_lands_on_its_own_square_or_one_beside_never_a_wall_or_obstacle() {
    // 8.1.1: each square beside 1 in 6 a try, its own 1 in 3; the ring and
    // an obstacle fail the try; a square with a thing or an individual
    // takes it.
    both(|p| {
        // In the corner (1,1): west and north are the walls; a pillar
        // south on (1,2); a gem east on (2,1); the player far off.
        let mut seen = std::collections::BTreeMap::new();
        for seed in 0..120 {
            let mut a = dropper(p, 1, 1);
            item(&mut a, 0, 1, 2, PILLAR, 0);
            item(&mut a, 0, 2, 1, GEM, 0);
            let mut g = start_seeded(a, seed, 0, 7, 5);
            let ev = wait(&mut g);
            let at = dropped_at(&ev, 1, ROCK).expect("drops the rock");
            *seen.entry(at).or_insert(0) += 1;
            assert_eq!(top(&g, 0, at.0, at.1), Some(ROCK));
        }
        let keys: Vec<_> = seen.keys().copied().collect();
        assert_eq!(keys, [(1, 1), (2, 1)], "{seen:?}");
        // Own square 1/3 against east 1/6 a try: about two to one.
        assert!(seen[&(1, 1)] > seen[&(2, 1)], "{seen:?}");
        // Beside a fellow enemy (not a foe, so the drop goes ahead, 8.1.2):
        // its square takes the thing too.
        let mut on_it = false;
        for seed in 0..60 {
            let mut a = dropper(p, 5, 3);
            let mut s = spec(0, 6, 3, Alignment::Enemy, false);
            s.speed = 0;
            put(&mut a, 2, s);
            let mut g = start_seeded(a, seed, 0, 1, 1);
            let ev = wait(&mut g);
            let at = dropped_at(&ev, 1, ROCK).expect("drops the rock");
            assert!(steps(at, (5, 3)) <= 1);
            on_it |= at == (6, 3);
        }
        assert!(on_it);
    });
}

#[test]
fn a_speed_0_creature_never_walks_and_heals_one_a_turn_with_no_roll() {
    // 2.7, magic 5.3.1 (still-hurt, still-enemy, still-whole): one action
    // a turn, the step cancelled; the idle move heals 1 (2 with the die's
    // 2) while Life Force is below Constitution, never past it.
    both(|p| {
        for (alignment, aggressive) in [(Alignment::Friend, false), (Alignment::Enemy, true)] {
            for seed in 0..8 {
                let mut a = adventure(p);
                let mut s = spec(0, 7, 3, alignment, aggressive);
                s.speed = 0;
                put(&mut a, 1, s);
                let c = &mut a.regions[0].residents[1];
                c.constitution = 40;
                c.life_force = 30;
                let mut g = start_seeded(a, seed, 0, 2, 3);
                let mut life = 30;
                for _ in 0..14 {
                    let ev = wait(&mut g);
                    assert!(!ev
                        .iter()
                        .any(|e| matches!(e, Event::CreatureMoved { slot: 1, .. })));
                    let now = g.creature(1, 1).unwrap().life_force;
                    if life < 40 {
                        assert!(now == life + 1 || now == (life + 2).min(40), "{life} to {now}");
                    } else {
                        assert_eq!(now, 40);
                    }
                    life = now;
                }
                assert_eq!(life, 40);
            }
        }
    });
}

#[test]
fn a_speed_0_thief_takes_gold_beside_it() {
    // 2.7: the step is not cancelled onto a thing it takes.
    both(|p| {
        let mut a = adventure(p);
        let mut s = spec(0, 5, 3, Alignment::Thief, false);
        s.speed = 0;
        put(&mut a, 1, s);
        item(&mut a, 0, 6, 3, GOLD, 30);
        let mut g = start(a, 0, 1, 1);
        let ev = wait(&mut g);
        assert!(ev
            .iter()
            .any(|e| matches!(e, Event::CreatureGoldPickedUp { slot: 1, .. })));
        assert_eq!(xy(&g, 1), (0, 5, 3));
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
        // 2.4: the removal keeps the slot's random mark.
        assert!(g.is_random(1, 0));
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
            let moves = if hindered { 8 } else { 15 };
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
        // Characters 6.2: the newcomer joins at the round's end.
        g.apply(Command::Wait);
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
        // Characters 6.2: the newcomer joins at the round's end.
        g.apply(Command::Wait);
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

fn on_map(a: Adventure, speed: u8) -> Game {
    Game::new(
        a,
        3,
        Rules { diagonals: true },
        PlayerSetup {
            speed,
            ..Default::default()
        },
    )
    .0
}

fn moves_at_start(ev: &[Event], player: usize) -> Option<u8> {
    ev.iter().find_map(|e| match *e {
        Event::TurnStarted { player: q, moves, .. } if q == player => Some(moves),
        _ => None,
    })
}

/// 2.2.4, movement 2.3: a map door into a room ends the turn; a lone
/// player's first room turn follows at once with the moves it had left.
#[test]
fn a_map_door_ends_the_turn_and_keeps_the_moves_left() {
    both(|p| {
        let mut g = on_map(adventure(p), 15);
        let ev = g.apply(Command::Move { dir: Dir::N });
        let ended = ev.iter().position(|e| *e == Event::TurnEnded { player: 0 });
        let entered = ev.iter().position(|e| matches!(e, Event::EnteredRoom { .. }));
        assert!(entered.is_some() && entered < ended, "{ev:?}");
        assert_eq!(moves_at_start(&ev, 0), Some(14));
        assert_eq!(g.current_player().moves_left, 14);
        assert_eq!(g.current_player().at, at(0, 2, 2));
        // The turn after is a full one.
        let ev = wait(&mut g);
        assert_eq!(moves_at_start(&ev, 0), Some(15));
    });
}

/// 2.2.4: with no moves left at the door the first room turn has full Speed.
#[test]
fn a_map_door_on_the_last_move_gives_a_full_turn() {
    both(|p| {
        let mut g = on_map(adventure(p), 2);
        g.apply(Command::Move { dir: Dir::E });
        let ev = g.apply(Command::Move { dir: Dir::NW });
        assert!(
            ev.iter().any(|e| matches!(e, Event::EnteredRoom { .. })),
            "{ev:?}"
        );
        assert_eq!(moves_at_start(&ev, 0), Some(2));
    });
}

/// 2.2.3, 2.2.4: players come into the room region by player number
/// whichever came in first, the later one with its moves left; then the
/// room's creatures.
#[test]
fn the_first_room_round_lists_players_by_number() {
    both(|p| {
        let mut a = adventure(p);
        put(
            &mut a,
            1,
            Spec {
                speed: 0,
                ..spec(0, 7, 5, Alignment::Friend, false)
            },
        );
        let mut g = on_map(a, 15);
        g.apply(Command::AddPlayer {
            setup: PlayerSetup {
                speed: 15,
                ..Default::default()
            },
        });
        wait(&mut g);
        // Next map round: player 0 waits, player 1 takes the door.
        wait(&mut g);
        assert_eq!(g.current(), 1);
        let ev = g.apply(Command::Move { dir: Dir::N });
        assert!(ev
            .iter()
            .any(|e| matches!(e, Event::EnteredRoom { player: 1, .. })));
        // The map goes on with player 0 alone; player 1 waits for its region
        // (and keeps its 14 moves).
        assert_eq!(g.current(), 0);
        g.apply(Command::Move { dir: Dir::E });
        let ev = g.apply(Command::Move { dir: Dir::NW });
        assert!(ev
            .iter()
            .any(|e| matches!(e, Event::EnteredRoom { player: 0, .. })));
        assert_eq!(moves_at_start(&ev, 0), Some(13));
        assert!(!ev
            .iter()
            .any(|e| matches!(e, Event::TurnStarted { player: 1, .. })));
        let ev = wait(&mut g);
        assert_eq!(moves_at_start(&ev, 1), Some(14));
        assert!(!ev.iter().any(|e| matches!(e, Event::CreatureTurn { .. })));
        let ev = wait(&mut g);
        let creature = ev
            .iter()
            .position(|e| matches!(e, Event::CreatureTurn { slot: 1, .. }));
        let next = ev
            .iter()
            .position(|e| matches!(e, Event::TurnStarted { player: 0, .. }));
        assert!(creature.is_some() && creature < next, "{ev:?}");
    });
}

// ---- the restore point (2.2.6, 2.2.7) --------------------------------------

const PIT: u16 = 90; // space, Life Force -10 (kills a player of 9)

/// The fixture with a deadly space at WEST (3,4).
fn with_pit(p: Platform) -> Adventure {
    use acs_core::model::{Access, SpellAction, SpellActionKind, ThingKind};
    let mut a = adventure(p);
    let t = &mut a.things[PIT as usize - 1];
    t.name = "PIT".into();
    t.raw = [0; 4];
    t.kind = ThingKind::Space {
        picture: 4,
        access: Access::InvokeSpell {
            action: SpellAction {
                kind: SpellActionKind::ChangeLifeForce,
                parameter: 0x80 | 10,
            },
        },
    };
    item(&mut a, 0, 3, 4, PIT, 0);
    a
}

fn frail() -> PlayerSetup {
    PlayerSetup {
        name: "FRAIL".into(),
        speed: 15,
        life_force: 9,
        ..Default::default()
    }
}

fn saved(ev: &[Event]) -> Vec<(u32, Option<u8>)> {
    ev.iter()
        .filter_map(|e| match *e {
            Event::Saved { round, visit } => Some((round, visit)),
            _ => None,
        })
        .collect()
}

/// 2.2.7.1: the save is made when the room region's visit begins, its
/// first round on the counter, not at the map door; visits of the same
/// region one after another save nothing; going back to the map saves.
#[test]
fn a_visit_to_another_region_saves_at_its_first_round() {
    both(|p| {
        let mut g = on_map(adventure(p), 15);
        assert_eq!(g.save_point(), 0);
        let door_round = g.round();
        let ev = g.apply(Command::Move { dir: Dir::N });
        let entered = ev.iter().position(|e| matches!(e, Event::EnteredRegion { .. }));
        let save = ev.iter().position(|e| matches!(e, Event::Saved { .. }));
        assert!(entered.is_some() && entered < save, "{ev:?}");
        assert_eq!(saved(&ev), [(door_round + 1, Some(1))]);
        assert_eq!((g.round(), g.visiting()), (door_round + 1, 1));
        assert_eq!(g.save_point(), 1);
        // Visit after visit of region 1 alone: nothing more is saved.
        for _ in 0..25 {
            assert!(saved(&wait(&mut g)).is_empty());
        }
        assert_eq!(g.save_point(), 1);
        assert_eq!(g.restore_point(), 1);
        // Out through WEST's map door (4,0): the map's visit saves.
        g.apply(Command::Move { dir: Dir::E });
        g.apply(Command::Move { dir: Dir::E });
        g.apply(Command::Move { dir: Dir::N });
        let ev = g.apply(Command::Move { dir: Dir::N });
        assert!(
            ev.iter().any(|e| matches!(e, Event::EnteredWorldMap { .. })),
            "{ev:?}"
        );
        // A room door to the map does not end the turn: the save comes with
        // the map visit, after the round.
        assert!(saved(&ev).is_empty());
        let r = g.round();
        let ev = wait(&mut g);
        assert_eq!(saved(&ev), [(r + 1, Some(0))]);
        assert_eq!(g.save_point(), g.log().len());
    });
}

/// 2.2.7.2: SAVE GAME uses no move and ends no turn, and saves at the
/// round's end, after the adds, with the counter gone up; asked twice in
/// one round it saves once. The command replays.
#[test]
fn save_game_saves_at_the_end_of_its_round() {
    both(|p| {
        let mut g = start(adventure(p), 0, 2, 2);
        let round = g.round();
        g.apply(Command::AddPlayer {
            setup: PlayerSetup {
                speed: 15,
                ..Default::default()
            },
        });
        let ev = g.apply(Command::SaveGame);
        assert_eq!(ev, [Event::SaveRequested { player: 0 }]);
        assert_eq!(g.current(), 0);
        let ev = wait(&mut g);
        let added = ev.iter().position(|e| matches!(e, Event::PlayerAdded { .. }));
        let save = ev.iter().position(|e| matches!(e, Event::Saved { .. }));
        assert!(added.is_some() && added < save, "{ev:?}");
        assert_eq!(saved(&ev), [(round + 1, None)]);
        assert_eq!(g.save_point(), 3);
        // Both players in WEST: each asks in the same round, one save.
        g.place(1, at(0, 3, 2));
        let base = g.save_point();
        assert_eq!(base, g.log().len());
        assert_eq!(g.current(), 0);
        g.apply(Command::SaveGame);
        let ev = wait(&mut g);
        assert!(saved(&ev).is_empty());
        assert_eq!(g.current(), 1);
        g.apply(Command::SaveGame);
        let ev = wait(&mut g);
        assert_eq!(saved(&ev).len(), 1, "{ev:?}");
        assert_eq!(g.save_point(), g.log().len());
        let json = serde_json::to_string(&Command::SaveGame).unwrap();
        assert_eq!(json, r#"{"command":"save_game"}"#);
    });
}

/// 2.2.7.2, magic 5.3: SAVE GAME ends no turn. Its moves are walked on,
/// or a Wait runs them out as idle moves with one healing roll each, the
/// same draws as without the save, at full Life Force too.
#[test]
fn save_game_leaves_the_turn_and_its_idle_rolls() {
    both(|p| {
        for low in [None, Some(3)] {
            let mut plain = start(adventure(p), 0, 2, 2);
            if let Some(lf) = low {
                plain.set_life_force(0, lf);
            }
            let mut asked = plain.clone();
            let left = asked.current_player().moves_left;
            assert!(left > 1);
            assert_eq!(
                asked.apply(Command::SaveGame),
                [Event::SaveRequested { player: 0 }]
            );
            assert_eq!(asked.current_player().moves_left, left);
            // Walked on: the turn has its moves.
            let mut walked = asked.clone();
            walked.apply(Command::Move { dir: Dir::E });
            assert_eq!(walked.current(), 0);
            assert_eq!(walked.current_player().moves_left, left - 1);
            // Run out: the same draws as a plain Wait.
            let a = asked.apply(Command::Wait);
            let b = plain.apply(Command::Wait);
            assert!(a.contains(&Event::TurnEnded { player: 0 }), "{a:?}");
            assert_eq!(asked.players(), plain.players());
            assert_eq!(asked.round(), plain.round());
            assert_eq!(saved(&a).len(), 1, "{a:?}");
            assert!(saved(&b).is_empty());
            // And the rolls after it stay in step.
            for _ in 0..5 {
                asked.apply(Command::Wait);
                plain.apply(Command::Wait);
            }
            assert_eq!(asked.players(), plain.players());
        }
    });
}

/// 2.2.7.3, characters 7.3: a Retire with Save saves at the round's end, a
/// `Saved` after `Retired`; with a SAVE GAME in the same round that one
/// saves again after it (2.2.7.4's order: the retire's save, the adds, then
/// the round's other save). A Destroy saves nothing.
#[test]
fn a_retire_with_save_is_a_save_point() {
    both(|p| {
        for (save, also_save_game) in [(true, false), (true, true), (false, false)] {
            let mut g = start(adventure(p), 0, 2, 2);
            g.apply(Command::AddPlayer {
                setup: PlayerSetup {
                    speed: 15,
                    ..Default::default()
                },
            });
            wait(&mut g);
            g.place(1, at(0, 3, 2));
            let before = g.save_point();
            let round = g.round();
            assert_eq!(g.current(), 0);
            g.apply(Command::Retire { save });
            if also_save_game {
                g.apply(Command::SaveGame);
            }
            let ev = wait(&mut g);
            assert!(saved(&ev).is_empty());
            let ev = wait(&mut g);
            let retired = ev
                .iter()
                .position(|e| matches!(e, Event::Retired { player: 0, .. }));
            assert!(retired.is_some(), "{ev:?}");
            if save {
                let s = ev
                    .iter()
                    .position(|e| matches!(e, Event::Saved { visit: None, .. }));
                assert!(retired < s, "{ev:?}");
                let n = if also_save_game { 2 } else { 1 };
                assert_eq!(saved(&ev), vec![(round + 1, None); n]);
                assert_eq!(g.save_point(), g.log().len());
            } else {
                assert!(saved(&ev).is_empty(), "{ev:?}");
                assert_eq!(g.save_point(), before);
            }
            assert_eq!(g.restore_point(), g.save_point());
        }
    });
}

/// Characters 3.1: a character retired with Save rejoins as saved, its
/// weapon readied, on the square it retired from, and its joining saves
/// (2.2.7.4). With that square taken since, it joins at the start.
#[test]
fn a_saved_character_rejoins_where_it_retired_with_its_gear() {
    both(|p| {
        for taken in [false, true] {
            let (mut g, _) = Game::new(
                adventure(p),
                3,
                Rules { diagonals: true },
                PlayerSetup {
                    speed: 15,
                    possessions: vec![SWORD, GEM],
                    ..Default::default()
                },
            );
            g.place(0, at(0, 2, 2));
            g.apply(Command::AddPlayer {
                setup: PlayerSetup {
                    name: "TWO".into(),
                    speed: 15,
                    ..Default::default()
                },
            });
            wait(&mut g);
            g.place(1, at(0, 5, 4));
            assert_eq!(g.current(), 0);
            g.apply(Command::Ready { thing: SWORD });
            g.apply(Command::Retire { save: true });
            let mut record = None;
            for _ in 0..4 {
                for e in wait(&mut g) {
                    if let Event::Retired { saved: Some(r), .. } = e {
                        record = Some(r);
                    }
                }
                if record.is_some() {
                    break;
                }
            }
            let record = record.expect("retired with Save");
            assert_eq!(record.weapon, Some(SWORD));
            assert_eq!(record.at, at(0, 2, 2));
            if taken {
                g.place(1, at(0, 2, 2));
            }
            assert_eq!(g.current(), 1);
            g.apply(Command::Rejoin {
                player: record.clone(),
            });
            let mut joined = None;
            let mut saves = 0;
            for _ in 0..4 {
                for e in wait(&mut g) {
                    match e {
                        Event::PlayerAdded { player, at } => joined = Some((player, at)),
                        Event::Saved { .. } => saves += 1,
                        _ => {}
                    }
                }
                if joined.is_some() {
                    break;
                }
            }
            let (slot, place) = joined.expect("joined");
            let start = g.adventure().world.start;
            let expect = if taken {
                Location::World {
                    x: start.x,
                    y: start.y,
                }
            } else {
                at(0, 2, 2)
            };
            assert_eq!(place, expect);
            let me = &g.players()[slot];
            assert_eq!(me.at, expect);
            assert_eq!(me.weapon, Some(SWORD));
            assert_eq!(me.possessions, record.possessions);
            assert_eq!((me.life_force, me.power), (record.life_force, record.power));
            assert!(saves >= 1, "a roster add saves");
        }
    });
}

/// 2.2.7, 2.2.6: a death in the round of a Retire with Save offers the
/// save from before that round; the retire's save is made after it.
#[test]
fn a_death_in_a_retire_round_offers_the_earlier_save() {
    both(|p| {
        let mut g = start(with_pit(p), 0, 2, 2);
        g.apply(Command::AddPlayer { setup: frail() });
        wait(&mut g);
        g.place(1, at(0, 2, 4));
        let before = g.save_point();
        assert_eq!(g.current(), 0);
        g.apply(Command::Retire { save: true });
        wait(&mut g);
        assert_eq!(g.current(), 1);
        let ev = g.apply(Command::Move { dir: Dir::E });
        let msg = ev.iter().position(|e| matches!(e, Event::LivingChanged { .. }));
        let retired = ev
            .iter()
            .position(|e| matches!(e, Event::Retired { player: 0, .. }));
        let save = ev
            .iter()
            .position(|e| matches!(e, Event::Saved { visit: None, .. }));
        assert!(msg.is_some() && msg < retired && retired < save, "{ev:?}");
        assert_eq!(g.save_point(), g.log().len());
        assert_eq!(g.restore_point(), before);
    });
}

/// 2.2.7, the engine's choice: a SAVE GAME in the round whose end brings
/// a visit to another region saves at the round's end, then the visit's
/// start saves again, and that is the later one (the same command).
#[test]
fn save_game_and_a_region_change_in_one_round() {
    both(|p| {
        let mut g = on_map(adventure(p), 15);
        g.apply(Command::AddPlayer {
            setup: PlayerSetup {
                speed: 15,
                ..Default::default()
            },
        });
        wait(&mut g);
        // Player 0 in the room region, player 1 on the map: the map's
        // visits alternate with the room's.
        g.place(0, at(0, 2, 2));
        let mut both_in_one = None;
        for _ in 0..40 {
            if g.current() == 1 {
                let round = g.round();
                g.apply(Command::SaveGame);
                let ev = wait(&mut g);
                let s = saved(&ev);
                assert_eq!(s[0], (round + 1, None), "{ev:?}");
                if s.len() > 1 {
                    both_in_one = Some((round, s));
                    break;
                }
            } else {
                wait(&mut g);
            }
        }
        let (round, s) = both_in_one.expect("a map visit ended on a SAVE GAME");
        assert_eq!(s, [(round + 1, None), (round + 1, Some(1))]);
        assert_eq!(g.save_point(), g.log().len());
        assert_eq!(g.visiting(), 1);
    });
}

/// 2.2.6: the death message's button restores the save as it stood at the
/// message; a SAVE GAME of the same round is made after it (2.2.5), so it
/// counts only once play goes on (F1).
#[test]
fn a_death_offers_the_save_made_before_its_round_ends() {
    both(|p| {
        let mut g = start(with_pit(p), 0, 2, 2);
        g.apply(Command::AddPlayer { setup: frail() });
        wait(&mut g);
        g.place(1, at(0, 2, 4));
        let before = g.save_point();
        assert_eq!(g.current(), 0);
        g.apply(Command::SaveGame);
        wait(&mut g);
        assert_eq!(g.current(), 1);
        let ev = g.apply(Command::Move { dir: Dir::E });
        let msg = ev
            .iter()
            .position(|e| *e == Event::LivingChanged { before: 2, after: 1 });
        let save = ev
            .iter()
            .position(|e| matches!(e, Event::Saved { visit: None, .. }));
        assert!(msg.is_some() && msg < save, "{ev:?}");
        assert_eq!(g.save_point(), g.log().len());
        assert_eq!(g.restore_point(), before);
        // F1: play goes on, and the round's save is the one to go back to.
        wait(&mut g);
        assert_eq!(g.restore_point(), g.save_point());
        assert_eq!(g.save_point(), g.log().len() - 1);
    });
}

/// 2.2.6: the button brings back the moment of the save, not the turn the
/// saving command went on to start: played on the game at the save point,
/// the restored game starts that same turn with the same draws. The
/// scheduler holds no region, so the new cycle's first visit saves; the
/// button is logged.
#[test]
fn the_button_goes_back_to_the_moment_of_the_save() {
    both(|p| {
        let mut g = start(adventure(p), 0, 2, 2);
        g.set_life_force(0, 3);
        g.apply(Command::SaveGame);
        let ev = wait(&mut g);
        assert_eq!(saved(&ev).len(), 1, "{ev:?}");
        assert_eq!(g.save_point(), g.log().len());
        let mut kept = g.clone();
        let mut r = g.clone();
        let ev = r.apply(Command::Restore);
        assert_eq!(ev.first(), Some(&Event::Restored { round: kept.round() }));
        assert_eq!(saved(&ev), [(kept.round(), Some(1))], "{ev:?}");
        assert!(ev.contains(&Event::TurnStarted {
            player: 0,
            moves: 15,
            hindered: false
        }));
        assert_eq!(r.log().last(), Some(&Command::Restore));
        assert_eq!(r.save_point(), r.log().len());
        assert_eq!((r.round(), r.visiting(), r.current()), (kept.round(), 1, 0));
        assert_eq!(r.players(), kept.players());
        for _ in 0..8 {
            wait(&mut r);
            wait(&mut kept);
            assert_eq!(r.players(), kept.players());
            assert_eq!(r.round(), kept.round());
        }
    });
}

/// 2.2.6, 2.2.7.3, 2.2.7.4: a Retire with Save and a roster "Add Someone"
/// in one round save twice: the retire's, then one after the add, the
/// retired slot refilled by the newcomer, with or without a SAVE GAME too.
/// The button clears the requests, so none is made again.
/// With nobody in play the button does nothing.
#[test]
fn the_button_clears_the_requests() {
    both(|p| {
        for also_save_game in [false, true] {
            let mut g = start(adventure(p), 0, 2, 2);
            g.apply(Command::AddPlayer {
                setup: PlayerSetup {
                    speed: 15,
                    ..Default::default()
                },
            });
            wait(&mut g);
            g.place(1, at(0, 3, 2));
            assert_eq!(g.current(), 0);
            g.apply(Command::AddPlayer { setup: frail() });
            g.apply(Command::Retire { save: true });
            if also_save_game {
                g.apply(Command::SaveGame);
            }
            wait(&mut g);
            let ev = wait(&mut g);
            assert!(
                ev.iter()
                    .any(|e| matches!(e, Event::PlayerAdded { player: 0, .. })),
                "{ev:?}"
            );
            assert_eq!(saved(&ev).len(), 2, "{ev:?}");
            assert_eq!(g.save_point(), g.log().len());
            assert_eq!(g.players()[0].name, "FRAIL");
            let mut r = g.clone();
            let ev = r.apply(Command::Restore);
            assert!(
                !ev.iter().any(|e| matches!(
                    e,
                    Event::PlayerAdded { .. } | Event::Retired { .. } | Event::AddRefused
                )),
                "{ev:?}"
            );
            assert_eq!(r.players()[0].name, "FRAIL");
            assert!(r.players()[0].alive);
            assert!(r.players()[1].alive);
            // The newcomer starts on the map, which comes first.
            assert_eq!((r.round(), r.visiting(), r.current()), (g.round(), 0, 0));
        }
        // Nobody in play: ignored, not logged.
        let mut g = start(with_pit(p), 0, 2, 4);
        g.set_life_force(0, 9);
        let ev = g.apply(Command::Move { dir: Dir::E });
        assert_eq!(ev.last(), Some(&Event::AllPlayersDead), "{ev:?}");
        let logged = g.log().len();
        assert!(g.apply(Command::Restore).is_empty());
        assert_eq!(g.log().len(), logged);
        assert!(g.is_over());
    });
}

/// The game the death message's button brings back: the save at
/// [`Game::restore_point`], replayed as a save file is, then `Restore`.
fn button(g: &Game, a: &Adventure) -> Game {
    let mut s = acs_engine::SaveFile::new(g, a, g);
    s.autosave = g.restore_point();
    let mut r = s.restore(a).expect("restores").autosave;
    r.apply(Command::Restore);
    r
}

/// 2.2.7.4, characters 6.2: a roster Add Someone in play (`AddPlayer`) is a
/// save point once the newcomer joins at the round's end: one `Saved` after
/// `PlayerAdded`, holding the counter already gone up, and the button
/// brings back that round's end with the newcomer in. No region change is
/// needed, and the visit goes on.
#[test]
fn a_roster_add_in_play_is_a_save_point() {
    both(|p| {
        let a = adventure(p);
        let mut g = on_map(a.clone(), 15);
        // A few rounds in, nothing saved since the game's start.
        for _ in 0..3 {
            wait(&mut g);
        }
        assert_eq!(g.save_point(), 0);
        let round = g.round();
        let ev = g.apply(Command::AddPlayer { setup: frail() });
        assert!(saved(&ev).is_empty(), "{ev:?}");
        let ev = wait(&mut g);
        let added = ev
            .iter()
            .position(|e| matches!(e, Event::PlayerAdded { player: 1, .. }));
        let save = ev
            .iter()
            .position(|e| matches!(e, Event::Saved { visit: None, .. }));
        assert!(added.is_some() && added < save, "{ev:?}");
        assert_eq!(saved(&ev), [(round + 1, None)], "{ev:?}");
        assert_eq!(g.save_point(), g.log().len());
        assert_eq!(g.visiting(), 0);
        // Play on, then the button: the round after the request, both in.
        for _ in 0..5 {
            wait(&mut g);
        }
        let r = button(&g, &a);
        assert_eq!(r.round(), round + 1);
        assert_eq!(r.players().len(), 2);
        assert!(r.players().iter().all(|q| q.alive));
        assert_eq!(r.players()[1].name, "FRAIL");
    });
}

/// 2.2.7.4 (capture.md 10.38 `i1d-button`, `i1c-button`): a created add
/// (`CreatePlayer`) saves nothing, on the map or in a room region's visit;
/// the visit goes on, and the button goes back past the add, the
/// newcomer's slot empty.
#[test]
fn a_created_add_saves_nothing_and_ends_no_visit() {
    both(|p| {
        // On the map: back to the game's start.
        let a = adventure(p);
        let mut g = on_map(a.clone(), 15);
        for _ in 0..3 {
            wait(&mut g);
        }
        g.apply(Command::CreatePlayer {
            name: "NEW".into(),
            picture: None,
        });
        let ev = wait(&mut g);
        assert!(ev
            .iter()
            .any(|e| matches!(e, Event::PlayerAdded { player: 1, .. })));
        assert!(saved(&ev).is_empty(), "{ev:?}");
        assert_eq!(g.save_point(), 0);
        let r = button(&g, &a);
        assert_eq!(r.round(), 0);
        assert!(r.players().iter().all(|q| q.name != "NEW"), "{:?}", r.players());
        // In a room region's visit, U (from a roster) on the map: the visit
        // goes on to its end, the newcomer on the map waiting for it; the
        // button goes back to U's add, without the newcomer.
        let mut g = start(a.clone(), 0, 2, 2);
        g.apply(Command::AddPlayer {
            setup: PlayerSetup {
                name: "U".into(),
                speed: 15,
                ..Default::default()
            },
        });
        wait(&mut g);
        let (point, round) = (g.save_point(), g.round());
        assert_eq!(point, g.log().len());
        assert_eq!((g.visiting(), g.current()), (1, 0));
        g.apply(Command::CreatePlayer {
            name: "NEW".into(),
            picture: None,
        });
        let ev = wait(&mut g);
        assert!(
            ev.iter().any(|e| matches!(e, Event::PlayerAdded { .. })),
            "{ev:?}"
        );
        assert!(saved(&ev).is_empty(), "{ev:?}");
        assert_eq!((g.visiting(), g.current()), (1, 0), "the visit goes on");
        assert_eq!(g.save_point(), point);
        let r = button(&g, &a);
        assert_eq!(r.round(), round);
        assert!(r.players().iter().any(|q| q.name == "U" && q.alive));
        assert!(r.players().iter().all(|q| q.name != "NEW"), "{:?}", r.players());
    });
}

/// 2.2.7.4's engine choice, characters 6.2, 6.4: a refused add (four
/// living) saves nothing; two requests give one refusal.
#[test]
fn a_refused_add_saves_nothing() {
    both(|p| {
        for twice in [false, true] {
            let mut g = on_map(adventure(p), 15);
            while g.players().len() < 4 {
                g.apply(Command::AddPlayer { setup: frail() });
                for _ in 0..8 {
                    let ev = wait(&mut g);
                    if ev.iter().any(|e| matches!(e, Event::PlayerAdded { .. })) {
                        break;
                    }
                }
            }
            let point = g.save_point();
            g.apply(Command::AddPlayer { setup: frail() });
            if twice {
                g.apply(Command::CreatePlayer {
                    name: "TWO".into(),
                    picture: None,
                });
            }
            let mut ev = Vec::new();
            while !ev.contains(&Event::AddRefused) {
                ev.extend(wait(&mut g));
            }
            let refusals = ev.iter().filter(|e| **e == Event::AddRefused).count();
            assert_eq!(refusals, 1, "{ev:?}");
            assert!(saved(&ev).is_empty(), "{ev:?}");
            assert_eq!(g.save_point(), point);
            for _ in 0..4 {
                assert!(!wait(&mut g).contains(&Event::AddRefused));
            }
        }
    });
}

/// 2.2.7.4's engine choice: with a SAVE GAME in the round, an add, created
/// or from a roster, is followed by one save holding the newcomer. With a
/// Retire with Save, the retire's save comes first, before the add: a
/// roster newcomer saves again after it, a created one does not and is not
/// in the restore.
#[test]
fn an_add_and_another_save_in_one_round() {
    both(|p| {
        for other in [Command::SaveGame, Command::Retire { save: true }] {
            for roster in [true, false] {
                let a = adventure(p);
                let mut g = on_map(a.clone(), 15);
                g.apply(Command::AddPlayer { setup: frail() });
                wait(&mut g);
                assert_eq!(g.players().len(), 2);
                while g.current() != 0 {
                    wait(&mut g);
                }
                let round = g.round();
                let add = if roster {
                    Command::AddPlayer {
                        setup: PlayerSetup {
                            name: "NEW".into(),
                            speed: 15,
                            ..Default::default()
                        },
                    }
                } else {
                    Command::CreatePlayer {
                        name: "NEW".into(),
                        picture: None,
                    }
                };
                let retiring = matches!(other, Command::Retire { .. });
                // SAVE GAME asked after the add here, before it otherwise.
                if retiring {
                    g.apply(add);
                    g.apply(other.clone());
                } else {
                    g.apply(other.clone());
                    g.apply(add);
                }
                let mut ev = wait(&mut g);
                while !ev.iter().any(|e| matches!(e, Event::PlayerAdded { .. })) {
                    ev.extend(wait(&mut g));
                }
                let saves: Vec<usize> = ev
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| matches!(e, Event::Saved { .. }))
                    .map(|(i, _)| i)
                    .collect();
                let added = ev
                    .iter()
                    .position(|e| matches!(e, Event::PlayerAdded { .. }))
                    .unwrap();
                let n = if retiring && roster { 2 } else { 1 };
                assert_eq!(
                    saved(&ev),
                    vec![(round + 1, None); n],
                    "{other:?} {roster}: {ev:?}"
                );
                if retiring {
                    let retired = ev
                        .iter()
                        .position(|e| matches!(e, Event::Retired { .. }))
                        .unwrap();
                    assert!(retired < saves[0] && saves[0] < added, "{ev:?}");
                    if roster {
                        assert!(added < saves[1], "{ev:?}");
                    }
                } else {
                    assert!(added < saves[0], "{ev:?}");
                }
                // The retired slot 0 is freed and refilled by the newcomer.
                let slot = if retiring { 0 } else { 2 };
                assert_eq!(g.players()[slot].name, "NEW");
                for _ in 0..4 {
                    wait(&mut g);
                }
                let r = button(&g, &a);
                assert_eq!(r.round(), round + 1);
                if retiring && !roster {
                    // The retire's save: the slot free, no newcomer.
                    assert!(r.players().iter().all(|q| q.name != "NEW"), "{:?}", r.players());
                    assert!(!r.players()[slot].alive);
                } else {
                    assert_eq!(r.players()[slot].name, "NEW");
                    assert!(r.players()[slot].alive);
                }
            }
        }
    });
}

/// 2.2.6, 2.2.7.4: a death in the round of an add offers the save from
/// before that round (the button clears the add and the newcomer is not
/// there); with F1 the add goes ahead and is the save point.
#[test]
fn a_death_in_an_add_round_offers_the_earlier_save() {
    both(|p| {
        let a = with_pit(p);
        let mut g = start(a.clone(), 0, 2, 2);
        g.apply(Command::AddPlayer { setup: frail() });
        wait(&mut g);
        g.place(1, at(0, 2, 4));
        let before = g.save_point();
        assert_eq!(g.current(), 0);
        g.apply(Command::AddPlayer {
            setup: PlayerSetup {
                name: "NEW".into(),
                speed: 15,
                ..Default::default()
            },
        });
        wait(&mut g);
        assert_eq!(g.current(), 1);
        let ev = g.apply(Command::Move { dir: Dir::E });
        let msg = ev
            .iter()
            .position(|e| *e == Event::LivingChanged { before: 2, after: 1 });
        let added = ev
            .iter()
            .position(|e| matches!(e, Event::PlayerAdded { player: 1, .. }));
        let save = ev
            .iter()
            .position(|e| matches!(e, Event::Saved { visit: None, .. }));
        assert!(msg.is_some() && msg < added && added < save, "{ev:?}");
        assert_eq!(g.save_point(), g.log().len());
        assert_eq!(g.restore_point(), before);
        // The button: the save before the round, without the newcomer.
        let r = button(&g, &a);
        assert!(r.players().iter().all(|q| q.name != "NEW"), "{:?}", r.players());
        // F1: play goes on, and the add's save is the one to go back to.
        wait(&mut g);
        assert_eq!(g.restore_point(), g.save_point());
        let r = button(&g, &a);
        assert_eq!(r.players()[1].name, "NEW");
        assert!(r.players()[1].alive);
    });
}

/// 2.2.6: with nobody alive every record is cleared to 0; declining needs
/// no command (anything else is ignored: no turn, the counter and region
/// stay); an added player's visit to the map is a region change and saves,
/// after the roster add's own save (2.2.7.4); a created one saves only at
/// the visit.
#[test]
fn with_nobody_alive_the_records_are_cleared_and_the_game_waits() {
    both(|p| {
        let mut g = start(with_pit(p), 0, 2, 4);
        g.set_life_force(0, 9);
        g.give(0, GOLD);
        let point = g.save_point();
        let name = g.players()[0].name.clone();
        let ev = g.apply(Command::Move { dir: Dir::E });
        assert_eq!(ev.last(), Some(&Event::AllPlayersDead), "{ev:?}");
        assert_eq!(g.restore_point(), point);
        // The log still names the player in the lines of the turn that
        // cleared their record.
        let died = ev.iter().find(|e| matches!(e, Event::Died { .. })).unwrap();
        let line = acs_engine::describe(&g, died);
        assert!(!name.is_empty() && line.contains(&name), "{line}");
        let p0 = &g.players()[0];
        assert!(!p0.alive && p0.name.is_empty() && p0.gold == 0 && p0.life_force == 0);
        assert_eq!(p0.at, Location::World { x: 0, y: 0 });
        assert!(p0.possessions.is_empty() && p0.speed == 0);
        let (round, region, logged) = (g.round(), g.visiting(), g.log().len());
        for cmd in [Command::Wait, Command::SaveGame, Command::Move { dir: Dir::E }] {
            assert!(g.apply(cmd).is_empty());
        }
        assert_eq!((g.round(), g.visiting(), g.log().len()), (round, region, logged));
        assert_eq!(g.restore_point(), point);
        let mut h = g.clone();
        let ev = g.apply(Command::AddPlayer { setup: frail() });
        assert_eq!(saved(&ev), [(round, None), (round, Some(0))], "{ev:?}");
        assert_eq!(g.save_point(), g.log().len());
        assert_eq!(g.players()[0].name, "FRAIL");
        let ev = h.apply(Command::CreatePlayer {
            name: "NEW".into(),
            picture: None,
        });
        assert_eq!(saved(&ev), [(round, Some(0))], "{ev:?}");
        assert_eq!(h.save_point(), h.log().len());
    });
}

/// Each of slot 1's turns in `ev`: its moves, then whether it dropped and
/// the steps it took, in order (`None` a step, `Some(thing)` a drop).
fn turns_of_1(ev: &[Event]) -> Vec<(u8, Vec<Option<u16>>)> {
    let mut out: Vec<(u8, Vec<Option<u16>>)> = Vec::new();
    let mut mine = false;
    for e in ev {
        match e {
            Event::CreatureTurn { slot, moves, .. } => {
                mine = *slot == 1;
                if mine {
                    out.push((*moves, Vec::new()));
                }
            }
            Event::TurnStarted { .. } => mine = false,
            Event::CreatureMoved { slot: 1, .. } if mine => out.last_mut().unwrap().1.push(None),
            Event::CreatureDropped { slot: 1, thing, .. } if mine => {
                out.last_mut().unwrap().1.push(Some(*thing))
            }
            _ => {}
        }
    }
    out
}

/// A peaceful friend of Speed `speed` on WEST (8,5), Strength 2 (capacity
/// 200), carrying `gold` gold pieces (weight 1 each); the player far off
/// and the doors blocked.
fn burdened(p: Platform, speed: u8, gold: u16) -> Game {
    start(burdened_adventure(p, speed, gold), 0, 1, 1)
}

fn burdened_adventure(p: Platform, speed: u8, gold: u16) -> Adventure {
    let mut a = adventure(p);
    let mut s = spec(0, 8, 5, Alignment::Friend, false);
    s.speed = speed;
    put(&mut a, 1, s);
    let c = &mut a.regions[0].residents[1];
    c.strength = 2;
    c.constitution = 20;
    c.life_force = 20;
    c.possessions.clear();
    c.wealth = gold;
    // Pillars in front of WEST's four doors keep it in the room.
    for (x, y) in [(8, 3), (1, 3), (4, 1), (4, 5)] {
        item(&mut a, 0, x, y, PILLAR, 0);
    }
    a
}

#[test]
fn above_half_its_capacity_a_creature_drops_with_its_first_move_then_steps() {
    // 8.1.2 (rr28 burden2-sw): loads of 150 and 101 of 200 begin with 3
    // and 5 moves (the load before the drop), drop with the first, then
    // step with the rest; the next turn's moves come from the load after.
    both(|p| {
        for (gold, moves) in [(150, 3), (101, 5)] {
            let mut g = burdened(p, 6, gold);
            let turns = turns_of_1(&wait(&mut g));
            let (m, actions) = &turns[0];
            assert_eq!(*m, moves, "load {gold}");
            assert_eq!(actions[0], Some(GOLD), "load {gold}: {actions:?}");
            assert_eq!(&actions[1..], vec![None; moves as usize - 1], "load {gold}");
            assert_eq!(g.creature(1, 1).unwrap().wealth, 0);
            let next = turns
                .get(1)
                .cloned()
                .unwrap_or_else(|| turns_of_1(&wait(&mut g))[0].clone());
            assert_eq!(next.0, 6, "load {gold}: the next turn");
            assert!(!next.1.contains(&Some(GOLD)));
        }
    });
}

/// Slot 1's turns in `ev`: for each, its moves and its events.
fn turn_events_of_1(ev: &[Event]) -> Vec<(u8, Vec<Event>)> {
    let mut out: Vec<(u8, Vec<Event>)> = Vec::new();
    let mut mine = false;
    for e in ev {
        match e {
            Event::CreatureTurn { slot, moves, .. } => {
                mine = *slot == 1;
                if mine {
                    out.push((*moves, Vec::new()));
                }
            }
            Event::TurnStarted { .. } => mine = false,
            _ if mine => out.last_mut().unwrap().1.push(e.clone()),
            _ => {}
        }
    }
    out
}

/// A peaceful friend on WEST (8,5) of Speed `speed` and Strength
/// `strength`, owning the bow (not held) and carrying `gold` gold; the
/// player far off, the doors blocked.
fn burdened_archer(p: Platform, speed: u8, strength: u8, gold: u16) -> Game {
    let mut a = adventure(p);
    let mut s = spec(0, 8, 5, Alignment::Friend, false);
    s.speed = speed;
    put(&mut a, 1, s);
    let c = &mut a.regions[0].residents[1];
    c.strength = strength;
    c.constitution = 20;
    c.life_force = 20;
    c.possessions = vec![Possession { thing: BOW, count: 1 }];
    c.weapon = 0;
    c.wealth = gold;
    for (x, y) in [(8, 3), (1, 3), (4, 1), (4, 5)] {
        item(&mut a, 0, x, y, PILLAR, 0);
    }
    start(a, 0, 1, 1)
}

fn readied_bow(ev: &[Event]) -> bool {
    ev.iter().any(|e| {
        matches!(
            e,
            Event::CreatureReadied {
                slot: 1,
                thing: BOW,
                ..
            }
        )
    })
}

#[test]
fn a_drop_that_leaves_no_move_ends_the_turn_before_readying() {
    // 8.1.2, combat 6.2 (rr33 `r0b`): Speed 2 carrying 400 of 100, owning
    // a bow it does not hold, nobody beside: it drops and does not ready;
    // the next turn readies the bow.
    both(|p| {
        let mut g = burdened_archer(p, 2, 1, 400);
        let mut turns = turn_events_of_1(&wait(&mut g));
        while turns.len() < 2 {
            turns.extend(turn_events_of_1(&wait(&mut g)));
        }
        let (moves, first) = &turns[0];
        assert_eq!(*moves, 0, "{first:?}");
        assert!(first.iter().any(|e| matches!(
            e,
            Event::CreatureDropped {
                slot: 1,
                thing: GOLD,
                ..
            }
        )));
        assert!(!readied_bow(first), "{first:?}");
        let (_, second) = &turns[1];
        assert!(readied_bow(second), "{second:?}");
        assert!(!second
            .iter()
            .any(|e| matches!(e, Event::CreatureDropped { slot: 1, .. })));
    });
}

#[test]
fn with_moves_left_after_a_drop_a_creature_readies_then_steps() {
    // 8.1.2 (engine choice): with moves left the readying follows the drop,
    // before the first step: a load above 150 of 200 at Speed 6 leaves
    // moves for the drop and the bow.
    both(|p| {
        let mut g = burdened_archer(p, 6, 2, 150);
        let turns = turn_events_of_1(&wait(&mut g));
        let (moves, ev) = &turns[0];
        assert!(*moves >= 2, "{moves}");
        let drop = ev
            .iter()
            .position(|e| {
                matches!(
                    e,
                    Event::CreatureDropped {
                        slot: 1,
                        thing: GOLD,
                        ..
                    }
                )
            })
            .expect("drops");
        let ready = ev
            .iter()
            .position(|e| {
                matches!(
                    e,
                    Event::CreatureReadied {
                        slot: 1,
                        thing: BOW,
                        ..
                    }
                )
            })
            .expect("readies");
        assert!(drop < ready, "{ev:?}");
        assert!(!ev[..ready]
            .iter()
            .any(|e| matches!(e, Event::CreatureMoved { slot: 1, .. })));
    });
}

#[test]
fn at_exactly_half_its_capacity_a_creature_walks_its_full_moves_and_keeps_its_load() {
    // 8.1.2: exactly half is not overburdened.
    both(|p| {
        let mut g = burdened(p, 6, 100);
        let turns = turns_of_1(&wait(&mut g));
        assert_eq!(turns[0], (6, vec![None; 6]));
        assert_eq!(g.creature(1, 1).unwrap().wealth, 100);
    });
}

#[test]
fn a_speed_0_creature_never_drops_however_much_it_carries() {
    // 8.1.2: 400 of a capacity of 100, never a drop.
    both(|p| {
        let mut a = adventure(p);
        let mut s = spec(0, 8, 5, Alignment::Friend, false);
        s.speed = 0;
        put(&mut a, 1, s);
        let c = &mut a.regions[0].residents[1];
        c.strength = 1;
        c.constitution = 20;
        c.life_force = 20;
        c.possessions = vec![Possession {
            thing: ROCK,
            count: 2,
        }];
        let mut g = start(a, 0, 1, 1);
        for _ in 0..6 {
            let ev = wait(&mut g);
            assert!(!ev
                .iter()
                .any(|e| matches!(e, Event::CreatureDropped { slot: 1, .. })));
        }
        assert_eq!(creature_rocks(&g), 2);
    });
}

fn creature_rocks(g: &Game) -> u8 {
    g.creature(1, 1)
        .unwrap()
        .possessions
        .iter()
        .filter(|p| p.thing == ROCK)
        .map(|p| p.count)
        .sum()
}

#[test]
fn a_foe_beside_it_keeps_an_overburdened_creature_from_dropping() {
    // 8.1.2 (engine choice): no drop with a foe beside, the player here.
    both(|p| {
        let mut a = adventure(p);
        let mut s = spec(0, 5, 3, Alignment::Enemy, false);
        s.speed = 6;
        put(&mut a, 1, s);
        let c = &mut a.regions[0].residents[1];
        c.strength = 2;
        c.constitution = 20;
        c.life_force = 20;
        c.possessions.clear();
        c.wealth = 150;
        let mut g = start(a, 0, 6, 3);
        let ev = wait(&mut g);
        assert!(turns_of_1(&ev)[0].1.iter().all(|a| *a != Some(GOLD)), "{ev:?}");
        assert_eq!(g.creature(1, 1).unwrap().wealth, 150);
    });
}

#[test]
fn a_creatures_moves_come_from_the_life_force_its_catch_up_left() {
    // Inventory 3.4 (engine): catch-up and drift, then the moves, then the
    // die. Strength 2, Constitution 20, Life Force 17 (wound 3): capacity
    // 100, so a load of 100 gold leaves Speed 0. A catch-up of many rounds
    // heals it to 20 first: capacity 200, the load at half, the full 6
    // moves on that same turn.
    both(|p| {
        for (byte, moves) in [(0u8, 0u8), (128, 6)] {
            let mut g = burdened(p, 6, 100);
            let c = g.creature_mut(1, 1).unwrap();
            c.life_force = 17;
            c.last_turn_round = byte;
            let turns = turns_of_1(&wait(&mut g));
            assert_eq!(turns[0].0, moves, "byte {byte}");
        }
    });
}

#[test]
fn a_creatures_moves_are_set_before_its_turn_top_die() {
    // Inventory 3.4, creatures 2.5 (capture.md 10.43 item 9): Speed 10,
    // Strength 10, Constitution 20, Life Force 14, carrying 600 (capacity
    // 700: 2 moves; at 15 the capacity is 800 and the moves 5). Its first
    // turn starts with 2 moves whatever the die does; a heal by the die
    // lengthens only the next turn.
    both(|p| {
        let mut healed = 0;
        for seed in 0..60 {
            let mut a = burdened_adventure(p, 10, 600);
            let c = &mut a.regions[0].residents[1];
            c.strength = 10;
            c.life_force = 14;
            let mut g = start_seeded(a, seed, 0, 1, 1);
            let ev = wait(&mut g);
            let turns = turns_of_1(&ev);
            assert_eq!(turns[0].0, 2, "seed {seed}: {turns:?}");
            if turns.len() == 1 && turns[0].1.len() == 2 && g.creature(1, 1).unwrap().life_force == 15 {
                healed += 1;
            }
        }
        // The die (no idle move: both moves used) healed it on some seeds.
        assert!(healed > 0);
    });
}

/// Inventory 2.7: a creature's take adds the placed parameter too; a
/// treasure placed with 0 is taken off the floor and owned by no one.
#[test]
fn a_creatures_take_of_a_thing_placed_with_0_owns_nothing() {
    both(|p| {
        let mut a = adventure(p);
        put(&mut a, 1, spec(0, 2, 3, Alignment::Thief, false));
        item(&mut a, 0, 4, 3, GEM, 0);
        let mut g = start(a, 0, 1, 1);
        let mut took = false;
        for _ in 0..6 {
            let ev = wait(&mut g);
            took |= ev.iter().any(|e| {
                matches!(
                    e,
                    Event::CreaturePickedUp {
                        slot: 1,
                        thing: GEM,
                        ..
                    }
                )
            });
            if took {
                break;
            }
        }
        assert!(took, "the thief took the gem");
        let c = g.creature(1, 1).cloned().expect("alive");
        assert_eq!(acs_engine::creatures::count(&c, GEM), 0, "{c:?}");
        assert!(!g.adventure().regions[0].rooms[0]
            .items
            .iter()
            .any(|it| it.thing == GEM));
    });
}

// ---- capture.md 10.41: the slot mark, carried moves, the bump at the goal

/// The generator's state, read from the game's serialized form.
fn rng_of(g: &Game) -> acs_engine::Rng {
    serde_json::from_value(serde_json::to_value(g).expect("serializes")["rng"].clone()).expect("an Rng")
}

/// The game with exactly these region-1 slots carrying the random mark,
/// listed in this order (2.4).
fn marked(g: &Game, slots: &[usize]) -> Game {
    let mut v = serde_json::to_value(g).expect("serializes");
    v["random"] = serde_json::json!(slots.iter().map(|&s| (1u8, s)).collect::<Vec<_>>());
    serde_json::from_value(v).expect("deserializes")
}

#[test]
fn every_marked_slot_draws_at_the_round_end_from_slot_0_up() {
    // 2.4 (capture.md 10.41 item 1): the slot roll, and when it hits a
    // slot in use one percentile per marked slot from slot 0 up, a marked
    // empty slot (0 here) drawing its percentile with nothing happening.
    // Slots 2 and 3 are marked random creatures in EAST, slot 5 an
    // unmarked resident there; the player waits in WEST.
    both(|p| {
        let mut a = adventure(p);
        for slot in [2, 3, 5] {
            let mut s = spec(1, slot as u8, 2, Alignment::Friend, false);
            s.speed = 0;
            put(&mut a, slot, s);
        }
        let mut removals = 0;
        for seed in 0..24 {
            let g = start_seeded(a.clone(), seed, 0, 2, 2);
            // Listed out of order: the draws still go 0, 2, 3.
            let mut g = marked(&g, &[3, 0, 2]);
            for _ in 0..12 {
                let pl = g.current_player();
                let (left, full) = (pl.moves_left, pl.turn_moves);
                let mut r = rng_of(&g);
                // The player's idle moves (magic 5.3.1), then the round end.
                for _ in 0..left {
                    r.below(u32::from(full));
                }
                let mut gone = Vec::new();
                let pick = r.below(16) as usize;
                if g.creature(1, pick).is_some() {
                    for slot in [0, 2, 3] {
                        if r.percent(20) && g.creature(1, slot).is_some() {
                            gone.push(slot);
                        }
                    }
                }
                let ev = wait(&mut g);
                let seen: Vec<usize> = ev
                    .iter()
                    .filter_map(|e| match e {
                        Event::CreatureRemoved {
                            slot,
                            reason: RemovalReason::LeftBehind,
                            ..
                        } => Some(*slot),
                        _ => None,
                    })
                    .collect();
                assert_eq!(seen, gone, "seed {seed}");
                removals += gone.len();
                // The removals keep their marks; the resident never had one.
                assert!(g.is_random(1, 0) && g.is_random(1, 2) && g.is_random(1, 3));
                assert!(!g.is_random(1, 5));
                assert!(g.creature(1, 5).is_some());
                // The next turn's healing die (magic 5.3.2) and nothing else.
                r.below(6);
                assert_eq!(rng_of(&g), r, "seed {seed}: the round end's draws");
            }
        }
        assert!(removals > 0);
    });
}

#[test]
fn a_random_creature_marks_a_slot_and_its_removal_keeps_the_mark() {
    // 2.4, 3.3 (engine choice): the mark goes with the slot, not the
    // creature; a random creature put into a marked slot keeps it set.
    both(|p| {
        let mut a = adventure(p);
        let mut bat = a.regions[0].residents[0].clone();
        bat.in_use = true;
        bat.name = "BAT".into();
        bat.speed = 0;
        a.regions[0].random_creatures[2] = bat;
        a.regions[0].rooms[1].random_chance = 100;
        a.regions[0].rooms[1].random_creature = 2;
        let g = start(a, 0, 8, 3);
        let mut g = marked(&g, &[0]);
        assert!(g.creature(1, 0).is_none() && g.is_random(1, 0));
        g.apply(Command::Move { dir: Dir::E });
        assert!(g.creature(1, 0).is_some() && g.is_random(1, 0));
        assert_eq!(
            serde_json::to_value(&g).expect("serializes")["random"],
            serde_json::json!([[1, 0]]),
            "marked once"
        );
    });
}

/// The moves of every `CreatureTurn` of `slot`.
fn turns_of(ev: &[Event], slot: usize) -> Vec<u8> {
    ev.iter()
        .filter_map(|e| match e {
            Event::CreatureTurn { slot: s, moves, .. } if *s == slot => Some(*moves),
            _ => None,
        })
        .collect()
}

#[test]
fn a_creature_carries_its_moves_through_a_door_and_is_not_halved() {
    // 5.4, 2.6 (capture.md 10.41 item 4): its next turn starts with the
    // moves left after the door's step, not halved beside a foe; with none
    // carried, the full bar (halved beside one).
    both(|p| {
        let mut carried_seen = 0;
        for seed in 0..16 {
            let mut a = adventure(p);
            // An unarmed aggressive enemy, Speed 15: the player its foe.
            put(&mut a, 1, spec(0, 6, 3, Alignment::Enemy, true));
            let mut g = start_seeded(a, seed, 0, 8, 2);
            // Through the east door onto EAST (1,3); the enemy follows.
            let ev = g.apply(Command::Move { dir: Dir::SE });
            let at_door = ev
                .iter()
                .position(|e| matches!(e, Event::CreatureTookDoor { slot: 1, .. }));
            let Some(at_door) = at_door else {
                continue;
            };
            let steps = ev[..at_door]
                .iter()
                .filter(|e| matches!(e, Event::CreatureMoved { slot: 1, .. }))
                .count() as u8;
            let carry = 15 - steps - 1;
            assert_eq!(xy(&g, 1), (1, 1, 3));
            // The player steps off east, so the enemy starts beside it.
            g.apply(Command::Move { dir: Dir::E });
            assert_eq!(g.current_player().at, at(1, 2, 3));
            let ev = wait(&mut g);
            let turns = turns_of(&ev, 1);
            let expected = if carry > 0 { carry } else { 8 };
            assert_eq!(turns.first(), Some(&expected), "seed {seed}: {ev:?}");
            carried_seen += usize::from(carry > 0 && carry != 8);
        }
        assert!(carried_seen > 0);
    });
}

/// PILLAR set to bump for Life Force -5 (the player stands on it), and
/// to vanish when bumped if `vanishes`.
fn bump_pillar(a: &mut Adventure, vanishes: bool) {
    use acs_core::model::{SpellAction, SpellActionKind, ThingKind};
    let t = &mut a.things[PILLAR as usize - 1];
    t.kind = ThingKind::Obstacle {
        picture: 7,
        bump_invokes_spell: true,
        action: SpellAction {
            kind: SpellActionKind::ChangeLifeForce,
            parameter: 0x85, // -5, permanent
        },
    };
    t.raw = [if vanishes { 0x0A | 0x10 } else { 0x0A }, 0, 0, 0];
}

/// A slinker friend of Speed `speed` (it seeks the player, 6.3) in slot 1.
fn seeker(a: &mut Adventure, x: u8, y: u8, speed: u8) {
    let mut s = spec(0, x, y, Alignment::Friend, false);
    s.speed = speed;
    put(a, 1, s);
    let c = &mut a.regions[0].residents[1];
    c.constitution = 30;
    c.life_force = 30;
}

fn bumps(ev: &[Event]) -> usize {
    ev.iter()
        .filter(|e| matches!(e, Event::CreatureBumped { slot: 1, .. }))
        .count()
}

/// The first of seeds 0..64 whose first wait has the seeker bump, with
/// that wait's events.
fn first_bump(a: &Adventure) -> (Game, Vec<Event>) {
    (0..64)
        .find_map(|seed| {
            let mut g = start_seeded(a.clone(), seed, 0, 5, 3);
            let ev = wait(&mut g);
            (bumps(&ev) > 0).then_some((g, ev))
        })
        .expect("a bump in 64 seeds")
}

#[test]
fn a_creature_bumps_the_obstacle_its_goal_stands_on() {
    // 5.1 (engine choice, capture.md 10.44 item 1): the goal's square, its
    // top an obstacle, opens to a creature that does not strike only on a
    // turn it began on that square, once it has stepped off; picking it is
    // a bump: it stays put, the bump spell acts on it, the move is spent
    // and the turn goes on.
    both(|p| {
        let mut a = adventure(p);
        bump_pillar(&mut a, false);
        item(&mut a, 0, 5, 3, PILLAR, 0);
        seeker(&mut a, 5, 3, 2);
        let (g, ev) = first_bump(&a);
        assert_eq!(turns_of(&ev, 1), vec![2]);
        assert_eq!(bumps(&ev), 1, "{ev:?}");
        let Some(Event::CreatureMoved { to, .. }) = ev
            .iter()
            .find(|e| matches!(e, Event::CreatureMoved { slot: 1, .. }))
        else {
            panic!("it stepped off first: {ev:?}");
        };
        assert!(ev.contains(&Event::CreatureBumped {
            region: 1,
            slot: 1,
            at: at(0, 5, 3),
            thing: PILLAR,
        }));
        let Location::Room { x, y, .. } = *to else {
            unreachable!()
        };
        assert_eq!(xy(&g, 1), (0, x, y), "it stays where it stepped");
        assert_eq!(g.creature(1, 1).expect("alive").life_force, 25);
        assert_eq!(top(&g, 0, 5, 3), Some(PILLAR));
        assert_eq!(g.current_player().at, at(0, 5, 3));
    });
}

#[test]
fn a_creature_beside_the_goals_obstacle_never_bumps() {
    // 5.1 (capture.md 10.44 item 1): from beside the square, on a turn it
    // did not begin there, the goal's obstacle square is closed.
    both(|p| {
        for seed in 0..32 {
            let mut a = adventure(p);
            bump_pillar(&mut a, false);
            item(&mut a, 0, 5, 3, PILLAR, 0);
            seeker(&mut a, 6, 3, 2);
            let mut g = start_seeded(a, seed, 0, 5, 3);
            for _ in 0..3 {
                let ev = wait(&mut g);
                assert_eq!(bumps(&ev), 0, "seed {seed}: {ev:?}");
            }
            assert_eq!(g.creature(1, 1).expect("alive").life_force, 30);
        }
    });
}

#[test]
fn a_disappearing_obstacle_vanishes_at_a_creature_bump() {
    // 5.1, movement 5.9: the first bump removes it; the player's square is
    // then closed to the creature again.
    both(|p| {
        let mut a = adventure(p);
        bump_pillar(&mut a, true);
        item(&mut a, 0, 5, 3, PILLAR, 0);
        seeker(&mut a, 5, 3, 3);
        let (g, ev) = first_bump(&a);
        assert_eq!(bumps(&ev), 1, "{ev:?}");
        assert!(ev
            .iter()
            .any(|e| matches!(e, Event::ObstacleVanished { thing: PILLAR, .. })));
        assert_eq!(top(&g, 0, 5, 3), None);
        assert_eq!(g.creature(1, 1).expect("alive").life_force, 25);
        assert_ne!(xy(&g, 1), (0, 5, 3));
    });
}

#[test]
fn the_goal_obstacle_costs_4_once_stood_on_and_strikers_never_bump() {
    // 5.1: a creature that shared the goal's square and stepped off scores
    // it as stood on (cost 4), so it seldom picks it; with the goal's cost
    // 1 it always would.
    both(|p| {
        let mut bumped = 0;
        for seed in 0..32 {
            let mut a = adventure(p);
            bump_pillar(&mut a, false);
            item(&mut a, 0, 5, 3, PILLAR, 0);
            seeker(&mut a, 5, 3, 2);
            let mut g = start_seeded(a, seed, 0, 5, 3);
            bumped += usize::from(bumps(&wait(&mut g)) > 0);
        }
        assert!(bumped < 8, "{bumped} of 32");
        // A fighter owning a melee weapon strikes instead (combat 2.3),
        // though it began on the square.
        let mut a = adventure(p);
        bump_pillar(&mut a, false);
        item(&mut a, 0, 5, 3, PILLAR, 0);
        put(&mut a, 1, spec(0, 5, 3, Alignment::Enemy, true));
        armed(&mut a, 1);
        let mut g = start(a, 0, 5, 3);
        let mut ev = Vec::new();
        for _ in 0..3 {
            ev.extend(wait(&mut g));
        }
        assert_eq!(bumps(&ev), 0);
        assert!(ev.iter().any(|e| matches!(e, Event::Attacked { .. })), "{ev:?}");
    });
}

/// PILLAR as a custom obstacle: Life Force -5 in its master byte, the
/// placement's parameter 0, property bytes `raw`.
fn bump_idol(a: &mut Adventure, raw: [u8; 4]) {
    use acs_core::model::{SpellAction, SpellActionKind, ThingKind};
    let t = &mut a.things[PILLAR as usize - 1];
    t.kind = ThingKind::CustomObstacle {
        picture: 7,
        action: SpellAction {
            kind: SpellActionKind::ChangeLifeForce,
            parameter: 0x85, // -5, permanent
        },
    };
    t.raw = raw;
}

#[test]
fn a_creatures_bump_of_a_custom_obstacle_follows_its_bits() {
    // 5.1, movement 5.5, 5.9: the spell only with bit 6 of byte 1, the
    // master's parameter without bit 7 of byte 0; with bit 4 it goes,
    // the spell run or not.
    both(|p| {
        for (raw, hurt, goes) in [
            ([0x0B, 0x40, 0, 0], true, false),
            ([0x0B | 0x10, 0, 0, 0], false, true),
            ([0x0B | 0x10, 0x40, 0, 0], true, true),
        ] {
            let mut a = adventure(p);
            bump_idol(&mut a, raw);
            item(&mut a, 0, 5, 3, PILLAR, 0);
            seeker(&mut a, 5, 3, 2);
            let (g, ev) = first_bump(&a);
            assert_eq!(bumps(&ev), 1, "{raw:?}: {ev:?}");
            let life = if hurt { 25 } else { 30 };
            assert_eq!(g.creature(1, 1).expect("alive").life_force, life, "{raw:?}");
            assert_eq!(
                ev.iter()
                    .any(|e| matches!(e, Event::ObstacleVanished { thing: PILLAR, .. })),
                goes,
                "{raw:?}"
            );
            let left = if goes { None } else { Some(PILLAR) };
            assert_eq!(top(&g, 0, 5, 3), left, "{raw:?}");
        }
    });
}

#[test]
fn a_mimic_becomes_the_players_copy_once_and_drops_its_gold() {
    // 4.5 (capture.md 10.47): at its first turn with the player in its
    // room, a mimic takes the player's ratings, gear and things, keeps its
    // own Life Force, place and strategy, and drops its gold on its square
    // with its first move; a later change in the player is not copied.
    both(|p| {
        let mut a = adventure(p);
        put(&mut a, 1, spec(0, 8, 3, Alignment::Enemy, false));
        let c = &mut a.regions[0].residents[1];
        c.mimic = true;
        c.speed = 3;
        c.strength = 5;
        c.constitution = 12;
        c.life_force = 12;
        c.wealth = 7;
        c.possessions = vec![Possession { thing: GEM, count: 1 }];
        let mut control = a.clone();
        control.regions[0].residents[1].mimic = false;
        let mut g = start(a, 0, 1, 1);
        let ev = wait(&mut g);
        assert!(ev.iter().any(|e| matches!(
            e,
            Event::CreatureMimicked {
                slot: 1,
                player: 0,
                ..
            }
        )));
        let t = g.players()[0].clone();
        let c = g.creature(1, 1).unwrap();
        assert_eq!(
            (
                c.speed,
                c.strength,
                c.constitution,
                c.dexterity,
                c.wisdom,
                c.melee,
                c.dodge
            ),
            (
                t.speed,
                t.strength,
                t.constitution,
                t.dexterity,
                t.wisdom,
                t.melee,
                t.dodge
            )
        );
        assert_eq!(c.life_force, 12, "its own Life Force");
        assert_eq!(c.strategy.alignment, Alignment::Enemy);
        assert!(
            c.possessions.is_empty(),
            "its gem is gone: the player carries nothing"
        );
        // Its gold, on its own square (its goal may take it back later).
        assert_eq!(dropped_at(&ev, 1, GOLD), Some((8, 3)));
        // Once only.
        let ev = wait(&mut g);
        assert!(!ev.iter().any(|e| matches!(e, Event::CreatureMimicked { .. })));
        // Without the Mimics choice nothing is copied.
        let mut g = start(control, 0, 1, 1);
        let ev = wait(&mut g);
        assert!(!ev.iter().any(|e| matches!(e, Event::CreatureMimicked { .. })));
        let c = g.creature(1, 1).unwrap();
        assert_eq!((c.strength, c.wealth), (5, 7));
    });
}
