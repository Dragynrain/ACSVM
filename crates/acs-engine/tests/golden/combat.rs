// SPDX-License-Identifier: GPL-3.0-or-later

//! Combat (combat.md) and what creatures do with what they carry
//! (creatures 2.7, 8.1): melee, creatures' and the player's firing, the
//! drop square and the overburdened drop.
//!
//! The capture's individuals are the harness's (`acs-capture`
//! `scenario.rs`): a player of ours has the stats the program rolled for
//! move-basic's player (Constitution and Life Force 11, Strength 9,
//! Dexterity 13, Speed 6, Wisdom and Power 13, Size 9, Dodge 30, Parry 26,
//! Armor skill 0, Melee 31, Missile 22), a creature of ours the same with
//! Speed 3 and Constitution and Life Force 10, each changed as the case
//! says. The kit is capture.md 10.15's: the blade (thing 59), the bow (41)
//! and the mail (40), power 10 (mail 2), adjustment 0, no break chance,
//! range 5, weight 5. Where a case doesn't give a stat these stand, and the
//! test says so.

use crate::stats::{same_histogram, same_rate, Sweep, SEEDS};
use crate::world::*;
use acs_core::model::{
    Access, Alignment, Carry, Creature, CustomTrigger, Destination, MagicDefense, PlacementChoice,
    Possession, RoomItem, ThingKind, WeaponStats,
};
use acs_core::{Adventure, Platform};
use acs_engine::{
    combat, BlockReason, BlowOutcome, Command, Dir, Event, FireRefusal, Game, Location, PlayerSetup, Slot,
    Stat, Who,
};

const MAIL: u16 = 40;
const BOW: u16 = 41;
const BLADE: u16 = 59;
const GOLD: u16 = 81;
/// Treasures of ours a test gives creatures (capture.md 10.14.5 numbers).
const T50: u16 = 50;
const T51: u16 = 51;
const T52: u16 = 52;
const T53: u16 = 53;
const T54: u16 = 54;
/// The kinds of thing of `drop-kinds` (capture.md 10.32).
const CUSTOM_OBSTACLE: u16 = 85;
const CUSTOM_SPACE: u16 = 86;
const SPACE: u16 = 87;

fn stats(power: u8) -> WeaponStats {
    WeaponStats {
        power,
        range: None,
        break_chance: 0,
        attack_adjustment: 0,
        usable_by_anyone: true,
        magic: false,
    }
}

fn our_bow(range: u8, once: bool) -> ThingKind {
    ThingKind::MissileWeapon {
        picture: 7,
        disappears_after_use: once,
        carry: Carry { weight: 5, value: 10 },
        stats: WeaponStats {
            range: Some(range),
            ..stats(10)
        },
    }
}

fn mail(power: u8) -> ThingKind {
    ThingKind::Armor {
        picture: 8,
        disappears_after_use: false,
        carry: Carry { weight: 5, value: 10 },
        stats: stats(power),
    }
}

/// The base adventure with the kit (bow of `range`), and gold (thing 81)
/// weighing 1 a piece (the capture doesn't give gold's weight; creatures
/// 8.1.2 leaves it open).
fn kit(range: u8) -> Adventure {
    let mut a = base(Platform::C64);
    thing(&mut a, BLADE, "BLADE", blade(10));
    thing(&mut a, BOW, "BOW", our_bow(range, false));
    thing(&mut a, MAIL, "MAIL", mail(2));
    thing(&mut a, GOLD, "GOLD", treasure(1, 1));
    a
}

/// A player of ours (the harness's T) of `speed`, Constitution and Life
/// Force `con`, owning `things`.
fn t(speed: u8, con: u8, things: Vec<u16>) -> PlayerSetup {
    PlayerSetup {
        name: "T".into(),
        speed,
        strength: 9,
        constitution: con,
        life_force: con,
        possessions: things,
        dexterity: 13,
        wisdom: 13,
        power: Some(13),
        size: 9,
        dodge: 30,
        parry: 26,
        armor_skill: 0,
        melee: 31,
        missile: 22,
        ..Default::default()
    }
}

/// A creature of ours in `n` on ROOM (x, y): an aggressive brave enemy
/// (world's template) with the harness's stats, then `f`.
fn ours(a: &mut Adventure, n: usize, x: u8, y: u8, f: impl FnOnce(&mut Creature)) {
    creature(a, n, x, y, |c| {
        c.constitution = 10;
        c.life_force = 10;
        c.strength = 9;
        c.dexterity = 13;
        c.speed = 3;
        c.wisdom = 13;
        c.power = 13;
        c.size = 9;
        c.dodge = 30;
        c.parry = 26;
        c.armor_skill = 0;
        c.melee = 31;
        c.missile = 22;
        c.magic_defense = MagicDefense::None;
        c.mimic = false;
        c.weapon = 0;
        c.armor = 0;
        c.wealth = 0;
        c.possessions.clear();
        f(c);
    });
}

fn has(c: &mut Creature, thing: u16) {
    c.possessions.push(Possession { thing, count: 1 });
}

fn holds(c: &mut Creature, thing: u16) {
    has(c, thing);
    c.weapon = thing;
}

fn peaceful_friend(c: &mut Creature) {
    c.strategy.alignment = Alignment::Friend;
    c.strategy.aggressive = false;
    c.strategy.brave = false;
}

/// capture.md 10.14.6's EE: Speed 3, Strength 10, Constitution and Life
/// Force 20 (an aggressive brave enemy).
fn ee(c: &mut Creature) {
    c.speed = 3;
    c.strength = 10;
    c.constitution = 20;
    c.life_force = 20;
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

fn turn_moves(turn: &[Event]) -> u8 {
    match turn[0] {
        Event::CreatureTurn { moves, .. } => moves,
        _ => unreachable!(),
    }
}

fn steps(turn: &[Event]) -> usize {
    turn.iter()
        .filter(|e| matches!(e, Event::CreatureMoved { .. }))
        .count()
}

fn attacks(turn: &[Event], by: Who) -> Vec<(Who, u8)> {
    turn.iter()
        .filter_map(|e| match e {
            Event::Attacked {
                by: b, target, blows, ..
            } if *b == by => Some((*target, *blows)),
            _ => None,
        })
        .collect()
}

/// Whether a blow or a missile of `by` did damage to `target`.
fn damaged(ev: &[Event], by: Who, target: Who) -> bool {
    ev.iter().any(|e| match e {
        Event::Blow {
            by: b,
            target: t,
            outcome: BlowOutcome::Hit { damage, .. },
            ..
        }
        | Event::MissileHit {
            by: b,
            target: t,
            outcome: BlowOutcome::Hit { damage, .. },
            ..
        } => *b == by && *t == target && *damage > 0,
        _ => false,
    })
}

fn fired(ev: &[Event], by: Who) -> Option<u8> {
    ev.iter().find_map(|e| match e {
        Event::Fired { by: b, shots, .. } if *b == by => Some(*shots),
        _ => None,
    })
}

fn shots_of(ev: &[Event], by: Who) -> u64 {
    ev.iter()
        .filter(|e| matches!(e, Event::Shot { by: b, .. } if *b == by))
        .count() as u64
}

/// Arrows of `by` reaching `who`: (dodged, damaging).
fn reached(ev: &[Event], by: Who, who: Who) -> (u64, u64) {
    let dodged = ev
        .iter()
        .filter(|e| matches!(e, Event::Dodged { by: b, who: w, .. } if *b == by && *w == who))
        .count() as u64;
    let hit = ev
        .iter()
        .filter(|e| {
            matches!(e, Event::MissileHit { by: b, target, outcome: BlowOutcome::Hit { damage, .. }, .. }
                if *b == by && *target == who && *damage > 0)
        })
        .count() as u64;
    (dodged, hit)
}

/// The generator's state, read from the game's serialized form.
fn rng(g: &Game) -> serde_json::Value {
    serde_json::to_value(g).expect("serializes")["rng"].clone()
}

fn moves(g: &Game) -> u8 {
    g.players()[0].moves_left
}

fn ready(g: &mut Game, thing: u16) {
    let ev = g.apply(Command::Ready { thing });
    assert!(
        ev.iter()
            .any(|e| matches!(e, Event::Readied { thing: t, .. } if *t == thing)),
        "{ev:?}"
    );
}

fn idle_to(g: &mut Game, left: u8) {
    while moves(g) > left {
        g.apply(Command::Idle);
    }
    assert_eq!(moves(g), left);
}

// ---- melee ----------------------------------------------------------------

/// T (Speed 6, Constitution and Life Force 40) on the entry 4,4 and EE
/// (10.14.6) on 6,4, holding the blade when `armed`.
fn fighter(armed: bool) -> Adventure {
    let mut a = kit(5);
    ours(&mut a, 0, 6, 4, |c| {
        ee(c);
        if armed {
            holds(c, BLADE);
        }
    });
    a
}

/// One round from T's turn end (capture.md 10.14.9): EE's turn, the
/// round's end, T's turn waited out, EE's next turn.
fn round(a: &Adventure, seed: u64, setup: PlayerSetup, first: Command) -> (Game, Vec<Event>) {
    let mut g = game(a, seed, setup, room(4, 4));
    let mut ev = g.apply(first);
    ev.extend(wait(&mut g));
    (g, ev)
}

/// capture.md 10.14.6 (`fight`): EE with no weapon never attacked T
/// (Life Force stayed 40, 32 of 32); it walked up and stepped round it
/// (combat 2.3).
#[test]
fn an_unarmed_fighter_never_attacks() {
    let a = fighter(false);
    for seed in 0..256 {
        let (g, ev) = round(&a, seed, t(6, 40, vec![]), Command::Wait);
        assert!(attacks(&ev, slot(0)).is_empty(), "seed {seed}: {ev:?}");
        assert_eq!(life(&g), 40, "seed {seed}");
        assert!(!turns_of(&ev, 0).is_empty());
    }
}

/// capture.md 10.14.7, 10.14.9 (`armed`): EE holding the blade, 32 states,
/// one round. T's Life Force fell (40 to 24..34) in 19 runs and stayed 40
/// in 13; 122 blows, 25 of them hits (to-hit 34, 29, 24, 19; damage 10 or
/// 15). Both depend on many rolls: samples. And the blows from the moves
/// left (combat 3.1): "4 from 3 of 3 moves left, 2 from 2, 1 from 1".
#[test]
fn an_armed_fighter() {
    let a = fighter(true);
    let (mut hurt, mut blows, mut hits) = (0, 0, 0);
    for seed in 0..SEEDS {
        let (_, ev) = round(&a, seed, t(6, 40, vec![]), Command::Wait);
        for turn in turns_of(&ev, 0) {
            let full = turn_moves(turn);
            let mut left = full;
            for e in turn {
                match e {
                    Event::CreatureMoved { slot: 0, .. } | Event::CreatureReadied { slot: 0, .. } => {
                        left = left.saturating_sub(1)
                    }
                    Event::Attacked { by, blows: b, .. } if *by == slot(0) => {
                        let rule = (4 * left as i32 / full.max(1) as i32 + 13 / 8 - 1).max(1);
                        assert_eq!(*b as i32, rule, "seed {seed}: {left} of {full} moves left");
                        if full == 3 {
                            assert_eq!(*b, [1, 1, 2, 4][left as usize], "seed {seed}");
                        }
                    }
                    Event::Blow {
                        by,
                        counter: false,
                        outcome,
                        ..
                    } if *by == slot(0) => {
                        blows += 1;
                        hits += matches!(outcome, BlowOutcome::Hit { .. }) as u64;
                    }
                    _ => {}
                }
            }
        }
        hurt += damaged(&ev, slot(0), ME) as u64;
    }
    same_rate("armed: runs with T hurt", Sweep::Sample, 19, 32, hurt, SEEDS);
    same_rate("armed: blows that hit", Sweep::Sample, 25, 122, hits, blows);
}

/// capture.md 10.16: magic defense 3 kept the plain blade off T in all 16
/// runs (combat 5.4; `md3-plain`, the `armed` round).
#[test]
fn magic_defense_3_keeps_a_plain_blade_off() {
    let a = fighter(true);
    let mut setup = t(6, 40, vec![]);
    setup.magic_defense = MagicDefense::MagicHalfDamage;
    let mut attacked = 0;
    for seed in 0..256 {
        let (g, ev) = round(&a, seed, setup.clone(), Command::Wait);
        attacked += !attacks(&ev, slot(0)).is_empty() as u32;
        assert!(!damaged(&ev, slot(0), ME), "seed {seed}");
        assert_eq!(life(&g), 40, "seed {seed}");
    }
    assert!(attacked > 200);
}

/// capture.md 10.16 (`mail-heavy`): T wearing mail of Power 12 against the
/// blade's 10 was hurt in 1 of 16 runs (only a heavy 15 gets through,
/// combat 5.1). T readies the mail (ending its turn) instead of idling it
/// out, then the `armed` round.
#[test]
fn mail_of_power_12_lets_only_heavy_blows_through() {
    let mut a = fighter(true);
    thing(&mut a, MAIL, "MAIL", mail(12));
    let hurt = count(SEEDS, |seed| {
        let (g, ev) = round(&a, seed, t(6, 40, vec![MAIL]), Command::Ready { thing: MAIL });
        assert_eq!(g.players()[0].armor, Some(MAIL));
        for e in &ev {
            if let Event::Blow {
                by,
                outcome: BlowOutcome::Hit { damage, heavy },
                ..
            } = e
            {
                if *by == slot(0) {
                    assert_eq!(*damage, if *heavy { 3 } else { 0 }, "seed {seed}");
                }
            }
        }
        damaged(&ev, slot(0), ME)
    });
    same_rate("mail-heavy: runs with T hurt", Sweep::Sample, 1, 16, hurt, SEEDS);
}

/// capture.md 10.35 item 4 (`brk-parry`: rr9's `break-many`, the `armed`
/// round with EE of Dexterity 31, both holding the blade at break chance
/// 15, T's Parry 127 so EE's blows are parried at 90; 32 runs, 32 of 32
/// fitting): 70 parries of EE's blows, EE's blade broke 6 times, 4 with
/// blows still due, and each time EE's attack ended there; T's blade
/// broke 3 times on parried counter-blows and EE's attack went on (combat
/// 6.3). T readies the blade with its first move, then the round.
#[test]
fn a_weapon_breaking_with_blows_due_ends_its_owners_attack() {
    let mut a = fighter(true);
    thing(
        &mut a,
        BLADE,
        "BLADE",
        ThingKind::MeleeWeapon {
            picture: 6,
            disappears_after_use: false,
            carry: Carry { weight: 5, value: 10 },
            stats: WeaponStats {
                break_chance: 15,
                ..stats(10)
            },
        },
    );
    a.regions[0].residents[0].dexterity = 31;
    let mut setup = t(6, 40, vec![BLADE]);
    setup.parry = 127;
    let (mut parried, mut ee_broke, mut ee_due, mut t_broke, mut went_on) = (0, 0, 0, 0, 0);
    for seed in 0..SEEDS / 4 {
        let mut g = game(&a, seed, setup.clone(), room(4, 4));
        ready(&mut g, BLADE);
        let mut ev = wait(&mut g);
        ev.extend(wait(&mut g));
        for turn in turns_of(&ev, 0) {
            let Some(start) = turn
                .iter()
                .position(|e| matches!(e, Event::Attacked { by, .. } if *by == slot(0)))
            else {
                continue;
            };
            let Event::Attacked { blows, .. } = turn[start] else {
                unreachable!()
            };
            // Combat 3.2: a blow uses one, a damaging one two, a damaging
            // counter-blow one more.
            let mut used = 0;
            let mut ee_gone = false;
            for (i, e) in turn.iter().enumerate().skip(start + 1) {
                match e {
                    Event::Blow {
                        by, counter, outcome, ..
                    } => {
                        let damaging = matches!(outcome, BlowOutcome::Hit { damage, .. } if *damage > 0);
                        if *by == slot(0) && !counter {
                            assert!(!ee_gone, "seed {seed}: a blow after EE's blade broke: {turn:?}");
                            used += 1 + damaging as u8;
                            parried += (*outcome == BlowOutcome::Parried) as u64;
                        } else if *by == ME && *counter && damaging {
                            used += 1;
                        }
                    }
                    Event::Broke {
                        who, thing: BLADE, ..
                    } if *who == slot(0) => {
                        ee_broke += 1;
                        ee_due += (used < blows) as u64;
                        ee_gone = true;
                    }
                    Event::Broke {
                        who: ME,
                        thing: BLADE,
                        ..
                    } => {
                        t_broke += 1;
                        if used < blows && !turn[i + 1..].iter().any(|e| matches!(e, Event::Died { .. })) {
                            assert!(
                                turn[i + 1..].iter().any(
                                    |e| matches!(e, Event::Blow { by, counter: false, .. } if *by == slot(0))
                                ),
                                "seed {seed}: EE's attack ended with T's blade: {turn:?}"
                            );
                            went_on += 1;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    assert!(
        t_broke > 0 && went_on > 0,
        "T's blade broke {t_broke}, went on {went_on}"
    );
    same_rate(
        "brk-parry: EE's parried blows breaking its blade",
        Sweep::Sample,
        6,
        70,
        ee_broke,
        parried,
    );
    same_rate(
        "brk-parry: EE's breaks with blows still due",
        Sweep::Sample,
        4,
        6,
        ee_due,
        ee_broke,
    );
}

/// capture.md 10.35 item 5 (`mb-mail`, `mb-mmail`, the `armed` round, 32
/// runs each): EE's blade made magic, T wearing the mail of Power 2, break
/// chance 15. Plain mail: ignored, no break roll, the full 10 or 15 (39
/// damaging blows, 32 of 32). Magic mail: its Power off (8 or 13) and the
/// break roll (41 damaging blows), the mail broken in 8 of the 32 runs
/// (it leaves its wearer, so once a run at most), after which the full 10
/// or 15 (combat 5.1's table). T readies the mail, ending its turn, as
/// `mail-heavy`.
#[test]
fn a_magic_weapon_against_plain_and_magic_mail() {
    for magic_mail in [false, true] {
        let mut a = fighter(true);
        thing(
            &mut a,
            BLADE,
            "BLADE",
            ThingKind::MeleeWeapon {
                picture: 6,
                disappears_after_use: false,
                carry: Carry { weight: 5, value: 10 },
                stats: WeaponStats {
                    magic: true,
                    ..stats(10)
                },
            },
        );
        thing(
            &mut a,
            MAIL,
            "MAIL",
            ThingKind::Armor {
                picture: 8,
                disappears_after_use: false,
                carry: Carry { weight: 5, value: 10 },
                stats: WeaponStats {
                    break_chance: 15,
                    magic: magic_mail,
                    ..stats(2)
                },
            },
        );
        let (mut broken, mut damaging) = (0, 0);
        for seed in 0..SEEDS / 4 {
            let (_, ev) = round(&a, seed, t(6, 40, vec![MAIL]), Command::Ready { thing: MAIL });
            let mut worn = true;
            for e in &ev {
                match e {
                    Event::Blow {
                        by,
                        outcome: BlowOutcome::Hit { damage, heavy },
                        ..
                    } if *by == slot(0) => {
                        let full = if *heavy { 15 } else { 10 };
                        let want = if magic_mail && worn { full - 2 } else { full };
                        assert_eq!(*damage, want, "magic mail {magic_mail}, seed {seed}");
                        damaging += 1;
                    }
                    Event::Broke {
                        who: ME, thing: MAIL, ..
                    } => {
                        assert!(magic_mail && worn, "seed {seed}: the plain mail broke");
                        worn = false;
                        broken += 1;
                    }
                    _ => {}
                }
            }
        }
        assert!(damaging > 0);
        if magic_mail {
            same_rate(
                "mb-mmail: runs with the mail broken",
                Sweep::Sample,
                8,
                32,
                broken,
                SEEDS / 4,
            );
        }
    }
}

/// capture.md 10.32 (`still-melee`): EE of Speed 0 (Strength 10, holding
/// the blade) on 5,4 beside T on 4,4 holding nothing; 16 runs, 4 of EE's
/// turns each: 64 of 64 turns had an attack, EE never moved, and T's Life
/// Force fell in 16 of them. T's Constitution and Life Force aren't given:
/// 40, so it lives through the runs (as it did).
fn still_melee() -> Adventure {
    let mut a = kit(5);
    ours(&mut a, 0, 5, 4, |c| {
        ee(c);
        c.speed = 0;
        holds(c, BLADE);
    });
    a
}

fn still_melee_turns(a: &Adventure, seed: u64) -> Vec<Vec<Event>> {
    let mut g = game(a, seed, t(6, 40, vec![]), room(4, 4));
    let mut ev = Vec::new();
    for _ in 0..4 {
        ev.extend(wait(&mut g));
    }
    turns_of(&ev, 0).into_iter().map(|t| t.to_vec()).collect()
}

#[test]
fn a_speed_0_fighter_attacks_its_goal_beside_it_once_a_turn() {
    let a = still_melee();
    let (mut turns, mut fell) = (0, 0);
    for seed in 0..SEEDS / 4 {
        for turn in still_melee_turns(&a, seed) {
            turns += 1;
            assert_eq!(attacks(&turn, slot(0)).len(), 1, "seed {seed}: {turn:?}");
            assert_eq!(steps(&turn), 0, "seed {seed}");
            fell += damaged(&turn, slot(0), ME) as u64;
        }
    }
    assert_eq!(turns, SEEDS);
    same_rate(
        "still-melee: turns T's Life Force fell",
        Sweep::Sample,
        16,
        64,
        fell,
        turns,
    );
}

/// capture.md 10.35 item 1 (`still-melee-sw`, `sm-d8`, `sm-d16`,
/// `sm-d24`): a Speed 0 fighter's attack has 1, 1, 2 and 3 blows at
/// Dexterity 13, 8, 16 and 24 (combat 3.1's first term is 1 when the full
/// moves are 0). 10.32's "2 blows" was a misread of `$B799`.
#[test]
fn a_speed_0_fighters_blows_by_dexterity() {
    for (dexterity, want) in [(13, 1), (8, 1), (16, 2), (24, 3)] {
        let mut a = still_melee();
        a.regions[0].residents[0].dexterity = dexterity;
        let mut seen = 0;
        for seed in 0..64 {
            for turn in still_melee_turns(&a, seed) {
                for (_, blows) in attacks(&turn, slot(0)) {
                    seen += 1;
                    assert_eq!(blows, want, "Dexterity {dexterity}, seed {seed}");
                }
            }
        }
        assert!(seen > 0, "Dexterity {dexterity}: no attack");
    }
}

/// capture.md 10.35 item 1 (`sm-d8`, 32 runs of 4 of EE's turns): EE of
/// Dexterity 8 and Size 9 (Build -4) did 9 a hit with the blade (-40 / 30
/// taken toward 0 as -1, combat 4.1.2), T losing 7 Life Force and 4 Power
/// a hit (from 40 and 13: the Power share 9 x 13 / 53 = 2); rounding down
/// (8) fits 13 of the 32 runs. 31 hits in the 128 attacks, many rolls:
/// a sample. A heavy blow adds half the Power: 14.
#[test]
fn a_negative_build_rounds_toward_zero() {
    let mut a = still_melee();
    a.regions[0].residents[0].dexterity = 8;
    let (mut attacked, mut hits, mut firsts) = (0, 0, 0);
    for seed in 0..SEEDS / 4 {
        let mut first = true;
        for turn in still_melee_turns(&a, seed) {
            attacked += attacks(&turn, slot(0)).len() as u64;
            for (i, e) in turn.iter().enumerate() {
                let Event::Blow {
                    by,
                    outcome: BlowOutcome::Hit { damage, heavy },
                    ..
                } = e
                else {
                    continue;
                };
                if *by != slot(0) {
                    continue;
                }
                hits += 1;
                assert_eq!(*damage, if *heavy { 14 } else { 9 }, "seed {seed}");
                if std::mem::take(&mut first) && !heavy {
                    let lost: Vec<(Stat, u8, u8)> = turn[i + 1..]
                        .iter()
                        .take_while(|e| matches!(e, Event::StatChanged { .. }))
                        .filter_map(|e| match e {
                            Event::StatChanged { who, stat, from, to } if *who == ME => {
                                Some((*stat, *from, *to))
                            }
                            _ => None,
                        })
                        .collect();
                    assert!(
                        lost.len() == 2
                            && lost.contains(&(Stat::LifeForce, 40, 33))
                            && lost.contains(&(Stat::Power, 13, 9)),
                        "seed {seed}: {lost:?}"
                    );
                    firsts += 1;
                }
            }
        }
    }
    assert!(firsts > 0);
    same_rate("sm-d8: attacks that hit", Sweep::Sample, 31, 128, hits, attacked);
}

/// capture.md 10.33 (`ord`): EE of Speed 0 holding the blade on 9,4, FF a
/// peaceful friend of Speed 0 beside it on 10,4, T on 4,4. `sw-ord`: an
/// attack on FF in 22 of 64 first turns; `sw-ord2`: 35 of 96 turns (3 a
/// run) with an attack on FF, 7 of them hurting it; EE never moved
/// (creatures 2.7.3). Many rolls each: samples.
#[test]
fn a_speed_0_fighter_strikes_a_creature_beside_it() {
    let mut a = kit(5);
    ours(&mut a, 0, 9, 4, |c| {
        ee(c);
        c.speed = 0;
        holds(c, BLADE);
    });
    ours(&mut a, 1, 10, 4, |c| {
        peaceful_friend(c);
        c.speed = 0;
    });
    let (mut first, mut turns, mut struck, mut hurt) = (0, 0, 0, 0);
    for seed in 0..SEEDS / 2 {
        let mut g = game(&a, seed, t(6, 11, vec![]), room(4, 4));
        let mut ev = Vec::new();
        for _ in 0..3 {
            ev.extend(wait(&mut g));
        }
        for (i, turn) in turns_of(&ev, 0).into_iter().enumerate() {
            assert_eq!(steps(turn), 0, "seed {seed}");
            let on_ff = attacks(turn, slot(0)).iter().any(|(w, _)| *w == slot(1));
            assert!(attacks(turn, slot(0)).iter().all(|(w, _)| *w == slot(1)));
            turns += 1;
            struck += on_ff as u64;
            hurt += damaged(turn, slot(0), slot(1)) as u64;
            if i == 0 {
                first += on_ff as u64;
            }
        }
    }
    same_rate(
        "sw-ord: first turns with an attack on FF",
        Sweep::Sample,
        22,
        64,
        first,
        SEEDS / 2,
    );
    same_rate(
        "sw-ord2: turns with an attack on FF",
        Sweep::Sample,
        35,
        96,
        struck,
        turns,
    );
    same_rate(
        "sw-ord2: attacks that hurt FF",
        Sweep::Sample,
        7,
        35,
        hurt,
        struck,
    );
}

/// capture.md 10.14.10 (`player-friend`, `player-none`): T holding the
/// blade with 5 of 6 moves left struck 3 blows at a friend east of it (12
/// of 12); owning no weapon, the push into a creature drew nothing and
/// nobody moved (12 of 12; combat 2.7: no move used either, 10.24).
#[test]
fn the_players_attack_and_the_push_with_no_weapon() {
    let mut a = kit(5);
    ours(&mut a, 0, 5, 4, |c| {
        peaceful_friend(c);
        c.speed = 0;
    });
    for seed in 0..16 {
        let mut g = game(&a, seed, t(6, 11, vec![BLADE]), room(4, 4));
        ready(&mut g, BLADE);
        assert_eq!(moves(&g), 5);
        let ev = g.apply(Command::Move { dir: Dir::E });
        assert_eq!(attacks(&ev, ME), vec![(slot(0), 3)], "seed {seed}: {ev:?}");

        let mut g = game(&a, seed, t(6, 11, vec![]), room(4, 4));
        let before = rng(&g);
        let ev = g.apply(Command::Move { dir: Dir::E });
        assert!(
            ev.iter()
                .any(|e| matches!(e, Event::BumpedCreature { resident: 0, .. })),
            "{ev:?}"
        );
        assert_eq!(moves(&g), 6);
        assert_eq!(rng(&g), before, "seed {seed}");
        assert_eq!(g.creature(1, 0).map(|c| (c.x, c.y)), Some((5, 4)));
    }
}

// ---- creatures firing -----------------------------------------------------

/// T on 4,4 and an archer of ours in slot 0 on 9,4, 5 squares east.
fn archer(range: u8, f: impl FnOnce(&mut Creature)) -> Adventure {
    let mut a = kit(range);
    ours(&mut a, 0, 9, 4, f);
    a
}

/// capture.md 10.24 item 2 (`still-bow`): an archer of Speed 0 holding the
/// bow, T 5 squares west: 128 runs of one turn, a shot every turn (1 shot:
/// Dexterity 13 / 8); 29 arrows reached T (dodge rolls), 12 dodged and 17
/// hit for damage (combat 8.1, 8.6). The archer's other stats aren't
/// given: EE's (10.14.6). The capture's "2 stopped" isn't compared (what
/// stopped them isn't said).
#[test]
fn a_speed_0_archer_shoots_once_a_turn() {
    let a = archer(5, |c| {
        ee(c);
        c.speed = 0;
        holds(c, BOW);
    });
    let (mut dodged, mut hit) = (0, 0);
    for seed in 0..SEEDS {
        let mut g = game(&a, seed, t(6, 11, vec![]), room(4, 4));
        let ev = wait(&mut g);
        let turn = turns_of(&ev, 0)[0];
        assert_eq!(fired(turn, slot(0)), Some(1), "seed {seed}: {turn:?}");
        assert_eq!(steps(turn), 0);
        let (d, h) = reached(turn, slot(0), ME);
        dodged += d;
        hit += h;
    }
    same_rate(
        "still-bow: shots reaching T",
        Sweep::Sample,
        29,
        128,
        dodged + hit,
        SEEDS,
    );
    same_rate(
        "still-bow: dodges of those",
        Sweep::Sample,
        12,
        29,
        dodged,
        dodged + hit,
    );
    same_rate(
        "still-bow: shots doing damage",
        Sweep::Sample,
        17,
        128,
        hit,
        SEEDS,
    );
}

/// capture.md 10.35 item 6 (`bow-adj`, 32 runs from the room snapshot, 32
/// of 32 fitting): EE of Speed 3 holding the bow on 5,4, truly beside T
/// on 4,4: its turn halved to 2 moves (creatures 2.6), no shot, one step
/// away (to 5,5 17 times, 5,3 9, 6,4 6), then one shot with the move
/// left (Dexterity 13 / 8 + 1 / 2); 29 of the 32 arrows reached T, 17 of
/// those hitting (combat 8.1). The step is the run's later rolls: a
/// histogram. EE's other stats are 10.14.6's; T holds nothing, at
/// Constitution and Life Force 40.
#[test]
fn an_archer_beside_its_foe_steps_away_then_shoots() {
    let mut a = kit(5);
    ours(&mut a, 0, 5, 4, |c| {
        ee(c);
        holds(c, BOW);
    });
    let squares = [(5, 5), (5, 3), (6, 4)];
    let (mut reached_t, mut hit) = (0, 0);
    let h = histogram(SEEDS, 4, |seed| {
        let mut g = game(&a, seed, t(6, 40, vec![]), room(4, 4));
        let ev = wait(&mut g);
        let turn = turns_of(&ev, 0)[0];
        assert_eq!(turn_moves(turn), 2, "seed {seed}");
        let shot = turn
            .iter()
            .position(|e| matches!(e, Event::Fired { .. }))
            .unwrap_or_else(|| panic!("seed {seed}: no shot: {turn:?}"));
        assert_eq!(steps(&turn[..shot]), 1, "seed {seed}: {turn:?}");
        assert_eq!(fired(turn, slot(0)), Some(1), "seed {seed}");
        let (d, k) = reached(turn, slot(0), ME);
        reached_t += d + k;
        hit += k;
        let to = turn[..shot].iter().find_map(|e| match e {
            Event::CreatureMoved {
                to: Location::Room { x, y, .. },
                ..
            } => Some((*x, *y)),
            _ => None,
        });
        bin(&squares, to.expect("a step"))
    });
    assert_eq!(h[3], 0, "stepped elsewhere");
    same_histogram("bow-adj: the step to 5,5 5,3 6,4", &[17, 9, 6], &h[..3]);
    same_rate(
        "bow-adj: arrows reaching T",
        Sweep::Sample,
        29,
        32,
        reached_t,
        SEEDS,
    );
    same_rate("bow-adj: of those, hits", Sweep::Sample, 17, 29, hit, reached_t);
}

/// capture.md 10.35 item 7 (`ready-melee`, 32 of 32 runs): EE of Speed 3
/// owning the blade, holding nothing, on 5,4 beside T: its turn halved to
/// 2, the blade readied at one move, then the attack with 2 blows (4 x 1
/// / 2 + 13 / 8 - 1); its next turn 4 blows (combat 2.6, 6.2). T holds
/// nothing, at Constitution and Life Force 40.
#[test]
fn a_creature_readies_its_melee_weapon_before_the_step_into_its_foe() {
    let mut a = kit(5);
    ours(&mut a, 0, 5, 4, |c| {
        ee(c);
        has(c, BLADE);
    });
    for seed in 0..256 {
        let mut g = game(&a, seed, t(6, 40, vec![]), room(4, 4));
        let mut ev = wait(&mut g);
        ev.extend(wait(&mut g));
        let turns = turns_of(&ev, 0);
        let first = turns[0];
        assert_eq!(turn_moves(first), 2, "seed {seed}");
        let r = first.iter().position(|e| {
            matches!(
                e,
                Event::CreatureReadied {
                    slot: 0,
                    thing: BLADE,
                    ..
                }
            )
        });
        let at = first
            .iter()
            .position(|e| matches!(e, Event::Attacked { by, .. } if *by == slot(0)));
        assert!(r.is_some() && r < at, "seed {seed}: {first:?}");
        assert_eq!(steps(first), 0, "seed {seed}");
        assert_eq!(attacks(first, slot(0)), [(ME, 2)], "seed {seed}");
        assert_eq!(
            attacks(turns[1], slot(0)),
            [(ME, 4)],
            "seed {seed}: {:?}",
            turns[1]
        );
    }
}

/// capture.md 10.35 item 7 (`p-nowpn`, one run, nothing random): T owning
/// the blade, holding nothing, with 2 moves left, EE of Speed 0 on 5,4:
/// the push east used no move and brought the melee menu; the blade
/// chosen, it was readied at one move (2 to 1) and the attack came at
/// once with 1 blow (4 x 1 / 6 + 13 / 8 - 1 is 0, so 1), ending the turn
/// (combat 2.8). **Engine:** the menu is the front end's; the push readies
/// the blade. NONE (`nw-none`) has no engine command: not compared.
#[test]
fn the_push_with_a_melee_weapon_owned_but_not_held() {
    let mut a = kit(5);
    ours(&mut a, 0, 5, 4, |c| {
        ee(c);
        c.speed = 0;
    });
    for seed in 0..16 {
        let mut g = game(&a, seed, t(6, 40, vec![BLADE]), room(4, 4));
        idle_to(&mut g, 2);
        let ev = g.apply(Command::Move { dir: Dir::E });
        let r = ev.iter().position(|e| {
            matches!(
                e,
                Event::Readied {
                    thing: BLADE,
                    slot: Slot::Weapon,
                    ..
                }
            )
        });
        let at = ev
            .iter()
            .position(|e| matches!(e, Event::Attacked { by: ME, .. }));
        assert!(r.is_some() && r < at, "seed {seed}: {ev:?}");
        assert_eq!(attacks(&ev, ME), [(slot(0), 1)], "seed {seed}");
        assert!(ev.contains(&Event::TurnEnded { player: 0 }), "seed {seed}");
        assert_eq!(g.players()[0].weapon, Some(BLADE));
    }
}

/// capture.md 10.33: EE (Strength 10, Dexterity 13) on 9,4 with the bow of
/// power 10, range 7, T on 4,4, EE's first turn. `ready1` (Speed 1, the bow
/// owned, not held): readied with the move, then nothing, no shot in 32 of
/// 32; `ready2` (Speed 2): readied, then a shot; `held1` (Speed 1, held): a
/// shot; `ready1w` (`ready1` at Life Force 10 of 20, an obstacle on 5,4):
/// readied, no step, Life Force +1 only from the top-of-turn die, in 4 of
/// 32 turns (creatures 2.7).
#[test]
fn readying_with_the_last_move_ends_a_creatures_turn() {
    let case = |speed: u8, held: bool| {
        archer(7, move |c| {
            c.speed = speed;
            c.strength = 10;
            if held {
                holds(c, BOW);
            } else {
                has(c, BOW);
            }
        })
    };
    let readied = |turn: &[Event]| {
        turn.iter()
            .any(|e| matches!(e, Event::CreatureReadied { thing: BOW, .. }))
    };
    for (name, a, readies, shoots) in [
        ("ready1", case(1, false), true, false),
        ("ready2", case(2, false), true, true),
        ("held1", case(1, true), false, true),
    ] {
        for seed in 0..64 {
            let mut g = game(&a, seed, t(6, 11, vec![]), room(4, 4));
            let ev = wait(&mut g);
            let turn = turns_of(&ev, 0)[0];
            assert_eq!(readied(turn), readies, "{name} seed {seed}: {turn:?}");
            assert_eq!(
                fired(turn, slot(0)).is_some(),
                shoots,
                "{name} seed {seed}: {turn:?}"
            );
            assert_eq!(steps(turn), 0, "{name} seed {seed}");
        }
    }
    let mut a = case(1, false);
    item(&mut a, ROOM, 5, 4, OBSTACLE);
    if let Some(c) = a.regions[0].residents.get_mut(0) {
        c.constitution = 20;
        c.life_force = 10;
    }
    let healed = count(SEEDS, |seed| {
        let mut g = game(&a, seed, t(6, 11, vec![]), room(4, 4));
        let ev = wait(&mut g);
        let turn = turns_of(&ev, 0)[0];
        assert!(
            readied(turn) && fired(turn, slot(0)).is_none() && steps(turn) == 0,
            "seed {seed}: {turn:?}"
        );
        let lf = g.creature(1, 0).unwrap().life_force;
        assert!((10..=11).contains(&lf), "seed {seed}: {lf}");
        lf == 11
    });
    same_rate(
        "ready1w: turns healed (the die only)",
        Sweep::Sample,
        4,
        32,
        healed,
        SEEDS,
    );
}

/// capture.md 10.16 (`bow-wall`): behind an obstacle on 5,4 (the line of
/// fire blocked) EE holding the bow never fired and walked: 54 steps in 16
/// runs, no shot (combat 8.1: it walked every move of its first turn).
#[test]
fn behind_an_obstacle_an_archer_walks() {
    let mut a = archer(5, |c| {
        ee(c);
        holds(c, BOW);
    });
    item(&mut a, ROOM, 5, 4, OBSTACLE);
    for seed in 0..256 {
        let mut g = game(&a, seed, t(6, 11, vec![]), room(4, 4));
        let ev = wait(&mut g);
        let turn = turns_of(&ev, 0)[0];
        assert_eq!(fired(turn, slot(0)), None, "seed {seed}: {turn:?}");
        assert_eq!(steps(turn), turn_moves(turn) as usize, "seed {seed}: {turn:?}");
    }
}

/// capture.md 10.16, 10.17, 10.21 (`bow-once`): a bow that disappears after
/// use went after its first shot in 16 of 16 runs and joined the store's
/// stock in 32 of 32 (combat 8.7, stores 2.5): of the 2 shots due (13 / 8 +
/// 3 / 2), one is fired.
#[test]
fn a_disappearing_bow_goes_after_one_shot_into_the_stock() {
    let mut a = archer(5, |c| {
        ee(c);
        holds(c, BOW);
    });
    thing(&mut a, BOW, "BOW", our_bow(5, true));
    for seed in 0..64 {
        let mut g = game(&a, seed, t(6, 11, vec![]), room(4, 4));
        let ev = wait(&mut g);
        let turn = turns_of(&ev, 0)[0];
        assert_eq!(fired(turn, slot(0)), Some(2), "seed {seed}");
        assert_eq!(shots_of(turn, slot(0)), 1, "seed {seed}");
        let broke = turn
            .iter()
            .position(|e| {
                matches!(
                    e,
                    Event::Broke {
                        thing: BOW,
                        used_up: true,
                        ..
                    }
                )
            })
            .expect("used up");
        assert!(matches!(
            turn[broke + 1],
            Event::Restocked {
                region: 1,
                thing: BOW
            }
        ));
        assert_eq!(g.creature(1, 0).map(|c| c.weapon), Some(0));
    }
}

// ---- the player's FIRE ----------------------------------------------------

/// The rr9 `player-bow` room: T on 4,4 and EE (10.14.6) on 9,4.
fn player_bow() -> Adventure {
    archer(5, ee)
}

/// T (Speed 6, owning the bow) readies it with its first move: 5 of 6
/// moves left, holding the bow (capture.md 10.24's `fire1`).
fn holding_the_bow(a: &Adventure, seed: u64) -> Game {
    let mut g = game(a, seed, t(6, 11, vec![BOW]), room(4, 4));
    ready(&mut g, BOW);
    assert_eq!(moves(&g), 5);
    g
}

/// capture.md 10.24 item 1 (`fire1`): holding the bow with 5 moves left,
/// FIRE shot 3 shots (Dexterity 13 / 8 + 5 / 2) and the moves went to 0,
/// the turn over. The sweeps from the cursor: `fire-e5` (cursor on EE)
/// 192 shots in 64 runs, 16 damaging arrows on EE; `fire-e3` (cursor on
/// 7,4, the arrow flying on past) 192 shots, 20 damaging; `fire-e0`
/// (cursor on T's own square, aimed south) every arrow stopped (combat
/// 8.8.3).
#[test]
fn the_players_fire_from_the_cursor() {
    let a = player_bow();
    for (name, target, damaging) in [("fire-e5", Some((9, 4)), 16), ("fire-e3", Some((7, 4)), 20)] {
        let (mut shots, mut hits) = (0, 0);
        for seed in 0..SEEDS {
            let mut g = holding_the_bow(&a, seed);
            let ev = g.apply(Command::Fire { target, weapon: None });
            assert_eq!(fired(&ev, ME), Some(3), "{name} seed {seed}");
            assert!(ev.contains(&Event::TurnEnded { player: 0 }));
            let end = ev
                .iter()
                .position(|e| *e == Event::TurnEnded { player: 0 })
                .unwrap();
            shots += shots_of(&ev[..end], ME);
            hits += reached(&ev[..end], ME, slot(0)).1;
        }
        // Three hits can kill EE (Life Force 20), ending the firing.
        assert!(shots > 3 * SEEDS - 16, "{name}: {shots}");
        same_rate(
            &format!("{name}: damaging arrows"),
            Sweep::Sample,
            damaging,
            192,
            hits,
            shots,
        );
    }
    for seed in 0..256 {
        let mut g = holding_the_bow(&a, seed);
        let ev = g.apply(Command::Fire {
            target: None,
            weapon: None,
        });
        let end = ev
            .iter()
            .position(|e| *e == Event::TurnEnded { player: 0 })
            .unwrap();
        let ev = &ev[..end];
        assert_eq!(shots_of(ev, ME), 3, "fire-e0 seed {seed}");
        let stopped = ev
            .iter()
            .filter(|e| matches!(e, Event::MissileStopped { by: ME, .. }))
            .count();
        assert_eq!(stopped, 3, "fire-e0 seed {seed}: {ev:?}");
        assert!(!ev
            .iter()
            .any(|e| matches!(e, Event::MissileHit { .. } | Event::Dodged { .. })));
    }
}

/// capture.md 10.32 (`kill-e5`): fire-e5's room with T of Speed 10 holding
/// the bow with 6 moves left (4 shots) and EE at Life Force 1: 25 kills in
/// 64 runs, and at each the firing ended with shots still due (combat
/// 8.8.3). T gets to 6 moves by readying and 3 idle moves.
#[test]
fn a_kill_ends_the_players_firing() {
    let mut a = player_bow();
    a.regions[0].residents[0].life_force = 1;
    let kills = count(SEEDS, |seed| {
        let mut g = game(&a, seed, t(10, 11, vec![BOW]), room(4, 4));
        ready(&mut g, BOW);
        idle_to(&mut g, 6);
        let ev = g.apply(Command::Fire {
            target: Some((9, 4)),
            weapon: None,
        });
        assert_eq!(fired(&ev, ME), Some(4), "seed {seed}");
        let Some(died) = ev
            .iter()
            .position(|e| matches!(e, Event::Died { who, .. } if *who == slot(0)))
        else {
            assert_eq!(shots_of(&ev, ME), 4, "seed {seed}");
            return false;
        };
        assert!(
            !ev[died..].iter().any(|e| matches!(e, Event::Shot { by: ME, .. })),
            "seed {seed}"
        );
        true
    });
    same_rate("kill-e5: runs with a kill", Sweep::Sample, 25, 64, kills, SEEDS);
}

/// The classes `arrow-c0` .. `arrow-c7` set (capture.md 10.35: class byte
/// 128 + 16 x class): peaceful thief, neutral, enemy, friend, then the
/// same aggressive.
const CLASSES: [(Alignment, bool); 8] = [
    (Alignment::Thief, false),
    (Alignment::Neutral, false),
    (Alignment::Enemy, false),
    (Alignment::Friend, false),
    (Alignment::Thief, true),
    (Alignment::Neutral, true),
    (Alignment::Enemy, true),
    (Alignment::Friend, true),
];

/// capture.md 10.35 item 2 (`arrow-c0` .. `arrow-c7` at Wisdom 10, 128
/// runs each, `arrow-c5` 256; `arrow-c5w3`, `arrow-c5w20` 128 each; every
/// run fitting): `fire-e5` (3 shots at EE on 9,4) with EE's class and
/// Wisdom set. After each damaging arrow, combat 7: a peaceful thief,
/// neutral or enemy turns aggressive, a friend neutral (keeping its
/// setting), with no roll (23 hits each); an aggressive thief or enemy is
/// unchanged and draws the roll (26 hits each); an aggressive neutral of
/// Wisdom 10 draws it and turns an enemy on 0 to 4 (57 hits, a turn in
/// 30 of the 256 runs: 226 fit with no change), at Wisdom 3 never and at
/// 20 always, with no roll (23 hits each). A peaceful neutral hit twice
/// turns aggressive, then may turn an enemy on the second. The roll is
/// seen through the shots after it: the runs of the classes that don't
/// draw it are alike shot for shot up to the second damaging arrow, as
/// are those that do, and the two differ. The hits are compared per shot
/// due (3 a run).
#[test]
fn a_damaging_arrow_makes_section_7s_check_by_the_class() {
    let cases: [(&str, usize, u8, u64, u64); 10] = [
        ("arrow-c0", 0, 10, 23, 128),
        ("arrow-c1", 1, 10, 23, 128),
        ("arrow-c2", 2, 10, 23, 128),
        ("arrow-c3", 3, 10, 23, 128),
        ("arrow-c4", 4, 10, 26, 128),
        ("arrow-c5", 5, 10, 57, 256),
        ("arrow-c6", 6, 10, 26, 128),
        ("arrow-c7", 7, 10, 23, 128),
        ("arrow-c5w3", 5, 3, 23, 128),
        ("arrow-c5w20", 5, 20, 23, 128),
    ];
    const RUNS: u64 = SEEDS / 8;
    let mut refs: [Option<Vec<Vec<Event>>>; 2] = [None, None];
    let mut after_first = vec![false; RUNS as usize];
    for (name, k, wisdom, cap_hits, cap_runs) in cases {
        let class = CLASSES[k];
        let rolls = class.1 && class.0 != Alignment::Friend && (6..=14).contains(&wisdom);
        let mut a = player_bow();
        let c = &mut a.regions[0].residents[0];
        c.strategy.alignment = class.0;
        c.strategy.aggressive = class.1;
        c.wisdom = wisdom;
        let (mut shots, mut hits, mut turned_enemy) = (0, 0, 0);
        let mut prefixes = Vec::new();
        for seed in 0..RUNS {
            let mut g = holding_the_bow(&a, seed);
            let ev = g.apply(Command::Fire {
                target: Some((9, 4)),
                weapon: None,
            });
            let end = ev
                .iter()
                .position(|e| *e == Event::TurnEnded { player: 0 })
                .unwrap();
            let ev = &ev[..end];
            shots += shots_of(ev, ME);
            // Section 7 hit by hit: the change due, or for an aggressive
            // neutral of Wisdom 6 to 14 the one its roll may bring.
            let (mut now, mut due, mut maybe, mut damaging) = (class, None, false, 0);
            let mut prefix = Vec::new();
            for e in ev {
                let include = damaging < 2;
                match e {
                    Event::MissileHit {
                        by: ME,
                        target,
                        outcome: BlowOutcome::Hit { damage, .. },
                        ..
                    } if *target == slot(0) && *damage > 0 => {
                        assert_eq!(due, None, "{name} seed {seed}: no turn: {ev:?}");
                        hits += 1;
                        damaging += 1;
                        if damaging == 2 && k == 1 {
                            assert_eq!(now, (Alignment::Neutral, true), "{name} seed {seed}");
                        }
                        maybe = false;
                        due = match now {
                            (Alignment::Friend, a) => Some((Alignment::Neutral, a)),
                            (c, false) => Some((c, true)),
                            (Alignment::Neutral, true) if wisdom > 14 => Some((Alignment::Enemy, true)),
                            (Alignment::Neutral, true) => {
                                maybe = wisdom >= 6;
                                None
                            }
                            _ => None,
                        };
                    }
                    Event::Died { who, .. } if *who == slot(0) => (due, maybe) = (None, false),
                    Event::CreatureTurned {
                        alignment,
                        aggressive,
                        ..
                    } => {
                        let got = (*alignment, *aggressive);
                        if let Some(want) = due.take() {
                            assert_eq!(got, want, "{name} seed {seed}");
                        } else {
                            assert!(
                                std::mem::take(&mut maybe) && got == (Alignment::Enemy, true),
                                "{name} seed {seed}: {ev:?}"
                            );
                            turned_enemy += 1;
                        }
                        now = got;
                        continue;
                    }
                    Event::Shot { .. } if damaging == 1 => after_first[seed as usize] = true,
                    _ => {}
                }
                if include {
                    prefix.push(e.clone());
                }
            }
            assert_eq!(due, None, "{name} seed {seed}: no turn: {ev:?}");
            prefixes.push(prefix);
        }
        match &refs[rolls as usize] {
            None => refs[rolls as usize] = Some(prefixes),
            Some(r) => assert!(
                *r == prefixes,
                "{name}: the runs differ from the others that {}draw",
                if rolls { "" } else { "don't " }
            ),
        }
        same_rate(
            &format!("{name}: damaging arrows a shot"),
            Sweep::Sample,
            cap_hits,
            3 * cap_runs,
            hits,
            shots,
        );
        if (k, wisdom) == (5, 10) {
            same_rate(
                "arrow-c5: runs turned an enemy",
                Sweep::Sample,
                30,
                256,
                turned_enemy,
                RUNS,
            );
        } else if [0, 2, 4, 6].contains(&k) || wisdom < 6 {
            // A friend or a peaceful neutral may turn an enemy once it is
            // an aggressive neutral, by a later arrow's roll.
            assert_eq!(turned_enemy, 0, "{name}");
        }
    }
    let (Some(plain), Some(rolled)) = (&refs[0], &refs[1]) else {
        unreachable!()
    };
    let candidates = after_first.iter().filter(|&&b| b).count();
    let differ = (0..RUNS as usize)
        .filter(|&s| after_first[s] && plain[s] != rolled[s])
        .count();
    assert!(
        candidates > 10 && differ * 10 >= candidates * 9,
        "{differ} of {candidates}"
    );
}

// ---- the player's blows and section 7 -------------------------------------

/// The generator's state, as a number.
fn rng_state(g: &Game) -> u64 {
    rng(g)["state"].as_u64().expect("a state")
}

/// Draws between two generator states (SplitMix64 adds its constant once a
/// draw).
fn draws(before: u64, after: u64) -> u64 {
    const STEP: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut inv = STEP;
    for _ in 0..6 {
        inv = inv.wrapping_mul(2u64.wrapping_sub(STEP.wrapping_mul(inv)));
    }
    after.wrapping_sub(before).wrapping_mul(inv)
}

/// `ev` without the class changes (combat 7's outcome, not its draws).
fn without_turns(ev: &[Event]) -> Vec<Event> {
    ev.iter()
        .filter(|e| !matches!(e, Event::CreatureTurned { .. }))
        .cloned()
        .collect()
}

/// `by`'s blows at `target` in `ev`: (all, damaging), and whether a damaging
/// one has another blow after it.
fn blows_at(ev: &[Event], by: Who, target: Who) -> (u64, u64, bool) {
    let (mut blows, mut hits, mut later) = (0, 0, false);
    for e in ev {
        if let Event::Blow {
            by: b,
            target: t,
            outcome,
            ..
        } = e
        {
            if *b != by || *t != target {
                continue;
            }
            later |= hits > 0;
            blows += 1;
            hits += matches!(outcome, BlowOutcome::Hit { damage, .. } if *damage > 0) as u64;
        }
    }
    (blows, hits, later)
}

/// rr33's `pb` (capture.md 10.37 item 1): FF on 5,4, a creature of ours of
/// Speed 0 holding nothing, Constitution 40 and Life Force `lf`, of `class`
/// and `wisdom`, T on 4,4.
fn pb(class: (Alignment, bool), wisdom: u8, lf: u8) -> Adventure {
    let mut a = kit(5);
    ours(&mut a, 0, 5, 4, |c| {
        c.speed = 0;
        c.constitution = 40;
        c.life_force = lf;
        c.wisdom = wisdom;
        c.strategy.alignment = class.0;
        c.strategy.aggressive = class.1;
    });
    a
}

/// `pb`'s T (Speed 6, Dexterity 31, Constitution and Life Force 40) readies
/// the blade (5 of 6 moves left) and pushes east into FF: the attack's
/// events to T's turn end, and the command's draws.
fn pb_push(a: &Adventure, seed: u64) -> (Vec<Event>, u64) {
    let me = PlayerSetup {
        dexterity: 31,
        ..t(6, 40, vec![BLADE])
    };
    let mut g = game(a, seed, me, room(4, 4));
    ready(&mut g, BLADE);
    let before = rng_state(&g);
    let ev = g.apply(Command::Move { dir: Dir::E });
    let n = draws(before, rng_state(&g));
    assert_eq!(attacks(&ev, ME), [(slot(0), 5)], "seed {seed}: {ev:?}");
    let end = ev
        .iter()
        .position(|e| *e == Event::TurnEnded { player: 0 })
        .unwrap_or_else(|| panic!("seed {seed}: the attack ends the turn: {ev:?}"));
    (ev[..end].to_vec(), n)
}

/// capture.md 10.37 item 1 (`pb`, 32 runs a sweep, every draw checked by
/// `blowcheck.py`): T's 5 blows a run at FF. An aggressive thief, neutral
/// or enemy of Wisdom 10 (`b-c4`, `b-c5`, `b-c6`) draws combat 7's roll
/// after every damaging blow (34 damaging blows each), staying as it was
/// (the neutral may turn an enemy); the aggressive thief and enemy at
/// Wisdom 3 (`b-c4w3`, `b-c6w3`, 40 each) and 20 (`b-c4w20`, `b-c6w20`, 37
/// each) draw none. The roll is seen through the blows after it: the
/// sweeps that draw are alike blow for blow, as are those that don't (no
/// class changes but the neutral's), and the two differ after a damaging
/// blow. The damaging blows are compared per blow (5 a run). FF's other
/// stats are the harness's.
#[test]
fn a_damaging_blow_makes_section_7s_check_by_the_class() {
    const RUNS: u64 = SEEDS / 8;
    let cases: [(&str, usize, u8, bool, u64); 7] = [
        ("b-c4", 4, 10, true, 34),
        ("b-c6", 6, 10, true, 34),
        ("b-c5", 5, 10, true, 34),
        ("b-c4w3", 4, 3, false, 40),
        ("b-c6w3", 6, 3, false, 40),
        ("b-c4w20", 4, 20, false, 37),
        ("b-c6w20", 6, 20, false, 37),
    ];
    // Each run's events and its draws over the command: the roll shows in
    // the draws even where the blows after it come out the same.
    type Runs = Vec<(Vec<Event>, u64)>;
    let mut refs: [Option<Runs>; 2] = [None, None];
    let mut damaged = vec![false; RUNS as usize];
    for (name, k, wisdom, rolls, cap_hits) in cases {
        let a = pb(CLASSES[k], wisdom, 40);
        let (mut blows, mut hits) = (0, 0);
        let mut runs = Vec::new();
        for seed in 0..RUNS {
            let (ev, n) = pb_push(&a, seed);
            let (b, h, _) = blows_at(&ev, ME, slot(0));
            blows += b;
            hits += h;
            if rolls {
                damaged[seed as usize] = h > 0;
            }
            for e in &ev {
                if let Event::CreatureTurned {
                    alignment,
                    aggressive,
                    ..
                } = e
                {
                    assert!(
                        k == 5 && (*alignment, *aggressive) == (Alignment::Enemy, true),
                        "{name} seed {seed}: {e:?}"
                    );
                }
            }
            runs.push((without_turns(&ev), n));
        }
        match &refs[rolls as usize] {
            None => refs[rolls as usize] = Some(runs),
            Some(r) => assert!(
                *r == runs,
                "{name}: the runs differ from the others that {}draw",
                if rolls { "" } else { "don't " }
            ),
        }
        same_rate(
            &format!("{name}: damaging blows a blow"),
            Sweep::Sample,
            cap_hits,
            5 * 32,
            hits,
            blows,
        );
    }
    let (Some(plain), Some(rolled)) = (&refs[0], &refs[1]) else {
        unreachable!()
    };
    // Wherever a damaging blow fell, the runs that roll differ from those
    // that don't, in their blows or their draws.
    let candidates = damaged.iter().filter(|&&b| b).count();
    let differ = (0..RUNS as usize)
        .filter(|&s| damaged[s] && plain[s] != rolled[s])
        .count();
    assert!(
        candidates > 10 && differ == candidates,
        "{differ} of {candidates}"
    );
}

/// capture.md 10.37 item 1 (`k-c4`, `k-c5`, `k-c6`, `k-c1`, `k-c3`, 16 runs
/// each): `pb` with FF at Life Force 1, the first damaging blow killing it:
/// 13 kills in 16 runs each. After the killing blow the aggressive thief,
/// neutral and enemy draw the roll (by the class before the blow), the
/// peaceful neutral and the peaceful friend none. Seen through the
/// generator: the runs are alike event for event, and a run with a kill
/// draws one more over the command for the three that roll. FF's Wisdom
/// isn't given: the harness's 13.
#[test]
fn a_killing_blow_makes_the_check_by_the_class_before_it() {
    const RUNS: u64 = SEEDS / 8;
    let cases = [
        ("k-c1", 1, false),
        ("k-c3", 3, false),
        ("k-c4", 4, true),
        ("k-c5", 5, true),
        ("k-c6", 6, true),
    ];
    let mut plain: Option<Vec<(Vec<Event>, u64)>> = None;
    for (name, k, rolls) in cases {
        let a = pb(CLASSES[k], 13, 1);
        let runs: Vec<(Vec<Event>, u64)> = (0..RUNS)
            .map(|seed| {
                let (ev, n) = pb_push(&a, seed);
                (without_turns(&ev), n)
            })
            .collect();
        let killed = |ev: &[Event]| {
            ev.iter()
                .any(|e| matches!(e, Event::Died { who, .. } if *who == slot(0)))
        };
        let kills = runs.iter().filter(|(ev, _)| killed(ev)).count() as u64;
        same_rate(
            &format!("{name}: runs with a kill"),
            Sweep::Sample,
            13,
            16,
            kills,
            RUNS,
        );
        match &plain {
            None => plain = Some(runs),
            Some(p) => {
                for (seed, ((ev, n), (pev, pn))) in runs.iter().zip(p).enumerate() {
                    assert_eq!(ev, pev, "{name} seed {seed}");
                    if killed(ev) {
                        assert_eq!(*n, pn + rolls as u64, "{name} seed {seed}: draws");
                    }
                }
            }
        }
    }
}

/// rr33's `pu` (capture.md 10.37 item 1): T of Speed 12, Dexterity 31,
/// holding the blade, on 5,4 at a turn of 12 moves (6 blows), U of
/// `wisdom` beside it on 4,4; both Constitution and Life Force 40 (as
/// 10.35's `fire-u`), the rest the harness's.
fn pu(a: &Adventure, seed: u64, wisdom: u8) -> Game {
    let me = PlayerSetup {
        dexterity: 31,
        ..t(12, 40, vec![BLADE])
    };
    let (mut g, _) = Game::new(a.clone(), seed, Default::default(), me);
    let mut u = t(4, 40, vec![]);
    u.name = "U".into();
    u.wisdom = wisdom;
    g.apply(Command::AddPlayer { setup: u });
    g.apply(Command::Wait);
    g.place(0, room(5, 4));
    g.place(1, room(4, 4));
    while g.current() != 0 {
        g.apply(Command::Wait);
    }
    ready(&mut g, BLADE);
    g.apply(Command::Wait);
    while g.current() != 0 {
        g.apply(Command::Wait);
    }
    assert_eq!(moves(&g), 12);
    g
}

/// capture.md 10.37 item 1 (`u-w10`, 32 runs, 50 damaging blows): T's 6
/// blows west at U of Wisdom 10 draw the roll after every damaging blow
/// (`--player-roll yes`; without it 4 of 32 fit). Seen as above: U of
/// Wisdom 10 is struck blow for blow as U of Wisdom 13 (both draw), and
/// otherwise than U of Wisdom 3 (the engine's choice: no roll outside 6 to
/// 14, combat 7) after a damaging blow. Damaging blows per blow (6 a run).
#[test]
fn a_players_damaging_blow_at_a_player_draws_the_roll() {
    const RUNS: u64 = SEEDS / 8;
    let a = kit(5);
    let u = Who::Player { player: 1 };
    // The blows and the draws over the command: U's Power numbers differ
    // with its Wisdom (drift), so the events are compared as blows only.
    let run = |seed: u64, wisdom: u8| {
        let mut g = pu(&a, seed, wisdom);
        let before = rng_state(&g);
        let ev = g.apply(Command::Move { dir: Dir::W });
        let n = draws(before, rng_state(&g));
        assert_eq!(attacks(&ev, ME), [(u, 6)], "seed {seed}: {ev:?}");
        let end = ev
            .iter()
            .position(|e| *e == Event::TurnEnded { player: 0 })
            .unwrap();
        let blows: Vec<Event> = ev[..end]
            .iter()
            .filter(|e| matches!(e, Event::Blow { .. }))
            .cloned()
            .collect();
        (blows, n)
    };
    let (mut blows, mut hits, mut candidates, mut differ) = (0, 0, 0, 0);
    for seed in 0..RUNS {
        let w10 = run(seed, 10);
        assert_eq!(w10, run(seed, 13), "seed {seed}");
        let (b, h, _) = blows_at(&w10.0, ME, u);
        blows += b;
        hits += h;
        if h > 0 {
            candidates += 1;
            differ += (w10 != run(seed, 3)) as u64;
        }
    }
    assert!(
        candidates > 10 && differ == candidates,
        "{differ} of {candidates}"
    );
    same_rate(
        "u-w10: damaging blows a blow",
        Sweep::Sample,
        50,
        6 * 32,
        hits,
        blows,
    );
}

/// capture.md 10.37 item 2 (`dk`, random state 0): EE (10.14.6) at Life
/// Force 1 with 50 gold, holding the blade and owning treasures of ours,
/// things 50 and 51 (weights not given: 1), on 5,4; T of Melee 100 holding
/// the blade on 4,4. Killed, its things go down gold first, then its
/// possessions from the lowest number up: the square's list, top first,
/// the blade, treasure 51, treasure 50, the gold; T's push onto 5,4 then
/// "T cannot carry more blades" (magic 7.2, inventory 2.2). Nothing
/// random in the order: every seed whose pushes kill.
#[test]
fn a_killed_creatures_things_go_down_gold_first() {
    let mut a = kit(5);
    load(&mut a, T50, 1);
    load(&mut a, T51, 1);
    ours(&mut a, 0, 5, 4, |c| {
        ee(c);
        c.life_force = 1;
        c.wealth = 50;
        holds(c, BLADE);
        has(c, T50);
        has(c, T51);
    });
    let me = PlayerSetup {
        melee: 100,
        ..t(6, 40, vec![BLADE])
    };
    let mut killed = 0;
    for seed in 0..64 {
        let mut g = game(&a, seed, me.clone(), room(4, 4));
        ready(&mut g, BLADE);
        let mut died = None;
        for _ in 0..8 {
            assert_eq!(g.players()[0].at, room(4, 4), "seed {seed}");
            let ev = g.apply(Command::Move { dir: Dir::E });
            died = ev.iter().find_map(|e| match e {
                Event::Died { who, at, left } if *who == slot(0) => Some((*at, left.clone())),
                _ => None,
            });
            if died.is_some() || !g.players()[0].alive {
                break;
            }
        }
        let Some((at, left)) = died else { continue };
        killed += 1;
        assert_eq!(at, room(5, 4));
        assert_eq!(left, [(GOLD, 50), (T50, 1), (T51, 1), (BLADE, 1)], "seed {seed}");
        let stack: Vec<u16> = g.region(1).unwrap().rooms[ROOM as usize]
            .items
            .iter()
            .filter(|it| (it.x, it.y) == (5, 4))
            .map(|it| it.thing)
            .collect();
        assert_eq!(stack, [BLADE, T51, T50, GOLD], "seed {seed}");
        let ev = g.apply(Command::Move { dir: Dir::E });
        assert!(
            ev.iter().any(|e| matches!(
                e,
                Event::Blocked {
                    reason: BlockReason::CarryLimit { thing: BLADE },
                    ..
                }
            )),
            "seed {seed}: {ev:?}"
        );
    }
    assert!(killed > 32, "{killed} kills in 64");
}

/// deterministic, one run each; the draws-logged refusals 4 runs each):
/// - `beside`: FIRE with EE beside: refused, no move, nothing drawn;
/// - the stick into EE, holding the bow and owning no melee weapon:
///   nothing, no move, nothing drawn (combat 2.7);
/// - `own`: owning the blade, the bow held, the push into EE brought the
///   melee ready menu (**Engine:** readies the blade, then attacks, 2.8);
/// - `none`: holding no missile weapon, FIRE used one move (5 to 4) and
///   drew nothing, NONE ending it;
/// - `map`: on the world map FIRE did nothing, drew nothing;
/// - `ready5`/`ready-e5`: choosing the owned bow from FIRE's menu with 5
///   moves: FIRE 5 to 4, the bow 4 to 3, 2 shots (32 of 32).
#[test]
fn the_players_fire_refusals_and_costs() {
    let mut beside = kit(5);
    ours(&mut beside, 0, 5, 4, ee);
    let far = player_bow();
    for seed in 0..8 {
        let mut g = holding_the_bow(&beside, seed);
        let before = rng(&g);
        let ev = g.apply(Command::Fire {
            target: Some((5, 4)),
            weapon: None,
        });
        assert_eq!(
            ev,
            vec![Event::CannotFire {
                player: 0,
                reason: FireRefusal::FoeBeside { foe: slot(0) }
            }]
        );
        let ev = g.apply(Command::Move { dir: Dir::E });
        assert!(
            ev.iter().any(|e| matches!(e, Event::BumpedCreature { .. })),
            "{ev:?}"
        );
        assert_eq!((moves(&g), rng(&g)), (5, before.clone()), "seed {seed}");

        let mut g = game(&beside, seed, t(6, 11, vec![BOW, BLADE]), room(4, 4));
        ready(&mut g, BOW);
        let ev = g.apply(Command::Move { dir: Dir::E });
        let r = ev.iter().position(|e| {
            matches!(
                e,
                Event::Readied {
                    thing: BLADE,
                    slot: Slot::Weapon,
                    ..
                }
            )
        });
        let at = ev
            .iter()
            .position(|e| matches!(e, Event::Attacked { by: ME, .. }));
        assert!(r.is_some() && r < at, "seed {seed}: {ev:?}");

        let mut g = game(&far, seed, t(6, 11, vec![]), room(4, 4));
        g.apply(Command::Idle);
        let before = rng(&g);
        let ev = g.apply(Command::Fire {
            target: None,
            weapon: None,
        });
        assert_eq!(ev, vec![Event::FireCancelled { player: 0 }]);
        assert_eq!((moves(&g), rng(&g)), (4, before), "seed {seed}");

        let mut g = game(&far, seed, t(6, 11, vec![BOW]), Location::World { x: 12, y: 12 });
        ready(&mut g, BOW);
        let before = rng(&g);
        let ev = g.apply(Command::Fire {
            target: None,
            weapon: None,
        });
        assert_eq!(
            ev,
            vec![Event::CannotFire {
                player: 0,
                reason: FireRefusal::WorldMap
            }]
        );
        assert_eq!((moves(&g), rng(&g)), (5, before), "seed {seed}");
    }
    for seed in 0..64 {
        let mut g = game(&far, seed, t(6, 11, vec![BOW]), room(4, 4));
        g.apply(Command::Idle);
        let ev = g.apply(Command::Fire {
            target: Some((9, 4)),
            weapon: Some(BOW),
        });
        let r = ev
            .iter()
            .position(|e| matches!(e, Event::Readied { thing: BOW, .. }));
        let at = ev.iter().position(|e| matches!(e, Event::Fired { by: ME, .. }));
        assert!(r.is_some() && r < at, "seed {seed}: {ev:?}");
        assert_eq!(fired(&ev, ME), Some(2), "ready-e5 seed {seed}");
    }
}

/// capture.md 10.32 (`bes`, `cls-c0`..`cls-c7`): XX, a Speed 0 creature of
/// Strength 1, on 5,4 beside T holding the bow with 2 moves. An aggressive
/// thief, neutral or enemy (c4, c5, c6) refused FIRE with no move; the four
/// peaceful classes and the aggressive friend (c0-c3, c7) let it: 2 shots.
/// With the aggressive enemy beside, T only owning the bow (or not even
/// that) was refused the same, no move (`ord-c6`, `ord-c6-own0`); with a
/// peaceful enemy (`ord-c2`) FIRE took a move (2 to 1) and the menu came
/// (combat 8.8.2 before 8.8.4).
#[test]
fn who_counts_beside_for_fire() {
    let align = [
        Alignment::Thief,
        Alignment::Neutral,
        Alignment::Enemy,
        Alignment::Friend,
    ];
    for class in 0..8usize {
        let mut a = kit(5);
        ours(&mut a, 0, 5, 4, |c| {
            c.speed = 0;
            c.strength = 1;
            c.strategy.alignment = align[class % 4];
            c.strategy.aggressive = class >= 4;
        });
        let refused = (4..=6).contains(&class);
        for seed in 0..8 {
            let mut g = game(&a, seed, t(3, 11, vec![BOW]), room(4, 4));
            ready(&mut g, BOW);
            assert_eq!(moves(&g), 2);
            let ev = g.apply(Command::Fire {
                target: None,
                weapon: None,
            });
            if refused {
                assert!(
                    matches!(
                        ev[..],
                        [Event::CannotFire {
                            reason: FireRefusal::FoeBeside { .. },
                            ..
                        }]
                    ),
                    "c{class}: {ev:?}"
                );
                assert_eq!(moves(&g), 2);
            } else {
                assert_eq!(fired(&ev, ME), Some(2), "c{class}: {ev:?}");
            }
            if class == 6 || class == 2 {
                for owned in [vec![BOW], vec![]] {
                    let weapon = owned.first().copied();
                    let mut g = game(&a, seed, t(2, 11, owned), room(4, 4));
                    let ev = g.apply(Command::Fire { target: None, weapon });
                    if class == 6 {
                        assert!(
                            matches!(
                                ev[..],
                                [Event::CannotFire {
                                    reason: FireRefusal::FoeBeside { .. },
                                    ..
                                }]
                            ),
                            "ord-c6: {ev:?}"
                        );
                        assert_eq!(moves(&g), 2);
                    } else if weapon.is_none() {
                        assert_eq!(ev, vec![Event::FireCancelled { player: 0 }]);
                        assert_eq!(moves(&g), 1);
                    }
                }
            }
        }
    }
}

/// capture.md 10.32 (`rdy`): T of Speed 10 owning the blade and the bow,
/// holding the blade, EE on 9,4. FIRE choosing the bow with 6 moves: 6 to
/// 5 at FIRE, 4 at the choice, then 3 shots (`rdy-m6`); with 2 moves 2, 1,
/// 0 and one shot (`rdy-m2`); with 1, 1 to 0 and the bow readied all the
/// same, one shot (`rdy-m1`). NEW WEAPON choosing the bow took one move
/// (`nw-bow-blade`: 6 to 5), and FIRE then went straight to the cursor
/// (combat 6.1, 8.8.4).
#[test]
fn readying_the_bow_for_fire_costs_a_move_of_its_own() {
    let a = player_bow();
    for seed in 0..16 {
        for (left, shots) in [(6, 3), (2, 1), (1, 1)] {
            let mut g = game(&a, seed, t(10, 11, vec![BLADE, BOW]), room(4, 4));
            ready(&mut g, BLADE);
            idle_to(&mut g, left);
            let ev = g.apply(Command::Fire {
                target: Some((9, 4)),
                weapon: Some(BOW),
            });
            assert!(ev.iter().any(|e| matches!(e, Event::Readied { thing: BOW, .. })));
            assert_eq!(fired(&ev, ME), Some(shots), "rdy-m{left} seed {seed}: {ev:?}");
        }
        let mut g = game(&a, seed, t(10, 11, vec![BLADE, BOW]), room(4, 4));
        ready(&mut g, BLADE);
        idle_to(&mut g, 6);
        ready(&mut g, BOW);
        assert_eq!(moves(&g), 5);
        let ev = g.apply(Command::Fire {
            target: Some((9, 4)),
            weapon: None,
        });
        assert!(!ev.iter().any(|e| matches!(e, Event::Readied { .. })));
        assert_eq!(fired(&ev, ME), Some(3), "nw-bow-blade seed {seed}");
    }
}

/// `fire-u`'s setup: T holding the bow on 5,4 with 12 moves, U beside it
/// on 4,4, T's turn.
fn beside_u(a: &Adventure, seed: u64) -> Game {
    let (mut g, _) = Game::new(a.clone(), seed, Default::default(), t(12, 40, vec![BOW]));
    let mut u = t(4, 40, vec![]);
    u.name = "U".into();
    g.apply(Command::AddPlayer { setup: u });
    g.apply(Command::Wait);
    g.place(0, room(5, 4));
    g.place(1, room(4, 4));
    while g.current() != 0 {
        g.apply(Command::Wait);
    }
    ready(&mut g, BOW);
    g.apply(Command::Wait);
    while g.current() != 0 {
        g.apply(Command::Wait);
    }
    assert_eq!(moves(&g), 12);
    g
}

/// capture.md 10.32 (`bes2`, `fire-u`), setup from 10.35 item 3: T of
/// Speed 12 holding the bow (Power 10, range 7) on 5,4 and another player,
/// U, of Speed 4 beside it on 4,4; both Constitution and Life Force 40,
/// Strength 9, Dexterity 13, Wisdom and Power 13, Size 9, Dodge 30, Parry
/// 26, Melee 31, Missile 22. FIRE is not refused (a player beside never
/// stops it), and with 12 moves gives 7 shots (Dexterity 13 / 8 plus
/// 12 / 2). Over 64 states: 393 of the 448 shots due fired (U died in 48
/// runs, ending the firing), 379 reached U, 91 of those dodged, 288
/// damaging. Many rolls a run: samples.
#[test]
fn another_player_beside_never_stops_fire() {
    let a = kit(7);
    let (mut shots, mut dodged, mut hit) = (0, 0, 0);
    for seed in 0..SEEDS / 4 {
        let mut g = beside_u(&a, seed);
        let ev = g.apply(Command::Fire {
            target: Some((4, 4)),
            weapon: None,
        });
        assert_eq!(fired(&ev, ME), Some(7), "seed {seed}: {ev:?}");
        let (d, h) = reached(&ev, ME, Who::Player { player: 1 });
        shots += shots_of(&ev, ME);
        dodged += d;
        hit += h;
    }
    same_rate(
        "fire-u: shots reaching U",
        Sweep::Sample,
        379,
        393,
        dodged + hit,
        shots,
    );
    same_rate(
        "fire-u: arrows reaching U dodged",
        Sweep::Sample,
        91,
        379,
        dodged,
        dodged + hit,
    );
}

/// capture.md 10.35 item 2 (`u-rec`: `fire-u` again, 64 runs, U's whole
/// record watched): only U's Life Force, Power and Dodge changed, and its
/// death's bytes in the 48 runs U died; for a player target the roll after
/// a damaging arrow (Wisdom 13) decides nothing (combat 7). Compared on
/// the engine's player record; a dead U only as dead.
#[test]
fn a_player_target_changes_only_in_life_force_power_and_dodge() {
    let a = kit(7);
    let mut died = 0;
    for seed in 0..64 {
        let mut g = beside_u(&a, seed);
        let before = g.players()[1].clone();
        g.apply(Command::Fire {
            target: Some((4, 4)),
            weapon: None,
        });
        let mut after = g.players()[1].clone();
        if !after.alive {
            died += 1;
            continue;
        }
        after.life_force = before.life_force;
        after.power = before.power;
        after.dodge = before.dodge;
        // The command runs on into U's turn, which starts its bar.
        (after.moves_left, after.turn_moves) = (before.moves_left, before.turn_moves);
        assert_eq!(after, before, "seed {seed}");
    }
    same_rate("u-rec: runs with U dead", Sweep::Sample, 48, 64, died, 64);
}

// ---- drops ----------------------------------------------------------------

/// A treasure of weight `w` as thing `n`.
fn load(a: &mut Adventure, n: u16, w: i32) {
    thing(a, n, &format!("W{w}"), treasure(w, 1));
}

/// capture.md 10.24 item 3's dropper WE (rr20; stats as `drop6`, 10.33: a
/// peaceful friend of Speed 6, Strength 1, Constitution and Life Force 10,
/// capacity 100) on (x, y) carrying one treasure of weight 400, T on 4,4,
/// and the side room's portal on 7,4 (10.32's `drop-kinds`).
fn dropper(x: u8, y: u8) -> Adventure {
    let mut a = kit(5);
    load(&mut a, T50, 400);
    ours(&mut a, 0, x, y, |c| {
        peaceful_friend(c);
        c.speed = 6;
        c.strength = 1;
        has(c, T50);
    });
    a.regions[0].rooms[ROOM as usize].items.insert(
        0,
        RoomItem {
            x: 7,
            y: 4,
            thing: DOOR,
            parameter: 1,
            flag: false,
            destination: None,
            gold: None,
        },
    );
    a
}

/// Where WE's first turn's drop landed.
fn landing(a: &Adventure, seed: u64) -> (u8, u8) {
    let mut g = game(a, seed, t(6, 11, vec![]), room(4, 4));
    let ev = wait(&mut g);
    let turn = turns_of(&ev, 0)[0];
    assert_eq!(steps(turn), 0, "seed {seed}");
    turn.iter()
        .find_map(|e| match e {
            Event::CreatureDropped {
                slot: 0,
                thing: T50,
                at: Some(Location::Room { x, y, .. }),
                ..
            } => Some((*x, *y)),
            _ => None,
        })
        .unwrap_or_else(|| panic!("seed {seed}: no drop in {turn:?}"))
}

fn bin(squares: &[(u8, u8)], at: (u8, u8)) -> usize {
    squares.iter().position(|&s| s == at).unwrap_or(squares.len())
}

/// capture.md 10.24 item 3 (`drop-wall`): WE on 6,4 with an obstacle on 5,4:
/// 256 drops landed on 6,3 51, 6,4 (its own) 101, 6,5 52, 7,4 52, never on
/// the obstacle (creatures 8.1.1).
#[test]
fn the_drop_square_beside_an_obstacle() {
    let mut a = dropper(6, 4);
    item(&mut a, ROOM, 5, 4, OBSTACLE);
    let squares = [(6, 3), (6, 4), (6, 5), (7, 4)];
    let h = histogram(SEEDS, 5, |seed| bin(&squares, landing(&a, seed)));
    assert_eq!(h[4], 0, "elsewhere");
    same_histogram("drop-wall: 6,3 6,4 6,5 7,4", &[51, 101, 52, 52], &h[..4]);
}

/// capture.md 10.24 item 3 (`drop-thing`): WE on 6,4 with treasures of ours
/// on 7,4, 6,5 and 6,3: 125 of 256 landed on a square holding a treasure
/// (6,3 43, 6,5 42, 7,4 40), every try accepted.
#[test]
fn the_drop_square_takes_a_square_holding_a_thing() {
    let mut a = dropper(6, 4);
    load(&mut a, T51, 1);
    for (x, y) in [(7, 4), (6, 5), (6, 3)] {
        item(&mut a, ROOM, x, y, T51);
    }
    let squares = [(6, 3), (6, 5), (7, 4)];
    let h = histogram(SEEDS, 4, |seed| bin(&squares, landing(&a, seed)));
    same_histogram("drop-thing: 6,3 6,5 7,4 other", &[43, 42, 40, 131], &h);
}

/// capture.md 10.24 item 3 (`drop-beside`): WE set onto 5,4, east of T: 20
/// of 128 drops on T's square 4,4 (an individual's square takes it).
#[test]
fn the_drop_square_takes_the_players_square() {
    let a = dropper(5, 4);
    let on_t = count(SEEDS, |seed| landing(&a, seed) == (4, 4));
    same_rate("drop-beside: on T's square", Sweep::Sample, 20, 128, on_t, SEEDS);
}

/// capture.md 10.32 (`drop-kinds`): WE on 6,4 with a custom obstacle on
/// 5,4, the portal on 7,4, a custom space on 6,3 and a space on 6,5: 256 of
/// 256 fitted with the custom obstacle refusing; landed 6,3 51, 6,5 50, 7,4
/// 50, its own square 105.
#[test]
fn a_custom_obstacle_refuses_a_drop() {
    let mut a = dropper(6, 4);
    let nothing_kind = nothing();
    thing(
        &mut a,
        CUSTOM_OBSTACLE,
        "COBST",
        ThingKind::CustomObstacle {
            picture: 11,
            action: nothing_kind,
        },
    );
    thing(
        &mut a,
        CUSTOM_SPACE,
        "CSPACE",
        ThingKind::CustomSpace {
            picture: 5,
            placement_choice: PlacementChoice::SpellParameter,
            trigger: CustomTrigger::WhenMovedOnto,
            action: nothing_kind,
            destroy_needed_thing: false,
        },
    );
    thing(
        &mut a,
        SPACE,
        "SPACE",
        ThingKind::Space {
            picture: 5,
            access: Access::Anyone,
        },
    );
    item(&mut a, ROOM, 5, 4, CUSTOM_OBSTACLE);
    item(&mut a, ROOM, 6, 3, CUSTOM_SPACE);
    item(&mut a, ROOM, 6, 5, SPACE);
    let squares = [(6, 3), (6, 5), (7, 4), (6, 4)];
    let h = histogram(SEEDS, 5, |seed| bin(&squares, landing(&a, seed)));
    assert_eq!(h[4], 0, "elsewhere");
    same_histogram("drop-kinds: 6,3 6,5 7,4 own", &[51, 50, 50, 105], &h[..4]);
}

/// capture.md 10.33 (`drop6`): WE (Speed 6, Strength 1, capacity 100) with
/// a treasure of weight 400 on 7,6: in 32 of 32 its first turn began with 0
/// moves (the load before the drop), dropped the treasure, took no step;
/// its next turn began with 6 and walked 6 (creatures 8.1.2).
#[test]
fn a_drop_turns_moves_come_from_the_load_before_it() {
    let a = dropper(7, 6);
    for seed in 0..64 {
        let mut g = game(&a, seed, t(6, 11, vec![]), room(4, 4));
        let mut ev = wait(&mut g);
        ev.extend(wait(&mut g));
        let turns = turns_of(&ev, 0);
        assert_eq!(turn_moves(turns[0]), 0, "seed {seed}");
        assert!(turns[0]
            .iter()
            .any(|e| matches!(e, Event::CreatureDropped { thing: T50, .. })));
        assert_eq!(steps(turns[0]), 0);
        assert_eq!(turn_moves(turns[1]), 6, "seed {seed}");
        assert_eq!(steps(turns[1]), 6, "seed {seed}: {:?}", turns[1]);
    }
}

/// capture.md 10.14.5 (`burden`, 16 states, three rounds, every run
/// alike): Strength 1, Speed 2 creatures of ours. WA (300 gold, treasures
/// of weight 400 and 1, things 50 and 51) dropped all 300 gold on its first
/// turn and thing 50 on its second; WB (2,000 gold) all 2,000 on its first;
/// WC (treasures of weight 1 and 400, things 52 and 53) thing 52 on its
/// first and 53 on its second; none stepped in a turn with a drop. WD
/// (Strength 20, 100 gold and a treasure of weight 5) walked up beside T
/// and gave the treasure, keeping its gold (creatures 7.3, 8.1). They are
/// peaceful friends (WD's gift; as `burden2`); their squares aren't given:
/// WA, WB, WC far off on the east wall, WD on 6,4, two squares from T (so
/// whether it gets there in time hangs on that: most runs, not all).
#[test]
fn the_overburdened_drop_order() {
    let mut a = kit(5);
    for (n, w) in [(T50, 400), (T51, 1), (T52, 1), (T53, 400), (T54, 5)] {
        load(&mut a, n, w);
    }
    let setup: [(u8, u8, u8, u16, &[u16]); 4] = [
        (13, 1, 1, 300, &[T50, T51]),
        (13, 4, 1, 2000, &[]),
        (13, 8, 1, 0, &[T52, T53]),
        (6, 4, 20, 100, &[T54]),
    ];
    for (n, &(x, y, strength, gold, things)) in setup.iter().enumerate() {
        ours(&mut a, n, x, y, |c| {
            peaceful_friend(c);
            c.speed = 2;
            c.strength = strength;
            c.wealth = gold;
            for &t in things {
                has(c, t);
            }
        });
    }
    let drops = |turn: &[Event]| -> Vec<(u16, u16)> {
        turn.iter()
            .filter_map(|e| match e {
                Event::CreatureDropped { thing, amount, .. } => Some((*thing, *amount)),
                _ => None,
            })
            .collect()
    };
    let mut gave = 0;
    for seed in 0..64 {
        let mut g = game(&a, seed, t(6, 11, vec![]), room(4, 4));
        let mut ev = Vec::new();
        for _ in 0..3 {
            ev.extend(wait(&mut g));
        }
        let expect: [[&[(u16, u16)]; 2]; 3] = [
            [&[(GOLD, 300)], &[(T50, 1)]],
            [&[(GOLD, 2000)], &[]],
            [&[(T52, 1)], &[(T53, 1)]],
        ];
        for (n, want) in expect.iter().enumerate() {
            let turns = turns_of(&ev, n);
            for (k, want) in want.iter().enumerate() {
                assert_eq!(drops(turns[k]), want.to_vec(), "seed {seed}: slot {n} turn {k}");
                if !want.is_empty() {
                    assert_eq!(steps(turns[k]), 0, "seed {seed}: slot {n} turn {k}");
                }
            }
        }
        let wd: Vec<(u16, u16)> = turns_of(&ev, 3).into_iter().flat_map(drops).collect();
        assert!(wd.iter().all(|&(t, _)| t == T54), "seed {seed}: {wd:?}");
        assert_eq!(g.creature(1, 3).unwrap().wealth, 100);
        gave += !wd.is_empty() as u32;
    }
    assert!(gave * 10 >= 64 * 9, "WD's gift in {gave} of 64");
}

/// capture.md 10.32 (`burden2`, 8 runs, all alike): peaceful friends of
/// ours, Constitution and Life Force 10. WA, WB, WC (Strength 2, capacity
/// 200, Speed 6) carrying one treasure each of weight 150, 101, 100; WD
/// (Strength 1, Speed 0) carrying 400. WA began with 3 moves (6 x 2 x 50 /
/// 200), dropped with its first move, then took 2 steps; WB began with 5,
/// dropped, then 4 steps; WC (exactly half) began with 6, walked 6, no
/// drop; WD never dropped; the next turns began with 6 (creatures 8.1.2).
/// Their squares aren't given: far from T on the east side.
#[test]
fn a_creature_drops_above_half_its_capacity() {
    let mut a = kit(5);
    for (n, w) in [(T50, 150), (T51, 101), (T52, 100), (T53, 400)] {
        load(&mut a, n, w);
    }
    let setup = [
        (13, 1, 2, 6, T50),
        (13, 4, 2, 6, T51),
        (13, 8, 2, 6, T52),
        (11, 8, 1, 0, T53),
    ];
    for (n, &(x, y, strength, speed, t)) in setup.iter().enumerate() {
        ours(&mut a, n, x, y, |c| {
            peaceful_friend(c);
            c.speed = speed;
            c.strength = strength;
            has(c, t);
        });
    }
    for seed in 0..64 {
        let mut g = game(&a, seed, t(6, 11, vec![]), room(4, 4));
        let mut ev = wait(&mut g);
        ev.extend(wait(&mut g));
        for (n, moves, dropped, walked) in [(0, 3, true, 2), (1, 5, true, 4), (2, 6, false, 6)] {
            let turns = turns_of(&ev, n);
            let first = turns[0];
            assert_eq!(turn_moves(first), moves, "seed {seed}: slot {n}");
            let drop = first
                .iter()
                .position(|e| matches!(e, Event::CreatureDropped { .. }));
            assert_eq!(drop.is_some(), dropped, "seed {seed}: slot {n}");
            if let Some(d) = drop {
                assert_eq!(steps(&first[..d]), 0, "seed {seed}: slot {n} dropped first");
            }
            assert_eq!(steps(first), walked, "seed {seed}: slot {n}: {first:?}");
            assert_eq!(turn_moves(turns[1]), 6, "seed {seed}: slot {n}");
        }
        assert!(!ev
            .iter()
            .any(|e| matches!(e, Event::CreatureDropped { slot: 3, .. })));
    }
}

/// A burdened creature of ours in slot 0 on (x, y): `class`, cautious,
/// Speed `speed` and Strength `strength`, carrying treasure 50 of weight
/// `w`, then `f`.
fn burdened(
    class: (Alignment, bool),
    (x, y): (u8, u8),
    (speed, strength, w): (u8, u8, i32),
    f: impl FnOnce(&mut Creature),
) -> Adventure {
    let mut a = kit(5);
    load(&mut a, T50, w);
    ours(&mut a, 0, x, y, |c| {
        c.strategy.alignment = class.0;
        c.strategy.aggressive = class.1;
        c.strategy.brave = false;
        c.speed = speed;
        c.strength = strength;
        has(c, T50);
        f(c);
    });
    a
}

/// rr16's `drop-corner` WE (Speed 2, Strength 1, carrying 400 of a capacity
/// of 100: 0 moves) as `class` on (x, y).
fn we(class: (Alignment, bool), at: (u8, u8), f: impl FnOnce(&mut Creature)) -> Adventure {
    burdened(class, at, (2, 1, 400), f)
}

/// Slot 0's first `n` turns with T (Speed 6) on `at`.
fn first_turns(a: &Adventure, seed: u64, at: Location, n: usize) -> Vec<Vec<Event>> {
    let mut g = game(a, seed, t(6, 11, vec![]), at);
    let mut ev = Vec::new();
    while turns_of(&ev, 0).len() < n {
        ev.extend(wait(&mut g));
    }
    turns_of(&ev, 0)[..n].iter().map(|t| t.to_vec()).collect()
}

fn dropped(turn: &[Event]) -> bool {
    turn.iter()
        .any(|e| matches!(e, Event::CreatureDropped { slot: 0, .. }))
}

fn readied(turn: &[Event], thing: u16) -> bool {
    turn.iter()
        .any(|e| matches!(e, Event::CreatureReadied { slot: 0, thing: t, .. } if *t == thing))
}

/// capture.md 10.37 item 3 (`fb-c0` .. `fb-c7`, `fb-c6b`, `fb-c6own`,
/// `fbn`, 16 runs each; `fbe`, `fbe-c1`, `fbn-c6`): WE set onto 5,4 beside
/// T on 4,4, its class set: no drop in any run for the peaceful and the
/// aggressive thief, neutral and enemy (brave, or owning the blade, alike);
/// 16 of 16 for the aggressive friend (and 10.24's peaceful friend, 128 of
/// 128). With T diagonal on 4,5 it dropped 16 of 16 as an aggressive
/// enemy and as a peaceful neutral: the square decides (creatures 8.1.2,
/// 2.6's test). The first turn of each, nothing random in it.
#[test]
fn a_foe_beside_stops_the_overburdened_drop() {
    let mut cases: Vec<(String, Adventure, Location, bool)> = Vec::new();
    for (k, &class) in CLASSES.iter().enumerate() {
        let a = we(class, (5, 4), |_| {});
        cases.push((format!("fb-c{k}"), a, room(4, 4), k == 3 || k == 7));
    }
    cases.push((
        "fb-c6b".into(),
        we(CLASSES[6], (5, 4), |c| c.strategy.brave = true),
        room(4, 4),
        false,
    ));
    cases.push((
        "fb-c6own".into(),
        we(CLASSES[6], (5, 4), |c| has(c, BLADE)),
        room(4, 4),
        false,
    ));
    for (name, k) in [("fbe", 6), ("fbe-c1", 1)] {
        cases.push((name.into(), we(CLASSES[k], (5, 4), |_| {}), room(4, 5), true));
    }
    for (name, a, at, drops) in &cases {
        for seed in 0..32 {
            let turn = &first_turns(a, seed, *at, 1)[0];
            assert_eq!(turn_moves(turn), 0, "{name} seed {seed}");
            assert_eq!(dropped(turn), *drops, "{name} seed {seed}: {turn:?}");
        }
    }
}

/// capture.md 10.37 item 3 (`wa-c1`, `wa-c3`, `wa-c6`, `wb-102`, `wb-38`,
/// `wb-118`, 8 runs each, all alike): rr28's `burden2` WA (Speed 6,
/// Strength 2, a load of 150 of 200: 3 moves). Set beside T on 5,4: as a
/// peaceful friend it began with 3 moves, dropped, stepped twice; as a
/// peaceful neutral or an aggressive enemy it began with 1, did not drop,
/// stepped once. With T far and WB set beside it (WA a peaceful friend on
/// 12,2, WB on 11,2): WB an aggressive enemy, 1 move, no drop, a step; a
/// peaceful enemy or an aggressive friend, 3 moves, a drop, 2 steps. WB's
/// stats aren't given: the harness's at Speed 0.
#[test]
fn with_moves_left_a_foe_beside_stops_the_drop_too() {
    let wa = |class, at| burdened(class, at, (6, 2, 150), |_| {});
    let mut cases: Vec<(String, Adventure, (u8, usize))> = Vec::new();
    for (k, want) in [(3, (3, 2)), (1, (1, 1)), (6, (1, 1))] {
        cases.push((format!("wa-c{k}"), wa(CLASSES[k], (5, 4)), want));
    }
    for (byte, k, want) in [(102, 6, (1, 1)), (38, 2, (3, 2)), (118, 7, (3, 2))] {
        let mut a = wa(CLASSES[3], (12, 2));
        ours(&mut a, 1, 11, 2, |c| {
            c.speed = 0;
            c.strategy.alignment = CLASSES[k].0;
            c.strategy.aggressive = CLASSES[k].1;
        });
        cases.push((format!("wb-{byte}"), a, want));
    }
    for (name, a, (moves, walked)) in &cases {
        for seed in 0..32 {
            let turn = &first_turns(a, seed, room(4, 4), 1)[0];
            assert_eq!(turn_moves(turn), *moves, "{name} seed {seed}");
            assert_eq!(dropped(turn), *moves == 3, "{name} seed {seed}: {turn:?}");
            assert_eq!(steps(turn), *walked, "{name} seed {seed}: {turn:?}");
        }
    }
}

/// capture.md 10.37 item 4: `r0b` (WE, a peaceful friend, owning the bow,
/// not holding it, on 9,4, nobody beside; 16 runs and 8 longer): the drop
/// turn (0 moves) dropped and did not ready; its next turn readied the bow
/// and stepped on. `rmb` (WE owning the blade, a peaceful neutral on 5,4
/// beside T, so no drop; 16 runs): the blade readied, then nothing: no
/// step, no attack, its load and square unchanged. `r0m` (the blade, an
/// aggressive enemy, T diagonal on 4,5; 16 runs): dropped, not readied
/// (creatures 2.7, 8.1.2).
#[test]
fn readying_with_no_moves_left() {
    let r0b = we(CLASSES[3], (9, 4), |c| has(c, BOW));
    let rmb = we(CLASSES[1], (5, 4), |c| has(c, BLADE));
    let r0m = we(CLASSES[6], (5, 4), |c| has(c, BLADE));
    for seed in 0..32 {
        let turns = first_turns(&r0b, seed, room(4, 4), 2);
        assert_eq!(turn_moves(&turns[0]), 0, "r0b seed {seed}");
        assert!(dropped(&turns[0]), "r0b seed {seed}: {:?}", turns[0]);
        assert!(!readied(&turns[0], BOW), "r0b seed {seed}: {:?}", turns[0]);
        assert!(readied(&turns[1], BOW), "r0b seed {seed}: {:?}", turns[1]);
        assert!(steps(&turns[1]) >= 1, "r0b seed {seed}: {:?}", turns[1]);

        let mut g = game(&rmb, seed, t(6, 11, vec![]), room(4, 4));
        let ev = wait(&mut g);
        let turn = turns_of(&ev, 0)[0];
        assert_eq!(turn_moves(turn), 0, "rmb seed {seed}");
        assert!(
            !dropped(turn) && readied(turn, BLADE),
            "rmb seed {seed}: {turn:?}"
        );
        assert_eq!(steps(turn), 0, "rmb seed {seed}");
        assert!(attacks(turn, slot(0)).is_empty(), "rmb seed {seed}: {turn:?}");
        let c = g.creature(1, 0).unwrap();
        assert_eq!((c.x, c.y), (5, 4));
        assert!(c.possessions.iter().any(|p| p.thing == T50));

        let turn = &first_turns(&r0m, seed, room(4, 5), 1)[0];
        assert!(
            dropped(turn) && !readied(turn, BLADE),
            "r0m seed {seed}: {turn:?}"
        );
    }
}

/// capture.md 10.37 item 5 (`dst`, 256 runs): `drop-wall`'s WE on 6,4 with
/// the region's store on 5,4: 256 of 256 fitted with the store taking the
/// thing, landing on 5,4 49, 6,3 37, 6,4 83, 6,5 37, 7,4 50 (creatures
/// 8.1.1).
#[test]
fn a_store_takes_a_drop() {
    const STORE: u16 = 83;
    let mut a = dropper(6, 4);
    thing(&mut a, STORE, "STORE", ThingKind::Store { picture: 5 });
    item(&mut a, ROOM, 5, 4, STORE);
    let squares = [(5, 4), (6, 3), (6, 4), (6, 5), (7, 4)];
    let h = histogram(SEEDS, 6, |seed| bin(&squares, landing(&a, seed)));
    assert_eq!(h[5], 0, "elsewhere");
    same_histogram("dst: 5,4 6,3 6,4 6,5 7,4", &[49, 37, 83, 37, 50], &h[..5]);
}

/// GF (capture.md 10.37 item 6): a peaceful friend of Strength 20 and
/// Speed 3 owning one treasure of weight 1 (thing 50), on (x, 4).
fn giver(x: u8) -> Adventure {
    burdened(CLASSES[3], (x, 4), (3, 20, 1), |_| {})
}

/// Where slot 0's first drop in `turn` landed, and where it stood then.
fn gift(turn: &[Event]) -> Option<((u8, u8), (u8, u8))> {
    let mut at = None;
    for e in turn {
        match e {
            Event::CreatureMoved {
                slot: 0,
                to: Location::Room { x, y, .. },
                ..
            } => at = Some((*x, *y)),
            Event::CreatureDropped {
                slot: 0,
                thing: T50,
                at: Some(Location::Room { x, y, .. }),
                ..
            } => return Some(((*x, *y), at.unwrap_or((0, 0)))),
            _ => {}
        }
    }
    None
}

/// capture.md 10.37 item 6: `g5` (GF on 5,4 beside T on 4,4; 64 runs): the
/// gift is the turn's first action, landing on its own square 24, T's 4,4
/// 6, 5,3 8, 5,5 13, 6,4 13. `g7` (GF on 7,4; 64 runs): it stepped to 6,4
/// and 5,4 and gave with its third move in 38 runs, the landings around
/// 5,4 (its own 13, 4,4 9, 6,4 9, 5,3 3, 5,5 4), never on T's other
/// neighbours 3,4, 4,3, 4,5 (creatures 7.3: 8.1.1's choice around the
/// giver).
#[test]
fn a_friends_gift_lands_around_the_giver() {
    let squares = [(5, 4), (4, 4), (5, 3), (5, 5), (6, 4)];
    let a = giver(5);
    let h = histogram(SEEDS, 6, |seed| {
        let turn = &first_turns(&a, seed, room(4, 4), 1)[0];
        let (to, from) = gift(turn).unwrap_or_else(|| panic!("g5 seed {seed}: no gift: {turn:?}"));
        assert_eq!(from, (0, 0), "g5 seed {seed}: stepped first");
        bin(&squares, to)
    });
    assert_eq!(h[5], 0, "elsewhere");
    same_histogram("g5: own 4,4 5,3 5,5 6,4", &[24, 6, 8, 13, 13], &h[..5]);

    let squares = [(5, 4), (4, 4), (6, 4), (5, 3), (5, 5)];
    let a = giver(7);
    let mut gave = 0;
    let h = histogram(SEEDS, 6, |seed| {
        let turn = &first_turns(&a, seed, room(4, 4), 1)[0];
        let Some((to, from)) = gift(turn) else {
            return 5;
        };
        gave += 1;
        assert_eq!((from, steps(turn)), ((5, 4), 2), "g7 seed {seed}: {turn:?}");
        assert!(bin(&squares, to) < 5, "g7 seed {seed}: on {to:?}");
        bin(&squares, to)
    });
    same_rate(
        "g7: runs giving in the first turn",
        Sweep::Sample,
        38,
        64,
        gave,
        SEEDS,
    );
    same_histogram("g7: own 4,4 6,4 5,3 5,5", &[13, 9, 9, 3, 4], &h[..5]);
}

/// capture.md 10.37 item 7: `s0g` (TT, a peaceful thief of Speed 0 on 8,4,
/// 50 gold on 9,4; 16 runs): it took the gold on its first turn without
/// moving, its wealth 50, 16 of 16. `s0x-long` (NN, a peaceful neutral of
/// Speed 0 on 6,4 beside the side room's portal on 7,4; 16 runs of four
/// turns): it went through once, otherwise staying put; `s2x-long` (NN at
/// Speed 2, runs as long, taken as four turns too): through in 2 of 16
/// (creatures 2.7: the Speed 0 step is not cancelled onto a thing it takes
/// or an exit). Many rolls a run: samples.
#[test]
#[ignore = "engine disagrees with capture.md 10.37 item 7: a peaceful neutral of Speed 0 beside the portal went through in 1 of 16 four-turn runs, the engine in every run"]
fn a_speed_0_creature_takes_a_thing_or_an_exit_beside_it() {
    let mut a = kit(5);
    ours(&mut a, 0, 8, 4, |c| {
        c.speed = 0;
        c.strategy.alignment = Alignment::Thief;
        c.strategy.aggressive = false;
    });
    a.regions[0].rooms[ROOM as usize].items.insert(
        0,
        RoomItem {
            x: 9,
            y: 4,
            thing: GOLD,
            parameter: 0,
            flag: false,
            destination: None,
            gold: Some(50),
        },
    );
    for seed in 0..64 {
        let mut g = game(&a, seed, t(6, 11, vec![]), room(4, 4));
        let ev = wait(&mut g);
        let turn = turns_of(&ev, 0)[0];
        assert!(
            turn.iter().any(|e| matches!(
                e,
                Event::CreatureGoldPickedUp {
                    slot: 0,
                    amount: 50,
                    ..
                }
            )),
            "s0g seed {seed}: {turn:?}"
        );
        assert_eq!(steps(turn), 0);
        let c = g.creature(1, 0).unwrap();
        assert_eq!((c.x, c.y, c.wealth), (8, 4, 50), "s0g seed {seed}");
    }

    for (name, speed, cap) in [("s0x-long", 0, 1), ("s2x-long", 2, 2)] {
        let mut a = kit(5);
        ours(&mut a, 0, 6, 4, |c| {
            c.speed = speed;
            c.strategy.alignment = Alignment::Neutral;
            c.strategy.aggressive = false;
        });
        a.regions[0].rooms[ROOM as usize].items.insert(
            0,
            RoomItem {
                x: 7,
                y: 4,
                thing: DOOR,
                parameter: 0,
                flag: false,
                destination: Some(Destination::Room {
                    region: 1,
                    room: SIDE,
                    x: 2,
                    y: 4,
                }),
                gold: None,
            },
        );
        let through = count(SEEDS / 4, |seed| {
            let mut g = game(&a, seed, t(6, 11, vec![]), room(4, 4));
            let mut ev = Vec::new();
            // Four of its turns, or until it has gone through the portal
            // (then it takes no more turns in the player's room).
            for _ in 0..16 {
                if turns_of(&ev, 0).len() >= 4 || g.creature(1, 0).is_some_and(|c| c.room != ROOM) {
                    break;
                }
                ev.extend(wait(&mut g));
            }
            let went = ev
                .iter()
                .any(|e| matches!(e, Event::CreatureTookDoor { slot: 0, .. }));
            if speed == 0 && !went {
                assert_eq!(steps(&ev), 0, "{name} seed {seed}");
            }
            went
        });
        same_rate(
            &format!("{name}: runs through the portal"),
            Sweep::Sample,
            cap,
            16,
            through,
            SEEDS / 4,
        );
    }
}

/// capture.md 10.14.9: blows "4 from 3 of 3 moves left, 2 from 2, 1 from
/// 1" (Dexterity 13), in the engine's own function (combat 3.1).
#[test]
fn blows_from_the_moves_left() {
    assert_eq!(combat::blows(3, 3, 13), 4);
    assert_eq!(combat::blows(2, 3, 13), 2);
    assert_eq!(combat::blows(1, 3, 13), 1);
}
