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

/// Per-individual state the record has no room for (5.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Vitals {
    pub who: Who,
    /// Turns of its own since the game began (or since it arrived).
    pub turns: u32,
}

/// A temporary change still to be taken back (5.2): due in round `due`,
/// Life Force moves by `step`; then, with `count` left, again a die roll
/// of rounds later.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Reversal {
    pub who: Who,
    pub due: u32,
    pub step: i8,
    pub count: u8,
}

/// Reversals queued at most (5.2).
pub const REVERSALS: usize = 30;

/// Never filled any more: summonses are made at once (3.2). The field that
/// held them stays, always empty, so a game's fingerprint (saves 3) is the
/// same as before for every game without a summons.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PendingSummon {
    pub region: u8,
    pub room: u8,
    pub x: u8,
    pub y: u8,
    pub creature: u16,
}

/// What a cast or a use acts on (2.4): the individual on the target
/// square (none on an empty one) and the square.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Aim {
    pub who: Option<Who>,
    pub at: Location,
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
    /// Where it takes effect (room spells, stacks).
    pub at: Location,
    /// Where a summons (3.2) and Add to Room (3.13) take effect: a cast's
    /// or use's target square (2.4); with no target press, the square of
    /// the thing that set it off: the space stepped on, the obstacle's own
    /// square (the bumper's own for one on the wall line), a portal's own
    /// square in the room left (the traveller's own for one on the wall
    /// line), the item's square for a pick-up, the drop square.
    pub on: Location,
    /// The holding thing's index in the room's item list, when it lies in a
    /// stack at `at` (for Activate All, 3.14).
    pub stack: Option<usize>,
}

/// Whether an action acts on one individual (2.5; Kill All, 3.1).
pub fn acts_on_individual(kind: SpellActionKind) -> bool {
    use SpellActionKind::*;
    matches!(
        kind,
        KillAllButOwnerOf
            | IncreaseMagicDefense
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
/// The top-of-turn healing die heals on this face (5.3).
pub const HEAL_FACE: u32 = 2;

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

/// The lowest and highest chance a spell with a cost has (4.1).
pub const CAST_CHANCE_MIN: u8 = 14;
pub const CAST_CHANCE_MAX: u8 = 90;

/// The Power a cast of `cost` spends from `power` (4.1): the cost, or all
/// of the caster's Power if that is less. Spent whether or not it works.
pub fn cast_spend(power: u8, cost: u8) -> u8 {
    cost.min(power)
}

/// The chance in percent that a spell of `cost` works for `power` (4.1):
/// a cost of 0 always works; otherwise 90 x the Power spent / the cost,
/// rounded down, held to 14..90.
pub fn cast_chance(power: u8, cost: u8) -> u8 {
    if cost == 0 {
        return 100;
    }
    let chance = 90 * cast_spend(power, cost) as u32 / cost as u32;
    chance.clamp(CAST_CHANCE_MIN as u32, CAST_CHANCE_MAX as u32) as u8
}

/// Rolls a cast of `cost` for `power` (4.1): a cost of 0 takes no roll and
/// works; otherwise a roll of 0..99 works when it is below the chance.
pub fn cast_roll(rng: &mut crate::Rng, power: u8, cost: u8) -> bool {
    cost == 0 || rng.below(100) < cast_chance(power, cost) as u32
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
                self.vitals.push(Vitals { who, turns: 0 });
                self.vitals.len() - 1
            }
        };
        &mut self.vitals[i]
    }

    /// The Life Force a temporary change will still take back (5.2): a
    /// temporary +10 reads 11 at once (it is taken back once more than it
    /// was given), 0 when nothing is pending.
    pub fn temporary_life(&self, who: Who) -> i16 {
        self.reversals
            .iter()
            .filter(|r| r.who == who)
            .map(|r| -(r.step as i16) * (r.count as i16 + 1))
            .sum()
    }

    pub(crate) fn forget_vitals(&mut self, who: Who) {
        self.vitals.retain(|v| v.who != who);
        self.reversals.retain(|r| r.who != who);
        if let Who::Player { player } = who {
            self.carried.retain(|&(q, _)| q != player);
        }
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
            // 3.1: only the individual it acts on, unless it owns the thing.
            KillAllButOwnerOf => {
                if self.owned(s.target, p) == 0 {
                    self.kill(s.target, ev);
                }
            }
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
                // 3.13: on top of the square it takes effect on (`on`): a
                // cast's target, the space's or obstacle's own square, the
                // bumper's for a thing on the wall line, a portal's own
                // square in the room left, the item's square for a pick-up,
                // the drop square. Nothing on the world map.
                if let Location::Room { region, room, x, y } = s.on {
                    self.put_on_top(region, room, x, y, p, 1);
                    ev.push(Event::ThingAdded {
                        thing: p,
                        at: Location::Room { region, room, x, y },
                    });
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
        if temporary && to != from && self.reversals.len() < REVERSALS {
            let due = self.sched.round + 1 + self.rng.below(6);
            self.reversals.push(Reversal {
                who,
                due,
                step: -(to - from).signum() as i8,
                count: (to - from).unsigned_abs().min(255) as u8,
            });
        }
        self.change_stat(who, Stat::LifeForce, to as u8, ev);
        if to == 0 {
            self.kill(who, ev);
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

    /// Whether the region has a free resident slot (3.2).
    pub(crate) fn has_free_slot(&self, region: u8) -> bool {
        self.region(region)
            .is_some_and(|r| r.residents.iter().any(|c| !c.in_use))
    }

    /// 3.2: a copy of master creature `creature`, made at once in the
    /// region's lowest free slot on the square the spell takes effect on,
    /// shared with whoever stands there; it takes no turn this round.
    fn summon(&mut self, s: &Spell, creature: u16, ev: &mut Vec<Event>) {
        let Location::Room { region, room, x, y } = s.on else {
            ev.push(Event::SummonUnanswered { creature });
            return;
        };
        let record = self
            .adv
            .creatures
            .get((creature as usize).wrapping_sub(1))
            .filter(|c| c.in_use)
            .cloned();
        let slot = self
            .region(region)
            .and_then(|r| r.residents.iter().position(|c| !c.in_use));
        let (Some(mut c), Some(slot)) = (record, slot) else {
            ev.push(Event::SummonUnanswered { creature });
            return;
        };
        c.in_use = true;
        c.number = slot as u16 + 1;
        c.region = region;
        c.room = room;
        c.x = x;
        c.y = y;
        c.class = None;
        if let Some(reg) = self.region_mut(region) {
            reg.residents[slot] = c;
        }
        let who = Who::Creature { region, slot };
        // Creatures 2.4: a summons clears the slot's random mark.
        self.fill_slot(region, slot, false);
        // No turn in this round: a dead creature's slot may still be
        // listed further on.
        let pos = self.sched.pos.min(self.sched.list.len());
        let mut rest = self.sched.list.split_off(pos);
        rest.retain(|&w| w != who);
        self.sched.list.extend(rest);
        ev.push(Event::CreatureSummoned {
            region,
            slot,
            creature,
            at: Location::Room { region, room, x, y },
        });
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
                    on: s.at,
                    stack: Some(index),
                },
                ev,
            );
        }
    }

    // ---- places --------------------------------------------------------------

    pub(crate) fn inside(&self, region: u8, room: u8, x: u8, y: u8) -> bool {
        self.room(region, room)
            .is_some_and(|r| x > 0 && y > 0 && x + 1 < r.width && y + 1 < r.height)
    }

    /// Long message 1..=255 as one line (lines are split by position).
    fn long_message(&self, number: u16) -> Option<String> {
        let s = self.adv.long_messages.get((number as usize).checked_sub(1)?)?;
        let joined: String = s.lines().collect();
        let t = joined.split_whitespace().collect::<Vec<_>>().join(" ");
        (!t.is_empty()).then_some(t)
    }

    // ---- casting and using (section 4) ---------------------------------------

    /// 2.4: what a cast or use aimed at `target` (an individual) acts on.
    pub(crate) fn aim_at(&self, target: Who) -> Option<Aim> {
        Some(Aim {
            who: Some(target),
            at: self.location(target)?,
        })
    }

    /// 2.4: the player's target, a square of the caster's room (`at`), or
    /// the square an individual stands on now (`target`), or the caster's
    /// own when neither is given. On the world map only the caster's own.
    fn aim(&self, by: Who, at: Option<(u8, u8)>, target: Option<Who>) -> Result<Aim, CastRefusal> {
        let here = self.location(by).ok_or(CastRefusal::BadTarget)?;
        let (hx, hy) = match here {
            Location::World { x, y } | Location::Room { x, y, .. } => (x, y),
        };
        match (at, target) {
            (Some((x, y)), _) => match here {
                Location::World { .. } if (x, y) == (hx, hy) => Ok(Aim {
                    who: Some(by),
                    at: here,
                }),
                Location::World { .. } => Err(CastRefusal::BadTarget),
                Location::Room { region, room, .. } => {
                    if !self.inside(region, room, x, y) {
                        return Err(CastRefusal::BadTarget);
                    }
                    let who = if (x, y) == (hx, hy) {
                        Some(by)
                    } else {
                        self.others_in_room(region, room, Some(by))
                            .into_iter()
                            .find(|o| (o.x, o.y) == (x, y))
                            .map(|o| o.who)
                    };
                    Ok(Aim {
                        who,
                        at: Location::Room { region, room, x, y },
                    })
                }
            },
            (None, Some(t)) if t != by => match self.location(t).filter(|_| self.alive(t)) {
                Some(l) if here.room().is_some() && l.room() == here.room() => {
                    Ok(Aim { who: Some(t), at: l })
                }
                _ => Err(CastRefusal::BadTarget),
            },
            _ => Ok(Aim {
                who: Some(by),
                at: here,
            }),
        }
    }

    /// Casts `thing` (a magic spell `by` owns) at `aim`; true if it worked
    /// (4.1). A summons into a full region spends and rolls as any cast,
    /// then fails whatever the roll (3.2); at cost 0 nothing is spent or
    /// drawn.
    pub(crate) fn cast(&mut self, by: Who, thing: ThingId, aim: Aim, ev: &mut Vec<Event>) -> bool {
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
        let full = action.kind == SpellActionKind::SummonOrBanish
            && action.parameter != 0
            && matches!(self.location(by), Some(Location::Room { region, .. }) if !self.has_free_slot(region));
        let unanswered = Event::SummonUnanswered {
            creature: action.parameter,
        };
        if full && cost == 0 {
            ev.push(unanswered);
            return false;
        }
        let target = aim.who.unwrap_or(by);
        let power = self.stat(by, Stat::Power).unwrap_or(0);
        let chance = cast_chance(power, cost);
        let success = cast_roll(&mut self.rng, power, cost) && !full;
        self.set_stat(by, Stat::Power, power - cast_spend(power, cost));
        ev.push(Event::SpellCast {
            by,
            thing,
            target,
            chance,
            cost,
            success,
        });
        if full {
            ev.push(unanswered);
        }
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
                on: aim.at,
                stack: None,
            },
            ev,
        );
        true
    }

    /// Uses `thing` (a magic item `by` owns that acts when used) at `aim`
    /// (4.2): 2 Power, then the item's spell.
    pub(crate) fn use_item(&mut self, by: Who, thing: ThingId, aim: Aim, ev: &mut Vec<Event>) {
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
        let target = aim.who.unwrap_or(by);
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
                on: aim.at,
                stack: None,
            },
            ev,
        );
    }

    fn check_cast(
        &self,
        thing: ThingId,
        at: Option<(u8, u8)>,
        target: Option<Who>,
        usable: bool,
    ) -> Result<Aim, CastRefusal> {
        let p = self.current;
        let by = Who::Player { player: p };
        if thing == GOLD || !self.players[p].owns(thing) {
            return Err(CastRefusal::NotCarried);
        }
        let action = match self.thing(thing).map(|t| &t.kind) {
            Some(ThingKind::MagicSpell { action, .. }) if !usable => *action,
            Some(ThingKind::MagicItem { on_use, action, .. }) if usable && *on_use => *action,
            _ => {
                return Err(if usable {
                    CastRefusal::NotUsable
                } else {
                    CastRefusal::NotASpell
                })
            }
        };
        let aim = self.aim(by, at, target)?;
        // 2.4: a spell that acts on an individual needs one there.
        if aim.who.is_none() && acts_on_individual(action.kind) {
            return Err(CastRefusal::NoOneThere);
        }
        Ok(aim)
    }

    /// `Cast { thing, at, target }`: 2.4, 4.1, 4.3.
    pub(crate) fn cast_command(
        &mut self,
        thing: ThingId,
        at: Option<(u8, u8)>,
        target: Option<Who>,
        ev: &mut Vec<Event>,
    ) {
        let p = self.current;
        match self.check_cast(thing, at, target, false) {
            Err(reason) => ev.push(Event::CastRefused {
                player: p,
                thing,
                reason,
            }),
            Ok(aim) => {
                self.cast(Who::Player { player: p }, thing, aim, ev);
                if self.players[p].alive {
                    self.end_turn(ev);
                }
            }
        }
    }

    /// `Use { thing, at, target }`: 2.4, 4.2, 4.3.
    pub(crate) fn use_command(
        &mut self,
        thing: ThingId,
        at: Option<(u8, u8)>,
        target: Option<Who>,
        ev: &mut Vec<Event>,
    ) {
        let p = self.current;
        match self.check_cast(thing, at, target, true) {
            Err(reason) => ev.push(Event::CastRefused {
                player: p,
                thing,
                reason,
            }),
            Ok(aim) => {
                self.use_item(Who::Player { player: p }, thing, aim, ev);
                // 4.2, 4.3: using spends two steps (ending the turn when
                // fewer are left); otherwise the turn goes on.
                if self.players[p].alive && !self.over {
                    let pl = &mut self.players[p];
                    pl.moves_left = pl.moves_left.saturating_sub(1);
                    self.spend(ev);
                }
            }
        }
    }

    // ---- time (section 5) ----------------------------------------------------

    /// The top of an individual's turn: a creature's catch-up (5.5), Power
    /// drift (5.1), then the healing die (5.3).
    pub(crate) fn turn_top(&mut self, who: Who, ev: &mut Vec<Event>) {
        if !self.alive(who) {
            return;
        }
        if let Who::Creature { region, slot } = who {
            self.catch_up(region, slot);
        }
        self.vitals_mut(who).turns += 1;
        let power = self.stat(who, Stat::Power).unwrap_or(0);
        let wisdom = self.stat(who, Stat::Wisdom).unwrap_or(0);
        if power != wisdom {
            let to = if power < wisdom { power + 1 } else { power - 1 };
            self.set_stat(who, Stat::Power, to);
        }
        let face = self.rng.below(6) + 1;
        if face == HEAL_FACE {
            self.heal_one(who, ev);
        }
    }

    /// 5.5: with N the rounds since the creature's last turn (low bytes,
    /// modulo 256), up to N - 1 Power, never past Wisdom, and up to N / 2
    /// Life Force, never past Constitution; none on its first turn (byte
    /// 0). Then the byte is the current round's low byte, 1 for 0. No
    /// draw. N 0 (a turn at a round whose low byte is 0 notes 1, so the
    /// next round's turn finds N 0) gains nothing: N - 1 saturates.
    fn catch_up(&mut self, region: u8, slot: usize) {
        let now = (self.sched.round & 0xFF) as u8;
        let Some(c) = self.creature_mut(region, slot) else {
            return;
        };
        if c.last_turn_round != 0 {
            let n = now.wrapping_sub(c.last_turn_round);
            if c.power < c.wisdom {
                c.power = c.power.saturating_add(n.saturating_sub(1)).min(c.wisdom);
            }
            if c.life_force < c.constitution {
                c.life_force = c.life_force.saturating_add(n / 2).min(c.constitution);
            }
        }
        c.last_turn_round = now.max(1);
    }

    /// 5.3: `n` idle moves of a turn of `full` moves, each a roll of
    /// 0..full - 1 that heals on a 0; with `full` 0 (a Speed 0 creature's
    /// idle move, creatures 2.7) no roll is drawn and it heals.
    pub(crate) fn idle_moves(&mut self, who: Who, n: u8, full: u8, ev: &mut Vec<Event>) {
        for _ in 0..n {
            if !self.alive(who) {
                return;
            }
            if full == 0 || self.rng.below(u32::from(full)) == 0 {
                self.heal_one(who, ev);
            }
        }
    }

    /// 5.3: 1 point while Life Force is below Constitution.
    fn heal_one(&mut self, who: Who, _ev: &mut Vec<Event>) {
        let life = self.stat(who, Stat::LifeForce).unwrap_or(0);
        let con = self.stat(who, Stat::Constitution).unwrap_or(0);
        if life < con {
            self.set_stat(who, Stat::LifeForce, life + 1);
        }
    }

    /// 5.2: at the end of a round, every reversal due takes its step, then
    /// counts down and is due again a die roll of rounds later, or is done.
    pub(crate) fn reversals_due(&mut self, ev: &mut Vec<Event>) {
        let round = self.sched.round;
        let mut i = 0;
        while i < self.reversals.len() {
            if self.reversals[i].due > round {
                i += 1;
                continue;
            }
            let r = self.reversals[i].clone();
            if !self.alive(r.who) {
                self.reversals.remove(i);
                continue;
            }
            let life = self.stat(r.who, Stat::LifeForce).unwrap_or(0) as i16;
            let to = (life + r.step as i16).clamp(0, Stat::LifeForce.max() as i16);
            if to != life {
                self.set_stat(r.who, Stat::LifeForce, to as u8);
            }
            if r.count >= 1 {
                let due = round + 1 + self.rng.below(6);
                let e = &mut self.reversals[i];
                e.count -= 1;
                e.due = due;
                i += 1;
            } else {
                self.reversals.remove(i);
            }
            if to == 0 {
                self.kill(r.who, ev);
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
            // 7.2: the gold goes down first, as one pile (several only past
            // the pile limit), then the possessions from the lowest thing
            // number up, each on top: the highest-numbered ends on top.
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
                self.put_gold(region, room, x, y, n);
                left.push((GOLD, n));
                g -= n;
            }
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
                // Creatures 2.4 (engine choice): a death keeps the slot's
                // random mark; its carried moves (5.4) go.
                self.creature_carried
                    .retain(|c| (c.region, c.slot) != (region, slot));
            }
        }
        self.forget_vitals(who);
        ev.push(Event::Died { who, at, left });
        // Encounters 7.4: a player's death may leave its area empty.
        if let (Who::Player { .. }, Location::Room { region: 0, room, .. }) = (who, at) {
            self.close_if_empty(room, ev);
        }
    }
}
