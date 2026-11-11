// SPDX-License-Identifier: GPL-3.0-or-later

//! Adventure profiles and the size limits each one allows.
//!
//! A **Classic** adventure is one decoded from an original disk or file and
//! keeps every original limit. An **Extended** adventure is our own format:
//! the same rules, larger counts and sizes. The engine reads sizes from the
//! adventure (its world map, its region slots and [`Limits`]) rather than
//! from constants, so both profiles run on the same code. See "Extended
//! adventures" in `docs/design.md`.

use crate::model::Adventure;
use serde::{Deserialize, Serialize};

/// Which rule set an adventure's sizes are checked against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Profile {
    /// Decoded from an original disk or file; original limits.
    Classic,
    /// Our own format with raised limits.
    Extended,
}

/// Size limits an adventure must stay within.
///
/// These are caps, not sizes: an adventure may use less. Ids that can pass
/// 255 under [`Limits::EXTENDED`] are `u16` in the model, so raising a cap
/// later is a validator change, not a format change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Limits {
    /// World map width and height, in squares.
    pub world_size: u16,
    pub terrain_types: u16,
    pub map_doors: u16,
    pub regions: u16,
    pub rooms_per_region: u16,
    /// Room width and height in squares, walls included.
    pub room_size: u16,
    /// Creature slots per region (residents plus random arrivals).
    pub residents_per_region: u16,
    pub random_creatures_per_region: u16,
    /// Most creatures in one room at once. Turns are taken per room, so this
    /// is what bounds turn order and AI cost.
    pub creatures_per_room: u16,
    pub things: u16,
    pub creatures: u16,
    pub creature_classes: u16,
    pub pictures: u16,
    pub long_messages: u16,
    pub short_messages: u16,
    /// Longest region or world name, in characters.
    pub name_length: u16,
}

impl Limits {
    /// The original program's limits, the same on every platform.
    pub const CLASSIC: Limits = Limits {
        world_size: 40,
        terrain_types: 16,
        map_doors: 32,
        regions: 15,
        rooms_per_region: 16,
        room_size: 16,
        residents_per_region: 16,
        random_creatures_per_region: 8,
        // Every creature in a region shares its 16 slots.
        creatures_per_room: 16,
        things: 128,
        creatures: 128,
        creature_classes: 8,
        pictures: 110,
        long_messages: 255,
        short_messages: 80,
        name_length: 20,
    };

    /// Extended v1. Keeps Classic's proportions: about 4 residents per room
    /// against 1 in Classic, and a per-room cap for turn-order cost.
    pub const EXTENDED: Limits = Limits {
        world_size: 128,
        terrain_types: 64,
        map_doors: 512,
        regions: 64,
        rooms_per_region: 64,
        room_size: 32,
        residents_per_region: 256,
        random_creatures_per_region: 32,
        creatures_per_room: 32,
        things: 1024,
        creatures: 1024,
        creature_classes: 32,
        pictures: 1024,
        long_messages: 4096,
        short_messages: 4096,
        name_length: 64,
    };

    pub fn for_profile(profile: Profile) -> Limits {
        match profile {
            Profile::Classic => Limits::CLASSIC,
            Profile::Extended => Limits::EXTENDED,
        }
    }
}

/// One place where an adventure exceeds its limits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    /// What is too big, e.g. `"regions"` or `"region 3 room 2 width"`.
    pub what: String,
    pub found: usize,
    pub limit: usize,
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {} (limit {})", self.what, self.found, self.limit)
    }
}

/// Checks an adventure against its own `limits`. Shared by the importer and
/// the future editor; an empty result means the adventure fits.
pub fn check(adv: &Adventure) -> Vec<Violation> {
    let l = &adv.limits;
    let mut out = Vec::new();
    let mut cap = |what: String, found: usize, limit: u16| {
        if found > limit as usize {
            out.push(Violation {
                what,
                found,
                limit: limit as usize,
            });
        }
    };
    let w = &adv.world;
    cap("world height".into(), w.tiles.len(), l.world_size);
    let width = w.tiles.iter().map(Vec::len).max().unwrap_or(0);
    cap("world width".into(), width, l.world_size);
    cap("terrain types".into(), w.terrain.len(), l.terrain_types);
    cap("map doors".into(), w.doors.len(), l.map_doors);
    cap("regions".into(), adv.regions.len(), l.regions);
    cap("things".into(), adv.things.len(), l.things);
    cap("creatures".into(), adv.creatures.len(), l.creatures);
    cap(
        "creature classes".into(),
        adv.creature_classes.len(),
        l.creature_classes,
    );
    cap("pictures".into(), adv.pictures.len(), l.pictures);
    cap("long messages".into(), adv.long_messages.len(), l.long_messages);
    cap(
        "short messages".into(),
        adv.short_messages.len(),
        l.short_messages,
    );
    cap("world name".into(), w.name.chars().count(), l.name_length);
    for r in &adv.regions {
        let n = r.number;
        cap(format!("region {n} name"), r.name.chars().count(), l.name_length);
        cap(format!("region {n} rooms"), r.rooms.len(), l.rooms_per_region);
        cap(
            format!("region {n} residents"),
            r.residents.len(),
            l.residents_per_region,
        );
        cap(
            format!("region {n} random creatures"),
            r.random_creatures.len(),
            l.random_creatures_per_region,
        );
        for room in r.rooms.iter().filter(|room| !room.deleted) {
            let k = room.number;
            cap(
                format!("region {n} room {k} width"),
                room.width as usize,
                l.room_size,
            );
            cap(
                format!("region {n} room {k} height"),
                room.height as usize,
                l.room_size,
            );
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extended_is_never_smaller_than_classic() {
        let (c, e) = (Limits::CLASSIC, Limits::EXTENDED);
        let pairs = [
            (c.world_size, e.world_size),
            (c.terrain_types, e.terrain_types),
            (c.map_doors, e.map_doors),
            (c.regions, e.regions),
            (c.rooms_per_region, e.rooms_per_region),
            (c.room_size, e.room_size),
            (c.residents_per_region, e.residents_per_region),
            (c.random_creatures_per_region, e.random_creatures_per_region),
            (c.creatures_per_room, e.creatures_per_room),
            (c.things, e.things),
            (c.creatures, e.creatures),
            (c.creature_classes, e.creature_classes),
            (c.pictures, e.pictures),
            (c.long_messages, e.long_messages),
            (c.short_messages, e.short_messages),
            (c.name_length, e.name_length),
        ];
        // Every Classic adventure must be a valid Extended one.
        assert!(pairs.iter().all(|(c, e)| c <= e));
    }
}
