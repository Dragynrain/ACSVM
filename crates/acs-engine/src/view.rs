// SPDX-License-Identifier: GPL-3.0-or-later

//! What a front end shows: read-only questions about a [`Game`], answered
//! by the rules, so a client draws the engine's answers instead of working
//! them out itself. Nothing here changes the game or rolls.

use crate::characters::{mark, Mark, Rating};
use crate::game::{Game, Player};
use crate::inventory;
use crate::magic::cast_chance;
use crate::stores::{buy_price, offered, sell_offer, value};
use crate::{Location, StoreRefusal};
use acs_core::model::{Creature, ThingId, ThingKind};
use serde::{Deserialize, Serialize};

/// One thing on the open store's shelves (stores 2.7, 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuyOffer {
    pub thing: ThingId,
    /// How many the shelf holds.
    pub count: u8,
    pub price: i32,
    /// Why `Command::Buy` would be refused, if it would.
    pub refusal: Option<StoreRefusal>,
    /// False when the buyer is at the thing's carry limit: the purchase
    /// goes ahead and the thing is lost (4.4).
    pub kept: bool,
}

/// One thing the player carries that the store would buy (stores 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SellOffer {
    pub thing: ThingId,
    /// How many the player has.
    pub owned: u16,
    /// What the store pays for one (5.1).
    pub offer: i32,
}

/// The open store as its visitor sees it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreOffers {
    pub player: usize,
    pub region: u8,
    /// The visitor's gold.
    pub gold: u16,
    /// What is for sale, in thing order; only things the store offers
    /// (2.7) and has in stock.
    pub buy: Vec<BuyOffer>,
    /// What the visitor could sell, in thing order.
    pub sell: Vec<SellOffer>,
}

impl Game {
    /// Whether `at` is a square a player can stand on: on the world map, or
    /// inside a room that exists and isn't deleted (for tool placements).
    pub fn is_square(&self, at: Location) -> bool {
        match at {
            Location::World { x, y } => self.on_map(x, y),
            Location::Room { region, room, x, y } => self
                .room(region, room)
                .is_some_and(|r| x < r.width && y < r.height),
        }
    }

    /// The open store's shelves and offers, or `None` when no store is open.
    pub fn store_offers(&self) -> Option<StoreOffers> {
        let visit = self.store_visit()?;
        let p = &self.players()[visit.player];
        let adv = self.adventure();
        let buy = self
            .stock(visit.region)
            .iter()
            .filter(|s| s.count > 0 && offered(adv, s.thing))
            .filter_map(|s| {
                let price = buy_price(adv, s.thing)?;
                let refusal = ((p.gold as i32) < price).then_some(StoreRefusal::NotEnoughGold);
                Some(BuyOffer {
                    thing: s.thing,
                    count: s.count,
                    price,
                    refusal,
                    kept: p.count(s.thing) < inventory::limit(s.thing),
                })
            })
            .collect();
        let mut goods = p.possessions.clone();
        goods.sort_unstable();
        goods.dedup();
        let sell = goods
            .into_iter()
            .filter_map(|thing| {
                let value = value(adv, thing)?;
                Some(SellOffer {
                    thing,
                    owned: p.count(thing),
                    offer: sell_offer(value, p.wisdom),
                })
            })
            .collect();
        Some(StoreOffers {
            player: visit.player,
            region: visit.region,
            gold: p.gold,
            buy,
            sell,
        })
    }

    /// The chance in percent that `player` casting the spell `thing` now
    /// works (magic 4.1), or `None` when `thing` is not a spell. A summons
    /// into a full region still fails whatever this says (3.2).
    pub fn cast_chance(&self, player: usize, thing: ThingId) -> Option<u8> {
        let p = self.players().get(player)?;
        match self
            .adventure()
            .things
            .get((thing as usize).wrapping_sub(1))?
            .kind
        {
            ThingKind::MagicSpell { power: cost, .. } => Some(cast_chance(p.power, cost)),
            _ => None,
        }
    }

    /// The live creatures in room `room` of `region`, with their slots.
    pub fn residents_in(&self, region: u8, room: u8) -> impl Iterator<Item = (usize, &Creature)> {
        (0..self.slots(region))
            .filter_map(move |s| self.creature(region, s).map(|c| (s, c)))
            .filter(move |(_, c)| c.room == room)
    }
}

impl Player {
    /// A rating as the profile shows it (characters 4).
    pub fn rating(&self, rating: Rating) -> u16 {
        use Rating::*;
        (match rating {
            Constitution => self.constitution,
            LifeForce => self.life_force,
            Wisdom => self.wisdom,
            Power => self.power,
            Strength => self.strength,
            Size => self.size,
            Dexterity => self.dexterity,
            ArmorSkill => self.armor_skill,
            Missile => self.missile,
            Dodge => self.dodge,
            Melee => self.melee,
            Parry => self.parry,
            Speed => self.speed,
            Gold => return self.gold,
        }) as u16
    }

    /// The profile's mark beside a rating (characters 4.1).
    pub fn mark(&self, rating: Rating) -> Option<Mark> {
        mark(rating, self.rating(rating))
    }
}

/// How often a creature's smartness check passes (magic 6, the check
/// before a creature casts): what a client labels instead of reading
/// Wisdom against the thresholds itself (client 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wits {
    /// Wisdom over 14: always.
    Smart,
    /// Wisdom under 6: never.
    Dumb,
    /// 6 to 14: (Wisdom - 5) x 10 percent of the time.
    Sometimes,
}

/// The [`Wits`] of a creature with this Wisdom.
pub fn wits(wisdom: u8) -> Wits {
    match wisdom {
        w if w > 14 => Wits::Smart,
        w if w < 6 => Wits::Dumb,
        _ => Wits::Sometimes,
    }
}
