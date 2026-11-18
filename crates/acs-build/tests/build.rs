// SPDX-License-Identifier: GPL-3.0-or-later

//! `acs build` (docs/content.md 3) on a small adventure written here:
//! it builds, its bundle loads, the engine plays it, and broken sources
//! are reported by file.

use acs_build::{build, Folder};
use acs_core::model::{Destination, SpellActionKind, ThingKind};
use acs_engine::{Command, Dir, Game, Location, PlayerSetup, Rules};

/// An atlas of eight 16 px tiles, four across, each one flat colour.
fn atlas() -> Vec<u8> {
    let colours: [[u8; 4]; 8] = [
        [200, 180, 120, 255],
        [120, 80, 40, 255],
        [220, 220, 230, 255],
        [250, 210, 0, 255],
        [90, 60, 30, 255],
        [200, 40, 40, 255],
        [0, 40, 120, 255],
        [0, 0, 0, 0],
    ];
    let (w, h) = (64u32, 32u32);
    let mut rgba = Vec::new();
    for y in 0..h {
        for x in 0..w {
            rgba.extend_from_slice(&colours[((y / 16) * 4 + x / 16) as usize]);
        }
    }
    let mut out = Vec::new();
    {
        let mut e = png::Encoder::new(&mut out, w, h);
        e.set_color(png::ColorType::Rgba);
        e.set_depth(png::BitDepth::Eight);
        e.write_header().unwrap().write_image_data(&rgba).unwrap();
    }
    out
}

const ADVENTURE: &str = r##"
name = "THE COVE"
byline = "TEST"
intro = "A SMALL COVE."
palette = ["black", "brown", "light gray", "yellow"]
classes = ["PIRATE"]

[gold]
picture = "gold"

[area_door]
picture = "door"

[floor]
picture = "sand"
wall = "plank"
"##;

const PICTURES: &str = r##"
atlas = "art/atlas.png"
tile = 16

[pictures]
sand = 0
plank = 1
cutlass = 2
gold = 3
door = 4
harrow = 5
sea = 6
clear = 7
"##;

const THINGS: &str = r##"
[[thing]]
key = "cutlass"
name = "CUTLASS"
type = "melee_weapon"
picture = "cutlass"
weight = 4
value = 30
power = 6
attack = 10

[[thing]]
key = "tonic"
name = "TONIC"
type = "magic_item"
picture = "gold"
weight = 1
value = 400
once = true
on_use = true
spell = { action = "change_life_force", by = 5, temporary = true }
spell_message = "drink"

[[thing]]
key = "door"
name = "DOOR"
type = "portal"
picture = "door"
two_way = true

[[thing]]
key = "gate"
name = "GATE"
type = "portal"
picture = "door"
access = { owners_of = "cutlass" }
blocked = "locked"

[[thing]]
key = "trader"
name = "TRADER"
type = "store"
picture = "plank"
"##;

const CREATURES: &str = r##"
[[creature]]
key = "harrow"
name = "MEG HARROW"
class = "PIRATE"
picture = "harrow"
constitution = 30
strength = 14
speed = 8
wisdom = 12
melee = 40
alignment = "friend"
weapon = "cutlass"
carries = [{ thing = "cutlass" }]
"##;

const MESSAGES: &str = r##"
[long]
welcome = "WELCOME TO THE COVE."

[short]
locked = "THE GATE IS SHUT."
drink = "YOU FEEL BETTER."
"##;

const WORLD: &str = r##"
name = "THE SEA"
start = [1, 1]
map = """
~~~~~
~..P~
~~~~~
"""

[legend]
"~" = "sea"
"." = "beach"
"P" = "cove"

[[terrain]]
key = "sea"
name = "SEA"
picture = "sea"
pass = "impassable"

[[terrain]]
key = "beach"
name = "BEACH"
picture = "sand"
pass = "open"

[[terrain]]
key = "cove"
name = "COVE"
picture = "door"
pass = "open"
door = true

[[door]]
at = [3, 1]
to = "cove.beach"
square = [1, 1]

[[encounter]]
creature = "harrow"
terrain = "beach"
chance = 0
"##;

const COVE: &str = r##"
key = "cove"
name = "THE COVE"
store = [{ thing = "tonic", count = 2 }]
random = ["harrow"]

[[room]]
key = "beach"
name = "BEACH"
at = [0, 0]
random = { creature = "harrow", chance = 0 }
map = """
#####
#.c.D
#.$.#
##E##
"""

[room.legend]
"#" = "wall"
"." = "floor"
"c" = "cutlass"
"$" = { gold = 50 }
"D" = { thing = "door", to = "cove.hut" }
"E" = { thing = "door", to = "map", square = [2, 1] }

[[room]]
key = "hut"
name = "HUT"
at = [4, 0]
map = """
#####
D.H.#
#.s.#
#####
"""

[room.legend]
"#" = "wall"
"." = "floor"
"D" = { thing = "door", to = "cove.beach" }
"H" = { creature = "harrow" }
"s" = "trader"
"##;

fn folder() -> Folder {
    Folder::default()
        .with("adventure.toml", ADVENTURE)
        .with("pictures.toml", PICTURES)
        .with("things.toml", THINGS)
        .with("creatures.toml", CREATURES)
        .with("messages.toml", MESSAGES)
        .with("world.toml", WORLD)
        .with("regions/01-cove.toml", COVE)
        .with_bytes("art/atlas.png", atlas())
}

fn built() -> acs_bundle::Bundle {
    build(&folder()).unwrap_or_else(|p| {
        panic!(
            "{}",
            p.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n")
        )
    })
}

#[test]
fn the_cove_builds_with_the_numbers_the_rules_assume() {
    let b = built();
    let a = &b.adventure;
    assert_eq!(a.name, "THE COVE");
    assert_eq!(a.things.len(), 128);
    let thing = |n: u16| &a.things[n as usize - 1];
    // Carriables from 1, fixtures from 83, the built-ins in their places.
    assert_eq!(thing(1).name, "CUTLASS");
    assert_eq!(thing(2).name, "TONIC");
    assert_eq!(thing(83).name, "DOOR");
    assert_eq!(thing(84).name, "GATE");
    assert_eq!(thing(85).name, "TRADER");
    assert!(matches!(thing(81).kind, ThingKind::Treasure { .. }));
    assert!(matches!(thing(82).kind, ThingKind::Portal { two_way: true, .. }));
    assert!(matches!(thing(127).kind, ThingKind::RoomFloor { picture: 1 }));
    assert!(matches!(thing(128).kind, ThingKind::RoomFloor { picture: 2 }));
    assert!(matches!(thing(3).kind, ThingKind::Undefined));
    match &thing(2).kind {
        ThingKind::MagicItem { action, on_use, .. } => {
            assert_eq!(action.kind, SpellActionKind::ChangeLifeForce);
            assert_eq!(action.parameter, 5 | 0x40);
            assert!(*on_use);
        }
        k => panic!("{k:?}"),
    }
    assert_eq!(thing(2).spell_message.as_ref().unwrap().slot, 2);
    assert_eq!(thing(84).blocked_message.as_ref().unwrap().slot, 1);

    // Pictures: the blank one, then the list; Classic pixels from the art.
    assert_eq!(a.pictures.len(), 9);
    assert!(a.pictures[0].is_blank());
    assert!(
        a.pictures[4].pixels.iter().all(|&p| p == 3),
        "gold is nearest yellow"
    );
    assert!(
        a.pictures[8].pixels.iter().all(|&p| p == 0),
        "transparent is colour 0"
    );
    let art = b.art.as_ref().unwrap();
    assert_eq!(art.places.get(&3), Some(&2));

    // Doors: through the shared wall both ways; to the map; the map door.
    let r = &a.regions[0];
    let door_to = |room: usize, x: u8, y: u8| {
        r.rooms[room]
            .items
            .iter()
            .find(|i| i.x == x && i.y == y)
            .and_then(|i| i.destination)
    };
    assert_eq!(
        door_to(0, 4, 1),
        Some(Destination::Room {
            region: 1,
            room: 1,
            x: 1,
            y: 1
        })
    );
    assert_eq!(
        door_to(1, 0, 1),
        Some(Destination::Room {
            region: 1,
            room: 0,
            x: 3,
            y: 1
        })
    );
    assert_eq!(door_to(0, 2, 3), Some(Destination::WorldMap { x: 2, y: 1 }));
    assert_eq!(r.portal_table, vec![[0, 0], [0x11, 0x11], [0, 32], [0x10, 0x31]]);
    assert_eq!(
        a.world.exit_points[0],
        Some(acs_core::model::MapPoint { x: 2, y: 1 })
    );
    assert_eq!(a.world.tiles[1], vec![0, 1, 1, 12, 0]);
    assert_eq!(a.world.terrain.len(), 13);

    // Gold, the resident, the store, the room's random creature.
    // A carriable is placed with 1, so a take gives one (inventory 2.7).
    let cutlass = r.rooms[0].items.iter().find(|i| i.thing == 1).unwrap();
    assert_eq!(cutlass.parameter, 1);
    let gold = r.rooms[0].items.iter().find(|i| i.thing == 81).unwrap();
    assert_eq!(gold.gold, Some(50));
    let harrow = &r.residents[0];
    assert!(harrow.in_use);
    assert_eq!((harrow.x, harrow.y, harrow.region, harrow.room), (2, 1, 1, 1));
    assert_eq!(harrow.weapon, 1);
    assert_eq!(r.residents.len(), 16);
    assert!(!r.residents[1].in_use);
    assert_eq!(r.store[0].thing, 2);
    assert_eq!(a.creatures[0].class, Some(0));
    assert_eq!(a.world.random_creatures[0].creature.name, "MEG HARROW");
    assert!(acs_core::limits::check(a).is_empty());
}

#[test]
fn the_bundle_loads_and_is_the_same_every_time() {
    let b = built();
    let bytes = acs_bundle::write(&b).unwrap();
    let back = acs_bundle::read(&bytes).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(back.adventure, b.adventure);
    assert_eq!(acs_bundle::write(&built()).unwrap(), bytes);
}

#[test]
fn the_engine_plays_it() {
    let a = built().adventure;
    let (mut g, _) = Game::new(
        a,
        7,
        Rules::default(),
        PlayerSetup {
            speed: 15,
            ..Default::default()
        },
    );
    // From the beach onto the cove: the map door into the beach room.
    g.place(0, Location::World { x: 2, y: 1 });
    g.apply(Command::Move { dir: Dir::E });
    assert_eq!(
        g.players()[0].at,
        Location::Room {
            region: 1,
            room: 0,
            x: 1,
            y: 1
        }
    );
    // Through the shared wall into the hut.
    g.place(
        0,
        Location::Room {
            region: 1,
            room: 0,
            x: 3,
            y: 1,
        },
    );
    g.apply(Command::Move { dir: Dir::E });
    assert_eq!(
        g.players()[0].at,
        Location::Room {
            region: 1,
            room: 1,
            x: 1,
            y: 1
        }
    );
}

fn problems(f: Folder) -> Vec<String> {
    match build(&f) {
        Ok(_) => panic!("built"),
        Err(p) => p.iter().map(ToString::to_string).collect(),
    }
}

fn has(p: &[String], needle: &str) {
    assert!(p.iter().any(|m| m.contains(needle)), "no {needle:?} in {p:#?}");
}

#[test]
fn problems_name_their_file_and_place() {
    let f = folder().with(
        "things.toml",
        &THINGS
            .replace("picture = \"cutlass\"", "picture = \"cutlas\"")
            .replace("attack = 10", "attack = 10\nwings = true"),
    );
    let p = problems(f);
    has(&p, "things.toml: thing cutlass: picture: no picture \"cutlas\"");
    has(&p, "things.toml: thing cutlass: wings: not a field here");
}

#[test]
fn walls_doors_and_terrain_are_checked() {
    let p = problems(folder().with("regions/01-cove.toml", &COVE.replace("#.c.D", "c...D")));
    has(&p, "cutlass can't stand in a wall");
    let p = problems(folder().with("world.toml", &WORLD.replace("~..P~", "~.PP~")));
    has(&p, "door terrain at [2, 1] has no [[door]]");
    let p = problems(folder().with(
        "regions/01-cove.toml",
        &COVE.replace(
            "\"D\" = { thing = \"door\", to = \"cove.hut\" }",
            "\"D\" = { thing = \"door\", to = \"cove.cellar\" }",
        ),
    ));
    has(&p, "no room \"cove.cellar\"");
}

#[test]
fn creatures_must_carry_what_they_ready() {
    let p = problems(folder().with(
        "creatures.toml",
        &CREATURES.replace("carries = [{ thing = \"cutlass\" }]", ""),
    ));
    has(&p, "weapon: cutlass must also be carried");
}

#[test]
fn missing_files_are_reported() {
    let mut f = folder();
    f.files.remove("world.toml");
    has(&problems(f), "world.toml: missing");
}
