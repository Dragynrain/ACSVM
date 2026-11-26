// SPDX-License-Identifier: GPL-3.0-or-later

//! Magic rules (`docs/rules/magic.md`) on a synthetic adventure built with
//! acs-core's fixture builder, then given its spells as model values. No
//! real adventure data.

#[path = "../../acs-core/tests/common/mod.rs"]
mod common;

use acs_core::model::{
    Access, Alignment, Carry, CustomTrigger, MagicDefense, PlacementChoice, Possession, RoomItem,
    SpellAction, SpellActionKind as K, ThingKind, WeaponStats,
};
use acs_core::{decode, Adventure, Platform};
use acs_engine::inventory::GOLD;
use acs_engine::magic::{cast_chance, cast_roll, cast_spend};
use acs_engine::{
    describe, BlockReason, CastRefusal, Command, Dir, Event, Game, Location, PlayerSetup, RemovalReason,
    Rules, SpellSource, Stat, Who,
};
use common::{creature_record, Fixture, RegionSpec, RoomSpec};

const GEM: u16 = 1; // treasure
const AMULET: u16 = 2; // treasure, the Kill All key
const DUST: u16 = 3; // treasure that vanishes when dropped
const CLAW: u16 = 4; // owner-only melee weapon
const BOLT: u16 = 33; // magic spell: Life Force -6, cost 7
const HEAL: u16 = 34; // magic spell: Life Force +5, cost 0, disappears
const CALL: u16 = 35; // magic spell: summon creature 5, cost 0
const BANISH: u16 = 36; // magic spell: banish all, cost 0
const DOOM: u16 = 37; // magic spell: kill all but owners of AMULET, cost 0
const POTION: u16 = 40; // magic item, used: Life Force +4, disappears
const ROD: u16 = 41; // magic item, used: Life Force -3
const STONE: u16 = 42; // magic item, not usable
const BOMB: u16 = 43; // magic item, dropped: Life Force -2
const DOOR: u16 = 82; // portal, anyone
const SONG: u16 = 83; // portal, invokes Play Music 3
const DOALL: u16 = 84; // space, Activate All
const LIGHT: u16 = 85; // custom space, owners of (placement), Life Force +5
const DARK: u16 = 86; // custom space, non-owners of (placement), Do Nothing
const TUNE: u16 = 87; // space, Play Music (stored 12 = choice 13)
const SLOT: u16 = 88; // custom space, when dropped here, Give GEM
const DOALL2: u16 = 89; // space, Activate All (nested)
const STAT: u16 = 90; // custom space, Increase stat (placement)
const LOWER: u16 = 91; // custom space, Decrease stat (placement)
const WARD: u16 = 92; // space, raise magic defense up to 2
const DISEASE_UP: u16 = 93; // space, Life Force +10 temporarily
const DISEASE_DOWN: u16 = 94; // space, Life Force -10 permanently
const GIFT: u16 = 95; // space, Give GEM
const SCROLL: u16 = 96; // space, long message 7
const CLEAN: u16 = 97; // space, rid room of GEM
const CHEST: u16 = 98; // obstacle, bump: Add to Room GEM
const IMP: u16 = 5; // master creature

fn act(kind: K, parameter: u16) -> SpellAction {
    SpellAction { kind, parameter }
}

fn life(n: i8, temporary: bool) -> u16 {
    let mut p = n.unsigned_abs() & 0x3F;
    if n < 0 {
        p |= 0x80;
    }
    if temporary {
        p |= 0x40;
    }
    u16::from(p)
}

fn spell(cost: u8, action: SpellAction, disappears: bool) -> ThingKind {
    ThingKind::MagicSpell {
        disappears_after_use: disappears,
        power: cost,
        action,
    }
}

fn item(action: SpellAction, on_use: bool, on_drop: bool, disappears: bool) -> ThingKind {
    ThingKind::MagicItem {
        picture: 3,
        disappears_after_use: disappears,
        carry: Carry { weight: 1, value: 1 },
        action,
        on_use,
        on_drop,
        on_pickup: false,
    }
}

fn space(action: SpellAction) -> ThingKind {
    ThingKind::Space {
        picture: 4,
        access: Access::InvokeSpell { action },
    }
}

fn custom(trigger: CustomTrigger, choice: PlacementChoice, action: SpellAction) -> ThingKind {
    ThingKind::CustomSpace {
        picture: 4,
        placement_choice: choice,
        trigger,
        action,
        destroy_needed_thing: false,
    }
}

/// One region "DEN" with HALL (10 x 7, inside 1..8 x 1..5) and SIDE behind
/// a door at HALL (9,3) <-> SIDE (0,3), and a music door at HALL (9,5) to
/// SIDE (1,5). Slot 0 is a template creature, not in use.
fn adventure(platform: Platform) -> Adventure {
    let mut f = Fixture::new(platform);
    f.header("DEN", "", "");
    f.thing(GEM as usize, "GEM", [0x01, 1, 50, 5]);
    // Doors must be portals when decoded, so their destinations resolve.
    f.thing(DOOR as usize, "DOOR", [0x27, 5, 0, 0]);
    f.thing(SONG as usize, "SONG", [0x07, 5 | 1 << 6, 11, 2]);
    f.thing(127, "EMPTY FLOOR", [0x00, 1, 0, 0]);
    f.thing(128, "ROOM BORDER", [0x00, 12, 0, 0]);
    f.long_message(7, "AN OLD SCROLL");
    f.long_message(8, &split_message());
    f.creature(IMP as usize, "IMP", 0, creature_record(6, 0x60, 70));
    let mut template = creature_record(10, 0x60, 70);
    template[0x10] = 0x10;
    f.region(RegionSpec {
        name: "DEN",
        rooms: vec![
            RoomSpec {
                name: "HALL",
                w: 10,
                h: 7,
                wall: 12,
                items: vec![(9, 3, (DOOR as u8), 1), (9, 5, (SONG as u8), 3)],
                ..Default::default()
            },
            RoomSpec {
                name: "SIDE",
                w: 10,
                h: 7,
                x: 9,
                wall: 12,
                items: vec![(0, 3, (DOOR as u8), 2)],
                ..Default::default()
            },
        ],
        portals: vec![[0, 0], [0x11, 0x13], [0x10, 0x83], [0x11, 0x15]],
        residents: vec![(0, "TEMPLATE", template)],
        ..Default::default()
    });
    let mut a = decode(&f.build()).expect("decodes").adventure;
    a.regions[0].residents[0].in_use = false;
    let mut set = |n: u8, name: &str, kind: ThingKind| {
        let t = &mut a.things[n as usize - 1];
        t.name = name.into();
        t.kind = kind;
        t.raw = [0; 4];
    };
    let weapon = WeaponStats {
        power: 3,
        range: None,
        break_chance: 0,
        attack_adjustment: 0,
        usable_by_anyone: false,
        magic: false,
    };
    set(
        AMULET as u8,
        "AMULET",
        ThingKind::Treasure {
            picture: 5,
            disappears_when_dropped: false,
            carry: Carry { weight: 1, value: 9 },
        },
    );
    set(
        DUST as u8,
        "DUST",
        ThingKind::Treasure {
            picture: 5,
            disappears_when_dropped: true,
            carry: Carry { weight: 0, value: 1 },
        },
    );
    set(
        CLAW as u8,
        "CLAW",
        ThingKind::MeleeWeapon {
            picture: 6,
            disappears_after_use: false,
            carry: Carry { weight: 0, value: 0 },
            stats: weapon,
        },
    );
    set(
        BOLT as u8,
        "BOLT",
        spell(7, act(K::ChangeLifeForce, life(-6, false)), false),
    );
    set(
        HEAL as u8,
        "HEAL",
        spell(0, act(K::ChangeLifeForce, life(5, false)), true),
    );
    set(CALL as u8, "CALL", spell(0, act(K::SummonOrBanish, IMP), false));
    set(BANISH as u8, "BANISH", spell(0, act(K::SummonOrBanish, 0), false));
    set(
        DOOM as u8,
        "DOOM",
        spell(0, act(K::KillAllButOwnerOf, AMULET), false),
    );
    set(
        POTION as u8,
        "POTION",
        item(act(K::ChangeLifeForce, life(4, false)), true, false, true),
    );
    set(
        ROD as u8,
        "ROD",
        item(act(K::ChangeLifeForce, life(-3, false)), true, false, false),
    );
    set(
        STONE as u8,
        "STONE",
        item(act(K::Nothing, 0), false, false, false),
    );
    set(
        BOMB as u8,
        "BOMB",
        item(act(K::ChangeLifeForce, life(-2, false)), false, true, false),
    );
    set(
        SONG as u8,
        "SONG",
        ThingKind::Portal {
            picture: 5,
            two_way: false,
            access: Access::InvokeSpell {
                action: act(K::PlayMusic, 2),
            },
            destroy_needed_thing: false,
        },
    );
    set(DOALL as u8, "DOALL", space(act(K::ActivateAll, 0)));
    set(
        LIGHT as u8,
        "LIGHT",
        custom(
            CustomTrigger::OpenToOwnersOf,
            PlacementChoice::Thing,
            act(K::ChangeLifeForce, life(5, false)),
        ),
    );
    set(
        DARK as u8,
        "DARK",
        custom(
            CustomTrigger::OpenToNonOwnersOf,
            PlacementChoice::Thing,
            act(K::Nothing, 0),
        ),
    );
    set(TUNE as u8, "TUNE", space(act(K::PlayMusic, 12)));
    set(
        SLOT as u8,
        "SLOT",
        custom(
            CustomTrigger::WhenDroppedHere,
            PlacementChoice::Thing,
            act(K::Give, GEM),
        ),
    );
    set(DOALL2 as u8, "DOALL2", space(act(K::ActivateAll, 0)));
    set(
        STAT as u8,
        "STAT",
        custom(
            CustomTrigger::WhenMovedOnto,
            PlacementChoice::SpellParameter,
            act(K::IncreaseStat, 0),
        ),
    );
    set(
        LOWER as u8,
        "LOWER",
        custom(
            CustomTrigger::WhenMovedOnto,
            PlacementChoice::SpellParameter,
            act(K::DecreaseStat, 0),
        ),
    );
    set(WARD as u8, "WARD", space(act(K::IncreaseMagicDefense, 2)));
    set(
        DISEASE_UP as u8,
        "FEVER",
        space(act(K::ChangeLifeForce, life(10, true))),
    );
    set(
        DISEASE_DOWN as u8,
        "CHILL",
        space(act(K::ChangeLifeForce, life(-10, false))),
    );
    set(GIFT as u8, "GIFT", space(act(K::Give, GEM)));
    set(SCROLL as u8, "SCROLL", space(act(K::LongMessage, 7)));
    set(CLEAN as u8, "CLEAN", space(act(K::RidRoomOf, GEM)));
    set(
        CHEST as u8,
        "CHEST",
        ThingKind::Obstacle {
            picture: 7,
            bump_invokes_spell: true,
            action: act(K::AddToRoom, GEM),
        },
    );
    a
}

fn put_item(a: &mut Adventure, room: usize, x: u8, y: u8, thing: u16, parameter: u8) {
    a.regions[0].rooms[room].items.insert(
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

/// Puts a creature in slot `slot` of HALL: an enemy fighter by default.
fn put_creature(
    a: &mut Adventure,
    slot: usize,
    x: u8,
    y: u8,
    f: impl FnOnce(&mut acs_core::model::Creature),
) {
    let mut c = a.regions[0].residents[0].clone();
    c.in_use = true;
    c.name = format!("C{slot}");
    c.number = u16::from(slot as u8 + 1);
    c.region = 1;
    c.room = 0;
    c.x = x;
    c.y = y;
    c.speed = 0;
    c.strategy.alignment = Alignment::Enemy;
    c.strategy.aggressive = true;
    c.strategy.brave = true;
    c.wisdom = 3; // dumb: casts nothing unless a test says so
    c.possessions.clear();
    c.weapon = 0;
    c.armor = 0;
    f(&mut c);
    a.regions[0].residents[slot] = c;
}

fn setup() -> PlayerSetup {
    PlayerSetup {
        speed: 15,
        ..Default::default()
    }
}

fn start(a: Adventure, x: u8, y: u8, setup: PlayerSetup) -> Game {
    // Tests ask for wounded players; joining raises Life Force to
    // Constitution (5.1), so set it afterwards.
    let life_force = setup.life_force;
    let (mut g, _) = Game::new(a, 5, Rules { diagonals: true }, setup);
    g.set_life_force(0, life_force);
    g.place(0, hall(x, y));
    g
}

fn hall(x: u8, y: u8) -> Location {
    Location::Room {
        region: 1,
        room: 0,
        x,
        y,
    }
}

const ME: Who = Who::Player { player: 0 };

fn cr(slot: usize) -> Who {
    Who::Creature { region: 1, slot }
}

fn lf(g: &Game, who: Who) -> u8 {
    g.stat(who, Stat::LifeForce).expect("alive")
}

fn mv(g: &mut Game, d: Dir) -> Vec<Event> {
    g.apply(Command::Move { dir: d })
}

fn stack(g: &Game, room: usize, x: u8, y: u8) -> Vec<u16> {
    g.adventure().regions[0].rooms[room]
        .items
        .iter()
        .filter(|it| (it.x, it.y) == (x, y))
        .map(|it| it.thing)
        .collect()
}

fn both(test: impl Fn(Platform)) {
    test(Platform::Pc);
    test(Platform::C64);
}

#[test]
fn cast_chance_is_90_power_over_cost_held_to_14_90() {
    // magic 4.1 [research], confirmed by a 256-state capture: the
    // researcher's and the capture's pairs.
    assert_eq!(cast_chance(0, 0), 100);
    for (power, cost, chance) in [
        (20, 10, 90),
        (20, 20, 90),
        (20, 30, 60),
        (20, 40, 45),
        (20, 60, 30),
        (20, 90, 20),
        (10, 20, 45),
        (27, 31, 78),
        (27, 62, 39),
        (5, 100, 14),
        (1, 127, 14),
        (0, 10, 14),
        (127, 1, 90),
    ] {
        assert_eq!(cast_chance(power, cost), chance, "{power}/{cost}");
    }
}

#[test]
fn the_game_answers_a_spells_chance_for_the_casters_power() {
    both(|p| {
        let mut g = start(adventure(p), 2, 2, setup());
        g.set_power(0, 20);
        // BOLT costs 7: 90 x 7 / 7; HEAL costs 0; a treasure is no spell.
        assert_eq!(g.cast_chance(0, BOLT), Some(90));
        assert_eq!(g.cast_chance(0, HEAL), Some(100));
        assert_eq!(g.cast_chance(0, GEM), None);
        g.set_power(0, 3);
        assert_eq!(g.cast_chance(0, BOLT), Some(cast_chance(3, 7)));
        assert_eq!(g.cast_chance(1, BOLT), None);
    });
}

#[test]
fn a_cast_spends_the_cost_or_all_power() {
    assert_eq!(cast_spend(20, 10), 10);
    assert_eq!(cast_spend(20, 30), 20);
    assert_eq!(cast_spend(0, 10), 0);
    assert_eq!(cast_spend(20, 0), 0);
}

#[test]
fn cast_roll_matches_its_chance() {
    let mut rng = acs_engine::Rng::new(7);
    for (power, cost) in [(20u8, 20u8), (20, 60), (10, 20), (0, 5), (5, 0)] {
        let n = 20_000;
        let hits = (0..n).filter(|_| cast_roll(&mut rng, power, cost)).count();
        let want = cast_chance(power, cost) as f64 / 100.0;
        assert!(
            (hits as f64 / n as f64 - want).abs() < 0.015,
            "{power}/{cost}: {hits}"
        );
    }
}

#[test]
fn casting_pays_power_ends_the_turn_and_can_disappear() {
    both(|p| {
        let mut a = adventure(p);
        put_creature(&mut a, 1, 5, 3, |_| {});
        let mut g = start(
            a,
            2,
            2,
            PlayerSetup {
                wisdom: 20, // Power starts at Wisdom (5.1)
                life_force: 6,
                possessions: vec![BOLT, HEAL, STONE, GEM],
                ..setup()
            },
        );
        // Refusals cost nothing and keep the turn.
        let refused = |ev: &[Event]| match ev {
            [Event::CastRefused { reason, .. }] => Some(*reason),
            _ => None,
        };
        assert_eq!(
            refused(&g.apply(Command::Cast {
                thing: GEM,
                at: None,
                target: None
            })),
            Some(CastRefusal::NotASpell)
        );
        assert_eq!(
            refused(&g.apply(Command::Cast {
                thing: CALL,
                at: None,
                target: None
            })),
            Some(CastRefusal::NotCarried)
        );
        assert_eq!(
            refused(&g.apply(Command::Use {
                thing: STONE,
                at: None,
                target: None
            })),
            Some(CastRefusal::NotUsable)
        );
        assert_eq!(
            refused(&g.apply(Command::Cast {
                thing: BOLT,
                at: None,
                target: Some(cr(9))
            })),
            Some(CastRefusal::BadTarget)
        );
        assert_eq!(g.current_player().moves_left, 15);

        // A bolt at the creature: cost 7 below Power 20 works 90% of the
        // time (4.1; this seed's roll works), and costs 7.
        let ev = g.apply(Command::Cast {
            thing: BOLT,
            at: None,
            target: Some(cr(1)),
        });
        let Some(Event::SpellCast {
            chance,
            cost,
            success,
            ..
        }) = ev.first()
        else {
            panic!("{ev:?}")
        };
        assert_eq!((*chance, *cost, *success), (90, 7, true));
        // Power: 20 - 7, then 1 back toward Wisdom 20 at the top of the
        // next turn (5.1), which starts within the same command.
        assert_eq!(g.current_player().power, 14);
        // 10 - 6 (the creature heals afterwards in its own turn, 5.3).
        assert!(ev.contains(&Event::StatChanged {
            who: cr(1),
            stat: Stat::LifeForce,
            from: 10,
            to: 4
        }));
        assert!(ev.contains(&Event::TurnEnded { player: 0 }));
        assert!(g.current_player().owns(BOLT));

        // A free heal on yourself always works and then is gone.
        let ev = g.apply(Command::Cast {
            thing: HEAL,
            at: None,
            target: None,
        });
        assert!(ev.contains(&Event::SpellGone { who: ME, thing: HEAL }));
        // +5 (from 6, or 7 if the top-of-turn die healed; a cast leaves no
        // idle moves to heal on, 5.3).
        assert!(ev.iter().any(|e| matches!(
            e,
            Event::StatChanged { who: ME, stat: Stat::LifeForce, from, to } if *from >= 6 && *to == *from + 5
        )));
        assert!(!g.current_player().owns(HEAL));
    });
}

#[test]
fn using_items_needs_no_power() {
    both(|p| {
        let mut a = adventure(p);
        put_creature(&mut a, 1, 5, 3, |_| {});
        let mut g = start(
            a,
            2,
            2,
            PlayerSetup {
                wisdom: 0, // so Power 0 (5.1)
                life_force: 5,
                possessions: vec![POTION, ROD],
                ..setup()
            },
        );
        let ev = g.apply(Command::Use {
            thing: POTION,
            at: None,
            target: None,
        });
        assert_eq!(
            ev[0],
            Event::ItemUsed {
                by: ME,
                thing: POTION,
                target: ME
            }
        );
        assert_eq!(lf(&g, ME), 9);
        assert!(!g.current_player().owns(POTION));
        g.apply(Command::Use {
            thing: ROD,
            at: None,
            target: Some(cr(1)),
        });
        assert_eq!(lf(&g, cr(1)), 7);
        assert!(g.current_player().owns(ROD));
    });
}

#[test]
fn a_use_costs_two_power_and_two_steps_and_the_turn_goes_on() {
    // magic 4.2, 4.3, as captured: 2 Power and two steps a use; using
    // doesn't end the turn.
    both(|p| {
        let a = adventure(p);
        let mut g = start(
            a,
            2,
            2,
            PlayerSetup {
                wisdom: 20,
                possessions: vec![ROD],
                ..setup()
            },
        );
        assert_eq!(g.current_player().moves_left, 15);
        // (The rod lowers Life Force: two uses are safe.)
        for (power, moves) in [(18, 13), (16, 11)] {
            let ev = g.apply(Command::Use {
                thing: ROD,
                at: None,
                target: None,
            });
            assert!(!ev.iter().any(|e| matches!(e, Event::TurnEnded { .. })));
            assert_eq!(g.stat(ME, Stat::Power), Some(power));
            assert_eq!(g.current_player().moves_left, moves);
        }
    });
}

#[test]
fn kill_all_spares_owners_and_the_dead_leave_their_things() {
    both(|p| {
        let mut a = adventure(p);
        put_creature(&mut a, 1, 5, 3, |c| {
            c.possessions = vec![
                Possession { thing: GEM, count: 2 },
                Possession {
                    thing: DUST,
                    count: 1,
                },
                Possession {
                    thing: CLAW,
                    count: 1,
                },
                Possession {
                    thing: BOLT,
                    count: 1,
                },
            ];
            c.weapon = CLAW;
            c.wealth = 300;
        });
        put_creature(&mut a, 2, 6, 4, |c| {
            c.possessions = vec![Possession {
                thing: AMULET,
                count: 1,
            }];
        });
        let mut g = start(
            a,
            2,
            2,
            PlayerSetup {
                possessions: vec![DOOM, AMULET],
                ..setup()
            },
        );
        // 3.1: cast at the square of creature 1, which owns no amulet.
        let ev = g.apply(Command::Cast {
            thing: DOOM,
            at: Some((5, 3)),
            target: None,
        });
        let died: Vec<_> = ev
            .iter()
            .filter_map(|e| match e {
                Event::Died { who, at, left } => Some((*who, *at, left.clone())),
                _ => None,
            })
            .collect();
        // The vanishing dust, the owner-only claw and the spell are lost;
        // the gold lands first, as one pile (a pile's size is two bytes),
        // the gems on it (7.2).
        assert_eq!(
            died,
            vec![(cr(1), hall(5, 3), vec![(GOLD, 300), (GEM, 1), (GEM, 1)])]
        );
        assert!(g.creature(1, 1).is_none());
        assert!(g.creature(1, 2).is_some());
        assert!(g.current_player().alive);
        assert_eq!(stack(&g, 0, 5, 3), vec![GEM, GEM, GOLD]);
    });
}

#[test]
fn the_dead_put_their_gold_down_first_then_their_things_lowest_number_up() {
    // 7.2 (capture.md 10.37 item 2): the gold goes down first, as one pile,
    // then the possessions from the lowest thing number up, each on top;
    // what the square held before stays under them all.
    both(|p| {
        let mut a = adventure(p);
        put_item(&mut a, 0, 5, 3, GEM, 0);
        put_creature(&mut a, 1, 5, 3, |c| {
            c.possessions = vec![
                Possession {
                    thing: STONE,
                    count: 1,
                },
                Possession { thing: GEM, count: 1 },
                Possession {
                    thing: POTION,
                    count: 1,
                },
            ];
            c.wealth = 50;
        });
        let mut g = start(
            a,
            2,
            2,
            PlayerSetup {
                possessions: vec![DOOM, AMULET],
                ..setup()
            },
        );
        let ev = g.apply(Command::Cast {
            thing: DOOM,
            at: None,
            target: Some(cr(1)),
        });
        assert!(ev.contains(&Event::Died {
            who: cr(1),
            at: hall(5, 3),
            left: vec![(GOLD, 50), (GEM, 1), (POTION, 1), (STONE, 1)]
        }));
        // Top first: the highest-numbered on top, the old gem at the bottom.
        assert_eq!(stack(&g, 0, 5, 3), vec![STONE, POTION, GEM, GOLD, GEM]);
    });
}

#[test]
fn a_player_who_dies_ends_their_turn_and_the_last_one_ends_the_game() {
    both(|p| {
        let mut a = adventure(p);
        put_item(&mut a, 0, 3, 2, DISEASE_DOWN, 0);
        put_item(&mut a, 0, 3, 4, DISEASE_DOWN, 0);
        let mut g = start(
            a,
            2,
            2,
            PlayerSetup {
                life_force: 9,
                possessions: vec![GEM],
                gold: 3,
                ..setup()
            },
        );
        g.apply(Command::AddPlayer {
            setup: PlayerSetup {
                name: "TWO".into(),
                life_force: 4,
                ..setup()
            },
        });
        // Characters 6.2: the newcomer joins at the round's end.
        g.apply(Command::Wait);
        // Joining raised TWO to its Constitution (5.1): wound it again.
        g.set_life_force(1, 4);
        let ev = mv(&mut g, Dir::E);
        assert!(ev.contains(&Event::Died {
            who: ME,
            at: hall(3, 2),
            left: vec![(GOLD, 3), (GEM, 1)]
        }));
        assert!(ev.contains(&Event::TurnEnded { player: 0 }));
        assert_eq!(g.current(), 1);
        assert!(!g.players()[0].alive);
        // The dead are skipped: TWO plays on alone.
        g.apply(Command::Wait);
        assert_eq!(g.current(), 1);
        g.place(1, hall(2, 4));
        let ev = mv(&mut g, Dir::E);
        assert!(ev.contains(&Event::AllPlayersDead));
        assert!(g.is_over());
        assert!(g.apply(Command::Wait).is_empty());
    });
}

const CALL10: u16 = 38; // magic spell: summon creature 5, cost 10
const MAKE: u16 = 39; // magic spell: Add to Room GEM, cost 0
const BECKON: u16 = 99; // space: summon creature 5
const BELL: u16 = 100; // obstacle, bump: summon creature 5
const LURE: u16 = 44; // magic item, picked up: summon creature 5
const SEED: u16 = 45; // magic item, picked up: Add to Room GEM
const GATE: u16 = 101; // portal, invokes summon creature 5
const ARCH: u16 = 102; // portal, invokes Add to Room GEM

/// The extra things the target-square tests use.
fn more(a: &mut Adventure) {
    let mut set = |n: u16, name: &str, kind: ThingKind| {
        let t = &mut a.things[n as usize - 1];
        t.name = name.into();
        t.kind = kind;
        t.raw = [0; 4];
    };
    set(CALL10, "CALL10", spell(10, act(K::SummonOrBanish, IMP), false));
    set(MAKE, "MAKE", spell(0, act(K::AddToRoom, GEM), false));
    set(BECKON, "BECKON", space(act(K::SummonOrBanish, IMP)));
    set(
        BELL,
        "BELL",
        ThingKind::Obstacle {
            picture: 7,
            bump_invokes_spell: true,
            action: act(K::SummonOrBanish, IMP),
        },
    );
    let pick = |action| ThingKind::MagicItem {
        picture: 3,
        disappears_after_use: false,
        carry: Carry { weight: 1, value: 1 },
        action,
        on_use: false,
        on_drop: false,
        on_pickup: true,
    };
    set(LURE, "LURE", pick(act(K::SummonOrBanish, IMP)));
    set(SEED, "SEED", pick(act(K::AddToRoom, GEM)));
    let portal = |action| ThingKind::Portal {
        picture: 5,
        two_way: false,
        access: Access::InvokeSpell { action },
        destroy_needed_thing: false,
    };
    set(GATE, "GATE", portal(act(K::SummonOrBanish, IMP)));
    set(ARCH, "ARCH", portal(act(K::AddToRoom, GEM)));
}

/// Puts a portal of `thing` on HALL (x, y) leading to SIDE (2, 2).
fn put_portal(a: &mut Adventure, x: u8, y: u8, thing: u16) {
    a.regions[0].rooms[0].items.insert(
        0,
        RoomItem {
            x,
            y,
            thing,
            parameter: 0,
            flag: false,
            destination: Some(acs_core::model::Destination::Room {
                region: 1,
                room: 1,
                x: 2,
                y: 2,
            }),
            gold: None,
        },
    );
}

fn side(x: u8, y: u8) -> Location {
    Location::Room {
        region: 1,
        room: 1,
        x,
        y,
    }
}

fn rng_state(g: &Game) -> serde_json::Value {
    serde_json::to_value(g).expect("serializes")["rng"].clone()
}

fn summoned(ev: &[Event]) -> Option<(usize, u16, Location)> {
    ev.iter().find_map(|e| match e {
        Event::CreatureSummoned {
            slot, creature, at, ..
        } => Some((*slot, *creature, *at)),
        _ => None,
    })
}

#[test]
fn summons_are_made_at_once_on_the_target_square() {
    // 3.2 (capture.md 10.40 item 2): made as the spell takes effect, in
    // the lowest free slot, on the target square, shared with whoever
    // stands there.
    both(|p| {
        let mut a = adventure(p);
        put_creature(&mut a, 1, 6, 4, |_| {});
        let mut g = start(
            a,
            2,
            2,
            PlayerSetup {
                possessions: vec![CALL, BANISH],
                ..setup()
            },
        );
        // No target: the caster's own square, before the turn ends.
        let ev = g.apply(Command::Cast {
            thing: CALL,
            at: None,
            target: None,
        });
        assert_eq!(summoned(&ev), Some((0, IMP, hall(2, 2))));
        let made = ev
            .iter()
            .position(|e| matches!(e, Event::CreatureSummoned { .. }))
            .expect("summoned");
        let ended = ev
            .iter()
            .position(|e| matches!(e, Event::TurnEnded { .. }))
            .expect("turn ended");
        assert!(made < ended);
        assert_eq!(
            g.creature(1, 0).map(|c| c.name.trim().to_string()).as_deref(),
            Some("IMP")
        );
        assert!(!g.is_random(1, 0));
        // At a creature (the old form of the command): its square, shared.
        let ev = g.apply(Command::Cast {
            thing: CALL,
            at: None,
            target: Some(cr(1)),
        });
        assert_eq!(summoned(&ev), Some((2, IMP, hall(6, 4))));
        // At an empty square.
        let ev = g.apply(Command::Cast {
            thing: CALL,
            at: Some((4, 5)),
            target: None,
        });
        assert_eq!(summoned(&ev), Some((3, IMP, hall(4, 5))));

        // Banishing takes every creature in the room, not the player.
        let ev = g.apply(Command::Cast {
            thing: BANISH,
            at: None,
            target: None,
        });
        let gone: Vec<_> = ev
            .iter()
            .filter_map(|e| match e {
                Event::CreatureRemoved {
                    slot,
                    reason: RemovalReason::Banished,
                    ..
                } => Some(*slot),
                _ => None,
            })
            .collect();
        assert_eq!(gone, vec![0, 1, 2, 3]);
        assert!(g.current_player().alive);

        // On the world map nobody answers.
        g.place(0, Location::World { x: 20, y: 20 });
        let ev = g.apply(Command::Cast {
            thing: CALL,
            at: None,
            target: None,
        });
        assert!(ev.contains(&Event::SummonUnanswered { creature: IMP }));
    });
}

#[test]
fn a_summons_takes_no_turn_in_its_round() {
    // 3.2: even in the slot of a creature that died earlier in the round
    // and is still on the round's list.
    both(|p| {
        let mut a = adventure(p);
        put_creature(&mut a, 0, 5, 3, |c| {
            c.constitution = 3;
            c.life_force = 3;
        });
        let mut g = start(
            a,
            2,
            2,
            PlayerSetup {
                wisdom: 20,
                possessions: vec![ROD, CALL],
                ..setup()
            },
        );
        // A round whose list holds slot 0 after the player.
        let ev = g.apply(Command::Wait);
        assert!(ev
            .iter()
            .any(|e| matches!(e, Event::CreatureTurn { slot: 0, .. })));
        // The rod, used at its square, kills it; the turn goes on (4.2).
        let ev = g.apply(Command::Use {
            thing: ROD,
            at: Some((5, 3)),
            target: None,
        });
        assert!(ev
            .iter()
            .any(|e| matches!(e, Event::Died { who, .. } if *who == cr(0))));
        let ev = g.apply(Command::Cast {
            thing: CALL,
            at: Some((4, 4)),
            target: None,
        });
        assert_eq!(summoned(&ev), Some((0, IMP, hall(4, 4))));
        assert!(
            !ev.iter()
                .any(|e| matches!(e, Event::CreatureTurn { slot: 0, .. })),
            "{ev:?}"
        );
        // Its first turn comes in the next round.
        let ev = g.apply(Command::Wait);
        assert!(ev
            .iter()
            .any(|e| matches!(e, Event::CreatureTurn { slot: 0, .. })));
    });
}

#[test]
fn a_summons_into_a_full_region_spends_and_rolls_then_fails() {
    // 3.2 (capture.md 10.42 item 1): at a cost above 0 the Power is spent
    // and the roll drawn as for any cast, then the full region fails it
    // whatever the roll; no slot changes and the turn ends. The same
    // states with a slot free draw alike.
    both(|p| {
        let cast = |full: usize, seed: u64| {
            let mut a = adventure(p);
            more(&mut a);
            for slot in 0..full {
                put_creature(&mut a, slot, 1 + (slot % 8) as u8, 4 + (slot / 8) as u8, |_| {});
            }
            let (mut g, _) = Game::new(
                a,
                seed,
                Rules { diagonals: true },
                PlayerSetup {
                    wisdom: 31,
                    possessions: vec![CALL10],
                    ..setup()
                },
            );
            g.place(0, hall(2, 2));
            let before = rng_state(&g);
            let ev = g.apply(Command::Cast {
                thing: CALL10,
                at: Some((3, 2)),
                target: None,
            });
            (g, ev, before)
        };
        let mut successes = 0;
        for seed in 0..12 {
            let (g, ev, before) = cast(16, seed);
            let (free, free_ev, free_before) = cast(15, seed);
            assert_eq!(before, free_before);
            let cast_at = ev
                .iter()
                .position(|e| {
                    matches!(
                        e,
                        Event::SpellCast {
                            success: false,
                            cost: 10,
                            ..
                        }
                    )
                })
                .expect("cast and failed");
            assert_eq!(ev[cast_at + 1], Event::SummonUnanswered { creature: IMP });
            assert_eq!(summoned(&ev), None);
            assert!(ev.contains(&Event::TurnEnded { player: 0 }));
            // The cost spent in both (the next turn's drift may follow).
            assert_eq!(g.stat(ME, Stat::Power), free.stat(ME, Stat::Power), "seed {seed}");
            assert!(g.stat(ME, Stat::Power) < Some(31 - 10 + 3), "seed {seed}");
            let roll_ok = free_ev
                .iter()
                .any(|e| matches!(e, Event::SpellCast { success: true, .. }));
            successes += roll_ok as u32;
            assert_eq!(summoned(&free_ev).is_some(), roll_ok);
            // A failed roll in the free region: both games drew alike.
            if !roll_ok {
                assert_eq!(rng_state(&g), rng_state(&free), "seed {seed}");
            }
        }
        assert!(successes > 0, "no seed whose roll worked");
    });
}

#[test]
fn a_cost_0_summons_into_a_full_region_spends_and_draws_nothing() {
    // 3.2: at cost 0 nothing is spent and nothing drawn; the turn ends.
    both(|p| {
        let full = || {
            let mut a = adventure(p);
            for slot in 0..16 {
                put_creature(&mut a, slot, 1 + (slot % 8) as u8, 4 + (slot / 8) as u8, |_| {});
            }
            start(
                a,
                2,
                2,
                PlayerSetup {
                    wisdom: 13,
                    possessions: vec![CALL, HEAL],
                    ..setup()
                },
            )
        };
        let mut g = full();
        let ev = g.apply(Command::Cast {
            thing: CALL,
            at: None,
            target: None,
        });
        assert_eq!(ev[0], Event::SummonUnanswered { creature: IMP });
        assert!(!ev.iter().any(|e| matches!(e, Event::SpellCast { .. })));
        assert!(ev.contains(&Event::TurnEnded { player: 0 }));
        assert_eq!(g.stat(ME, Stat::Power), Some(13));
        // The cast drew nothing: a game whose player casts another cost-0
        // spell, one that draws nothing, ends the turn with the same state.
        let mut w = full();
        let ev = w.apply(Command::Cast {
            thing: HEAL,
            at: None,
            target: None,
        });
        assert!(ev.contains(&Event::TurnEnded { player: 0 }));
        assert_eq!(rng_state(&w), rng_state(&g));
    });
}

#[test]
fn summons_with_no_target_press_land_on_the_setting_off_things_square() {
    // 3.2 (capture.md 10.42 item 1): the space stepped on; the obstacle's
    // own square, the bumper beside it; on the wall line, the bumper's
    // own; a portal's own square in the room left; the item's square for
    // a pick-up.
    both(|p| {
        let mut a = adventure(p);
        more(&mut a);
        put_item(&mut a, 0, 3, 2, BECKON, 0);
        put_item(&mut a, 0, 3, 4, BELL, 0);
        put_item(&mut a, 0, 0, 2, BELL, 0); // on the west wall line
        put_item(&mut a, 0, 6, 2, LURE, 1);
        put_portal(&mut a, 6, 4, GATE);
        let mut g = start(a, 2, 2, setup());
        let ev = mv(&mut g, Dir::E);
        assert_eq!(summoned(&ev), Some((0, IMP, hall(3, 2))));
        g.place(0, hall(2, 4));
        let ev = mv(&mut g, Dir::E);
        assert_eq!(summoned(&ev), Some((1, IMP, hall(3, 4))));
        assert_eq!(g.current_player().at, hall(2, 4));
        g.place(0, hall(1, 2));
        let ev = mv(&mut g, Dir::W);
        assert_eq!(summoned(&ev), Some((2, IMP, hall(1, 2))));
        g.place(0, hall(5, 2));
        let ev = mv(&mut g, Dir::E);
        assert!(ev.contains(&Event::PickedUp {
            player: 0,
            thing: LURE
        }));
        assert_eq!(summoned(&ev), Some((3, IMP, hall(6, 2))));
        assert_eq!(g.current_player().at, hall(5, 2));
        g.place(0, hall(5, 4));
        let ev = mv(&mut g, Dir::E);
        assert_eq!(summoned(&ev), Some((4, IMP, hall(6, 4))));
        assert_eq!(g.current_player().at, side(2, 2));
    });
}

#[test]
fn add_to_room_with_no_target_press_lands_on_the_setting_off_things_square() {
    // 3.13 (capture.md 10.42 item 4): an obstacle on the wall line puts
    // it under the bumper; a portal on its own square in the room left;
    // a pick-up on the item's square.
    both(|p| {
        let mut a = adventure(p);
        more(&mut a);
        put_item(&mut a, 0, 0, 4, CHEST, 0); // on the west wall line
        put_item(&mut a, 0, 6, 2, SEED, 1);
        put_portal(&mut a, 6, 4, ARCH);
        let mut g = start(a, 1, 4, setup());
        let added = |ev: &[Event]| {
            ev.iter().find_map(|e| match e {
                Event::ThingAdded { thing: GEM, at } => Some(*at),
                _ => None,
            })
        };
        let ev = mv(&mut g, Dir::W);
        assert_eq!(added(&ev), Some(hall(1, 4)));
        assert_eq!(stack(&g, 0, 1, 4), vec![GEM]);
        assert_eq!(g.current_player().at, hall(1, 4));
        g.place(0, hall(5, 2));
        let ev = mv(&mut g, Dir::E);
        assert_eq!(added(&ev), Some(hall(6, 2)));
        assert_eq!(stack(&g, 0, 6, 2), vec![GEM]);
        g.place(0, hall(5, 4));
        let ev = mv(&mut g, Dir::E);
        assert_eq!(added(&ev), Some(hall(6, 4)));
        assert_eq!(stack(&g, 0, 6, 4), vec![GEM, ARCH]);
        assert_eq!(g.current_player().at, side(2, 2));
    });
}

#[test]
fn kill_all_acts_on_its_target_alone_and_spares_no_caster() {
    // 3.1 (capture.md 10.40 item 1).
    both(|p| {
        let mut a = adventure(p);
        put_creature(&mut a, 1, 5, 3, |_| {});
        put_creature(&mut a, 2, 6, 4, |c| {
            c.possessions = vec![Possession {
                thing: AMULET,
                count: 1,
            }];
        });
        put_creature(&mut a, 3, 7, 4, |_| {});
        let mut g = start(
            a,
            2,
            2,
            PlayerSetup {
                possessions: vec![DOOM],
                ..setup()
            },
        );
        let deaths = |ev: &[Event]| {
            ev.iter()
                .filter_map(|e| match e {
                    Event::Died { who, .. } => Some(*who),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        // At an owner: nobody dies.
        let ev = g.apply(Command::Cast {
            thing: DOOM,
            at: Some((6, 4)),
            target: None,
        });
        assert_eq!(deaths(&ev), vec![]);
        // At a non-owner: it alone dies, the other non-owners live.
        let ev = g.apply(Command::Cast {
            thing: DOOM,
            at: Some((5, 3)),
            target: None,
        });
        assert_eq!(deaths(&ev), vec![cr(1)]);
        assert!(g.creature(1, 3).is_some());
        // At an empty square: refused, at no cost (2.4).
        let ev = g.apply(Command::Cast {
            thing: DOOM,
            at: Some((4, 4)),
            target: None,
        });
        assert_eq!(
            ev,
            vec![Event::CastRefused {
                player: 0,
                thing: DOOM,
                reason: CastRefusal::NoOneThere
            }]
        );
        // At itself, owning no amulet: the caster dies.
        let ev = g.apply(Command::Cast {
            thing: DOOM,
            at: None,
            target: None,
        });
        assert_eq!(deaths(&ev), vec![ME]);
        assert!(!g.players()[0].alive);
        assert!(g.creature(1, 2).is_some() && g.creature(1, 3).is_some());
    });
}

#[test]
fn a_cast_is_aimed_at_a_square_and_add_to_room_lands_there() {
    // 2.4: a square inside the caster's room; walls and squares beyond
    // refuse at no cost. 3.13: Add to Room lands on top of the target.
    both(|p| {
        let mut a = adventure(p);
        more(&mut a);
        put_item(&mut a, 0, 2, 2, TUNE, 0); // a space under the caster
        let mut g = start(
            a,
            2,
            2,
            PlayerSetup {
                wisdom: 20,
                possessions: vec![BOLT, MAKE],
                ..setup()
            },
        );
        let refused = |ev: &[Event]| match ev {
            [Event::CastRefused { reason, .. }] => Some(*reason),
            _ => None,
        };
        for square in [(0, 0), (9, 3), (2, 6), (30, 2)] {
            assert_eq!(
                refused(&g.apply(Command::Cast {
                    thing: MAKE,
                    at: Some(square),
                    target: None,
                })),
                Some(CastRefusal::BadTarget),
                "{square:?}"
            );
        }
        assert_eq!(
            refused(&g.apply(Command::Cast {
                thing: BOLT,
                at: Some((5, 3)),
                target: None,
            })),
            Some(CastRefusal::NoOneThere)
        );
        assert_eq!(g.current_player().moves_left, 15);
        assert_eq!(g.stat(ME, Stat::Power), Some(20));

        let ev = g.apply(Command::Cast {
            thing: MAKE,
            at: Some((6, 2)),
            target: None,
        });
        assert!(ev.contains(&Event::ThingAdded {
            thing: GEM,
            at: hall(6, 2)
        }));
        assert_eq!(stack(&g, 0, 6, 2), vec![GEM]);
        // The caster's own square: on top of the space there.
        let ev = g.apply(Command::Cast {
            thing: MAKE,
            at: None,
            target: None,
        });
        assert!(ev.contains(&Event::ThingAdded {
            thing: GEM,
            at: hall(2, 2)
        }));
        assert_eq!(stack(&g, 0, 2, 2), vec![GEM, TUNE]);

        // On the world map only the caster's own square.
        g.place(0, Location::World { x: 20, y: 20 });
        assert_eq!(
            refused(&g.apply(Command::Cast {
                thing: BOLT,
                at: Some((21, 20)),
                target: None,
            })),
            Some(CastRefusal::BadTarget)
        );
    });
}

#[test]
fn summons_go_unanswered_in_a_full_region() {
    both(|p| {
        let mut a = adventure(p);
        for slot in 0..16 {
            put_creature(&mut a, slot, 1 + (slot % 8) as u8, 4 + (slot / 8) as u8, |_| {});
        }
        let mut g = start(
            a,
            2,
            2,
            PlayerSetup {
                possessions: vec![CALL],
                ..setup()
            },
        );
        let ev = g.apply(Command::Cast {
            thing: CALL,
            at: None,
            target: None,
        });
        assert!(ev.contains(&Event::SummonUnanswered { creature: IMP }));
    });
}

#[test]
fn a_trait_past_its_maximum_is_never_pulled_down_to_it() {
    // magic 3.5-3.6: Constitution has 6 bits, so 40 is a real value; an
    // increase leaves it, a decrease takes 1 from it (not down to 31).
    let mut a = adventure(Platform::Pc);
    put_item(&mut a, 0, 2, 1, STAT, 0); // Constitution +1
    put_item(&mut a, 0, 3, 1, LOWER, 0); // Constitution -1
    let setup = PlayerSetup {
        constitution: 40,
        life_force: 40,
        ..setup()
    };
    let mut g = start(a, 1, 1, setup);
    let changed = |ev: &[Event]| {
        ev.iter()
            .filter_map(|e| match e {
                Event::StatChanged { stat, from, to, .. } => Some((*stat, *from, *to)),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let up = changed(&mv(&mut g, Dir::E));
    assert!(
        up.iter().all(|&(s, _, to)| s != Stat::Constitution || to == 40),
        "{up:?}"
    );
    assert_eq!(changed(&mv(&mut g, Dir::E)), vec![(Stat::Constitution, 40, 39)]);
}

#[test]
fn stats_magic_defense_give_messages_music_rid_and_add() {
    both(|p| {
        let mut a = adventure(p);
        put_item(&mut a, 0, 2, 1, STAT, 8); // Melee +5
        put_item(&mut a, 0, 3, 1, STAT, 1); // Strength +1
        put_item(&mut a, 0, 4, 1, LOWER, 3); // Speed -1
        put_item(&mut a, 0, 5, 1, WARD, 0);
        put_item(&mut a, 0, 6, 1, GIFT, 0);
        put_item(&mut a, 0, 7, 1, SCROLL, 0);
        put_item(&mut a, 0, 8, 1, TUNE, 0);
        put_item(&mut a, 0, 4, 4, GEM, 1);
        put_item(&mut a, 0, 6, 5, GEM, 1);
        put_item(&mut a, 0, 6, 5, DOALL, 0); // a gem under something
        put_item(&mut a, 0, 1, 3, CLEAN, 0);
        put_item(&mut a, 0, 1, 5, CHEST, 0);
        let mut g = start(a, 1, 1, setup());
        let changed = |ev: &[Event]| {
            ev.iter()
                .filter_map(|e| match e {
                    Event::StatChanged { stat, from, to, .. } => Some((*stat, *from, *to)),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(changed(&mv(&mut g, Dir::E)), vec![(Stat::Melee, 30, 35)]);
        assert_eq!(changed(&mv(&mut g, Dir::E)), vec![(Stat::Strength, 11, 12)]);
        assert_eq!(changed(&mv(&mut g, Dir::E)), vec![(Stat::Speed, 15, 14)]);
        assert_eq!(changed(&mv(&mut g, Dir::E)), vec![(Stat::MagicDefense, 0, 1)]);
        assert_eq!(g.current_player().magic_defense, MagicDefense::NonMagicHalfDamage);
        let ev = mv(&mut g, Dir::E);
        assert!(ev.contains(&Event::Given {
            who: ME,
            thing: GEM,
            kept: true
        }));
        let ev = mv(&mut g, Dir::E);
        assert!(ev.contains(&Event::LongMessage {
            number: 7,
            text: "AN OLD SCROLL".into()
        }));
        let ev = mv(&mut g, Dir::E);
        assert!(ev.contains(&Event::Music { number: 13 }));

        // Rid Room takes every loose GEM, in stacks too, not the carried one.
        g.place(0, hall(1, 2));
        let ev = mv(&mut g, Dir::S);
        assert!(ev.contains(&Event::RoomRidOf {
            region: 1,
            room: 0,
            thing: GEM,
            count: 2
        }));
        assert_eq!(stack(&g, 0, 6, 5), vec![DOALL]);
        assert!(g.current_player().owns(GEM));

        // Add to Room on a bump: the chest is inside, so the gem lands on it.
        g.place(0, hall(2, 5));
        let ev = mv(&mut g, Dir::W);
        assert!(ev.contains(&Event::ThingAdded {
            thing: GEM,
            at: hall(1, 5)
        }));
        assert_eq!(stack(&g, 0, 1, 5), vec![GEM, CHEST]);
    });
}

#[test]
fn activate_all_runs_the_stack_below_it() {
    both(|p| {
        let mut a = adventure(p);
        // Top to bottom: DOALL, LIGHT (owners of GEM: +5), DARK (non-owners
        // of GEM), DOALL2 (skipped), SLOT (when dropped here: skipped), TUNE.
        for t in [TUNE, SLOT, DOALL2, DARK, LIGHT, DOALL] {
            let param = if matches!(t, LIGHT | DARK | SLOT) { GEM } else { 0 };
            put_item(&mut a, 0, 3, 2, t, param as u8);
        }
        let mut g = start(
            a,
            2,
            2,
            PlayerSetup {
                life_force: 5,
                possessions: vec![GEM],
                ..setup()
            },
        );
        let ev = mv(&mut g, Dir::E);
        let fired: Vec<_> = ev
            .iter()
            .filter_map(|e| match e {
                Event::SpellTriggered { thing, source, .. } => Some((*thing, *source)),
                _ => None,
            })
            .collect();
        assert_eq!(
            fired,
            vec![
                (DOALL, SpellSource::Space),
                (LIGHT, SpellSource::CustomSpace),
                (TUNE, SpellSource::Space)
            ]
        );
        assert_eq!(lf(&g, ME), 10);
        assert!(ev.contains(&Event::Music { number: 13 }));
    });
}

#[test]
fn disease_power_drift_and_healing() {
    both(|p| {
        let mut a = adventure(p);
        // The manual's disease stack: Activate All, +10 for a while, -10.
        for t in [DISEASE_DOWN, DISEASE_UP, DOALL] {
            put_item(&mut a, 0, 3, 2, t, 0);
        }
        let mut g = start(
            a,
            2,
            2,
            PlayerSetup {
                wisdom: 11,
                ..setup()
            },
        );
        // Power starts at Wisdom (5.1); put it above to watch it drift.
        g.set_power(0, 15);
        mv(&mut g, Dir::E);
        assert_eq!(lf(&g, ME), 11);
        // 5.2: a temporary +10 is taken back 11 times.
        assert_eq!(g.temporary_life(ME), 11);
        // Each turn's top: Power 1 toward Wisdom (5.1); healing on Life
        // Force below Constitution 11 (5.3); every 1 to 6 rounds a step of
        // the reversal (5.2): Life Force dips under 11 (two steps can come
        // before a heal) until the 11 steps are done and healing is back.
        let mut power = Vec::new();
        for _ in 0..200 {
            g.apply(Command::Wait);
            power.push(g.current_player().power);
            assert!((5..=11).contains(&lf(&g, ME)), "{}", lf(&g, ME));
        }
        assert_eq!(power[..5], [14, 13, 12, 11, 11]);
        assert_eq!(g.temporary_life(ME), 0);
        assert_eq!(lf(&g, ME), 11);
    });
}

#[test]
fn smart_creatures_cast_at_foes_and_heal_friends() {
    both(|p| {
        let mut a = adventure(p);
        // A smart enemy with a bolt; a dumb one with a bolt; a smart slinker.
        put_creature(&mut a, 1, 6, 2, |c| {
            c.wisdom = 15;
            c.power = 20;
            c.possessions = vec![Possession {
                thing: BOLT,
                count: 1,
            }];
        });
        put_creature(&mut a, 2, 6, 4, |c| {
            c.wisdom = 5;
            c.power = 40;
            c.possessions = vec![Possession {
                thing: BOLT,
                count: 1,
            }];
        });
        put_creature(&mut a, 3, 7, 5, |c| {
            c.wisdom = 15;
            c.power = 40;
            c.strategy.aggressive = false;
            c.possessions = vec![Possession {
                thing: BOLT,
                count: 1,
            }];
        });
        let mut g = start(a, 2, 2, setup());
        let ev = g.apply(Command::Wait);
        let casts: Vec<_> = ev
            .iter()
            .filter_map(|e| match e {
                Event::SpellCast { by, target, cost, .. } => Some((*by, *target, *cost)),
                _ => None,
            })
            .collect();
        assert_eq!(casts, vec![(cr(1), ME, 7)]);
        // 5.1: the drift toward 15 at the top of its turn (19), then 7 for
        // the cast.
        assert_eq!(g.creature(1, 1).unwrap().power, 12);
        // Next turn 13, still below its Wisdom (15): no second cast.
        let ev = g.apply(Command::Wait);
        assert!(!ev.iter().any(|e| matches!(e, Event::SpellCast { .. })), "{ev:?}");
    });
}

#[test]
fn a_creature_that_casts_does_not_ready_first() {
    // Combat 6.2: readying comes after the smartness check, so a smart
    // enemy beside the player that casts never readies its weapon.
    both(|p| {
        let mut a = adventure(p);
        put_creature(&mut a, 1, 3, 2, |c| {
            c.wisdom = 15;
            c.power = 20;
            c.possessions = vec![
                Possession {
                    thing: BOLT,
                    count: 1,
                },
                Possession {
                    thing: CLAW,
                    count: 1,
                },
            ];
        });
        let mut g = start(a, 2, 2, setup());
        let ev = g.apply(Command::Wait);
        assert!(ev.iter().any(|e| matches!(e, Event::SpellCast { .. })), "{ev:?}");
        assert!(
            !ev.iter().any(|e| matches!(e, Event::CreatureReadied { .. })),
            "{ev:?}"
        );
    });
}

#[test]
fn friends_heal_the_player_and_half_power_creatures_use_items() {
    both(|p| {
        let mut a = adventure(p);
        put_creature(&mut a, 1, 6, 3, |c| {
            c.strategy.alignment = Alignment::Friend;
            c.wisdom = 15;
            c.power = 8; // below Wisdom, at least half: items only
            c.possessions = vec![
                Possession {
                    thing: HEAL,
                    count: 1,
                },
                Possession {
                    thing: POTION,
                    count: 1,
                },
            ];
        });
        let mut g = start(
            a,
            2,
            2,
            PlayerSetup {
                life_force: 3,
                ..setup()
            },
        );
        let ev = g.apply(Command::Wait);
        assert!(ev.contains(&Event::ItemUsed {
            by: cr(1),
            thing: POTION,
            target: ME
        }));
        // The potion's +4 (the player's own healing came first, 5.3).
        assert!(ev.iter().any(|e| matches!(
            e,
            Event::StatChanged { who: ME, stat: Stat::LifeForce, from, to } if *to == *from + 4
        )));
        assert!(!ev.iter().any(|e| matches!(e, Event::SpellCast { .. })));
    });
}

#[test]
fn creatures_set_off_door_and_drop_spells() {
    both(|p| {
        let mut a = adventure(p);
        // A friend fighter that wants the player, who has just left by the
        // music door; and an overburdened one with a bomb. The friend
        // starts two squares from the door: beside it, the door's cost of 4
        // on the turn's first step choice (creatures 6.5) would mostly keep
        // it there.
        put_creature(&mut a, 1, 7, 5, |c| {
            c.speed = 3;
            c.strategy.alignment = Alignment::Friend;
            c.strategy.aggressive = false; // a slinker friend: it seeks the player
        });
        put_creature(&mut a, 2, 4, 4, |c| {
            c.speed = 2; // creatures 8.1.2: a Speed 0 creature never drops
            c.strength = 0;
            c.constitution = 10;
            c.life_force = 10;
            c.possessions = vec![Possession {
                thing: BOMB,
                count: 1,
            }];
        });
        let mut g = start(a, 8, 4, setup());
        // A door into another room ends the turn (movement 2.3), so the
        // creature phase comes with the move.
        let mut ev = mv(&mut g, Dir::SE);
        assert!(ev.contains(&Event::Music { number: 3 }), "{ev:?}");
        ev.extend(g.apply(Command::Wait));
        // The friend follows through the music door and sets it off.
        let music = ev.iter().filter(|e| **e == Event::Music { number: 3 }).count();
        assert_eq!(music, 2, "{ev:?}");
        let bomb = ev.iter().any(|e| {
            matches!(
                e,
                Event::SpellTriggered {
                    by,
                    source: SpellSource::Dropped,
                    thing: BOMB,
                    ..
                } if *by == cr(2)
            )
        });
        assert!(bomb, "{ev:?}");
        assert!(ev.contains(&Event::StatChanged {
            who: cr(2),
            stat: Stat::LifeForce,
            from: 10,
            to: 8
        }));
    });
}

#[test]
fn drop_spells_hit_the_creature_they_land_on() {
    both(|p| {
        let mut a = adventure(p);
        put_creature(&mut a, 1, 3, 2, |_| {});
        let mut g = start(
            a,
            2,
            2,
            PlayerSetup {
                possessions: vec![BOMB, BOMB],
                ..setup()
            },
        );
        g.apply(Command::Drop {
            thing: BOMB,
            dir: Some(Dir::E),
        });
        assert_eq!(lf(&g, cr(1)), 8);
        g.apply(Command::Drop {
            thing: BOMB,
            dir: Some(Dir::W),
        });
        assert_eq!(lf(&g, ME), 9);
    });
}

#[test]
fn magic_is_deterministic() {
    both(|p| {
        let run = || {
            let mut a = adventure(p);
            put_creature(&mut a, 1, 6, 2, |c| {
                c.wisdom = 12;
                c.power = 30;
                c.speed = 4;
                c.possessions = vec![Possession {
                    thing: BOLT,
                    count: 1,
                }];
            });
            let mut g = start(
                a,
                2,
                2,
                PlayerSetup {
                    wisdom: 25, // Power starts at Wisdom (5.1)
                    life_force: 63,
                    possessions: vec![BOLT, CALL],
                    ..setup()
                },
            );
            let mut all = Vec::new();
            for c in [
                Command::Cast {
                    thing: BOLT,
                    at: None,
                    target: Some(cr(1)),
                },
                Command::Wait,
                Command::Cast {
                    thing: CALL,
                    at: None,
                    target: None,
                },
                Command::Wait,
                Command::Wait,
            ] {
                all.extend(g.apply(c));
            }
            (g, all)
        };
        let (a, ea) = run();
        let (b, eb) = run();
        assert_eq!(ea, eb);
        assert_eq!(a, b);
        assert!(ea.iter().any(|e| matches!(e, Event::CreatureSummoned { .. })));
    });
}

/// 5.3: a waited Speed 1 turn always heals (its one idle move rolls 0..0),
/// a turn walked to its end heals only on the top-of-turn die (1 in 6).
#[test]
fn healing_rolls_on_idle_moves_and_at_the_top_of_the_turn() {
    both(|p| {
        // Strength 31: the wound must not slow it (inventory 3.3).
        let wounded = PlayerSetup {
            speed: 1,
            strength: 31,
            constitution: 40,
            life_force: 5,
            ..setup()
        };
        let mut g = start(adventure(p), 2, 2, wounded.clone());
        for _ in 0..20 {
            let before = lf(&g, ME);
            g.apply(Command::Wait);
            let gain = lf(&g, ME) - before;
            assert!((1..=2).contains(&gain), "a waited turn gained {gain}");
        }
        let mut g = start(adventure(p), 2, 2, wounded);
        let before = lf(&g, ME);
        for i in 0..60 {
            // East and back: each step is the turn's only move.
            mv(&mut g, if i % 2 == 0 { Dir::E } else { Dir::W });
        }
        let gain = lf(&g, ME) - before;
        assert!((2..=25).contains(&gain), "60 walked turns gained {gain}");
    });
}

/// 5.2: a temporary change is taken back a step at a time, each 1 to 6
/// rounds after the last (a lone player's turn is a round).
#[test]
fn reversals_come_one_to_six_rounds_apart() {
    both(|p| {
        let mut a = adventure(p);
        for t in [DISEASE_DOWN, DISEASE_UP, DOALL] {
            put_item(&mut a, 0, 3, 2, t, 0);
        }
        let mut g = start(a, 2, 2, setup());
        mv(&mut g, Dir::E);
        let mut left = g.temporary_life(ME);
        assert_eq!(left, 11);
        let mut since = 0;
        while left > 0 {
            g.apply(Command::Wait);
            since += 1;
            let now = g.temporary_life(ME);
            if now != left {
                assert_eq!(now, left - 1);
                assert!((1..=6).contains(&since), "a step after {since} rounds");
                left = now;
                since = 0;
            }
            assert!(since <= 6, "no step for {since} rounds");
        }
    });
}

/// Creatures 2.2.2, 2.2.6: a death alone ends no visit; the dead player is
/// skipped and the visit runs its 10 rounds before the map's turn.
#[test]
fn a_death_does_not_end_the_visit() {
    both(|p| {
        let mut a = adventure(p);
        put_item(&mut a, 0, 3, 2, DISEASE_DOWN, 0);
        let mut g = start(
            a,
            2,
            2,
            PlayerSetup {
                life_force: 9,
                ..setup()
            },
        );
        // Characters 6.2, 6.4: one newcomer a round, each joining at the
        // round's end; then back to player 0's turn.
        for name in ["TWO", "THREE"] {
            g.apply(Command::AddPlayer {
                setup: PlayerSetup {
                    name: name.into(),
                    ..setup()
                },
            });
            let mut ev = Vec::new();
            while !ev.iter().any(|e| matches!(e, Event::PlayerAdded { .. })) {
                ev.extend(g.apply(Command::Wait));
            }
            while g.current() != 0 {
                g.apply(Command::Wait);
            }
        }
        assert_eq!(g.players().len(), 3);
        // The waits may have healed it.
        g.set_life_force(0, 9);
        // 0 and 1 in the hall, 2 on the map: the hall's visit from here.
        g.place(1, hall(2, 4));
        let ev = mv(&mut g, Dir::E);
        assert!(ev.iter().any(|e| matches!(e, Event::Died { who: ME, .. })));
        assert_eq!(g.current(), 1);
        let mut who = Vec::new();
        for _ in 0..10 {
            g.apply(Command::Wait);
            who.push(g.current());
        }
        let mut want = vec![1; 9];
        want.push(2);
        assert_eq!(who, want);
    });
}

/// Creatures 2.2.5: the round finishes after the last player dies (the
/// creature after it on the list still takes its turn), and only then
/// does the game wait for a new player (2.2.6, magic 7.4).
#[test]
fn the_round_finishes_before_all_players_dead() {
    both(|p| {
        let mut a = adventure(p);
        put_item(&mut a, 0, 3, 2, DISEASE_DOWN, 0);
        put_creature(&mut a, 1, 7, 4, |c| c.strategy.alignment = Alignment::Friend);
        let mut g = start(
            a,
            2,
            2,
            PlayerSetup {
                life_force: 9,
                ..setup()
            },
        );
        let ev = mv(&mut g, Dir::E);
        let died = ev.iter().position(|e| matches!(e, Event::Died { who: ME, .. }));
        let turn = ev
            .iter()
            .position(|e| matches!(e, Event::CreatureTurn { slot: 1, .. }));
        let over = ev.iter().position(|e| *e == Event::AllPlayersDead);
        assert!(died < turn && turn < over && died.is_some(), "{ev:?}");
        // Characters 6.3: the round-end message comes first.
        let msg = ev
            .iter()
            .position(|e| *e == Event::LivingChanged { before: 1, after: 0 });
        assert!(turn < msg && msg < over, "{ev:?}");
        assert_eq!(ev.last(), Some(&Event::AllPlayersDead));
        assert!(g.is_over());
    });
}

/// Creatures 2.2.6, magic 7.4: with nobody alive every command but adding
/// a player is ignored; the newcomer takes the first dead player's slot
/// and play goes on, and the game replays.
#[test]
fn after_all_players_dead_a_new_player_takes_the_first_dead_slot() {
    both(|p| {
        let mut a = adventure(p);
        put_item(&mut a, 0, 3, 2, DISEASE_DOWN, 0);
        put_item(&mut a, 0, 3, 4, DISEASE_DOWN, 0);
        let mut g = start(
            a,
            2,
            2,
            PlayerSetup {
                life_force: 9,
                ..setup()
            },
        );
        g.apply(Command::AddPlayer {
            setup: PlayerSetup {
                name: "TWO".into(),
                ..setup()
            },
        });
        // Characters 6.2: the newcomer joins at the round's end.
        g.apply(Command::Wait);
        g.place(1, hall(2, 4));
        g.set_life_force(1, 4);
        mv(&mut g, Dir::E);
        let ev = mv(&mut g, Dir::E);
        assert!(ev.contains(&Event::AllPlayersDead));
        assert!(g.apply(Command::Wait).is_empty());
        let logged = g.log().len();
        let ev = g.apply(Command::AddPlayer {
            setup: PlayerSetup {
                name: "NEW".into(),
                ..setup()
            },
        });
        assert!(ev
            .iter()
            .any(|e| matches!(e, Event::PlayerAdded { player: 0, .. })));
        assert!(ev
            .iter()
            .any(|e| matches!(e, Event::TurnStarted { player: 0, .. })));
        assert!(!g.is_over());
        assert_eq!(g.log().len(), logged + 1);
        assert_eq!(g.players().len(), 2);
        assert_eq!(g.players()[0].name, "NEW");
        assert!(g.players()[0].alive && !g.players()[1].alive);
        assert_eq!(g.current(), 0);
        // On the map, alone: one turn a round.
        g.apply(Command::Wait);
        assert_eq!(g.current(), 0);
        // A rolled character is accepted the same way.
        let mut a = adventure(p);
        put_item(&mut a, 0, 3, 2, DISEASE_DOWN, 0);
        let mut h = start(
            a,
            2,
            2,
            PlayerSetup {
                life_force: 9,
                ..setup()
            },
        );
        assert!(mv(&mut h, Dir::E).contains(&Event::AllPlayersDead));
        let ev = h.apply(Command::CreatePlayer {
            name: "ROLL".into(),
            picture: None,
        });
        assert!(ev
            .iter()
            .any(|e| matches!(e, Event::PlayerAdded { player: 0, .. })));
        assert!(!h.is_over() && h.players().len() == 1);
    });
}

/// Characters 6.2, 6.3, creatures 2.2.5: a request made in the round the
/// last player dies is handled after the round-end message; the newcomer
/// takes the dead player's slot and play goes on. Rounds with no change
/// bring no message.
#[test]
fn an_add_in_the_round_of_a_death_comes_after_the_message() {
    both(|p| {
        let mut a = adventure(p);
        put_item(&mut a, 0, 3, 2, DISEASE_DOWN, 0);
        let mut g = start(
            a,
            2,
            2,
            PlayerSetup {
                life_force: 9,
                ..setup()
            },
        );
        assert_eq!(
            g.apply(Command::AddPlayer {
                setup: PlayerSetup {
                    name: "NEW".into(),
                    ..setup()
                },
            }),
            vec![Event::AddRequested]
        );
        let ev = mv(&mut g, Dir::E);
        let died = ev.iter().position(|e| matches!(e, Event::Died { who: ME, .. }));
        let msg = ev
            .iter()
            .position(|e| *e == Event::LivingChanged { before: 1, after: 0 });
        let added = ev
            .iter()
            .position(|e| matches!(e, Event::PlayerAdded { player: 0, .. }));
        assert!(died.is_some() && died < msg && msg < added, "{ev:?}");
        assert!(!ev.contains(&Event::AllPlayersDead));
        assert!(!g.is_over());
        assert_eq!(g.players().len(), 1);
        assert_eq!(g.players()[0].name, "NEW");
        assert_eq!(g.current(), 0);
        // The newcomer's first round: no message.
        let ev = g.apply(Command::Wait);
        assert!(!ev.iter().any(|e| matches!(e, Event::LivingChanged { .. })));
    });
}

/// Characters 6.1, 6.2, 6.4: a newcomer takes the lowest free slot, a dead
/// player's below a living one included, with that slot's number and its
/// place in the turn list; several requests in a round add one newcomer,
/// the first asked, and the list never holds more than 4.
#[test]
fn a_newcomer_takes_a_dead_slot_below_a_living_player() {
    both(|p| {
        let mut a = adventure(p);
        put_item(&mut a, 0, 3, 2, DISEASE_DOWN, 0);
        let mut g = start(
            a,
            2,
            2,
            PlayerSetup {
                life_force: 9,
                ..setup()
            },
        );
        g.apply(Command::AddPlayer {
            setup: PlayerSetup {
                name: "TWO".into(),
                ..setup()
            },
        });
        g.apply(Command::Wait);
        assert_eq!(g.players().len(), 2);
        // Player 0 dies in the hall; TWO, on the map, plays on.
        assert_eq!(g.current(), 0);
        let ev = mv(&mut g, Dir::E);
        assert!(ev.iter().any(|e| matches!(e, Event::Died { who: ME, .. })));
        assert_eq!(g.current(), 1);
        // Characters 6.2, 6.4: two requests in the round, both answered
        // alike; only the first one's newcomer is added, in slot 0.
        for name in ["NEW", "NEXT"] {
            assert_eq!(
                g.apply(Command::AddPlayer {
                    setup: PlayerSetup {
                        name: name.into(),
                        ..setup()
                    },
                }),
                vec![Event::AddRequested]
            );
        }
        let ev = g.apply(Command::Wait);
        let added: Vec<usize> = ev
            .iter()
            .filter_map(|e| match e {
                Event::PlayerAdded { player, .. } => Some(*player),
                _ => None,
            })
            .collect();
        assert_eq!(added, [0]);
        let names: Vec<&str> = g.players().iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["NEW", "TWO"]);
        assert!(g.players().iter().all(|p| p.alive));
        // The newcomer in slot 0 moves before TWO; no other add comes in
        // the rounds after.
        let mut seen = Vec::new();
        let mut ev = Vec::new();
        for _ in 0..6 {
            seen.push(g.current());
            ev.extend(g.apply(Command::Wait));
        }
        assert_eq!(seen, [0, 1, 0, 1, 0, 1]);
        assert!(!ev.iter().any(|e| matches!(e, Event::PlayerAdded { .. })));
        assert_eq!(g.players().len(), 2);
        // Four at most: one a round fills slots 2 and 3; with none free,
        // two requests in a round bring one refusal.
        for (name, slot) in [("THREE", 2), ("FOUR", 3)] {
            g.apply(Command::AddPlayer {
                setup: PlayerSetup {
                    name: name.into(),
                    ..setup()
                },
            });
            let mut ev = Vec::new();
            while !ev.iter().any(|e| matches!(e, Event::PlayerAdded { .. })) {
                ev.extend(g.apply(Command::Wait));
            }
            assert!(ev
                .iter()
                .any(|e| matches!(e, Event::PlayerAdded { player, .. } if *player == slot)));
        }
        for name in ["FIVE", "SIX"] {
            g.apply(Command::AddPlayer {
                setup: PlayerSetup {
                    name: name.into(),
                    ..setup()
                },
            });
        }
        let mut ev = Vec::new();
        for _ in 0..4 {
            ev.extend(g.apply(Command::Wait));
        }
        assert_eq!(ev.iter().filter(|e| **e == Event::AddRefused).count(), 1);
        assert_eq!(g.players().len(), 4);
    });
}

/// 5.5: a creature's byte 0x0C keeps the round of its last turn; at the
/// top of the next it gains up to N - 1 Power (to Wisdom) and N / 2 Life
/// Force (to Constitution), N the rounds since, modulo 256; a first turn
/// (byte 0) only notes the round, and the byte is never left 0.
#[test]
fn creatures_catch_up_on_the_rounds_they_missed() {
    both(|p| {
        let mut a = adventure(p);
        put_creature(&mut a, 1, 7, 5, |c| {
            c.strategy.alignment = Alignment::Neutral;
            c.strategy.aggressive = false;
            c.wisdom = 40;
            c.power = 0;
            c.constitution = 100;
            c.life_force = 1;
            c.last_turn_round = 0;
        });
        let mut g = start(a, 2, 2, setup());
        let state = |g: &Game| {
            let c = g.creature(1, 1).expect("alive");
            (c.power, c.life_force, c.last_turn_round)
        };
        // The first turn, in round 0: no catch-up, drift only; round 0
        // is noted as 1.
        g.apply(Command::Wait);
        let (power, life, byte) = state(&g);
        assert_eq!((power, byte), (1, 1));
        assert!((1..=3).contains(&life), "{life}");
        // Round 1 notes 1 too (N 0 from round 0's 1): drift only.
        g.apply(Command::Wait);
        let (power, _, byte) = state(&g);
        assert_eq!((power, byte), (2, 1), "round 1");
        // Round 2, N 1: drift and healing only.
        g.apply(Command::Wait);
        let (power, _, byte) = state(&g);
        assert_eq!((power, byte), (3, 2), "round 2");
        // An absence of N rounds before the next turn (round byte + 1),
        // 200 and 255 wrapping below 0.
        for n in [10u8, 39, 2, 200, 255] {
            let (power, life, byte) = state(&g);
            let next = byte + 1;
            g.creature_mut(1, 1).expect("slot").last_turn_round = next.wrapping_sub(n);
            g.apply(Command::Wait);
            let caught_power = (u16::from(power) + u16::from(n) - 1).min(40) as u8;
            let want_power = if caught_power < 40 { caught_power + 1 } else { 40 };
            let caught_life = (u16::from(life) + u16::from(n / 2)).min(100) as u8;
            let (p2, l2, b2) = state(&g);
            assert_eq!(p2, want_power, "N {n}");
            assert!(
                (caught_life..=(caught_life + 2).min(100)).contains(&l2),
                "N {n}: life {life} -> {l2}"
            );
            assert_eq!(b2, next, "N {n}");
        }
        // Capped: Life Force at Constitution, Power at Wisdom.
        let (power, life, _) = state(&g);
        assert_eq!((power, life), (40, 100));
        // Power above Wisdom is not lowered by the catch-up, only by drift.
        g.creature_mut(1, 1).expect("slot").power = 45;
        let (_, _, byte) = state(&g);
        g.creature_mut(1, 1).expect("slot").last_turn_round = (byte + 1).wrapping_sub(50);
        g.apply(Command::Wait);
        assert_eq!(state(&g).0, 44);
    });
}

/// 5.5: the byte is the round counter's low byte, so the turn of a round
/// whose low byte is 0 notes 1, never 0.
#[test]
fn the_catch_up_round_is_never_noted_as_0() {
    both(|p| {
        let mut a = adventure(p);
        put_creature(&mut a, 1, 7, 5, |c| {
            c.strategy.alignment = Alignment::Neutral;
            c.strategy.aggressive = false;
        });
        let mut g = start(a, 2, 2, setup());
        let mut seen = Vec::new();
        for _ in 0..600 {
            g.apply(Command::Wait);
            let b = g.creature(1, 1).expect("alive").last_turn_round;
            assert_ne!(b, 0);
            if seen.last() != Some(&b) {
                seen.push(b);
            }
        }
        // It went round 256 at least twice: 255 then 1, never 0.
        let wraps = seen.windows(2).filter(|w| w[0] == 255 && w[1] == 1).count();
        assert!(wraps >= 2, "{wraps} wraps in {seen:?}");
    });
}

#[test]
fn a_magic_item_placed_with_0_runs_its_pick_up_spell_and_is_not_owned() {
    // Inventory 2.7 (capture.md 10.42 items 1 and 4): the take owns
    // nothing, the pick-up spell still goes off.
    both(|p| {
        let mut a = adventure(p);
        more(&mut a);
        put_item(&mut a, 0, 6, 2, SEED, 0);
        put_item(&mut a, 0, 6, 4, LURE, 0);
        let mut g = start(a, 5, 2, setup());
        let ev = mv(&mut g, Dir::E);
        assert!(ev.contains(&Event::PickedUp {
            player: 0,
            thing: SEED
        }));
        assert!(ev.contains(&Event::ThingAdded {
            thing: GEM,
            at: hall(6, 2)
        }));
        assert!(!g.current_player().owns(SEED));
        assert_eq!(stack(&g, 0, 6, 2), vec![GEM]);
        g.place(0, hall(5, 4));
        let ev = mv(&mut g, Dir::E);
        assert_eq!(summoned(&ev), Some((0, IMP, hall(6, 4))));
        assert!(!g.current_player().owns(LURE));
    });
}

#[test]
fn a_death_keeps_the_slot_random_mark_and_a_summons_clears_it() {
    // Creatures 2.4 (engine choice): the mark is the slot's; a death
    // leaves it set, a summons put into the slot clears it.
    both(|p| {
        let mut a = adventure(p);
        put_creature(&mut a, 1, 5, 3, |_| {});
        let g = start(
            a,
            2,
            2,
            PlayerSetup {
                possessions: vec![DOOM, CALL],
                ..setup()
            },
        );
        let mut v = serde_json::to_value(&g).expect("serializes");
        v["random"] = serde_json::json!([[1, 0], [1, 1]]);
        let mut g: Game = serde_json::from_value(v).expect("deserializes");
        let ev = g.apply(Command::Cast {
            thing: DOOM,
            at: Some((5, 3)),
            target: None,
        });
        assert!(
            ev.iter()
                .any(|e| matches!(e, Event::Died { who, .. } if *who == cr(1))),
            "{ev:?}"
        );
        assert!(g.creature(1, 1).is_none());
        assert!(g.is_random(1, 1), "the dead creature's slot keeps its mark");
        while g.current() != 0 {
            g.apply(Command::Wait);
        }
        let ev = g.apply(Command::Cast {
            thing: CALL,
            at: None,
            target: None,
        });
        assert_eq!(summoned(&ev).map(|s| s.0), Some(0), "{ev:?}");
        assert!(!g.is_random(1, 0), "the summons clears it");
        assert!(g.is_random(1, 1));
    });
}

// ---- capture.md 10.43: creatures casting, half power, a death's marks ----

/// The SpellCast and ItemUsed events of creature `slot`, in order.
fn magic_of(ev: &[Event], slot: usize) -> Vec<(bool, u16, Who)> {
    ev.iter()
        .filter_map(|e| match e {
            Event::SpellCast {
                by, thing, target, ..
            } if *by == cr(slot) => Some((true, *thing, *target)),
            Event::ItemUsed { by, thing, target } if *by == cr(slot) => Some((false, *thing, *target)),
            _ => None,
        })
        .collect()
}

fn owning(things: &[u16]) -> Vec<Possession> {
    things
        .iter()
        .map(|&thing| Possession { thing, count: 1 })
        .collect()
}

#[test]
fn a_peaceful_friend_casts_and_its_summons_lands_on_the_goal() {
    // 6.2 (capture.md 10.43 item 7): a friend casts whether a fighter or a
    // slinker; 6.4: a summons fits a friend goal too, made on its square.
    both(|p| {
        let mut a = adventure(p);
        put_creature(&mut a, 1, 6, 3, |c| {
            c.strategy.alignment = Alignment::Friend;
            c.strategy.aggressive = false;
            c.wisdom = 20;
            c.power = 30;
            c.possessions = owning(&[HEAL]);
        });
        let mut g = start(
            a,
            2,
            2,
            PlayerSetup {
                life_force: 3,
                ..setup()
            },
        );
        let ev = g.apply(Command::Wait);
        assert_eq!(magic_of(&ev, 1), vec![(true, HEAL, ME)], "{ev:?}");

        let mut a = adventure(p);
        put_creature(&mut a, 1, 6, 3, |c| {
            c.strategy.alignment = Alignment::Friend;
            c.strategy.aggressive = false;
            c.wisdom = 20;
            c.power = 30;
            c.possessions = owning(&[CALL]);
        });
        let mut g = start(a, 2, 2, setup());
        let ev = g.apply(Command::Wait);
        assert_eq!(magic_of(&ev, 1), vec![(true, CALL, ME)], "{ev:?}");
        let (_, creature, at) = summoned(&ev).expect("summoned");
        assert_eq!((creature, at), (IMP, hall(2, 2)));
    });
}

#[test]
fn a_peaceful_enemy_still_casts_nothing() {
    // 6.2: other creatures cast and use items only as fighters.
    both(|p| {
        let mut a = adventure(p);
        put_creature(&mut a, 1, 6, 3, |c| {
            c.strategy.aggressive = false;
            c.wisdom = 20;
            c.power = 30;
            c.possessions = owning(&[CALL, ROD]);
        });
        let mut g = start(a, 2, 2, setup());
        let ev = g.apply(Command::Wait);
        assert!(magic_of(&ev, 1).is_empty(), "{ev:?}");
    });
}

#[test]
fn an_enemy_summons_at_its_foe_goal() {
    // 6.4: the summons fits a foe goal as well, made on its square.
    both(|p| {
        let mut a = adventure(p);
        put_creature(&mut a, 1, 7, 5, |c| {
            c.wisdom = 20;
            c.power = 20;
            c.possessions = owning(&[CALL]);
        });
        let mut g = start(a, 2, 2, setup());
        let ev = g.apply(Command::Wait);
        assert_eq!(magic_of(&ev, 1), vec![(true, CALL, ME)], "{ev:?}");
        assert_eq!(summoned(&ev).map(|s| s.2), Some(hall(2, 2)));
    });
}

#[test]
fn half_power_is_power_after_drift_above_half_the_wisdom() {
    // 6.5 (capture.md 10.43 item 7): at Wisdom 20 a Power of 11 uses the
    // item, 10 does not; at 15, 8 does and 7 does not (2 x Power >
    // Wisdom). The Power is the one after the turn's drift (5.1). 6.3:
    // casting needs Power after drift at least the Wisdom: 19 drifts to 20.
    both(|p| {
        for (wisdom, power, used) in [(20, 12, true), (20, 9, false), (15, 9, true), (15, 6, false)] {
            let mut a = adventure(p);
            put_creature(&mut a, 1, 7, 5, |c| {
                c.wisdom = wisdom;
                c.power = power;
                c.possessions = owning(&[ROD]);
            });
            let mut g = start(a, 2, 2, setup());
            let ev = g.apply(Command::Wait);
            let want = if used { vec![(false, ROD, ME)] } else { vec![] };
            assert_eq!(magic_of(&ev, 1), want, "Wisdom {wisdom}, Power {power}");
        }
        let mut a = adventure(p);
        put_creature(&mut a, 1, 7, 5, |c| {
            c.wisdom = 20;
            c.power = 19;
            c.possessions = owning(&[HEAL, BOLT]);
        });
        let mut g = start(a, 2, 2, setup());
        let ev = g.apply(Command::Wait);
        assert_eq!(magic_of(&ev, 1).first().map(|m| (m.0, m.1)), Some((true, BOLT)));
    });
}

#[test]
fn a_creatures_use_costs_two_moves_and_its_turn_goes_on() {
    // 6.5: the use spends 2 Power and two moves; with moves left the
    // creature walks on, with fewer than two its turn ends. At most one
    // use a turn.
    both(|p| {
        for (speed, steps) in [(3u8, 1usize), (2, 0), (1, 0)] {
            let mut a = adventure(p);
            put_creature(&mut a, 1, 7, 5, |c| {
                c.wisdom = 20;
                c.power = 28;
                c.speed = speed;
                c.possessions = owning(&[ROD]);
            });
            let mut g = start(a, 2, 2, setup());
            let ev = g.apply(Command::Wait);
            assert_eq!(magic_of(&ev, 1), vec![(false, ROD, ME)], "Speed {speed}");
            let moved = ev
                .iter()
                .filter(|e| matches!(e, Event::CreatureMoved { slot: 1, .. }))
                .count();
            assert_eq!(moved, steps, "Speed {speed}: {ev:?}");
            // Drift 28 to 27, then the use's 2.
            assert_eq!(g.creature(1, 1).unwrap().power, 25);
        }
        let mut a = adventure(p);
        put_creature(&mut a, 1, 7, 5, |c| {
            c.wisdom = 20;
            c.power = 28;
            c.speed = 8;
            c.possessions = owning(&[ROD]);
        });
        let mut g = start(a, 2, 2, setup());
        let ev = g.apply(Command::Wait);
        assert_eq!(magic_of(&ev, 1).len(), 1, "{ev:?}");
    });
}

#[test]
fn at_a_death_the_owner_only_mark_alone_loses_a_weapon() {
    // 7.2 (capture.md 10.43 item 10): a blade of weight 0 anyone may use
    // goes down; one of weight 5 only its owner may use is lost, as is the
    // claw with both marks.
    const OPEN0: u16 = 6;
    const OWN5: u16 = 7;
    both(|p| {
        let mut a = adventure(p);
        let blade = |weight, anyone| ThingKind::MeleeWeapon {
            picture: 6,
            disappears_after_use: false,
            carry: Carry { weight, value: 0 },
            stats: WeaponStats {
                power: 3,
                range: None,
                break_chance: 0,
                attack_adjustment: 0,
                usable_by_anyone: anyone,
                magic: false,
            },
        };
        a.things[OPEN0 as usize - 1].kind = blade(0, true);
        a.things[OWN5 as usize - 1].kind = blade(5, false);
        put_creature(&mut a, 1, 5, 3, |c| {
            c.strategy.alignment = Alignment::Friend;
            c.possessions = owning(&[CLAW, OPEN0, OWN5]);
            c.weapon = OPEN0;
        });
        let mut g = start(
            a,
            2,
            2,
            PlayerSetup {
                possessions: vec![DOOM, AMULET],
                ..setup()
            },
        );
        let ev = g.apply(Command::Cast {
            thing: DOOM,
            at: None,
            target: Some(cr(1)),
        });
        assert!(
            ev.contains(&Event::Died {
                who: cr(1),
                at: hall(5, 3),
                left: vec![(OPEN0, 1)]
            }),
            "{ev:?}"
        );
        assert_eq!(stack(&g, 0, 5, 3), vec![OPEN0]);
    });
}

// ---- things that disappear, custom obstacles' bits (capture.md 10.45) -----

const VGIFT: u16 = 104; // space, Give GEM, disappears
const VLIGHT: u16 = 105; // custom space, owners of (placement), Life Force +5, disappears
const VMOVE: u16 = 106; // custom space, when moved onto, Give (placement), disappears
const VDROP: u16 = 107; // custom space, when dropped here, Give GEM, disappears
const VROCK: u16 = 108; // obstacle, no spell, disappears
const IDOL: u16 = 109; // custom obstacle, Give AMULET (master), bump bit, stays
const IDOL_P: u16 = 110; // custom obstacle, Give, bump bit, placement parameter
const IDOL_X: u16 = 111; // custom obstacle, Give AMULET, no bump bit, disappears
const IDOL_0: u16 = 112; // custom obstacle, Give 0 (master), bump bit, disappears
const VDOALL: u16 = 113; // space, Activate All, disappears
const NEED: u16 = 114; // custom space, owners of CLAW (master), destroys it, Give (placement), disappears
const VSONG: u16 = 115; // portal, invokes Play Music, bit 4 (ignored)
const ADDER: u16 = 116; // space, Add to Room VGIFT

/// Property byte 0 bit 4: disappears; bit 7: the placement choice.
const GOES: u8 = 0x10;
const PLACED: u8 = 0x80;
/// Property byte 1 bit 6: a bump invokes the spell.
const BUMPS: u8 = 0x40;

/// Long message 8: a word split across the first line end, two spaces
/// kept, the second line padded with blanks.
fn split_message() -> String {
    format!(
        "{:>32}{:<32}{}",
        "TWO  SPACES THEN A WO", "RD SPLIT", "ACROSS LINES"
    )
}

fn disappearing(a: &mut Adventure) {
    let mut set = |n: u16, name: &str, kind: ThingKind, raw: [u8; 4]| {
        let t = &mut a.things[n as usize - 1];
        t.name = name.into();
        t.kind = kind;
        t.raw = raw;
    };
    let idol = |action| ThingKind::CustomObstacle { picture: 7, action };
    set(VGIFT, "VGIFT", space(act(K::Give, GEM)), [GOES, 0, 0, 0]);
    set(
        VLIGHT,
        "VLIGHT",
        custom(
            CustomTrigger::OpenToOwnersOf,
            PlacementChoice::Thing,
            act(K::ChangeLifeForce, life(5, false)),
        ),
        [GOES, 0, 0, 0],
    );
    set(
        VMOVE,
        "VMOVE",
        custom(
            CustomTrigger::WhenMovedOnto,
            PlacementChoice::SpellParameter,
            act(K::Give, 0),
        ),
        [GOES | PLACED, 0, 0, 0],
    );
    set(
        VDROP,
        "VDROP",
        custom(
            CustomTrigger::WhenDroppedHere,
            PlacementChoice::Thing,
            act(K::Give, GEM),
        ),
        [GOES, 0, 0, 0],
    );
    set(
        VROCK,
        "VROCK",
        ThingKind::Obstacle {
            picture: 7,
            bump_invokes_spell: false,
            action: act(K::Nothing, 0),
        },
        [GOES, 0, 0, 0],
    );
    set(IDOL, "IDOL", idol(act(K::Give, AMULET)), [0, BUMPS, 0, 0]);
    set(
        IDOL_P,
        "IDOL_P",
        idol(act(K::Give, AMULET)),
        [PLACED, BUMPS, 0, 0],
    );
    set(IDOL_X, "IDOL_X", idol(act(K::Give, AMULET)), [GOES, 0, 0, 0]);
    set(IDOL_0, "IDOL_0", idol(act(K::Give, 0)), [GOES, BUMPS, 0, 0]);
    set(VDOALL, "VDOALL", space(act(K::ActivateAll, 0)), [GOES, 0, 0, 0]);
    set(
        NEED,
        "NEED",
        ThingKind::CustomSpace {
            picture: 4,
            placement_choice: PlacementChoice::SpellParameter,
            trigger: CustomTrigger::OpenToOwnersOf,
            action: act(K::Give, CLAW),
            destroy_needed_thing: true,
        },
        [GOES | PLACED, 0, 0, 0],
    );
    set(
        VSONG,
        "VSONG",
        ThingKind::Portal {
            picture: 5,
            two_way: false,
            access: Access::InvokeSpell {
                action: act(K::PlayMusic, 4),
            },
            destroy_needed_thing: false,
        },
        [GOES, 0, 0, 0],
    );
    set(ADDER, "ADDER", space(act(K::AddToRoom, VGIFT)), [0; 4]);
}

fn given(ev: &[Event]) -> Vec<u16> {
    ev.iter()
        .filter_map(|e| match e {
            Event::Given { thing, .. } => Some(*thing),
            _ => None,
        })
        .collect()
}

fn gone(ev: &[Event]) -> Vec<u16> {
    ev.iter()
        .filter_map(|e| match e {
            Event::ObstacleVanished { thing, .. } => Some(*thing),
            _ => None,
        })
        .collect()
}

fn fired(ev: &[Event]) -> Vec<u16> {
    ev.iter()
        .filter_map(|e| match e {
            Event::SpellTriggered { thing, .. } => Some(*thing),
            _ => None,
        })
        .collect()
}

#[test]
fn a_space_with_bit_4_goes_after_its_spell_and_what_lies_below_acts_next() {
    // Movement 5.9: the space goes at the step onto it, after its spell,
    // the player on its square; the thing below acts at the next step.
    both(|p| {
        let mut a = adventure(p);
        disappearing(&mut a);
        put_item(&mut a, 0, 3, 1, TUNE, 0);
        put_item(&mut a, 0, 3, 1, VGIFT, 0);
        let mut g = start(a, 2, 1, setup());
        let ev = mv(&mut g, Dir::E);
        assert_eq!(given(&ev), vec![GEM]);
        assert_eq!(gone(&ev), vec![VGIFT]);
        let at_given = ev.iter().position(|e| matches!(e, Event::Given { .. }));
        let at_gone = ev
            .iter()
            .position(|e| matches!(e, Event::ObstacleVanished { .. }));
        assert!(at_given < at_gone, "after its spell: {ev:?}");
        assert_eq!(g.current_player().at, hall(3, 1));
        assert_eq!(stack(&g, 0, 3, 1), vec![TUNE]);
        mv(&mut g, Dir::W);
        let ev = mv(&mut g, Dir::E);
        assert!(ev.contains(&Event::Music { number: 13 }));
        assert!(gone(&ev).is_empty());
        assert_eq!(stack(&g, 0, 3, 1), vec![TUNE]);
    });
}

#[test]
fn a_space_put_back_by_add_to_room_works_and_goes_again() {
    // Movement 5.9: a spell space with the bit over a space adding it gave
    // its thing and went, came back at the next step, gave and went again.
    both(|p| {
        let mut a = adventure(p);
        disappearing(&mut a);
        put_item(&mut a, 0, 3, 1, ADDER, 0);
        put_item(&mut a, 0, 3, 1, VGIFT, 0);
        let mut g = start(a, 2, 1, setup());
        let ev = mv(&mut g, Dir::E);
        assert_eq!((given(&ev), gone(&ev)), (vec![GEM], vec![VGIFT]));
        assert_eq!(stack(&g, 0, 3, 1), vec![ADDER]);
        mv(&mut g, Dir::W);
        let ev = mv(&mut g, Dir::E);
        assert!(gone(&ev).is_empty());
        assert_eq!(stack(&g, 0, 3, 1), vec![VGIFT, ADDER]);
        mv(&mut g, Dir::W);
        let ev = mv(&mut g, Dir::E);
        assert_eq!((given(&ev), gone(&ev)), (vec![GEM], vec![VGIFT]));
        assert_eq!(stack(&g, 0, 3, 1), vec![ADDER]);
        assert_eq!(g.current_player().count(GEM), 2);
    });
}

#[test]
fn custom_spaces_with_bit_4_go_when_set_off_and_a_refused_one_stays() {
    // Movement 5.9: a "when moved onto" and a passed "owners of" space go
    // after the spell; a refused "owners of" stays however often tried;
    // a "when dropped here" one is not set off by a step (engine choice).
    both(|p| {
        let mut a = adventure(p);
        disappearing(&mut a);
        put_item(&mut a, 0, 3, 1, VLIGHT, GEM as u8);
        put_item(&mut a, 0, 2, 2, VMOVE, GEM as u8);
        put_item(&mut a, 0, 1, 1, VDROP, 0);
        let mut g = start(
            a,
            2,
            1,
            PlayerSetup {
                life_force: 5,
                ..setup()
            },
        );
        for _ in 0..2 {
            let ev = mv(&mut g, Dir::E);
            assert!(matches!(
                ev[0],
                Event::Blocked {
                    reason: BlockReason::NeedsThing { thing: GEM },
                    ..
                }
            ));
            assert!(gone(&ev).is_empty());
            assert_eq!(stack(&g, 0, 3, 1), vec![VLIGHT]);
        }
        let ev = mv(&mut g, Dir::W);
        assert!(given(&ev).is_empty() && gone(&ev).is_empty(), "{ev:?}");
        assert_eq!(stack(&g, 0, 1, 1), vec![VDROP]);
        mv(&mut g, Dir::E);
        // The placement is the spell's parameter: GEM.
        let ev = mv(&mut g, Dir::S);
        assert_eq!((given(&ev), gone(&ev)), (vec![GEM], vec![VMOVE]));
        assert_eq!(stack(&g, 0, 2, 2), Vec::<u16>::new());
        mv(&mut g, Dir::N);
        let ev = mv(&mut g, Dir::E);
        assert_eq!(gone(&ev), vec![VLIGHT]);
        assert_eq!(g.current_player().at, hall(3, 1));
        assert_eq!(lf(&g, ME), 10);
        assert_eq!(stack(&g, 0, 3, 1), Vec::<u16>::new());
    });
}

#[test]
fn custom_obstacles_follow_their_bump_bit_and_placement_choice() {
    // Movement 5.5: the spell only with bit 6 of byte 1; the parameter the
    // placement's with bit 7 of byte 0, else the master's. 5.9: one with
    // bit 4 goes at the first bump, its spell run or not. Magic 3.9: a
    // Give of 0 gives thing 80.
    both(|p| {
        let mut a = adventure(p);
        disappearing(&mut a);
        put_item(&mut a, 0, 2, 1, IDOL, GEM as u8);
        put_item(&mut a, 0, 3, 2, IDOL_P, GEM as u8);
        put_item(&mut a, 0, 1, 3, IDOL_X, GEM as u8);
        put_item(&mut a, 0, 4, 3, IDOL_0, GEM as u8);
        put_item(&mut a, 0, 2, 3, VROCK, 0);
        let mut g = start(a, 2, 2, setup());
        // IDOL: the master's AMULET, bump after bump; it stays.
        for _ in 0..2 {
            let ev = mv(&mut g, Dir::N);
            assert_eq!((given(&ev), gone(&ev)), (vec![AMULET], vec![]));
        }
        assert_eq!(stack(&g, 0, 2, 1), vec![IDOL]);
        // IDOL_P: the placement's GEM.
        let ev = mv(&mut g, Dir::E);
        assert_eq!(given(&ev), vec![GEM]);
        // VROCK: a plain obstacle with the bit and no spell goes; the
        // square is then open.
        let ev = mv(&mut g, Dir::S);
        assert_eq!((fired(&ev), gone(&ev)), (vec![], vec![VROCK]));
        assert_eq!(g.current_player().at, hall(2, 2));
        mv(&mut g, Dir::S);
        assert_eq!(g.current_player().at, hall(2, 3));
        // IDOL_X: no bump bit: no spell, but it goes.
        let ev = mv(&mut g, Dir::W);
        assert!(matches!(
            ev[0],
            Event::Blocked {
                reason: BlockReason::Obstacle { thing: IDOL_X },
                ..
            }
        ));
        assert_eq!((fired(&ev), gone(&ev)), (vec![], vec![IDOL_X]));
        assert_eq!(stack(&g, 0, 1, 3), Vec::<u16>::new());
        // IDOL_0: the master's 0 gives thing 80, then it goes.
        g.place(0, hall(3, 3));
        let ev = mv(&mut g, Dir::E);
        assert_eq!((given(&ev), gone(&ev)), (vec![80], vec![IDOL_0]));
        assert!(g.current_player().owns(80));
        assert_eq!(g.current_player().at, hall(3, 3));
    });
}

#[test]
fn activate_all_uses_up_a_needed_thing_and_takes_what_disappears() {
    // Magic 3.14, movement 5.7, 5.9: top to bottom, VDOALL (bit 4), IDOL_X
    // (no bump bit: runs anyway, the master's AMULET), NEED for GEM and
    // NEED for DUST (owners of CLAW, destroying it), VGIFT (GEM), VSONG
    // (a portal: bit 4 ignored), TUNE. The first NEED takes the CLAW, so
    // the second fails and stays; every bit-4 thing whose spell ran goes,
    // the Activate All space on top too.
    both(|p| {
        let mut a = adventure(p);
        disappearing(&mut a);
        for (t, param) in [
            (TUNE, 0),
            (VSONG, 0),
            (VGIFT, 0),
            (NEED, DUST),
            (NEED, GEM),
            (IDOL_X, 0),
            (VDOALL, 0),
        ] {
            put_item(&mut a, 0, 3, 2, t, param as u8);
        }
        let mut g = start(
            a,
            2,
            2,
            PlayerSetup {
                possessions: vec![CLAW],
                ..setup()
            },
        );
        let ev = mv(&mut g, Dir::E);
        assert_eq!(fired(&ev), vec![VDOALL, IDOL_X, NEED, VGIFT, VSONG, TUNE]);
        assert_eq!(given(&ev), vec![AMULET, GEM, GEM]);
        assert!(ev.contains(&Event::ThingUsedUp {
            player: 0,
            thing: CLAW
        }));
        assert!(!g.current_player().owns(CLAW));
        assert_eq!(gone(&ev), vec![IDOL_X, NEED, VGIFT, VDOALL]);
        assert_eq!(g.current_player().at, hall(3, 2));
        assert_eq!(stack(&g, 0, 3, 2), vec![NEED, VSONG, TUNE]);
        let left = g.adventure().regions[0].rooms[0]
            .items
            .iter()
            .find(|it| (it.x, it.y, it.thing) == (3, 2, NEED))
            .map(|it| it.parameter);
        assert_eq!(left, Some(DUST as u8));
        // The next step runs only what is left: NEED refuses (no CLAW).
        mv(&mut g, Dir::W);
        let ev = mv(&mut g, Dir::E);
        assert!(matches!(
            ev[0],
            Event::Blocked {
                reason: BlockReason::NeedsThing { thing: CLAW },
                ..
            }
        ));
        assert_eq!(stack(&g, 0, 3, 2), vec![NEED, VSONG, TUNE]);
    });
}

#[test]
fn a_long_message_keeps_its_stored_lines() {
    // 3.10: the stored lines of 32, one per line, a split word left
    // split, spaces kept, trailing blanks trimmed.
    both(|p| {
        let mut a = adventure(p);
        put_item(&mut a, 0, 3, 1, SCROLL, 0);
        a.things[SCROLL as usize - 1].kind = space(act(K::LongMessage, 8));
        let mut g = start(a, 2, 1, setup());
        let ev = mv(&mut g, Dir::E);
        let i = ev
            .iter()
            .position(|e| matches!(e, Event::LongMessage { number: 8, .. }))
            .expect("a long message");
        let Event::LongMessage { text, .. } = &ev[i] else {
            unreachable!()
        };
        assert_eq!(text, "           TWO  SPACES THEN A WO\nRD SPLIT\nACROSS LINES");
        assert_eq!(describe(&g, &ev[i]).lines().count(), 3);
    });
}

#[test]
fn an_endings_trigger_wins_after_its_spell_and_play_stops() {
    // Winning 4.2, 4.3: the player stepping onto the trigger in its room
    // wins once the command's events are out, its spell first; from then
    // every command is ignored and none logged.
    both(|p| {
        let mut a = adventure(p);
        put_item(&mut a, 0, 3, 1, TUNE, 0);
        a.endings = vec![acs_core::model::Ending {
            region: 1,
            room: 0,
            thing: TUNE,
            message: 7,
        }];
        let mut g = start(a.clone(), 2, 1, setup());
        let ev = mv(&mut g, Dir::E);
        let music = ev.iter().position(|e| matches!(e, Event::Music { .. }));
        assert!(music.is_some(), "{ev:?}");
        assert_eq!(ev.last(), Some(&Event::AdventureWon { ending: 0, player: 0 }));
        assert_eq!(g.won(), Some(acs_engine::Won { ending: 0, player: 0 }));
        let logged = g.log().len();
        assert!(mv(&mut g, Dir::W).is_empty());
        assert!(g.apply(Command::Wait).is_empty());
        assert_eq!(g.log().len(), logged);
        assert_eq!(g.current_player().at, hall(3, 1));
    });
}

#[test]
fn a_win_on_the_last_move_starts_no_one_elses_turn() {
    // Winning 4.2: a Speed 1 player steps onto the trigger with its only
    // move. The turn's end is the command's last turn event: no creature
    // turn, no other player's turn, no round's end after the win.
    both(|p| {
        let mut a = adventure(p);
        put_item(&mut a, 0, 3, 1, TUNE, 0);
        a.endings = vec![acs_core::model::Ending {
            region: 1,
            room: 0,
            thing: TUNE,
            message: 7,
        }];
        let one = PlayerSetup { speed: 1, ..setup() };
        let mut g = start(a, 2, 1, one);
        let ev = mv(&mut g, Dir::E);
        assert_eq!(ev.last(), Some(&Event::AdventureWon { ending: 0, player: 0 }));
        let ended = ev.iter().position(|e| matches!(e, Event::TurnEnded { .. }));
        assert!(ended.is_some(), "{ev:?}");
        assert!(
            ev[ended.unwrap() + 1..]
                .iter()
                .all(|e| matches!(e, Event::AdventureWon { .. })),
            "{ev:?}"
        );
    });
}

#[test]
fn an_ending_under_activate_all_wins_but_not_in_another_room() {
    // Winning 4.2: a trigger an Activate All above it runs counts; the
    // same thing set off in a room the ending doesn't name doesn't.
    both(|p| {
        let mut a = adventure(p);
        put_item(&mut a, 0, 3, 1, TUNE, 0);
        put_item(&mut a, 0, 3, 1, DOALL, 0);
        a.endings = vec![acs_core::model::Ending {
            region: 1,
            room: 0,
            thing: TUNE,
            message: 7,
        }];
        let mut g = start(a.clone(), 2, 1, setup());
        let ev = mv(&mut g, Dir::E);
        assert!(ev.contains(&Event::Music { number: 13 }), "{ev:?}");
        assert_eq!(g.won().map(|w| w.ending), Some(0), "{ev:?}");
        let mut a = adventure(p);
        put_item(&mut a, 0, 3, 1, TUNE, 0);
        a.endings = vec![acs_core::model::Ending {
            region: 1,
            room: 1,
            thing: TUNE,
            message: 7,
        }];
        let mut g = start(a, 2, 1, setup());
        mv(&mut g, Dir::E);
        assert_eq!(g.won(), None);
    });
}
