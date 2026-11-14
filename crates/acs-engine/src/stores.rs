// SPDX-License-Identifier: GPL-3.0-or-later

//! Stores: opening one, buying, selling back, leaving. Spec:
//! `docs/rules/stores.md` (section numbers in the comments refer to it).
//! A store is opened by bumping its square (`step.rs`); while it is open
//! the player buys and sells with `Command::Buy`, `Command::Sell` and
//! leaves with `Command::LeaveStore`. How the store looks is the front
//! end's (3.2).

use crate::game::Game;
use crate::inventory::{self, GOLD, GOLD_LIMIT};
use crate::{Event, Slot, StoreRefusal};
use acs_core::model::{Adventure, Possession, ThingId, ThingKind};
use serde::{Deserialize, Serialize};

/// The most the store's cut on a sale comes to (5.1).
pub const CUT_MAX: i32 = 1334;
/// The most of a third of the value the Wisdom bonus counts (5.1).
pub const THIRD_MAX: i32 = 666;

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

    /// 3.4: leaving puts the player back where it bumped the store from,
    /// its turn going on.
    pub(crate) fn leave_store(&mut self, ev: &mut Vec<Event>) {
        if let Some(v) = self.store.take() {
            ev.push(Event::StoreClosed { player: v.player });
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
        let p = &self.players[player];
        // 4.3: too little gold; at the carry limit (inventory 2.2).
        if (p.gold as i32) < price {
            ev.push(refuse(StoreRefusal::NotEnoughGold));
            return;
        }
        if p.count(thing) >= inventory::limit(thing) {
            ev.push(refuse(StoreRefusal::CarryLimit));
            return;
        }
        if let Some(r) = self.region_mut(region) {
            if let Some(i) = r.store.iter().position(|s| s.thing == thing) {
                r.store[i].count -= 1;
                if r.store[i].count == 0 {
                    r.store.remove(i);
                }
            }
        }
        let p = &mut self.players[player];
        p.gold -= price as u16;
        p.possessions.push(thing);
        ev.push(Event::Bought { player, thing, price });
        // 4.4: as if picked up: a weapon is readied (inventory 2.4); a
        // magic item set to act when picked up does not act.
        if matches!(
            self.thing(thing).map(|t| &t.kind),
            Some(ThingKind::MeleeWeapon { .. } | ThingKind::MissileWeapon { .. })
        ) {
            self.players[player].weapon = Some(thing);
            ev.push(Event::Readied {
                player,
                thing,
                slot: Slot::Weapon,
            });
        }
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
        // 1..80), unless it is a treasure that disappears when dropped.
        let disappears = matches!(
            self.thing(thing).map(|t| &t.kind),
            Some(ThingKind::Treasure {
                disappears_when_dropped: true,
                ..
            })
        );
        let mut joined = false;
        if !disappears && (1..=80).contains(&thing) {
            if let Some(r) = self.region_mut(region) {
                match r.store.iter_mut().find(|s| s.thing == thing) {
                    Some(s) if (s.count as u16) < inventory::limit(thing) => {
                        s.count += 1;
                        joined = true;
                    }
                    Some(_) => {}
                    None => {
                        r.store.push(Possession { thing, count: 1 });
                        r.store.sort_by_key(|s| s.thing);
                        joined = true;
                    }
                }
            }
        }
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
