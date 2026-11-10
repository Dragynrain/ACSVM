// SPDX-License-Identifier: GPL-3.0-or-later

//! End-to-end decoding of synthetic adventures built byte by byte.
//! Every test runs for both PC and C64 layouts. PC is covered *only* here:
//! no real PC adventure has been decoded yet.

mod common;

use acs_core::layout::{adventure as adv, world_map as wm};
use acs_core::model::*;
use acs_core::{decode, DiagnosticKind, Error, Platform, Severity};
use common::{creature_record, Fixture, RegionSpec, RoomSpec};

const PLATFORMS: [Platform; 2] = [Platform::Pc, Platform::C64];

fn sample(platform: Platform) -> Fixture {
    let mut f = Fixture::new(platform);
    f.header(
        "TEST QUEST",
        "BY NOBODY",
        "WELCOME, BRAVE TESTER. THIS LINE IS EXACTLY 32 CHARS",
    );

    // Things (props byte 0: type | disappears 0x10 | weight mult bits 5-6 | value x20 0x80)
    f.thing(1, "IRON SWORD", [0x84, 30, 5, 0]);
    f.extended(None, 0, [0x0C, 0x93, 0x40 | 20]);
    f.thing(2, "LONG BOW", [0x03, 40, 10, 1]);
    f.extended(None, 1, [0x05 | (4 << 5), 0x70, 0x80 | 21]);
    f.thing(3, "CHAIN MAIL", [0x05 | 0x40, 10, 50, 2]);
    f.extended(None, 2, [0x08, 0xF0, 22]);
    f.thing(4, "MAGIC BAG", [0x01 | 0x20 | 0x10, 100, 3, 23]);
    f.thing(5, "HEAL RING", [0x02, 1, 50, 0]);
    f.extended(Some(9), 0, [0x88, 0x40 | 24, 0x05]);
    f.thing_message(5, 1, 3);
    f.thing(6, "FIREBALL", [0x06, 30, 0x08, 0x85]);
    f.thing_message(6, 1, 0x80 | 2);
    f.thing(7, "OAK DOOR", [0x07 | 0x20, 25, 0, 0]);
    f.thing_message(7, 0, 1);
    f.thing(8, "PIT", [0x08, 0x40 | 26, 0x08, 0x8A]);
    f.thing(9, "ALTAR", [0x09 | 0x80, 0x40 | 27, 0x80 | 0x05, 0x03]);
    f.thing(10, "WALL", [0x0A, 28, 0, 0]);
    f.thing(11, "STATUE", [0x0B, 29, 0x0B, 0]);
    f.thing(12, "SHOP", [0x0C, 30, 0, 0]);
    f.thing(13, "GRASS", [0x00, 31, 0, 0]);
    f.thing(14, "ONE WAY", [0x07, 25, 0, 0]);
    let acs_adds = f.l.things + acs_core::layout::things::ACS_ADDS;
    f.put(acs_adds, &[0x30]);

    f.short_message(1, "THE DOOR IS LOCKED.");
    f.short_message(3, "YOU FEEL BETTER.");
    f.long_message(1, "A LONG MESSAGE.");

    // Classes and one master creature.
    let classes = f.l.creatures;
    f.ascii(classes, "MONSTER", 15);
    f.ascii(classes + 15, "BEAST", 15);
    f.ascii(classes + 30, "GOBLINOID", 15);
    let mut gob = creature_record(0x40 | 20, 0xE7, 70);
    gob[0x09] = 0x80 | 35;
    gob[0x12] = 0x02;
    gob[0x13] = 0x01;
    gob[0x14] = 1;
    gob[0x15] = 3;
    gob[0x17] = 0b01_00_00_11;
    gob[0x1F] = 0x80;
    f.creature(1, "GOBLIN", 2, gob);

    // World map.
    let w = f.l.world_map;
    let a = f.l.adventure;
    f.put(a + adv::MAP_WRAP, &[1]);
    f.ascii(a + adv::MAP_NAME, "THE WORLD", adv::MAP_NAME_LEN);
    f.put(w + wm::TERRAIN, &[0x40 | 3, 0, 0x04, 0, 0x80 | 5, 4]);
    f.ascii(w + wm::TERRAIN_NAMES, "GRASS", 15);
    f.ascii(w + wm::TERRAIN_NAMES + 15, "MOUNTAIN", 15);
    f.ascii(w + wm::TERRAIN_NAMES + 30, "FOREST", 15);
    f.put(w + wm::MAP, &[0x12]);
    f.put(w + wm::START_X, &[20, 21]);
    f.put(w + wm::POINTS, &[10, 11, 12, 12]); // doors 0 and 1
    f.put(w + wm::POINTS + (32 + 1) * 2, &[30, 31]); // exit point 33
    f.put(w + wm::DOOR_DESTINATIONS, &[0x10, 0x23, 0x00, 0x00]);
    f.packed(w + wm::CREATURE_NAMES, "WOLF");
    f.put(w + wm::CREATURES, &creature_record(10, 0x60, 66));
    f.put(w + wm::CREATURE_CHANCES, &[0x27]);

    f.put(
        f.l.palette,
        &match platform {
            Platform::Pc => [0x01, 0x0E, 0x04, 0x0F],
            Platform::C64 => [0x00, 0x06, 0x02, 0x0B],
        },
    );

    f.region(RegionSpec {
        name: "DUNGEON",
        rooms: vec![
            RoomSpec {
                name: "HALL",
                w: 10,
                h: 8,
                x: 3,
                y: 4,
                wall: 5,
                random: 0x23,
                items: vec![(1, 1, 1, 0), (0, 4, 7, 1), (5, 5, 14, 2)],
            },
            RoomSpec {
                name: "CELL",
                w: 4,
                h: 4,
                ..Default::default()
            },
        ],
        portals: vec![[0, 0], [0x21, 0x34], [0x00, 0x21]],
        ..Default::default()
    });
    let mut guard = creature_record(15, 0x50, 80);
    guard[0x0D] = 2;
    guard[0x0E] = 2;
    guard[0x10] = 0x21;
    f.region(RegionSpec {
        name: "TOWER",
        rooms: vec![
            RoomSpec {
                name: "STAIRS",
                w: 3,
                h: 9,
                ..Default::default()
            },
            RoomSpec {
                name: "TOP",
                w: 6,
                h: 6,
                items: vec![(2, 0, 7, 1)],
                ..Default::default()
            },
        ],
        portals: vec![[0, 0], [0x10, 0x22]],
        // Pushes every heap pointer past 0x200, where ACS Viewer's fixed
        // resident-name offset would read the wrong bytes.
        extra_door_entries: 0xA8,
        header_03: None,
        residents: vec![(0, "GUARD", guard)],
    });
    f
}

#[test]
fn decodes_sample_on_both_platforms() {
    for p in PLATFORMS {
        let img = sample(p).build();
        let d = decode(&img).unwrap_or_else(|e| panic!("{p}: {e}"));
        assert!(d.diagnostics.is_empty(), "{p}: {:?}", d.diagnostics);
        check_sample(&d.adventure, p);
    }
}

fn check_sample(a: &Adventure, p: Platform) {
    assert_eq!(a.platform, p);
    assert_eq!(a.name, "TEST QUEST");
    assert_eq!(a.byline, "BY NOBODY");
    assert_eq!(a.intro, "WELCOME, BRAVE TESTER. THIS LINE\n IS EXACTLY 32 CHARS");
    assert_eq!(a.theme, Theme::Fantasy);
    let pal: Vec<&str> = a.palette.colors.iter().map(|c| c.name.as_str()).collect();
    match p {
        Platform::Pc => {
            assert_eq!(pal, ["blue", "yellow", "red", "white"]);
            assert_eq!(a.pictures.len(), 96);
            assert_eq!((a.pictures[0].width, a.pictures[0].pixel_aspect), (16, 1));
        }
        Platform::C64 => {
            assert_eq!(pal, ["black", "blue", "red", "cyan"]);
            assert_eq!(a.pictures.len(), 110);
            assert_eq!((a.pictures[0].width, a.pictures[0].pixel_aspect), (8, 2));
        }
    }

    // Things
    let t = |n: usize| &a.things[n - 1];
    assert_eq!(a.things.len(), 128);
    assert_eq!(t(1).name, "IRON SWORD");
    assert_eq!(t(1).acs_may_add, 3);
    match &t(1).kind {
        ThingKind::MeleeWeapon {
            picture,
            carry,
            stats,
            ..
        } => {
            assert_eq!(*picture, 20);
            assert_eq!(
                *carry,
                Carry {
                    weight: 30,
                    value: 100
                }
            );
            assert_eq!(
                (stats.power, stats.break_chance, stats.attack_adjustment),
                (12, 3, 10)
            );
            assert!(stats.usable_by_anyone && !stats.magic && stats.range.is_none());
        }
        k => panic!("thing 1: {k:?}"),
    }
    match &t(2).kind {
        ThingKind::MissileWeapon { stats, picture, .. } => {
            assert_eq!(
                (stats.power, stats.range, stats.attack_adjustment, *picture),
                (5, Some(6), 0, 21)
            );
            assert!(stats.magic);
        }
        k => panic!("thing 2: {k:?}"),
    }
    match &t(3).kind {
        ThingKind::Armor { carry, stats, .. } => {
            assert_eq!(carry.weight, 200);
            assert_eq!((stats.power, stats.attack_adjustment), (8, 10));
        }
        k => panic!("thing 3: {k:?}"),
    }
    match &t(4).kind {
        ThingKind::Treasure {
            carry,
            picture,
            disappears_when_dropped,
        } => {
            assert_eq!((carry.weight, carry.value, *picture), (-100, 3, 23));
            assert!(*disappears_when_dropped);
        }
        k => panic!("thing 4: {k:?}"),
    }
    match &t(5).kind {
        ThingKind::MagicItem {
            picture,
            action,
            on_use,
            on_drop,
            on_pickup,
            ..
        } => {
            assert_eq!(*picture, 24);
            assert_eq!(
                *action,
                SpellAction {
                    kind: SpellActionKind::ChangeLifeForce,
                    parameter: 5
                }
            );
            assert!(*on_use && *on_drop && !*on_pickup);
        }
        k => panic!("thing 5: {k:?}"),
    }
    assert_eq!(t(5).spell_message, Some(MessageRef { slot: 3, shown: true }));
    assert_eq!(a.short_messages[2], "YOU FEEL BETTER.");
    match &t(6).kind {
        ThingKind::MagicSpell { power, action, .. } => {
            assert_eq!(*power, 30);
            assert_eq!(
                *action,
                SpellAction {
                    kind: SpellActionKind::ChangeLifeForce,
                    parameter: 0x85
                }
            );
        }
        k => panic!("thing 6: {k:?}"),
    }
    assert_eq!(
        t(6).spell_message,
        Some(MessageRef {
            slot: 2,
            shown: false
        })
    );
    match &t(7).kind {
        ThingKind::Portal {
            picture,
            two_way,
            access,
            ..
        } => {
            assert_eq!((*picture, *two_way), (25, true));
            assert_eq!(*access, Access::Anyone);
        }
        k => panic!("thing 7: {k:?}"),
    }
    assert_eq!(t(7).blocked_message, Some(MessageRef { slot: 1, shown: true }));
    assert_eq!(a.short_messages[0], "THE DOOR IS LOCKED.");
    match &t(8).kind {
        ThingKind::Space { picture, access } => {
            assert_eq!(*picture, 26);
            assert_eq!(
                *access,
                Access::InvokeSpell {
                    action: SpellAction {
                        kind: SpellActionKind::ChangeLifeForce,
                        parameter: 0x8A
                    }
                }
            );
        }
        k => panic!("thing 8: {k:?}"),
    }
    match &t(9).kind {
        ThingKind::CustomSpace {
            placement_choice,
            trigger,
            action,
            destroy_needed_thing,
            ..
        } => {
            assert_eq!(*placement_choice, PlacementChoice::SpellParameter);
            assert_eq!(*trigger, CustomTrigger::WhenMovedOnto);
            assert_eq!(action.kind, SpellActionKind::IncreaseStat);
            assert!(destroy_needed_thing);
        }
        k => panic!("thing 9: {k:?}"),
    }
    assert!(matches!(
        t(10).kind,
        ThingKind::Obstacle {
            picture: 28,
            bump_invokes_spell: false,
            ..
        }
    ));
    assert!(matches!(
        t(11).kind,
        ThingKind::CustomObstacle {
            picture: 29,
            action: SpellAction {
                kind: SpellActionKind::PlayMusic,
                ..
            }
        }
    ));
    assert!(matches!(t(12).kind, ThingKind::Store { picture: 30 }));
    assert!(matches!(t(13).kind, ThingKind::RoomFloor { picture: 31 }));
    assert!(matches!(t(14).kind, ThingKind::Portal { two_way: false, .. }));
    assert!(matches!(t(15).kind, ThingKind::Undefined));
    assert_eq!(a.long_messages[0], "A LONG MESSAGE.");

    // Creatures
    assert_eq!(a.creature_classes[2], "GOBLINOID");
    let g = &a.creatures[0];
    assert!(g.in_use);
    assert_eq!(
        (g.name.as_str(), g.class, g.picture),
        ("GOBLIN", Some(2), Some(70))
    );
    assert_eq!(
        (g.constitution, g.speed, g.wisdom, g.melee, g.wealth),
        (20, 7, 9, 35, 258)
    );
    assert_eq!(g.magic_defense, MagicDefense::NonMagicHalfDamage);
    assert_eq!(
        g.strategy,
        Strategy {
            alignment: Alignment::Enemy,
            aggressive: true,
            brave: true
        }
    );
    assert!(g.mimic);
    assert_eq!((g.weapon, g.armor), (1, 3));
    assert_eq!(
        g.possessions,
        [
            Possession { thing: 1, count: 1 },
            Possession { thing: 4, count: 3 },
            Possession { thing: 33, count: 1 }
        ]
    );
    assert!(a.creatures[1..].iter().all(|c| !c.in_use));
    assert!(a.players.is_empty() && a.retired_players.is_empty());

    // World map
    let w = &a.world;
    assert_eq!(
        (w.name.as_str(), w.wraps, w.start),
        ("THE WORLD", true, MapPoint { x: 20, y: 21 })
    );
    assert_eq!(w.tiles.len(), 40);
    assert_eq!(&w.tiles[0][..3], &[1, 2, 0]);
    assert_eq!(w.terrain[0].passability, Passability::Open);
    assert_eq!((w.terrain[0].name.as_str(), w.terrain[0].picture), ("GRASS", 3));
    assert_eq!(w.terrain[1].passability, Passability::Impassable);
    assert_eq!(
        (w.terrain[2].passability, w.terrain[2].parameter),
        (Passability::OwnersOf, 4)
    );
    assert_eq!(w.doors.len(), 2);
    assert_eq!(w.doors[0].at, MapPoint { x: 10, y: 11 });
    assert_eq!(
        w.doors[0].destination,
        Destination::Room {
            region: 1,
            room: 0,
            x: 2,
            y: 3
        }
    );
    assert_eq!(w.doors[1].destination, Destination::WorldMap { x: 30, y: 31 });
    assert_eq!(w.exit_points[1], Some(MapPoint { x: 30, y: 31 }));
    assert_eq!(w.random_creatures[0].creature.name, "WOLF");
    assert_eq!(
        (w.random_creatures[0].terrain, w.random_creatures[0].chance),
        (2, 20)
    );

    // Regions
    assert_eq!(a.regions.len(), 2);
    let r1 = &a.regions[0];
    assert_eq!(r1.name, "DUNGEON");
    assert_eq!(r1.rooms.len(), 16);
    assert_eq!(r1.rooms.iter().filter(|r| !r.deleted).count(), 2);
    let hall = &r1.rooms[0];
    assert_eq!(
        (hall.name.as_str(), hall.width, hall.height, hall.x, hall.y),
        ("HALL", 10, 8, 3, 4)
    );
    assert_eq!(
        (hall.wall_picture, hall.random_chance, hall.random_creature),
        (5, 15, 2)
    );
    assert_eq!(hall.items.len(), 3);
    assert_eq!((hall.items[0].x, hall.items[0].y, hall.items[0].thing), (1, 1, 1));
    assert_eq!(
        hall.items[1].destination,
        Some(Destination::Room {
            region: 2,
            room: 1,
            x: 3,
            y: 4
        })
    );
    assert_eq!(
        hall.items[2].destination,
        Some(Destination::WorldMap { x: 30, y: 31 })
    );
    assert_eq!(r1.rooms[1].name, "CELL");

    let r2 = &a.regions[1];
    assert_eq!(
        r2.rooms[1].items[0].destination,
        Some(Destination::Room {
            region: 1,
            room: 0,
            x: 2,
            y: 2
        })
    );
    let guard = &r2.residents[0];
    assert!(guard.in_use);
    assert_eq!(
        (guard.name.as_str(), guard.region, guard.room, guard.x, guard.y),
        ("GUARD", 2, 1, 2, 2)
    );
}

#[test]
fn json_round_trip() {
    for p in PLATFORMS {
        let a = decode(&sample(p).build()).unwrap().adventure;
        let json = serde_json::to_string(&a).unwrap();
        let back: Adventure = serde_json::from_str(&json).unwrap();
        assert_eq!(back, a, "{p}");
    }
}

#[test]
fn rejects_program_disks() {
    for p in PLATFORMS {
        let mut f = sample(p);
        let mut img = f.build();
        img.data[f.l.adventure + adv::REGION_COUNT] = 83;
        assert!(matches!(decode(&img), Err(Error::NotAnAdventure(_))), "{p}");
    }
}

#[test]
fn reports_unknown_thing_type() {
    for p in PLATFORMS {
        let mut f = sample(p);
        f.thing(20, "ODDITY", [0x0D, 0, 0, 0]);
        let d = decode(&f.build()).unwrap();
        assert!(matches!(
            d.adventure.things[19].kind,
            ThingKind::Unknown { code: 13 }
        ));
        let errs: Vec<_> = d.errors().collect();
        assert_eq!(errs.len(), 1, "{p}");
        assert_eq!(errs[0].kind, DiagnosticKind::UnknownThingType);
    }
}

#[test]
fn detects_room_item_length_mismatch() {
    for p in PLATFORMS {
        let mut f = sample(p);
        let mut img = f.build();
        // Claim the item lists end 3 bytes later than they do.
        let base = f.l.regions;
        let end = u16::from_le_bytes([img.data[base + 1], img.data[base + 2]]) + 3;
        img.data[base + 1..base + 3].copy_from_slice(&end.to_le_bytes());
        let d = decode(&img).unwrap();
        assert!(d
            .diagnostics
            .iter()
            .any(|x| x.kind == DiagnosticKind::BadRoomItems && x.severity == Severity::Error));
    }
}

#[test]
fn c64_long_messages_skip_track_18() {
    let mut f = sample(Platform::C64);
    f.long_message(250, "AFTER THE BAM.");
    // Garbage where the BAM would be must not leak into messages 249/250.
    f.put(0x16500, &[0xFF; 0x200]);
    let a = decode(&f.build()).unwrap().adventure;
    assert_eq!(a.long_messages[249], "AFTER THE BAM.");
    assert_eq!(a.long_messages[254], a.intro);
}

/// EA's PC Rivers of Light stores other values in region header bytes 3-4 and
/// numbers its doors from 0. The door table must still be read from heap +
/// 0xF0 and indexed directly by the door's parameter.
#[test]
fn door_table_is_fixed_and_indexed_from_zero() {
    for p in PLATFORMS {
        let mut f = Fixture::new(p);
        f.header("DOORS", "", "");
        let pal = f.l.palette;
        f.put(pal, &[0x00, 0x06, 0x02, 0x0B]);
        f.thing(1, "DOOR", [0x07 | 0x20, 25, 0, 0]);
        f.region(RegionSpec {
            name: "A",
            rooms: vec![
                RoomSpec {
                    name: "WEST",
                    w: 8,
                    h: 8,
                    items: vec![(7, 3, 1, 0)],
                    ..Default::default()
                },
                RoomSpec {
                    name: "EAST",
                    w: 8,
                    h: 8,
                    items: vec![(0, 3, 1, 1)],
                    ..Default::default()
                },
            ],
            // Entry 0 used: door 0 (west room) leads to east room square (1,3) and back.
            portals: vec![[0x11, 0x13], [0x10, 0x63]],
            header_03: Some(0x00B8),
            ..Default::default()
        });
        let d = decode(&f.build()).unwrap();
        assert!(d.diagnostics.is_empty(), "{p}: {:?}", d.diagnostics);
        let r = &d.adventure.regions[0];
        assert_eq!(r.unknown_header_03, 0xB8);
        assert_eq!(r.portal_table.len(), 2);
        assert_eq!(
            r.rooms[0].items[0].destination,
            Some(Destination::Room {
                region: 1,
                room: 1,
                x: 1,
                y: 3
            })
        );
        assert_eq!(
            r.rooms[1].items[0].destination,
            Some(Destination::Room {
                region: 1,
                room: 0,
                x: 6,
                y: 3
            })
        );
    }
}
