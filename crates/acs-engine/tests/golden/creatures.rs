// SPDX-License-Identifier: GPL-3.0-or-later

//! Creatures and turn order (creatures 2-3, magic 5): who moves when, the
//! visits, doors and the moves they carry, the adjacency slowdown,
//! following through a door, random creatures appearing and falling away,
//! the catch-up, Power drift and the idle healing roll's range; a creature's
//! take, its bar against its load and the turn-top die, fighter or slinker
//! afresh each turn, and the moves it carries through a door.
//!
//! The capture's scenarios differ from [`base`] in their room: no door in
//! the top wall (the `one` room has no way out) and, with `--side-room`, a
//! one-way portal on ROOM (7,4) to SIDE (2,4). [`closed`] and
//! [`with_portal`] rebuild that. Their creatures of ours default to a
//! peaceful, cautious friend of Speed 3, Constitution and Life Force 10
//! (capture.md 10.10), [`ours`].

use crate::stats::{same_histogram, same_rate, Sweep, SEEDS, Z_FAIL};
use crate::world::*;
use acs_core::model::{
    Access, Alignment, Creature, CustomTrigger, Destination, PlacementChoice, RoomItem, SpellAction,
    SpellActionKind, ThingKind,
};
use acs_core::{Adventure, Platform};
use acs_engine::{Command, Dir, Event, Game, Location, PlayerSetup, RemovalReason, Rules, Stat};

/// A two-way portal (10.29 `two`).
const TWO_WAY: u16 = 83;
const SPELL: u16 = 33;
const GOLD: u16 = 81;
/// The second region of 10.29 `reg`.
const FAR: u8 = 2;

fn side(x: u8, y: u8) -> Location {
    Location::Room {
        region: 1,
        room: SIDE,
        x,
        y,
    }
}

fn to_side() -> Destination {
    Destination::Room {
        region: 1,
        room: SIDE,
        x: 2,
        y: 4,
    }
}

/// The capture's room: base's top-wall door taken out.
fn closed() -> Adventure {
    let mut a = base(Platform::C64);
    a.regions[0].rooms[ROOM as usize]
        .items
        .retain(|i| i.thing != DOOR);
    a
}

/// A door `thing` on ROOM (x, y) leading to `to`.
fn door_at(a: &mut Adventure, x: u8, y: u8, thing: u16, to: Destination) {
    a.regions[0].rooms[ROOM as usize].items.insert(
        0,
        RoomItem {
            x,
            y,
            thing,
            parameter: 0,
            flag: false,
            destination: Some(to),
            gold: None,
        },
    );
}

/// `--side-room`: the portal `door` on ROOM (7,4) to `to`.
fn with_portal(door: u16, to: Destination) -> Adventure {
    let mut a = closed();
    if door == TWO_WAY {
        thing(
            &mut a,
            TWO_WAY,
            "DOOR2",
            ThingKind::Portal {
                picture: 5,
                two_way: true,
                access: Access::Anyone,
                destroy_needed_thing: false,
            },
        );
    }
    door_at(&mut a, 7, 4, door, to);
    a
}

/// `--side-door region`: region 2, its room 0 a 10 x 8 FAR ROOM.
fn with_far(a: &mut Adventure) {
    let mut far = a.regions[0].clone();
    far.number = FAR;
    far.name = "FAR".into();
    for c in &mut far.residents {
        c.in_use = false;
    }
    let mut room0 = far.rooms[SIDE as usize].clone();
    room0.number = 0;
    room0.name = "FAR ROOM".into();
    far.rooms[0] = room0;
    a.regions.push(far);
}

/// A creature of ours (`--creature`): a peaceful, cautious friend of Speed
/// 3, Constitution and Life Force 10, in region 1's `slot` on ROOM (x, y),
/// then changed by `f`.
fn ours(a: &mut Adventure, slot: usize, name: &str, x: u8, y: u8, f: impl FnOnce(&mut Creature)) {
    creature(a, slot, x, y, |c| {
        c.name = name.into();
        c.speed = 3;
        c.constitution = 10;
        c.life_force = 10;
        c.strategy.alignment = Alignment::Friend;
        c.strategy.aggressive = false;
        c.strategy.brave = false;
        f(c);
    });
}

/// T of the turn-order disks: Constitution and Life Force 40.
fn tee(speed: u8) -> PlayerSetup {
    PlayerSetup {
        name: "T".into(),
        speed,
        strength: 40,
        constitution: 40,
        life_force: 40,
        ..Default::default()
    }
}

/// The second player U.
fn you(speed: u8) -> Command {
    Command::AddPlayer {
        setup: PlayerSetup {
            name: "U".into(),
            speed,
            ..Default::default()
        },
    }
}

/// A game starting on the map's start (12,12), as the capture's players do.
fn start(a: &Adventure, seed: u64, player: PlayerSetup) -> Game {
    Game::new(a.clone(), seed, Rules::default(), player).0
}

/// The current player walks from (12,12) south through the map door into
/// ROOM (4,4): 3 squares and the door's step.
fn walk_in(g: &mut Game) -> Vec<Event> {
    let mut ev = Vec::new();
    for _ in 0..4 {
        ev.extend(g.apply(Command::Move { dir: Dir::S }));
    }
    assert!(
        ev.iter().any(|e| matches!(
            e,
            Event::EnteredRoom {
                region: 1,
                room: ROOM,
                ..
            }
        )),
        "{ev:?}"
    );
    ev
}

fn east(g: &mut Game, n: usize) -> Vec<Event> {
    let mut ev = Vec::new();
    for _ in 0..n {
        ev.extend(g.apply(Command::Move { dir: Dir::E }));
    }
    ev
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mover {
    /// A player's turn and the moves it started with.
    P(usize, u8),
    /// A region 1 creature's turn.
    C(usize),
}
use Mover::{C, P};

fn movers(ev: &[Event]) -> Vec<Mover> {
    ev.iter()
        .filter_map(|e| match *e {
            Event::TurnStarted { player, moves, .. } => Some(P(player, moves)),
            Event::CreatureTurn { region: 1, slot, .. } => Some(C(slot)),
            _ => None,
        })
        .collect()
}

/// The last turn start: player, moves, hindered.
fn started(ev: &[Event]) -> (usize, u8, bool) {
    ev.iter()
        .rev()
        .find_map(|e| match *e {
            Event::TurnStarted {
                player,
                moves,
                hindered,
            } => Some((player, moves, hindered)),
            _ => None,
        })
        .expect("a turn started")
}

fn took_door(ev: &[Event], to: Location) -> bool {
    ev.iter()
        .any(|e| matches!(e, Event::TookDoor { to: t, .. } if *t == to))
}

/// The heals in `n` idle moves that don't end the turn.
fn idle_heals(g: &mut Game, n: usize) -> u64 {
    let mut healed = 0;
    for _ in 0..n {
        let before = life(g);
        let ev = g.apply(Command::Idle);
        assert!(!ev.iter().any(|e| matches!(e, Event::TurnEnded { .. })), "{ev:?}");
        healed += u64::from(life(g) > before);
    }
    healed
}

/// The engine's heal rate per idle move is 1 in `range` and none of
/// `others` (capture.md 10.29's fitted ranges: the count of heals is not
/// in the doc, only the range that fits every move).
fn heal_range(what: &str, healed: u64, moves: u64, range: u32, others: &[u32]) {
    let eng = healed as f64 / moves as f64;
    let se = |p: f64| (p * (1.0 - p) / moves as f64).sqrt();
    let p = 1.0 / f64::from(range);
    let line = format!(
        "{what}: engine {healed}/{moves} ({:.2}%), range {range} gives {:.2}%",
        100.0 * eng,
        100.0 * p
    );
    println!("{line}");
    assert!((eng - p).abs() <= Z_FAIL * se(p), "{line}");
    for &o in others {
        let q = 1.0 / f64::from(o);
        assert!((eng - q).abs() > Z_FAIL * se(q), "{line}: range {o} fits too");
    }
}

// ---- turn order (creatures 2.2) ---------------------------------------------

/// `one`'s creatures: AA (12,2, Speed 3), BB (2,7, Speed 5), CC (12,7, Speed
/// 4, a neutral); `first` is the name in slot 0 (on 12,2), the other in
/// slot 1 (on 2,7).
fn three(first: &str) -> Adventure {
    let mut a = closed();
    let speed = |n: &str| if n == "AA" { 3 } else { 5 };
    let second = if first == "AA" { "BB" } else { "AA" };
    ours(&mut a, 0, first, 12, 2, |c| c.speed = speed(first));
    ours(&mut a, 1, second, 2, 7, |c| c.speed = speed(second));
    ours(&mut a, 2, "CC", 12, 7, |c| {
        c.speed = 4;
        c.strategy.alignment = Alignment::Neutral;
    });
    a
}

/// capture.md 7 / 10.10.1 (`one`, 6 runs; `slots`, 3): T (Speed 6) walks
/// into the room with AA, BB, CC in slots 0-2; nobody moves while T is on
/// the map; after each of T's turns AA, BB, CC, then T. With BB in slot 0
/// and AA in slot 1, BB, AA, CC: slot order, not name or square.
#[test]
fn turn_order_one_creature_phase_in_slot_order() {
    for (first, want) in [("AA", ["AA", "BB", "CC"]), ("BB", ["BB", "AA", "CC"])] {
        let a = three(first);
        let mut g = start(&a, 1, tee(6));
        let ev = walk_in(&mut g);
        assert_eq!(
            movers(&ev),
            [P(0, 2)],
            "{first}: nobody else moves while T comes in"
        );
        for _ in 0..6 {
            let ev = wait(&mut g);
            assert_eq!(movers(&ev), [C(0), C(1), C(2), P(0, 6)], "{first}: {ev:?}");
        }
        let names: Vec<_> = (0..3).map(|s| g.creature(1, s).unwrap().name.clone()).collect();
        assert_eq!(names, want);
    }
}

/// capture.md 7 / 10.10.1 (`two`, 7 runs): T and U (Speed 4), U added on
/// T's first turn, both walk in on their first turns: T, U, then AA, BB,
/// CC once, then T; none after T's turn alone; no creature phase until
/// both had ended a turn in the room; U on the entry square with T.
#[test]
fn turn_order_two_players_then_the_creatures() {
    let a = three("AA");
    let mut g = start(&a, 1, tee(6));
    g.apply(you(4));
    let mut log = movers(&walk_in(&mut g));
    log.extend(movers(&walk_in(&mut g)));
    assert_eq!(g.players()[1].at, room(4, 4));
    assert_eq!(g.players()[0].at, room(4, 4));
    for _ in 0..8 {
        log.extend(movers(&wait(&mut g)));
    }
    // U's map turn after T's; T's first room turn with the 2 moves it
    // kept, U's with all 4 (it took the door with its last move).
    let mut want = vec![P(1, 4), P(0, 2)];
    for _ in 0..4 {
        want.extend([P(1, 4), C(0), C(1), C(2), P(0, 6)]);
    }
    assert_eq!(log, want);
}

/// capture.md 10.13 (the split case confirmed, 2 runs to 25 of T's turns;
/// 7 / 10.10.1 `split`): T in the room with AA, BB, CC, U left on the map.
/// U alone in rounds 21 to 39, T, AA, BB, CC in 40 to 49, U in 50 to 59,
/// the room from 60: the rest of the map's visit and a whole one, then
/// blocks of exactly 10 rounds (T walked in during round 20, the capture's
/// counter; the engine's game starts at 0, so 0, 1-19, 20-29 ...).
#[test]
fn split_visits_of_ten_rounds() {
    let a = three("AA");
    let mut g = start(&a, 1, tee(6));
    g.apply(you(4));
    walk_in(&mut g);
    let mut seen = vec![(g.round(), g.current())];
    for _ in 0..58 {
        let me = g.current();
        let ev = wait(&mut g);
        let phase: Vec<_> = movers(&ev).into_iter().filter(|m| matches!(m, C(_))).collect();
        if me == 0 {
            assert_eq!(phase, [C(0), C(1), C(2)], "round {}", g.round());
        } else {
            assert!(phase.is_empty(), "round {}", g.round());
        }
        seen.push((g.round(), g.current()));
    }
    let mut want = Vec::new();
    for (rounds, who) in [
        (1..=19, 1),
        (20..=29, 0),
        (30..=39, 1),
        (40..=49, 0),
        (50..=59, 1),
    ] {
        want.extend(rounds.map(|r| (r, who)));
    }
    assert_eq!(seen, want);
}

/// `rooms2`'s disk (capture.md 10.10.4, 10.22): the side portal and two
/// peaceful friends of Speed 3 on 2,7 and 2,2, west of the entry.
fn rooms2() -> Adventure {
    let mut a = with_portal(DOOR, to_side());
    ours(&mut a, 0, "AA", 2, 7, |_| {});
    ours(&mut a, 1, "BB", 2, 2, |_| {});
    a
}

/// Each command's turns, with the round after it.
fn play(g: &mut Game, cmds: usize) -> Vec<(u32, Vec<Mover>)> {
    (0..cmds)
        .map(|_| {
            let ev = wait(g);
            (g.round(), movers(&ev))
        })
        .collect()
}

/// The room region's rounds from `first` on, `n` of them: T, U (4), AA,
/// BB, as `play` reads them from T's (first) turn on.
fn room_rounds(first: u32, n: u32) -> Vec<(u32, Vec<Mover>)> {
    let mut want = Vec::new();
    for r in first..first + n {
        want.push((r, vec![P(1, 4)]));
        want.push((r + 1, vec![C(0), C(1), P(0, 12)]));
    }
    want
}

/// capture.md 10.22 item 1 (`held`, `tfirst`, `ulater`, one run each): T
/// (Speed 12) and U (Speed 4) come in through the map door. `held`: U in
/// round 11, T in round 12 after 4 of its 12 squares; the room region from
/// round 13 with T first, then U, AA, BB, and so in rounds 14 and 15.
/// `tfirst`: both in during round 12; T, U, AA, BB from 13 to 17.
/// `ulater`: U in round 11, T alone on the map in rounds 12 to 14 (12 the
/// visit's last, 13 and 14 a new cycle's map visit) and in during 14; T,
/// U, AA, BB from 15 to 18, U with no turn in 12 to 14. No round missed.
/// T's first room turn had the moves left at the door: 8, 8 and 1; U's its
/// full 4. The engine's game starts at round 0, so the rounds are shifted.
#[test]
fn players_come_into_the_room_region_by_number() {
    let a = rooms2();
    // held: U in during round 1, T during round 2.
    let mut g = start(&a, 1, tee(12));
    g.apply(you(4));
    wait(&mut g);
    wait(&mut g);
    assert_eq!((g.round(), g.current()), (1, 1));
    walk_in(&mut g);
    assert_eq!((g.round(), g.current()), (2, 0));
    let ev = walk_in(&mut g);
    assert_eq!(movers(&ev), [P(0, 8)]);
    assert_eq!(g.round(), 3);
    let rooms2 = g.clone();
    assert_eq!(play(&mut g, 6), room_rounds(3, 3), "held");

    // tfirst: both in during round 1.
    let mut g = start(&a, 1, tee(12));
    g.apply(you(4));
    wait(&mut g);
    assert_eq!((g.round(), g.current()), (1, 0));
    walk_in(&mut g);
    let ev = walk_in(&mut g);
    assert_eq!(movers(&ev), [P(0, 8)]);
    assert_eq!(g.round(), 2);
    assert_eq!(play(&mut g, 10), room_rounds(2, 5), "tfirst");

    // ulater: U in during round 8, the map visit's ninth; T alone in round
    // 9 and in a new cycle's rounds 10 and 11, in during 11 with 1 move.
    let mut g = start(&a, 1, tee(12));
    g.apply(you(4));
    wait(&mut g);
    for _ in 1..8 {
        wait(&mut g);
        wait(&mut g);
    }
    wait(&mut g);
    assert_eq!((g.round(), g.current()), (8, 1));
    walk_in(&mut g);
    for r in 9..=11 {
        assert_eq!((g.round(), g.current()), (r, 0), "T alone");
        if r < 11 {
            assert_eq!(movers(&wait(&mut g)).len(), 1);
        }
    }
    for _ in 0..7 {
        g.apply(Command::Idle);
    }
    let ev = walk_in(&mut g);
    assert_eq!(movers(&ev), [P(0, 1)]);
    assert_eq!(g.round(), 12);
    assert_eq!(play(&mut g, 8), room_rounds(12, 4), "ulater");

    // capture.md 10.10.4 (`rooms2`, 3 runs, 8 of T's turns): from held's
    // first room round T goes on through the portal (`e3`, 5 moves left);
    // from the next round every round went T (side room), then U, AA, BB
    // (room 0). The friends wander round U and may step onto the portal
    // (the capture's paths are jitter runs): the list is T, the side
    // room's creatures, U, room 0's, by where they stand at the round's
    // start; how often both stayed in room 0 is a sample.
    let mut g = rooms2;
    let ev = east(&mut g, 3);
    assert!(took_door(&ev, side(2, 4)), "{ev:?}");
    assert_eq!(movers(&ev), [P(1, 4)]);
    let ev = wait(&mut g);
    assert_eq!(movers(&ev), [C(0), C(1), P(0, 5)]);
    assert_eq!(g.players()[0].at, side(2, 4));
    let stayed = count(256, |seed| {
        let mut g = rooms2_from_side(&a, seed);
        let mut stayed = true;
        for _ in 0..8 {
            let in_side: Vec<_> = (0..2)
                .filter(|&s| g.creature(1, s).unwrap().room == SIDE)
                .collect();
            stayed &= in_side.is_empty();
            let mut want: Vec<_> = in_side.iter().map(|&s| C(s)).collect();
            want.push(P(1, 4));
            assert_eq!(movers(&wait(&mut g)), want, "seed {seed}");
            let mut want: Vec<_> = (0..2).filter(|s| !in_side.contains(s)).map(C).collect();
            want.push(P(0, 12));
            assert_eq!(movers(&wait(&mut g)), want, "seed {seed}");
        }
        stayed
    });
    same_rate(
        "rooms2: both friends stay in room 0",
        Sweep::Sample,
        3,
        3,
        stayed,
        256,
    );
}

/// `held` up to T's first turn in the side room, at `seed`.
fn rooms2_from_side(a: &Adventure, seed: u64) -> Game {
    let mut g = start(a, seed, tee(12));
    g.apply(you(4));
    wait(&mut g);
    wait(&mut g);
    walk_in(&mut g);
    walk_in(&mut g);
    east(&mut g, 3);
    wait(&mut g);
    assert_eq!(g.players()[0].at, side(2, 4));
    g
}

// ---- doors and the moves they carry (creatures 2.2.4, 2.6) ------------------

/// capture.md 10.29 items 1-2 (`one`, `two`, `reg`; one run each): T alone,
/// Speed 12, Constitution 40, Life Force 10, on its first room turn with 8
/// moves (`x1`) walks 3 squares east onto the portal with 5 left: the turn
/// ends, the counter goes up one (8 to 9), and the next turn begins on the
/// destination 2,4 with 5 moves, the one after with 12. With 3 moves
/// (`x0`) the portal takes the last and the next turn has 12. One-way and
/// two-way alike; into region 2's FAR ROOM the same, the first turn there
/// the very next round.
#[test]
fn a_room_door_ends_the_turn_and_carries_the_moves() {
    let far = Destination::Room {
        region: FAR,
        room: 0,
        x: 2,
        y: 4,
    };
    let far_at = Location::Room {
        region: FAR,
        room: 0,
        x: 2,
        y: 4,
    };
    for (door, to, at, label) in [
        (DOOR, to_side(), side(2, 4), "one"),
        (TWO_WAY, to_side(), side(2, 4), "two"),
        (DOOR, far, far_at, "reg"),
    ] {
        let mut a = with_portal(door, to);
        with_far(&mut a);
        for (idle, carried, left) in [(0, 8, 5), (5, 3, 12)] {
            let mut g = start(
                &a,
                1,
                PlayerSetup {
                    life_force: 10,
                    ..tee(12)
                },
            );
            for _ in 0..idle {
                g.apply(Command::Idle);
            }
            assert_eq!(started(&walk_in(&mut g)), (0, carried, false), "{label}");
            let round = g.round();
            let ev = east(&mut g, 3);
            assert!(took_door(&ev, at), "{label}: {ev:?}");
            assert_eq!(g.round(), round + 1, "{label}");
            assert_eq!(started(&ev), (0, left, false), "{label} from {carried}");
            assert_eq!(g.players()[0].at, at);
            assert_eq!(g.visiting(), at.region());
            assert_eq!(started(&wait(&mut g)).1, 12, "{label}");
        }
    }
}

/// capture.md 10.29 item 1 (`reg2`, one run): T (Speed 12) in with 8
/// moves, U (Speed 4) with all 4, both in region 1 from round 13; T
/// through the portal into region 2 with 5 left at once. U alone in region
/// 1 for rounds 13 to 32, T in region 2 for 33 to 42 (its first turn there
/// with the 5 moves), U in region 1 from 43: the carried moves wait for the
/// next cycle. Engine rounds 2-21, 22-31, 32.
#[test]
fn carried_moves_wait_for_the_next_cycle() {
    let mut a = with_portal(
        DOOR,
        Destination::Room {
            region: FAR,
            room: 0,
            x: 2,
            y: 4,
        },
    );
    with_far(&mut a);
    let mut g = start(&a, 1, tee(12));
    g.apply(you(4));
    wait(&mut g);
    walk_in(&mut g);
    walk_in(&mut g);
    assert_eq!((g.round(), g.current()), (2, 0));
    let ev = east(&mut g, 3);
    assert!(ev.iter().any(|e| matches!(
        e,
        Event::EnteredRegion {
            player: 0,
            region: FAR
        }
    )));
    let mut seen = vec![(g.round(), g.current(), g.visiting())];
    let mut first_far = None;
    for _ in 0..30 {
        let ev = wait(&mut g);
        let (p, moves, _) = started(&ev);
        if p == 0 && first_far.is_none() {
            first_far = Some(moves);
        }
        seen.push((g.round(), g.current(), g.visiting()));
    }
    let mut want: Vec<_> = (2..=21).map(|r| (r, 1, 1)).collect();
    want.extend((22..=31).map(|r| (r, 0, FAR)));
    want.push((32, 1, 1));
    assert_eq!(seen, want);
    assert_eq!(first_far, Some(5));
}

/// capture.md 10.29 item 3 (`adj`, `diag`; one run each): EE, an
/// aggressive enemy of Speed 0 and Strength 1, on 5,4 east of the entry
/// (`diag`: 5,5). T (Speed 12) through the map door: its first room turn
/// began with 8 of 8 carried (`adj r8`), 7 of 7 (`adj r7`, one idle move on
/// the map first), 8 (`diag r8`), none halved; with none left at the door
/// (`adj r0`) with 6, the full 12 halved; later turns beside EE 6 each.
#[test]
fn carried_moves_beside_an_enemy_are_not_halved() {
    for (x, y, idle, first, hindered, later) in [
        (5, 4, 0, 8, false, 6),
        (5, 4, 1, 7, false, 6),
        (5, 5, 0, 8, false, 12),
        (5, 4, 8, 6, true, 6),
    ] {
        let mut a = closed();
        creature(&mut a, 0, x, y, |c| {
            c.name = "EE".into();
            c.speed = 0;
            c.strength = 1;
        });
        let mut g = start(
            &a,
            1,
            PlayerSetup {
                life_force: 10,
                ..tee(12)
            },
        );
        for _ in 0..idle {
            g.apply(Command::Idle);
        }
        let case = format!("EE on {x},{y}, {idle} idle");
        assert_eq!(started(&walk_in(&mut g)), (0, first, hindered), "{case}");
        for _ in 0..3 {
            assert_eq!(started(&wait(&mut g)), (0, later, later == 6), "{case}");
        }
    }
}

// ---- the adjacency slowdown (creatures 2.6) ---------------------------------

/// capture.md 7 / 10.10.2 (the `adj-` cases, 2 runs each): T (Speed 12,
/// 96 px), Constitution and Life Force 40, walks in; one creature of
/// Speed 0, Strength 1 near the entry 4,4. T's bar at the top of its
/// turns in the room: 48 px (6) with an aggressive enemy or an aggressive
/// neutral on 5,4; 96 (12) with it on 5,5 or 6,4, with an aggressive
/// friend or a peaceful enemy on 5,4.
#[test]
fn adjacency_slowdown_table() {
    use Alignment::*;
    for (label, x, y, alignment, aggressive, moves) in [
        ("adj-enemy", 5, 4, Enemy, true, 6),
        ("adj-neutral", 5, 4, Neutral, true, 6),
        ("adj-diag", 5, 5, Enemy, true, 12),
        ("adj-far", 6, 4, Enemy, true, 12),
        ("adj-friend", 5, 4, Friend, true, 12),
        ("adj-peaceful", 5, 4, Enemy, false, 12),
    ] {
        let mut a = closed();
        creature(&mut a, 0, x, y, |c| {
            c.speed = 0;
            c.strength = 1;
            c.strategy.alignment = alignment;
            c.strategy.aggressive = aggressive;
        });
        let mut g = start(&a, 1, tee(12));
        walk_in(&mut g);
        for _ in 0..3 {
            assert_eq!(started(&wait(&mut g)), (0, moves, moves == 6), "{label}");
        }
    }
}

/// capture.md 10.16 (every pass 2 and 3 combat run; creatures 2.6): moves
/// halved, rounding up, at the top of a turn beside a foe. An enemy of
/// Speed 3 had 2 moves on every turn it began beside the player, 3
/// otherwise, and the player 3 of 6 beside it. Here EE (aggressive, brave,
/// unarmed, so it keeps stepping round T, capture.md 10.14.6) and T (Speed
/// 6) on ROOM (4,4).
#[test]
fn moves_halved_rounding_up_beside_a_foe() {
    let mut a = closed();
    creature(&mut a, 0, 6, 4, |c| {
        c.name = "EE".into();
        c.speed = 3;
        c.strength = 10;
    });
    let beside = |g: &Game| {
        let c = g.creature(1, 0).expect("EE");
        (i16::from(c.x) - 4).abs() + (i16::from(c.y) - 4).abs() == 1
    };
    let mut seen = [[0; 2]; 2];
    for seed in 0..16 {
        let mut g = game(&a, seed, tee(6), room(4, 4));
        for _ in 0..6 {
            let was = beside(&g);
            let ev = wait(&mut g);
            let moves = ev
                .iter()
                .find_map(|e| match *e {
                    Event::CreatureTurn { slot: 0, moves, .. } => Some(moves),
                    _ => None,
                })
                .expect("EE's turn");
            assert_eq!(moves, if was { 2 } else { 3 }, "seed {seed}");
            let now = beside(&g);
            assert_eq!(started(&ev), (0, if now { 3 } else { 6 }, now), "seed {seed}");
            seen[0][was as usize] += 1;
            seen[1][now as usize] += 1;
        }
    }
    assert!(seen.iter().flatten().all(|&n| n > 0), "{seen:?}");
}

// ---- following through a door (creatures 5.4, 6.4) --------------------------

/// The `follow` disk: AA a peaceful friend of Speed 6 on 10,2, CC a
/// peaceful neutral of Speed 4 on 12,7; `held` adds SS (Speed 0) on the
/// side room's 2,4.
fn follow(held: bool) -> Adventure {
    let mut a = with_portal(DOOR, to_side());
    ours(&mut a, 0, "AA", 10, 2, |c| c.speed = 6);
    ours(&mut a, 1, "CC", 12, 7, |c| {
        c.speed = 4;
        c.strategy.alignment = Alignment::Neutral;
    });
    if held {
        ours(&mut a, 2, "SS", 2, 4, |c| {
            c.speed = 0;
            c.room = SIDE;
        });
    }
    a
}

/// One `route` run: T (Speed 12) in, `e3` onto the portal, then idling, to
/// `turns` of T's turns in all. Whether AA followed through the portal.
fn route(a: &Adventure, seed: u64, held: bool, turns: usize) -> bool {
    let mut g = start(a, seed, tee(12));
    walk_in(&mut g);
    let mut ev = east(&mut g, 3);
    // T's turn ended on the portal's step with 5 left; its next turn is in
    // the side room on 2,4, with SS there in `held`.
    assert_eq!(started(&ev), (0, 5, false), "seed {seed}");
    assert_eq!(g.players()[0].at, side(2, 4));
    if held {
        let ss = g.creature(1, 2).expect("SS");
        assert_eq!((ss.room, ss.x, ss.y), (SIDE, 2, 4));
    }
    for _ in 1..turns {
        ev.extend(wait(&mut g));
    }
    let mut landed = None;
    for (i, e) in ev.iter().enumerate() {
        match *e {
            Event::CreatureMoved { slot, from, to, .. } => {
                let (a, b) = (from.xy(), to.xy());
                let d = (i16::from(a.0) - i16::from(b.0)).abs() + (i16::from(a.1) - i16::from(b.1)).abs();
                assert_eq!(d, 1, "seed {seed}: slot {slot} stepped diagonally");
            }
            // A creature arriving lands on the destination, over T.
            Event::CreatureTookDoor { slot: 0, to, .. } if to.room() == Some((1, SIDE)) => {
                assert_eq!(to, side(2, 4), "seed {seed}");
                landed = Some(i);
            }
            _ => {}
        }
    }
    // On its next turn it steps off (capture: 2 of 2, and every `held`
    // follower).
    if let Some(i) = landed {
        let mut next = ev[i + 1..]
            .iter()
            .skip_while(|e| !matches!(e, Event::CreatureTurn { slot: 0, .. }))
            .skip(1)
            .take_while(|e| !matches!(e, Event::CreatureTurn { .. } | Event::TurnStarted { .. }));
        if ev[i + 1..]
            .iter()
            .any(|e| matches!(e, Event::CreatureTurn { slot: 0, .. }))
        {
            assert!(
                next.any(|e| matches!(e, Event::CreatureMoved { slot: 0, from, .. } if *from == side(2, 4))),
                "seed {seed}: AA stayed on the destination"
            );
        }
    }
    landed.is_some()
}

/// capture.md 7 / 10.10.3 (`route`, 3 runs, 4 of T's turns): T's turn
/// ended on the portal step with 5 steps left, and T showed in SIDE ROOM
/// at its next turn, 3 of 3; AA went onto the portal in 2 of the 3 route
/// runs and showed on the portal's destination over T; creatures never
/// stepped diagonally. capture.md 10.10.5 (`held`, 6 wide runs, 5 of T's
/// turns): with SS on the destination, T landed on 2,4 with SS in 6 of 6;
/// AA followed in 5 and landed on 2,4 too. Jitter runs, so samples.
#[test]
fn following_through_a_door() {
    for (held, turns, cap) in [(false, 4, 2), (true, 5, 5)] {
        let a = follow(held);
        let n = 256;
        let eng = count(n, |seed| route(&a, seed, held, turns));
        let label = if held { "held" } else { "route" };
        same_rate(
            &format!("AA follows T through the portal ({label})"),
            Sweep::Sample,
            cap,
            if held { 6 } else { 3 },
            eng,
            n,
        );
    }
}

// ---- random creatures (creatures 2.4, 3) ------------------------------------

/// The random creature 0 (Speed 0) of region 1, room `room` at `chance` %.
fn random_creature(a: &mut Adventure, room: u8, chance: u8) {
    let mut rc = a.regions[0].residents[0].clone();
    rc.in_use = true;
    rc.name = "RANDOM".into();
    rc.speed = 0;
    a.regions[0].random_creatures[0] = rc;
    let r = &mut a.regions[0].rooms[room as usize];
    r.random_chance = chance;
    r.random_creature = 0;
}

fn appeared(ev: &[Event], room: u8) -> Option<usize> {
    ev.iter().find_map(|e| match *e {
        Event::RoomEncounter {
            region: 1,
            room: r,
            slot,
            ..
        } if r == room => slot,
        _ => None,
    })
}

/// capture.md 10.14.2 (`turns` with the slot peeked, 10 runs each): at
/// chance 15 (75%) a random creature appeared through the map door in 8 of
/// 10 runs, through the portal into the side room in 6 of 10; always in
/// slot 0 with the random mark. Jitter runs: samples.
#[test]
fn random_creatures_appear() {
    let mut a = closed();
    random_creature(&mut a, ROOM, 75);
    let eng = count(SEEDS, |seed| {
        let mut g = start(&a, seed, tee(12));
        let slot = appeared(&walk_in(&mut g), ROOM);
        if let Some(s) = slot {
            assert!(s == 0 && g.is_random(1, 0));
        }
        slot.is_some()
    });
    same_rate(
        "random creature through the map door",
        Sweep::Sample,
        8,
        10,
        eng,
        SEEDS,
    );

    let mut a = with_portal(DOOR, to_side());
    random_creature(&mut a, SIDE, 75);
    let eng = count(SEEDS, |seed| {
        let mut g = start(&a, seed, tee(12));
        walk_in(&mut g);
        let ev = east(&mut g, 3);
        assert!(took_door(&ev, side(2, 4)));
        let slot = appeared(&ev, SIDE);
        if let Some(s) = slot {
            assert!(s == 0 && g.is_random(1, 0));
        }
        slot.is_some()
    });
    same_rate(
        "random creature through the portal",
        Sweep::Sample,
        6,
        10,
        eng,
        SEEDS,
    );
}

/// capture.md 10.14.3 (`fall`, 256 states, five round ends each): seven
/// residents of ours (Speed 0, slots 0-6, on the top row and 12,8) and the
/// random creature (Speed 0, slot 7, on 7,1) in room 0; a Speed 7 player,
/// Constitution and Life Force 20, walks in and on through the portal.
/// From the end of the round T left room 0 in, the creature went in 114
/// runs: 27, 29, 26, 19 and 13 at the first to fifth round end; 142 kept
/// it. Every later state follows from the first: a histogram.
#[test]
fn random_creatures_fall_away() {
    let mut a = with_portal(DOOR, to_side());
    for (slot, (x, y)) in [(1, 1), (2, 1), (3, 1), (4, 1), (5, 1), (6, 1), (12, 8)]
        .into_iter()
        .enumerate()
    {
        ours(&mut a, slot, &format!("R{slot}"), x, y, |c| c.speed = 0);
    }
    random_creature(&mut a, ROOM, 100);
    let gone = |ev: &[Event]| {
        ev.contains(&Event::CreatureRemoved {
            region: 1,
            slot: 7,
            reason: RemovalReason::LeftBehind,
        })
    };
    let player = PlayerSetup {
        speed: 7,
        constitution: 20,
        life_force: 20,
        ..Default::default()
    };
    let ends = histogram(SEEDS, 6, |seed| {
        let mut g = start(&a, seed, player.clone());
        assert_eq!(appeared(&walk_in(&mut g), ROOM), Some(7));
        let c = g.creature_mut(1, 7).expect("the random creature");
        (c.x, c.y) = (7, 1);
        // 3 moves: (5,4), (6,4) and the portal.
        let ev = east(&mut g, 3);
        assert_eq!(g.players()[0].at, side(2, 4));
        if gone(&ev) {
            return 0;
        }
        for end in 1..5 {
            if gone(&wait(&mut g)) {
                return end;
            }
        }
        5
    });
    same_histogram(
        "random creature gone by round end",
        &[27, 29, 26, 19, 13, 142],
        &ends,
    );
}

// ---- the catch-up (magic 5.5) ------------------------------------------------

/// The catch-up's AA: Speed 0, Constitution 20, Wisdom 20 (a peaceful
/// friend on 12,2, the player far off).
fn aa() -> Adventure {
    let mut a = closed();
    ours(&mut a, 0, "AA", 12, 2, |c| {
        c.speed = 0;
        c.constitution = 20;
        c.life_force = 5;
        c.wisdom = 20;
        c.power = 2;
    });
    a
}

/// AA with Life Force 5 and Power 2 takes its turn N rounds after the one
/// its byte names (`None`: the byte 0, no turn yet). Its Life Force and
/// Power after the turn. The engine's turn is in round 3 (so no N here
/// makes the byte 0).
fn catch(a: &Adventure, seed: u64, n: Option<u8>) -> (u8, u8) {
    let mut g = game(a, seed, PlayerSetup::default(), room(4, 4));
    for _ in 0..3 {
        wait(&mut g);
    }
    assert_eq!(g.round(), 3);
    let c = g.creature_mut(1, 0).expect("AA");
    c.life_force = 5;
    c.power = 2;
    c.last_turn_round = n.map_or(0, |n| 3u8.wrapping_sub(n));
    wait(&mut g);
    let c = g.creature(1, 0).expect("AA");
    assert_eq!(c.last_turn_round, 3, "the byte is the turn's round after it");
    (c.life_force, c.power)
}

/// Life Force after a Speed 0 creature's turn: `caught` after the catch-up,
/// then the top-of-turn die's point (1 in 6) or not, then the idle move's 1
/// (capped at Constitution 20).
fn after_turn(caught: u8) -> [u8; 2] {
    [(caught + 1).min(20), (caught + 2).min(20)]
}

/// capture.md 10.23 (`catch-B`, one run each): AA (Speed 0, Constitution
/// 20, Life Force 5, Power 2) its byte set to B before its turn in round
/// 40. With N = 40 - B (mod 256), Life Force 5 + min(N / 2, 15) and Power
/// 2 + min(N - 1, 18), plus the turn's own 1 each (to 20): B 39: 6, 3; 38:
/// 7, 4; 30: 11, 12; 24: 14, 18; 10: 20, 20; 200 (the wrap): 20, 20; 0 (no
/// catch-up): 7, 3, whose extra point is the top-of-turn die (10.27,
/// 10.33). capture.md 10.27 (`catch-40`, `-39`, `-0`, `-41`): N = 0 gains
/// nothing (6, 3 in 18 of 20, 7, 3 in 2); N = 1 6, 3 in 16 of 20, 7, 3 in
/// 4; byte 0 6, 3 in 16 of 17, 7, 3 in 1; N = 255 20, 20 in 4 of 4. Power
/// is exact; the die's point is a rate: 7 of the 57.
#[test]
fn catching_up_table() {
    let a = aa();
    // (B, Life Force and Power captured)
    for (b, life, power) in [
        (39u8, 6, 3),
        (38, 7, 4),
        (30, 11, 12),
        (24, 14, 18),
        (10, 20, 20),
        (200, 20, 20),
        (0, 7, 3),
        (40, 6, 3),
        (41, 20, 20),
    ] {
        let n = (b != 0).then(|| 40u8.wrapping_sub(b));
        let caught = match n {
            Some(n) => (5 + n / 2).min(20),
            None => 5,
        };
        let want = after_turn(caught);
        assert!(want.contains(&life), "B {b}: the rule against the capture");
        let mut seen = [false; 2];
        for seed in 0..64 {
            let (l, p) = catch(&a, seed, n);
            assert_eq!(p, power, "B {b} seed {seed}: Power");
            let i = want.iter().position(|&w| w == l);
            assert!(i.is_some(), "B {b} seed {seed}: Life Force {l}, not {want:?}");
            seen[i.unwrap()] = true;
        }
        assert!(seen[0] && (seen[1] || want[0] == want[1]), "B {b}: {seen:?}");
    }
    // The extra point at N 0, N 1 and byte 0: the die.
    let extra = count(SEEDS, |seed| {
        let n = [Some(0), Some(1), None][(seed % 3) as usize];
        catch(&a, seed, n).0 == 7
    });
    same_rate(
        "the Speed 0 creature's extra point",
        Sweep::Sample,
        7,
        57,
        extra,
        SEEDS,
    );
}

/// capture.md 10.30 (`ab-4`, `ab-10`, `ab-30`, one run each): AA (Speed 0,
/// Constitution 20, Wisdom 20) left while T went out to the map; it came
/// back from the disk with Life Force 8, Power 5, and at its first turn N
/// = 6, 12 and 32 rounds after its last read Life Force, Power 12, 11; 14,
/// 17; 20, 20 (the rule: 8 + N / 2; 5 + (N - 1) + 1 drift; `ab-4`'s one
/// more is the die). Read at its turn, before its idle move, which the
/// engine's reading after the turn includes (+1 below 20). Here T leaves by
/// a door on ROOM (4,5) to the map's start and comes back by the map door;
/// AA's record is set to 8, 5 while the region is unvisited.
#[test]
fn catching_up_across_a_real_absence() {
    let mut a = aa();
    door_at(&mut a, 4, 5, DOOR, Destination::WorldMap { x: 12, y: 12 });
    for (n, life, power) in [(6u32, 12, 11), (12, 14, 17), (32, 20, 20)] {
        let caught = (8 + n / 2).min(20) as u8;
        assert!([caught, (caught + 1).min(20)].contains(&life), "N {n}: the rule");
        for seed in 0..32 {
            let mut g = game(&a, seed, PlayerSetup::default(), room(4, 4));
            for _ in 0..3 {
                wait(&mut g);
            }
            let ev = g.apply(Command::Move { dir: Dir::S });
            assert!(took_door(&ev, Location::World { x: 12, y: 12 }), "{ev:?}");
            wait(&mut g);
            let last = u32::from(g.creature(1, 0).expect("AA").last_turn_round);
            let c = g.creature_mut(1, 0).expect("AA");
            c.life_force = 8;
            c.power = 5;
            while g.round() < last + n - 1 {
                assert!(matches!(g.players()[0].at, Location::World { .. }));
                wait(&mut g);
            }
            assert_eq!(g.round(), last + n - 1, "N {n}");
            walk_in(&mut g);
            wait(&mut g);
            let c = g.creature(1, 0).expect("AA");
            assert_eq!(u32::from(c.last_turn_round), last + n, "N {n}: AA's turn");
            assert_eq!(c.power, power, "N {n} seed {seed}");
            let want = after_turn(caught);
            assert!(
                want.contains(&c.life_force),
                "N {n} seed {seed}: {} not {want:?}",
                c.life_force
            );
        }
    }
}

// ---- Power drift (magic 5.1) -------------------------------------------------

/// capture.md 8.4 / 10.18 (`join.sh`, `jointime.sh`): a joining player
/// keeps its record's Power and Life Force; then Power moves 1 a round
/// toward Wisdom: 20 to 13 in 7 rounds, 5 to 19 in 14 (also 10 under 20,
/// 31 under 31: not held to 27); Life Force 5 under Constitution 20 is not
/// raised to it but heals. A lone player has one turn a round; the first
/// drift comes at the top of its first turn.
#[test]
fn power_drift_on_joining() {
    let a = base(Platform::C64);
    for (power, wisdom) in [(20u8, 13u8), (10, 20), (31, 31), (5, 30)] {
        let mut g = start(
            &a,
            1,
            PlayerSetup {
                wisdom,
                power: Some(power),
                ..Default::default()
            },
        );
        for k in 1..=20u8 {
            let want = if power > wisdom {
                power.saturating_sub(k).max(wisdom)
            } else {
                (power + k).min(wisdom)
            };
            assert_eq!(
                g.players()[0].power,
                want,
                "Power {power} Wisdom {wisdom}, turn {k}"
            );
            wait(&mut g);
        }
    }
    let mut g = start(
        &a,
        1,
        PlayerSetup {
            constitution: 20,
            life_force: 5,
            ..Default::default()
        },
    );
    assert!([5, 6].contains(&life(&g)), "{}", life(&g));
    for _ in 0..40 {
        wait(&mut g);
        assert!(life(&g) <= 20);
    }
    assert_eq!(life(&g), 20);
}

/// capture.md 7 / 10.6.5 (`drift`, 3 runs a case, all alike): a Speed 1
/// player, Wisdom, Constitution and Life Force 20, casts a cost-0 spell on
/// itself on turn 1, the profile read each of 60 turns. Power -10 (`0x8A`):
/// 12, 13, ... 20 one a turn, then 20; Power +10 (`0x0A`): 28, 27, ... 20;
/// Life Force +10 permanent (`0x0A`): 30 for all 60 turns. The first reading
/// came two turn ends after the cast, so it showed two turns' drift.
#[test]
fn power_drifts_back_to_wisdom_one_a_turn() {
    // (spell, parameter, the capture's first reading)
    for (kind, parameter, first) in [
        (SpellActionKind::ChangePower, 0x8A, 12u8),
        (SpellActionKind::ChangePower, 0x0A, 28),
        (SpellActionKind::ChangeLifeForce, 0x0A, 30),
    ] {
        let mut a = base(Platform::C64);
        thing(&mut a, SPELL, "SPELL", spell(0, SpellAction { kind, parameter }));
        let mut g = start(
            &a,
            1,
            PlayerSetup {
                speed: 1,
                wisdom: 20,
                constitution: 20,
                life_force: 20,
                possessions: vec![SPELL],
                ..Default::default()
            },
        );
        let ev = g.apply(Command::Cast {
            thing: SPELL,
            at: None,
            target: None,
        });
        assert!(ev
            .iter()
            .any(|e| matches!(e, Event::SpellCast { success: true, .. })));
        // The engine's reading at the top of each turn after the cast's;
        // the capture's first came a turn later, at the engine's second.
        let mut seen = Vec::new();
        for _ in 0..61 {
            let p = &g.players()[0];
            seen.push(if kind == SpellActionKind::ChangePower {
                p.power
            } else {
                p.life_force
            });
            wait(&mut g);
        }
        let want: Vec<u8> = (0..60u8)
            .map(|j| match parameter {
                0x8A => (first + j).min(20),
                _ if kind == SpellActionKind::ChangePower => first.saturating_sub(j).max(20),
                _ => first,
            })
            .collect();
        assert_eq!(seen[1..], want[..], "{kind:?} {parameter:#x}");
    }
}

// ---- SAVE GAME's moves (creatures 2.2.7, magic 5.3) --------------------------

/// T (Speed 6, Constitution 40, Life Force 10) on ROOM (4,4) with 4 of
/// its 6 moves left.
fn four_left(a: &Adventure, seed: u64) -> Game {
    let mut g = game(
        a,
        seed,
        PlayerSetup {
            life_force: 10,
            ..tee(6)
        },
        room(4, 4),
    );
    g.apply(Command::Idle);
    g.apply(Command::Idle);
    assert_eq!(g.players()[0].moves_left, 4);
    g
}

/// capture.md 10.28 (`c1`, `c3`, `c4-push`; one run each): after SAVE
/// GAME the prompt came back with 4 of 6 moves; pushed south T walked 4
/// squares, 4,4 to 4,8; left alone the 4 went one by one as idle moves.
#[test]
fn save_game_does_not_end_the_turn() {
    let a = closed();
    let mut g = four_left(&a, 1);
    let ev = g.apply(Command::SaveGame);
    assert!(!ev.iter().any(|e| matches!(e, Event::TurnEnded { .. })), "{ev:?}");
    assert_eq!(g.players()[0].moves_left, 4);
    let mut push = g.clone();
    let mut ev = Vec::new();
    for _ in 0..4 {
        ev.extend(push.apply(Command::Move { dir: Dir::S }));
    }
    assert_eq!(ev.iter().filter(|e| matches!(e, Event::Moved { .. })).count(), 4);
    assert!(ev.iter().any(|e| matches!(e, Event::TurnEnded { player: 0 })));
    assert!(ev.iter().any(|e| matches!(e, Event::Saved { .. })));
    // The round's other turn starts on 4,8.
    assert_eq!(push.players()[0].at, room(4, 8));
    for left in (1..4).rev() {
        let ev = g.apply(Command::Idle);
        assert!(!ev.iter().any(|e| matches!(e, Event::TurnEnded { .. })));
        assert_eq!(g.players()[0].moves_left, left);
    }
    let ev = g.apply(Command::Idle);
    assert!(ev.iter().any(|e| matches!(e, Event::TurnEnded { player: 0 })));
}

/// capture.md 10.28 item 4 (`c4-save`, states 0..7, 8 runs; control
/// `c4-idle`, 8 runs): Life Force 10 under Constitution 40, a turn of 6
/// full moves. After SAVE GAME, 32 idle moves, one draw each, Life Force
/// +1 at 4; without it, 40 idle moves, +1 at 7. Here the first 3 of the
/// 4 idle moves a seed (the 4th ends the turn and mixes in the next top's
/// die). Samples.
#[test]
fn save_game_moves_are_ordinary_idle_moves() {
    let a = closed();
    for (save, cap, cap_n) in [(true, 4, 32), (false, 7, 40)] {
        let mut healed = 0;
        for seed in 0..SEEDS {
            let mut g = four_left(&a, seed);
            if save {
                g.apply(Command::SaveGame);
            }
            healed += idle_heals(&mut g, 3);
        }
        let what = if save { "after SAVE GAME" } else { "without it" };
        same_rate(
            &format!("idle move heals {what}"),
            Sweep::Sample,
            cap,
            cap_n,
            healed,
            3 * SEEDS,
        );
    }
}

// ---- the idle healing roll's range (magic 5.3.1) -----------------------------

/// capture.md 10.29 (`--set-random-state --draws`, `turnstarts.py
/// --range`): T (Speed 12, Constitution 40, Life Force 10), EE (aggressive
/// enemy, Speed 0, Strength 1). The range fitting every idle move: `adj
/// s8` turn 0 (8 carried, the door turn 12; 64 runs, 512 moves) 12, where
/// 8 fit 486, 6 468, 4 415; `adj s8` turn 1 (beside EE, 6; 384 moves) 6,
/// where 12 fit 362, 8 369, 4 350; `adj s0` (none carried, 6; 192 moves) 6,
/// where 12 fit 173, 8 176; `adjside s3` (EE on 3,4; 3 carried through the
/// side portal from a door turn of 6; 192 moves) 6, where 12 fit 176, 8
/// 182, 3 161. The doc gives the fits, not the heals: the engine's rate per
/// idle move must be 1 in the fitted range and not 1 in the others.
#[test]
fn idle_healing_roll_range() {
    let t = PlayerSetup {
        life_force: 10,
        ..tee(12)
    };
    let ee = |a: &mut Adventure, x: u8| {
        creature(a, 0, x, 4, |c| {
            c.name = "EE".into();
            c.speed = 0;
            c.strength = 1;
        })
    };
    let mut a = closed();
    ee(&mut a, 5);
    let (mut s8_0, mut s8_1, mut s0) = (0, 0, 0);
    for seed in 0..SEEDS {
        let mut g = start(&a, seed, t.clone());
        assert_eq!(started(&walk_in(&mut g)), (0, 8, false));
        s8_0 += idle_heals(&mut g, 7);
        assert_eq!(started(&g.apply(Command::Idle)), (0, 6, true));
        s8_1 += idle_heals(&mut g, 5);
        let mut g = start(&a, seed, t.clone());
        for _ in 0..8 {
            g.apply(Command::Idle);
        }
        assert_eq!(started(&walk_in(&mut g)), (0, 6, true));
        s0 += idle_heals(&mut g, 5);
    }
    heal_range("adj s8 turn 0", s8_0, 7 * SEEDS, 12, &[8, 6, 4]);
    heal_range("adj s8 turn 1", s8_1, 5 * SEEDS, 6, &[12, 8, 4]);
    heal_range("adj s0", s0, 5 * SEEDS, 6, &[12, 8]);

    let mut a = with_portal(DOOR, to_side());
    ee(&mut a, 3);
    let mut s3 = 0;
    for seed in 0..SEEDS {
        let mut g = start(&a, seed, t.clone());
        walk_in(&mut g);
        assert_eq!(started(&wait(&mut g)), (0, 6, true));
        let ev = east(&mut g, 3);
        assert_eq!(started(&ev), (0, 3, false));
        assert_eq!(g.players()[0].at, side(2, 4));
        s3 += idle_heals(&mut g, 2);
    }
    heal_range("adjside s3", s3, 2 * SEEDS, 6, &[12, 8, 3]);
}

// ---- goals and exits (creatures 6.2-6.5, 2.7) -------------------------------

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

fn through(turn: &[Event]) -> bool {
    turn.iter()
        .any(|e| matches!(e, Event::CreatureTookDoor { slot: 0, .. }))
}

/// rr33's `s0x-turn-end` room (10.39 item 1): the side portal on 7,4, T on
/// 4,4, and NN, a creature of ours of Wisdom 3 and `speed`, in slot 0 on
/// (x, y), its class byte set to `class` (alignment, aggressive).
fn nn(x: u8, y: u8, speed: u8, class: (Alignment, bool)) -> Adventure {
    let mut a = with_portal(DOOR, to_side());
    ours(&mut a, 0, "NN", x, y, |c| {
        c.speed = speed;
        c.wisdom = 3;
        c.strategy.alignment = class.0;
        c.strategy.aggressive = class.1;
    });
    a
}

/// NN's first `turns` turns with T waiting on 4,4, up to the one that takes
/// it through the portal: (turns seen, 1 if it went through). A Speed 0
/// creature that stays never walks (2.7.3).
fn exit_turns(a: &Adventure, seed: u64, turns: usize) -> (u64, u64) {
    let mut g = game(a, seed, tee(6), room(4, 4));
    let mut ev = Vec::new();
    for _ in 0..4 * turns + 4 {
        let seen = turns_of(&ev, 0);
        if seen.len() > turns || seen.iter().any(|t| through(t)) {
            break;
        }
        ev.extend(wait(&mut g));
    }
    let speed = a.regions[0].residents[0].speed;
    let mut n = 0;
    for turn in turns_of(&ev, 0).into_iter().take(turns) {
        n += 1;
        if through(turn) {
            return (n, 1);
        }
        if speed == 0 {
            assert!(
                !turn
                    .iter()
                    .any(|e| matches!(e, Event::CreatureMoved { slot: 0, .. })),
                "seed {seed}: a Speed 0 creature walked: {turn:?}"
            );
        }
    }
    assert_eq!(n, turns as u64, "seed {seed}: NN's turns");
    (n, 0)
}

/// capture.md 10.39 item 1 (rr35, `goalcheck.py`; creatures 6.5's exit
/// cost of 4 on the turn's first step choice, 1 or 3 after): NN beside the
/// portal on 6,4 with T on 4,4. `s0x-long` and `x-pn0` (a peaceful neutral,
/// Speed 0, its goal the exit): 48 runs, 188 turns, through in 3; `x-pf0`,
/// `x-pf2` (a peaceful friend, Speed 0 and 2, its goal the player): 64
/// runs, 224 turns, never; `x-an0` (an aggressive neutral, a fighter,
/// Speed 0, its goal the player): 16 runs, 64 turns, never. `ta-pn0`,
/// `ta-pf0` (NN on 7,5 below the portal, T gone through it: NN's one turn
/// with no player in the room, its goal the exit): 96 turns, through in 3;
/// `ta-an0` 48, through in 1. `s26-pn2` (Speed 2 from 7,6, two squares
/// off: the exit beside it at the second step, cost 1): 32 turns, through
/// in 23; `s26-an2` (its goal the player): 32, through in 2. Many rolls a
/// turn: samples, compared per turn.
#[test]
fn an_exit_costs_4_at_a_turns_first_step_choice() {
    use Alignment::{Friend, Neutral};
    const RUNS: u64 = SEEDS / 4;
    let pn = (Neutral, false);
    let check = |name: &str, cap: (u64, u64), eng: (u64, u64)| {
        same_rate(
            &format!("{name}: turns through the portal"),
            Sweep::Sample,
            cap.0,
            cap.1,
            eng.1,
            eng.0,
        )
    };
    let sum = |a: &Adventure, turns: usize| {
        (0..RUNS).fold((0, 0), |acc, seed| {
            let (n, k) = exit_turns(a, seed, turns);
            (acc.0 + n, acc.1 + k)
        })
    };

    check("s0x-long, x-pn0", (3, 188), sum(&nn(6, 4, 0, pn), 4));
    let pf = [0, 2].map(|speed| sum(&nn(6, 4, speed, (Friend, false)), 4));
    check("x-pf0, x-pf2", (0, 224), (pf[0].0 + pf[1].0, pf[0].1 + pf[1].1));
    check("x-an0", (0, 64), sum(&nn(6, 4, 0, (Neutral, true)), 4));

    // `ta-*`: T walks east through the portal; NN's next turn.
    let ta = |class: (Alignment, bool)| {
        let a = nn(7, 5, 0, class);
        let k = count(RUNS, |seed| {
            let mut g = game(&a, seed, tee(6), room(4, 4));
            let mut ev = Vec::new();
            for _ in 0..3 {
                ev.extend(g.apply(Command::Move { dir: Dir::E }));
            }
            assert!(took_door(&ev, side(2, 4)), "seed {seed}: {ev:?}");
            for _ in 0..4 {
                if !turns_of(&ev, 0).is_empty() {
                    break;
                }
                ev.extend(wait(&mut g));
            }
            let turns = turns_of(&ev, 0);
            let turn = turns
                .first()
                .unwrap_or_else(|| panic!("seed {seed}: NN took no turn: {ev:?}"));
            through(turn)
        });
        (RUNS, k)
    };
    let (n1, k1) = ta(pn);
    let (n2, k2) = ta((Friend, false));
    check("ta-pn0, ta-pf0", (3, 96), (n1 + n2, k1 + k2));
    check("ta-an0", (1, 48), ta((Neutral, true)));

    check("s26-pn2", (23, 32), sum(&nn(7, 6, 2, pn), 1));
    check("s26-an2", (2, 32), sum(&nn(7, 6, 2, (Neutral, true)), 1));
}

/// capture.md 10.39 item 2 (creatures 6.2, 6.3; 32 states each, two or
/// three of EE's turns): EE, an aggressive enemy of Speed 3 owning no
/// weapon, Wisdom 3, with T on 4,4 and FF, a friend of Speed 0, Wisdom 3.
/// `g-f`: EE on 12,4, FF peaceful on 12,5 (FF scores 6 + 1 = 7, T 0 + 8):
/// 64 turns, FF the goal in 60; `g-t`: EE on 11,4, FF on 11,5 (7 against
/// 7, the tie to the player): 70 turns, FF in 1; `g-a`: EE on 12,4, FF
/// aggressive on 12,6 (4 + 2 against 8): 91 turns, FF in 89. The goal is
/// read as `goalcheck.py` reads it: 6.2's score (rank + steps) from the
/// squares at each of EE's turn starts, with those ranks; where EE has
/// wandered to is the engine's. Samples, per turn.
#[test]
fn a_fighters_goal_among_other_individuals() {
    const RUNS: u64 = SEEDS / 4;
    // (name, EE's square, FF's square, FF aggressive, FF's rank, EE's turns
    // a run, capture: turns with FF the goal, turns)
    type Case = (&'static str, (u8, u8), (u8, u8), bool, u16, usize, u64, u64);
    let cases: [Case; 3] = [
        ("g-f", (12, 4), (12, 5), false, 6, 2, 60, 64),
        ("g-t", (11, 4), (11, 5), false, 6, 2, 1, 70),
        ("g-a", (12, 4), (12, 6), true, 4, 3, 89, 91),
    ];
    let steps = |a: (u8, u8), b: (u8, u8)| u16::from(a.0.abs_diff(b.0) + a.1.abs_diff(b.1));
    for (name, ee_at, ff_at, aggressive, rank, turns, cap_ff, cap_n) in cases {
        let mut a = with_portal(DOOR, to_side());
        ours(&mut a, 0, "EE", ee_at.0, ee_at.1, |c| {
            c.speed = 3;
            c.wisdom = 3;
            c.strategy.alignment = Alignment::Enemy;
            c.strategy.aggressive = true;
        });
        ours(&mut a, 1, "FF", ff_at.0, ff_at.1, |c| {
            c.speed = 0;
            c.wisdom = 3;
            c.strategy.aggressive = aggressive;
        });
        let (mut n, mut ff) = (0, 0);
        for seed in 0..RUNS {
            let mut g = game(&a, seed, tee(6), room(4, 4));
            let (mut ee, mut f, mut k) = (ee_at, ff_at, 0);
            'run: for _ in 0..4 * turns {
                for e in wait(&mut g) {
                    match e {
                        Event::CreatureTurn {
                            region: 1, slot: 0, ..
                        } => {
                            n += 1;
                            ff += u64::from(rank + steps(ee, f) < steps(ee, (4, 4)));
                            k += 1;
                            if k == turns {
                                break 'run;
                            }
                        }
                        Event::CreatureMoved {
                            region: 1, slot, to, ..
                        } => match slot {
                            0 => ee = to.xy(),
                            1 => f = to.xy(),
                            _ => {}
                        },
                        Event::CreatureTookDoor {
                            region: 1, slot: 0, ..
                        } => break 'run,
                        _ => {}
                    }
                }
            }
        }
        same_rate(
            &format!("{name}: turns with FF the goal"),
            Sweep::Sample,
            cap_ff,
            cap_n,
            ff,
            n,
        );
    }
}

/// capture.md 10.40 (`rc-sw`, 32 runs from the map start, the random state
/// set to 0, 8, .., 248 before the walk in; movement 7.2, creatures 3.2):
/// RR, a random creature of Speed 0 at room chance 15 (75 %), appeared as
/// T came in by the map door in 21 of 32, always in slot 0 with the random
/// mark, on 19 different squares among the 21, all over the inside, never
/// T's 4,4; the room held no things. A sample; the engine's squares must
/// cover the inside but T's.
#[test]
fn the_room_random_creature_lands_anywhere_inside_but_the_players_square() {
    let mut a = closed();
    random_creature(&mut a, ROOM, 75);
    let mut squares = std::collections::BTreeSet::new();
    let eng = count(SEEDS, |seed| {
        let mut g = start(&a, seed, tee(12));
        let ev = walk_in(&mut g);
        let Some(at) = ev.iter().find_map(|e| match *e {
            Event::RoomEncounter { at, slot, .. } => {
                assert_eq!(slot, Some(0), "seed {seed}");
                at
            }
            _ => None,
        }) else {
            return false;
        };
        assert!(g.is_random(1, 0), "seed {seed}");
        let (x, y) = at.xy();
        assert!(
            (1..=13).contains(&x) && (1..=8).contains(&y),
            "seed {seed}: {at:?}"
        );
        assert_ne!((x, y), (4, 4), "seed {seed}: on T's square");
        squares.insert((x, y));
        true
    });
    same_rate("rc-sw: runs with RR", Sweep::Sample, 21, 32, eng, SEEDS);
    assert_eq!(squares.len(), 13 * 8 - 1, "{squares:?}");
}

// ---- capture.md 10.41: creatures and movement, the open cases left ---------

/// The game with exactly these region-1 slots carrying the random mark
/// (`--set E1A9=158` on the slot's mark byte, creatures 2.4).
fn marked(g: &Game, slots: &[usize]) -> Game {
    let mut v = serde_json::to_value(g).expect("serializes");
    v["random"] = serde_json::json!(slots.iter().map(|&s| (1u8, s)).collect::<Vec<_>>());
    serde_json::from_value(v).expect("deserializes")
}

/// capture.md 10.41 item 1 (rr37 `f2/sweep`, 256 states, five round ends
/// each; creatures 2.4): 10.14.3's `fall/side.vsf` (residents of ours in
/// slots 0 to 6, the random creature in slot 7, T gone on into the side
/// room) with slot 0's mark set too, so two random creatures stand in room
/// 0 with no player. Slot 0 was removed in 115 runs, slot 7 in 97, both at
/// the same round end in 14. Later rolls follow from the first: samples.
#[test]
fn two_random_creatures_take_a_roll_each() {
    let mut a = with_portal(DOOR, to_side());
    for (slot, (x, y)) in [(1, 1), (2, 1), (3, 1), (4, 1), (5, 1), (6, 1), (12, 8)]
        .into_iter()
        .enumerate()
    {
        ours(&mut a, slot, &format!("R{slot}"), x, y, |c| c.speed = 0);
    }
    random_creature(&mut a, ROOM, 100);
    let player = PlayerSetup {
        speed: 7,
        constitution: 20,
        life_force: 20,
        ..Default::default()
    };
    let gone = |ev: &[Event], n: usize| {
        ev.contains(&Event::CreatureRemoved {
            region: 1,
            slot: n,
            reason: RemovalReason::LeftBehind,
        })
    };
    let (mut runs, mut zero, mut seven, mut both) = (0, 0, 0, 0);
    for seed in 0..SEEDS / 4 {
        let mut g = start(&a, seed, player.clone());
        assert_eq!(appeared(&walk_in(&mut g), ROOM), Some(7));
        let c = g.creature_mut(1, 7).expect("the random creature");
        (c.x, c.y) = (7, 1);
        let ev = east(&mut g, 3);
        assert_eq!(g.players()[0].at, side(2, 4));
        // The snapshot had both standing: a run losing slot 7 first is not
        // one of the capture's.
        if gone(&ev, 7) {
            continue;
        }
        let mut g = marked(&g, &[0, 7]);
        runs += 1;
        let (mut at0, mut at7) = (None, None);
        for end in 0..5 {
            let ev = wait(&mut g);
            assert_eq!(g.players()[0].at, side(2, 4));
            if gone(&ev, 0) {
                at0 = Some(end);
            }
            if gone(&ev, 7) {
                at7 = Some(end);
            }
        }
        // A removed slot keeps its mark.
        assert!(g.is_random(1, 0) && g.is_random(1, 7), "seed {seed}");
        zero += u64::from(at0.is_some());
        seven += u64::from(at7.is_some());
        both += u64::from(at0.is_some() && at0 == at7);
    }
    same_rate("f2: slot 0 removed", Sweep::Sample, 115, 256, zero, runs);
    same_rate("f2: slot 7 removed", Sweep::Sample, 97, 256, seven, runs);
    same_rate("f2: both at one round end", Sweep::Sample, 14, 256, both, runs);
}

/// capture.md 10.41 item 2 (rr37 `hp-s6` .. `hp-s1`, one run each;
/// creatures 2.6): T on 4,4, EE an aggressive enemy of Speed 0, Wisdom 3,
/// on 6,4. T of Speed 6, 7, 5, 3 or 1 steps east beside EE: its moves at
/// that turn's start 6, 7, 5, 3, 1; right after the step 5, 6, 4, 2, 0;
/// at each of its next two turn starts 3, 4, 3, 2, 1, halved rounding up.
#[test]
fn halving_rounds_up_at_odd_speeds_and_only_at_the_turns_start() {
    let mut a = closed();
    creature(&mut a, 0, 6, 4, |c| {
        c.name = "EE".into();
        c.speed = 0;
        c.wisdom = 3;
    });
    for (speed, after, halved) in [(6, 5, 3), (7, 6, 4), (5, 4, 3), (3, 2, 2), (1, 0, 1)] {
        let mut g = game(&a, 1, tee(speed), room(4, 4));
        assert_eq!(started(&wait(&mut g)), (0, speed, false), "hp-s{speed}");
        let ev = g.apply(Command::Move { dir: Dir::E });
        assert_eq!(g.players()[0].at, room(5, 4), "hp-s{speed}");
        let mut starts = Vec::new();
        if after == 0 {
            assert!(
                ev.contains(&Event::TurnEnded { player: 0 }),
                "hp-s{speed}: {ev:?}"
            );
            starts.push(started(&ev).1);
        } else {
            assert_eq!(g.players()[0].moves_left, after, "hp-s{speed}: after the step");
        }
        while starts.len() < 2 {
            starts.push(started(&wait(&mut g)).1);
        }
        assert_eq!(starts, [halved; 2], "hp-s{speed}");
    }
}

/// Every turn start in `ev`, read against the squares then: (who, moves,
/// beside a foe at that moment). T stays on 4,4; the creatures' squares
/// follow their steps from `at`.
fn turn_starts(ev: &[Event], at: &mut [(u8, u8)]) -> Vec<(Mover, u8, bool)> {
    let beside = |p: (u8, u8)| p.0.abs_diff(4) + p.1.abs_diff(4) == 1;
    let mut out = Vec::new();
    for e in ev {
        match *e {
            Event::CreatureMoved {
                region: 1, slot, to, ..
            } => at[slot] = to.xy(),
            Event::CreatureTurn {
                region: 1,
                slot,
                moves,
            } => out.push((C(slot), moves, beside(at[slot]))),
            Event::TurnStarted { player, moves, .. } => {
                out.push((P(player, moves), moves, at.iter().any(|&p| beside(p))))
            }
            _ => {}
        }
    }
    out
}

/// capture.md 10.41 item 2 (rr37 `mc-m`, `cr-m`, 8 states each, 40M;
/// `adjcheck.py`; creatures 2.6). `mc`: MM, an aggressive enemy of Speed 7
/// on 8,4, walks up to T (Speed 6) on 4,4: MM's 13 turns begun beside T
/// had 4 moves, its 13 others 7; T's 15 turns begun with MM beside it 3,
/// its 7 others 6. `cr`: peaceful enemies of Speed 1, 3, 5, 7 on the four
/// squares beside T (Speed 2) and the same four far off: beside T at a turn
/// start 1, 2, 3, 4, elsewhere 1, 3, 5, 7; every turn start as predicted.
/// The rule is deterministic: every engine turn start must fit it.
#[test]
fn creatures_are_halved_rounding_up_beside_a_foe() {
    let up = |s: u8| s.div_ceil(2);
    // `mc`
    let mut a = closed();
    creature(&mut a, 0, 8, 4, |c| {
        c.name = "MM".into();
        c.speed = 7;
        c.wisdom = 3;
    });
    let mut seen = [[0; 2]; 2];
    for seed in 0..8 {
        let mut g = game(&a, seed, tee(6), room(4, 4));
        let mut at = [(8, 4)];
        for _ in 0..8 {
            for (who, moves, beside) in turn_starts(&wait(&mut g), &mut at) {
                let (k, speed) = match who {
                    C(_) => (0, 7),
                    P(..) => (1, 6),
                };
                let want = if beside { up(speed) } else { speed };
                assert_eq!(moves, want, "mc seed {seed}: {who:?} beside {beside}");
                seen[k][beside as usize] += 1;
            }
        }
    }
    assert!(seen.iter().flatten().all(|&n| n > 0), "mc: {seen:?}");

    // `cr`
    let mut a = closed();
    let squares = [(3, 4), (5, 4), (4, 3), (4, 5), (1, 1), (13, 1), (1, 8), (13, 8)];
    let speeds = [1, 3, 5, 7, 1, 3, 5, 7];
    for (n, (&(x, y), &speed)) in squares.iter().zip(&speeds).enumerate() {
        creature(&mut a, n, x, y, |c| {
            c.name = format!("E{n}");
            c.speed = speed;
            c.wisdom = 3;
            c.strategy.aggressive = false;
        });
    }
    let mut seen = [[0; 2]; 4];
    for seed in 0..8 {
        let mut g = game(&a, seed, tee(2), room(4, 4));
        let mut at = squares;
        for _ in 0..5 {
            for (who, moves, beside) in turn_starts(&wait(&mut g), &mut at) {
                let C(n) = who else { continue };
                let want = if beside { up(speeds[n]) } else { speeds[n] };
                assert_eq!(moves, want, "cr seed {seed}: E{n} beside {beside}");
                seen[n % 4][beside as usize] += 1;
            }
        }
    }
    assert!(seen.iter().all(|s| s[1] > 0), "cr: {seen:?}");
}

/// capture.md 10.41 item 3 (rr37 `cs-m`, 8 states, 40M; creatures 5.1,
/// movement 5.5): peaceful friends of Speed 3 (Constitution 30, Life Force
/// 20) in the four corners, both open squares beside each holding a space:
/// FA's an "invoke spell" space (Life Force -5), FB's a plain one open to
/// anyone, FC's a custom space "when moved onto" (-5 from the placement,
/// 133), FD's a custom space open to owners of thing 50, which FD owns. In
/// 64 creature turns no creature left its corner and no Life Force fell.
/// Deterministic: every engine turn must so.
#[test]
fn creatures_never_step_onto_spaces() {
    const FIFTY: u16 = 50;
    const MINUS5: u16 = 0x85;
    let mut a = closed();
    thing(&mut a, FIFTY, "FIFTY", treasure(1, 1));
    let life = |parameter| SpellAction {
        kind: SpellActionKind::ChangeLifeForce,
        parameter,
    };
    let custom = |placement_choice, trigger, action| ThingKind::CustomSpace {
        picture: 4,
        placement_choice,
        trigger,
        action,
        destroy_needed_thing: false,
    };
    let kinds = [
        ThingKind::Space {
            picture: 4,
            access: Access::InvokeSpell { action: life(MINUS5) },
        },
        ThingKind::Space {
            picture: 4,
            access: Access::Anyone,
        },
        custom(
            PlacementChoice::SpellParameter,
            CustomTrigger::WhenMovedOnto,
            life(0),
        ),
        custom(
            PlacementChoice::Thing,
            CustomTrigger::OpenToOwnersOf,
            life(MINUS5),
        ),
    ];
    // (corner, the two squares beside it, the placement parameter)
    type Corner = ((u8, u8), [(u8, u8); 2], u8);
    let corners: [Corner; 4] = [
        ((1, 1), [(2, 1), (1, 2)], 0),
        ((13, 1), [(12, 1), (13, 2)], 0),
        ((1, 8), [(2, 8), (1, 7)], MINUS5 as u8),
        ((13, 8), [(12, 8), (13, 7)], FIFTY as u8),
    ];
    for (n, (kind, &(corner, beside, parameter))) in kinds.into_iter().zip(&corners).enumerate() {
        let t = 101 + n as u16;
        thing(&mut a, t, &format!("SPACE{n}"), kind);
        for (x, y) in beside {
            item(&mut a, ROOM, x, y, t);
            a.regions[0].rooms[ROOM as usize].items[0].parameter = parameter;
        }
        ours(&mut a, n, ["FA", "FB", "FC", "FD"][n], corner.0, corner.1, |c| {
            c.constitution = 30;
            c.life_force = 20;
            if n == 3 {
                owns(c, FIFTY);
            }
        });
    }
    let mut turns = 0;
    for seed in 0..8 {
        let mut g = game(&a, seed, tee(6), room(4, 4));
        let mut lives = [20u8; 4];
        let mut mine = 0;
        for _ in 0..8 {
            if mine >= 8 {
                break;
            }
            for e in &wait(&mut g) {
                match e {
                    Event::CreatureTurn { region: 1, .. } => mine += 1,
                    Event::CreatureMoved { slot, .. }
                    | Event::CreatureTookDoor { slot, .. }
                    | Event::CreatureBumped { slot, .. } => {
                        panic!("cs seed {seed}: slot {slot} left its corner: {e:?}")
                    }
                    _ => {}
                }
            }
            for (n, &(corner, ..)) in corners.iter().enumerate() {
                let c = g.creature(1, n).expect("alive");
                assert_eq!((c.x, c.y), corner, "cs seed {seed}: slot {n}");
                assert!(
                    c.life_force >= lives[n],
                    "cs seed {seed}: slot {n}'s Life Force fell"
                );
                lives[n] = c.life_force;
            }
        }
        assert!(mine >= 8, "cs seed {seed}: {mine} turns");
        turns += mine;
    }
    assert!(turns >= 64, "{turns}");
}

/// rr37's `ad-*` disks: on the side room's 2,4, where the portal leads, a
/// space "invoke spell" (Life Force -5, `ad-sp`), an obstacle that bumps
/// for -5 (`ad-ob`), or a custom space open only to owners of the blade
/// giving +5 (`ad-cs`); FF, a peaceful friend of Speed 6, Life Force 15,
/// on 5,5 (its Constitution 15 too, so no healing hides a fall).
fn arrival(case: &str) -> Adventure {
    const BLADE: u16 = 59;
    const PAD: u16 = 101;
    let minus5 = SpellAction {
        kind: SpellActionKind::ChangeLifeForce,
        parameter: 0x85,
    };
    let mut a = with_portal(DOOR, to_side());
    thing(&mut a, BLADE, "BLADE", blade(10));
    let (kind, parameter) = match case {
        "ad-sp" => (
            ThingKind::Space {
                picture: 4,
                access: Access::InvokeSpell { action: minus5 },
            },
            0,
        ),
        "ad-ob" => (
            ThingKind::Obstacle {
                picture: 7,
                bump_invokes_spell: true,
                action: minus5,
            },
            0,
        ),
        _ => (
            ThingKind::CustomSpace {
                picture: 4,
                placement_choice: PlacementChoice::Thing,
                trigger: CustomTrigger::OpenToOwnersOf,
                action: SpellAction {
                    kind: SpellActionKind::ChangeLifeForce,
                    parameter: 5,
                },
                destroy_needed_thing: false,
            },
            BLADE as u8,
        ),
    };
    thing(&mut a, PAD, "PAD", kind);
    item(&mut a, SIDE, 2, 4, PAD);
    a.regions[0].rooms[SIDE as usize].items[0].parameter = parameter;
    ours(&mut a, 0, "FF", 5, 5, |c| {
        c.speed = 6;
        c.constitution = 15;
        c.life_force = 15;
    });
    a
}

/// capture.md 10.41 item 4 (rr37 `ad-sp`, `ad-ob`, `ad-cs`; movement 6.3,
/// creatures 5.4): T (Speed 12) through the portal onto the side room's
/// 2,4 holding a space "invoke spell" (-5), an obstacle that bumps for -5
/// or a custom space open only to owners of the blade (+5): T stood on 2,4,
/// its Life Force unchanged, each time (`ad-*-a`). `ad-*-m` (2 states
/// each): FF followed through the portal on its first turn in 6 of 6
/// runs, landed on 2,4 with its Life Force unchanged; its next turn began
/// with its Speed 6 less the steps it took to the portal (3 in 5 runs, 1
/// in the sixth). The following is a sample; the rest exact.
#[test]
fn arriving_by_a_room_door_triggers_nothing_and_a_creature_carries_its_moves() {
    let quiet = |ev: &[Event]| {
        !ev.iter().any(|e| {
            matches!(
                e,
                Event::SpellTriggered { .. }
                    | Event::Given { .. }
                    | Event::StatChanged { .. }
                    | Event::CreatureBumped { .. }
            )
        })
    };
    let (mut n, mut followed) = (0, 0);
    for case in ["ad-sp", "ad-ob", "ad-cs"] {
        let a = arrival(case);
        for seed in 0..SEEDS / 16 {
            let mut g = start(&a, seed, tee(12));
            walk_in(&mut g);
            let mut ev = east(&mut g, 3);
            assert_eq!(g.players()[0].at, side(2, 4), "{case}");
            assert_eq!(life(&g), 40, "{case} seed {seed}");
            assert!(quiet(&ev), "{case} seed {seed}: {ev:?}");
            n += 1;
            // FF's first turn: the one in the round T went through.
            let first = turns_of(&ev, 0)
                .first()
                .unwrap_or_else(|| panic!("{case} seed {seed}: FF took no turn"))
                .to_vec();
            if !first
                .iter()
                .any(|e| matches!(e, Event::CreatureTookDoor { slot: 0, to, .. } if *to == side(2, 4)))
            {
                continue;
            }
            followed += 1;
            let steps = first
                .iter()
                .filter(|e| matches!(e, Event::CreatureMoved { slot: 0, .. }))
                .count() as u8;
            let ff = g.creature(1, 0).expect("FF");
            assert_eq!(
                (ff.room, ff.x, ff.y, ff.life_force),
                (SIDE, 2, 4, 15),
                "{case} seed {seed}"
            );
            // Its next turn: the moves left after the door's step.
            for _ in 0..4 {
                if turns_of(&ev, 0).len() >= 2 {
                    break;
                }
                ev.extend(wait(&mut g));
            }
            let Event::CreatureTurn { moves, .. } = turns_of(&ev, 0)[1][0] else {
                unreachable!()
            };
            assert_eq!(moves, 6 - steps - 1, "{case} seed {seed}: FF's carried moves");
        }
    }
    same_rate(
        "ad-*-m: FF through the portal on its first turn",
        Sweep::Sample,
        6,
        6,
        followed,
        n,
    );
}

/// capture.md 10.44 item 1 (rr40 `bump-m`, 256 states; creatures 5.1,
/// movement 5.5): T on the side room's 2,4, where the obstacle bumps for
/// -5, FF (T its goal) following it through the portal. 9 bumps in 8 runs,
/// every one on FF's first turn after following, the turn it began on 2,4
/// sharing T's square: 8 of those 231 turns bumped; in the 586 other turns
/// with FF beside 2,4 and a move left, never. Each bump is the -5 at once
/// with FF staying put, and FF never steps back onto 2,4 (10.41 item 3,
/// rr37 `ad-ob-m` run 1, the same bump). T's Speed 1 there only shortens
/// the rounds; here T waits out its turns. The rate per such first turn is
/// a sample; no bump on any other turn, exact.
#[test]
fn a_creature_bumps_the_obstacle_its_goal_stands_on() {
    let a = arrival("ad-ob");
    let (mut first_turns, mut first_bumped, mut others) = (0, 0, 0);
    for seed in 0..SEEDS / 4 {
        let mut g = start(&a, seed, tee(12));
        walk_in(&mut g);
        let mut ev = east(&mut g, 3);
        assert_eq!(g.players()[0].at, side(2, 4));
        for _ in 0..8 {
            if g.creature(1, 0).is_none() {
                break;
            }
            ev.extend(wait(&mut g));
        }
        assert_eq!(g.players()[0].at, side(2, 4), "seed {seed}");
        let beside = |l: Location| {
            matches!(l, Location::Room { region: 1, room: SIDE, x, y }
                if x.abs_diff(2) + y.abs_diff(4) == 1)
        };
        // FF's square, followed through its moves and door.
        let mut at = Location::Room {
            region: 1,
            room: ROOM,
            x: 5,
            y: 5,
        };
        let mut after_door = false;
        for turn in turns_of(&ev, 0) {
            let began = at;
            let first = after_door && began == side(2, 4);
            after_door = false;
            // The squares it chose a step from, but the last.
            let mut chose_beside = beside(began);
            let mut bumped = 0;
            for (i, e) in turn.iter().enumerate() {
                match *e {
                    Event::CreatureMoved { slot: 0, to, .. } => {
                        assert_ne!(to, side(2, 4), "seed {seed}: FF stepped back onto the obstacle");
                        at = to;
                        chose_beside |= beside(to) && i + 1 < turn.len();
                    }
                    Event::CreatureTookDoor { slot: 0, to, .. } => {
                        at = to;
                        after_door = to == side(2, 4);
                    }
                    Event::CreatureBumped { slot: 0, at: b, .. } => {
                        assert!(first && b == side(2, 4) && beside(at), "seed {seed}: {e:?}");
                        let change = turn[i + 1..]
                            .iter()
                            .find(|e| matches!(e, Event::StatChanged { .. }));
                        assert!(
                            matches!(
                                change,
                                Some(Event::StatChanged { who, stat: Stat::LifeForce, from, to })
                                    if *who == slot(0) && *to == from.saturating_sub(5)
                            ),
                            "seed {seed}: the bump's -5: {change:?}"
                        );
                        bumped += 1;
                    }
                    _ => {}
                }
            }
            if first {
                first_turns += 1;
                first_bumped += u64::from(bumped > 0);
            } else {
                assert_eq!(bumped, 0, "seed {seed}");
                others += u64::from(chose_beside);
            }
        }
    }
    println!("bump-m: {others} other turns with FF beside 2,4, no bump");
    same_rate(
        "bump-m: FF bumps on its first turn on the goal's obstacle square",
        Sweep::Sample,
        8,
        231,
        first_bumped,
        first_turns,
    );
}

/// Puts a pile of `amount` gold on ROOM (x, y).
fn gold_at(a: &mut Adventure, x: u8, y: u8, amount: u16) {
    a.regions[0].rooms[ROOM as usize].items.insert(
        0,
        RoomItem {
            x,
            y,
            thing: GOLD,
            parameter: 0,
            flag: false,
            destination: None,
            gold: Some(amount),
        },
    );
}

/// rr37's `w5`: aggressive neutrals of Speed 0, Wisdom 3, in the corners,
/// our obstacles on one square beside each so the other is the only open
/// one: NA (holding our blade) and NC (no weapon) beside a portal to the
/// side room on 1,2 and 1,7; NB (the blade) and ND (none) beside 50 gold on
/// 13,2 and 13,7. The player on 4,4 their goal.
fn w5() -> Adventure {
    const BLADE: u16 = 59;
    let mut a = closed();
    thing(&mut a, BLADE, "BLADE", blade(10));
    thing(&mut a, GOLD, "GOLD", treasure(1, 1));
    for (n, (x, y)) in [(1, 1), (13, 1), (1, 8), (13, 8)].into_iter().enumerate() {
        creature(&mut a, n, x, y, |c| {
            c.name = ["NA", "NB", "NC", "ND"][n].into();
            c.speed = 0;
            c.wisdom = 3;
            c.strategy.alignment = Alignment::Neutral;
            c.weapon = 0;
            c.possessions.clear();
            if n < 2 {
                owns(c, BLADE);
                c.weapon = BLADE;
            }
        });
    }
    for (x, y) in [(2, 1), (12, 1), (2, 8), (12, 8)] {
        item(&mut a, ROOM, x, y, OBSTACLE);
    }
    for (x, y) in [(1, 2), (1, 7)] {
        door_at(&mut a, x, y, DOOR, to_side());
    }
    for (x, y) in [(13, 2), (13, 7)] {
        gold_at(&mut a, x, y, 50);
    }
    a
}

/// capture.md 10.41 item 5 (rr37 `w5-m2`, 16 states, 40M; creatures 2.7):
/// every one of the four went through the portal or took the gold from
/// beside on its first turn, 64 of 64, its square unchanged by the take.
/// Owning a melee weapon changes nothing. Exact.
#[test]
fn a_speed_0_fighter_takes_the_exit_or_gold_beside_it() {
    let a = w5();
    for seed in 0..16 {
        let mut g = game(&a, seed, tee(6), room(4, 4));
        let ev = wait(&mut g);
        for (n, corner) in [(0, (1, 1)), (1, (13, 1)), (2, (1, 8)), (3, (13, 8))] {
            let turn = turns_of(&ev, n)[0];
            let ok = if corner.0 == 1 {
                turn.iter()
                    .any(|e| matches!(e, Event::CreatureTookDoor { slot, .. } if *slot == n))
            } else {
                let c = g.creature(1, n).expect("alive");
                assert_eq!(
                    (c.x, c.y, c.wealth),
                    (corner.0, corner.1, 50),
                    "seed {seed}: slot {n}"
                );
                turn.iter()
                    .any(|e| matches!(e, Event::CreatureGoldPickedUp { slot, amount: 50, .. } if *slot == n))
            };
            assert!(ok, "w5-m2 seed {seed}: slot {n}: {turn:?}");
        }
    }
}

/// capture.md 10.41 item 6 (creatures 6.5). `w5-m2`'s NE, an aggressive
/// neutral of Speed 0 on 8,4 with 50 gold on 7,4 between it and T on 4,4,
/// its goal (gold rank 96, cost 3): 16 runs of two turns, the gold taken in
/// 14. `g1-m` (16 states): TH, a peaceful thief of Speed 0 on 8,4, the gold
/// on 7,4 its goal (cost 1): taken on its first turn 16 of 16, exact.
#[test]
fn gold_costs_its_rank_on_the_first_step() {
    let mut a = closed();
    thing(&mut a, GOLD, "GOLD", treasure(1, 1));
    gold_at(&mut a, 7, 4, 50);
    let took = |ev: &[Event]| {
        ev.iter().any(|e| {
            matches!(
                e,
                Event::CreatureGoldPickedUp {
                    slot: 0,
                    amount: 50,
                    ..
                }
            )
        })
    };
    let mut ne = a.clone();
    creature(&mut ne, 0, 8, 4, |c| {
        c.name = "NE".into();
        c.speed = 0;
        c.wisdom = 3;
        c.strategy.alignment = Alignment::Neutral;
    });
    let runs = SEEDS / 4;
    let eng = count(runs, |seed| {
        let mut g = game(&ne, seed, tee(6), room(4, 4));
        let mut ev = Vec::new();
        for _ in 0..4 {
            if turns_of(&ev, 0).len() >= 2 {
                break;
            }
            ev.extend(wait(&mut g));
        }
        turns_of(&ev, 0).iter().take(2).any(|t| took(t))
    });
    same_rate(
        "w5-m2: NE takes the gold in two turns",
        Sweep::Sample,
        14,
        16,
        eng,
        runs,
    );

    let mut th = a;
    creature(&mut th, 0, 8, 4, |c| {
        c.name = "TH".into();
        c.speed = 0;
        c.wisdom = 3;
        c.strategy.alignment = Alignment::Thief;
        c.strategy.aggressive = false;
    });
    for seed in 0..16 {
        let mut g = game(&th, seed, tee(6), room(4, 4));
        let ev = wait(&mut g);
        assert!(took(turns_of(&ev, 0)[0]), "g1-m seed {seed}: {ev:?}");
        let c = g.creature(1, 0).expect("TH");
        assert_eq!((c.x, c.y, c.wealth), (8, 4, 50), "g1-m seed {seed}");
    }
}

// ---- capture.md 10.43: a creature's take and its turn start ---------------

/// The first `n` turns of slot 0, T waiting: up to `4 * n` waits.
fn first_turns(g: &mut Game, n: usize) -> Vec<Vec<Event>> {
    let mut ev = Vec::new();
    for _ in 0..4 * n {
        if turns_of(&ev, 0).len() >= n {
            break;
        }
        ev.extend(wait(g));
    }
    turns_of(&ev, 0).into_iter().take(n).map(|t| t.to_vec()).collect()
}

fn creature_moves(turn: &[Event]) -> u8 {
    match turn[0] {
        Event::CreatureTurn { moves, .. } => moves,
        _ => unreachable!(),
    }
}

/// capture.md 10.43 item 8 (rr39 `ct-m`, 8 states, 30M; inventory 2.7):
/// TH, a peaceful thief of Speed 0 on 8,4, our treasure 60 on 7,4 placed
/// with 0, 61 on 9,4 placed with 1. Every run: two takes, both things gone
/// from the floor, only 61 owned; 6 runs took 60 first, 2 took 61 first.
/// The takes exact; which first, a sample.
#[test]
fn a_creatures_take_of_a_thing_placed_with_0_owns_nothing() {
    const PUTCI: u16 = 60;
    const PUTGI: u16 = 61;
    let mut a = closed();
    thing(&mut a, PUTCI, "PUTCI", treasure(1, 1));
    thing(&mut a, PUTGI, "PUTGI", treasure(1, 1));
    // The room's list as the case wrote it, 60 before 61 (the goal's ties
    // go to the earlier entry, creatures 6.2).
    item(&mut a, ROOM, 9, 4, PUTGI);
    item(&mut a, ROOM, 7, 4, PUTCI);
    a.regions[0].rooms[ROOM as usize].items[0].parameter = 0;
    creature(&mut a, 0, 8, 4, |c| {
        c.name = "TH".into();
        c.speed = 0;
        c.strategy.alignment = Alignment::Thief;
        c.strategy.aggressive = false;
    });
    let runs = SEEDS / 8;
    let first_60 = count(runs, |seed| {
        let mut g = game(&a, seed, tee(6), room(4, 4));
        let mut took = Vec::new();
        for _ in 0..8 {
            for e in wait(&mut g) {
                if let Event::CreaturePickedUp { slot: 0, thing, .. } = e {
                    took.push(thing);
                }
            }
            if took.len() >= 2 {
                break;
            }
        }
        assert!(
            took.len() == 2 && took.contains(&PUTCI) && took.contains(&PUTGI),
            "seed {seed}: {took:?}"
        );
        let c = g.creature(1, 0).expect("TH");
        let n = |t| acs_engine::creatures::count(c, t);
        assert_eq!((n(PUTCI), n(PUTGI)), (0, 1), "seed {seed}");
        assert!(
            !g.adventure().regions[0].rooms[ROOM as usize]
                .items
                .iter()
                .any(|i| matches!((i.x, i.y), (7, 4) | (9, 4))),
            "seed {seed}: both gone from the floor"
        );
        took[0] == PUTCI
    });
    same_rate("ct-m: 60 taken first", Sweep::Sample, 6, 8, first_60, runs);
}

/// capture.md 10.43 item 9 (rr39 `cw-m`, 16 states, 40M, `cw-m2`, 96, 10M;
/// inventory 3.4, creatures 2.5, magic 5.1): CW, a peaceful friend of
/// Speed 10, Strength 10, Constitution 20, Life Force 14, carrying a
/// treasure of weight 600 (capacity 700 at Life Force 14: 2 moves; 800 at
/// 15: 5). Every first turn began with 2 moves, 112 of 112; CW dropped the
/// treasure as its first action and stepped once, and its next turn began
/// with 10. The top-of-turn die healed it 14 to 15 in 23 of `cw-m2`'s 96
/// and 1 of `cw-m`'s 16, the moves set to 2 before the heal each time. The
/// moves exact; the heals a sample.
#[test]
fn a_creatures_moves_are_set_before_its_turn_top_die() {
    const LOAD: u16 = 50;
    let mut a = closed();
    thing(&mut a, LOAD, "LOAD", treasure(600, 1));
    ours(&mut a, 0, "CW", 12, 7, |c| {
        c.speed = 10;
        c.strength = 10;
        c.constitution = 20;
        c.life_force = 14;
        owns(c, LOAD);
    });
    let runs = SEEDS / 4;
    let healed = count(runs, |seed| {
        let mut g = game(&a, seed, tee(6), room(4, 4));
        // CW's first turn comes in T's first wait; its Life Force after it
        // is the die's alone (both moves used: no idle roll).
        let mut ev = wait(&mut g);
        assert_eq!(turns_of(&ev, 0).len(), 1, "seed {seed}");
        let healed = match g.creature(1, 0).expect("CW").life_force {
            14 => false,
            15 => true,
            lf => panic!("seed {seed}: Life Force {lf}"),
        };
        for _ in 0..4 {
            if turns_of(&ev, 0).len() >= 2 {
                break;
            }
            ev.extend(wait(&mut g));
        }
        let turns = turns_of(&ev, 0);
        assert!(turns.len() >= 2, "seed {seed}: CW's turns");
        let first = turns[0];
        assert_eq!(creature_moves(first), 2, "seed {seed}: {first:?}");
        assert!(
            matches!(
                first[1],
                Event::CreatureDropped {
                    slot: 0,
                    thing: LOAD,
                    ..
                }
            ),
            "seed {seed}: the drop first: {first:?}"
        );
        let steps = first
            .iter()
            .filter(|e| matches!(e, Event::CreatureMoved { slot: 0, .. }))
            .count();
        assert_eq!(steps, 1, "seed {seed}: {first:?}");
        assert_eq!(creature_moves(turns[1]), 10, "seed {seed}");
        healed
    });
    same_rate(
        "cw-m, cw-m2: first turns healed at the top",
        Sweep::Sample,
        24,
        112,
        healed,
        runs,
    );
}

// ---- capture.md 10.44: fighter or slinker, the load, carried moves --------

/// capture.md 10.44 item 2 (rr40 `fsw-m`, `fsw-set`, `fsw2-m`, `fsw2-set`,
/// `fsb-m`, 4 states each, 30M, `--answer T`; `fs-*`, 15 runs; creatures
/// 4.2): EE, an aggressive cautious enemy of Speed 3, Constitution 20, on
/// 5,4 beside T (Speed 1), owning and holding our blade of power 3. Built
/// unwounded (Life Force 20): it struck T in 4 of 4 runs; set to 10 at the
/// run's start, after the visit began: 0 of 4, walking off. Built wounded
/// (10): 0 of 4; set to 20 at the start: 4 of 4. Brave and unwounded: 4 of
/// 4. With no weapon none ever struck. Over EE's first four turns here;
/// exact.
#[test]
fn fighter_or_slinker_is_worked_out_afresh_each_turn() {
    const BLADE: u16 = 59;
    for armed in [true, false] {
        for (name, built, set, brave, strikes) in [
            ("fsw-m", 20, None, false, true),
            ("fsw-set", 20, Some(10), false, false),
            ("fsw2-m", 10, None, false, false),
            ("fsw2-set", 10, Some(20), false, true),
            ("fsb-m", 20, None, true, true),
        ] {
            let mut a = closed();
            thing(&mut a, BLADE, "BLADE", blade(3));
            creature(&mut a, 0, 5, 4, |c| {
                c.name = "EE".into();
                c.speed = 3;
                c.constitution = 20;
                c.life_force = built;
                c.strategy.brave = brave;
                c.weapon = 0;
                c.possessions.clear();
                if armed {
                    owns(c, BLADE);
                    c.weapon = BLADE;
                }
            });
            for seed in 0..16 {
                let mut g = game(&a, seed, tee(1), room(4, 4));
                if let Some(lf) = set {
                    g.creature_mut(1, 0).expect("EE").life_force = lf;
                }
                let turns = first_turns(&mut g, 4);
                assert_eq!(turns.len(), 4, "{name} seed {seed}");
                let struck = turns
                    .iter()
                    .flatten()
                    .any(|e| matches!(e, Event::Attacked { by, target: ME, .. } if *by == slot(0)));
                assert_eq!(struck, strikes && armed, "{name} armed {armed} seed {seed}");
            }
        }
    }
}

/// capture.md 10.44 item 3 (rr40 `wt-m`, 8 states, 40M; `wt2-m`, 4 states,
/// 60M; creatures 2.5, 8.1): peaceful friends of Speed 10, Strength 10,
/// Constitution 20: CA carrying 700 (Life Force 20, capacity 1,000), CB
/// 500, CC 700 at Life Force 18 (capacity 900), CD 700 gold and nothing
/// else (Gold's weight 1). The first turn's moves CA 6, CB 10, CC 4, CD 6
/// in 8 of 8 runs each, their next turns 10. CA, CC and CD dropped at the
/// top of the first turn, CD its gold as one pile, its gold going to 0; CB
/// did not. Exact.
#[test]
fn a_creatures_bar_counts_its_load_and_its_gold() {
    const HEAVY: u16 = 50;
    const LIGHT: u16 = 51;
    let mut a = closed();
    thing(&mut a, HEAVY, "HEAVY", treasure(700, 1));
    thing(&mut a, LIGHT, "LIGHT", treasure(500, 1));
    thing(&mut a, GOLD, "GOLD", treasure(1, 1));
    // (name, square, Life Force, the thing carried, gold, first turn's moves)
    type Case = (&'static str, (u8, u8), u8, Option<u16>, u16, u8);
    let cases: [Case; 4] = [
        ("CA", (12, 1), 20, Some(HEAVY), 0, 6),
        ("CB", (12, 8), 20, Some(LIGHT), 0, 10),
        ("CC", (1, 8), 18, Some(HEAVY), 0, 4),
        ("CD", (1, 1), 20, None, 700, 6),
    ];
    for (n, &(name, (x, y), lf, load, gold, _)) in cases.iter().enumerate() {
        ours(&mut a, n, name, x, y, |c| {
            c.speed = 10;
            c.strength = 10;
            c.constitution = 20;
            c.life_force = lf;
            c.wealth = gold;
            c.possessions.clear();
            if let Some(t) = load {
                owns(c, t);
            }
        });
    }
    for seed in 0..32 {
        let mut g = game(&a, seed, tee(6), room(4, 4));
        let mut ev = Vec::new();
        for _ in 0..8 {
            if (0..4).all(|n| turns_of(&ev, n).len() >= 2) {
                break;
            }
            ev.extend(wait(&mut g));
        }
        for (n, &(name, _, _, load, gold, moves)) in cases.iter().enumerate() {
            let turns = turns_of(&ev, n);
            assert!(turns.len() >= 2, "{name} seed {seed}");
            assert_eq!(creature_moves(turns[0]), moves, "{name} seed {seed}");
            assert_eq!(creature_moves(turns[1]), 10, "{name} seed {seed}");
            let first = &turns[0][1];
            let over = moves < 10;
            let dropped = match (load, gold) {
                (_, 700) => matches!(
                    first,
                    Event::CreatureDropped {
                        thing: GOLD,
                        amount: 700,
                        ..
                    }
                ),
                (Some(t), _) => matches!(first, Event::CreatureDropped { thing, .. } if *thing == t),
                _ => unreachable!(),
            };
            assert_eq!(dropped, over, "{name} seed {seed}: {:?}", turns[0]);
        }
    }
}

/// rr40's `arrive` rooms (as rr37's `ad-*`, the side portal on 7,4 to the
/// side room's 2,4): FF, a peaceful friend of `speed`, on ROOM (x, y).
fn arrive(speed: u8, at: (u8, u8)) -> Adventure {
    let mut a = with_portal(DOOR, to_side());
    ours(&mut a, 0, "FF", at.0, at.1, |c| {
        c.speed = speed;
        c.constitution = 15;
        c.life_force = 15;
    });
    a
}

/// EE of rr40 `df`, `fh`: an aggressive enemy of Speed 0, no weapon, in
/// slot 1 on `room` (x, y).
fn still_enemy(a: &mut Adventure, room: u8, x: u8, y: u8) {
    creature(a, 1, x, y, |c| {
        c.name = "EE".into();
        c.room = room;
        c.speed = 0;
        c.wisdom = 3;
        c.weapon = 0;
        c.possessions.clear();
    });
}

/// capture.md 10.44 item 4 (rr40 `dz`, `d1`, `df`, `fh`, `arrive.txt`,
/// `run mem` from T's arrival through the side portal, 30M; creatures 2.6,
/// 5.4): FF following T through the portal on its first turn. `dz` (Speed
/// 2 on 6,5, the portal its second step): followed in 3 of 4 runs, its
/// next turn beginning with 2, its full bar. `d1` (Speed 3 on 6,5): 4 of 4
/// followed with 1 left and began with 1. `df` (Speed 6 on 5,5, EE an
/// aggressive enemy of Speed 0 with no weapon on the side room's 2,5
/// beside the arrival square): 6 of 8 followed and began beside EE with
/// its carried moves, 3 (5 runs) or 1 (1), not halved. `fh`: the same FF
/// beside the same kind of enemy on its first turn, nothing carried: 3 of
/// 6 (2 of 2). The moves exact; the following a sample.
#[test]
fn a_creatures_carried_moves_are_not_halved_and_none_left_is_a_full_bar() {
    for (name, speed, at, enemy, cap_k, cap_n) in [
        ("dz", 2, (6, 5), false, 3, 4),
        ("d1", 3, (6, 5), false, 4, 4),
        ("df", 6, (5, 5), true, 6, 8),
    ] {
        let mut a = arrive(speed, at);
        if enemy {
            still_enemy(&mut a, SIDE, 2, 5);
        }
        let runs = SEEDS / 16;
        let followed = count(runs, |seed| {
            let mut g = start(&a, seed, tee(12));
            walk_in(&mut g);
            let mut ev = east(&mut g, 3);
            assert_eq!(g.players()[0].at, side(2, 4), "{name}");
            let first = turns_of(&ev, 0)
                .first()
                .unwrap_or_else(|| panic!("{name} seed {seed}: FF took no turn"))
                .to_vec();
            if !first
                .iter()
                .any(|e| matches!(e, Event::CreatureTookDoor { slot: 0, to, .. } if *to == side(2, 4)))
            {
                return false;
            }
            let steps = first
                .iter()
                .filter(|e| matches!(e, Event::CreatureMoved { slot: 0, .. }))
                .count() as u8;
            let ff = g.creature(1, 0).expect("FF");
            assert_eq!((ff.room, ff.x, ff.y), (SIDE, 2, 4), "{name} seed {seed}");
            for _ in 0..4 {
                if turns_of(&ev, 0).len() >= 2 {
                    break;
                }
                ev.extend(wait(&mut g));
            }
            let left = speed - steps - 1;
            let want = if left == 0 { speed } else { left };
            assert_eq!(
                creature_moves(turns_of(&ev, 0)[1]),
                want,
                "{name} seed {seed}: FF's carried moves"
            );
            true
        });
        same_rate(
            &format!("{name}: FF through the portal on its first turn"),
            Sweep::Sample,
            cap_k,
            cap_n,
            followed,
            runs,
        );
    }
    // `fh`
    let mut a = arrive(6, (5, 5));
    still_enemy(&mut a, ROOM, 5, 6);
    for seed in 0..16 {
        let mut g = game(&a, seed, tee(6), room(4, 4));
        let turns = first_turns(&mut g, 1);
        assert_eq!(creature_moves(&turns[0]), 3, "fh seed {seed}");
    }
}
