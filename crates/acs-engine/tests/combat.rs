// SPDX-License-Identifier: GPL-3.0-or-later

//! Combat rules (`docs/rules/combat.md`) on the creature tests' synthetic
//! adventure, built byte by byte with acs-core's fixture builder. No real
//! adventure data.
#![allow(dead_code, unused_imports)]

#[path = "../../acs-core/tests/common/mod.rs"]
mod common;

use acs_core::layout::world_map as wm;
use acs_core::model::{Alignment, Creature, MagicDefense, Possession, RoomItem, ThingKind};
use acs_core::{decode, Adventure, Platform};
use acs_engine::combat;
use acs_engine::inventory::GOLD;
use acs_engine::{
    BlowOutcome, Command, Dir, Event, Game, Location, PlayerSetup, RemovalReason, Rules, Slot, Stat, Who,
};
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

const ME: Who = Who::Player { player: 0 };
const C1: Who = Who::Creature { region: 1, slot: 1 };

/// A player of Speed 15 on WEST (2,2) with `setup`'s other traits; creature
/// slot 1 (a peaceful neutral of Speed 0, so it never walks, Constitution
/// and Life Force 40, Power and Wisdom 10) east of it on (3,2), as `them`
/// changes it.
fn duel(p: Platform, seed: u64, setup: PlayerSetup, them: impl Fn(&mut Creature)) -> Game {
    let mut a = adventure(p);
    let mut s = spec(0, 3, 2, Alignment::Neutral, false);
    s.speed = 0;
    put(&mut a, 1, s);
    let c = &mut a.regions[0].residents[1];
    c.constitution = 40;
    c.life_force = 40;
    c.power = 10;
    c.wisdom = 10;
    them(c);
    let (mut g, _) = Game::new(
        a,
        seed,
        Rules { diagonals: true },
        PlayerSetup { speed: 15, ..setup },
    );
    g.place(0, at(0, 2, 2));
    g
}

/// Owns the sword (not readied), Melee 127 (the hit chance at its 90).
fn swordsman() -> PlayerSetup {
    PlayerSetup {
        possessions: vec![SWORD],
        melee: 127,
        ..Default::default()
    }
}

fn blows(ev: &[Event]) -> Vec<(Who, bool, BlowOutcome)> {
    ev.iter()
        .filter_map(|e| match e {
            Event::Blow {
                by, counter, outcome, ..
            } => Some((*by, *counter, *outcome)),
            _ => None,
        })
        .collect()
}

fn damaging(o: &BlowOutcome) -> bool {
    matches!(o, BlowOutcome::Hit { damage, .. } if *damage > 0)
}

fn sword_power(g: &Game) -> u8 {
    match &g.adventure().things[SWORD as usize - 1].kind {
        ThingKind::MeleeWeapon { stats, .. } => stats.power,
        _ => panic!("the sword"),
    }
}

#[test]
fn owning_no_melee_weapon_a_player_bumps_and_loses_nothing() {
    both(|p| {
        let mut g = duel(p, 1, PlayerSetup::default(), |_| {});
        let ev = g.apply(Command::Move { dir: Dir::E });
        assert!(
            matches!(
                ev[..],
                [Event::BumpedCreature {
                    player: 0,
                    resident: 1,
                    ..
                }]
            ),
            "{ev:?}"
        );
        assert_eq!(g.current_player().moves_left, 15);
    });
}

#[test]
fn a_player_owning_a_sword_readies_it_on_contact_and_the_attack_ends_the_turn() {
    both(|p| {
        let mut g = duel(p, 1, swordsman(), |_| {});
        let ev = g.apply(Command::Move { dir: Dir::E });
        // 2.7: readied (a move: 14 of 15 left), then 4 x 14 / 15 + 11 / 8 - 1
        // = 3 blows (3.1); the turn ends (2.5).
        assert_eq!(
            ev[0],
            Event::Readied {
                player: 0,
                thing: SWORD,
                slot: Slot::Weapon
            }
        );
        assert_eq!(
            ev[1],
            Event::Attacked {
                by: ME,
                target: C1,
                weapon: SWORD,
                blows: 3
            }
        );
        assert!(ev.contains(&Event::TurnEnded { player: 0 }));
        assert_eq!(g.players()[0].at, at(0, 2, 2), "an attack does not move");
    });
}

#[test]
fn damage_takes_life_force_and_power_by_the_share() {
    both(|p| {
        let mut hits = 0;
        for seed in 0..40 {
            let mut g = duel(p, seed, swordsman(), |c| c.strategy.aggressive = true);
            let ev = g.apply(Command::Move { dir: Dir::E });
            // The attack's own events: up to the player's turn's end (the
            // creature's turn and its healing come after).
            let end = ev
                .iter()
                .position(|e| matches!(e, Event::TurnEnded { .. }))
                .unwrap();
            let (mut life, mut power) = (40u8, 10u8);
            for e in &ev[..end] {
                match *e {
                    Event::Blow {
                        by: ME,
                        outcome: BlowOutcome::Hit { damage, heavy },
                        ..
                    } => {
                        // Section 4: the sword's Power, Build 11 + 11 - 6 - 15.
                        let want = combat::damage(11, 11, sword_power(&g), heavy);
                        assert_eq!(damage as i32, want);
                        let (pl, ll) = combat::losses(want, power, life);
                        power = (power as i32 - pl).max(0) as u8;
                        life = (life as i32 - ll).max(0) as u8;
                        hits += 1;
                    }
                    Event::StatChanged {
                        who: C1,
                        stat: Stat::LifeForce,
                        to,
                        ..
                    } => assert_eq!(to, life, "seed {seed}"),
                    Event::StatChanged {
                        who: C1,
                        stat: Stat::Power,
                        to,
                        ..
                    } => assert_eq!(to, power, "seed {seed}"),
                    _ => {}
                }
            }
        }
        assert!(hits > 20, "{hits} hits");
    });
}

#[test]
fn a_damaging_blow_uses_two() {
    both(|p| {
        for seed in 0..40 {
            let mut g = duel(p, seed, swordsman(), |c| c.strategy.aggressive = true);
            let ev = g.apply(Command::Move { dir: Dir::E });
            let mut left = 3i32;
            for (by, counter, o) in blows(&ev) {
                assert!(left > 0, "a blow past the count, seed {seed}");
                assert_eq!((by, counter), (ME, false), "no counter without a weapon");
                left -= if damaging(&o) { 2 } else { 1 };
            }
            assert!(left <= 0, "seed {seed}: {ev:?}");
        }
    });
}

#[test]
fn a_friend_hurt_by_a_player_turns_neutral_then_aggressive() {
    both(|p| {
        // Section 7: one change a damaging blow.
        let mut twice = false;
        for seed in 0..20 {
            let mut g = duel(p, seed, swordsman(), |c| {
                c.strategy.alignment = Alignment::Friend;
                c.life_force = 63;
                c.constitution = 63;
            });
            let ev = g.apply(Command::Move { dir: Dir::E });
            let turned: Vec<_> = ev
                .iter()
                .filter_map(|e| match e {
                    Event::CreatureTurned {
                        alignment,
                        aggressive,
                        ..
                    } => Some((*alignment, *aggressive)),
                    _ => None,
                })
                .collect();
            let n = blows(&ev).iter().filter(|b| damaging(&b.2)).count();
            let want: Vec<_> = [(Alignment::Neutral, false), (Alignment::Neutral, true)]
                .into_iter()
                .take(n)
                .collect();
            assert_eq!(turned, want, "seed {seed}");
            twice |= n >= 2;
        }
        assert!(twice);
    });
}

#[test]
fn a_defender_holding_a_melee_weapon_strikes_back_on_a_miss() {
    both(|p| {
        let mut counters = 0;
        for seed in 0..40 {
            let setup = PlayerSetup {
                melee: 0,
                ..swordsman()
            };
            let mut g = duel(p, seed, setup, |c| {
                c.possessions = vec![Possession {
                    thing: SWORD,
                    count: 1,
                }];
                c.weapon = SWORD;
                c.life_force = 63;
                c.constitution = 63;
            });
            let ev = g.apply(Command::Move { dir: Dir::E });
            let b = blows(&ev);
            for (i, &(by, counter, o)) in b.iter().enumerate() {
                if counter {
                    counters += 1;
                    assert_eq!(by, C1);
                    // 3.3.6: only right after a miss of ours.
                    assert_eq!(b[i - 1], (ME, false, BlowOutcome::Miss));
                }
                if by == ME && o == BlowOutcome::Miss && i + 1 < b.len() {
                    assert!(b[i + 1].1, "seed {seed}: a miss draws a counter");
                }
            }
        }
        assert!(counters > 10, "{counters} counters");
    });
}

#[test]
fn magic_defense_2_stops_a_non_magic_weapon_and_calls_the_attack_off() {
    both(|p| {
        let mut stopped = false;
        for seed in 0..20 {
            let mut g = duel(p, seed, swordsman(), |c| {
                c.magic_defense = MagicDefense::OnlyMagicDamages
            });
            let ev = g.apply(Command::Move { dir: Dir::E });
            let b = blows(&ev);
            if let Some(i) = b.iter().position(|x| x.2 == BlowOutcome::Stopped) {
                assert_eq!(i, b.len() - 1, "the attack is called off");
                stopped = true;
            }
            assert!(!b.iter().any(|x| matches!(x.2, BlowOutcome::Hit { .. })));
            assert_eq!(g.stat(C1, Stat::LifeForce), Some(40));
        }
        assert!(stopped);
    });
}

#[test]
fn a_creature_killed_in_battle_leaves_its_slot() {
    both(|p| {
        let mut died = false;
        for seed in 0..20 {
            let mut g = duel(p, seed, swordsman(), |c| c.life_force = 1);
            let ev = g.apply(Command::Move { dir: Dir::E });
            if ev.iter().any(|e| matches!(e, Event::Died { who: C1, .. })) {
                died = true;
                assert!(g.creature(1, 1).is_none());
                // The attack stops with the death.
                assert!(matches!(
                    blows(&ev).last(),
                    Some((ME, false, BlowOutcome::Hit { .. }))
                ));
            }
        }
        assert!(died);
    });
}

#[test]
fn armor_that_disappears_goes_after_the_first_blow_it_meets() {
    both(|p| {
        let mut gone = false;
        for seed in 0..20 {
            let mut a = adventure(p);
            // The bow's record turned into armor that disappears after use.
            let bow = &mut a.things[BOW as usize - 1];
            if let ThingKind::MissileWeapon {
                picture,
                carry,
                mut stats,
                ..
            } = bow.kind.clone()
            {
                stats.power = 2;
                stats.attack_adjustment = 0;
                bow.kind = ThingKind::Armor {
                    picture,
                    disappears_after_use: true,
                    carry,
                    stats,
                };
            }
            let mut s = spec(0, 3, 2, Alignment::Neutral, true);
            s.speed = 0;
            put(&mut a, 1, s);
            let c = &mut a.regions[0].residents[1];
            c.possessions = vec![Possession { thing: BOW, count: 1 }];
            c.armor = BOW;
            c.life_force = 63;
            c.constitution = 63;
            let (mut g, _) = Game::new(
                a,
                seed,
                Rules { diagonals: true },
                PlayerSetup {
                    speed: 15,
                    ..swordsman()
                },
            );
            g.place(0, at(0, 2, 2));
            let ev = g.apply(Command::Move { dir: Dir::E });
            let first_hit = ev.iter().position(|e| {
                matches!(
                    e,
                    Event::Blow {
                        outcome: BlowOutcome::Hit { .. },
                        ..
                    }
                )
            });
            if let Some(i) = first_hit {
                gone = true;
                assert!(ev[..i].iter().all(|e| !matches!(e, Event::Broke { .. })));
                assert!(
                    ev.contains(&Event::Broke {
                        who: C1,
                        thing: BOW,
                        used_up: true
                    }),
                    "seed {seed}"
                );
                assert_eq!(g.creature(1, 1).map(|c| c.armor), Some(0));
            }
        }
        assert!(gone);
    });
}

#[test]
fn a_fighter_enemy_attacks_a_player_it_reaches_and_its_turn_ends() {
    both(|p| {
        let (mut hurt, mut attacks) = (false, 0);
        for seed in 0..10 {
            let mut a = adventure(p);
            put(&mut a, 1, spec(0, 7, 5, Alignment::Enemy, true));
            armed(&mut a, 1);
            let mut g = start_seeded(a, seed, 0, 2, 2);
            g.set_life_force(0, 63);
            // Its steps take a roll each (creatures 6.5): now and then a
            // turn's wandering doesn't reach the player.
            let ev = wait(&mut g);
            let Some(at) = ev.iter().position(|e| {
                matches!(
                    e,
                    Event::Attacked {
                        by: C1,
                        target: ME,
                        ..
                    }
                )
            }) else {
                continue;
            };
            attacks += 1;
            // 2.5: no step of the creature's after its attack this turn.
            let end = ev[at..]
                .iter()
                .position(|e| matches!(e, Event::TurnStarted { .. }))
                .map_or(ev.len(), |n| at + n);
            assert!(!ev[at..end]
                .iter()
                .any(|e| matches!(e, Event::CreatureMoved { .. })));
            hurt |= g.players()[0].life_force < 63;
        }
        assert!(hurt && attacks >= 7, "{attacks} attacks");
    });
}

#[test]
fn a_fighter_friend_never_strikes_a_player() {
    both(|p| {
        for seed in 0..10 {
            let mut a = adventure(p);
            put(&mut a, 1, spec(0, 3, 2, Alignment::Friend, true));
            armed(&mut a, 1);
            let mut g = start_seeded(a, seed, 0, 2, 2);
            for _ in 0..3 {
                let ev = wait(&mut g);
                assert!(
                    !ev.iter().any(|e| matches!(e, Event::Attacked { .. })),
                    "seed {seed}"
                );
            }
        }
    });
}

// ---- firing (section 8) ----------------------------------------------------

const C2: Who = Who::Creature { region: 1, slot: 2 };

/// The bow made Power 10, range 7, break chance 0, adjustment 0, plain.
fn plain_bow(a: &mut Adventure) {
    if let ThingKind::MissileWeapon { stats, .. } = &mut a.things[BOW as usize - 1].kind {
        stats.power = 10;
        stats.range = Some(7);
        stats.break_chance = 0;
        stats.attack_adjustment = 0;
        stats.magic = false;
    }
}

/// An aggressive enemy archer in slot 1 on WEST (7,3) holding the bow (or
/// only owning it, `held` false): Missile 22, Power 13, Dexterity 13,
/// unhurt, Speed `speed`. The player will stand on (2,3), 5 squares west.
fn archer(p: Platform, held: bool, speed: u8) -> Adventure {
    let mut a = adventure(p);
    plain_bow(&mut a);
    let mut s = spec(0, 7, 3, Alignment::Enemy, true);
    s.speed = speed;
    put(&mut a, 1, s);
    let c = &mut a.regions[0].residents[1];
    c.possessions = vec![Possession { thing: BOW, count: 1 }];
    c.weapon = if held { BOW } else { 0 };
    c.missile = 22;
    c.power = 13;
    c.dexterity = 13;
    c.wisdom = 10;
    c.constitution = 40;
    c.life_force = 40;
    a
}

/// The player on (2,3) at Life Force 63 waits out its turn once.
fn shoot(a: Adventure, seed: u64) -> (Game, Vec<Event>) {
    let mut g = start_seeded(a, seed, 0, 2, 3);
    g.set_life_force(0, 63);
    let ev = wait(&mut g);
    (g, ev)
}

fn fired(ev: &[Event]) -> Option<u8> {
    ev.iter().find_map(|e| match e {
        Event::Fired { by: C1, shots, .. } => Some(*shots),
        _ => None,
    })
}

/// Per shot, whom the missile reached, in order: (who, dodged).
fn reached(ev: &[Event]) -> Vec<Vec<(Who, bool)>> {
    let mut shots: Vec<Vec<(Who, bool)>> = Vec::new();
    for e in ev {
        match e {
            Event::Shot { .. } => shots.push(Vec::new()),
            Event::Dodged { who, .. } => shots.last_mut().expect("a shot").push((*who, true)),
            Event::MissileHit { target, .. } => shots.last_mut().expect("a shot").push((*target, false)),
            _ => {}
        }
    }
    shots
}

fn missile_gains(ev: &[Event]) -> u8 {
    ev.iter()
        .filter(|e| {
            matches!(
                e,
                Event::StatChanged {
                    who: C1,
                    stat: Stat::Missile,
                    ..
                }
            )
        })
        .count() as u8
}

#[test]
fn an_archer_fires_its_shots_instead_of_stepping_with_the_captured_spreads() {
    both(|p| {
        for seed in 0..20 {
            let (_, ev) = shoot(archer(p, true, 3), seed);
            // 8.2: 13 / 8 + 3 / 2 = 2 shots; the turn ends without a step.
            assert_eq!(fired(&ev), Some(2), "seed {seed}: {ev:?}");
            assert!(!ev
                .iter()
                .any(|e| matches!(e, Event::CreatureMoved { slot: 1, .. })));
            // 8.4: 60, then 55 less what Missile gained on the first shot.
            let spreads: Vec<u8> = ev
                .iter()
                .filter_map(|e| match e {
                    Event::Shot { by: C1, spread, .. } => Some(*spread),
                    _ => None,
                })
                .collect();
            let second = ev
                .iter()
                .rposition(|e| matches!(e, Event::Shot { .. }))
                .expect("a shot");
            assert_eq!(spreads, [60, 55 - missile_gains(&ev[..second])], "seed {seed}");
        }
    });
}

#[test]
fn an_archer_readies_its_bow_first_and_shoots_from_the_moves_left() {
    both(|p| {
        let (g, ev) = shoot(archer(p, false, 6), 1);
        // 6.2: readied (a move), then 13 / 8 + 5 / 2 = 3 shots.
        let readied = ev
            .iter()
            .position(|e| {
                *e == Event::CreatureReadied {
                    region: 1,
                    slot: 1,
                    thing: BOW,
                }
            })
            .expect("readies the bow");
        let at = ev
            .iter()
            .position(|e| matches!(e, Event::Fired { .. }))
            .expect("fires");
        assert!(readied < at);
        assert_eq!(fired(&ev), Some(3));
        assert_eq!(g.creature(1, 1).map(|c| c.weapon), Some(BOW));
    });
}

#[test]
fn a_friend_in_the_line_is_reached_first_and_a_dodge_lets_the_arrow_fly_on() {
    both(|p| {
        let (mut friend_hit, mut flew_on) = (false, false);
        for seed in 0..300 {
            let mut a = archer(p, true, 3);
            // A peaceful friend of Speed 0 on (5,3), between the two.
            let mut s = spec(0, 5, 3, Alignment::Friend, false);
            s.speed = 0;
            put(&mut a, 2, s);
            let f = &mut a.regions[0].residents[2];
            f.constitution = 63;
            f.life_force = 63;
            f.dodge = if seed % 2 == 0 { 0 } else { 127 };
            let (_, ev) = shoot(a, seed);
            assert_eq!(fired(&ev), Some(2), "seed {seed}");
            for shot in reached(&ev) {
                // 8.5: in path order, the friend before the player.
                if let Some(i) = shot.iter().position(|r| r.0 == ME) {
                    assert!(
                        shot[..i].iter().all(|r| r.0 == C2 && r.1),
                        "seed {seed}: {shot:?}"
                    );
                    flew_on |= i > 0;
                }
                // 8.6: a hit stops the missile.
                if let Some(i) = shot.iter().position(|r| !r.1) {
                    assert_eq!(i, shot.len() - 1, "seed {seed}: {shot:?}");
                    friend_hit |= shot[i].0 == C2;
                }
            }
        }
        assert!(friend_hit && flew_on);
    });
}

/// The archer's turn began with a step; if it fired, after that step.
fn steps_before_firing(ev: &[Event], seed: u64) {
    let turn = ev
        .iter()
        .position(|e| matches!(e, Event::CreatureTurn { slot: 1, .. }))
        .expect("its turn");
    let after = &ev[turn..];
    let moved = after
        .iter()
        .position(|e| matches!(e, Event::CreatureMoved { slot: 1, .. }));
    let shot = after
        .iter()
        .position(|e| matches!(e, Event::Fired { by: C1, .. }));
    assert!(moved.is_some(), "seed {seed}");
    assert!(shot.is_none_or(|s| moved < Some(s)), "seed {seed}");
}

#[test]
fn no_firing_with_a_foe_beside_a_wall_in_the_line_or_out_of_range() {
    both(|p| {
        for seed in 0..10 {
            // The player beside it, west: holding only the bow it can't
            // strike, and it doesn't fire from there. Its moves are halved
            // (3 to 2); it steps off to a square not beside the player and
            // fires with the move left: 1 shot (combat 8.1, pass 3).
            let mut a = archer(p, true, 3);
            a.regions[0].residents[1].x = 3;
            let (_, ev) = shoot(a, seed);
            assert!(ev.contains(&Event::CreatureTurn {
                region: 1,
                slot: 1,
                moves: 2
            }));
            let moved = ev
                .iter()
                .position(|e| matches!(e, Event::CreatureMoved { slot: 1, .. }));
            let shot = ev.iter().position(|e| matches!(e, Event::Fired { by: C1, .. }));
            assert!(moved.is_some() && moved < shot, "seed {seed}");
            assert_eq!(fired(&ev), Some(1), "seed {seed}");
            assert!(!ev.iter().any(|e| matches!(e, Event::Attacked { .. })));
            // With a single move (Speed 1, halved rounding up to 1) it steps
            // and has none left to fire with.
            let mut a = archer(p, true, 1);
            a.regions[0].residents[1].x = 3;
            let (_, ev) = shoot(a, seed);
            assert_eq!(fired(&ev), None, "seed {seed}");
            // A pillar in the line, or 5 squares on a bow of range 4: no
            // shot from where it stands, no message; it walks, asking again
            // before every step (bow-wall, pass 3).
            let mut a = archer(p, true, 3);
            item(&mut a, 0, 5, 3, PILLAR, 0);
            let (_, ev) = shoot(a, seed);
            steps_before_firing(&ev, seed);
            let mut a = archer(p, true, 3);
            if let ThingKind::MissileWeapon { stats, .. } = &mut a.things[BOW as usize - 1].kind {
                stats.range = Some(4);
            }
            let (_, ev) = shoot(a, seed);
            steps_before_firing(&ev, seed);
        }
    });
}

#[test]
fn a_bow_that_disappears_goes_after_one_shot() {
    both(|p| {
        let mut a = archer(p, true, 3);
        if let ThingKind::MissileWeapon {
            disappears_after_use, ..
        } = &mut a.things[BOW as usize - 1].kind
        {
            *disappears_after_use = true;
        }
        let (g, ev) = shoot(a, 2);
        assert_eq!(fired(&ev), Some(2));
        let shots = ev.iter().filter(|e| matches!(e, Event::Shot { .. })).count();
        assert_eq!(shots, 1);
        assert!(ev.contains(&Event::Broke {
            who: C1,
            thing: BOW,
            used_up: true
        }));
        assert_eq!(g.creature(1, 1).map(|c| c.weapon), Some(0));
    });
}

#[test]
fn a_player_cannot_fire() {
    both(|p| {
        let mut g = duel(p, 1, PlayerSetup::default(), |_| {});
        let ev = g.apply(Command::Fire);
        assert_eq!(ev, [Event::CannotFire { player: 0 }]);
        assert_eq!(g.current_player().moves_left, 15);
    });
}
