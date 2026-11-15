// SPDX-License-Identifier: GPL-3.0-or-later

//! Creatures and turn order (creatures 2-3, magic 5): who moves when, the
//! visits, doors and the moves they carry, the adjacency slowdown,
//! following through a door, random creatures appearing and falling away,
//! the catch-up, Power drift and the idle healing roll's range.
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
    Access, Alignment, Creature, Destination, RoomItem, SpellAction, SpellActionKind, ThingKind,
};
use acs_core::{Adventure, Platform};
use acs_engine::{Command, Dir, Event, Game, Location, PlayerSetup, RemovalReason, Rules};

/// A two-way portal (10.29 `two`).
const TWO_WAY: u16 = 83;
const SPELL: u16 = 33;
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
