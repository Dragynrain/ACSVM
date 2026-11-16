// SPDX-License-Identifier: GPL-3.0-or-later

//! A synthetic adventure shaped like the capture's scenario disks
//! (docs/capture.md 4, 7): an open field on the world map with a door into
//! region 1, whose ROOM is 15 x 10 (inside 1..13 x 1..8) with the player's
//! entry on (4,4), and a SIDE room behind a portal on ROOM (7,4). Built
//! byte by byte with acs-core's fixture builder, then given the things and
//! creatures each golden test needs as model values. No real adventure data.

use crate::common::{creature_record, Fixture, RegionSpec, RoomSpec};
use acs_core::layout::world_map as wm;
use acs_core::model::{
    Access, Alignment, Carry, Creature, Possession, RoomItem, SpellAction, SpellActionKind, ThingKind,
    WeaponStats,
};
use acs_core::{decode, Adventure, Platform};
use acs_engine::{Command, Event, Game, Location, PlayerSetup, Rules, Who};

/// Room numbers inside region 1.
pub const ROOM: u8 = 0;
pub const SIDE: u8 = 1;

/// Things the base adventure defines; tests add their own with [`thing`].
pub const DOOR: u16 = 82; // portal, anyone
pub const OBSTACLE: u16 = 84;

/// The player, and the creatures in region 1's slots.
pub const ME: Who = Who::Player { player: 0 };
pub fn slot(n: usize) -> Who {
    Who::Creature { region: 1, slot: n }
}

pub fn base(platform: Platform) -> Adventure {
    let mut f = Fixture::new(platform);
    f.header("GOLD", "", "");
    f.thing(DOOR as usize, "DOOR", [0x27, 5, 0, 0]);
    f.thing(OBSTACLE as usize, "PILLAR", [0x0A, 11, 0, 0]);
    f.thing(127, "EMPTY FLOOR", [0x00, 1, 0, 0]);
    f.thing(128, "ROOM BORDER", [0x00, 12, 0, 0]);
    let mut template = creature_record(20, 0x60, 70);
    template[0x10] = 0x10;
    f.region(RegionSpec {
        name: "TEST",
        rooms: vec![
            RoomSpec {
                name: "ROOM",
                w: 15,
                h: 10,
                wall: 12,
                items: vec![(7, 0, (DOOR as u8), 1)],
                ..Default::default()
            },
            RoomSpec {
                name: "SIDE",
                w: 10,
                h: 8,
                x: 15,
                wall: 12,
                items: vec![(0, 4, (DOOR as u8), 2)],
                ..Default::default()
            },
        ],
        // Portal 1: ROOM's door to SIDE (1,4); portal 2: SIDE's back to
        // ROOM (7,1).
        portals: vec![[0, 0], [0x11, 0x14], [0x10, 0x71]],
        residents: vec![(0, "TEMPLATE", template)],
        ..Default::default()
    });
    let w = f.l.world_map;
    f.put(w + wm::TERRAIN, &[1 | 1 << 6, 0]);
    f.put(w + wm::TERRAIN + 12 * 2, &[5 | 1 << 6, 0]); // door terrain, open
    f.put(w + wm::START_X, &[12, 12]);
    // Map door 0 at (12,16) -> ROOM (4,4).
    let cell = w + wm::MAP + 16 * 20 + 6;
    f.data[cell] = (f.data[cell] & 0x0F) | 12 << 4;
    f.put(w + wm::POINTS, &[12, 16]);
    f.put(w + wm::DOOR_DESTINATIONS, &[0x10, 0x44]);
    let mut a = decode(&f.build()).expect("decodes").adventure;
    a.regions[0].residents[0].in_use = false;
    a
}

/// Sets master thing `n` (1-based).
pub fn thing(a: &mut Adventure, n: u16, name: &str, kind: ThingKind) {
    let t = &mut a.things[n as usize - 1];
    t.name = name.into();
    t.kind = kind;
    t.raw = [0; 4];
}

pub fn treasure(weight: i32, value: i32) -> ThingKind {
    ThingKind::Treasure {
        picture: 5,
        disappears_when_dropped: false,
        carry: Carry { weight, value },
    }
}

pub fn spell(cost: u8, action: SpellAction) -> ThingKind {
    ThingKind::MagicSpell {
        disappears_after_use: false,
        power: cost,
        action,
    }
}

pub fn nothing() -> SpellAction {
    SpellAction {
        kind: SpellActionKind::Nothing,
        parameter: 0,
    }
}

pub fn blade(power: u8) -> ThingKind {
    ThingKind::MeleeWeapon {
        picture: 6,
        disappears_after_use: false,
        carry: Carry { weight: 5, value: 10 },
        stats: WeaponStats {
            power,
            range: None,
            break_chance: 0,
            attack_adjustment: 0,
            usable_by_anyone: true,
            magic: false,
        },
    }
}

pub fn bow(power: u8, range: u8) -> ThingKind {
    ThingKind::MissileWeapon {
        picture: 7,
        disappears_after_use: false,
        carry: Carry { weight: 3, value: 10 },
        stats: WeaponStats {
            power,
            range: Some(range),
            break_chance: 0,
            attack_adjustment: 0,
            usable_by_anyone: true,
            magic: false,
        },
    }
}

pub fn portal_kind() -> ThingKind {
    ThingKind::Portal {
        picture: 5,
        two_way: false,
        access: Access::Anyone,
        destroy_needed_thing: false,
    }
}

/// Puts a creature in region 1's `slot` on ROOM (x, y): the template (an
/// aggressive brave enemy, Constitution and Life Force 20) changed by `f`.
pub fn creature(a: &mut Adventure, n: usize, x: u8, y: u8, f: impl FnOnce(&mut Creature)) {
    let mut c = a.regions[0].residents[0].clone();
    c.in_use = true;
    c.name = format!("C{n}");
    c.number = n as u16 + 1;
    c.region = 1;
    c.room = ROOM;
    c.x = x;
    c.y = y;
    c.strategy.alignment = Alignment::Enemy;
    c.strategy.aggressive = true;
    c.strategy.brave = true;
    f(&mut c);
    a.regions[0].residents[n] = c;
}

pub fn owns(c: &mut Creature, thing: u16) {
    c.possessions.push(Possession { thing, count: 1 });
}

/// Puts `thing` on top of ROOM (x, y): a carriable thing with parameter
/// 1, an ordinary take that owns it (inventory 2.7), anything else with 0.
pub fn item(a: &mut Adventure, room: u8, x: u8, y: u8, thing: u16) {
    let carried = a
        .things
        .get(thing as usize - 1)
        .is_some_and(|t| acs_engine::inventory::carriable(&t.kind));
    a.regions[0].rooms[room as usize].items.insert(
        0,
        RoomItem {
            x,
            y,
            thing,
            parameter: carried as u8,
            flag: false,
            destination: None,
            gold: None,
        },
    );
}

pub fn room(x: u8, y: u8) -> Location {
    Location::Room {
        region: 1,
        room: ROOM,
        x,
        y,
    }
}

/// A game of `a` with `player` on `at`, no diagonals (the C64's rule).
pub fn game(a: &Adventure, seed: u64, player: PlayerSetup, at: Location) -> Game {
    let (mut g, _) = Game::new(a.clone(), seed, Rules::default(), player);
    g.place(0, at);
    g
}

/// The player's current Life Force.
pub fn life(g: &Game) -> u8 {
    g.players()[0].life_force
}

pub fn wait(g: &mut Game) -> Vec<Event> {
    g.apply(Command::Wait)
}

/// Runs `f` once per golden seed and counts the seeds it returns true for.
pub fn count(seeds: u64, mut f: impl FnMut(u64) -> bool) -> u64 {
    (0..seeds).filter(|&s| f(s)).count() as u64
}

/// Runs `f` once per golden seed; the bin it returns is counted.
pub fn histogram(seeds: u64, bins: usize, mut f: impl FnMut(u64) -> usize) -> Vec<u64> {
    let mut h = vec![0; bins];
    for s in 0..seeds {
        h[f(s).min(bins - 1)] += 1;
    }
    h
}
