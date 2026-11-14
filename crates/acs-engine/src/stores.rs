// SPDX-License-Identifier: GPL-3.0-or-later

//! Stores: opening one, buying, selling back, leaving. Spec:
//! `docs/rules/stores.md` (section numbers in the comments refer to it).
//! A store is opened by stepping onto its square (`step.rs`); while it is
//! open the player buys and sells with `Command::Buy`, `Command::Sell` and
//! leaves with `Command::LeaveStore`. Walking about the store screen is the
//! front end's, from the fixed layout here (3.3, 3.8).

use crate::game::Game;
use crate::inventory::{self, GOLD, GOLD_LIMIT};
use crate::{Event, StoreRefusal};
use acs_core::model::{Adventure, Possession, ThingId, ThingKind};
use serde::{Deserialize, Serialize};

/// The most the store's cut on a sale comes to (5.1).
pub const CUT_MAX: i32 = 1334;
/// The most of a third of the value the Wisdom bonus counts (5.1).
pub const THIRD_MAX: i32 = 666;

/// The store screen's size in squares (3.3).
pub const SCREEN_W: u8 = 15;
pub const SCREEN_H: u8 = 10;
/// Where the player appears on the store screen (3.3).
pub const ENTRY: (u8, u8) = (7, 5);

/// The store screen, row by row (3.3): a thing's shelf place (1..80),
/// `0` floor, `EXIT` a way out.
const EXIT: u8 = 255;
#[rustfmt::skip]
const LAYOUT: [[u8; SCREEN_W as usize]; SCREEN_H as usize] = [
    [ 0, 32,  0,  0, 30,  0,  0, 76,  0,  0, 78,  0,  0, 80,  0],
    [74,  0, 26, 21,  0, 16, 11,  0, 45, 50,  0, 55, 60,  0, 67],
    [73,  0, 25, 20,  0, 15, 10,  0, 44, 49,  0, 54, 59,  0, 66],
    [72,  0, 24, 19,  0, 14,  9,  0, 43, 48,  0, 53, 58,  0, 65],
    [EXIT, 0, 8,  6,  0,  4,  2,  0, 34, 36,  0, 38, 40,  0, EXIT],
    [71,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0, 64],
    [68,  0,  7,  5,  0,  3,  1,  0, 33, 35,  0, 37, 39,  0, 61],
    [69,  0, 27, 22,  0, 17, 12,  0, 41, 46,  0, 51, 56,  0, 62],
    [70,  0, 28, 23,  0, 18, 13,  0, 42, 47,  0, 52, 57,  0, 63],
    [ 0, 31,  0,  0, 29,  0,  0, 75,  0,  0, 77,  0,  0, 79,  0],
];

/// What a square of the store screen is (3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StoreSquare {
    /// The player can stand here.
    Floor,
    /// The shelf place of one thing.
    Shelf(ThingId),
    /// A way out (3.6).
    Exit,
}

/// The store screen's square `x`, `y` (3.3); `None` off the screen, where
/// a push says the item is out of stock (3.5).
pub fn square(x: u8, y: u8) -> Option<StoreSquare> {
    let b = *LAYOUT.get(y as usize)?.get(x as usize)?;
    Some(match b {
        0 => StoreSquare::Floor,
        EXIT => StoreSquare::Exit,
        t => StoreSquare::Shelf(t as ThingId),
    })
}

/// Where `thing`'s shelf place is on the store screen (3.3); things above
/// 80 have none.
pub fn shelf_place(thing: ThingId) -> Option<(u8, u8)> {
    (0..SCREEN_H)
        .flat_map(|y| (0..SCREEN_W).map(move |x| (x, y)))
        .find(|&(x, y)| square(x, y) == Some(StoreSquare::Shelf(thing)))
}

/// Whether `thing` can be in a store's stock: things 1..80 (2.3).
fn stockable(thing: ThingId) -> bool {
    (1..=80).contains(&thing)
}

/// A store visit in progress: whose, and which region's stock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreVisit {
    pub player: usize,
    pub region: u8,
}

/// A thing's value in gold (its multiplier applied), for the things a
/// store deals in: treasures, magic items, weapons and armor (2.1, 5.2).
pub fn value(adv: &Adventure, thing: ThingId) -> Option<i32> {
    if thing == GOLD {
        return None;
    }
    match adv.things.get((thing as usize).wrapping_sub(1))?.kind {
        ThingKind::Treasure { carry, .. }
        | ThingKind::MagicItem { carry, .. }
        | ThingKind::MissileWeapon { carry, .. }
        | ThingKind::MeleeWeapon { carry, .. }
        | ThingKind::Armor { carry, .. } => Some(carry.value.max(0)),
        _ => None,
    }
}

/// 4.1, 4.2: the buying price is the full value, whatever the Wisdom.
pub fn buy_price(adv: &Adventure, thing: ThingId) -> Option<i32> {
    value(adv, thing)
}

/// 5.1: what a store offers for a thing of value `value` from a player of
/// Wisdom `wisdom`: the value less the store's cut (the smaller of the
/// value less a third of it, and `CUT_MAX`), plus the smaller of a third
/// and `THIRD_MAX`, times Wisdom, over 25. Thirds and the bonus round down.
pub fn sell_offer(value: i32, wisdom: u8) -> i32 {
    let value = value.max(0);
    let third = value / 3;
    value - (value - third).min(CUT_MAX) + third.min(THIRD_MAX) * wisdom as i32 / 25
}

/// How many of `thing` a region's stock holds.
pub fn stocked(stock: &[Possession], thing: ThingId) -> u8 {
    stock.iter().find(|p| p.thing == thing).map_or(0, |p| p.count)
}

impl Game {
    /// The store visit in progress, if any.
    pub fn store_visit(&self) -> Option<StoreVisit> {
        self.store
    }

    /// A region's store stock as play has changed it (2.3).
    pub fn stock(&self, region: u8) -> &[Possession] {
        self.region(region).map_or(&[], |r| r.store.as_slice())
    }

    /// 3.1, 3.2: bumping a store square opens the region's store.
    pub(crate) fn open_store(&mut self, player: usize, region: u8, ev: &mut Vec<Event>) {
        self.store = Some(StoreVisit { player, region });
        ev.push(Event::StoreOpened {
            player,
            region,
            gold: self.players[player].gold,
        });
    }

    /// 3.6, 3.7: leaving puts the player on the store's square (where the
    /// bump put it) with exactly one move left in its turn.
    pub(crate) fn leave_store(&mut self, ev: &mut Vec<Event>) {
        if let Some(v) = self.store.take() {
            self.players[v.player].moves_left = 1;
            ev.push(Event::StoreClosed { player: v.player });
        }
    }

    /// One of `thing` joins `region`'s stock within its limit (2.5, 2.6);
    /// never on the world map (2.7) or for things above 80. Returns whether
    /// it joined.
    fn add_to_stock(&mut self, region: u8, thing: ThingId) -> bool {
        if region == 0 || !stockable(thing) {
            return false;
        }
        let Some(r) = self.region_mut(region) else {
            return false;
        };
        match r.store.iter_mut().find(|s| s.thing == thing) {
            Some(s) if (s.count as u16) < inventory::limit(thing) => {
                s.count += 1;
                true
            }
            Some(_) => false,
            None => {
                r.store.push(Possession { thing, count: 1 });
                r.store.sort_by_key(|s| s.thing);
                true
            }
        }
    }

    /// 2.5: a weapon or armor broken or used up goes back into the stock of
    /// the region where it happened.
    pub(crate) fn restock(&mut self, region: u8, thing: ThingId, ev: &mut Vec<Event>) {
        if self.add_to_stock(region, thing) {
            ev.push(Event::Restocked { region, thing });
        }
    }

    /// 4: buying one of `thing` from the open store.
    pub(crate) fn buy(&mut self, thing: ThingId, ev: &mut Vec<Event>) {
        let Some(StoreVisit { player, region }) = self.store else {
            ev.push(Event::StoreRefused {
                player: self.current,
                thing,
                reason: StoreRefusal::NoStore,
            });
            return;
        };
        let refuse = |reason| Event::StoreRefused {
            player,
            thing,
            reason,
        };
        let Some(price) = buy_price(&self.adv, thing) else {
            ev.push(refuse(StoreRefusal::NotStocked));
            return;
        };
        if stocked(self.stock(region), thing) == 0 {
            ev.push(refuse(StoreRefusal::NotStocked));
            return;
        }
        // 4.3: gold is the only check.
        if (self.players[player].gold as i32) < price {
            ev.push(refuse(StoreRefusal::NotEnoughGold));
            return;
        }
        // 2.7: on the world map the stock never changes.
        if region != 0 {
            if let Some(r) = self.region_mut(region) {
                if let Some(i) = r.store.iter().position(|s| s.thing == thing) {
                    r.store[i].count -= 1;
                    if r.store[i].count == 0 {
                        r.store.remove(i);
                    }
                }
            }
        }
        // 4.3, 4.4: one more owned, unless at the limit, when it is lost;
        // nothing is readied and nothing acts.
        let p = &mut self.players[player];
        p.gold -= price as u16;
        let kept = p.count(thing) < inventory::limit(thing);
        if kept {
            p.possessions.push(thing);
        }
        ev.push(Event::Bought {
            player,
            thing,
            price,
            kept,
        });
    }

    /// 5: selling one of `thing` to the open store.
    pub(crate) fn sell(&mut self, thing: ThingId, ev: &mut Vec<Event>) {
        let Some(StoreVisit { player, region }) = self.store else {
            ev.push(Event::StoreRefused {
                player: self.current,
                thing,
                reason: StoreRefusal::NoStore,
            });
            return;
        };
        let refuse = |reason| Event::StoreRefused {
            player,
            thing,
            reason,
        };
        // 5.2: treasures, magic items, weapons and armor; never spells or
        // gold.
        let Some(value) = value(&self.adv, thing) else {
            ev.push(refuse(StoreRefusal::NotSellable));
            return;
        };
        if self.players[player].count(thing) == 0 {
            ev.push(refuse(StoreRefusal::NotOwned));
            return;
        }
        let offer = sell_offer(value, self.players[player].wisdom);
        // Leaving the hand or body as a drop would (inventory 4.4).
        self.take_one(player, thing);
        // 5.5: never refused; what passes the purse cap is lost.
        let p = &mut self.players[player];
        let before = p.gold;
        p.gold = (p.gold as i32 + offer).min(GOLD_LIMIT as i32) as u16;
        let paid = (p.gold - before) as i32;
        // 5.3, 5.4: it joins the stock within the thing's limit (things
        // 1..80, not on the map), unless it is a treasure that disappears
        // when dropped.
        let disappears = matches!(
            self.thing(thing).map(|t| &t.kind),
            Some(ThingKind::Treasure {
                disappears_when_dropped: true,
                ..
            })
        );
        let joined = !disappears && self.add_to_stock(region, thing);
        ev.push(Event::Sold {
            player,
            thing,
            offer,
            paid,
            stocked: joined,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_layout() {
        // 3.3: every thing 1..80 has exactly one place; the captured ones.
        for t in 1..=80 {
            assert!(shelf_place(t).is_some(), "{t}");
        }
        assert_eq!(shelf_place(81), None);
        assert_eq!(shelf_place(1), Some((6, 6)));
        assert_eq!(shelf_place(2), Some((6, 4)));
        assert_eq!(shelf_place(3), Some((5, 6)));
        assert_eq!(shelf_place(13), Some((6, 8)));
        assert_eq!(shelf_place(14), Some((5, 3)));
        assert_eq!(square(0, 4), Some(StoreSquare::Exit));
        assert_eq!(square(14, 4), Some(StoreSquare::Exit));
        assert_eq!(square(ENTRY.0, ENTRY.1), Some(StoreSquare::Floor));
        assert_eq!(square(6, 0), Some(StoreSquare::Floor));
        assert_eq!(square(15, 0), None);
        let places = (0..SCREEN_H)
            .flat_map(|y| (0..SCREEN_W).map(move |x| square(x, y)))
            .filter(|s| matches!(s, Some(StoreSquare::Shelf(_))))
            .count();
        assert_eq!(places, 80);
    }

    #[test]
    fn the_captured_offers() {
        // capture.md 10.17: a third at Wisdom 0, x (25 + W) / 25.
        assert_eq!(sell_offer(100, 13), 50);
        assert_eq!(sell_offer(40, 13), 19);
        for v in [99, 100, 101] {
            assert_eq!(sell_offer(v, 0), 33);
            assert_eq!(sell_offer(v, 31), 73);
        }
        assert_eq!(sell_offer(1, 31), 0);
        assert_eq!(sell_offer(10, 13), 4);
        assert_eq!(sell_offer(255, 0), 85);
        assert_eq!(sell_offer(2000, 31), 1491);
        // Above 2,000 the cut stays at 1,334, the bonus at 666 x W / 25.
        assert_eq!(sell_offer(2020, 0), 686);
        assert_eq!(sell_offer(2500, 12), 1485);
        assert_eq!(sell_offer(5100, 0), 3766);
        assert_eq!(sell_offer(5100, 13), 4112);
        assert_eq!(sell_offer(5100, 31), 4591);
        assert_eq!(sell_offer(0, 31), 0);
    }
}
