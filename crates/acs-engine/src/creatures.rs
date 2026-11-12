// SPDX-License-Identifier: GPL-3.0-or-later

//! Creatures outside combat: who acts when, random room creatures, and the
//! personality rules the AI uses. Spec: `docs/rules/creatures.md` (section
//! numbers in the comments refer to it). The per-creature turn is in
//! `ai.rs`.

use crate::game::Game;
use crate::inventory::{self, slowed_speed, GOLD};
use crate::{Dir, Event, Location, RemovalReason, Who};
use acs_core::model::ThingId;
use acs_core::model::{Adventure, Alignment, Creature, ThingKind};

/// Fighter or slinker (4.2).
pub fn is_fighter(c: &Creature) -> bool {
    c.strategy.aggressive && (c.strategy.brave || c.life_force >= c.constitution)
}

pub fn is_wounded(c: &Creature) -> bool {
    c.life_force < c.constitution
}

/// What the AI needs to know about another individual.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Other {
    pub who: Who,
    /// `None` for players.
    pub alignment: Option<Alignment>,
    pub aggressive: bool,
    pub x: u8,
    pub y: u8,
}

/// Whether `c` treats `o` as a foe (7.4).
pub fn is_foe(c: &Creature, o: &Other) -> bool {
    match c.strategy.alignment {
        Alignment::Enemy => matches!(o.alignment, None | Some(Alignment::Friend)),
        Alignment::Friend => match o.alignment {
            Some(Alignment::Enemy) => true,
            Some(Alignment::Neutral | Alignment::Thief) => o.aggressive,
            _ => false,
        },
        Alignment::Neutral | Alignment::Thief => true,
    }
}

/// How many of `thing` a creature owns (gold pieces for thing 81).
pub fn count(c: &Creature, thing: ThingId) -> u16 {
    if thing == GOLD {
        c.wealth
    } else {
        c.possessions
            .iter()
            .filter(|p| p.thing == thing)
            .map(|p| p.count as u16)
            .sum()
    }
}

/// Total weight a creature carries, gold included (inventory 3.1).
pub fn load(adv: &Adventure, c: &Creature) -> i32 {
    let items: i32 = c
        .possessions
        .iter()
        .map(|p| inventory::weight(adv, p.thing) * p.count as i32)
        .sum();
    items + c.wealth as i32 * inventory::weight(adv, GOLD)
}

/// Inventory 3.2 with the creature's traits.
pub fn capacity(c: &Creature) -> i32 {
    let wound = (c.constitution as i32 - c.life_force as i32) / 2;
    (c.strength as i32 - wound) * 100
}

/// Where a creature's bar starts (2.5).
pub fn effective_speed(adv: &Adventure, c: &Creature) -> u8 {
    slowed_speed(c.speed, load(adv, c), capacity(c))
}

/// Whether a creature picks up `kind` (5.2), ignoring limits and load.
pub fn picks_up(c: &Creature, thing: ThingId, kind: &ThingKind) -> bool {
    match kind {
        ThingKind::Treasure { .. } if thing == GOLD => {
            !(c.strategy.alignment == Alignment::Friend
                || (c.strategy.alignment == Alignment::Neutral && !is_fighter(c)))
        }
        ThingKind::Treasure { .. } => c.strategy.alignment == Alignment::Thief,
        _ => false,
    }
}

impl Game {
    /// The live creatures and players standing in a room, other than `not`.
    pub(crate) fn others_in_room(&self, region: u8, room: u8, not: Option<Who>) -> Vec<Other> {
        let mut out = Vec::new();
        for (i, p) in self.players.iter().enumerate() {
            if let Location::Room {
                region: r,
                room: m,
                x,
                y,
            } = p.at
            {
                let who = Who::Player { player: i };
                if p.alive && r == region && m == room && Some(who) != not {
                    out.push(Other {
                        who,
                        alignment: None,
                        aggressive: false,
                        x,
                        y,
                    });
                }
            }
        }
        if let Some(reg) = self.region(region) {
            for (slot, c) in reg.residents.iter().enumerate() {
                let who = Who::Creature { region, slot };
                if c.in_use && c.room == room && Some(who) != not {
                    out.push(Other {
                        who,
                        alignment: Some(c.strategy.alignment),
                        aggressive: c.strategy.aggressive,
                        x: c.x,
                        y: c.y,
                    });
                }
            }
        }
        out
    }

    /// Whether any individual stands on the square.
    pub(crate) fn occupied(&self, region: u8, room: u8, x: u8, y: u8) -> bool {
        self.others_in_room(region, room, None)
            .iter()
            .any(|o| o.x == x && o.y == y)
    }

    fn player_in_room(&self, region: u8, room: u8) -> bool {
        self.players
            .iter()
            .any(|p| p.alive && p.at.room() == Some((region, room)))
    }

    /// 2.6: an aggressive creature that is not a Friend stands on one of
    /// the 4 squares beside the player (not diagonally).
    pub(crate) fn hindered(&self, p: usize) -> bool {
        let Location::Room { region, room, x, y } = self.players[p].at else {
            return false;
        };
        let Some(reg) = self.region(region) else {
            return false;
        };
        reg.residents.iter().any(|c| {
            c.in_use
                && c.room == room
                && c.strategy.aggressive
                && c.strategy.alignment != Alignment::Friend
                && (c.x as i16 - x as i16).abs() + (c.y as i16 - y as i16).abs() == 1
        })
    }

    /// 2.4: at the end of a round, random creatures in rooms no player
    /// occupies are removed.
    pub(crate) fn left_behind(&mut self, ev: &mut Vec<Event>) {
        for (region, slot) in self.random.clone() {
            let Some(c) = self.creature(region, slot) else {
                continue;
            };
            if !self.player_in_room(region, c.room) {
                self.remove_creature(region, slot, RemovalReason::LeftBehind, ev);
            }
        }
    }

    /// Frees a slot; what the creature carried goes with it.
    pub(crate) fn remove_creature(
        &mut self,
        region: u8,
        slot: usize,
        reason: RemovalReason,
        ev: &mut Vec<Event>,
    ) {
        if let Some(c) = self.creature_mut(region, slot) {
            c.in_use = false;
            c.possessions.clear();
            c.wealth = 0;
        }
        self.random.retain(|&r| r != (region, slot));
        self.forget_vitals(Who::Creature { region, slot });
        ev.push(Event::CreatureRemoved { region, slot, reason });
    }

    /// 3.2, 3.3: copies random creature `which` onto a uniformly chosen
    /// empty inside square, in the lowest free slot.
    pub(crate) fn place_random(&mut self, region: u8, room: u8, which: u8) -> Option<(usize, Location)> {
        let r = self.room(region, room)?;
        let (w, h) = (r.width, r.height);
        let mut free = Vec::new();
        for y in 1..h.saturating_sub(1) {
            for x in 1..w.saturating_sub(1) {
                let has_item = r.items.iter().any(|it| it.x == x && it.y == y);
                if !has_item && !self.occupied(region, room, x, y) {
                    free.push((x, y));
                }
            }
        }
        if free.is_empty() {
            return None;
        }
        let reg = self.region(region)?;
        let slot = reg.residents.iter().position(|c| !c.in_use)?;
        let mut c = reg.random_creatures.get(which as usize)?.clone();
        let (x, y) = free[self.rng.below(free.len() as u32) as usize];
        c.in_use = true;
        c.number = slot as u16 + 1;
        c.region = region;
        c.room = room;
        c.x = x;
        c.y = y;
        let reg = self.region_mut(region)?;
        reg.residents[slot] = c;
        self.random.push((region, slot));
        self.forget_vitals(Who::Creature { region, slot });
        Some((slot, Location::Room { region, room, x, y }))
    }

    /// The square next to (x, y) in `dir`, if inside the room's bounds.
    pub(crate) fn neighbour(&self, region: u8, room: u8, x: u8, y: u8, dir: Dir) -> Option<(u8, u8)> {
        let r = self.room(region, room)?;
        let (dx, dy) = dir.delta();
        let (nx, ny) = (x as i16 + dx as i16, y as i16 + dy as i16);
        ((0..r.width as i16).contains(&nx) && (0..r.height as i16).contains(&ny))
            .then_some((nx as u8, ny as u8))
    }
}
