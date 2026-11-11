// SPDX-License-Identifier: GPL-3.0-or-later

//! Magic: what the 15 spell actions do, casting and using, Power drift,
//! temporary Life Force, natural healing and death. Spec:
//! `docs/rules/magic.md` (section numbers in the comments refer to it).
//! Creatures choosing to cast are in `ai.rs`.

use crate::creatures;
use crate::game::Game;
use crate::inventory::{self, GOLD, PILE_LIMIT};
use crate::{CastRefusal, Event, Location, RemovalReason, SpellSource, Stat, Who};
use acs_core::model::ThingId;
use acs_core::model::{
    Creature, CustomTrigger, MagicDefense, Possession, SpellAction, SpellActionKind, ThingKind,
};
use serde::{Deserialize, Serialize};

/// Per-individual state the record has no room for (5.2-5.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Vitals {
    pub who: Who,
    /// The temporary part of Life Force, positive or negative.
    pub temp: i16,
    /// Turns of its own since the game began (or since it arrived).
    pub turns: u32,
}

/// A summons waiting for the next player turn (3.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PendingSummon {
    pub region: u8,
    pub room: u8,
    pub x: u8,
    pub y: u8,
    pub creature: u16,
}

/// One spell going off.
#[derive(Debug, Clone)]
pub(crate) struct Spell {
    /// Who set it off.
    pub by: Who,
    /// The individual it acts on (2.1-2.4).
    pub target: Who,
    pub source: SpellSource,
    /// The master thing that holds it.
    pub thing: ThingId,
    pub action: SpellAction,
    pub message: Option<String>,
    /// Where it takes effect.
    pub at: Location,
    /// The holding thing's index in the room's item list, when it lies in a
    /// stack at `at` (for Activate All, 3.14).
    pub stack: Option<usize>,
}

/// Whether an action acts on one individual (2.5).
pub fn acts_on_individual(kind: SpellActionKind) -> bool {
    use SpellActionKind::*;
    matches!(
        kind,
        IncreaseMagicDefense
            | DecreaseMagicDefense
            | IncreaseStat
            | DecreaseStat
            | ChangePower
            | ChangeLifeForce
            | Give
    )
}

/// Power a magic item's use costs (4.2).
pub const USE_POWER: u8 = 2;
/// The temporary part of Life Force ticks 1 toward 0 with chance 1 in
/// this, each turn (5.2, captured).
pub const TEMP_TICK: u32 = 3;
/// Natural healing adds 1 more with chance 1 in this (5.3, captured).
pub const HEAL_EXTRA: u32 = 9;

/// The signed amount of a Change Power / Change Life Force parameter and
/// whether it is temporary (3.7, 3.8).
pub fn amount(parameter: u16) -> (i16, bool) {
    // Classic packs the amount into one byte.
    let parameter = parameter as u8;
    let n = (parameter & 0x3F) as i16;
    let n = if parameter & 0x80 != 0 { -n } else { n };
    (n, parameter & 0x40 != 0)
}

/// How a creature sees a spell (6.4): `Some(true)` hurtful, `Some(false)`
/// helpful, `None` never cast by creatures.
pub fn hurtful(action: SpellAction) -> Option<bool> {
    use SpellActionKind::*;
    match action.kind {
        KillAllButOwnerOf | SummonOrBanish | DecreaseMagicDefense | DecreaseStat => Some(true),
        IncreaseMagicDefense | IncreaseStat | Give => Some(false),
        ChangePower | ChangeLifeForce => Some(amount(action.parameter).0 < 0),
        _ => None,
    }
}

/// The chance in percent that a spell of `cost` works for `power`: the
/// share of rolls 0..=cost below Power, rounded down (4.1).
pub fn cast_chance(power: u8, cost: u8) -> u8 {
    if cost == 0 {
        return 100;
    }
    ((power as u32 * 100) / (cost as u32 + 1)).min(100) as u8
}

/// Rolls a cast of `cost` for `power` (4.1): cost 0 always works,
/// otherwise a roll of 0..=cost works when it is below Power.
pub fn cast_roll(rng: &mut crate::Rng, power: u8, cost: u8) -> bool {
    cost == 0 || rng.below(cost as u32 + 1) < power as u32
}

fn level(m: MagicDefense) -> u8 {
    match m {
        MagicDefense::None => 0,
        MagicDefense::NonMagicHalfDamage => 1,
        MagicDefense::OnlyMagicDamages => 2,
        MagicDefense::MagicHalfDamage => 3,
    }
}

macro_rules! stat_field {
    ($x:expr, $s:expr) => {
        match $s {
            Stat::Constitution => Some(&mut $x.constitution),
            Stat::Strength => Some(&mut $x.strength),
            Stat::Dexterity => Some(&mut $x.dexterity),
            Stat::Speed => Some(&mut $x.speed),
            Stat::Wisdom => Some(&mut $x.wisdom),
            Stat::Dodge => Some(&mut $x.dodge),
            Stat::Parry => Some(&mut $x.parry),
            Stat::ArmorSkill => Some(&mut $x.armor_skill),
            Stat::Melee => Some(&mut $x.melee),
            Stat::Missile => Some(&mut $x.missile),
            Stat::Power => Some(&mut $x.power),
            Stat::LifeForce => Some(&mut $x.life_force),
            Stat::MagicDefense => None,
        }
    };
}

pub(crate) fn add_possession(c: &mut Creature, thing: ThingId) {
    if let Some(p) = c.possessions.iter_mut().find(|p| p.thing == thing) {
        p.count += 1;
    } else {
        c.possessions.push(Possession { thing, count: 1 });
    }
}

/// Removes one; clears the readied slot when the last goes. False if none.
pub(crate) fn take_possession(c: &mut Creature, thing: ThingId) -> bool {
    if thing == GOLD {
        if c.wealth == 0 {
            return false;
        }
        c.wealth -= 1;
        return true;
    }
    let Some(i) = c.possessions.iter().position(|p| p.thing == thing && p.count > 0) else {
        return false;
    };
    c.possessions[i].count -= 1;
    if c.possessions[i].count == 0 {
        c.possessions.remove(i);
        if c.weapon == thing {
            c.weapon = 0;
        }
        if c.armor == thing {
            c.armor = 0;
        }
    }
    true
}

impl Game {
    // ---- individuals ---------------------------------------------------------

    /// Whether `who` is a living player or a creature in use.
    pub fn alive(&self, who: Who) -> bool {
        match who {
            Who::Player { player } => self.players.get(player).is_some_and(|p| p.alive),
            Who::Creature { region, slot } => self.creature(region, slot).is_some(),
        }
    }

    /// Where an individual stands.
    pub fn location(&self, who: Who) -> Option<Location> {
        match who {
            Who::Player { player } => self.players.get(player).map(|p| p.at),
            Who::Creature { region, slot } => self.creature(region, slot).map(|c| Location::Room {
                region,
                room: c.room,
                x: c.x,
                y: c.y,
            }),
        }
    }

    /// A rating of any individual (magic defense as its level 0..=3).
    pub fn stat(&self, who: Who, stat: Stat) -> Option<u8> {
        if stat == Stat::MagicDefense {
            return match who {
                Who::Player { player } => self.players.get(player).map(|p| level(p.magic_defense)),
                Who::Creature { region, slot } => self.creature(region, slot).map(|c| level(c.magic_defense)),
            };
        }
        match who {
            Who::Player { player } => {
                let mut copy = self.players.get(player)?.clone();
                stat_field!(copy, stat).copied()
            }
            Who::Creature { region, slot } => {
                let mut copy = self.creature(region, slot)?.clone();
                stat_field!(copy, stat).copied()
            }
        }
    }

    pub(crate) fn set_stat(&mut self, who: Who, stat: Stat, value: u8) {
        if stat == Stat::MagicDefense {
            let m = MagicDefense::from_code(value);
            match who {
                Who::Player { player } => self.players[player].magic_defense = m,
                Who::Creature { region, slot } => {
                    if let Some(c) = self.creature_mut(region, slot) {
                        c.magic_defense = m;
                    }
                }
            }
            return;
        }
        let field = match who {
            Who::Player { player } => stat_field!(self.players[player], stat),
            Who::Creature { region, slot } => match self.creature_mut(region, slot) {
                Some(c) => stat_field!(c, stat),
                None => None,
            },
        };
        if let Some(f) = field {
            *f = value;
        }
    }

    /// How many of `thing` an individual owns (gold in pieces).
    pub fn owned(&self, who: Who, thing: ThingId) -> u16 {
        match who {
            Who::Player { player } => self.players.get(player).map_or(0, |p| p.count(thing)),
            Who::Creature { region, slot } => self
                .creature(region, slot)
                .map_or(0, |c| creatures::count(c, thing)),
        }
    }

    /// Adds one within the carry limits; false at the limit.
    fn give_to(&mut self, who: Who, thing: ThingId) -> bool {
        if self.owned(who, thing) >= inventory::limit(thing) {
            return false;
        }
        match who {
            Who::Player { player } => {
                let p = &mut self.players[player];
                if thing == GOLD {
                    p.gold += 1;
                } else {
                    p.possessions.push(thing);
                }
            }
            Who::Creature { region, slot } => {
                if let Some(c) = self.creature_mut(region, slot) {
                    if thing == GOLD {
                        c.wealth += 1;
                    } else {
                        add_possession(c, thing);
                    }
                }
            }
        }
        true
    }

    pub(crate) fn take_from(&mut self, who: Who, thing: ThingId) -> bool {
        match who {
            Who::Player { player } => self.take_one(player, thing),
            Who::Creature { region, slot } => self
                .creature_mut(region, slot)
                .is_some_and(|c| take_possession(c, thing)),
        }
    }

    fn vitals_mut(&mut self, who: Who) -> &mut Vitals {
        let i = match self.vitals.iter().position(|v| v.who == who) {
            Some(i) => i,
            None => {
                self.vitals.push(Vitals {
                    who,
                    temp: 0,
                    turns: 0,
                });
                self.vitals.len() - 1
            }
        };
        &mut self.vitals[i]
    }

    /// The temporary part of an individual's Life Force (5.2).
    pub fn temporary_life(&self, who: Who) -> i16 {
        self.vitals.iter().find(|v| v.who == who).map_or(0, |v| v.temp)
    }

    pub(crate) fn forget_vitals(&mut self, who: Who) {
        self.vitals.retain(|v| v.who != who);
    }

    /// Living individuals in a room, players by number then creatures by slot.
    fn individuals_in_room(&self, region: u8, room: u8) -> Vec<Who> {
        self.others_in_room(region, room, None)
            .into_iter()
            .map(|o| o.who)
            .collect()
    }

    // ---- the effects ---------------------------------------------------------

    /// Runs one spell: the event with its message first, then the effect.
    pub(crate) fn run_spell(&mut self, s: Spell, ev: &mut Vec<Event>) {
        use SpellActionKind::*;
        let individual = acts_on_individual(s.action.kind);
        ev.push(Event::SpellTriggered {
            by: s.by,
            target: individual.then_some(s.target),
            source: s.source,
            thing: s.thing,
            action: s.action,
            message: s.message.clone(),
        });
        if individual && !self.alive(s.target) {
            return;
        }
        let p = s.action.parameter;
        match s.action.kind {
            KillAllButOwnerOf => self.kill_all_but(&s, p, ev),
            SummonOrBanish if p == 0 => self.banish_all(&s, ev),
            SummonOrBanish => self.summon(&s, p, ev),
            IncreaseMagicDefense | DecreaseMagicDefense => {
                let now = self.stat(s.target, Stat::MagicDefense).unwrap_or(0);
                let bound = (p & 3) as u8;
                let to = if s.action.kind == IncreaseMagicDefense {
                    if now < bound {
                        now + 1
                    } else {
                        now
                    }
                } else if now > bound {
                    now - 1
                } else {
                    now
                };
                self.change_stat(s.target, Stat::MagicDefense, to, ev);
            }
            IncreaseStat | DecreaseStat => {
                let Some(stat) = Stat::from_spell_parameter(p) else {
                    return;
                };
                let now = self.stat(s.target, stat).unwrap_or(0) as i16;
                let step = if stat.is_skill() { 5 } else { 1 };
                let to = if s.action.kind == IncreaseStat {
                    now + step
                } else {
                    now - step
                };
                self.change_stat(s.target, stat, to.clamp(0, stat.max() as i16) as u8, ev);
            }
            ChangePower => {
                let now = self.stat(s.target, Stat::Power).unwrap_or(0) as i16;
                let to = (now + amount(p).0).clamp(0, Stat::Power.max() as i16) as u8;
                self.change_stat(s.target, Stat::Power, to, ev);
            }
            ChangeLifeForce => {
                let (n, temporary) = amount(p);
                self.change_life(s.target, n, temporary, ev);
            }
            Give => {
                let kept = self.give_to(s.target, p);
                ev.push(Event::Given {
                    who: s.target,
                    thing: p,
                    kept,
                });
            }
            LongMessage => {
                if let Some(text) = self.long_message(p) {
                    ev.push(Event::LongMessage { number: p, text });
                }
            }
            PlayMusic => ev.push(Event::Music {
                // Classic: 1..=30, from a one-byte parameter.
                number: (p as u8).saturating_add(1),
            }),
            RidRoomOf => {
                if let Location::Room { region, room, .. } = s.at {
                    let count = self.rid_room(region, room, p);
                    ev.push(Event::RoomRidOf {
                        region,
                        room,
                        thing: p,
                        count,
                    });
                }
            }
            AddToRoom => {
                if let Location::Room { region, room, x, y } = s.at {
                    let spot = if self.inside(region, room, x, y) {
                        Some((x, y))
                    } else {
                        self.nearest_free(region, room, x, y)
                    };
                    if let Some((x, y)) = spot {
                        self.put_on_top(region, room, x, y, p, 1);
                        ev.push(Event::ThingAdded {
                            thing: p,
                            at: Location::Room { region, room, x, y },
                        });
                    }
                }
            }
            ActivateAll => self.activate_all(&s, ev),
            Nothing | Invalid(_) => {}
        }
    }

    fn change_stat(&mut self, who: Who, stat: Stat, to: u8, ev: &mut Vec<Event>) {
        let from = self.stat(who, stat).unwrap_or(0);
        if from == to {
            return;
        }
        self.set_stat(who, stat, to);
        ev.push(Event::StatChanged { who, stat, from, to });
    }

    /// 3.8 / 5.2: a temporary change moves the temporary part too; at 0 the
    /// individual dies.
    pub(crate) fn change_life(&mut self, who: Who, delta: i16, temporary: bool, ev: &mut Vec<Event>) {
        let from = self.stat(who, Stat::LifeForce).unwrap_or(0) as i16;
        let to = (from + delta).clamp(0, Stat::LifeForce.max() as i16);
        if temporary && to != from {
            self.vitals_mut(who).temp += to - from;
        }
        self.change_stat(who, Stat::LifeForce, to as u8, ev);
        if to == 0 {
            self.kill(who, ev);
        }
    }

    fn kill_all_but(&mut self, s: &Spell, thing: ThingId, ev: &mut Vec<Event>) {
        let victims = match s.at {
            Location::Room { region, room, .. } => self.individuals_in_room(region, room),
            Location::World { .. } => vec![s.target],
        };
        for who in victims {
            if self.alive(who) && self.owned(who, thing) == 0 {
                self.kill(who, ev);
            }
        }
    }

    fn banish_all(&mut self, s: &Spell, ev: &mut Vec<Event>) {
        let Location::Room { region, room, .. } = s.at else {
            return;
        };
        for who in self.individuals_in_room(region, room) {
            if let Who::Creature { region, slot } = who {
                if who != s.by {
                    self.remove_creature(region, slot, RemovalReason::Banished, ev);
                }
            }
        }
    }

    fn summon(&mut self, s: &Spell, creature: u16, ev: &mut Vec<Event>) {
        match s.at {
            Location::Room { region, room, x, y } => self.summons.push(PendingSummon {
                region,
                room,
                x,
                y,
                creature,
            }),
            Location::World { .. } => ev.push(Event::SummonUnanswered { creature }),
        }
    }

    /// 3.2: called at the start of each player turn.
    pub(crate) fn answer_summons(&mut self, ev: &mut Vec<Event>) {
        for s in std::mem::take(&mut self.summons) {
            let record = self
                .adv
                .creatures
                .get((s.creature as usize).wrapping_sub(1))
                .filter(|c| c.in_use)
                .cloned();
            let slot = self
                .region(s.region)
                .and_then(|r| r.residents.iter().position(|c| !c.in_use));
            let spot =
                if self.inside(s.region, s.room, s.x, s.y) && !self.occupied(s.region, s.room, s.x, s.y) {
                    Some((s.x, s.y))
                } else {
                    self.nearest_free(s.region, s.room, s.x, s.y)
                };
            let (Some(mut c), Some(slot), Some((x, y))) = (record, slot, spot) else {
                ev.push(Event::SummonUnanswered { creature: s.creature });
                continue;
            };
            c.in_use = true;
            c.number = slot as u16 + 1;
            c.region = s.region;
            c.room = s.room;
            c.x = x;
            c.y = y;
            c.class = None;
            if let Some(reg) = self.region_mut(s.region) {
                reg.residents[slot] = c;
            }
            self.forget_vitals(Who::Creature {
                region: s.region,
                slot,
            });
            ev.push(Event::CreatureSummoned {
                region: s.region,
                slot,
                creature: s.creature,
                at: Location::Room {
                    region: s.region,
                    room: s.room,
                    x,
                    y,
                },
            });
        }
    }

    fn rid_room(&mut self, region: u8, room: u8, thing: ThingId) -> u16 {
        let Some(r) = self
            .region_mut(region)
            .and_then(|reg| reg.rooms.get_mut(room as usize))
        else {
            return 0;
        };
        let before = r.items.len();
        r.items.retain(|it| it.thing != thing);
        (before - r.items.len()) as u16
    }

    /// 3.14: the spells lower in the stack, top to bottom.
    fn activate_all(&mut self, s: &Spell, ev: &mut Vec<Event>) {
        let Location::Room { region, room, x, y } = s.at else {
            return;
        };
        let Some(r) = self.room(region, room) else {
            return;
        };
        let below: Vec<(usize, ThingId, u8)> = r
            .items
            .iter()
            .enumerate()
            .filter(|(i, it)| it.x == x && it.y == y && s.stack.is_none_or(|top| *i > top))
            .map(|(i, it)| (i, it.thing, it.parameter))
            .collect();
        let mut spells = Vec::new();
        for (index, thing, parameter) in below {
            let Some(t) = self.thing(thing) else { continue };
            let message = self.message(&t.spell_message);
            let found = match t.kind {
                ThingKind::Space {
                    access: acs_core::model::Access::InvokeSpell { action },
                    ..
                } => Some((SpellSource::Space, action)),
                ThingKind::Portal {
                    access: acs_core::model::Access::InvokeSpell { action },
                    ..
                } => Some((SpellSource::Portal, action)),
                ThingKind::Obstacle {
                    bump_invokes_spell: true,
                    action,
                    ..
                } => Some((SpellSource::Obstacle, action)),
                ThingKind::CustomObstacle { action, .. } => Some((
                    SpellSource::CustomObstacle,
                    SpellAction {
                        kind: action.kind,
                        parameter: parameter.into(),
                    },
                )),
                ThingKind::CustomSpace {
                    placement_choice,
                    trigger,
                    action,
                    ..
                } => {
                    let (needed, action) =
                        crate::step::resolve_custom_space(placement_choice, action, parameter);
                    let owns = self.owned(s.target, needed) > 0;
                    let fires = match trigger {
                        CustomTrigger::WhenMovedOnto => true,
                        CustomTrigger::WhenDroppedHere => false,
                        CustomTrigger::OpenToOwnersOf => owns,
                        CustomTrigger::OpenToNonOwnersOf => !owns,
                    };
                    fires.then_some((SpellSource::CustomSpace, action))
                }
                ThingKind::MagicItem { action, .. } | ThingKind::MagicSpell { action, .. } => {
                    Some((SpellSource::Stack, action))
                }
                _ => None,
            };
            if let Some((source, action)) = found {
                if action.kind != SpellActionKind::ActivateAll {
                    spells.push((index, thing, source, action, message));
                }
            }
        }
        for (index, thing, source, action, message) in spells {
            self.run_spell(
                Spell {
                    by: s.by,
                    target: s.target,
                    source,
                    thing,
                    action,
                    message,
                    at: s.at,
                    stack: Some(index),
                },
                ev,
            );
        }
    }

    // ---- places --------------------------------------------------------------

    fn inside(&self, region: u8, room: u8, x: u8, y: u8) -> bool {
        self.room(region, room)
            .is_some_and(|r| x > 0 && y > 0 && x + 1 < r.width && y + 1 < r.height)
    }

    /// The nearest inside square with no individual and no thing, in rings
    /// of growing distance, top row first (creatures 5.4).
    pub(crate) fn nearest_free(&self, region: u8, room: u8, x: u8, y: u8) -> Option<(u8, u8)> {
        let r = self.room(region, room)?;
        let (w, h) = (r.width as i16, r.height as i16);
        for d in 1..w.max(h) {
            for ny in (y as i16 - d)..=(y as i16 + d) {
                for nx in (x as i16 - d)..=(x as i16 + d) {
                    if (nx - x as i16).abs().max((ny - y as i16).abs()) != d {
                        continue;
                    }
                    if nx <= 0 || ny <= 0 || nx >= w - 1 || ny >= h - 1 {
                        continue;
                    }
                    let (nx, ny) = (nx as u8, ny as u8);
                    let has_item = r.items.iter().any(|it| it.x == nx && it.y == ny);
                    if !has_item && !self.occupied(region, room, nx, ny) {
                        return Some((nx, ny));
                    }
                }
            }
        }
        None
    }

    /// Long message 1..=255 as one line (lines are split by position).
    fn long_message(&self, number: u16) -> Option<String> {
        let s = self.adv.long_messages.get((number as usize).checked_sub(1)?)?;
        let joined: String = s.lines().collect();
        let t = joined.split_whitespace().collect::<Vec<_>>().join(" ");
        (!t.is_empty()).then_some(t)
    }

    // ---- casting and using (section 4) ---------------------------------------

    /// 2.4: the caster, or a living individual in the caster's room.
    pub(crate) fn valid_target(&self, by: Who, target: Who) -> bool {
        if target == by {
            return true;
        }
        if !self.alive(target) {
            return false;
        }
        match (self.location(by), self.location(target)) {
            (Some(a), Some(b)) => a.room().is_some() && a.room() == b.room(),
            _ => false,
        }
    }

    /// Casts `thing` (a magic spell `by` owns); true if it worked (4.1).
    pub(crate) fn cast(&mut self, by: Who, thing: ThingId, target: Who, ev: &mut Vec<Event>) -> bool {
        let Some(t) = self.thing(thing).cloned() else {
            return false;
        };
        let ThingKind::MagicSpell {
            disappears_after_use,
            power: cost,
            action,
        } = t.kind
        else {
            return false;
        };
        let power = self.stat(by, Stat::Power).unwrap_or(0);
        let chance = cast_chance(power, cost);
        let success = cast_roll(&mut self.rng, power, cost);
        self.set_stat(by, Stat::Power, power.saturating_sub(cost));
        ev.push(Event::SpellCast {
            by,
            thing,
            target,
            chance,
            cost,
            success,
        });
        if !success {
            return false;
        }
        let Some(at) = self.location(by) else {
            return true;
        };
        if disappears_after_use && self.take_from(by, thing) {
            ev.push(Event::SpellGone { who: by, thing });
        }
        self.run_spell(
            Spell {
                by,
                target,
                source: SpellSource::Cast,
                thing,
                action,
                message: self.message(&t.spell_message),
                at,
                stack: None,
            },
            ev,
        );
        true
    }

    /// Uses `thing` (a magic item `by` owns that acts when used) (4.2): 2
    /// Power, then the item's spell.
    pub(crate) fn use_item(&mut self, by: Who, thing: ThingId, target: Who, ev: &mut Vec<Event>) {
        let Some(t) = self.thing(thing).cloned() else {
            return;
        };
        let ThingKind::MagicItem {
            disappears_after_use,
            action,
            ..
        } = t.kind
        else {
            return;
        };
        let Some(at) = self.location(by) else {
            return;
        };
        ev.push(Event::ItemUsed { by, thing, target });
        // 4.2: a use costs 2 Power, not below 0.
        let power = self.stat(by, Stat::Power).unwrap_or(0);
        self.set_stat(by, Stat::Power, power.saturating_sub(USE_POWER));
        if disappears_after_use && self.take_from(by, thing) {
            ev.push(Event::SpellGone { who: by, thing });
        }
        self.run_spell(
            Spell {
                by,
                target,
                source: SpellSource::Used,
                thing,
                action,
                message: self.message(&t.spell_message),
                at,
                stack: None,
            },
            ev,
        );
    }

    fn check_cast(&self, thing: ThingId, target: Option<Who>, usable: bool) -> Result<Who, CastRefusal> {
        let p = self.current;
        let by = Who::Player { player: p };
        if thing == GOLD || !self.players[p].owns(thing) {
            return Err(CastRefusal::NotCarried);
        }
        let ok = match self.thing(thing).map(|t| &t.kind) {
            Some(ThingKind::MagicSpell { .. }) => !usable,
            Some(ThingKind::MagicItem { on_use, .. }) => usable && *on_use,
            _ => false,
        };
        if !ok {
            return Err(if usable {
                CastRefusal::NotUsable
            } else {
                CastRefusal::NotASpell
            });
        }
        let target = target.unwrap_or(by);
        if !self.valid_target(by, target) {
            return Err(CastRefusal::BadTarget);
        }
        Ok(target)
    }

    /// `Cast { thing, target }`: 4.1, 4.3.
    pub(crate) fn cast_command(&mut self, thing: ThingId, target: Option<Who>, ev: &mut Vec<Event>) {
        let p = self.current;
        match self.check_cast(thing, target, false) {
            Err(reason) => ev.push(Event::CastRefused {
                player: p,
                thing,
                reason,
            }),
            Ok(target) => {
                self.cast(Who::Player { player: p }, thing, target, ev);
                if self.players[p].alive {
                    self.end_turn(ev);
                }
            }
        }
    }

    /// `Use { thing, target }`: 4.2, 4.3.
    pub(crate) fn use_command(&mut self, thing: ThingId, target: Option<Who>, ev: &mut Vec<Event>) {
        let p = self.current;
        match self.check_cast(thing, target, true) {
            Err(reason) => ev.push(Event::CastRefused {
                player: p,
                thing,
                reason,
            }),
            Ok(target) => {
                self.use_item(Who::Player { player: p }, thing, target, ev);
                // 4.3: using spends one step; the turn goes on.
                if self.players[p].alive && !self.over {
                    self.spend(ev);
                }
            }
        }
    }

    // ---- time (section 5) ----------------------------------------------------

    /// End of an individual's own turn: Power drift, temporary Life Force
    /// wearing off, natural healing.
    pub(crate) fn upkeep(&mut self, who: Who, ev: &mut Vec<Event>) {
        if !self.alive(who) {
            return;
        }
        let v = self.vitals_mut(who);
        v.turns += 1;
        let temp = v.temp;
        let power = self.stat(who, Stat::Power).unwrap_or(0);
        let wisdom = self.stat(who, Stat::Wisdom).unwrap_or(0);
        if power != wisdom {
            let to = if power < wisdom { power + 1 } else { power - 1 };
            self.set_stat(who, Stat::Power, to);
        }
        // 5.3: healing first, on Life Force itself, not above Constitution.
        let life = self.stat(who, Stat::LifeForce).unwrap_or(0) as i16;
        let con = self.stat(who, Stat::Constitution).unwrap_or(0) as i16;
        let mut to = life;
        if to < con {
            let extra = i16::from(self.rng.below(HEAL_EXTRA) == 0);
            to = (to + 1 + extra).min(con);
        }
        // 5.2: then the temporary part's tick.
        if temp != 0 && self.rng.below(TEMP_TICK) == 0 {
            let step = -temp.signum();
            to = (to + step).clamp(0, Stat::LifeForce.max() as i16);
            self.vitals_mut(who).temp += step;
        }
        if to != life {
            self.set_stat(who, Stat::LifeForce, to as u8);
            if to == 0 {
                self.kill(who, ev);
            }
        }
    }

    // ---- death (section 7) ---------------------------------------------------

    /// Life Force 0: the dead leave what they carried and are gone.
    pub(crate) fn kill(&mut self, who: Who, ev: &mut Vec<Event>) {
        let Some(at) = self.location(who).filter(|_| self.alive(who)) else {
            return;
        };
        let (mut things, gold): (Vec<ThingId>, u16) = match who {
            Who::Player { player } => {
                let p = &self.players[player];
                (p.possessions.clone(), p.gold)
            }
            Who::Creature { region, slot } => {
                let c = self.creature(region, slot).expect("alive");
                let mut v = Vec::new();
                for p in &c.possessions {
                    v.extend(std::iter::repeat_n(p.thing, p.count as usize));
                }
                (v, c.wealth)
            }
        };
        things.sort_unstable();
        let mut left = Vec::new();
        if let Location::Room { region, room, x, y } = at {
            for t in things {
                let keeps = match self.thing(t).map(|t| &t.kind) {
                    Some(ThingKind::Treasure {
                        disappears_when_dropped,
                        ..
                    }) => !disappears_when_dropped,
                    Some(ThingKind::MagicItem { .. }) => true,
                    Some(
                        ThingKind::MissileWeapon { stats, .. }
                        | ThingKind::MeleeWeapon { stats, .. }
                        | ThingKind::Armor { stats, .. },
                    ) => stats.usable_by_anyone,
                    _ => false,
                };
                if keeps {
                    self.put_on_top(region, room, x, y, t, 1);
                    left.push((t, 1));
                }
            }
            let gold_vanishes = matches!(
                self.thing(GOLD).map(|t| &t.kind),
                Some(ThingKind::Treasure {
                    disappears_when_dropped: true,
                    ..
                })
            );
            let mut g = if gold_vanishes { 0 } else { gold };
            while g > 0 {
                let n = g.min(PILE_LIMIT);
                self.put_on_top(region, room, x, y, GOLD, n as u8);
                left.push((GOLD, n));
                g -= n;
            }
        }
        match who {
            Who::Player { player } => {
                let p = &mut self.players[player];
                p.alive = false;
                p.possessions.clear();
                p.gold = 0;
                p.weapon = None;
                p.armor = None;
            }
            Who::Creature { region, slot } => {
                if let Some(c) = self.creature_mut(region, slot) {
                    c.in_use = false;
                    c.possessions.clear();
                    c.wealth = 0;
                }
                self.random.retain(|&r| r != (region, slot));
            }
        }
        self.forget_vitals(who);
        ev.push(Event::Died { who, at, left });
    }
}
