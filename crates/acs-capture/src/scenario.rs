// SPDX-License-Identifier: GPL-3.0-or-later

//! Scenario adventures, written byte by byte from our layout constants with
//! the same builder the synthetic tests use. Nothing here comes from a real
//! adventure; the pictures are plain marker patterns of our own.

#[path = "../../acs-core/tests/common/mod.rs"]
mod common;

use acs_core::layout::{world_map as wm, Layout};
use acs_core::{Image, Platform};
use common::{Fixture, RegionSpec, RoomSpec};

/// Marker colours (C64 colour codes): background, 1, 2 and 3 (3 only 0..7).
pub const PALETTE: [u8; 4] = [0x00, 0x01, 0x02, 0x05];

/// Pictures we draw: a blank floor, a wall pattern, and solid creature
/// pictures, so a character shows as a 16 x 16 block of colour 3.
pub const PIC_FLOOR: usize = 0;
pub const PIC_WALL: usize = 1;
/// A solid colour-2 square: a landmark that shows where the view is.
pub const PIC_MARKER: usize = 2;

/// The map cell the player starts on, and the open field around it.
pub struct MoveBasic {
    pub start: (u8, u8),
    /// Inclusive bounds of the open field; the ring around it is wall.
    pub field: (u8, u8, u8, u8),
    /// An open marker cell inside the field (terrain 2), always in view.
    pub marker: (u8, u8),
}

impl Default for MoveBasic {
    fn default() -> Self {
        MoveBasic {
            start: (12, 12),
            field: (6, 6, 24, 18),
            marker: (15, 14),
        }
    }
}

fn picture(f: &mut Fixture, n: usize, rows: [u8; 16], right: [u8; 16]) {
    let base = f.l.pictures + n * 16;
    for r in 0..16 {
        let half = if r < 8 { 0 } else { 0x800 };
        let row = r % 8;
        f.data[base + half + row] = rows[r];
        f.data[base + half + 8 + row] = right[r];
    }
}

/// Starts a scenario: from zeros, or from a copy of a blank adventure disk
/// made by the user's own copy of the original program ("make an adventure
/// disk"). The original refuses disks that lack what that step writes
/// outside the adventure blocks, so real runs need the template; it is read
/// at run time and never stored in the repository (docs/capture.md 1.4).
fn start(template: Option<&[u8]>) -> Fixture {
    let mut f = Fixture::new(Platform::C64);
    if let Some(t) = template {
        assert_eq!(t.len(), f.data.len(), "template must be a 35-track .d64");
        f.data = t.to_vec();
    }
    f
}

/// `move-basic` (docs/capture.md section 7): one player on an open field of
/// the world map, walled in, nothing else.
pub fn move_basic(s: &MoveBasic, template: Option<&[u8]>) -> Image {
    let mut f = start(template);
    let l: &Layout = f.l;
    f.header("MOVE TEST", "HARNESS", "A FIELD TO WALK ON.");

    let p = l.palette;
    f.data[p..p + 4].copy_from_slice(&[PALETTE[0], PALETTE[1], PALETTE[2], 0x08 | PALETTE[3]]);

    // Every non-creature picture blank (the template holds builder-set art),
    // then the wall: colour 1 checks (01 pairs = 0x55, 0x00 rows between).
    for n in 0..64 {
        picture(&mut f, n, [0; 16], [0; 16]);
    }
    let wall: [u8; 16] = std::array::from_fn(|r| if r % 2 == 0 { 0x55 } else { 0x00 });
    picture(&mut f, PIC_WALL, wall, wall);
    picture(&mut f, PIC_MARKER, [0xAA; 16], [0xAA; 16]);
    // Creature pictures 64..109: solid colour 3.
    for n in 64..110 {
        picture(&mut f, n, [0xFF; 16], [0xFF; 16]);
    }

    let w = l.world_map;
    // Terrain 0: open field, blank picture. Terrain 1: impassable wall.
    f.put(w + wm::TERRAIN, &[0x40 | PIC_FLOOR as u8, 0]);
    f.put(w + wm::TERRAIN + 2, &[PIC_WALL as u8, 0]);
    f.put(w + wm::TERRAIN + 4, &[0x40 | PIC_MARKER as u8, 0]);
    f.ascii(w + wm::TERRAIN_NAMES, "FIELD", wm::TERRAIN_NAME_LEN);
    f.ascii(
        w + wm::TERRAIN_NAMES + wm::TERRAIN_NAME_LEN,
        "WALL",
        wm::TERRAIN_NAME_LEN,
    );
    f.ascii(
        w + wm::TERRAIN_NAMES + 2 * wm::TERRAIN_NAME_LEN,
        "MARKER",
        wm::TERRAIN_NAME_LEN,
    );
    let (x0, y0, x1, y1) = s.field;
    for y in 0..wm::SIZE {
        for x in 0..wm::SIZE {
            let (xu, yu) = (x as u8, y as u8);
            let inside = xu >= x0 && xu <= x1 && yu >= y0 && yu <= y1;
            let t = if (xu, yu) == s.marker {
                2u8
            } else if inside {
                0
            } else {
                1
            };
            let a = w + wm::MAP + y * (wm::SIZE / 2) + x / 2;
            if x % 2 == 0 {
                f.data[a] = (f.data[a] & 0x0F) | t << 4;
            } else {
                f.data[a] = (f.data[a] & 0xF0) | t;
            }
        }
    }
    f.put(w + wm::START_X, &[s.start.0]);
    f.put(w + wm::START_Y, &[s.start.1]);
    f.ascii(
        l.adventure + acs_core::layout::adventure::MAP_NAME,
        "TEST FIELD",
        20,
    );

    // One region with one small room, so the region list isn't empty.
    f.region(RegionSpec {
        name: "TEST REGION",
        rooms: vec![RoomSpec {
            name: "TEST ROOM",
            w: 8,
            h: 6,
            ..RoomSpec::default()
        }],
        ..RegionSpec::default()
    });
    f.build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_basic_decodes() {
        let img = move_basic(&MoveBasic::default(), None);
        let adv = acs_core::decode(&img).expect("decodes").adventure;
        let m = &adv.world;
        assert_eq!((m.start.x, m.start.y), (12, 12));
        assert_eq!(m.tiles[12][12], 0);
        assert_eq!(m.tiles[5][12], 1);
        assert_eq!(m.terrain[0].name, "FIELD");
        assert_eq!(m.tiles[14][15], 2);
        assert_eq!(adv.pictures[70].pixel(3, 9), 3);
        assert!(adv.pictures[0].is_blank());
    }
}
