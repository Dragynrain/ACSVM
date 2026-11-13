// SPDX-License-Identifier: GPL-3.0-or-later

//! Inventory rules (`docs/rules/inventory.md`) on a synthetic adventure built
//! byte by byte with acs-core's fixture builder. No real adventure data.

#[path = "../../acs-core/tests/common/mod.rs"]
mod common;

use acs_core::layout::world_map as wm;
use acs_core::model::SpellActionKind;
use acs_core::{decode, Adventure, Platform};
use acs_engine::inventory::{self, slowed_speed, GOLD};
use acs_engine::{
    BlockReason, Command, Dir, DropRefusal, Event, Game, Location, PlayerSetup, ReadyRefusal, Rules, Slot,
    SpellSource,
};
use common::{Fixture, RegionSpec, RoomSpec};

// Master things (1-based). 1..32 may be owned 3 times, 33..80 once.
const SWORD: u16 = 1; // melee weapon, weight 5
const BOW: u16 = 2; // missile weapon
const CLAWS: u16 = 3; // melee weapon, usable only by its owner
const SHIELD: u16 = 4; // armor, weight 10
const ROCK: u16 = 5; // treasure, weight 200
const BAG: u16 = 6; // treasure, weight -1000 (50 x -20)
const DUST: u16 = 7; // treasure, disappears when dropped
const AMULET: u16 = 8; // magic item, drop spell (long message 3)
const CHARM: u16 = 9; // magic spell
const CROWN: u16 = 40; // treasure, one per owner
const SLOT: u16 = 82; // custom space, when GOLD dropped here: add CROWN, destroys
const BIN: u16 = 83; // custom space, when ROCK dropped here: rid room, keeps it
const PILLAR: u16 = 84; // obstacle

/// One region with one room, HALL (10 x 6, walls included):
///
/// ```text
///   y=1: . SWORD BOW ROCK AMULET SLOT BIN PILLAR .
///   y=2: . GOLD(30) CROWN DUST . . . . .
/// ```
fn fixture(platform: Platform) -> Fixture {
    let mut f = Fixture::new(platform);
    f.header("CARRY", "", "");
    // Weapons and armor: p3 = extended record; extended e2 bit 6 = usable by anyone.
    f.thing(SWORD as usize, "SWORD", [0x04, 5, 10, 0]);
    f.extended(None, 0, [5, 0, 20 | 0x40]);
    f.thing(BOW as usize, "BOW", [0x03, 3, 10, 1]);
    f.extended(None, 1, [3, 0, 21 | 0x40]);
    f.thing(CLAWS as usize, "CLAWS", [0x04, 0, 0, 2]);
    f.extended(None, 2, [4, 0, 22]);
    f.thing(SHIELD as usize, "SHIELD", [0x05, 10, 10, 3]);
    f.extended(None, 3, [0, 0, 23 | 0x40]);
    f.thing(ROCK as usize, "ROCK", [0x01, 200, 1, 4]);
    f.thing(BAG as usize, "BAG", [0x61, 50, 1, 5]);
    f.thing(DUST as usize, "DUST", [0x11, 1, 1, 6]);
    // Magic item: extended (magic table at 12) e0 = action, e1 = picture |
    // on drop (bit 6), e2 = parameter.
    f.thing(AMULET as usize, "AMULET", [0x02, 1, 1, 0]);
    f.extended(Some(12), 0, [10, 7 | 0x40, 3]);
    f.thing(CHARM as usize, "CHARM", [0x06, 5, 0, 0]);
    f.thing(CROWN as usize, "CROWN", [0x01, 2, 100, 8]);
    f.thing(GOLD as usize, "GOLD", [0x01, 1, 1, 9]);
    // Custom spaces: placement chooses the thing (bit 7 clear), trigger 0 =
    // when dropped here; p2 = spell | destroy (bit 7), p3 = fixed parameter.
    f.thing(SLOT as usize, "SLOT", [0x09, 10, 13 | 0x80, (CROWN as u8)]);
    f.thing(BIN as usize, "BIN", [0x09, 10, 12, 0]);
    f.thing(PILLAR as usize, "PILLAR", [0x0A, 11, 0, 0]);
    f.thing(127, "EMPTY FLOOR", [0x00, 1, 0, 0]);
    f.thing(128, "ROOM BORDER", [0x00, 12, 0, 0]);
    f.region(RegionSpec {
        name: "VAULT",
        rooms: vec![RoomSpec {
            name: "HALL",
            w: 10,
            h: 6,
            wall: 12,
            items: vec![
                (2, 1, (SWORD as u8), 1),
                (3, 1, (BOW as u8), 1),
                (4, 1, (ROCK as u8), 1),
                (5, 1, (AMULET as u8), 1),
                (6, 1, (SLOT as u8), (GOLD as u8)),
                (7, 1, (BIN as u8), (ROCK as u8)),
                (8, 1, (PILLAR as u8), 0),
                (2, 2, (GOLD as u8), 30),
                (3, 2, (CROWN as u8), 1),
                (4, 2, (DUST as u8), 1),
            ],
            ..Default::default()
        }],
        portals: vec![[0, 0]],
        ..Default::default()
    });
    let w = f.l.world_map;
    f.put(w + wm::TERRAIN, &[1 | 1 << 6, 0]); // terrain 0: open
    f.put(w + wm::START_X, &[20, 20]);
    f
}

fn adventure(platform: Platform) -> Adventure {
    decode(&fixture(platform).build()).expect("decodes").adventure
}

fn setup() -> PlayerSetup {
    PlayerSetup {
        speed: 15,
        ..Default::default()
    }
}

fn game_with(platform: Platform, setup: PlayerSetup) -> Game {
    Game::new(adventure(platform), 1, Rules { diagonals: true }, setup).0
}

fn game(platform: Platform) -> Game {
    game_with(platform, setup())
}

fn hall(x: u8, y: u8) -> Location {
    Location::Room {
        region: 1,
        room: 0,
        x,
        y,
    }
}

fn both(test: impl Fn(Platform)) {
    test(Platform::Pc);
    test(Platform::C64);
}

fn mv(g: &mut Game, dir: Dir) -> Vec<Event> {
    g.apply(Command::Move { dir })
}

fn drop(g: &mut Game, thing: u16, dir: Option<Dir>) -> Vec<Event> {
    g.apply(Command::Drop { thing, dir })
}

fn refused(ev: &[Event]) -> Option<DropRefusal> {
    ev.iter().find_map(|e| match e {
        Event::DropRefused { reason, .. } => Some(*reason),
        _ => None,
    })
}

/// Things lying on a HALL square, top first.
/// Each item's thing and parameter; a gold pile's size in place of the
/// parameter when it has one (5.8).
fn stack(g: &Game, x: u8, y: u8) -> Vec<(u16, u16)> {
    g.adventure().regions[0].rooms[0]
        .items
        .iter()
        .filter(|i| (i.x, i.y) == (x, y))
        .map(|i| (i.thing, i.gold.unwrap_or(i.parameter as u16)))
        .collect()
}

#[test]
fn ownership_limits_follow_the_thing_number() {
    assert_eq!(inventory::limit(1), 3);
    assert_eq!(inventory::limit(32), 3);
    assert_eq!(inventory::limit(33), 1);
    assert_eq!(inventory::limit(80), 1);
    assert_eq!(inventory::limit(GOLD), 25_000);
    assert_eq!(inventory::limit(100), 1);
}

#[test]
fn at_the_limit_the_move_is_refused_for_free() {
    both(|p| {
        let mut g = game(p);
        for _ in 0..3 {
            g.give(0, ROCK);
        }
        g.place(0, hall(5, 2));
        let before = g.current_player().moves_left;
        let ev = mv(&mut g, Dir::NW);
        assert!(matches!(
            ev[0],
            Event::Blocked {
                reason: BlockReason::CarryLimit { thing: ROCK },
                ..
            }
        ));
        assert_eq!(g.current_player().at, hall(5, 2));
        assert_eq!(g.current_player().moves_left, before);
        assert_eq!(stack(&g, 4, 1), vec![(ROCK, 1)]);

        // Two rocks: the third can be picked up.
        let mut g = game(p);
        g.give(0, ROCK);
        g.give(0, ROCK);
        g.place(0, hall(5, 2));
        mv(&mut g, Dir::NW);
        assert_eq!(g.current_player().count(ROCK), 3);

        // Things 33..80: only one.
        let mut g = game(p);
        g.give(0, CROWN);
        g.place(0, hall(3, 3));
        assert!(matches!(
            mv(&mut g, Dir::N)[0],
            Event::Blocked {
                reason: BlockReason::CarryLimit { thing: CROWN },
                ..
            }
        ));
    });
}

#[test]
fn gold_piles_add_to_the_count_up_to_25000() {
    both(|p| {
        let mut g = game(p);
        g.place(0, hall(2, 3));
        let ev = mv(&mut g, Dir::N);
        assert_eq!(
            ev[0],
            Event::GoldPickedUp {
                player: 0,
                amount: 30
            }
        );
        assert_eq!(g.current_player().gold, 30);
        assert!(g.current_player().owns(GOLD));
        assert!(g.current_player().possessions.is_empty());
        assert!(stack(&g, 2, 2).is_empty());

        // Near the cap: take what fits, leave the rest. (The gold comes
        // after the turn started: that much is far past the capacity, which
        // only counts from the next turn.)
        let mut g = game(p);
        for _ in 0..24_990 {
            g.give(0, GOLD);
        }
        g.place(0, hall(2, 3));
        let ev = mv(&mut g, Dir::N);
        assert_eq!(
            ev[0],
            Event::GoldPickedUp {
                player: 0,
                amount: 10
            }
        );
        assert_eq!(g.current_player().gold, 25_000);
        assert_eq!(stack(&g, 2, 2), vec![(GOLD, 20)]);
        // Full: the pile refuses the move.
        g.place(0, hall(2, 3));
        assert!(matches!(
            mv(&mut g, Dir::N)[0],
            Event::Blocked {
                reason: BlockReason::CarryLimit { thing: GOLD },
                ..
            }
        ));
    });
}

#[test]
fn picking_up_a_weapon_readies_it_and_armor_takes_the_turn() {
    both(|p| {
        let mut g = game(p);
        g.place(0, hall(1, 1));
        let ev = mv(&mut g, Dir::E);
        assert_eq!(
            &ev[..2],
            &[
                Event::PickedUp {
                    player: 0,
                    thing: SWORD
                },
                Event::Readied {
                    player: 0,
                    thing: SWORD,
                    slot: Slot::Weapon
                },
            ]
        );
        // The taker stays put: step onto the sword's square, then take the
        // bow, which replaces the sword.
        assert_eq!(g.current_player().at, hall(1, 1));
        mv(&mut g, Dir::E);
        mv(&mut g, Dir::E);
        assert_eq!(g.current_player().weapon, Some(BOW));
        // Ready the sword again: free.
        let left = g.current_player().moves_left;
        let ev = g.apply(Command::Ready { thing: SWORD });
        assert_eq!(
            ev,
            vec![Event::Readied {
                player: 0,
                thing: SWORD,
                slot: Slot::Weapon
            }]
        );
        assert_eq!(g.current_player().moves_left, left);

        // Refusals cost nothing.
        let ev = g.apply(Command::Ready { thing: SHIELD });
        assert_eq!(
            ev,
            vec![Event::ReadyRefused {
                player: 0,
                thing: SHIELD,
                reason: ReadyRefusal::NotCarried
            }]
        );
        g.give(0, ROCK);
        assert!(matches!(
            g.apply(Command::Ready { thing: ROCK })[0],
            Event::ReadyRefused {
                reason: ReadyRefusal::NotWeaponOrArmor,
                ..
            }
        ));

        // Armor: readied, and the turn is over.
        g.give(0, SHIELD);
        let ev = g.apply(Command::Ready { thing: SHIELD });
        assert_eq!(
            ev[0],
            Event::Readied {
                player: 0,
                thing: SHIELD,
                slot: Slot::Armor
            }
        );
        assert!(matches!(ev[1], Event::TurnEnded { player: 0 }));
        assert!(matches!(ev[2], Event::TurnStarted { player: 0, .. }));
        assert_eq!(g.current_player().armor, Some(SHIELD));
    });
}

#[test]
fn capacity_follows_the_manual_formula() {
    let a = adventure(Platform::Pc);
    let (g, _) = Game::new(a.clone(), 1, Rules { diagonals: true }, setup());
    assert_eq!(g.current_player().capacity(), 1100);
    let wounded = PlayerSetup {
        constitution: 11,
        ..setup()
    };
    let (mut g, _) = Game::new(a.clone(), 1, Rules { diagonals: true }, wounded);
    g.set_life_force(0, 1);
    assert_eq!(g.current_player().capacity(), 600); // 11 - 10/2
    let hale = PlayerSetup {
        constitution: 10,
        life_force: 20,
        ..setup()
    };
    let (g, _) = Game::new(a, 1, Rules { diagonals: true }, hale);
    assert_eq!(g.current_player().capacity(), 1600); // 11 + 10/2

    // A straight line from half the capacity (full Speed) to the capacity
    // (0), rounding down: the points captured on the C64 (inventory 3.3).
    for (speed, load, cap, want) in [
        (15, 500, 1000, 15),
        (15, 501, 1000, 14),
        (15, 600, 1000, 12),
        (15, 700, 1000, 9),
        (15, 800, 1000, 6),
        (15, 1000, 1000, 0),
        (10, 750, 1500, 10),
        (10, 751, 1500, 9),
        (10, 900, 1500, 8),
        (10, 1050, 1500, 6),
        (10, 1200, 1500, 4),
        (10, 1350, 1500, 2),
        (10, 1500, 1500, 0),
    ] {
        assert_eq!(slowed_speed(speed, load, cap), want, "{speed} {load} {cap}");
    }
    assert_eq!(slowed_speed(15, 1101, 1100), 0);
    assert_eq!(slowed_speed(1, 600, 1100), 0);
    assert_eq!(slowed_speed(0, 0, 1100), 0);
    assert_eq!(slowed_speed(5, 0, 0), 5);
    assert_eq!(slowed_speed(5, 1, 0), 0);
}

#[test]
fn weight_slows_the_next_turn_and_negative_weight_helps() {
    both(|p| {
        let mut g = game(p);
        for _ in 0..3 {
            g.give(0, ROCK);
        }
        assert_eq!(g.current_player().load(g.adventure()), 600);
        assert_eq!(g.current_player().moves_left, 15, "this turn is not affected");
        let ev = g.apply(Command::Wait);
        // 15 x 2 x (1100 - 600) / 1100 = 13.6
        assert!(matches!(ev[1], Event::TurnStarted { moves: 13, .. }));
        assert_eq!(g.current_player().turn_moves, 13);

        g.give(0, BAG);
        assert_eq!(g.current_player().load(g.adventure()), -400);
        let ev = g.apply(Command::Wait);
        assert!(matches!(ev[1], Event::TurnStarted { moves: 15, .. }));

        // Gold weighs too: 1 a piece. Past the capacity: no moves at all.
        let g = game_with(
            p,
            PlayerSetup {
                gold: 1101,
                ..setup()
            },
        );
        assert_eq!(g.current_player().moves_left, 0);
    });
}

#[test]
fn drops_land_on_top_beside_or_under_the_player() {
    both(|p| {
        let mut g = game(p);
        g.give(0, ROCK);
        g.give(0, ROCK);
        g.place(0, hall(5, 3));
        let left = g.current_player().moves_left;
        let ev = drop(&mut g, ROCK, None);
        assert_eq!(
            ev,
            vec![Event::Dropped {
                player: 0,
                thing: ROCK,
                at: hall(5, 3)
            }]
        );
        assert_eq!(g.current_player().count(ROCK), 1);
        assert_eq!(g.current_player().moves_left, left, "dropping is free");
        // Onto the dust: the rock is now on top.
        drop(&mut g, ROCK, Some(Dir::NW));
        assert_eq!(stack(&g, 4, 2), vec![(ROCK, 1), (DUST, 1)]);
        // Walking onto it picks the rock up, not the dust.
        let ev = mv(&mut g, Dir::NW);
        assert_eq!(
            ev[0],
            Event::PickedUp {
                player: 0,
                thing: ROCK
            }
        );
        assert_eq!(stack(&g, 4, 2), vec![(DUST, 1)]);
    });
}

#[test]
fn drops_that_are_refused() {
    both(|p| {
        let mut g = game(p);
        g.give(0, ROCK);
        g.give(0, CHARM);
        g.give(0, CLAWS);
        g.place(0, hall(8, 2));
        assert_eq!(refused(&drop(&mut g, SWORD, None)), Some(DropRefusal::NotCarried));
        assert_eq!(refused(&drop(&mut g, CHARM, None)), Some(DropRefusal::MagicSpell));
        assert_eq!(refused(&drop(&mut g, CLAWS, None)), Some(DropRefusal::OwnerOnly));
        assert_eq!(
            refused(&drop(&mut g, ROCK, Some(Dir::N))),
            Some(DropRefusal::Thing { thing: PILLAR })
        );
        assert_eq!(
            refused(&drop(&mut g, ROCK, Some(Dir::E))),
            Some(DropRefusal::Wall)
        );
        assert_eq!(
            refused(&g.apply(Command::DropGold { amount: 0, dir: None })),
            Some(DropRefusal::BadAmount)
        );
        assert_eq!(
            refused(&g.apply(Command::DropGold { amount: 5, dir: None })),
            Some(DropRefusal::NotCarried)
        );
        let p0 = g.current_player();
        assert_eq!((p0.count(ROCK), p0.count(CHARM), p0.count(CLAWS)), (1, 1, 1));
    });
}

#[test]
fn vanishing_treasure_drop_spells_and_the_world_map() {
    both(|p| {
        let mut g = game(p);
        g.give(0, DUST);
        g.give(0, AMULET);
        g.give(0, SWORD);
        g.apply(Command::Ready { thing: SWORD });
        g.place(0, hall(5, 3));
        assert_eq!(
            drop(&mut g, DUST, None),
            vec![Event::VanishedWhenDropped {
                player: 0,
                thing: DUST
            }]
        );
        assert!(stack(&g, 5, 3).is_empty());

        let ev = drop(&mut g, AMULET, Some(Dir::S));
        assert_eq!(
            ev[0],
            Event::Dropped {
                player: 0,
                thing: AMULET,
                at: hall(5, 4)
            }
        );
        assert!(matches!(
            ev[1],
            Event::SpellTriggered { source: SpellSource::Dropped, thing: AMULET, action, .. }
                if action.kind == SpellActionKind::LongMessage && action.parameter == 3
        ));

        // Dropping the readied sword on the map loses it and unreadies it.
        g.place(0, Location::World { x: 20, y: 20 });
        assert_eq!(
            drop(&mut g, SWORD, Some(Dir::E)),
            vec![Event::LostOnMap {
                player: 0,
                thing: SWORD
            }]
        );
        assert_eq!(g.current_player().weapon, None);
        let ev = g.apply(Command::DropGold { amount: 1, dir: None });
        assert_eq!(refused(&ev), Some(DropRefusal::NotCarried));
    });
}

#[test]
fn when_dropped_here_spaces_make_vending_machines() {
    both(|p| {
        let mut g = game_with(p, PlayerSetup { gold: 10, ..setup() });
        g.give(0, ROCK);
        g.give(0, SWORD);
        g.place(0, hall(6, 2));
        // A coin into the slot: its spell, and the coin is gone.
        let ev = g.apply(Command::DropGold {
            amount: 1,
            dir: Some(Dir::N),
        });
        assert!(matches!(
            ev[0],
            Event::SpellTriggered { source: SpellSource::CustomSpace, thing: SLOT, action, .. }
                if action.kind == SpellActionKind::AddToRoom && action.parameter == CROWN
        ));
        // Magic 3.13: the crown lands on top of the slot.
        assert_eq!(
            ev[1],
            Event::ThingAdded {
                thing: CROWN,
                at: hall(6, 1)
            }
        );
        assert_eq!(
            ev[2],
            Event::ThingUsedUp {
                player: 0,
                thing: GOLD
            }
        );
        assert_eq!(g.current_player().gold, 9);
        assert_eq!(stack(&g, 6, 1), vec![(CROWN, 1), (SLOT, GOLD)]);

        // The bin keeps what it is given and casts once for the rock.
        g.place(0, hall(7, 2));
        let ev = drop(&mut g, ROCK, Some(Dir::N));
        assert!(matches!(
            ev[0],
            Event::SpellTriggered { thing: BIN, action, .. } if action.kind == SpellActionKind::RidRoomOf
        ));
        // Rid Room runs before the rock lands, so the rock stays.
        assert!(matches!(ev[1], Event::RoomRidOf { count: 0, .. }));
        assert_eq!(
            ev[2],
            Event::Dropped {
                player: 0,
                thing: ROCK,
                at: hall(7, 1)
            }
        );
        assert_eq!(stack(&g, 7, 1), vec![(ROCK, 1), (BIN, ROCK)]);
        // Something else just lands on top.
        g.place(0, hall(6, 2));
        let ev = drop(&mut g, SWORD, Some(Dir::N));
        assert_eq!(
            ev,
            vec![Event::Dropped {
                player: 0,
                thing: SWORD,
                at: hall(6, 1)
            }]
        );

        // A gold pile on the floor is picked up again by walking onto it.
        g.place(0, hall(5, 3));
        let ev = g.apply(Command::DropGold {
            amount: 4,
            dir: Some(Dir::E),
        });
        assert_eq!(
            ev,
            vec![Event::GoldDropped {
                player: 0,
                amount: 4,
                at: Some(hall(6, 3))
            }]
        );
        assert_eq!(stack(&g, 6, 3), vec![(GOLD, 4)]);
        mv(&mut g, Dir::E);
        assert_eq!(g.current_player().gold, 9);
    });
}

#[test]
fn inventory_commands_replay_exactly() {
    both(|p| {
        let a = adventure(p);
        let (mut g2, _) = Game::new(a.clone(), 5, Rules { diagonals: true }, setup());
        for c in [
            Command::Drop {
                thing: SWORD,
                dir: None,
            },
            Command::Ready { thing: SWORD },
            Command::DropGold { amount: 3, dir: None },
            Command::Move { dir: Dir::N },
            Command::Wait,
        ] {
            g2.apply(c);
        }
        let r = Game::replay(a, 5, Rules { diagonals: true }, setup(), g2.log());
        assert_eq!(r, g2);
    });
}
