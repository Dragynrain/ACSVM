// SPDX-License-Identifier: GPL-3.0-or-later
//
// Offsets come from ACS Viewer's AcsLib GameLoader (Copyright (C) 2016 Dan
// Boris, GPL-3.0-or-later) and Boris's format draft v0.5, checked and extended
// against the C64 test corpus. See docs/provenance.md and docs/format-notes.md.

//! Platform layouts: where each structure sits in an adventure image.
//!
//! Block bases differ per platform ([`Layout`]); offsets *inside* a block are
//! the same everywhere and live in the submodules ([`adventure`], [`things`], ...).

use crate::disk::Platform;

/// How pictures are stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PictureFormat {
    /// 16x16, 2 bits per pixel, CGA-style 4-pixels-per-byte in four banks.
    Pc,
    /// 8x16 multicolor character pairs, 2 bits per pixel, pixels double wide.
    C64Multicolor,
}

/// Base addresses of every block for one platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    /// Long messages 0..249, 256 bytes each.
    pub long_messages: usize,
    /// Long messages 249..255, 256 bytes each. The last one is the intro text.
    pub long_messages_tail: usize,
    pub intro: usize,
    pub short_messages: usize,
    pub creatures: usize,
    pub things: usize,
    pub adventure: usize,
    pub world_map: usize,
    pub regions: usize,
    pub pictures: usize,
    pub palette: usize,
    pub picture_format: PictureFormat,
    pub picture_count: usize,
}

/// Number of long messages (the last is the intro text).
pub const LONG_MESSAGE_COUNT: usize = 255;
/// Long messages stored at [`Layout::long_messages`]; the rest are at the tail.
pub const LONG_MESSAGES_HEAD: usize = 249;
pub const LONG_MESSAGE_LEN: usize = 256;
pub const INTRO_LEN: usize = 256;

/// The 0x200-byte shift between PC and C64 is the C64's track 18 sectors 0
/// and 1 (BAM and first directory sector), which the adventure data skips.
/// It applies to everything stored after track 18 begins (0x16500).
pub const C64_TRACK18_GAP: usize = 0x200;

pub const PC: Layout = Layout {
    long_messages: 0x06C00,
    long_messages_tail: 0x16500,
    intro: 0x16A00,
    short_messages: 0x16B00,
    creatures: 0x19300,
    things: 0x1B800,
    adventure: 0x1C200,
    world_map: 0x1C700,
    regions: 0x1D300,
    pictures: 0x28720,
    palette: 0x2A802,
    picture_format: PictureFormat::Pc,
    picture_count: 96,
};

const G: usize = C64_TRACK18_GAP;

pub const C64: Layout = Layout {
    long_messages: 0x06C00, // before track 18: no shift
    long_messages_tail: 0x16500 + G,
    intro: 0x16A00 + G,
    short_messages: 0x16B00 + G,
    creatures: 0x19300 + G,
    things: 0x1B800 + G,
    adventure: 0x1C200 + G,
    world_map: 0x1C700 + G,
    regions: 0x1D300 + G,
    pictures: 0x28720 + G,
    palette: 0x29880, // not PC + 0x200: the C64 colour block is laid out differently
    picture_format: PictureFormat::C64Multicolor,
    picture_count: 110,
};

impl Layout {
    pub const fn for_platform(p: Platform) -> &'static Layout {
        match p {
            Platform::Pc => &PC,
            Platform::C64 => &C64,
        }
    }

    /// Address of long message `i` (0-based, `i < LONG_MESSAGE_COUNT`).
    pub const fn long_message(&self, i: usize) -> usize {
        if i < LONG_MESSAGES_HEAD {
            self.long_messages + i * LONG_MESSAGE_LEN
        } else {
            self.long_messages_tail + (i - LONG_MESSAGES_HEAD) * LONG_MESSAGE_LEN
        }
    }

    /// Address of short message slot `slot` (0-based; things store `slot + 1`).
    /// Two 120-byte messages share each 256-byte page.
    pub const fn short_message(&self, slot: usize) -> usize {
        self.short_messages + (slot >> 1) * 0x100 + (slot & 1) * SHORT_MESSAGE_LEN
    }
}

/// Short messages: 3 lines of 40 characters.
pub const SHORT_MESSAGE_LEN: usize = 120;
pub const SHORT_MESSAGE_LINE: usize = 40;
/// Slots that fit between the short-message base and the creature block.
pub const SHORT_MESSAGE_SLOTS: usize = 80;

/// Offsets inside the adventure block ([`Layout::adventure`]).
pub mod adventure {
    pub const NAME: usize = 0x00;
    pub const NAME_LEN: usize = 0x13;
    pub const BYLINE: usize = 0x14;
    pub const BYLINE_LEN: usize = 0x13;
    pub const REGION_COUNT: usize = 0x29;
    /// Four active players, 37-byte creature records.
    pub const PLAYERS: usize = 0x2B;
    /// Their names, 10 packed bytes each.
    pub const PLAYER_NAMES: usize = 0x2B + 0x94;
    pub const MAP_WRAP: usize = 0x22C;
    pub const MAP_NAME: usize = 0x22D;
    /// 20 bytes: region names follow at 0x241. (ACS Viewer reads 22.)
    pub const MAP_NAME_LEN: usize = 20;
    /// 15 region names, 20 bytes each.
    pub const REGION_NAMES: usize = 0x241;
    pub const REGION_NAME_LEN: usize = 20;
    /// Theme: 0 fantasy, 1 spy/mystery, 2 sci-fi.
    pub const THEME: usize = 0x36E;
}

/// Offsets inside the thing block ([`Layout::things`]).
pub mod things {
    pub const COUNT: usize = 128;
    /// Names: 10 packed bytes each.
    pub const NAMES: usize = 0x000;
    /// Four property bytes per thing.
    pub const PROPS: usize = 0x500;
    /// Message references: one byte per thing per message slot (0 and 1).
    pub const MESSAGES: usize = 0x700;
    pub const MESSAGE_SLOT_STRIDE: usize = 0x80;
    /// "ACS may add" counts (high nibble).
    pub const ACS_ADDS: usize = 0x800;
    /// Byte offset from [`EXTENDED`] to the magic-item part of the table.
    pub const MAGIC_ITEM_OFFSET: usize = 0x882;
    /// Extended 3-byte records for weapons, armor and magic items.
    pub const EXTENDED: usize = 0x884;
}

/// Offsets inside the creature block ([`Layout::creatures`]).
pub mod creatures {
    pub const COUNT: usize = 128;
    pub const RECORD_LEN: usize = 37;
    /// 8 class names, 15 bytes each.
    pub const CLASS_NAMES: usize = 0x000;
    pub const CLASS_NAME_LEN: usize = 15;
    pub const CLASS_COUNT: usize = 8;
    /// Master list records: six per 256-byte page from here.
    pub const RECORDS: usize = 0x100;
    /// Retired players: names (10 bytes, 25 per page) and records (6 per page).
    pub const RETIRED_NAMES: usize = 0x1700;
    pub const RETIRED_RECORDS: usize = 0x1900;
    pub const RETIRED_MAX: usize = 34;
    /// One class byte per master creature; 8 = unused slot.
    pub const CLASSES: usize = 0x1F00;
    pub const NAMES: usize = 0x1F80;

    /// Offset of master record `i`.
    pub const fn record(i: usize) -> usize {
        RECORDS + (i / 6) * 0x100 + (i % 6) * RECORD_LEN
    }
}

/// Offsets inside the world map block ([`Layout::world_map`]).
pub mod world_map {
    /// 16 terrain types, 2 bytes each.
    pub const TERRAIN: usize = 0x000;
    /// 64 world map points (x, y). 0..32 are door locations; 32..64 are the
    /// extra exit points that one-way room doors lead to.
    pub const POINTS: usize = 0x020;
    pub const POINT_COUNT: usize = 64;
    pub const DOOR_COUNT: usize = 32;
    pub const START_X: usize = 0x0A0;
    pub const START_Y: usize = 0x0A1;
    /// Door destinations: (room | region << 4, y | x << 4), one pair per door.
    pub const DOOR_DESTINATIONS: usize = 0x0AA;
    /// 40x40 terrain nibbles, high nibble first.
    pub const MAP: usize = 0x0EA;
    pub const SIZE: usize = 40;
    /// 8 random world-map creatures, 37-byte records.
    pub const CREATURES: usize = 0x40A;
    pub const CREATURE_COUNT: usize = 8;
    /// Per creature: chance code (low nibble) and terrain (high nibble).
    pub const CREATURE_CHANCES: usize = 0x65A;
    /// 16 terrain names, 15 bytes each.
    pub const TERRAIN_NAMES: usize = 0x662;
    pub const TERRAIN_NAME_LEN: usize = 15;
    pub const CREATURE_NAMES: usize = 0x752;
}

/// Offsets inside one region ([`Layout::regions`] + n * [`region::STRIDE`]).
pub mod region {
    pub const STRIDE: usize = 0xC00;
    pub const MAX_REGIONS: usize = 15;
    pub const MAX_ROOMS: usize = 16;
    pub const ROOM_COUNT: usize = 0x00;
    /// u16: end of the 16 room item lists, relative to the region start.
    pub const ITEMS_END: usize = 0x01;
    /// u16 heap offsets of the variable-length parts.
    pub const PORTALS_PTR: usize = 0x03;
    pub const RESIDENT_NAMES_PTR: usize = 0x05;
    pub const ITEMS_PTR: usize = 0x07;
    /// 16 rooms, 5 bytes each.
    pub const ROOMS: usize = 0x09;
    pub const ROOM_LEN: usize = 5;
    /// Store stock: 14 bytes of possession bits.
    pub const STORE: usize = 0x6D;
    /// 8 random creatures, 37-byte records.
    pub const RANDOM_CREATURES: usize = 0x7B;
    pub const RANDOM_COUNT: usize = 8;
    /// 16 resident creatures, 37-byte records.
    pub const RESIDENTS: usize = 0x1A3;
    pub const RESIDENT_COUNT: usize = 16;
    /// Start of the region heap. Fixed parts: room names (16 x 10 packed
    /// bytes) then random creature names (8 x 10). The header pointers locate
    /// the portal table, resident names and room item lists after them.
    pub const HEAP: usize = 0x3F3;
    pub const ROOM_NAMES: usize = HEAP;
    pub const RANDOM_NAMES: usize = HEAP + 0xA0;
}

/// Bytes of a packed creature, thing or room name (15 characters).
pub const NAME_LEN: usize = 10;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c64_long_message_tail_skips_track_18() {
        assert_eq!(C64.long_message(248), 0x06C00 + 248 * 256);
        assert_eq!(C64.long_message(248) + 256, 0x16500); // track 18 sector 0
        assert_eq!(C64.long_message(249), 0x16700);
        // The last long message is the intro text on both platforms.
        assert_eq!(C64.long_message(254), C64.intro);
        assert_eq!(PC.long_message(254), PC.intro);
        assert_eq!(PC.long_message(249), PC.long_message(248) + 256);
    }

    #[test]
    fn short_message_slots_fit_before_creatures() {
        assert_eq!(PC.short_message(0), 0x16B00);
        assert_eq!(PC.short_message(1), 0x16B78);
        assert_eq!(PC.short_message(2), 0x16C00);
        let last = PC.short_message(SHORT_MESSAGE_SLOTS - 1) + SHORT_MESSAGE_LEN;
        assert!(last <= PC.creatures);
    }

    #[test]
    fn master_creature_records() {
        assert_eq!(creatures::record(0), 0x100);
        assert_eq!(creatures::record(5), 0x100 + 5 * 37);
        assert_eq!(creatures::record(6), 0x200);
        assert!(creatures::record(127) + 37 <= creatures::RETIRED_NAMES);
    }
}
