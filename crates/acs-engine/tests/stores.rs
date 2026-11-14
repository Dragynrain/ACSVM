// SPDX-License-Identifier: GPL-3.0-or-later

//! Store rules (`docs/rules/stores.md`) on a synthetic adventure built byte
//! by byte with acs-core's fixture builder. No real adventure data.

#[path = "../../acs-core/tests/common/mod.rs"]
mod common;

use acs_core::layout::world_map as wm;
use acs_core::{decode, Adventure, Platform};
use acs_engine::inventory::GOLD;
use acs_engine::stores::{self, StoreVisit};
use acs_engine::{Command, Dir, Event, Game, Location, PlayerSetup, Rules, StoreRefusal};
use common::{Fixture, RegionSpec, RoomSpec};

const GEM: u16 = 1; // treasure, value 50; 3 in stock
const CROWN: u16 = 2; // treasure, value 5,100 (x20); 1 in stock
const DUST: u16 = 3; // treasure that disappears when dropped, value 30
const SWORD: u16 = 33; // melee weapon, value 10; 1 in stock
const WAND: u16 = 40; // magic item, value 1
const SPELL: u16 = 41; // magic spell
const STORE: u16 = 83;

/// One region, one room 10 x 7 (inside 1..8 x 1..5) with the store on
/// (4,3); the stock 3 gems, a crown and the sword.
fn adventure(platform: Platform) -> Adventure {
    let mut f = Fixture::new(platform);
    f.header("SHOP", "", "");
    f.thing(GEM as usize, "GEM", [0x01, 1, 50, 5]);
    f.thing(CROWN as usize, "CROWN", [0x81, 1, 255, 5]);
    f.thing(DUST as usize, "DUST", [0x11, 1, 30, 5]);
    f.thing(SWORD as usize, "SWORD", [0x04, 5, 10, 0]);
    f.extended(None, 0, [5, 0, 20 | 0x40]);
    f.thing(WAND as usize, "WAND", [0x02, 1, 1, 0]);
    f.extended(Some(12), 0, [10, 7, 3]);
    f.thing(SPELL as usize, "SPELL", [0x06, 5, 10, 1]);
    // Weightless, so a full purse doesn't slow the player (inventory 3).
    f.thing(GOLD as usize, "GOLD", [0x01, 0, 1, 9]);
    f.thing(STORE as usize, "STORE", [0x0C, 6, 0, 0]);
    f.thing(127, "EMPTY FLOOR", [0x00, 1, 0, 0]);
    let mut stock = [0u8; 14];
    stock[0] = 3 << 6 | 1 << 4; // things 1 (x3) and 2 (x1)
    stock[8] = 0x80; // thing 33
    f.region(RegionSpec {
        name: "SHOP",
        rooms: vec![RoomSpec {
            name: "SHOP",
            w: 10,
            h: 7,
            wall: 12,
            items: vec![(4, 3, STORE as u8, 0)],
            ..Default::default()
        }],
        store: stock,
        ..Default::default()
    });
    let w = f.l.world_map;
    f.put(w + wm::TERRAIN, &[1 | 1 << 6, 0]);
    f.put(w + wm::START_X, &[20, 20]);
    decode(&f.build()).expect("decodes").adventure
}

/// The player (Wisdom `wisdom`, `gold` gold, carrying `carry`) on (3,3),
/// west of the store.
fn start(p: Platform, wisdom: u8, gold: u16, carry: &[u16]) -> Game {
    let (mut g, _) = Game::new(
        adventure(p),
        1,
        Rules::default(),
        PlayerSetup {
            speed: 6,
            wisdom,
            gold,
            possessions: carry.to_vec(),
            ..Default::default()
        },
    );
    g.place(
        0,
        Location::Room {
            region: 1,
            room: 0,
            x: 3,
            y: 3,
        },
    );
    g
}

/// Bumps the store from the west.
fn open(g: &mut Game) -> Vec<Event> {
    g.apply(Command::Move { dir: Dir::E })
}

fn both(test: impl Fn(Platform)) {
    test(Platform::Pc);
    test(Platform::C64);
}

fn stock(g: &Game, thing: u16) -> u8 {
    stores::stocked(g.stock(1), thing)
}

const STORE_SQUARE: Location = Location::Room {
    region: 1,
    room: 0,
    x: 4,
    y: 3,
};

#[test]
fn bumping_the_store_steps_onto_it_and_leaving_leaves_one_move() {
    both(|p| {
        let mut g = start(p, 13, 500, &[GEM]);
        let moves = g.current_player().moves_left;
        let ev = open(&mut g);
        // Stores 3.2: an ordinary step, one move.
        assert!(matches!(ev[0], Event::Moved { to: STORE_SQUARE, .. }));
        assert!(ev.contains(&Event::StoreOpened {
            player: 0,
            region: 1,
            gold: 500
        }));
        assert_eq!(g.store_visit(), Some(StoreVisit { player: 0, region: 1 }));
        assert_eq!(g.current_player().moves_left, moves - 1);
        assert_eq!(g.current_player().at, STORE_SQUARE);
        // 3.7: no time passes inside.
        g.apply(Command::Sell { thing: GEM });
        assert_eq!(g.current_player().moves_left, moves - 1);
        // Nor does the real-time drain (movement 2.4): Idle stays inside.
        assert!(g.apply(Command::Idle).is_empty());
        assert_eq!(g.store_visit(), Some(StoreVisit { player: 0, region: 1 }));
        assert_eq!(g.current_player().moves_left, moves - 1);
        // 3.6, 3.7: out on the store's square with exactly one move.
        let ev = g.apply(Command::LeaveStore);
        assert_eq!(ev, [Event::StoreClosed { player: 0 }]);
        assert_eq!(g.store_visit(), None);
        assert_eq!(g.current_player().at, STORE_SQUARE);
        assert_eq!(g.current_player().moves_left, 1);
        // 3.8: any other command leaves first, then runs with that move.
        let ev = g.apply(Command::Move { dir: Dir::W });
        assert!(ev.iter().any(|e| matches!(e, Event::TurnStarted { .. })));
        open(&mut g);
        let ev = g.apply(Command::Wait);
        assert_eq!(ev[0], Event::StoreClosed { player: 0 });
        let ev = g.apply(Command::Buy { thing: GEM });
        assert!(ev.iter().any(|e| matches!(
            e,
            Event::StoreRefused {
                reason: StoreRefusal::NoStore,
                ..
            }
        )));
    });
}

#[test]
fn buying_costs_the_full_value_at_any_wisdom() {
    both(|p| {
        for wisdom in [0, 13, 31] {
            let mut g = start(p, wisdom, 500, &[]);
            open(&mut g);
            let ev = g.apply(Command::Buy { thing: GEM });
            assert!(ev.contains(&Event::Bought {
                player: 0,
                thing: GEM,
                price: 50,
                kept: true
            }));
            assert_eq!(g.current_player().gold, 450);
            assert_eq!(g.current_player().count(GEM), 1);
            assert_eq!(stock(&g, GEM), 2);
        }
    });
}

#[test]
fn purchases_are_refused_for_gold_stock_and_the_carry_limit() {
    both(|p| {
        // 5,100 with 5,000 gold: not enough.
        let mut g = start(p, 13, 5000, &[GEM, GEM, GEM]);
        open(&mut g);
        let refused = |ev: &[Event], why| {
            ev.iter()
                .any(|e| matches!(e, Event::StoreRefused { reason, .. } if *reason == why))
        };
        assert!(refused(
            &g.apply(Command::Buy { thing: CROWN }),
            StoreRefusal::NotEnoughGold
        ));
        assert!(refused(
            &g.apply(Command::Buy { thing: WAND }),
            StoreRefusal::NotStocked
        ));
        assert_eq!(g.current_player().gold, 5000);
        // Exactly enough gold buys.
        let mut g = start(p, 13, 50, &[]);
        open(&mut g);
        g.apply(Command::Buy { thing: GEM });
        assert_eq!(g.current_player().gold, 0);
        // Bought out: the last one goes from the stock.
        let mut g = start(p, 13, 6000, &[]);
        open(&mut g);
        g.apply(Command::Buy { thing: CROWN });
        assert_eq!((g.current_player().gold, stock(&g, CROWN)), (900, 0));
        assert!(refused(
            &g.apply(Command::Buy { thing: CROWN }),
            StoreRefusal::NotStocked
        ));
    });
}

#[test]
fn a_weapon_bought_is_not_readied() {
    both(|p| {
        let mut g = start(p, 13, 500, &[]);
        open(&mut g);
        let ev = g.apply(Command::Buy { thing: SWORD });
        assert!(!ev.iter().any(|e| matches!(e, Event::Readied { .. })));
        assert_eq!(g.current_player().weapon, None);
        assert_eq!(g.current_player().count(SWORD), 1);
        assert_eq!(g.current_player().gold, 490);
    });
}

#[test]
fn buying_at_the_limit_pays_and_loses_the_thing() {
    // Stores 4.3: three gems owned, one more bought.
    both(|p| {
        let mut g = start(p, 13, 500, &[GEM, GEM, GEM]);
        open(&mut g);
        let ev = g.apply(Command::Buy { thing: GEM });
        assert!(ev.contains(&Event::Bought {
            player: 0,
            thing: GEM,
            price: 50,
            kept: false
        }));
        assert_eq!(g.current_player().gold, 450);
        assert_eq!(stock(&g, GEM), 2);
        assert_eq!(g.current_player().count(GEM), 3);
    });
}

#[test]
fn selling_pays_the_offer_and_stocks_the_thing() {
    both(|p| {
        let mut g = start(p, 31, 500, &[WAND, GEM, DUST]);
        open(&mut g);
        // Stores 5.1: a third x (25 + 31) / 25.
        let ev = g.apply(Command::Sell { thing: GEM });
        assert!(ev.contains(&Event::Sold {
            player: 0,
            thing: GEM,
            offer: 35,
            paid: 35,
            stocked: false
        }));
        // Already 3 in stock, the limit: it doesn't join (see below).
        // A magic item of value 1: 0 gold, and it joins the stock.
        let ev = g.apply(Command::Sell { thing: WAND });
        assert!(ev.iter().any(|e| matches!(
            e,
            Event::Sold {
                thing: WAND,
                offer: 0,
                stocked: true,
                ..
            }
        )));
        assert_eq!(stock(&g, WAND), 1);
        // A treasure that disappears when dropped sells but is gone.
        let ev = g.apply(Command::Sell { thing: DUST });
        assert!(ev.iter().any(|e| matches!(
            e,
            Event::Sold {
                thing: DUST,
                offer: 22,
                stocked: false,
                ..
            }
        )));
        assert_eq!(stock(&g, DUST), 0);
        assert_eq!(g.current_player().gold, 557);
        assert!(g.current_player().possessions.is_empty());
    });
}

#[test]
fn spells_and_gold_dont_sell_and_the_purse_cap_takes_the_rest() {
    both(|p| {
        let mut g = start(p, 13, 24_998, &[SPELL, SWORD]);
        g.apply(Command::Ready { thing: SWORD });
        open(&mut g);
        for (thing, why) in [
            (SPELL, StoreRefusal::NotSellable),
            (GOLD, StoreRefusal::NotSellable),
        ] {
            let ev = g.apply(Command::Sell { thing });
            assert!(ev
                .iter()
                .any(|e| matches!(e, Event::StoreRefused { reason, .. } if *reason == why)));
        }
        assert!(g.apply(Command::Sell { thing: GEM }).iter().any(|e| matches!(
            e,
            Event::StoreRefused {
                reason: StoreRefusal::NotOwned,
                ..
            }
        )));
        // The held sword: offered 4, only 2 fit; it leaves the hand.
        let ev = g.apply(Command::Sell { thing: SWORD });
        assert!(ev.contains(&Event::Sold {
            player: 0,
            thing: SWORD,
            offer: 4,
            paid: 2,
            stocked: false
        }));
        assert_eq!(g.current_player().gold, 25_000);
        assert_eq!(g.current_player().weapon, None);
        // The stock already has its one sword (thing 33, limit 1).
        assert_eq!(stock(&g, SWORD), 1);
    });
}

#[test]
fn a_store_visit_end_to_end() {
    let mut g = start(Platform::C64, 13, 500, &[GEM]);
    open(&mut g);
    g.apply(Command::Sell { thing: GEM });
    g.apply(Command::Buy { thing: SWORD });
    g.apply(Command::LeaveStore);
    // A gem at Wisdom 13: 16 x 38 / 25 = 24.
    assert_eq!(g.current_player().gold, 500 + 24 - 10);
    assert_eq!(stock(&g, GEM), 3);
}
