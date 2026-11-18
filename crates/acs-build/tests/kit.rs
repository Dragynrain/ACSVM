// SPDX-License-Identifier: GPL-3.0-or-later

//! Starter kits (docs/content.md 4): an adventure takes only the kit
//! entries it names, can replace one, and the pirate kit builds whole.

use acs_build::{build, Folder};
use acs_core::model::{Access, ThingKind};
use acs_engine::{Command, Dir, Game, Location, PlayerSetup, Rules};
use std::path::Path;

fn kit() -> Folder {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../kits/pirates");
    Folder::read(&dir).expect("kits/pirates")
}

/// Every key in a kit file's `[[thing]]` or `[[creature]]` tables, and
/// every picture in kit.toml.
fn kit_keys(k: &Folder) -> Vec<String> {
    let mut out = Vec::new();
    for (file, list) in [("things.toml", "thing"), ("creatures.toml", "creature")] {
        let t: toml::Table = std::str::from_utf8(&k.files[file]).unwrap().parse().unwrap();
        for e in t[list].as_array().unwrap() {
            out.push(e["key"].as_str().unwrap().to_string());
        }
    }
    let t: toml::Table = std::str::from_utf8(&k.files["kit.toml"])
        .unwrap()
        .parse()
        .unwrap();
    out.extend(t["pictures"].as_table().unwrap().keys().cloned());
    out
}

const WORLD: &str = r##"
name = "THE KEYS"
start = [1, 1]
map = """
~~~~~
~..P~
~~~~~
"""

[legend]
"~" = "sea"
"." = "beach"
"P" = "port"

[[terrain]]
key = "sea"
name = "DEEP SEA"
picture = "terrain_deepsea"
pass = "impassable"

[[terrain]]
key = "beach"
name = "BEACH"
picture = "terrain_beach"
pass = "open"

[[terrain]]
key = "port"
name = "PORT"
picture = "terrain_porttown"
pass = "open"
door = true

[[door]]
at = [3, 1]
to = "port.quay"
square = [1, 1]
"##;

const PORT: &str = r##"
key = "port"
name = "PORT"
store = [{ thing = "cutlass", count = 2 }, { thing = "rum_flask", count = 3 }]

[[room]]
key = "quay"
name = "QUAY"
at = [0, 0]
wall = "room_palisade"
map = """
######
#.b.kD
#.s..#
######
"""

[room.legend]
"#" = "wall"
"." = "floor"
"b" = "barrels"
"k" = { creature = "deckhand" }
"s" = "store"
"D" = { thing = "wooden_door_palisade", to = "port.tavern" }

[[room]]
key = "tavern"
name = "TAVERN"
at = [5, 0]
wall = "room_panelling"
map = """
#####
D.t.#
#.K.#
#####
"""

[room.legend]
"#" = "wall"
"." = "floor"
"t" = "tavern_table"
"K" = { creature = "tavern_keeper" }
"D" = { thing = "wooden_door_panelling", to = "port.quay" }
"##;

fn adventure(credits: &str) -> String {
    format!(
        r#"
name = "THE KEYS"
kit = "../../kits/pirates"
credits = [{credits}]

[gold]
picture = "goldpile"

[area_door]
picture = "door_cavepass"

[floor]
picture = "room_roomfloor"
wall = "room_hull"
"#
    )
}

fn folder(credits: &str) -> Folder {
    Folder::default()
        .with("adventure.toml", &adventure(credits))
        .with("things.toml", "")
        .with("creatures.toml", "")
        .with("world.toml", WORLD)
        .with("regions/01-port.toml", PORT)
        .with_kit(kit())
}

fn built(f: &Folder) -> acs_bundle::Bundle {
    build(f).unwrap_or_else(|p| {
        panic!(
            "{}",
            p.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n")
        )
    })
}

#[test]
fn an_adventure_takes_only_what_it_uses() {
    let b = built(&folder(""));
    let a = &b.adventure;
    let names: Vec<&str> = a
        .things
        .iter()
        .filter(|t| !matches!(t.kind, ThingKind::Undefined))
        .map(|t| t.name.as_str())
        .collect();
    // The deckhand's and keeper's gear, the stock, the dressing, two doors,
    // the store and the built-ins; nothing else of the kit's 123.
    for n in [
        "CUTLASS",
        "RUM FLASK",
        "GUTTING KNIFE",
        "BARRELS",
        "TAVERN TABLE",
        "TRADER'S COUNTER",
        "WOODEN DOOR",
    ] {
        assert!(names.contains(&n), "{n} missing from {names:?}");
    }
    assert!(!names.contains(&"HALBERD"));
    assert_eq!(names.len(), 12, "{names:?}");
    // Kit order holds: the rum flask (a consumable) comes before the knife.
    assert_eq!(a.things[0].name, "RUM FLASK");
    assert_eq!(a.creatures.len(), 2);
    // Pictures: only those used, and the bundle's atlas holds just them.
    assert!(a.pictures.len() < 30, "{} pictures", a.pictures.len());
    let art = b.art.as_ref().unwrap();
    assert_eq!(art.places.len(), a.pictures.len() - 1);
    assert_eq!(art.tile, 64);
    assert!(acs_bundle::read(&acs_bundle::write(&b).unwrap()).is_ok());
}

#[test]
fn an_adventure_entry_replaces_the_kits() {
    let f = folder("").with(
        "things.toml",
        r#"
[[thing]]
key = "wooden_door_palisade"
name = "LOCKED DOOR"
type = "portal"
picture = "door_woodendoor_palisade"
two_way = true
access = { owners_of = "cutlass" }
"#,
    );
    let a = built(&f).adventure;
    let door = a
        .things
        .iter()
        .find(|t| t.name == "LOCKED DOOR")
        .expect("replaced");
    assert!(matches!(
        door.kind,
        ThingKind::Portal {
            access: Access::OwnersOf { .. },
            ..
        }
    ));
    // Only the tavern's wooden door is left of the kit's two.
    assert_eq!(a.things.iter().filter(|t| t.name == "WOODEN DOOR").count(), 1);
}

#[test]
fn the_kit_adventure_plays() {
    let a = built(&folder("")).adventure;
    let (mut g, _) = Game::new(
        a,
        3,
        Rules::default(),
        PlayerSetup {
            speed: 15,
            ..Default::default()
        },
    );
    g.place(0, Location::World { x: 2, y: 1 });
    g.apply(Command::Move { dir: Dir::E });
    assert!(matches!(
        g.players()[0].at,
        Location::Room {
            region: 1,
            room: 0,
            ..
        }
    ));
}

/// Every kit entry and picture taken at once: the whole kit builds, so
/// no entry names a picture or thing that isn't there.
#[test]
fn the_whole_pirate_kit_builds() {
    let k = kit();
    let keys = kit_keys(&k);
    let credits = keys
        .iter()
        .map(|k| format!("{k:?}"))
        .collect::<Vec<_>>()
        .join(", ");
    let f = folder(&credits).with(
        "adventure.toml",
        &adventure(&credits).replace("[gold]", "[limits]\nthings = 200\npictures = 256\n\n[gold]"),
    );
    let b = built(&f);
    let a = &b.adventure;
    let used = a
        .things
        .iter()
        .filter(|t| !matches!(t.kind, ThingKind::Undefined))
        .count();
    assert_eq!(used, 123 + 4, "the kit's things and the four built-ins");
    assert_eq!(a.creatures.len(), 53);
    assert_eq!(a.pictures.len(), 202 + 1);
}

/// scene::layers (client.md 5): a thing is drawn over the space it
/// stands on, but what lies hidden between them stays hidden.
#[test]
fn a_thing_stands_on_its_ground_and_hides_what_is_under_it() {
    use acs_core::scene::{self, RoomScene};
    let f = folder("").with(
        "regions/01-port.toml",
        &PORT.replace(
            "\"b\" = \"barrels\"",
            "\"b\" = [\"barrels\", \"cutlass\", \"sand\"]",
        ),
    );
    let a = built(&f).adventure;
    let region = &a.regions[0];
    let quay = &region.rooms[0];
    let s = RoomScene::new(region, quay);
    let pic = |thing: &str| {
        let t = a.things.iter().find(|t| t.name == thing).unwrap();
        t.kind.picture().unwrap()
    };
    let layers = scene::layers(&a, region, quay, s.square(2, 1).unwrap());
    let floor = scene::floor_picture(&a).unwrap();
    assert_eq!(layers, vec![floor, pic("SAND"), pic("BARRELS")]);
    assert!(!layers.contains(&pic("CUTLASS")));
}
