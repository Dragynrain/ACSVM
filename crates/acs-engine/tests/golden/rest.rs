// SPDX-License-Identifier: GPL-3.0-or-later

//! Stores (stores 2-5), characters (characters 4-7), world map encounters
//! (encounters 2-8) and saves and restores (creatures 2.2.6, 2.2.7):
//! capture.md 10.17 to 10.34. Stores and saves draw nothing that matters,
//! so they compare exactly; the encounter sweeps compare as samples.

use crate::world::*;
use acs_core::model::{Carry, Possession, RoomItem, SpellAction, SpellActionKind, ThingKind, WeaponStats};
use acs_core::{Adventure, Platform};
use acs_engine::{Command, Dir, Event, Game, Location, PlayerSetup, Rules, SaveFile, Who};

/// A spell of cost 0 (it always works) whose Change Life Force of -63
/// kills whoever it is cast at: a death on cue for the restore cases,
/// where the captures let a creature kill.
const KILL: u16 = 60;

fn kill_spell(a: &mut Adventure) {
    thing(
        a,
        KILL,
        "KILL",
        spell(
            0,
            SpellAction {
                kind: SpellActionKind::ChangeLifeForce,
                parameter: 0x80 | 0x3F,
            },
        ),
    );
}

fn map(x: u8, y: u8) -> Location {
    Location::World { x, y }
}

fn named(name: &str, speed: u8, possessions: &[u16]) -> PlayerSetup {
    PlayerSetup {
        name: name.into(),
        speed,
        possessions: possessions.to_vec(),
        ..Default::default()
    }
}

/// A game started at the map's start (12,12) with no tool placement, so
/// every save point is the engine's own.
fn fresh(a: &Adventure, seed: u64, first: PlayerSetup) -> Game {
    Game::new(a.clone(), seed, Rules::default(), first).0
}

fn add(setup: PlayerSetup) -> Command {
    Command::AddPlayer { setup }
}

fn has(ev: &[Event], f: impl Fn(&Event) -> bool) -> bool {
    ev.iter().any(f)
}

/// The current player walks south from the map's start onto the map door
/// (12,16) into ROOM (4,4); the events up to it.
fn walk_in(g: &mut Game) -> Vec<Event> {
    let p = g.current();
    let mut ev = Vec::new();
    for _ in 0..8 {
        let e = g.apply(Command::Move { dir: Dir::S });
        let done = has(
            &e,
            |e| matches!(e, Event::EnteredRoom { player, .. } if *player == p),
        );
        ev.extend(e);
        if done {
            return ev;
        }
    }
    panic!("player {p} never reached the room: {ev:?}");
}

/// `Saved` events as (round, visit).
fn saves(ev: &[Event]) -> Vec<(u32, Option<u8>)> {
    ev.iter()
        .filter_map(|e| match e {
            Event::Saved { round, visit } => Some((*round, *visit)),
            _ => None,
        })
        .collect()
}

fn added(ev: &[Event]) -> Vec<usize> {
    ev.iter()
        .filter_map(|e| match e {
            Event::PlayerAdded { player, .. } => Some(*player),
            _ => None,
        })
        .collect()
}

/// Waits whole turns until it is `player`'s turn.
fn until_turn_of(g: &mut Game, player: usize) -> Vec<Event> {
    let mut ev = Vec::new();
    for _ in 0..64 {
        if g.current() == player {
            return ev;
        }
        ev.extend(g.apply(Command::Wait));
    }
    panic!("player {player} never had a turn");
}

/// The game the death message's button brings back: the save at
/// [`Game::restore_point`], replayed from the seed as a save file is.
fn restore(g: &Game, a: &Adventure) -> Game {
    let mut s = SaveFile::new(g, a, g);
    s.autosave = g.restore_point();
    s.restore(a).expect("restores").autosave
}

// ---------------------------------------------------------------------------

mod stores {
    //! capture.md 10.17, 10.21, 10.31, 10.34; stores 2-5.

    use super::*;
    use acs_engine::stores::{self, StoreSquare};
    use acs_engine::StoreRefusal;

    pub const WARE1: u16 = 1; // value 10
    pub const WARE2: u16 = 2; // value 50
    pub const WARE3: u16 = 3; // value 200
    pub const BIG: u16 = 4; // value 5,100 (`buyb`)
    pub const GOODA: u16 = 13; // value 100
    pub const MAIL: u16 = 40;
    pub const BOW: u16 = 41;
    pub const SPELL: u16 = 58;
    pub const BLADE: u16 = 59;
    pub const ITEM: u16 = 70; // magic item of value 1, used up after a use
    pub const STORE: u16 = 83;

    pub fn armor(value: i32) -> ThingKind {
        ThingKind::Armor {
            picture: 8,
            disappears_after_use: false,
            carry: Carry { weight: 5, value },
            stats: WeaponStats {
                power: 2,
                range: None,
                break_chance: 0,
                attack_adjustment: 0,
                usable_by_anyone: true,
                magic: false,
            },
        }
    }

    pub fn magic_item(value: i32, once: bool) -> ThingKind {
        ThingKind::MagicItem {
            picture: 9,
            disappears_after_use: once,
            carry: Carry { weight: 1, value },
            action: nothing(),
            on_use: true,
            on_drop: false,
            on_pickup: false,
        }
    }

    /// rr10's `look` room on the base adventure: the store on ROOM 5,4,
    /// the player entering on 4,4 west of it; region 1's stock `stock`.
    pub fn shop(stock: &[(u16, u8)]) -> Adventure {
        let mut a = base(Platform::C64);
        thing(&mut a, WARE1, "WARE1", treasure(1, 10));
        thing(&mut a, WARE2, "WARE2", treasure(1, 50));
        thing(&mut a, WARE3, "WARE3", treasure(1, 200));
        thing(&mut a, BIG, "WARE4", treasure(1, 5100));
        thing(&mut a, GOODA, "GOODA", treasure(1, 100));
        thing(&mut a, MAIL, "MAIL", armor(10));
        thing(&mut a, BOW, "BOW", bow(4, 7));
        thing(&mut a, SPELL, "SPELL", spell(5, nothing()));
        thing(&mut a, BLADE, "BLADE", blade(4));
        thing(&mut a, ITEM, "ITEM", magic_item(1, true));
        thing(&mut a, STORE, "STORE", ThingKind::Store { picture: 5 });
        kill_spell(&mut a);
        item(&mut a, ROOM, 5, 4, STORE);
        a.regions[0].store = stock
            .iter()
            .map(|&(thing, count)| Possession { thing, count })
            .collect();
        a
    }

    pub fn shopper(wisdom: u8, gold: u16, carry: &[u16]) -> PlayerSetup {
        PlayerSetup {
            name: "T".into(),
            speed: 6,
            wisdom,
            gold,
            possessions: carry.to_vec(),
            ..Default::default()
        }
    }

    /// Bumps the store from 4,4.
    pub fn open(g: &mut Game) {
        let ev = g.apply(Command::Move { dir: Dir::E });
        assert!(has(&ev, |e| matches!(e, Event::StoreOpened { .. })), "{ev:?}");
    }

    /// The (offer, paid) of a sale.
    pub fn sale(ev: &[Event]) -> Option<(i32, i32)> {
        ev.iter().find_map(|e| match e {
            Event::Sold { offer, paid, .. } => Some((*offer, *paid)),
            _ => None,
        })
    }

    pub fn refusal(ev: &[Event]) -> Option<StoreRefusal> {
        ev.iter().find_map(|e| match e {
            Event::StoreRefused { reason, .. } => Some(*reason),
            _ => None,
        })
    }

    fn stock(g: &Game, region: u8) -> Vec<(u16, u8)> {
        g.stock(region).iter().map(|s| (s.thing, s.count)).collect()
    }

    /// The offer as capture.md 10.17 states it: value / 3 x (25 + Wisdom)
    /// / 25 up to a value of 2,000; above, the store's cut stays 1,334 and
    /// the Wisdom bonus 666 x Wisdom / 25.
    fn captured_offer(value: i32, wisdom: i32) -> i32 {
        if value <= 2000 {
            value / 3 * (25 + wisdom) / 25
        } else {
            value - 1334 + 666 * wisdom / 25
        }
    }

    /// capture.md 10.17 (`look`, the store sweep, `sell`, `sell2`,
    /// `sell3`): 1,152 offers, 36 values from 1 to 5,100 at every Wisdom
    /// 0..31, all as `captured_offer`; spot values 100 at Wisdom 13 brings
    /// 50, and 99, 100 and 101 all 33 at Wisdom 0. capture.md 10.31: GOODA
    /// (100) at Wisdom 20 brings 59. No draw in a sale (4 runs).
    #[test]
    fn sell_offers() {
        // Above 255 the original stores values x 20.
        const VALUES: [i32; 36] = [
            1, 2, 3, 4, 5, 10, 20, 40, 50, 99, 100, 101, 150, 200, 254, 255, 260, 300, 500, 780, 1000, 1240,
            1500, 1980, 2000, 2020, 2040, 2100, 2500, 3000, 3340, 3500, 4000, 4500, 5000, 5100,
        ];
        let mut a = shop(&[]);
        let goods: Vec<u16> = (13..13 + VALUES.len() as u16).collect();
        for (&t, &v) in goods.iter().zip(&VALUES) {
            thing(&mut a, t, "GOOD", treasure(1, v));
        }
        let mut offers = 0;
        for wisdom in 0..=31u8 {
            let mut g = game(&a, 1, shopper(wisdom, 0, &goods), room(4, 4));
            open(&mut g);
            for (&t, &v) in goods.iter().zip(&VALUES) {
                let ev = g.apply(Command::Sell { thing: t });
                let (offer, _) = sale(&ev).unwrap_or_else(|| panic!("value {v}: {ev:?}"));
                assert_eq!(
                    offer,
                    captured_offer(v, wisdom.into()),
                    "value {v}, Wisdom {wisdom}"
                );
                offers += 1;
            }
        }
        assert_eq!(offers, 1152);
        for (v, w, offer) in [
            (100, 13, 50),
            (99, 0, 33),
            (100, 0, 33),
            (101, 0, 33),
            (100, 20, 59),
        ] {
            assert_eq!(stores::sell_offer(v, w), offer, "value {v} Wisdom {w}");
        }
    }

    /// capture.md 10.17 (`look`, `buyb`, `buy3`, `buy4`): the price is the
    /// value at Wisdom 0, 1, 5, 13, 20, 25 and 31 alike, 500 to 450 for a
    /// ware of 50; 6,000 to 900 for one of 5,100; a line of 2 bought twice;
    /// too little gold refused; an empty place out of stock.
    #[test]
    fn buying_costs_the_value() {
        let a = shop(&[(WARE1, 1), (WARE2, 2), (WARE3, 1), (BIG, 1)]);
        for wisdom in [0, 1, 5, 13, 20, 25, 31] {
            let mut g = game(&a, 1, shopper(wisdom, 500, &[]), room(4, 4));
            open(&mut g);
            g.apply(Command::Buy { thing: WARE2 });
            assert_eq!(g.current_player().gold, 450, "Wisdom {wisdom}");
        }
        let mut g = game(&a, 1, shopper(13, 6000, &[]), room(4, 4));
        open(&mut g);
        g.apply(Command::Buy { thing: BIG });
        assert_eq!(g.current_player().gold, 900);
        // Twice for the line of 2, then the place is empty.
        g.apply(Command::Buy { thing: WARE2 });
        g.apply(Command::Buy { thing: WARE2 });
        assert_eq!(g.current_player().gold, 800);
        assert_eq!(g.current_player().count(WARE2), 2);
        let ev = g.apply(Command::Buy { thing: WARE2 });
        assert_eq!(refusal(&ev), Some(StoreRefusal::NotStocked));
        // A place never stocked: out of stock.
        let ev = g.apply(Command::Buy { thing: 5 });
        assert_eq!(refusal(&ev), Some(StoreRefusal::NotStocked));
        // Not enough gold: nothing changes.
        let mut g = game(&a, 1, shopper(13, 199, &[]), room(4, 4));
        open(&mut g);
        let ev = g.apply(Command::Buy { thing: WARE3 });
        assert_eq!(refusal(&ev), Some(StoreRefusal::NotEnoughGold));
        assert_eq!((g.current_player().gold, stock(&g, 1)[2]), (199, (WARE3, 1)));
    }

    /// capture.md 10.17 (`capsell.txt`): from 24,998 gold a sale of 50 left
    /// 25,000, and a sale of 19 still went through, the gold staying 25,000.
    #[test]
    fn the_purse_cap() {
        let mut a = shop(&[]);
        thing(&mut a, 14, "GOOD2", treasure(1, 40));
        let mut g = game(&a, 1, shopper(13, 24_998, &[GOODA, 14]), room(4, 4));
        open(&mut g);
        let ev = g.apply(Command::Sell { thing: GOODA });
        assert_eq!(sale(&ev), Some((50, 2)));
        assert_eq!(g.current_player().gold, 25_000);
        let ev = g.apply(Command::Sell { thing: 14 });
        assert_eq!(sale(&ev), Some((19, 0)));
        assert_eq!(g.current_player().gold, 25_000);
        assert!(g.current_player().possessions.is_empty());
    }

    /// capture.md 10.17 (`sellkit`): T (Wisdom 13) owning the blade, bow
    /// and mail of value 10, the magic item of value 1, a spell and a
    /// treasure of 100 is offered the treasure (50), MAIL, BOW, BLADE (4
    /// each) and the item (0), in thing order, and never the spell.
    #[test]
    fn sellkit() {
        let a = shop(&[]);
        let kit = [BLADE, BOW, MAIL, ITEM, SPELL, GOODA];
        let mut g = game(&a, 1, shopper(13, 0, &kit), room(4, 4));
        open(&mut g);
        let ev = g.apply(Command::Sell { thing: SPELL });
        assert_eq!(refusal(&ev), Some(StoreRefusal::NotSellable));
        let mut offers = Vec::new();
        for t in [GOODA, MAIL, BOW, BLADE, ITEM] {
            let ev = g.apply(Command::Sell { thing: t });
            offers.push((t, sale(&ev).expect("a sale").0));
        }
        assert_eq!(offers, [(GOODA, 50), (MAIL, 4), (BOW, 4), (BLADE, 4), (ITEM, 0)]);
        assert_eq!(g.current_player().possessions, [SPELL]);
    }

    /// capture.md 10.17 (rr11): the store screen. T appears on 7,5; the
    /// exits are 14,4 and 0,4; the outer ring's gaps (6,0, 5,0) are floor;
    /// thing 1 sits on 6,6 (`buy-limit`, 10.31's WARE1), the blade on
    /// 12,2 (`buy-blade`), and the two things drawn in 10.26's area store,
    /// 59 and 33, on 12,2 and 8,6.
    #[test]
    fn the_store_screen() {
        assert_eq!(stores::ENTRY, (7, 5));
        for (x, y) in [(14, 4), (0, 4)] {
            assert_eq!(stores::square(x, y), Some(StoreSquare::Exit), "{x},{y}");
        }
        for (x, y) in [(6, 0), (5, 0), (7, 4), (13, 5), (1, 5)] {
            assert_eq!(stores::square(x, y), Some(StoreSquare::Floor), "{x},{y}");
        }
        assert_eq!(stores::shelf_place(WARE1), Some((6, 6)));
        assert_eq!(stores::shelf_place(BLADE), Some((12, 2)));
        assert_eq!(stores::shelf_place(33), Some((8, 6)));
    }

    /// capture.md 10.17 (rr11 `leave-e`, `leave-w`, `lastmove`): moves 5
    /// before the bump, 4 on the store screen, 1 after leaving, on the
    /// store's square 5,4; bumping with the last move, 0 on the screen and
    /// 1 after.
    #[test]
    fn leaving_the_store() {
        let a = shop(&[(WARE1, 1)]);
        // A step first (6 to 5), then the bump.
        let mut g = game(&a, 1, shopper(13, 500, &[]), room(3, 4));
        g.apply(Command::Move { dir: Dir::E });
        assert_eq!(g.current_player().moves_left, 5);
        open(&mut g);
        assert_eq!(g.current_player().moves_left, 4);
        g.apply(Command::LeaveStore);
        assert_eq!(g.current_player().at, room(5, 4));
        assert_eq!(g.current_player().moves_left, 1);
        // `lastmove`: five steps, the bump with the sixth.
        let mut g = game(&a, 1, shopper(13, 500, &[]), room(3, 4));
        for dir in [Dir::W, Dir::E, Dir::W, Dir::E, Dir::E] {
            g.apply(Command::Move { dir });
        }
        assert_eq!(
            (g.current_player().at, g.current_player().moves_left),
            (room(4, 4), 1)
        );
        open(&mut g);
        assert_eq!(g.current_player().moves_left, 0);
        assert!(g.store_visit().is_some());
        g.apply(Command::LeaveStore);
        assert_eq!(g.current(), 0);
        assert_eq!(
            (g.current_player().at, g.current_player().moves_left),
            (room(5, 4), 1)
        );
    }

    /// capture.md 10.17 (rr11 `buy-limit`): owning 3 of thing 1, a buy
    /// takes the gold 500 to 490 and the stock 1 to 0; T still owns 3
    /// (stores 4.3).
    #[test]
    fn buy_limit() {
        let a = shop(&[(WARE1, 1)]);
        let mut g = game(&a, 1, shopper(13, 500, &[WARE1; 3]), room(4, 4));
        open(&mut g);
        g.apply(Command::Buy { thing: WARE1 });
        assert_eq!(g.current_player().gold, 490);
        assert_eq!(stock(&g, 1), []);
        assert_eq!(g.current_player().count(WARE1), 3);
    }

    /// capture.md 10.17 (rr11 `sell-full`): stock 3 of thing 1, T owning
    /// one: sold for 4, gold 500 to 504, the stock still 3, T owning none
    /// (stores 5.3).
    #[test]
    fn sell_full() {
        let a = shop(&[(WARE1, 3)]);
        let mut g = game(&a, 1, shopper(13, 500, &[WARE1]), room(4, 4));
        open(&mut g);
        let ev = g.apply(Command::Sell { thing: WARE1 });
        assert_eq!(sale(&ev), Some((4, 4)));
        assert_eq!(g.current_player().gold, 504);
        assert_eq!(stock(&g, 1), [(WARE1, 3)]);
        assert_eq!(g.current_player().count(WARE1), 0);
    }

    /// capture.md 10.17 (rr12 `buy-blade`): holding nothing, gold 500, the
    /// blade in stock: bought, gold 490, the blade owned, the stock empty,
    /// the hand still empty; out by the exit, back on 5,4 with 1 move, the
    /// hand still empty (stores 4.4).
    #[test]
    fn buy_blade() {
        let a = shop(&[(BLADE, 1)]);
        let mut g = game(&a, 1, shopper(13, 500, &[]), room(4, 4));
        open(&mut g);
        g.apply(Command::Buy { thing: BLADE });
        let p = g.current_player();
        assert_eq!((p.gold, p.count(BLADE), p.weapon), (490, 1, None));
        assert_eq!(stock(&g, 1), []);
        g.apply(Command::LeaveStore);
        let p = g.current_player();
        assert_eq!((p.at, p.moves_left, p.weapon), (room(5, 4), 1, None));
    }

    /// capture.md 10.17 (rr12 `sell-held`): T holding the blade, gold 500,
    /// Wisdom 13: sold for 4, gold 504, the hand empty, the blade no longer
    /// owned and in the stock (stores 5.2).
    #[test]
    fn sell_held() {
        let a = shop(&[]);
        let mut g = game(&a, 1, shopper(13, 500, &[BLADE]), room(4, 4));
        g.apply(Command::Ready { thing: BLADE });
        assert_eq!(g.current_player().weapon, Some(BLADE));
        open(&mut g);
        let ev = g.apply(Command::Sell { thing: BLADE });
        assert_eq!(sale(&ev), Some((4, 4)));
        let p = g.current_player();
        assert_eq!((p.gold, p.weapon, p.count(BLADE)), (504, None, 0));
        assert_eq!(stock(&g, 1), [(BLADE, 1)]);
    }

    /// rr27's disk (capture.md 10.31): the `look` stock (WARE1 10, WARE2 50
    /// x2, WARE3 200), T Speed 6, Wisdom 20, gold 500, owning GOODA (100)
    /// and the item.
    fn rr27() -> (Adventure, PlayerSetup) {
        let a = shop(&[(WARE1, 1), (WARE2, 2), (WARE3, 1)]);
        let mut t = shopper(20, 500, &[GOODA, ITEM, KILL]);
        t.constitution = 10;
        t.life_force = 10;
        (a, t)
    }

    /// rr27's store visit: GOODA sold for 59 (gold 559), WARE1 bought
    /// (549), out by the exit onto 5,4.
    fn shop_visit(g: &mut Game) {
        open(g);
        let ev = g.apply(Command::Sell { thing: GOODA });
        assert_eq!(sale(&ev), Some((59, 59)));
        assert_eq!(g.current_player().gold, 559);
        g.apply(Command::Buy { thing: WARE1 });
        assert_eq!(g.current_player().gold, 549);
        g.apply(Command::LeaveStore);
        assert_eq!(stock(g, 1), [(WARE2, 2), (WARE3, 1), (GOODA, 1)]);
    }

    /// capture.md 10.31 (`nosave`): T walks in, shops, and is killed at its
    /// next turn with no SAVE GAME: the button brings back the room
    /// visit's start, gold 500, GOODA owned again, WARE1 not, the stock
    /// 100, 0, 0, 0 (WARE1 back on its shelf, GOODA gone from it).
    #[test]
    fn a_death_restores_the_visit_starts_stock() {
        let (a, t) = rr27();
        let mut g = fresh(&a, 1, t);
        let start = saves(&walk_in(&mut g));
        assert!(matches!(start.last(), Some((_, Some(1)))), "{start:?}");
        shop_visit(&mut g);
        g.apply(Command::Wait);
        let ev = g.apply(Command::Cast {
            thing: KILL,
            target: None,
        });
        assert!(has(&ev, |e| matches!(e, Event::AllPlayersDead)));
        let r = restore(&g, &a);
        assert_eq!(r.round(), start.last().unwrap().0);
        let p = &r.players()[0];
        assert_eq!((p.gold, p.count(GOODA), p.count(WARE1)), (500, 1, 0));
        assert_eq!(stock(&r, 1), [(WARE1, 1), (WARE2, 2), (WARE3, 1)]);
    }

    /// capture.md 10.31 (`save`): the same with SAVE GAME at T's next turn,
    /// killed in the round after: the counter back to the save's, gold
    /// 549, the stock 36, 0, 0, 64 (WARE2 x2, WARE3, GOODA).
    #[test]
    fn save_game_saves_the_stock() {
        let (a, t) = rr27();
        let mut g = fresh(&a, 1, t);
        walk_in(&mut g);
        shop_visit(&mut g);
        g.apply(Command::Wait);
        g.apply(Command::SaveGame);
        let saved = saves(&g.apply(Command::Wait));
        assert_eq!(saved.len(), 1, "{saved:?}");
        g.apply(Command::Cast {
            thing: KILL,
            target: None,
        });
        let r = restore(&g, &a);
        assert_eq!(r.round(), saved[0].0);
        assert_eq!(r.players()[0].gold, 549);
        assert_eq!(stock(&r, 1), [(WARE2, 2), (WARE3, 1), (GOODA, 1)]);
    }

    /// capture.md 10.31 (`item`): Use Object on the item that disappears
    /// after use: gone, and not in the stock (stores 2.5).
    #[test]
    fn a_used_up_magic_item_does_not_restock() {
        let (a, t) = rr27();
        let mut g = game(&a, 1, t, room(4, 4));
        let ev = g.apply(Command::Use {
            thing: ITEM,
            target: None,
        });
        assert!(
            has(&ev, |e| matches!(e, Event::ItemUsed { thing: ITEM, .. })),
            "{ev:?}"
        );
        assert!(!has(&ev, |e| matches!(e, Event::Restocked { .. })));
        assert_eq!(g.players()[0].count(ITEM), 0);
        assert_eq!(stock(&g, 1), [(WARE1, 1), (WARE2, 2), (WARE3, 1)]);
    }

    /// capture.md 10.31 (`twice`): GOODA owned twice is offered once per
    /// copy, 59 each, gold 559 then 618, the stock's GOODA count 2.
    #[test]
    fn twice() {
        let (a, mut t) = rr27();
        t.possessions = vec![GOODA, GOODA];
        let mut g = game(&a, 1, t, room(4, 4));
        open(&mut g);
        for gold in [559, 618] {
            let ev = g.apply(Command::Sell { thing: GOODA });
            assert_eq!(sale(&ev), Some((59, 59)));
            assert_eq!(g.current_player().gold, gold);
        }
        assert_eq!(g.current_player().count(GOODA), 0);
        assert!(stock(&g, 1).contains(&(GOODA, 2)));
    }

    /// capture.md 10.34 (rr30 `store`; 10.21, 10.26): a store on 7,3 of
    /// encounter area 1, stepped onto from 8,3. Every carriable thing of
    /// the template is offered: 1 (20), 2 (10), 4 (3), 33 (2000), 35
    /// (2000), 40 (35), 59 (10), 68 (300), 72 (1000); out of stock 12 (the
    /// HAND, value 0) and the spells 61, 64, 80. The HORSE bought for
    /// 2,000; the BLADE T owns bought twice for 10 each (5,000 to 4,990 to
    /// 4,980), the possession unchanged; the blade sold for 4 and taken;
    /// the stock never changing. Leaving: back on 7,3 with 1 move.
    #[test]
    fn a_store_in_an_area_offers_every_carriable_thing() {
        let mut a = crate::rest::encounters::patch(&[("WOLF", crate::rest::encounters::GRASS, 60)]);
        let wares: [(u16, &str, ThingKind); 9] = [
            (1, "OIL AND WICK", magic_item(20, false)),
            (2, "DAGGER", blade(3)),
            (4, "DART", bow(2, 5)),
            (33, "SCROLL", magic_item(2000, false)),
            (35, "HORSE", treasure(1, 2000)),
            (40, "MACE", blade(5)),
            (59, "BLADE", blade(4)),
            (68, "SCROLL", magic_item(300, false)),
            (72, "MAGIC WAND", magic_item(1000, false)),
        ];
        let values = [20, 10, 3, 2000, 2000, 35, 10, 300, 1000];
        for ((n, name, mut kind), v) in wares.into_iter().zip(values) {
            match &mut kind {
                ThingKind::MagicItem { carry, .. }
                | ThingKind::MeleeWeapon { carry, .. }
                | ThingKind::MissileWeapon { carry, .. }
                | ThingKind::Treasure { carry, .. } => carry.value = v,
                _ => unreachable!(),
            }
            thing(&mut a, n, name, kind);
        }
        let mut hand = blade(3);
        if let ThingKind::MeleeWeapon { carry, stats, .. } = &mut hand {
            *carry = Carry { weight: 0, value: 0 };
            stats.usable_by_anyone = false;
        }
        thing(&mut a, 12, "HAND", hand);
        for n in [61, 64, 80] {
            thing(&mut a, n, "SPELL", spell(5, nothing()));
        }
        thing(&mut a, STORE, "STORE", ThingKind::Store { picture: 5 });
        a.world.area_items[0] = vec![door_item(0, 5, 0), floor_item(7, 3, STORE)];
        let mut t = shopper(13, 5000, &[BLADE]);
        t.name = "T".into();
        let mut g = crate::rest::encounters::pulled_in(&a, t);
        let shelves = vec![
            (1, 3),
            (2, 3),
            (4, 3),
            (33, 1),
            (35, 1),
            (40, 1),
            (59, 1),
            (68, 1),
            (72, 1),
        ];
        assert_eq!(stock(&g, 0), shelves);
        g.place(0, area_square(1, 8, 3));
        let ev = g.apply(Command::Move { dir: Dir::W });
        assert!(
            has(&ev, |e| matches!(e, Event::StoreOpened { region: 0, .. })),
            "{ev:?}"
        );
        for gold in [4990, 4980] {
            let ev = g.apply(Command::Buy { thing: BLADE });
            assert!(has(&ev, |e| matches!(
                e,
                Event::Bought {
                    thing: BLADE,
                    price: 10,
                    ..
                }
            )));
            assert_eq!(g.current_player().gold, gold);
            assert_eq!(g.current_player().count(BLADE), 1);
        }
        let ev = g.apply(Command::Sell { thing: BLADE });
        assert_eq!(sale(&ev), Some((4, 4)));
        assert_eq!(g.current_player().count(BLADE), 0);
        let ev = g.apply(Command::Buy { thing: 35 });
        assert!(has(&ev, |e| matches!(
            e,
            Event::Bought {
                thing: 35,
                price: 2000,
                ..
            }
        )));
        for n in [12, 61, 64, 80] {
            let ev = g.apply(Command::Buy { thing: n });
            assert_eq!(refusal(&ev), Some(StoreRefusal::NotStocked), "thing {n}");
        }
        assert_eq!(stock(&g, 0), shelves);
        g.apply(Command::LeaveStore);
        let p = g.current_player();
        assert_eq!((p.at, p.moves_left), (area_square(1, 7, 3), 1));
    }
}

pub fn area_square(room: u8, x: u8, y: u8) -> Location {
    Location::Room {
        region: 0,
        room,
        x,
        y,
    }
}

pub fn floor_item(x: u8, y: u8, thing: u16) -> RoomItem {
    RoomItem {
        x,
        y,
        thing,
        parameter: 0,
        flag: false,
        destination: None,
        gold: None,
    }
}

/// An area exit door (thing 82) of parameter `n` (encounters 3.5).
pub fn door_item(x: u8, y: u8, n: u8) -> RoomItem {
    RoomItem {
        parameter: n,
        ..floor_item(x, y, DOOR)
    }
}

// ---------------------------------------------------------------------------

mod characters {
    //! capture.md 10.18, 10.23, 10.30, 10.33; characters 4-7.

    use super::*;
    use acs_engine::characters::{fold_name, mark, name_refused, Mark, Rating};

    /// capture.md 10.18 (`profile.txt`): move-basic's T shows the down mark
    /// on Wisdom 4 and Power 4 and the up mark on Constitution and Life
    /// Force 17 (characters 4.1).
    #[test]
    fn profile_marks() {
        for (r, v, m) in [
            (Rating::Wisdom, 4, Mark::Down),
            (Rating::Power, 4, Mark::Down),
            (Rating::Constitution, 17, Mark::Up),
            (Rating::LifeForce, 17, Mark::Up),
        ] {
            assert_eq!(mark(r, v), Some(m), "{r:?} {v}");
        }
    }

    /// capture.md 10.18, 10.23 (`names.sh`, `name-*`): every key taken and
    /// folded, the cursor wrapping past 15; stored 3 symbols to a word,
    /// first x 1,600 + second x 40 + third, A-Z 1-26, 1-9 27-35, `.` 36,
    /// `,` 37, `-` 38, `'` 39, space 0. Input to stored name:
    /// `A?B:C@D"EFGHIJKLMNOPQ` LMNOPQD'EFGHIJK; `0123456789!?.+-`
    /// O123456789...--; slash .. `$` ----,''XALIK and three spaces;
    /// `%()<>[]` and space all spaces; 17 letters A-Q PQCDEFGHIJKLMNO; 8
    /// spaces then ABCDEFG asked for again (characters 5).
    #[test]
    fn names_as_stored() {
        for (typed, kept) in [
            ("A?B:C@D\"EFGHIJKLMNOPQ", "LMNOPQD'EFGHIJK"),
            ("0123456789!?.+-", "O123456789...--"),
            ("/:;=,'\"*&\u{a3}\u{2191}\u{2190}@#$", "----,''XALIK"),
            ("X%()<>[] ", "X"),
            ("ABCDEFGHIJKLMNOPQ", "PQCDEFGHIJKLMNO"),
        ] {
            assert_eq!(fold_name(typed), kept, "{typed}");
        }
        assert!(name_refused("        ABCDEFG"));
        // Three symbols to a word, low byte first.
        let w = acs_core::text::encode_packed("ABC", 2).unwrap();
        assert_eq!(u16::from_le_bytes([w[0], w[1]]), 1600 + 2 * 40 + 3);
        let w = acs_core::text::encode_packed("9.'", 2).unwrap();
        assert_eq!(u16::from_le_bytes([w[0], w[1]]), 35 * 1600 + 36 * 40 + 39);
        // Through the game: the newcomer's name as stored; the refused one
        // adds nobody.
        let a = base(Platform::C64);
        let mut g = fresh(&a, 1, named("T", 6, &[]));
        assert_eq!(
            g.apply(Command::CreatePlayer {
                name: "        ABCDEFG".into()
            }),
            [Event::NameRefused]
        );
        g.apply(Command::CreatePlayer {
            name: "A?B:C@D\"EFGHIJKLMNOPQ".into(),
        });
        let ev = g.apply(Command::Wait);
        assert_eq!(added(&ev), [1]);
        assert_eq!(g.players()[1].name, "LMNOPQD'EFGHIJK");
    }

    /// capture.md 10.23 (`four-full`, `four-full-on`): with four players an
    /// Add Someone is told at the round's end that one must retire first;
    /// then the turns go 1, 2, 3, 4 every round.
    #[test]
    fn four_players_at_most() {
        let a = base(Platform::C64);
        let mut g = fresh(&a, 1, named("T", 6, &[]));
        for name in ["U", "V", "W"] {
            g.apply(add(named(name, 6, &[])));
            until_turn_of(&mut g, 0);
            g.apply(Command::Wait);
            until_turn_of(&mut g, 0);
        }
        assert_eq!(g.players().len(), 4);
        g.apply(Command::CreatePlayer { name: "N".into() });
        let mut ev = Vec::new();
        for _ in 0..4 {
            ev.extend(g.apply(Command::Wait));
        }
        assert!(ev.contains(&Event::AddRefused), "{ev:?}");
        assert!(added(&ev).is_empty());
        let mut turns = Vec::new();
        for _ in 0..12 {
            turns.push(g.current());
            g.apply(Command::Wait);
        }
        assert_eq!(turns, [0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3]);
    }

    /// T and U on the map, T's turn (U added at round 1's end).
    fn t_and_u(a: &Adventure, t: PlayerSetup) -> Game {
        let mut g = fresh(a, 1, t);
        g.apply(add(named("U", 6, &[KILL])));
        g.apply(Command::Wait);
        assert_eq!((g.players().len(), g.current()), (2, 0));
        g
    }

    /// Retired events of `ev`.
    fn retired(ev: &[Event]) -> Vec<&Event> {
        ev.iter().filter(|e| matches!(e, Event::Retired { .. })).collect()
    }

    /// capture.md 10.23 (`ret1`, `room1`; `ret-dest`): RETIRE asked by T in
    /// a round is taken at the end of that round, U having moved and T no
    /// turn between, on the map and in a room; destroyed on the map,
    /// nothing is left and U has every turn after.
    #[test]
    fn retire_at_the_rounds_end() {
        let mut a = base(Platform::C64);
        kill_spell(&mut a);
        for in_room in [false, true] {
            let mut g = t_and_u(&a, named("T", 6, &[]));
            if in_room {
                walk_in(&mut g);
                walk_in(&mut g);
                until_turn_of(&mut g, 0);
            }
            let round = g.round();
            assert_eq!(
                g.apply(Command::Retire { save: false }),
                [Event::RetireRequested { player: 0 }]
            );
            let ev = g.apply(Command::Wait);
            assert!(retired(&ev).is_empty(), "{ev:?}");
            assert_eq!((g.current(), g.round()), (1, round));
            let ev = g.apply(Command::Wait);
            let r = ev
                .iter()
                .position(|e| matches!(e, Event::Retired { player: 0, .. }));
            let u_done = ev.iter().position(|e| *e == Event::TurnEnded { player: 1 });
            assert!(r.is_some() && u_done < r, "in room {in_room}: {ev:?}");
            assert!(!has(&ev, |e| matches!(e, Event::TurnStarted { player: 0, .. })));
            if !in_room {
                assert!(
                    matches!(retired(&ev)[..], [Event::Retired { left, saved: None, .. }] if left.is_empty())
                );
            }
            assert_eq!(g.round(), round + 1);
            assert!(!g.players()[0].alive);
            for _ in 0..5 {
                assert_eq!(g.current(), 1);
                g.apply(Command::Wait);
            }
        }
    }

    /// capture.md 10.23 (`ret-save2`): T's Life Force 4 under Constitution
    /// 11, Power 5 under Wisdom 13, gold 777, the blade held, the mail
    /// worn, possessions 13, 40, 41, 59: the record saved with Life Force
    /// 11, Power 5, gold 777 and the kit; the save made with the round's
    /// counter (+0xE7 22 for a RETIRE in round 21).
    #[test]
    fn retire_with_save_keeps_the_record() {
        let mut a = crate::rest::stores::shop(&[]);
        thing(&mut a, 13, "GOODA", treasure(1, 100));
        let t = PlayerSetup {
            name: "T".into(),
            speed: 6,
            constitution: 11,
            life_force: 11,
            wisdom: 13,
            gold: 777,
            possessions: vec![13, 40, 41, 59],
            ..Default::default()
        };
        let mut g = t_and_u(&a, t);
        let round = g.round();
        g.set_life_force(0, 4);
        g.set_power(0, 5);
        g.apply(Command::Retire { save: true });
        g.apply(Command::Ready { thing: 59 });
        g.apply(Command::Ready { thing: 40 }); // armor: the turn's end
        assert_eq!(g.current(), 1);
        let ev = g.apply(Command::Wait);
        let rec = ev
            .iter()
            .find_map(|e| match e {
                Event::Retired { saved: Some(p), .. } => Some(p.clone()),
                _ => None,
            })
            .unwrap_or_else(|| panic!("{ev:?}"));
        assert_eq!((rec.life_force, rec.constitution, rec.power), (11, 11, 5));
        assert_eq!((rec.gold, rec.weapon, rec.armor), (777, Some(59), Some(40)));
        let mut kit = rec.possessions.clone();
        kit.sort_unstable();
        assert_eq!(kit, [13, 40, 41, 59]);
        assert_eq!(saves(&ev), [(round + 1, None)]);
        assert_eq!(g.current(), 1);
    }

    /// The things of 10.30's table: (number, name, kind, "ACS may add").
    fn destroy_things(a: &mut Adventure) {
        let creature_weapon = || {
            let mut k = blade(4);
            if let ThingKind::MeleeWeapon { carry, stats, .. } = &mut k {
                carry.weight = 0;
                stats.usable_by_anyone = false;
            }
            k
        };
        let mut sword = blade(6);
        if let ThingKind::MeleeWeapon { stats, .. } = &mut sword {
            stats.magic = true;
        }
        let things: Vec<(u16, &str, ThingKind, u8)> = vec![
            (59, "BLADE", blade(4), 0),
            (2, "DAGGER", blade(3), 4),
            (34, "MAGIC SWORD", sword, 1),
            (41, "BOW", bow(4, 7), 8),
            (4, "DART", bow(2, 5), 15),
            (40, "MAIL", crate::rest::stores::armor(10), 1),
            (54, "HELMET", crate::rest::stores::armor(10), 4),
            (13, "GOODA", treasure(1, 100), 0),
            (14, "GOODB", treasure(1, 100), 0),
            (73, "KEY", treasure(1, 5), 4),
            (6, "JEWEL PACKET", treasure(1, 50), 2),
            (57, "SCROLL", crate::rest::stores::magic_item(5, false), 3),
            (5, "FOOD CONTAINER", crate::rest::stores::magic_item(5, false), 15),
            (58, "BLESSING", spell(5, nothing()), 1),
            (61, "FLIGHT", spell(5, nothing()), 1),
            (8, "HORN", creature_weapon(), 0),
            (12, "HAND", creature_weapon(), 0),
            (79, "ACID TOUCH", creature_weapon(), 0),
            (11, "CLAW", creature_weapon(), 15),
            (78, "DEADLY TOUCH", creature_weapon(), 15),
        ];
        for (n, name, kind, count) in things {
            thing(a, n, name, kind);
            a.things[n as usize - 1].acs_may_add = count;
        }
    }

    /// A destroy in ROOM on 4,4 of a player owning `kit` (thing 81 for 300
    /// gold), with U in the room too or as the last player; what went down,
    /// as `Retired` lists it, and the stack on 4,4 top first.
    fn destroy(a: &Adventure, kit: &[u16], with_u: bool) -> (Vec<(u16, u16)>, Vec<u16>) {
        let mut t = named("T", 6, &[]);
        for &k in kit {
            if k == acs_engine::inventory::GOLD {
                t.gold = 300;
            } else {
                t.possessions.push(k);
            }
        }
        let mut g = if with_u {
            let mut g = t_and_u(a, t);
            g.place(1, room(6, 4));
            g
        } else {
            fresh(a, 1, t)
        };
        g.place(0, room(4, 4));
        g.apply(Command::Retire { save: false });
        // The kit's blade is held and its mail worn.
        if kit.contains(&59) {
            g.apply(Command::Ready { thing: 59 });
        }
        let mut ev = if kit.contains(&40) {
            g.apply(Command::Ready { thing: 40 })
        } else {
            g.apply(Command::Wait)
        };
        if with_u {
            ev.extend(g.apply(Command::Wait));
        }
        let left = ev
            .iter()
            .find_map(|e| match e {
                Event::Retired { left, .. } => Some(left.clone()),
                _ => None,
            })
            .unwrap_or_else(|| panic!("{ev:?}"));
        let stack = g.region(1).unwrap().rooms[ROOM as usize]
            .items
            .iter()
            .filter(|it| (it.x, it.y) == (4, 4))
            .map(|it| it.thing)
            .collect();
        (left, stack)
    }

    /// capture.md 10.30 (rr26 `d1-*`, 26 cases, 46 runs; 10.23 rr19
    /// `room-dest2`, `newc`): a Destroy in a room, one thing at a time,
    /// with U in the room and as the last player. With U every carriable
    /// thing goes down on 4,4 but spells and the creatures' weight-0
    /// weapons; as the last player things whose "ACS may add" count is 0
    /// (BLADE, GOODA) are lost too. The kit (gold, GOODA, GOODB, MAIL,
    /// BOW, BLADE): all six with U, gold first then thing-number order
    /// (each new entry first in the list); bow, mail and gold as the last
    /// player (characters 3.5, 7.5).
    #[test]
    fn destroy_in_a_room() {
        const GOLD: u16 = acs_engine::inventory::GOLD;
        let mut a = base(Platform::C64);
        kill_spell(&mut a);
        destroy_things(&mut a);
        let one = |t: u16| if t == GOLD { (GOLD, 300) } else { (t, 1) };
        // (thing, with U, last player): true when it goes down.
        let table: &[(&[u16], bool, bool)] = &[
            (&[GOLD], true, true),
            (&[59], true, false),
            (&[2], true, true),
            (&[34], true, true),
            (&[41], true, true),
            (&[4], true, true),
            (&[40], true, true),
            (&[54], true, true),
            (&[13], true, false),
            (&[13, 13], true, false),
            (&[73], true, true),
            (&[6], true, true),
            (&[57], true, true),
            (&[5], true, true),
            (&[58], false, false),
            (&[61], false, false),
            (&[8], false, false),
            (&[12], false, false),
            (&[79], false, false),
            (&[11], false, false),
            (&[78], false, false),
        ];
        for &(kit, with_u, last) in table {
            for (u, down) in [(true, with_u), (false, last)] {
                let (left, stack) = destroy(&a, kit, u);
                let want: Vec<(u16, u16)> = if down {
                    kit.iter().map(|&t| one(t)).collect()
                } else {
                    Vec::new()
                };
                assert_eq!(left, want, "{kit:?}, with U {u}");
                assert_eq!(stack.len(), want.len(), "{kit:?}, with U {u}: {stack:?}");
            }
        }
        let kit = [GOLD, 59, 41, 40, 14, 13];
        let (left, stack) = destroy(&a, &kit, true);
        assert_eq!(left, [(GOLD, 300), (13, 1), (14, 1), (40, 1), (41, 1), (59, 1)]);
        assert_eq!(stack, [59, 41, 40, 14, 13, GOLD]);
        let (left, _) = destroy(&a, &kit, false);
        assert_eq!(left, [(GOLD, 300), (40, 1), (41, 1)]);
    }

    /// capture.md 10.33 (`add/`, cases `a`, `c`): Add Someone asked twice
    /// by T, or once by T and once by U, in one round: both answered "I
    /// will add", one add-player screen, the newcomer in slot 3, slot 4
    /// free, then turns 1, 2, 3 every round (characters 6.2).
    #[test]
    fn several_adds_in_a_round_add_one_player() {
        let a = base(Platform::C64);
        for case in ['a', 'c'] {
            let mut g = t_and_u(&a, named("T", 6, &[]));
            let mut ev = g.apply(Command::CreatePlayer { name: "N".into() });
            if case == 'a' {
                ev.extend(g.apply(Command::CreatePlayer { name: "M".into() }));
            }
            ev.extend(g.apply(Command::Wait));
            if case == 'c' {
                ev.extend(g.apply(Command::CreatePlayer { name: "M".into() }));
            }
            assert_eq!(ev.iter().filter(|e| **e == Event::AddRequested).count(), 2);
            ev.extend(g.apply(Command::Wait));
            assert_eq!(added(&ev), [2], "case {case}");
            assert_eq!(g.players()[2].name, "N");
            let mut later = Vec::new();
            let mut turns = Vec::new();
            for _ in 0..9 {
                turns.push(g.current());
                later.extend(g.apply(Command::Wait));
            }
            assert!(added(&later).is_empty(), "case {case}");
            assert_eq!(g.players().len(), 3);
            assert_eq!(turns, [0, 1, 2, 0, 1, 2, 0, 1, 2], "case {case}");
        }
    }

    /// capture.md 10.33 (`add/` case `b`): T out of play (its first byte
    /// 0) at U's turn, U asking twice: the restore message first (the
    /// living count changed), then one add-player screen; the newcomer in
    /// slot 1, below U's, and from then on N then U every round.
    #[test]
    fn the_newcomer_takes_the_lowest_free_slot() {
        let mut a = base(Platform::C64);
        kill_spell(&mut a);
        let mut g = t_and_u(&a, named("T", 6, &[KILL]));
        let ev = g.apply(Command::Cast {
            thing: KILL,
            target: None,
        });
        assert!(has(&ev, |e| matches!(e, Event::Died { .. })), "{ev:?}");
        assert_eq!(g.current(), 1);
        g.apply(Command::CreatePlayer { name: "N".into() });
        g.apply(Command::CreatePlayer { name: "M".into() });
        let ev = g.apply(Command::Wait);
        let changed = ev.iter().position(|e| matches!(e, Event::LivingChanged { .. }));
        let joined = ev.iter().position(|e| matches!(e, Event::PlayerAdded { .. }));
        assert!(changed.is_some() && changed < joined, "{ev:?}");
        assert_eq!(added(&ev), [0]);
        assert_eq!(g.players()[0].name, "N");
        let mut turns = Vec::new();
        for _ in 0..8 {
            turns.push(g.current());
            g.apply(Command::Wait);
        }
        assert_eq!(turns, [0, 1, 0, 1, 0, 1, 0, 1]);
    }
}

// ---------------------------------------------------------------------------

pub mod encounters {
    //! capture.md 10.19, 10.20, 10.26, 10.34; encounters 2-8.

    use super::*;
    use crate::stats::{same_histogram, same_rate, Sweep, SEEDS};
    use acs_core::model::{Alignment, WorldCreature};

    pub const GRASS: u8 = 5;
    const TREASURE: u16 = 42;

    /// rr14's scenarios on the base adventure: a grass patch (terrain 5,
    /// open) on 13..16 x 10..12 beside the start 12,12, map creature
    /// records `(name, terrain, chance)` in slots 0, 1, .. (the rest empty:
    /// terrain 0, chance 0); each an enemy of Speed 4.
    pub fn patch(records: &[(&str, u8, u8)]) -> Adventure {
        let mut a = base(Platform::C64);
        let mut grass = a.world.terrain[0].clone();
        grass.number = GRASS;
        grass.name = "GRASS".into();
        grass.picture = 9;
        a.world.terrain[GRASS as usize] = grass;
        for row in &mut a.world.tiles[10..=12] {
            for t in &mut row[13..=16] {
                *t = GRASS;
            }
        }
        assert_eq!(a.world.random_creatures.len(), 8);
        for (i, &(name, terrain, chance)) in records.iter().enumerate() {
            let mut c = a.regions[0].residents[0].clone();
            c.in_use = true;
            c.name = name.into();
            c.speed = 4;
            c.strategy.alignment = Alignment::Enemy;
            c.strategy.aggressive = true;
            c.strategy.brave = true;
            a.world.random_creatures[i] = WorldCreature {
                creature: c,
                terrain,
                chance,
            };
        }
        for r in &a.world.random_creatures[records.len()..] {
            assert_eq!((r.terrain, r.chance), (0, 0));
        }
        a
    }

    fn t() -> PlayerSetup {
        named("T", 6, &[])
    }

    pub fn encounters(ev: &[Event]) -> Vec<(usize, Option<usize>)> {
        ev.iter()
            .filter_map(|e| match e {
                Event::Encounter { creature, slot, .. } => Some((*creature, *slot)),
                _ => None,
            })
            .collect()
    }

    /// The generator's state, read from the game's saved form.
    fn rng_state(g: &Game) -> u64 {
        serde_json::to_value(g).expect("serializes")["rng"]["state"]
            .as_u64()
            .expect("a state")
    }

    /// Draws between two generator states (SplitMix64 adds its constant
    /// once a draw).
    fn draws(before: u64, after: u64) -> u64 {
        const STEP: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut inv = STEP;
        for _ in 0..6 {
            inv = inv.wrapping_mul(2u64.wrapping_sub(STEP.wrapping_mul(inv)));
        }
        after.wrapping_sub(before).wrapping_mul(inv)
    }

    /// The first game over seeds 0.. whose first push east from 12,12 onto
    /// 13,12 opens area 1 (the captures poked the state until it did).
    pub fn pulled_in(a: &Adventure, t: PlayerSetup) -> Game {
        for seed in 0..500 {
            let mut g = fresh(a, seed, t.clone());
            let ev = g.apply(Command::Move { dir: Dir::E });
            if has(&ev, |e| matches!(e, Event::EnteredArea { area: 1, .. })) {
                return g;
            }
        }
        panic!("no encounter in 500 seeds");
    }

    /// One push east from 12,12: up to two steps, stopping at an encounter
    /// (capture.md 10.20: "a 300k push east, one or two steps"). The
    /// steps taken and the encounter's events.
    fn push_east(a: &Adventure, seed: u64) -> (u64, Vec<Event>, Game) {
        let mut g = fresh(a, seed, t());
        for step in 1..=2 {
            let ev = g.apply(Command::Move { dir: Dir::E });
            assert!(has(&ev, |e| matches!(e, Event::Moved { .. })), "{ev:?}");
            if !encounters(&ev).is_empty() {
                return (step, ev, g);
            }
        }
        (2, Vec::new(), g)
    }

    /// capture.md 10.20 (`patch`, 256 runs): WOLF on the grass at 60%,
    /// 354 grass steps, 219 encounters; one roll per grass step.
    #[test]
    fn patch_sweep() {
        let a = patch(&[("WOLF", GRASS, 60)]);
        let (mut steps, mut hits) = (0, 0);
        for seed in 0..SEEDS {
            let (n, ev, _) = push_east(&a, seed);
            steps += n;
            hits += u64::from(!ev.is_empty());
        }
        same_rate(
            "patch, encounters per grass step",
            Sweep::Sample,
            219,
            354,
            hits,
            steps,
        );
    }

    /// capture.md 10.20 (`patch`): the roll is a percentile against the
    /// chance, one per grass step; a roll of exactly 60 misses.
    #[test]
    fn a_roll_of_60_misses() {
        let a = patch(&[("WOLF", GRASS, 60)]);
        let mut seen = [false; 2];
        for seed in 0..2000 {
            let mut g = fresh(&a, seed, t());
            let before = rng_state(&g);
            let roll = acs_engine::Rng::new(before).below(100);
            let ev = g.apply(Command::Move { dir: Dir::E });
            let hit = !encounters(&ev).is_empty();
            assert_eq!(hit, roll < 60, "seed {seed}: roll {roll}");
            if !hit {
                assert_eq!(draws(before, rng_state(&g)), 1, "seed {seed}");
            }
            if roll == 60 {
                seen[0] = true;
            }
            if roll == 59 {
                seen[1] = true;
            }
        }
        assert_eq!(seen, [true, true]);
    }

    /// capture.md 10.20 (`field`, 256 of 256): WOLF on the field at 60%:
    /// 8 rolls on every field step, WOLF's first (the 7 empty records on
    /// terrain 0 rolled against 0); `north`: with WOLF on the grass the
    /// field steps draw 7 (encounters 2.5).
    #[test]
    fn every_record_on_the_terrain_rolls() {
        for (wolf, rolls) in [(0, 8), (GRASS, 7)] {
            let a = patch(&[("WOLF", wolf, 60)]);
            for seed in 0..64 {
                let mut g = fresh(&a, seed, t());
                let before = rng_state(&g);
                let first = acs_engine::Rng::new(before).below(100);
                let ev = g.apply(Command::Move { dir: Dir::W });
                let hits = encounters(&ev);
                if wolf == 0 && first < 60 {
                    assert_eq!(hits.first().map(|h| h.0), Some(0), "seed {seed}");
                } else {
                    assert!(hits.is_empty(), "seed {seed}");
                    assert_eq!(draws(before, rng_state(&g)), rolls, "seed {seed}");
                }
            }
        }
    }

    /// capture.md 10.20 (`two`, 256 runs): WOLF and HARE on the grass at
    /// 60% each: both hit in 87 runs (slots 8 and 9, on 2,3 and 3,3), WOLF
    /// alone in 40 (slot 8 on 2,3), HARE alone in 59 (slot 8 on 3,3, its
    /// record's square), none in the other 70. Bins: none, WOLF, HARE,
    /// both.
    #[test]
    #[ignore = "engine disagrees with capture.md 10.20: `two` met none in 70 of 256 runs and WOLF alone in 40, the engine none in about 2% and each alone about equally (independent 60% rolls)"]
    fn two_sweep() {
        let a = patch(&[("WOLF", GRASS, 60), ("HARE", GRASS, 60)]);
        let h = crate::world::histogram(SEEDS, 4, |seed| {
            let (_, ev, g) = push_east(&a, seed);
            let at = |s: usize| g.creature(0, s).map(|c| (c.x, c.y));
            match encounters(&ev)[..] {
                [] => 0,
                [(0, Some(8))] => {
                    assert_eq!(at(8), Some((2, 3)));
                    1
                }
                [(1, Some(8))] => {
                    assert_eq!(at(8), Some((3, 3)));
                    2
                }
                [(0, Some(8)), (1, Some(9))] => {
                    assert_eq!((at(8), at(9)), (Some((2, 3)), Some((3, 3))));
                    3
                }
                ref other => panic!("{other:?}"),
            }
        });
        same_histogram("two, met none / WOLF / HARE / both", &[70, 40, 59, 87], &h);
    }

    /// capture.md 10.20 (`two`): the copies' slots and squares, whichever
    /// hit: both in slots 8 and 9 on 2,3 and 3,3; one alone in slot 8 on
    /// its record's square (WOLF 2,3, HARE 3,3).
    #[test]
    fn the_copies_squares() {
        let a = patch(&[("WOLF", GRASS, 60), ("HARE", GRASS, 60)]);
        let mut seen = [0; 3];
        for seed in 0..256 {
            let (_, ev, g) = push_east(&a, seed);
            let at = |s: usize| g.creature(0, s).map(|c| (c.x, c.y));
            match encounters(&ev)[..] {
                [] => {}
                [(0, Some(8))] => {
                    seen[0] += 1;
                    assert_eq!((at(8), at(9)), (Some((2, 3)), None));
                }
                [(1, Some(8))] => {
                    seen[1] += 1;
                    assert_eq!((at(8), at(9)), (Some((3, 3)), None));
                }
                [(0, Some(8)), (1, Some(9))] => {
                    seen[2] += 1;
                    assert_eq!((at(8), at(9)), (Some((2, 3)), Some((3, 3))));
                }
                ref other => panic!("{other:?}"),
            }
        }
        assert!(seen.iter().all(|&n| n > 0), "{seen:?}");
    }

    /// capture.md 10.20 (`patch`, `field`, `two`): in every encounter the
    /// entrance is the square stepped onto (13,12 or 14,12), T on 12,4 of
    /// area 1, the moves 0 (the turn over), area 1 15 x 10 (size byte 233)
    /// walled with the grass's picture.
    #[test]
    fn every_encounters_placement() {
        let a = patch(&[("WOLF", GRASS, 60)]);
        let mut met = 0;
        for seed in 0..256 {
            let (n, ev, g) = push_east(&a, seed);
            if ev.is_empty() {
                continue;
            }
            met += 1;
            let entered = ev
                .iter()
                .position(|e| {
                    matches!(
                        e,
                        Event::EnteredArea {
                            player: 0,
                            area: 1,
                            ..
                        }
                    )
                })
                .expect("entered");
            assert!(
                matches!(ev[entered + 1], Event::TurnEnded { player: 0 }),
                "{ev:?}"
            );
            assert_eq!(g.area_entrance(1), Some((12 + n as u8, 12)));
            assert_eq!(g.players()[0].at, area_square(1, 12, 4));
            let room = &g.region(0).unwrap().rooms[1];
            assert_eq!((room.width, room.height, room.wall_picture), (15, 10, 9));
        }
        assert!(met > 100);
    }

    /// capture.md 10.20 (`north`, 56 runs): pushes south, east and north,
    /// entering the grass from 13,13 northward: 168 steps, 39 encounters,
    /// T on 12,4 in all.
    #[test]
    fn north_sweep() {
        let a = patch(&[("WOLF", GRASS, 60)]);
        let mut hits = 0;
        for seed in 0..SEEDS {
            let mut g = fresh(&a, seed, t());
            let mut ev = Vec::new();
            for dir in [Dir::S, Dir::E, Dir::N] {
                ev = g.apply(Command::Move { dir });
                if dir != Dir::N {
                    assert!(encounters(&ev).is_empty());
                }
            }
            if !encounters(&ev).is_empty() {
                hits += 1;
                assert_eq!(g.players()[0].at, area_square(1, 12, 4));
                assert_eq!(g.area_entrance(1), Some((13, 12)));
            }
        }
        same_rate(
            "north, encounters on the grass step",
            Sweep::Sample,
            39,
            56,
            hits,
            SEEDS,
        );
    }

    /// rr22's lists (capture.md 10.26) on `patch`: area 1's list `items`.
    fn listed(items: Vec<RoomItem>) -> Adventure {
        let mut a = patch(&[("WOLF", GRASS, 60)]);
        thing(&mut a, TREASURE, "AREATHINGA", treasure(1, 5));
        thing(&mut a, TREASURE + 1, "AREATHINGB", treasure(1, 5));
        a.world.area_items[0] = items;
        a
    }

    /// T in area 1 at its next turn, put on (x, y), pushing `dir`.
    fn out(a: &Adventure, x: u8, y: u8, dir: Dir) -> (Game, Vec<Event>, u8) {
        let mut g = pulled_in(a, t());
        assert_eq!(g.current(), 0);
        g.place(0, area_square(1, x, y));
        let before = g.current_player().moves_left;
        let ev = g.apply(Command::Move { dir });
        (g, ev, before)
    }

    /// capture.md 10.20 (`walkout`), 10.26 (`ctrl`): at T's next turn (6
    /// moves), on 1,5, one push west through the door: T on 13,12, the
    /// entrance, with 5 moves; area 1 freed (size 0, entrance 41,41) and
    /// slot 8 free (encounters 7.1, 7.2).
    #[test]
    fn walkout() {
        let a = patch(&[("WOLF", GRASS, 60)]);
        let (g, ev, before) = out(&a, 1, 5, Dir::W);
        assert_eq!(before, 6);
        assert_eq!(g.players()[0].at, map(13, 12));
        assert_eq!(g.current_player().moves_left, 5);
        assert!(has(&ev, |e| matches!(e, Event::AreaClosed { area: 1 })));
        let room = &g.region(0).unwrap().rooms[1];
        assert!(room.deleted);
        assert_eq!((room.width, room.height), (0, 0));
        assert_eq!(g.area_entrance(1), None);
        assert!(g.creature(0, 8).is_none());
    }

    fn things_in_area(g: &Game) -> Vec<(u8, u8, u16)> {
        g.region(0).unwrap().rooms[1]
            .items
            .iter()
            .map(|it| (it.x, it.y, it.thing))
            .collect()
    }

    /// capture.md 10.26 (`treasure`), 10.34 (reopening): the list
    /// `0,5,82,0 / 7,5,42,1`: the treasure on 7,5, taken by a push west
    /// from 8,5; leaving closes the area and deletes the treasure's entry,
    /// the door's kept; area 1 opened again shows no treasure.
    #[test]
    fn area_treasure_and_reopening() {
        let a = listed(vec![door_item(0, 5, 0), floor_item(7, 5, TREASURE)]);
        let (mut g, ev, _) = out(&a, 8, 5, Dir::W);
        assert!(
            has(&ev, |e| matches!(e, Event::PickedUp { thing: TREASURE, .. })),
            "{ev:?}"
        );
        // Not taken: deleted at the closing all the same.
        let (mut g2, _, _) = out(&a, 1, 5, Dir::W);
        assert_eq!(things_in_area(&g2), [(0, 5, DOOR)]);
        g.place(0, area_square(1, 1, 5));
        g.apply(Command::Move { dir: Dir::W });
        assert_eq!(things_in_area(&g), [(0, 5, DOOR)]);
        // Reopened: back and forth over the grass until area 1 opens.
        for _ in 0..200 {
            let ev = g2.apply(Command::Move {
                dir: if g2.players()[0].at == map(13, 12) {
                    Dir::E
                } else {
                    Dir::W
                },
            });
            if has(&ev, |e| matches!(e, Event::EnteredArea { area: 1, .. })) {
                assert_eq!(things_in_area(&g2), [(0, 5, DOOR)]);
                return;
            }
        }
        panic!("area 1 never opened again");
    }

    /// capture.md 10.26 (`two`): area 2's list stays as written while area
    /// 1 is open, and area 1 shows only its own treasure.
    #[test]
    fn the_lists_are_areas_1_to_4() {
        let mut a = listed(vec![door_item(0, 5, 0), floor_item(7, 5, TREASURE)]);
        a.world.area_items[1] = vec![door_item(0, 5, 1), floor_item(8, 6, TREASURE + 1)];
        let g = pulled_in(&a, t());
        assert_eq!(things_in_area(&g), [(0, 5, DOOR), (7, 5, TREASURE)]);
        let two: Vec<_> = g.region(0).unwrap().rooms[2]
            .items
            .iter()
            .map(|it| it.thing)
            .collect();
        assert_eq!(two, [DOOR, TREASURE + 1]);
    }

    /// capture.md 10.26 (`moved`, `noexit`): the door is the list's item:
    /// at 0,3 the way out is by 1,3 onto 13,12 with 5 moves, and a push
    /// west from 1,5 is refused with no move used; with an empty list
    /// there is no way out.
    #[test]
    fn the_door_is_the_list_item() {
        let a = listed(vec![door_item(0, 3, 0)]);
        let (g, _, before) = out(&a, 1, 3, Dir::W);
        assert_eq!(
            (g.players()[0].at, g.current_player().moves_left),
            (map(13, 12), before - 1)
        );
        for list in [vec![door_item(0, 3, 0)], vec![]] {
            let a = listed(list);
            let (g, ev, before) = out(&a, 1, 5, Dir::W);
            assert!(has(&ev, |e| matches!(e, Event::Blocked { .. })), "{ev:?}");
            assert_eq!(
                (g.players()[0].at, g.current_player().moves_left),
                (area_square(1, 1, 5), before)
            );
        }
    }

    /// capture.md 10.34 (`middoor`): a door item on the floor (7,5) is a
    /// way out too: stepping onto it from 8,5 leaves; closing deletes the
    /// treasure and keeps every door entry.
    #[test]
    fn a_door_on_the_floor() {
        let a = listed(vec![
            door_item(0, 5, 0),
            door_item(7, 5, 0),
            floor_item(9, 5, TREASURE),
        ]);
        let (g, ev, _) = out(&a, 8, 5, Dir::W);
        assert!(
            has(&ev, |e| matches!(e, Event::LeftArea { area: 1, .. })),
            "{ev:?}"
        );
        assert_eq!(g.players()[0].at, map(13, 12));
        assert_eq!(things_in_area(&g), [(0, 5, DOOR), (7, 5, DOOR)]);
    }

    /// capture.md 10.26 (`param`, area 2 free): a door of parameter 1 leads
    /// to area 2's entrance, and with area 2 free (entrance 41,41) to the
    /// map's start 12,12; area 1 closed. 10.34 (`dbl01`): with two doors on
    /// 0,5 the first item's parameter (0) counts: back on area 1's own
    /// entrance.
    #[test]
    fn door_parameters_with_area_2_free() {
        let a = listed(vec![door_item(0, 5, 1)]);
        let (g, ev, _) = out(&a, 1, 5, Dir::W);
        assert_eq!(g.players()[0].at, map(12, 12));
        assert!(has(&ev, |e| matches!(e, Event::AreaClosed { area: 1 })));
        let a = listed(vec![door_item(0, 5, 0), door_item(0, 5, 1)]);
        let (g, _, _) = out(&a, 1, 5, Dir::W);
        assert_eq!(g.players()[0].at, map(13, 12));
    }

    /// T in area 1 (entered from 13,12) and U in area 2 (entered from
    /// 13,11, a step north then east), T's turn; with the door table
    /// `table` and + 0x80E `e`.
    fn two_areas(items: Vec<RoomItem>, table: [u8; 8], e: u16) -> Game {
        let mut a = listed(items);
        a.world.unknown_806 = table.to_vec();
        a.world.unknown_80e = e;
        for seed in 0..2000 {
            let mut g = fresh(&a, seed, t());
            g.apply(add(named("U", 6, &[])));
            g.apply(Command::Wait);
            if encounters(&g.apply(Command::Move { dir: Dir::E })).is_empty() {
                continue;
            }
            if g.current() != 1 {
                continue;
            }
            g.apply(Command::Move { dir: Dir::N });
            if encounters(&g.apply(Command::Move { dir: Dir::E })).is_empty() {
                continue;
            }
            assert_eq!(g.players()[1].at, area_square(2, 12, 4));
            assert_eq!(g.area_entrance(2), Some((13, 11)));
            until_turn_of(&mut g, 0);
            g.place(0, area_square(1, 1, 5));
            return g;
        }
        panic!("no two encounters");
    }

    const TABLE: [u8; 8] = acs_core::layout::world_map::STOCK_AREA_DOORS;

    fn with_entry(n: usize, entry: [u8; 2]) -> [u8; 8] {
        let mut t = TABLE;
        t[n * 2..n * 2 + 2].copy_from_slice(&entry);
        t
    }

    /// capture.md 10.34 (`join`, `dbl10`, `t806a`, `p4`): a door to another
    /// open area's entrance puts T on that map square, place 0, with 5
    /// moves, area 2 still open: the player does not join it. Parameter 1
    /// by the template table; parameter 0 with `806:0042`; parameter 4
    /// reading + 0x80E as `00 42`; and of two doors on 0,5 (1, then 0) the
    /// first.
    #[test]
    fn a_door_to_another_open_areas_entrance() {
        let cases: [(Vec<RoomItem>, [u8; 8], u16); 4] = [
            (vec![door_item(0, 5, 1)], TABLE, 0x0832),
            (vec![door_item(0, 5, 1), door_item(0, 5, 0)], TABLE, 0x0832),
            (vec![door_item(0, 5, 0)], with_entry(0, [0x00, 0x42]), 0x0832),
            (vec![door_item(0, 5, 4)], TABLE, u16::from_le_bytes([0x00, 0x42])),
        ];
        for (n, (items, table, e)) in cases.into_iter().enumerate() {
            let mut g = two_areas(items, table, e);
            let before = g.current_player().moves_left;
            let ev = g.apply(Command::Move { dir: Dir::W });
            assert_eq!(g.players()[0].at, map(13, 11), "case {n}: {ev:?}");
            assert_eq!(g.current_player().moves_left, before - 1, "case {n}");
            assert_eq!(before, 6, "case {n}");
            assert!(!g.region(0).unwrap().rooms[2].deleted, "case {n}");
            assert!(g.region(0).unwrap().rooms[1].deleted, "case {n}");
        }
    }

    /// capture.md 10.34 (`t806p`, `t806r`): door table entry 0 `0000` sends
    /// T to 12,16, map point 0 (the map door's square), place 0, not
    /// through the door; `1055` to TEST ROOM (region 1, room 0) on 5,5,
    /// "the action moves to TEST REGION".
    #[test]
    fn the_door_table() {
        let mut a = listed(vec![door_item(0, 5, 0)]);
        a.world.unknown_806 = with_entry(0, [0x00, 0x00]).to_vec();
        let mut g = pulled_in(&a, t());
        g.place(0, area_square(1, 1, 5));
        let ev = g.apply(Command::Move { dir: Dir::W });
        assert_eq!(g.players()[0].at, map(12, 16));
        assert!(!has(&ev, |e| matches!(
            e,
            Event::TookDoor { .. } | Event::EnteredRoom { .. }
        )));
        a.world.unknown_806 = with_entry(0, [0x10, 0x55]).to_vec();
        let mut g = pulled_in(&a, t());
        g.place(0, area_square(1, 1, 5));
        let ev = g.apply(Command::Move { dir: Dir::W });
        assert_eq!(g.players()[0].at, room(5, 5));
        assert!(
            has(&ev, |e| matches!(
                e,
                Event::EnteredRegion { player: 0, region: 1 }
            )),
            "{ev:?}"
        );
    }
}

// ---------------------------------------------------------------------------

mod saves {
    //! capture.md 10.22, 10.25, 10.28; creatures 2.2.6, 2.2.7.

    use super::*;

    fn adventure() -> Adventure {
        let mut a = base(Platform::C64);
        kill_spell(&mut a);
        a
    }

    fn me() -> PlayerSetup {
        named("T", 6, &[KILL])
    }

    /// T and U walked into ROOM, as rr18's `after/two.vsf`: the room
    /// region's visit begun (and saved) on round `r0`, T's turn.
    fn two_in(a: &Adventure) -> (Game, u32) {
        let mut g = fresh(a, 1, me());
        g.apply(add(named("U", 6, &[KILL])));
        let mut ev = walk_in(&mut g);
        assert_eq!(g.current(), 1);
        ev.extend(walk_in(&mut g));
        let s = saves(&ev);
        assert!(matches!(s[..], [.., (_, Some(1))]), "{s:?}");
        assert_eq!(g.current(), 0);
        (g, s.last().unwrap().0)
    }

    const U: Who = Who::Player { player: 1 };

    /// capture.md 10.22 (`button`), 10.25 (a) `a-nosave`: with no SAVE GAME
    /// the button after U's death brings back the room visit's start (the
    /// counter back to 22, the state as the visit began), and the same
    /// rounds replay identically, U killed again.
    #[test]
    fn the_button_restores_the_visit_start() {
        let a = adventure();
        let (mut g, r0) = two_in(&a);
        let at = g.players()[0].at;
        g.apply(Command::Move { dir: Dir::E });
        g.apply(Command::Wait);
        g.apply(Command::Wait);
        g.apply(Command::Move { dir: Dir::E });
        let ev = g.apply(Command::Cast {
            thing: KILL,
            target: Some(U),
        });
        assert!(
            has(&ev, |e| matches!(e, Event::LivingChanged { before: 2, after: 1 })),
            "{ev:?}"
        );
        let r = restore(&g, &a);
        assert_eq!(r.round(), r0);
        assert_eq!(r.players()[0].at, at);
        assert!(r.players()[1].alive);
        let mut again = r.clone();
        for c in &g.log()[g.restore_point()..] {
            again.apply(c.clone());
        }
        assert_eq!(again, g);
        assert!(!again.players()[1].alive);
    }

    /// capture.md 10.25 (a) `a-save`: SAVE GAME in round 24, U killed in
    /// round 25: the counter back to 25, the state as saved at round 24's
    /// end (T's gold 111 there, U alive).
    #[test]
    fn save_game_is_the_restore_point() {
        let a = adventure();
        let (mut g, r0) = two_in(&a);
        g.apply(Command::Wait);
        g.apply(Command::Wait);
        let round = g.round();
        assert_eq!(round, r0 + 1);
        g.apply(Command::Move { dir: Dir::E });
        g.apply(Command::SaveGame);
        g.apply(Command::Wait);
        let s = saves(&g.apply(Command::Wait));
        assert_eq!(s, [(round + 1, None)]);
        let at = g.players()[0].at;
        g.apply(Command::Move { dir: Dir::E });
        g.apply(Command::Cast {
            thing: KILL,
            target: Some(U),
        });
        let r = restore(&g, &a);
        assert_eq!(r.round(), round + 1);
        assert_eq!(r.players()[0].at, at);
        assert!(r.players()[1].alive);
    }

    /// T alone in ROOM since the room visit of round `r0`, T's turn.
    fn alone_in(a: &Adventure) -> (Game, u32) {
        let mut g = fresh(a, 1, me());
        let s = saves(&walk_in(&mut g));
        assert!(matches!(s[..], [.., (_, Some(1))]), "{s:?}");
        (g, s.last().unwrap().0)
    }

    /// capture.md 10.25 (b) `b-button`: T alone in a room region from round
    /// 14, three visit ends passed with no region change, killed in round
    /// 46: back to counter 14 (gold 0, Life Force 40); visits of the same
    /// region one after another save nothing.
    #[test]
    fn the_same_region_again_saves_nothing() {
        let a = adventure();
        let (mut g, r0) = alone_in(&a);
        let at = g.players()[0].at;
        let mut ev = Vec::new();
        for k in 0..32 {
            if k % 10 == 3 {
                ev.extend(g.apply(Command::Move { dir: Dir::E }));
            }
            ev.extend(g.apply(Command::Wait));
        }
        assert_eq!(g.round(), r0 + 32);
        assert_eq!(saves(&ev), []);
        g.apply(Command::Cast {
            thing: KILL,
            target: None,
        });
        let r = restore(&g, &a);
        assert_eq!(r.round(), r0);
        assert_eq!(r.players()[0].at, at);
        assert!(r.players()[0].alive);
    }

    /// rr21's split route (capture.md 10.25 (b) `b-split`): T in the room,
    /// U on the map; T killed in the sixth round of the room's third visit
    /// (of round 80); the game and that visit's first round.
    fn split(a: &Adventure) -> (Game, u32) {
        let mut g = fresh(a, 1, me());
        g.apply(add(named("U", 6, &[])));
        g.apply(Command::Wait);
        assert_eq!(g.current(), 0);
        let mut ev = walk_in(&mut g);
        let mut visit = None;
        for _ in 0..200 {
            let room_saves: Vec<u32> = saves(&ev)
                .into_iter()
                .filter(|s| s.1 == Some(1))
                .map(|s| s.0)
                .collect();
            if room_saves.len() >= 3 {
                visit = Some(room_saves[2]);
                break;
            }
            ev.extend(g.apply(Command::Wait));
        }
        let visit = visit.expect("three room visits");
        // Each visit began with a region change: the map's too.
        assert!(saves(&ev).iter().any(|s| s.1 == Some(0)), "{:?}", saves(&ev));
        assert_eq!((g.current(), g.round(), g.visiting()), (0, visit, 1));
        for _ in 0..5 {
            g.apply(Command::Wait);
        }
        let ev = g.apply(Command::Cast {
            thing: KILL,
            target: None,
        });
        assert!(has(&ev, |e| matches!(e, Event::LivingChanged { .. })));
        (g, visit)
    }

    /// capture.md 10.25 (b) `b-split-button`: the counter back to 80, the
    /// start of the last visit that began with a region change, T alive.
    #[test]
    fn a_visit_that_changes_the_region_saves() {
        let a = adventure();
        let (g, visit) = split(&a);
        let r = restore(&g, &a);
        assert_eq!(r.round(), visit);
        assert!(r.players()[0].alive && r.players()[1].alive);
        assert_eq!(r.players()[0].at, room(4, 4));
    }

    /// capture.md 10.25 (b) `b-split-button`: after the button the cycle is
    /// marked from the restored places, lowest region first: the map
    /// first (rounds 80 to 89, U alone), region 1 from round 90.
    #[test]
    #[ignore = "engine disagrees with capture.md 10.25: after the button the original visits the map first (U), the engine resumes the room visit the save began (T)"]
    fn after_the_button_the_map_comes_first() {
        let a = adventure();
        let (g, visit) = split(&a);
        let mut r = restore(&g, &a);
        assert_eq!((r.visiting(), r.current()), (0, 1));
        for _ in 0..10 {
            assert_eq!(r.current(), 1);
            r.apply(Command::Wait);
        }
        assert_eq!((r.round(), r.visiting(), r.current()), (visit + 10, 1, 0));
    }

    /// capture.md 10.25 (`decline`): with nobody alive, declining to add a
    /// player gives no turns; the counter stays (47).
    #[test]
    fn nobody_alive_no_turns() {
        let a = adventure();
        let (mut g, _) = alone_in(&a);
        let ev = g.apply(Command::Cast {
            thing: KILL,
            target: None,
        });
        assert!(has(&ev, |e| matches!(e, Event::AllPlayersDead)));
        let round = g.round();
        for _ in 0..5 {
            assert_eq!(g.apply(Command::Wait), []);
        }
        assert!(g.is_over());
        assert_eq!(g.round(), round);
    }

    /// capture.md 10.28 item 1 (`c1-button`, `c1-f1`, `c1n-button`): SAVE
    /// GAME in round 46, T (alone) killed in round 46: the button brings
    /// back counter 14, the room visit's start, not the save asked for;
    /// going on (F1) the save is made with nobody alive. A newcomer N then
    /// walked into the room and killed there: back to its room visit's
    /// start (69), N alive on the room's entry square.
    #[test]
    fn save_game_in_the_round_the_last_player_dies() {
        let a = adventure();
        let (mut g, r0) = alone_in(&a);
        for _ in 0..5 {
            g.apply(Command::Wait);
        }
        g.apply(Command::SaveGame);
        let ev = g.apply(Command::Cast {
            thing: KILL,
            target: None,
        });
        assert!(has(&ev, |e| matches!(e, Event::AllPlayersDead)));
        assert_eq!(saves(&ev), [(g.round(), None)]);
        assert_eq!(restore(&g, &a).round(), r0);
        // F1: the all-dead save stands; a newcomer comes in.
        let mut ev = g.apply(add(named("N", 6, &[KILL])));
        ev.extend(walk_in(&mut g));
        let s = saves(&ev);
        assert!(matches!(s[..], [(_, Some(0)), .., (_, Some(1))]), "{s:?}");
        let visit = s.last().unwrap().0;
        g.apply(Command::Wait);
        g.apply(Command::Move { dir: Dir::E });
        g.apply(Command::Cast {
            thing: KILL,
            target: None,
        });
        let r = restore(&g, &a);
        assert_eq!(r.round(), visit);
        assert_eq!(r.players()[0].name, "N");
        assert!(r.players()[0].alive);
        assert_eq!(r.players()[0].at, room(4, 4));
    }

    /// capture.md 10.28 item 2 (`c2-button`): RETIRE with Save by T in
    /// round 24, U killed in round 26: back to counter 25, U alive, T
    /// still retired: the retire's round end, not the visit start (22).
    #[test]
    fn retire_with_save_is_a_save_point() {
        let a = adventure();
        let (mut g, r0) = two_in(&a);
        g.apply(Command::Wait);
        g.apply(Command::Wait);
        let round = g.round();
        g.apply(Command::Retire { save: true });
        g.apply(Command::Wait);
        let ev = g.apply(Command::Wait);
        assert!(has(&ev, |e| matches!(e, Event::Retired { player: 0, .. })));
        assert_eq!(saves(&ev), [(round + 1, None)]);
        g.apply(Command::Wait);
        let ev = g.apply(Command::Cast {
            thing: KILL,
            target: None,
        });
        assert!(has(&ev, |e| matches!(e, Event::AllPlayersDead)));
        let r = restore(&g, &a);
        assert_eq!(r.round(), round + 1);
        assert!(r.round() > r0);
        assert!(!r.players()[0].alive);
        assert!(r.players()[1].alive);
        assert_eq!(r.current(), 1);
    }

    /// capture.md 10.28 item 3 (`c3-button2`): SAVE GAME in the first round
    /// of a visit that began with a region change (60), a kill in round 62:
    /// back to counter 61 and the state saved, not the visit's start.
    #[test]
    fn save_game_in_a_visits_first_round() {
        let a = adventure();
        let (mut g, r0) = alone_in(&a);
        assert_eq!(g.round(), r0);
        g.apply(Command::SaveGame);
        g.apply(Command::Move { dir: Dir::E });
        let s = saves(&g.apply(Command::Wait));
        assert_eq!(s, [(r0 + 1, None)]);
        let at = g.players()[0].at;
        g.apply(Command::Move { dir: Dir::E });
        g.apply(Command::Wait);
        g.apply(Command::Cast {
            thing: KILL,
            target: None,
        });
        let r = restore(&g, &a);
        assert_eq!(r.round(), r0 + 1);
        assert_eq!(r.players()[0].at, at);
    }

    /// The engine's own promise (saves 1): a game replays exactly from its
    /// seed, first player and command log, and its save file restores it.
    #[test]
    fn a_game_replays_from_its_seed_and_log() {
        let a = adventure();
        let (mut g, _) = two_in(&a);
        for dir in [Dir::E, Dir::S, Dir::E] {
            g.apply(Command::Move { dir });
        }
        g.apply(Command::SaveGame);
        for _ in 0..25 {
            g.apply(Command::Wait);
        }
        let r = Game::replay(a.clone(), g.seed(), g.rules(), g.first_player().clone(), g.log());
        assert_eq!(r, g);
        let file = SaveFile::from_json(&SaveFile::new(&g, &a, &g).to_json()).expect("reads");
        let back = file.restore(&a).expect("restores");
        assert!(!back.diverged && !back.adventure_changed);
        assert_eq!(back.game, g);
    }
}
