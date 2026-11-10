// SPDX-License-Identifier: GPL-3.0-or-later

//! The canonical, platform-neutral adventure model.
//!
//! Every decoder produces this tree and the JSON export is its serde form.
//! Numbering follows the editor: things, creatures and long messages are
//! numbered from 1 in the original program's lists, rooms and regions as the
//! editor shows them. Each struct says which convention it uses.
//!
//! Fields whose meaning is not yet known are kept raw (`unknown_*`) so that a
//! future editor can write the file back without losing anything.

use crate::disk::Platform;
use crate::graphics::{Palette, Picture};
use serde::{Deserialize, Serialize};

/// Version of the JSON model. Bump when a field changes meaning.
pub const MODEL_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Adventure {
    pub model_version: u32,
    pub platform: Platform,
    pub name: String,
    pub byline: String,
    pub intro: String,
    /// The adventure's genre theme; it picks the "endless" background music.
    pub theme: Theme,
    pub palette: Palette,
    /// Picture 0..N. PC has 96, C64 110 (the last 46 for creatures only).
    pub pictures: Vec<Picture>,
    /// The master thing list: 128 entries, `number` 1..=128.
    pub things: Vec<Thing>,
    pub creature_classes: Vec<String>,
    /// The master creature list: 128 entries, `number` 1..=128.
    pub creatures: Vec<Creature>,
    /// Up to 4 characters saved with the adventure (in-use slots only).
    pub players: Vec<Creature>,
    pub retired_players: Vec<Creature>,
    pub world: WorldMap,
    pub regions: Vec<Region>,
    /// Long messages 1..=255 (index 0 here is message 1). Message 255 is the intro text.
    pub long_messages: Vec<String>,
    /// Short message slots 1..=80 (index 0 is slot 1), 3 lines each.
    pub short_messages: Vec<String>,
}

/// Genre theme stored with the adventure (one of the three builder sets).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    Fantasy,
    SpyMystery,
    SciFi,
    Other(u8),
}

impl Theme {
    pub fn from_code(code: u8) -> Self {
        match code {
            0 => Theme::Fantasy,
            1 => Theme::SpyMystery,
            2 => Theme::SciFi,
            c => Theme::Other(c),
        }
    }

    /// The looping theme it plays, as a 1-based Play Music number
    /// (14 Endless Fantasy, 15 Endless Spy/Mystery, 16 Endless Sci-Fi).
    pub fn music_number(self) -> u8 {
        match self {
            Theme::Fantasy => 14,
            Theme::SpyMystery => 15,
            Theme::SciFi => 16,
            Theme::Other(c) => c.saturating_add(14),
        }
    }
}

// ---------------------------------------------------------------------------
// Things

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Thing {
    /// 1..=128.
    pub number: u8,
    pub name: String,
    /// How many of this thing ACS may add to the adventure; 15 means "many".
    pub acs_may_add: u8,
    #[serde(flatten)]
    pub kind: ThingKind,
    /// Message slot 0: shown when passage is blocked (portals, spaces, obstacles).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub blocked_message: Option<MessageRef>,
    /// Message slot 1: shown when the thing's spell is cast.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub spell_message: Option<MessageRef>,
    /// The four property bytes as stored, for round-tripping.
    pub raw: [u8; 4],
}

/// A thing's reference to a short message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageRef {
    /// Short message slot, 1..=80.
    pub slot: u8,
    /// False when the designer chose not to show it (bit 7 of the reference).
    pub shown: bool,
}

/// Weight and value, already multiplied out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Carry {
    pub weight: i32,
    pub value: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeaponStats {
    pub power: u8,
    /// Missile weapons only, in squares.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub range: Option<u8>,
    /// Percent per use, 0..=15.
    pub break_chance: u8,
    /// Percent added to the attacker's hit chance (armor: -65..=10, weapons: -35..=40).
    pub attack_adjustment: i8,
    pub usable_by_anyone: bool,
    pub magic: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ThingKind {
    RoomFloor {
        picture: u8,
    },
    Treasure {
        picture: u8,
        disappears_when_dropped: bool,
        #[serde(flatten)]
        carry: Carry,
    },
    MagicItem {
        picture: u8,
        disappears_after_use: bool,
        #[serde(flatten)]
        carry: Carry,
        action: SpellAction,
        on_use: bool,
        on_drop: bool,
        on_pickup: bool,
    },
    MissileWeapon {
        picture: u8,
        disappears_after_use: bool,
        #[serde(flatten)]
        carry: Carry,
        #[serde(flatten)]
        stats: WeaponStats,
    },
    MeleeWeapon {
        picture: u8,
        disappears_after_use: bool,
        #[serde(flatten)]
        carry: Carry,
        #[serde(flatten)]
        stats: WeaponStats,
    },
    Armor {
        picture: u8,
        disappears_after_use: bool,
        #[serde(flatten)]
        carry: Carry,
        #[serde(flatten)]
        stats: WeaponStats,
    },
    MagicSpell {
        disappears_after_use: bool,
        /// Power needed to cast, 0..=127.
        power: u8,
        action: SpellAction,
    },
    Portal {
        picture: u8,
        two_way: bool,
        access: Access,
        destroy_needed_thing: bool,
    },
    Space {
        picture: u8,
        access: Access,
    },
    CustomSpace {
        picture: u8,
        /// What the designer picks when placing it: a thing or a spell parameter.
        placement_choice: PlacementChoice,
        trigger: CustomTrigger,
        action: SpellAction,
        destroy_needed_thing: bool,
    },
    Obstacle {
        picture: u8,
        bump_invokes_spell: bool,
        action: SpellAction,
    },
    CustomObstacle {
        picture: u8,
        /// The action; its parameter is chosen per placement (the room item parameter).
        action: SpellAction,
    },
    Store {
        picture: u8,
    },
    /// An empty slot in the master list (type 15).
    Undefined,
    /// A type code the decoder does not know (13 or 14).
    Unknown {
        code: u8,
    },
}

impl ThingKind {
    pub fn type_name(&self) -> &'static str {
        match self {
            ThingKind::RoomFloor { .. } => "room floor",
            ThingKind::Treasure { .. } => "treasure",
            ThingKind::MagicItem { .. } => "magic item",
            ThingKind::MissileWeapon { .. } => "missile weapon",
            ThingKind::MeleeWeapon { .. } => "melee weapon",
            ThingKind::Armor { .. } => "armor",
            ThingKind::MagicSpell { .. } => "magic spell",
            ThingKind::Portal { .. } => "portal",
            ThingKind::Space { .. } => "space",
            ThingKind::CustomSpace { .. } => "custom space",
            ThingKind::Obstacle { .. } => "obstacle",
            ThingKind::CustomObstacle { .. } => "custom obstacle",
            ThingKind::Store { .. } => "store",
            ThingKind::Undefined => "undefined",
            ThingKind::Unknown { .. } => "unknown",
        }
    }

    pub fn picture(&self) -> Option<u8> {
        match *self {
            ThingKind::RoomFloor { picture }
            | ThingKind::Treasure { picture, .. }
            | ThingKind::MagicItem { picture, .. }
            | ThingKind::MissileWeapon { picture, .. }
            | ThingKind::MeleeWeapon { picture, .. }
            | ThingKind::Armor { picture, .. }
            | ThingKind::Portal { picture, .. }
            | ThingKind::Space { picture, .. }
            | ThingKind::CustomSpace { picture, .. }
            | ThingKind::Obstacle { picture, .. }
            | ThingKind::CustomObstacle { picture, .. }
            | ThingKind::Store { picture } => Some(picture),
            _ => None,
        }
    }
}

/// Who may pass a portal or space.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "rule", rename_all = "snake_case")]
pub enum Access {
    Anyone,
    InvokeSpell { action: SpellAction },
    OwnersOf { thing: u8 },
    NonOwnersOf { thing: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlacementChoice {
    Thing,
    SpellParameter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CustomTrigger {
    WhenDroppedHere,
    WhenMovedOnto,
    OpenToOwnersOf,
    OpenToNonOwnersOf,
}

/// One of the 15 spell actions with its raw parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpellAction {
    pub kind: SpellActionKind,
    /// Meaning depends on `kind` (thing number, creature number, stat, amount, ...).
    pub parameter: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpellActionKind {
    Nothing,
    KillAllButOwnerOf,
    SummonOrBanish,
    IncreaseMagicDefense,
    DecreaseMagicDefense,
    IncreaseStat,
    DecreaseStat,
    ChangePower,
    ChangeLifeForce,
    Give,
    LongMessage,
    PlayMusic,
    RidRoomOf,
    AddToRoom,
    ActivateAll,
    /// A code outside 0..=14.
    Invalid(u8),
}

impl SpellActionKind {
    pub fn from_code(code: u8) -> Self {
        use SpellActionKind::*;
        match code {
            0 => Nothing,
            1 => KillAllButOwnerOf,
            2 => SummonOrBanish,
            3 => IncreaseMagicDefense,
            4 => DecreaseMagicDefense,
            5 => IncreaseStat,
            6 => DecreaseStat,
            7 => ChangePower,
            8 => ChangeLifeForce,
            9 => Give,
            10 => LongMessage,
            11 => PlayMusic,
            12 => RidRoomOf,
            13 => AddToRoom,
            14 => ActivateAll,
            c => Invalid(c),
        }
    }
}

// ---------------------------------------------------------------------------
// Creatures

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Alignment {
    Thief,
    Neutral,
    Enemy,
    Friend,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Strategy {
    pub alignment: Alignment,
    pub aggressive: bool,
    pub brave: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MagicDefense {
    None,
    NonMagicHalfDamage,
    OnlyMagicDamages,
    MagicHalfDamage,
}

impl MagicDefense {
    pub fn from_code(code: u8) -> Self {
        match code & 3 {
            0 => MagicDefense::None,
            1 => MagicDefense::NonMagicHalfDamage,
            2 => MagicDefense::OnlyMagicDamages,
            _ => MagicDefense::MagicHalfDamage,
        }
    }
}

/// A count of one thing carried by a creature or stocked by a store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Possession {
    /// Thing number, 1..=80.
    pub thing: u8,
    /// 1..=3 for things 1..=32, 1 for things 33..=80.
    pub count: u8,
}

/// A 37-byte creature record (master list, region, world map or player).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Creature {
    /// Position in its list: 1..=128 in the master list, 1-based slot elsewhere.
    pub number: u8,
    pub in_use: bool,
    pub name: String,
    /// Class index 0..=7 (master list only).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub class: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub picture: Option<u8>,
    pub constitution: u8,
    pub life_force: u8,
    pub strength: u8,
    pub dexterity: u8,
    pub speed: u8,
    pub wisdom: u8,
    pub power: u8,
    pub size: u8,
    pub dodge: u8,
    pub parry: u8,
    pub armor_skill: u8,
    pub melee: u8,
    pub missile: u8,
    pub strategy: Strategy,
    pub magic_defense: MagicDefense,
    pub mimic: bool,
    pub wealth: u16,
    /// Readied weapon / armor (thing numbers, 0 = none).
    pub weapon: u8,
    pub armor: u8,
    /// Square in its room or world map cell.
    pub x: u8,
    pub y: u8,
    /// Region (1-based, 0 = world map / none) and room (0-based) it stands in.
    pub region: u8,
    pub room: u8,
    pub possessions: Vec<Possession>,
    /// Bytes 0x0B and 0x0C: zero in every corpus record; kept raw.
    pub unknown_0b: u8,
    pub unknown_0c: u8,
}

// ---------------------------------------------------------------------------
// World map

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Passability {
    Impassable,
    Open,
    OwnersOf,
    TriggersSpell,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Terrain {
    /// 0..=15, the value used in [`WorldMap::tiles`].
    pub number: u8,
    pub name: String,
    pub picture: u8,
    pub passability: Passability,
    /// Thing number for `owners_of` / `triggers_spell`.
    pub parameter: u8,
    /// Byte 0 bit 5 as stored; meaning unknown.
    pub unknown_bit5: bool,
}

/// A world map location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapPoint {
    pub x: u8,
    pub y: u8,
}

/// Where a door leads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "to", rename_all = "snake_case")]
pub enum Destination {
    WorldMap {
        x: u8,
        y: u8,
    },
    /// `region` is 1-based, `room` 0-based, `x`/`y` a square in the room.
    Room {
        region: u8,
        room: u8,
        x: u8,
        y: u8,
    },
}

/// One of the 32 world map doors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapDoor {
    /// 0..=31.
    pub index: u8,
    pub at: MapPoint,
    pub destination: Destination,
    /// Bit 7 of the stored y coordinate; meaning unknown.
    pub unknown_y_bit7: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldCreature {
    pub creature: Creature,
    /// Terrain number (0..=15) it appears in.
    pub terrain: u8,
    /// Chance per move, percent (0..=60).
    pub chance: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldMap {
    pub name: String,
    pub wraps: bool,
    pub start: MapPoint,
    pub terrain: Vec<Terrain>,
    /// 40 rows of 40 terrain numbers.
    pub tiles: Vec<Vec<u8>>,
    pub doors: Vec<MapDoor>,
    /// The 32 extra exit points (world map points 32..64) one-way room doors lead to;
    /// `None` for unused points (stored as 0xFF).
    pub exit_points: Vec<Option<MapPoint>>,
    pub random_creatures: Vec<WorldCreature>,
}

// ---------------------------------------------------------------------------
// Regions

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Region {
    /// 1..=15.
    pub number: u8,
    pub name: String,
    /// All 16 room slots; deleted ones have `deleted: true`.
    pub rooms: Vec<Room>,
    /// 8 random creature slots.
    pub random_creatures: Vec<Creature>,
    /// 16 resident creature slots; each in-use one names its room.
    pub residents: Vec<Creature>,
    /// What the region's store sells.
    pub store: Vec<Possession>,
    /// Door table: entry n is what a portal with parameter n leads to (entry 0 unused).
    pub portal_table: Vec<[u8; 2]>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Room {
    /// 0..=15.
    pub number: u8,
    pub name: String,
    pub deleted: bool,
    pub wall_picture: u8,
    /// Size in squares including the walls (1..=16 as stored + 1).
    pub width: u8,
    pub height: u8,
    /// Position on the region layout grid.
    pub x: u8,
    pub y: u8,
    /// Percent chance of a random creature on entry (0..=75).
    pub random_chance: u8,
    /// Which of the region's 8 random creatures (0-based).
    pub random_creature: u8,
    pub items: Vec<RoomItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoomItem {
    pub x: u8,
    pub y: u8,
    /// Thing number 1..=127.
    pub thing: u8,
    /// Per-placement parameter: portal table index, custom space choice, ...
    pub parameter: u8,
    /// Bit 7 of the thing byte, as stored.
    pub flag: bool,
    /// Resolved destination for portals.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub destination: Option<Destination>,
}
