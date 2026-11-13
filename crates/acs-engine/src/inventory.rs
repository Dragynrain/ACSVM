// SPDX-License-Identifier: GPL-3.0-or-later

//! Inventory: carry limits, weight and capacity, readying, dropping.
//! Spec: `docs/rules/inventory.md` (section numbers in the comments refer
//! to it). Picking up happens in `step.rs`, since it is a move.

use crate::game::{Game, Player};
use crate::magic::Spell;
use crate::step::resolve_custom_space;
use crate::{Dir, DropRefusal, Event, Location, ReadyRefusal, Slot, SpellSource, Who};
use acs_core::model::ThingId;
use acs_core::model::{Adventure, CustomTrigger, RoomItem, ThingKind};

/// Thing 81 is Gold, counted in pieces (1.3).
pub const GOLD: ThingId = 81;
/// Most gold pieces anyone may own (1.3).
pub const GOLD_LIMIT: u16 = 25_000;
/// Most gold one pile holds: its size is two bytes (5.8), and no one owns
/// more than `GOLD_LIMIT`.
pub const PILE_LIMIT: u16 = GOLD_LIMIT;

/// How many of `thing` one individual may own (1.2-1.4).
pub fn limit(thing: ThingId) -> u16 {
    match thing {
        1..=32 => 3,
        GOLD => GOLD_LIMIT,
        _ => 1,
    }
}

/// Whether a thing kind is carried (1.1).
pub fn carriable(kind: &ThingKind) -> bool {
    matches!(
        kind,
        ThingKind::Treasure { .. }
            | ThingKind::MagicItem { .. }
            | ThingKind::MissileWeapon { .. }
            | ThingKind::MeleeWeapon { .. }
            | ThingKind::Armor { .. }
            | ThingKind::MagicSpell { .. }
    )
}

/// A thing's weight, multiplier applied; 0 for things without one.
pub fn weight(adv: &Adventure, thing: ThingId) -> i32 {
    let Some(t) = adv.things.get((thing as usize).wrapping_sub(1)) else {
        return 0;
    };
    match t.kind {
        ThingKind::Treasure { carry, .. }
        | ThingKind::MagicItem { carry, .. }
        | ThingKind::MissileWeapon { carry, .. }
        | ThingKind::MeleeWeapon { carry, .. }
        | ThingKind::Armor { carry, .. } => carry.weight,
        _ => 0,
    }
}

/// Load against capacity, for the slowdown (3.3, captured on the C64): no
/// slowdown up to half the capacity, then a straight line down to 0 at the
/// full capacity, rounding down.
pub fn slowed_speed(speed: u8, load: i32, capacity: i32) -> u8 {
    if load * 2 <= capacity {
        speed
    } else if load >= capacity || capacity <= 0 {
        0
    } else {
        (speed as i32 * 2 * (capacity - load) / capacity) as u8
    }
}

impl Player {
    /// Total weight carried, gold included (3.1).
    pub fn load(&self, adv: &Adventure) -> i32 {
        let items: i32 = self.possessions.iter().map(|&t| weight(adv, t)).sum();
        items + self.gold as i32 * weight(adv, GOLD)
    }

    /// (Strength - (Constitution - Life Force) / 2) x 100 (3.2).
    pub fn capacity(&self) -> i32 {
        let wound = (self.constitution as i32 - self.life_force as i32) / 2;
        (self.strength as i32 - wound) * 100
    }

    /// Speed after the weight slowdown: where the bar starts (3.3).
    pub fn effective_speed(&self, adv: &Adventure) -> u8 {
        slowed_speed(self.speed, self.load(adv), self.capacity())
    }
}

/// Where a drop lands.
enum Target {
    Map,
    Room {
        region: u8,
        room: u8,
        x: u8,
        y: u8,
        /// The square's top room item before the drop, if any.
        top: Option<RoomItem>,
    },
}

impl Game {
    /// Removes one `thing` from the player; clears a readied slot when the
    /// last copy goes (4.4). False if the player had none.
    pub(crate) fn take_one(&mut self, p: usize, thing: ThingId) -> bool {
        let pl = &mut self.players[p];
        if thing == GOLD {
            if pl.gold == 0 {
                return false;
            }
            pl.gold -= 1;
            return true;
        }
        let Some(i) = pl.possessions.iter().position(|&t| t == thing) else {
            return false;
        };
        pl.possessions.remove(i);
        if !pl.possessions.contains(&thing) {
            if pl.weapon == Some(thing) {
                pl.weapon = None;
            }
            if pl.armor == Some(thing) {
                pl.armor = None;
            }
        }
        true
    }

    /// `Ready { thing }` (section 4).
    pub(crate) fn ready(&mut self, thing: ThingId, ev: &mut Vec<Event>) {
        let p = self.current;
        let refuse = |reason| Event::ReadyRefused {
            player: p,
            thing,
            reason,
        };
        if thing == GOLD || !self.players[p].owns(thing) {
            ev.push(refuse(ReadyRefusal::NotCarried));
            return;
        }
        let slot = match self.thing(thing).map(|t| &t.kind) {
            Some(ThingKind::MeleeWeapon { .. } | ThingKind::MissileWeapon { .. }) => Slot::Weapon,
            Some(ThingKind::Armor { .. }) => Slot::Armor,
            _ => {
                ev.push(refuse(ReadyRefusal::NotWeaponOrArmor));
                return;
            }
        };
        let pl = &mut self.players[p];
        match slot {
            Slot::Weapon => pl.weapon = Some(thing),
            Slot::Armor => pl.armor = Some(thing),
        }
        ev.push(Event::Readied {
            player: p,
            thing,
            slot,
        });
        // 4.3: readying armor uses up the rest of the turn.
        if slot == Slot::Armor {
            self.end_turn(ev);
        }
    }

    /// Resolves the square a drop goes to (5.1, 5.3, 5.4).
    fn drop_target(&self, dir: Option<Dir>) -> Result<Target, DropRefusal> {
        let at = self.players[self.current].at;
        let Location::Room { region, room, x, y } = at else {
            return Ok(Target::Map);
        };
        let r = self.room(region, room).ok_or(DropRefusal::Wall)?;
        let (dx, dy) = dir.map(Dir::delta).unwrap_or((0, 0));
        let (nx, ny) = (x as i16 + dx as i16, y as i16 + dy as i16);
        let (w, h) = (r.width as i16, r.height as i16);
        if nx <= 0 || ny <= 0 || nx >= w - 1 || ny >= h - 1 {
            return Err(DropRefusal::Wall);
        }
        let (nx, ny) = (nx as u8, ny as u8);
        let top = r.items.iter().find(|it| it.x == nx && it.y == ny).cloned();
        if let Some(it) = &top {
            let refuses = matches!(
                self.thing(it.thing).map(|t| &t.kind),
                Some(
                    ThingKind::Portal { .. }
                        | ThingKind::Obstacle { .. }
                        | ThingKind::CustomObstacle { .. }
                        | ThingKind::Store { .. }
                )
            );
            if refuses {
                return Err(DropRefusal::Thing { thing: it.thing });
            }
        }
        Ok(Target::Room {
            region,
            room,
            x: nx,
            y: ny,
            top,
        })
    }

    /// A "when dropped here" custom space on top of the target that wants
    /// `thing`: casts its spell and says whether the thing is consumed (5.7).
    fn dropped_on_custom_space(
        &mut self,
        at: (u8, u8, u8, u8),
        top: &Option<RoomItem>,
        thing: ThingId,
        ev: &mut Vec<Event>,
    ) -> bool {
        let Some(it) = top else { return false };
        let Some(t) = self.thing(it.thing).cloned() else {
            return false;
        };
        let ThingKind::CustomSpace {
            placement_choice,
            trigger: CustomTrigger::WhenDroppedHere,
            action,
            destroy_needed_thing,
            ..
        } = t.kind
        else {
            return false;
        };
        let (needed, action) = resolve_custom_space(placement_choice, action, it.parameter);
        if needed != thing {
            return false;
        }
        let (region, room, x, y) = at;
        let stack = self
            .room(region, room)
            .and_then(|r| r.items.iter().position(|i| i.x == x && i.y == y));
        let me = Who::Player { player: self.current };
        self.run_spell(
            Spell {
                by: me,
                target: me,
                source: SpellSource::CustomSpace,
                thing: it.thing,
                action,
                message: self.message(&t.spell_message),
                at: Location::Room { region, room, x, y },
                stack,
            },
            ev,
        );
        if destroy_needed_thing {
            ev.push(Event::ThingUsedUp {
                player: self.current,
                thing,
            });
        }
        destroy_needed_thing
    }

    pub(crate) fn put_on_top(&mut self, region: u8, room: u8, x: u8, y: u8, thing: ThingId, parameter: u8) {
        if let Some(r) = self
            .region_mut(region)
            .and_then(|reg| reg.rooms.get_mut(room as usize))
        {
            // The list is stored top first (movement 5.3).
            r.items.insert(
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
    }

    /// A pile of `amount` gold on top of a square's stack (5.8).
    pub(crate) fn put_gold(&mut self, region: u8, room: u8, x: u8, y: u8, amount: u16) {
        self.put_on_top(region, room, x, y, GOLD, 0);
        if let Some(it) = self
            .region_mut(region)
            .and_then(|reg| reg.rooms.get_mut(room as usize))
            .and_then(|r| r.items.first_mut())
        {
            it.gold = Some(amount);
        }
    }

    /// The pieces in a gold pile: its size (0 is worth 0, as on the C64),
    /// else (a pile the data gave no size) its parameter, at least 1 (2.5).
    pub(crate) fn pile_size(item: &RoomItem) -> u16 {
        item.gold.unwrap_or((item.parameter as u16).max(1))
    }

    /// `Drop { thing, dir }` (section 5).
    pub(crate) fn drop_thing(&mut self, thing: ThingId, dir: Option<Dir>, ev: &mut Vec<Event>) {
        if thing == GOLD {
            self.drop_gold(1, dir, ev);
            return;
        }
        let p = self.current;
        let refuse = |reason| Event::DropRefused {
            player: p,
            thing,
            reason,
        };
        if !self.players[p].owns(thing) {
            ev.push(refuse(DropRefusal::NotCarried));
            return;
        }
        let Some(t) = self.thing(thing).cloned() else {
            ev.push(refuse(DropRefusal::NotCarried));
            return;
        };
        // 5.2: spells and owner-only weapons and armor stay with the owner.
        match t.kind {
            ThingKind::MagicSpell { .. } => {
                ev.push(refuse(DropRefusal::MagicSpell));
                return;
            }
            ThingKind::MissileWeapon { stats, .. }
            | ThingKind::MeleeWeapon { stats, .. }
            | ThingKind::Armor { stats, .. }
                if !stats.usable_by_anyone =>
            {
                ev.push(refuse(DropRefusal::OwnerOnly));
                return;
            }
            _ => {}
        }
        let target = match self.drop_target(dir) {
            Ok(t) => t,
            Err(reason) => {
                ev.push(refuse(reason));
                return;
            }
        };
        self.take_one(p, thing);
        let me = Who::Player { player: p };
        // Magic 2.3: it acts on the dropper unless it lands on a creature.
        let drop_spell = match t.kind {
            ThingKind::MagicItem {
                action,
                on_drop: true,
                ..
            } => {
                let (at, victim) = match &target {
                    Target::Map => (self.players[p].at, me),
                    Target::Room {
                        region, room, x, y, ..
                    } => {
                        let on = self
                            .others_in_room(*region, *room, Some(me))
                            .into_iter()
                            .find(|o| (o.x, o.y) == (*x, *y) && matches!(o.who, Who::Creature { .. }));
                        (
                            Location::Room {
                                region: *region,
                                room: *room,
                                x: *x,
                                y: *y,
                            },
                            on.map_or(me, |o| o.who),
                        )
                    }
                };
                Some(Spell {
                    by: me,
                    target: victim,
                    source: SpellSource::Dropped,
                    thing,
                    action,
                    message: self.message(&t.spell_message),
                    at,
                    stack: None,
                })
            }
            _ => None,
        };
        match target {
            // 5.3: lost for good.
            Target::Map => ev.push(Event::LostOnMap { player: p, thing }),
            Target::Room {
                region,
                room,
                x,
                y,
                top,
            } => {
                if self.dropped_on_custom_space((region, room, x, y), &top, thing, ev) {
                    return;
                }
                if matches!(
                    t.kind,
                    ThingKind::Treasure {
                        disappears_when_dropped: true,
                        ..
                    }
                ) {
                    ev.push(Event::VanishedWhenDropped { player: p, thing });
                } else {
                    self.put_on_top(region, room, x, y, thing, 1);
                    ev.push(Event::Dropped {
                        player: p,
                        thing,
                        at: Location::Room { region, room, x, y },
                    });
                }
            }
        }
        if let Some(s) = drop_spell {
            self.run_spell(s, ev);
        }
    }

    /// `DropGold { amount, dir }` (5.8).
    pub(crate) fn drop_gold(&mut self, amount: u16, dir: Option<Dir>, ev: &mut Vec<Event>) {
        let p = self.current;
        let refuse = |reason| Event::DropRefused {
            player: p,
            thing: GOLD,
            reason,
        };
        if amount == 0 || amount > PILE_LIMIT {
            ev.push(refuse(DropRefusal::BadAmount));
            return;
        }
        if self.players[p].gold < amount {
            ev.push(refuse(DropRefusal::NotCarried));
            return;
        }
        let target = match self.drop_target(dir) {
            Ok(t) => t,
            Err(reason) => {
                ev.push(refuse(reason));
                return;
            }
        };
        self.players[p].gold -= amount;
        match target {
            Target::Map => ev.push(Event::GoldDropped {
                player: p,
                amount,
                at: None,
            }),
            Target::Room {
                region,
                room,
                x,
                y,
                top,
            } => {
                if self.dropped_on_custom_space((region, room, x, y), &top, GOLD, ev) {
                    return;
                }
                let vanishes = matches!(
                    self.thing(GOLD).map(|t| &t.kind),
                    Some(ThingKind::Treasure {
                        disappears_when_dropped: true,
                        ..
                    })
                );
                if vanishes {
                    ev.push(Event::VanishedWhenDropped {
                        player: p,
                        thing: GOLD,
                    });
                } else {
                    self.put_gold(region, room, x, y, amount);
                    ev.push(Event::GoldDropped {
                        player: p,
                        amount,
                        at: Some(Location::Room { region, room, x, y }),
                    });
                }
            }
        }
    }
}
