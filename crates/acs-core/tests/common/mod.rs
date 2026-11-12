// SPDX-License-Identifier: GPL-3.0-or-later

//! Builds synthetic adventure images from scratch, byte by byte, using the
//! layout constants. No bytes here come from a real adventure.

#![allow(dead_code)]

use acs_core::layout::{
    self, adventure as adv, creatures as cr, region as rg, things as th, world_map as wm, Layout,
};
use acs_core::text::{encode_ascii, encode_packed};
use acs_core::{Image, Platform};

pub struct Fixture {
    pub platform: Platform,
    pub l: &'static Layout,
    pub data: Vec<u8>,
    regions: Vec<RegionSpec>,
}

#[derive(Default, Clone)]
pub struct RoomSpec {
    pub name: &'static str,
    pub w: u8,
    pub h: u8,
    pub x: u8,
    pub y: u8,
    pub wall: u8,
    pub random: u8,
    /// (x, y, thing, parameter)
    pub items: Vec<(u8, u8, u8, u8)>,
}

#[derive(Default, Clone)]
pub struct RegionSpec {
    pub name: &'static str,
    pub rooms: Vec<RoomSpec>,
    pub portals: Vec<[u8; 2]>,
    /// Extra (unused, zero) door-table entries after `portals`, to push the
    /// heap pointers past 0x1FF.
    pub extra_door_entries: usize,
    /// Value for header bytes 3-4 (default 0x00F0, as the editor writes).
    pub header_03: Option<u16>,
    /// (slot 0..16, name, record)
    pub residents: Vec<(usize, &'static str, [u8; 37])>,
    /// (record 0..8, name, record): the region's random creatures.
    pub random_creatures: Vec<(usize, &'static str, [u8; 37])>,
}

impl Fixture {
    pub fn new(platform: Platform) -> Self {
        let size = match platform {
            Platform::Pc => acs_core::disk::PC_MIN_SIZE,
            Platform::C64 => 174_848,
        };
        let mut f = Fixture {
            platform,
            l: Layout::for_platform(platform),
            data: vec![0; size],
            regions: Vec::new(),
        };
        // Empty thing slots are type 15 (undefined).
        for i in 0..th::COUNT {
            f.data[f.l.things + th::PROPS + i * 4] = 0x0F;
        }
        // Empty creature slots have class 8.
        for i in 0..cr::COUNT {
            f.data[f.l.creatures + cr::CLASSES + i] = 8;
        }
        // Unused world map points.
        for k in 0..wm::POINT_COUNT {
            f.data[f.l.world_map + wm::POINTS + k * 2] = 0xFF;
            f.data[f.l.world_map + wm::POINTS + k * 2 + 1] = 0xFF;
        }
        f
    }

    pub fn put(&mut self, addr: usize, bytes: &[u8]) -> &mut Self {
        self.data[addr..addr + bytes.len()].copy_from_slice(bytes);
        self
    }

    pub fn ascii(&mut self, addr: usize, text: &str, len: usize) -> &mut Self {
        let b = encode_ascii(text, len).expect("ascii");
        self.put(addr, &b)
    }

    pub fn packed(&mut self, addr: usize, text: &str) -> &mut Self {
        let b = encode_packed(text, layout::NAME_LEN).expect("packed");
        self.put(addr, &b)
    }

    pub fn header(&mut self, name: &str, byline: &str, intro: &str) -> &mut Self {
        let a = self.l.adventure;
        self.ascii(a + adv::NAME, name, adv::NAME_LEN);
        self.ascii(a + adv::BYLINE, byline, adv::BYLINE_LEN);
        let intro_addr = self.l.intro;
        self.ascii(intro_addr, intro, layout::INTRO_LEN)
    }

    /// Writes thing `number` (1-based): name, 4 property bytes.
    pub fn thing(&mut self, number: usize, name: &str, props: [u8; 4]) -> &mut Self {
        let t = self.l.things;
        self.packed(t + th::NAMES + (number - 1) * 10, name);
        self.put(t + th::PROPS + (number - 1) * 4, &props)
    }

    pub fn thing_message(&mut self, number: usize, slot: usize, value: u8) -> &mut Self {
        let a = self.l.things + th::MESSAGES + (number - 1) + slot * th::MESSAGE_SLOT_STRIDE;
        self.put(a, &[value])
    }

    /// Extended record `index` (weapons/armor at offset 0, magic items after `magic_offset`).
    pub fn extended(&mut self, magic_offset: Option<u8>, index: usize, rec: [u8; 3]) -> &mut Self {
        let t = self.l.things;
        let off = match magic_offset {
            Some(o) => {
                self.data[t + th::MAGIC_ITEM_OFFSET] = o;
                o as usize
            }
            None => 0,
        };
        self.put(t + th::EXTENDED + off + index * 3, &rec)
    }

    pub fn short_message(&mut self, slot: usize, text: &str) -> &mut Self {
        let a = self.l.short_message(slot - 1);
        self.ascii(a, text, layout::SHORT_MESSAGE_LEN)
    }

    pub fn long_message(&mut self, number: usize, text: &str) -> &mut Self {
        let a = self.l.long_message(number - 1);
        self.ascii(a, text, layout::LONG_MESSAGE_LEN)
    }

    /// Master creature `number` (1-based).
    pub fn creature(&mut self, number: usize, name: &str, class: u8, rec: [u8; 37]) -> &mut Self {
        let c = self.l.creatures;
        self.packed(c + cr::NAMES + (number - 1) * 10, name);
        self.put(c + cr::CLASSES + number - 1, &[class]);
        self.put(c + cr::record(number - 1), &rec)
    }

    pub fn region(&mut self, spec: RegionSpec) -> &mut Self {
        self.regions.push(spec);
        self
    }

    /// Lays out all regions (header pointers, heap, item lists) and returns the image.
    pub fn build(&mut self) -> Image {
        let a = self.l.adventure;
        self.data[a + adv::REGION_COUNT] = self.regions.len() as u8;
        for (r, spec) in self.regions.clone().iter().enumerate() {
            let base = self.l.regions + r * rg::STRIDE;
            self.ascii(
                a + adv::REGION_NAMES + r * adv::REGION_NAME_LEN,
                spec.name,
                adv::REGION_NAME_LEN,
            );
            self.data[base + rg::ROOM_COUNT] = spec.rooms.len() as u8;
            let heap = base + rg::HEAP;
            let table = rg::DOOR_TABLE_HEAP_OFFSET;
            let names_ptr = table + (spec.portals.len() + spec.extra_door_entries) * 2;
            let items_ptr = names_ptr + 0xA0;
            for (i, room) in spec.rooms.iter().enumerate() {
                let ra = base + rg::ROOMS + i * rg::ROOM_LEN;
                self.put(
                    ra,
                    &[
                        room.wall,
                        (room.w - 1) << 4 | (room.h - 1),
                        room.x,
                        room.y,
                        room.random,
                    ],
                );
                self.packed(base + rg::ROOM_NAMES + i * 10, room.name);
            }
            for (k, p) in spec.portals.iter().enumerate() {
                self.put(heap + table + k * 2, p);
            }
            for (slot, name, rec) in &spec.residents {
                self.packed(heap + names_ptr + slot * 10, name);
                self.put(base + rg::RESIDENTS + slot * 37, rec);
            }
            for (i, name, rec) in &spec.random_creatures {
                self.packed(base + rg::RANDOM_NAMES + i * 10, name);
                self.put(base + rg::RANDOM_CREATURES + i * 37, rec);
            }
            let mut cur = heap + items_ptr;
            for i in 0..rg::MAX_ROOMS {
                let items = spec.rooms.get(i).map(|r| r.items.clone()).unwrap_or_default();
                let len = (2 + items.len() * 3) as u16;
                self.put(cur, &len.to_le_bytes());
                cur += 2;
                for (x, y, t, p) in items {
                    self.put(cur, &[x << 4 | y, t, p]);
                    cur += 3;
                }
            }
            let end = (cur - base) as u16;
            self.put(base + rg::ITEMS_END, &end.to_le_bytes());
            let h03 = spec.header_03.unwrap_or(table as u16);
            self.put(base + rg::HEADER_03, &h03.to_le_bytes());
            self.put(base + rg::RESIDENT_NAMES_PTR, &(names_ptr as u16).to_le_bytes());
            self.put(base + rg::ITEMS_PTR, &(items_ptr as u16).to_le_bytes());
        }
        Image::from_bytes(self.platform, self.data.clone()).expect("fixture size")
    }
}

/// A creature record with the given core stats; everything else zero.
pub fn creature_record(con: u8, strategy: u8, picture: u8) -> [u8; 37] {
    let mut r = [0u8; 37];
    r[0x00] = con;
    r[0x01] = 12; // strength
    r[0x02] = 13; // dexterity
    r[0x03] = con & 0x3F; // life force
    r[0x04] = strategy;
    r[0x05] = 40; // power
    r[0x11] = 9; // wisdom
    r[0x16] = picture + 2;
    r
}
