// SPDX-License-Identifier: GPL-3.0-or-later

//! Combat: a melee attack's blows, to-hit, parrying, counter-blows,
//! damage, armor, magic defense, learning, breaking, creatures turning
//! hostile when a player hurts them, and firing missile weapons. Spec:
//! `docs/rules/combat.md` (section numbers in the comments refer to it).
//! Who attacks whom is decided in `ai.rs` (creatures, also when to fire)
//! and `step.rs` (players).

use crate::creatures::Other;
use crate::game::Game;
use crate::magic::{CAST_CHANCE_MAX, CAST_CHANCE_MIN};
use crate::{Event, Location, Slot, Stat, Who};
use acs_core::model::{Alignment, Creature, ThingId, ThingKind, WeaponStats};
use serde::{Deserialize, Serialize};

/// How one blow ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum BlowOutcome {
    Miss,
    Parried,
    /// Not parried. `damage` is what reached the defender after armor and
    /// magic defense (0 when they took it all).
    Hit {
        damage: u8,
        heavy: bool,
    },
    /// Magic defense stopped a non-magic weapon; the attack is called off
    /// (5.4).
    Stopped,
}

/// 1.1: held between 14 and 90.
pub fn clamp(v: i32) -> u8 {
    v.clamp(CAST_CHANCE_MIN as i32, CAST_CHANCE_MAX as i32) as u8
}

/// 1.3: halving rounds down, negative numbers too.
fn half(v: i32) -> i32 {
    v.div_euclid(2)
}

/// 1.2: 30 x (Constitution - Life Force) / Constitution, 0 when unhurt.
pub fn wound_penalty(constitution: u8, life: u8) -> i32 {
    if life >= constitution || constitution == 0 {
        0
    } else {
        30 * (constitution - life) as i32 / constitution as i32
    }
}

/// 1.5: the armor term for armor of attack adjustment `adj` (0 without).
pub fn armor_term(adj: Option<i8>, armor_skill: u8) -> i32 {
    match adj {
        None => 0,
        Some(a) => {
            let a = a as i32;
            (a + armor_skill as i32).min(a.max(0))
        }
    }
}

/// 3.1: the blows of an attack.
pub fn blows(left: u8, full: u8, dexterity: u8) -> i32 {
    let first = if full == 0 {
        1
    } else {
        4 * left as i32 / full as i32
    };
    (first + dexterity as i32 / 8 - 1).max(1)
}

/// 8.2: the shots of a firing: Dexterity / 8 (at least 1) plus half the
/// moves left.
pub fn shots(dexterity: u8, left: u8) -> i32 {
    (dexterity as i32 / 8).max(1) + left as i32 / 2
}

/// 8.4: the spread of a shot, 100 - accuracy, clamped; `after` is the
/// number of shots still to come after this one.
pub fn spread(missile: u8, adjustment: i8, armor_term: i32, power: u8, wound: i32, after: i32) -> u8 {
    let accuracy =
        missile as i32 + adjustment as i32 + armor_term + half(half(power as i32) - wound) - (5 * after - 20);
    clamp(100 - accuracy)
}

/// 8.6: the dodge chance of an individual a missile reaches.
pub fn dodge_chance(dodge: u8, armor_term: i32, power: u8, wound: i32, size: u8) -> u8 {
    clamp(dodge as i32 + 2 * armor_term + half(power as i32) - wound + 3 * (10 - size as i32))
}

/// Sixteenths of a square, the unit of a missile's flight (8.5): the
/// screen's pixels.
const SIXTEENTHS: i32 = 16;

/// Where on its square a point of a missile's path lies (8.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathPoint {
    /// Off the edge sixteenth on both axes: it reaches individuals there,
    /// and a wall or obstacle there stops it.
    Inner,
    /// On the edge on one axis: a wall or obstacle stops it, nobody is
    /// reached.
    Edge,
    /// On the edge on both axes: neither.
    Corner,
}

/// 8.5: the 64 points of a missile's path, as the square each lies on
/// (relative to the shooter) and where on it, for a target `(dx, dy)`
/// away (not both 0), a shot of `deviation` to the side the side roll
/// picked (`side` is a roll of 1), flying `range` squares along the longer
/// axis. Empty for a target on the shooter's own square.
pub fn flight(dx: i32, dy: i32, deviation: u32, side: bool, range: u8) -> Vec<(i32, i32, PathPoint)> {
    if dx == 0 && dy == 0 {
        return Vec::new();
    }
    let d = if side {
        -(deviation as i32)
    } else {
        deviation as i32
    };
    // Turned by the angle whose tangent is d / 100, each part truncated.
    let (ax, ay) = (SIXTEENTHS * dx, SIXTEENTHS * dy);
    let (nx, ny) = (ax + ay * d / 100, ay - ax * d / 100);
    // Scaled to the range along the longer axis, truncated.
    let m = nx.abs().max(ny.abs());
    if m == 0 {
        return Vec::new();
    }
    let len = range as i32 * SIXTEENTHS;
    let start = (SIXTEENTHS / 2, SIXTEENTHS / 2);
    let mut p = [(0i32, 0i32); 65];
    p[0] = start;
    p[64] = (start.0 + nx * len / m, start.1 + ny * len / m);
    // The points between by halving, each the average rounded down.
    fn fill(p: &mut [(i32, i32); 65], i: usize, j: usize) {
        if j - i < 2 {
            return;
        }
        let k = (i + j) / 2;
        p[k] = ((p[i].0 + p[j].0).div_euclid(2), (p[i].1 + p[j].1).div_euclid(2));
        fill(p, i, k);
        fill(p, k, j);
    }
    fill(&mut p, 0, 64);
    let edge = |v: i32| matches!(v.rem_euclid(SIXTEENTHS), 0 | 15);
    p[1..]
        .iter()
        .map(|&(x, y)| {
            let kind = match (edge(x), edge(y)) {
                (false, false) => PathPoint::Inner,
                (true, true) => PathPoint::Corner,
                _ => PathPoint::Edge,
            };
            (x.div_euclid(SIXTEENTHS), y.div_euclid(SIXTEENTHS), kind)
        })
        .collect()
}

/// 8.1: the squares (relative to the shooter) of the 19 points checked
/// for a clear line of fire to a target `(dx, dy)` away: 1/20 to 19/20 of
/// the way between the squares' middles, in twentieths of a square.
pub fn line_of_fire(dx: i32, dy: i32) -> Vec<(i32, i32)> {
    (1..20)
        .map(|j| ((10 + j * dx).div_euclid(20), (10 + j * dy).div_euclid(20)))
        .collect()
}

/// 4.1: a blow's damage before armor and magic defense, from Build and
/// the weapon's Power, plus half of it on a heavy blow; at most 1.5 x.
pub fn damage(dexterity: u8, size: u8, power: u8, heavy: bool) -> i32 {
    let p = power as i32;
    let build = (dexterity as i32 + size as i32 - 6).clamp(0, 30) - 15;
    let mut d = (build * p).div_euclid(30) + p;
    if heavy {
        d += half(p);
    }
    d.min(3 * p / 2).max(0)
}

/// 4.2: (Power lost, Life Force lost) for damage `d`.
pub fn losses(d: i32, power: u8, life: u8) -> (i32, i32) {
    let (pw, lf) = (power as i32, life as i32);
    let share = if pw + lf == 0 {
        0
    } else {
        (d * pw / (pw + lf)).min((pw + 1) / 2)
    };
    (2 * share, d - share)
}

/// 2.2: the cost of a square holding `o` for fighter `c`, `None` closed.
/// Only for a fighter owning a melee weapon (2.3); the caller checks that.
pub fn strike_cost(c: &Creature, o: &Other) -> Option<u16> {
    // Columns: P thief, P neutral, P enemy, P friend, A thief, A neutral,
    // A enemy, A friend, player.
    let col = match o.alignment {
        None => 8,
        Some(a) => {
            let base = match a {
                Alignment::Thief => 0,
                Alignment::Neutral => 1,
                Alignment::Enemy => 2,
                Alignment::Friend => 3,
            };
            base + if o.aggressive { 4 } else { 0 }
        }
    };
    const N: u16 = 0;
    let row: [u16; 9] = match c.strategy.alignment {
        Alignment::Thief => [1, 3, 3, 3, 1, 1, 1, 1, 1],
        Alignment::Neutral => [1, 3, 3, 3, 3, 3, 1, 1, 1],
        Alignment::Enemy => [3, 3, N, 1, 3, 3, N, 1, 1],
        Alignment::Friend => [3, 3, 1, N, 1, 1, 1, N, N],
    };
    Some(row[col]).filter(|&c| c != N)
}

/// What a blow needs to know about one side.
#[derive(Debug, Clone, Copy)]
struct Side {
    weapon: Option<(ThingId, WeaponStats, bool)>,
    armor: Option<(ThingId, WeaponStats, bool)>,
    constitution: u8,
    life: u8,
    power: u8,
    dexterity: u8,
    size: u8,
    wisdom: u8,
    parry: u8,
    armor_skill: u8,
    melee: u8,
    missile: u8,
    dodge: u8,
    magic_defense: u8,
}

impl Side {
    fn wound(&self) -> i32 {
        wound_penalty(self.constitution, self.life)
    }

    fn armor_term(&self) -> i32 {
        armor_term(self.armor.map(|a| a.1.attack_adjustment), self.armor_skill)
    }
}

impl Game {
    /// The melee weapon `who` holds: its number, stats and whether it
    /// disappears after use.
    pub(crate) fn held_melee(&self, who: Who) -> Option<(ThingId, WeaponStats, bool)> {
        let t = self.held(who, Slot::Weapon)?;
        match self.thing(t)?.kind {
            ThingKind::MeleeWeapon {
                stats,
                disappears_after_use,
                ..
            } => Some((t, stats, disappears_after_use)),
            _ => None,
        }
    }

    /// The missile weapon `who` holds: its number, stats and whether it
    /// disappears after use.
    pub(crate) fn held_missile(&self, who: Who) -> Option<(ThingId, WeaponStats, bool)> {
        let t = self.held(who, Slot::Weapon)?;
        match self.thing(t)?.kind {
            ThingKind::MissileWeapon {
                stats,
                disappears_after_use,
                ..
            } => Some((t, stats, disappears_after_use)),
            _ => None,
        }
    }

    fn worn_armor(&self, who: Who) -> Option<(ThingId, WeaponStats, bool)> {
        let t = self.held(who, Slot::Armor)?;
        match self.thing(t)?.kind {
            ThingKind::Armor {
                stats,
                disappears_after_use,
                ..
            } => Some((t, stats, disappears_after_use)),
            _ => None,
        }
    }

    /// The weapon or armor readied in `slot`.
    pub(crate) fn held(&self, who: Who, slot: Slot) -> Option<ThingId> {
        let t = match who {
            Who::Player { player } => {
                let p = self.players.get(player)?;
                match slot {
                    Slot::Weapon => p.weapon,
                    Slot::Armor => p.armor,
                }
            }
            Who::Creature { region, slot: s } => {
                let c = self.creature(region, s)?;
                Some(match slot {
                    Slot::Weapon => c.weapon,
                    Slot::Armor => c.armor,
                })
            }
        }?;
        (t != 0).then_some(t)
    }

    /// The lowest-numbered melee weapon `who` owns.
    pub(crate) fn owned_melee(&self, who: Who) -> Option<ThingId> {
        let mut owned: Vec<ThingId> = match who {
            Who::Player { player } => self.players.get(player)?.possessions.clone(),
            Who::Creature { region, slot } => self
                .creature(region, slot)?
                .possessions
                .iter()
                .filter(|p| p.count > 0)
                .map(|p| p.thing)
                .collect(),
        };
        if let Some(t) = self.held(who, Slot::Weapon) {
            owned.push(t);
        }
        owned.sort_unstable();
        owned.into_iter().find(|&t| {
            matches!(
                self.thing(t).map(|t| &t.kind),
                Some(ThingKind::MeleeWeapon { .. })
            )
        })
    }

    /// Readies `thing` in the weapon slot. True if the weapon in hand
    /// changed (which costs a move, 6.1).
    pub(crate) fn set_weapon(&mut self, who: Who, thing: ThingId) -> bool {
        if self.held(who, Slot::Weapon) == Some(thing) {
            return false;
        }
        match who {
            Who::Player { player } => self.players[player].weapon = Some(thing),
            Who::Creature { region, slot } => {
                if let Some(c) = self.creature_mut(region, slot) {
                    c.weapon = thing;
                }
            }
        }
        true
    }

    fn side(&self, who: Who) -> Option<Side> {
        let s = |stat| self.stat(who, stat);
        Some(Side {
            weapon: self.held_melee(who),
            armor: self.worn_armor(who),
            constitution: s(Stat::Constitution)?,
            life: s(Stat::LifeForce)?,
            power: s(Stat::Power)?,
            dexterity: s(Stat::Dexterity)?,
            size: match who {
                Who::Player { player } => self.players.get(player)?.size,
                Who::Creature { region, slot } => self.creature(region, slot)?.size,
            },
            wisdom: s(Stat::Wisdom)?,
            parry: s(Stat::Parry)?,
            armor_skill: s(Stat::ArmorSkill)?,
            melee: s(Stat::Melee)?,
            missile: s(Stat::Missile)?,
            dodge: s(Stat::Dodge)?,
            magic_defense: s(Stat::MagicDefense)?,
        })
    }

    /// A melee attack (section 3) by `by` on `target`, with `left` of
    /// `full` moves left. A player's blows can turn the defender hostile
    /// (7). The caller ends the attacker's turn (2.5).
    pub(crate) fn attack(&mut self, by: Who, target: Who, left: u8, full: u8, ev: &mut Vec<Event>) {
        let Some(me) = self.side(by) else {
            return;
        };
        let Some((weapon, ..)) = me.weapon else {
            return;
        };
        let mut blows = blows(left, full, me.dexterity);
        ev.push(Event::Attacked {
            by,
            target,
            weapon,
            blows: blows as u8,
        });
        let player_blow = matches!(by, Who::Player { .. });
        while blows > 0 && self.alive(by) && self.alive(target) {
            let (outcome, stop) = self.blow(by, target, blows, false, player_blow, ev);
            match outcome {
                BlowOutcome::Hit { damage, .. } if damage > 0 => blows -= 2,
                BlowOutcome::Miss if self.alive(target) && self.held_melee(target).is_some() && !stop => {
                    // 3.3.6: the counter-blow.
                    let (counter, stop2) = self.blow(target, by, blows, true, false, ev);
                    blows -= 1;
                    if matches!(counter, BlowOutcome::Hit { damage, .. } if damage > 0) {
                        blows -= 1;
                    }
                    if stop2 {
                        break;
                    }
                }
                _ => blows -= 1,
            }
            if stop || self.held_melee(by).is_none() {
                break;
            }
        }
    }

    /// One blow (3.3) with `blows` left (counting this one). Returns how it
    /// ended and whether the attack stops (weapon broken, magic defense).
    fn blow(
        &mut self,
        by: Who,
        target: Who,
        blows: i32,
        counter: bool,
        player_blow: bool,
        ev: &mut Vec<Event>,
    ) -> (BlowOutcome, bool) {
        let (Some(a), Some(d)) = (self.side(by), self.side(target)) else {
            return (BlowOutcome::Miss, true);
        };
        let Some((weapon, ws, weapon_goes)) = a.weapon else {
            return (BlowOutcome::Miss, true);
        };
        // 3.4: the hit chance.
        let chance = clamp(
            ws.attack_adjustment as i32 + a.armor_term() + a.melee as i32 + 5 * blows - 20
                + a.power as i32 / 4
                - a.wound(),
        );
        let push = |ev: &mut Vec<Event>, chance: u8, outcome| {
            ev.push(Event::Blow {
                by,
                target,
                counter,
                chance,
                outcome,
            })
        };
        if self.rng.below(100) >= chance as u32 {
            push(ev, chance, BlowOutcome::Miss);
            return (BlowOutcome::Miss, false);
        }
        // 5.5: the attacker learns.
        self.learn(by, Stat::Melee, a.melee, ev);
        if a.armor_term() < 0 {
            self.learn_armor(by, a.armor_skill, ev);
        }
        // 3.5: the parry roll, always drawn.
        let parry = match d.weapon {
            None => 0,
            Some((_, dw, _)) => clamp(
                d.parry as i32
                    + half(dw.attack_adjustment as i32)
                    + half(-d.wound())
                    + d.armor_term()
                    + half(d.power as i32)
                    + 15
                    - d.size as i32,
            ),
        };
        if self.rng.below(100) < parry as u32 {
            push(ev, chance, BlowOutcome::Parried);
            self.learn(target, Stat::Parry, d.parry, ev);
            if self.rng.below(100) < 30 && d.armor_term() < 0 {
                self.learn_armor(target, d.armor_skill, ev);
            }
            // 6.3: the striking weapon's break roll.
            if self.rng.below(100) < ws.break_chance as u32 {
                self.lose(by, weapon, false, ev);
                return (BlowOutcome::Parried, !counter);
            }
            return (BlowOutcome::Parried, false);
        }
        // Section 4: damage; what the armor loses is told after the blow.
        let (outcome, armor_lost) = self.harm(&a, &d, ws);
        push(ev, chance, outcome);
        if let Some((armor, used_up)) = armor_lost {
            self.lose(target, armor, used_up, ev);
        }
        let BlowOutcome::Hit { damage, heavy } = outcome else {
            return (outcome, true); // 5.4: stopped
        };
        if damage == 0 {
            return (outcome, false);
        }
        self.take_damage(target, &d, damage, ev);
        // 6.4: a melee weapon that disappears goes after its first
        // damaging blow.
        if weapon_goes {
            self.lose(by, weapon, true, ev);
        }
        // 7: a player's damaging blow.
        if player_blow && self.alive(target) {
            self.turn_hostile(target, d.wisdom, ev);
        }
        (BlowOutcome::Hit { damage, heavy }, weapon_goes && !counter)
    }

    /// Section 4 after the to-hit: the heavy-blow roll on the attacker's
    /// Dexterity, armor (5.1, 5.2, with its break roll) and magic defense
    /// (5.4). Returns the outcome (`Hit` or `Stopped`) and the armor the
    /// defender loses, if any, with whether it was used up.
    fn harm(&mut self, a: &Side, d: &Side, ws: WeaponStats) -> (BlowOutcome, Option<(ThingId, bool)>) {
        let heavy = self.rng.below(100) < a.dexterity as u32;
        let mut dmg = damage(a.dexterity, a.size, ws.power, heavy);
        let mut armor_lost = None;
        if let Some((armor, aw, armor_goes)) = d.armor {
            if dmg > 0 && !(ws.magic && !aw.magic) {
                if !ws.magic && aw.magic {
                    dmg = half(dmg + 1);
                }
                dmg -= aw.power as i32;
                let reached = dmg >= 0;
                dmg = dmg.max(0);
                // The break roll whenever the blow reaches the armor, also
                // for armor that then disappears.
                if reached && self.rng.below(100) < aw.break_chance as u32 {
                    armor_lost = Some((armor, false));
                } else if armor_goes {
                    armor_lost = Some((armor, true));
                }
            }
        }
        if dmg > 0 {
            match (d.magic_defense, ws.magic) {
                (1, false) => dmg = half(dmg + 1),
                (2 | 3, false) => return (BlowOutcome::Stopped, armor_lost),
                (3, true) => dmg = half(dmg + 1),
                _ => {}
            }
        }
        let damage = dmg.clamp(0, 255) as u8;
        (BlowOutcome::Hit { damage, heavy }, armor_lost)
    }

    /// 4.2: what damage takes from the defender.
    fn take_damage(&mut self, target: Who, d: &Side, damage: u8, ev: &mut Vec<Event>) {
        let (power_lost, life_lost) = losses(damage as i32, d.power, d.life);
        let power = (d.power as i32 - power_lost).max(0) as u8;
        let life = (d.life as i32 - life_lost).max(0) as u8;
        self.change_power(target, power, ev);
        self.change_life(target, life as i16 - d.life as i16, false, ev);
    }

    /// Firing (section 8) by `by` at `target` with `left` moves left: the
    /// shots (8.2), each with its break roll, deviation and side (8.3),
    /// then its flight (8.5, 8.6); a weapon that disappears goes after the
    /// first (8.7). The caller ends the shooter's turn. **Engine:** each
    /// shot aims at where the target stands then; the firing stops when the
    /// target is dead or out of the shooter's room.
    pub(crate) fn fire(&mut self, by: Who, target: Who, left: u8, ev: &mut Vec<Event>) {
        let Some((weapon, ws, goes)) = self.held_missile(by) else {
            return;
        };
        let Some(me) = self.side(by) else {
            return;
        };
        let total = shots(me.dexterity, left);
        ev.push(Event::Fired {
            by,
            target,
            weapon,
            shots: total as u8,
        });
        for shot in 0..total {
            if self.held_missile(by).map(|w| w.0) != Some(weapon) || !self.alive(target) {
                break;
            }
            let (Some(a), Some(from), Some(to)) = (self.side(by), self.room_xy(by), self.room_xy(target))
            else {
                break;
            };
            if (from.0, from.1) != (to.0, to.1) {
                break;
            }
            // 8.3: the break roll, drawn even at break chance 0.
            if self.rng.below(100) < ws.break_chance as u32 {
                self.lose(by, weapon, false, ev);
                break;
            }
            let spread = spread(
                a.missile,
                ws.attack_adjustment,
                a.armor_term(),
                a.power,
                a.wound(),
                total - 1 - shot,
            );
            let deviation = self.rng.below(spread as u32);
            let side = self.rng.below(2) == 1;
            ev.push(Event::Shot {
                by,
                spread,
                deviation: deviation as u8,
                side,
            });
            let path = flight(
                to.2 as i32 - from.2 as i32,
                to.3 as i32 - from.3 as i32,
                deviation,
                side,
                ws.range.unwrap_or(0),
            );
            self.missile_flies(by, from, &path, ws, ev);
            if goes {
                self.lose(by, weapon, true, ev);
                break;
            }
        }
    }

    /// 8.5, 8.6: one missile along `path` (squares relative to the shooter
    /// at `from`: region, room, x, y): stopped by a wall or obstacle, it
    /// reaches every individual on its way in order until one is hit.
    fn missile_flies(
        &mut self,
        by: Who,
        from: (u8, u8, u8, u8),
        path: &[(i32, i32, PathPoint)],
        ws: WeaponStats,
        ev: &mut Vec<Event>,
    ) {
        let (region, room, x0, y0) = from;
        let player_shot = matches!(by, Who::Player { .. });
        let Some((w, h)) = self.room(region, room).map(|r| (r.width as i32, r.height as i32)) else {
            return;
        };
        let mut reached = vec![(0, 0)];
        for &(dx, dy, kind) in path {
            let (x, y) = (x0 as i32 + dx, y0 as i32 + dy);
            // It ends where it leaves the room.
            if !(0..w).contains(&x) || !(0..h).contains(&y) {
                return;
            }
            let (x, y) = (x as u8, y as u8);
            if !self.missile_passes(region, room, x, y) {
                if kind == PathPoint::Corner {
                    continue;
                }
                ev.push(Event::MissileStopped { by, x, y });
                return;
            }
            // Each square counts once, on an inner point, never the
            // shooter's own.
            if kind != PathPoint::Inner || reached.contains(&(dx, dy)) {
                continue;
            }
            reached.push((dx, dy));
            let here: Vec<Who> = self
                .others_in_room(region, room, Some(by))
                .into_iter()
                .filter(|o| (o.x, o.y) == (x, y))
                .map(|o| o.who)
                .collect();
            for target in here {
                let (Some(a), Some(d)) = (self.side(by), self.side(target)) else {
                    continue;
                };
                // 8.6: the shooter's Missile check, then the dodge.
                self.learn(by, Stat::Missile, a.missile, ev);
                let chance = dodge_chance(d.dodge, d.armor_term(), d.power, d.wound(), d.size);
                if self.rng.below(100) < chance as u32 {
                    ev.push(Event::Dodged {
                        by,
                        who: target,
                        chance,
                    });
                    self.learn(target, Stat::Dodge, d.dodge, ev);
                    if self.rng.below(100) < 30 && d.armor_term() < 0 {
                        self.learn_armor(target, d.armor_skill, ev);
                    }
                    continue; // it flies on
                }
                let (outcome, armor_lost) = self.harm(&a, &d, ws);
                ev.push(Event::MissileHit {
                    by,
                    target,
                    dodge: chance,
                    outcome,
                });
                if let Some((armor, used_up)) = armor_lost {
                    self.lose(target, armor, used_up, ev);
                }
                if let BlowOutcome::Hit { damage, .. } = outcome {
                    if damage > 0 {
                        self.take_damage(target, &d, damage, ev);
                        if player_shot && self.alive(target) {
                            self.turn_hostile(target, d.wisdom, ev);
                        }
                    }
                }
                return; // it stops, also when the damage came to 0
            }
        }
    }

    /// 8.1, 8.5: whether a missile flies through a square: inside the
    /// room's walls (its edge squares), and no obstacle on top of it.
    pub(crate) fn missile_passes(&self, region: u8, room: u8, x: u8, y: u8) -> bool {
        let Some(r) = self.room(region, room) else {
            return false;
        };
        if x == 0 || y == 0 || x >= r.width.saturating_sub(1) || y >= r.height.saturating_sub(1) {
            return false;
        }
        match r.items.iter().find(|it| it.x == x && it.y == y) {
            None => true,
            Some(it) => !matches!(
                self.thing(it.thing).map(|t| &t.kind),
                Some(ThingKind::Obstacle { .. } | ThingKind::CustomObstacle { .. })
            ),
        }
    }

    /// Region, room and square of an individual standing in a room.
    pub(crate) fn room_xy(&self, who: Who) -> Option<(u8, u8, u8, u8)> {
        match who {
            Who::Player { player } => match self.players.get(player)?.at {
                Location::Room { region, room, x, y } => Some((region, room, x, y)),
                _ => None,
            },
            Who::Creature { region, slot } => {
                let c = self.creature(region, slot)?;
                Some((region, c.room, c.x, c.y))
            }
        }
    }

    fn change_power(&mut self, who: Who, to: u8, ev: &mut Vec<Event>) {
        let from = self.stat(who, Stat::Power).unwrap_or(0);
        if from != to {
            self.set_stat(who, Stat::Power, to);
            ev.push(Event::StatChanged {
                who,
                stat: Stat::Power,
                from,
                to,
            });
        }
    }

    /// 5.5: a roll of 0..99 above the skill raises it by 1 (at most 127).
    fn learn(&mut self, who: Who, stat: Stat, skill: u8, ev: &mut Vec<Event>) {
        if self.rng.below(100) > skill as u32 && skill < Stat::Melee.max() {
            self.gain(who, stat, skill, ev);
        }
    }

    /// 5.5: Armor skill rises on a roll below 65 minus it, clamped.
    fn learn_armor(&mut self, who: Who, skill: u8, ev: &mut Vec<Event>) {
        let chance = clamp(65 - skill as i32);
        if self.rng.below(100) < chance as u32 && skill < Stat::ArmorSkill.max() {
            self.gain(who, Stat::ArmorSkill, skill, ev);
        }
    }

    fn gain(&mut self, who: Who, stat: Stat, from: u8, ev: &mut Vec<Event>) {
        self.set_stat(who, stat, from + 1);
        ev.push(Event::StatChanged {
            who,
            stat,
            from,
            to: from + 1,
        });
    }

    /// 6.3, 6.4: a weapon or armor broken or used up leaves its owner.
    fn lose(&mut self, who: Who, thing: ThingId, used_up: bool, ev: &mut Vec<Event>) {
        if self.take_from(who, thing) {
            // The last copy leaving clears the slot; a further copy stays
            // owned but is no longer in hand.
            for slot in [Slot::Weapon, Slot::Armor] {
                if self.held(who, slot) == Some(thing) {
                    match who {
                        Who::Player { player } => match slot {
                            Slot::Weapon => self.players[player].weapon = None,
                            Slot::Armor => self.players[player].armor = None,
                        },
                        Who::Creature { region, slot: s } => {
                            if let Some(c) = self.creature_mut(region, s) {
                                match slot {
                                    Slot::Weapon => c.weapon = 0,
                                    Slot::Armor => c.armor = 0,
                                }
                            }
                        }
                    }
                }
            }
            ev.push(Event::Broke { who, thing, used_up });
        }
    }

    /// Section 7: a creature a player hurt changes its class.
    fn turn_hostile(&mut self, target: Who, wisdom: u8, ev: &mut Vec<Event>) {
        let Who::Creature { region, slot } = target else {
            return;
        };
        let Some(c) = self.creature(region, slot) else {
            return;
        };
        let mut s = c.strategy;
        match (s.alignment, s.aggressive) {
            (Alignment::Friend, _) => s.alignment = Alignment::Neutral,
            (_, false) => s.aggressive = true,
            (Alignment::Neutral, true) => {
                let smart = match wisdom {
                    w if w > 14 => true,
                    w if w < 6 => false,
                    w => self.rng.below(10) < (w - 5) as u32,
                };
                if !smart {
                    return;
                }
                s.alignment = Alignment::Enemy;
            }
            _ => return,
        }
        if let Some(c) = self.creature_mut(region, slot) {
            c.strategy = s;
        }
        ev.push(Event::CreatureTurned {
            region,
            slot,
            alignment: s.alignment,
            aggressive: s.aggressive,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_captured_numbers() {
        // capture.md 10.14.9: Melee 31, Power 13, unhurt, no armor.
        let c = |b: i32| clamp(31 + 5 * b - 20 + 13 / 4);
        assert_eq!([c(4), c(3), c(2), c(1)], [34, 29, 24, 19]);
        assert_eq!(blows(3, 3, 13), 4);
        assert_eq!(blows(2, 3, 13), 2);
        assert_eq!(blows(1, 3, 13), 1);
        assert_eq!(blows(0, 3, 13), 1);
        assert_eq!(blows(0, 0, 13), 1);
        // The blade (Power 10), Dexterity 13, Size 9: 10, heavy 15.
        assert_eq!(damage(13, 9, 10, false), 10);
        assert_eq!(damage(13, 9, 10, true), 15);
        // At Life Force 40, Power 13: 8 Life Force and 4 Power.
        assert_eq!(losses(10, 13, 40), (4, 8));
        assert_eq!(losses(15, 13, 40), (6, 12));
    }

    #[test]
    fn the_captured_firing_numbers() {
        // 8.2: Dexterity 13, 3 of 3 moves: 2 shots.
        assert_eq!(shots(13, 3), 2);
        assert_eq!(shots(5, 0), 1);
        assert_eq!(shots(31, 6), 6);
        // 8.4: Missile 22, Power 13, unhurt: 60 then 55; 54 as Missile grew.
        assert_eq!(spread(22, 0, 0, 13, 0, 1), 60);
        assert_eq!(spread(22, 0, 0, 13, 0, 0), 55);
        assert_eq!(spread(23, 0, 0, 13, 0, 0), 54);
        assert_eq!(spread(127, 40, 0, 63, 0, 0), 14);
        // 8.6: Dodge 30, Power 13, Size 9, unhurt: 39.
        assert_eq!(dodge_chance(30, 0, 13, 0, 9), 39);
    }

    /// The deviations (minus for a side roll of 1) at which a shot aimed
    /// `(dx, dy)` away reaches the square `at`, walls aside.
    fn reach_range(dx: i32, dy: i32, at: (i32, i32)) -> (i32, i32) {
        let ok: Vec<i32> = (0..100)
            .flat_map(|d| [(d, false), (d, true)])
            .filter(|&(d, side)| {
                flight(dx, dy, d as u32, side, 7)
                    .iter()
                    .any(|&(x, y, k)| (x, y) == at && k == PathPoint::Inner)
            })
            .map(|(d, side)| if side { -d } else { d })
            .collect();
        (*ok.iter().min().unwrap(), *ok.iter().max().unwrap())
    }

    #[test]
    fn the_flight() {
        // 8.5, the researcher's table (pass 3), checked on every shot of
        // our pass 2 and 3 runs: 5 squares west; a friend 3 squares out on
        // that shot; diagonal (9,7 to 4,4); the diagonal neighbour.
        assert_eq!(reach_range(-5, 0, (-5, 0)), (-9, 11));
        assert_eq!(reach_range(-5, 0, (-3, 0)), (-17, 18));
        assert_eq!(reach_range(-5, -3, (-5, -3)), (-10, 9));
        assert_eq!(reach_range(-1, 1, (-1, 1)), (-43, 49));
        // A plus deviation turns the aim anticlockwise: west strays south.
        let last = flight(-5, 0, 40, false, 7).last().copied().unwrap();
        assert_eq!((last.0, last.1), (-7, 3));
        // 64 points, out to the range along the longer axis.
        let p = flight(0, 3, 0, false, 4);
        assert_eq!(p.len(), 64);
        assert_eq!((p[63].0, p[63].1), (0, 4));
        assert!(flight(0, 0, 10, true, 5).is_empty());
        // The square's edge sixteenth: inner, edge and corner points.
        assert!(p.iter().all(|&(x, _, k)| x == 0 && k != PathPoint::Corner));
        assert!(p.iter().any(|&(.., k)| k == PathPoint::Edge));
        assert!(flight(4, 4, 0, false, 4)
            .iter()
            .any(|&(.., k)| k == PathPoint::Corner));
    }

    #[test]
    fn the_line_of_fire() {
        // 8.1: from 9,4 to 4,4 the line runs through 5,4 (the obstacle of
        // `bow-wall`); from 5,5 it doesn't.
        assert!(line_of_fire(-5, 0).contains(&(-4, 0)));
        assert!(!line_of_fire(-1, -1).contains(&(0, -1)));
        assert_eq!(line_of_fire(-1, -1).len(), 19);
        assert_eq!(*line_of_fire(3, 0).last().unwrap(), (3, 0));
    }

    #[test]
    fn the_strike_table() {
        let mut c = blank();
        let o = |alignment, aggressive| Other {
            who: Who::Player { player: 0 },
            alignment,
            aggressive,
            x: 0,
            y: 0,
        };
        c.strategy.alignment = Alignment::Enemy;
        assert_eq!(strike_cost(&c, &o(None, false)), Some(1));
        assert_eq!(strike_cost(&c, &o(Some(Alignment::Enemy), true)), None);
        assert_eq!(strike_cost(&c, &o(Some(Alignment::Friend), false)), Some(1));
        assert_eq!(strike_cost(&c, &o(Some(Alignment::Neutral), true)), Some(3));
        c.strategy.alignment = Alignment::Friend;
        assert_eq!(strike_cost(&c, &o(None, false)), None);
        assert_eq!(strike_cost(&c, &o(Some(Alignment::Friend), true)), None);
        assert_eq!(strike_cost(&c, &o(Some(Alignment::Thief), true)), Some(1));
        assert_eq!(strike_cost(&c, &o(Some(Alignment::Thief), false)), Some(3));
        c.strategy.alignment = Alignment::Thief;
        assert_eq!(strike_cost(&c, &o(Some(Alignment::Thief), false)), Some(1));
        assert_eq!(strike_cost(&c, &o(Some(Alignment::Enemy), false)), Some(3));
    }

    fn blank() -> Creature {
        use acs_core::model::{MagicDefense, Strategy};
        Creature {
            number: 1,
            in_use: true,
            name: String::new(),
            class: None,
            picture: None,
            constitution: 10,
            life_force: 10,
            strength: 10,
            dexterity: 10,
            speed: 1,
            wisdom: 10,
            power: 10,
            size: 10,
            dodge: 0,
            parry: 0,
            armor_skill: 0,
            melee: 0,
            missile: 0,
            strategy: Strategy {
                alignment: Alignment::Enemy,
                aggressive: true,
                brave: true,
            },
            magic_defense: MagicDefense::None,
            mimic: false,
            wealth: 0,
            weapon: 0,
            armor: 0,
            x: 0,
            y: 0,
            region: 1,
            room: 0,
            possessions: Vec::new(),
            unknown_0b: 0,
            unknown_0c: 0,
        }
    }

    #[test]
    fn edges() {
        assert_eq!(clamp(-50), 14);
        assert_eq!(clamp(200), 90);
        assert_eq!(half(-5), -3);
        assert_eq!(wound_penalty(20, 20), 0);
        assert_eq!(wound_penalty(20, 30), 0);
        assert_eq!(wound_penalty(20, 0), 30);
        assert_eq!(wound_penalty(20, 13), 10);
        assert_eq!(wound_penalty(0, 0), 0);
        // 1.5: skill makes up a negative adjustment, never above 0; a
        // positive one never grows.
        assert_eq!(armor_term(None, 50), 0);
        assert_eq!(armor_term(Some(-30), 10), -20);
        assert_eq!(armor_term(Some(-30), 50), 0);
        assert_eq!(armor_term(Some(10), 50), 10);
        // 4.1: Build -15 halves, the cap is 1.5 x P.
        assert_eq!(damage(0, 0, 10, false), 5);
        assert_eq!(damage(31, 31, 10, true), 15);
        assert_eq!(damage(13, 9, 0, true), 0);
        // 4.2: the share is at most (Power + 1) / 2; nothing to share.
        assert_eq!(losses(30, 40, 10), (2 * 20, 10));
        assert_eq!(losses(5, 0, 0), (0, 5));
        // 3.1: Dexterity counts.
        assert_eq!(blows(3, 3, 31), 6);
    }
}
