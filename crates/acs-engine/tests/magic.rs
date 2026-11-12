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
    CastRefusal, Command, Dir, Event, Game, Location, PlayerSetup, RemovalReason, Rules, SpellSource, Stat,
    Who,
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
                target: None
            })),
            Some(CastRefusal::NotASpell)
        );
        assert_eq!(
            refused(&g.apply(Command::Cast {
                thing: CALL,
                target: None
            })),
            Some(CastRefusal::NotCarried)
        );
        assert_eq!(
            refused(&g.apply(Command::Use {
                thing: STONE,
                target: None
            })),
            Some(CastRefusal::NotUsable)
        );
        assert_eq!(
            refused(&g.apply(Command::Cast {
                thing: BOLT,
                target: Some(cr(9))
            })),
            Some(CastRefusal::BadTarget)
        );
        assert_eq!(g.current_player().moves_left, 15);

        // A bolt at the creature: cost 7 below Power 20 works 90% of the
        // time (4.1; this seed's roll works), and costs 7.
        let ev = g.apply(Command::Cast {
            thing: BOLT,
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
        // Power: 20 - 7, then 1 back toward Wisdom 20 at the end of the turn.
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
            target: None,
        });
        assert!(ev.contains(&Event::SpellGone { who: ME, thing: HEAL }));
        // +5 (from 6 healed a little at the bolt's turn end, 5.3).
        assert!(ev.iter().any(|e| matches!(
            e,
            Event::StatChanged { who: ME, stat: Stat::LifeForce, from, to } if *from > 6 && *to == *from + 5
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
        let ev = g.apply(Command::Cast {
            thing: DOOM,
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
        // gold comes in piles of up to 255.
        assert_eq!(
            died,
            vec![(
                cr(1),
                hall(5, 3),
                vec![(GEM, 1), (GEM, 1), (GOLD, 255), (GOLD, 45)]
            )]
        );
        assert!(g.creature(1, 1).is_none());
        assert!(g.creature(1, 2).is_some());
        assert!(g.current_player().alive);
        assert_eq!(stack(&g, 0, 5, 3), vec![GOLD, GOLD, GEM, GEM]);
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
        // Joining raised TWO to its Constitution (5.1): wound it again.
        g.set_life_force(1, 4);
        let ev = mv(&mut g, Dir::E);
        assert!(ev.contains(&Event::Died {
            who: ME,
            at: hall(3, 2),
            left: vec![(GEM, 1), (GOLD, 3)]
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

#[test]
fn summons_arrive_next_turn_and_banish_spares_players_and_casters() {
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
        let ev = g.apply(Command::Cast {
            thing: CALL,
            target: None,
        });
        // Cast, turn over, and at the start of the next turn the imp is on
        // the nearest free square to the caster.
        let arrived = ev.iter().find_map(|e| match e {
            Event::CreatureSummoned {
                slot, creature, at, ..
            } => Some((*slot, *creature, *at)),
            _ => None,
        });
        let turn_started = ev
            .iter()
            .position(|e| matches!(e, Event::TurnStarted { .. }))
            .expect("next turn");
        let summoned = ev
            .iter()
            .position(|e| matches!(e, Event::CreatureSummoned { .. }))
            .expect("summoned");
        assert!(summoned < turn_started);
        assert_eq!(arrived, Some((0, IMP, hall(1, 1))));
        assert_eq!(
            g.creature(1, 0).map(|c| c.name.trim().to_string()).as_deref(),
            Some("IMP")
        );
        assert!(!g.is_random(1, 0));

        let ev = g.apply(Command::Cast {
            thing: BANISH,
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
        assert_eq!(gone, vec![0, 1]);
        assert!(g.current_player().alive);

        // On the world map nobody answers.
        g.place(0, Location::World { x: 20, y: 20 });
        let ev = g.apply(Command::Cast {
            thing: CALL,
            target: None,
        });
        assert!(ev.contains(&Event::SummonUnanswered { creature: IMP }));
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
            target: None,
        });
        assert!(ev.contains(&Event::SummonUnanswered { creature: IMP }));
    });
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
        assert_eq!(g.temporary_life(ME), 10);
        // Each turn end: Power 1 toward Wisdom; healing on Life Force below
        // Constitution 11, then the temporary part's tick (1 in 3): Life
        // Force stays at 10 or 11 until the 10 temporary points are gone.
        let mut power = Vec::new();
        for _ in 0..200 {
            g.apply(Command::Wait);
            power.push(g.current_player().power);
            assert!((10..=11).contains(&lf(&g, ME)), "{}", lf(&g, ME));
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
        // 20 - 7, and the drift back up toward 15 at the end of its turn.
        assert_eq!(g.creature(1, 1).unwrap().power, 14);
        // Now its Power (14) is below its Wisdom (15): no second cast.
        let ev = g.apply(Command::Wait);
        assert!(!ev.iter().any(|e| matches!(e, Event::SpellCast { .. })), "{ev:?}");
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
        // music door; and an overburdened one with a bomb.
        put_creature(&mut a, 1, 8, 5, |c| {
            c.speed = 3;
            c.strategy.alignment = Alignment::Friend;
            c.strategy.aggressive = false; // a slinker friend: it seeks the player
        });
        put_creature(&mut a, 2, 4, 4, |c| {
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
                    target: Some(cr(1)),
                },
                Command::Wait,
                Command::Cast {
                    thing: CALL,
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
