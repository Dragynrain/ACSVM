// SPDX-License-Identifier: GPL-3.0-or-later

//! Melee combat: an attack's blows, to-hit, parrying, counter-blows,
//! damage, armor, magic defense, learning, breaking, and creatures turning
//! hostile when a player hurts them. Spec: `docs/rules/combat.md` (section
//! numbers in the comments refer to it). Who attacks whom is decided in
//! `ai.rs` (creatures) and `step.rs` (players).

use crate::creatures::Other;
use crate::game::Game;
use crate::magic::{CAST_CHANCE_MAX, CAST_CHANCE_MIN};
use crate::{Event, Slot, Stat, Who};
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
        // Section 4: damage.
        let heavy = self.rng.below(100) < a.dexterity as u32;
        let mut dmg = damage(a.dexterity, a.size, ws.power, heavy);
        // 5.1, 5.2: armor; what it loses is told after the blow.
        let mut armor_lost = None;
        if let Some((armor, aw, armor_goes)) = d.armor {
            if dmg > 0 && !(ws.magic && !aw.magic) {
                if !ws.magic && aw.magic {
                    dmg = half(dmg + 1);
                }
                dmg -= aw.power as i32;
                let reached = dmg >= 0;
                dmg = dmg.max(0);
                if armor_goes {
                    armor_lost = Some((armor, true));
                } else if reached && self.rng.below(100) < aw.break_chance as u32 {
                    armor_lost = Some((armor, false));
                }
            }
        }
        // 5.4: magic defense.
        let mut stopped = false;
        if dmg > 0 {
            match (d.magic_defense, ws.magic) {
                (1, false) => dmg = half(dmg + 1),
                (2 | 3, false) => stopped = true,
                (3, true) => dmg = half(dmg + 1),
                _ => {}
            }
        }
        let damage = if stopped { 0 } else { dmg.clamp(0, 255) as u8 };
        let outcome = if stopped {
            BlowOutcome::Stopped
        } else {
            BlowOutcome::Hit { damage, heavy }
        };
        push(ev, chance, outcome);
        if let Some((armor, used_up)) = armor_lost {
            self.lose(target, armor, used_up, ev);
        }
        if stopped {
            return (outcome, true);
        }
        if damage == 0 {
            return (BlowOutcome::Hit { damage, heavy }, false);
        }
        // 4.2: what the damage takes.
        let (power_lost, life_lost) = losses(dmg, d.power, d.life);
        let power = (d.power as i32 - power_lost).max(0) as u8;
        let life = (d.life as i32 - life_lost).max(0) as u8;
        self.change_power(target, power, ev);
        self.change_life(target, life as i16 - d.life as i16, false, ev);
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
