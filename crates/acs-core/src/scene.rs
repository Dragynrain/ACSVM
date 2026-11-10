// SPDX-License-Identifier: GPL-3.0-or-later

//! What occupies each square of a room or region, as the original shows it.
//!
//! This is layout knowledge, not rendering: it says which picture belongs on
//! a square and which things and creatures stand there, and leaves drawing to
//! the front ends. The rules come from the corpus (see `docs/format-notes.md`,
//! "Drawing rooms"):
//!
//! - The squares on a room's edge are walls and show the room's wall
//!   picture; the rest is floor and shows the picture of master thing 127
//!   ("EMPTY FLOOR" in every corpus adventure).
//! - Room items stack on a square. The first item in the room's list is the
//!   top of its stack and is the one shown; covers are listed first.
//! - Resident creatures stand on top of everything.
//! - Rooms lie on a region grid in squares; neighbours share a wall line.

use crate::model::{Adventure, Region, Room};

/// Master thing whose picture is the floor of every room.
pub const FLOOR_THING: u8 = 127;
/// Master thing that holds the editor's default wall picture.
pub const BORDER_THING: u8 = 128;

/// The fixed part of a square, under any items.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Base {
    Wall,
    Floor,
}

/// One square of a room.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Square {
    pub base: Base,
    /// Indices into [`Room::items`], top of the stack first.
    pub items: Vec<usize>,
    /// Indices into [`Region::residents`] standing here.
    pub residents: Vec<usize>,
}

/// A room resolved into squares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomScene {
    pub width: u8,
    pub height: u8,
    /// Row-major, `width * height` squares.
    pub squares: Vec<Square>,
    /// Items that fall outside the room (the stash square 15,14 and any other).
    pub off_room: Vec<usize>,
}

impl RoomScene {
    /// Lays out `room`, placing the residents of `region` that stand in it.
    pub fn new(region: &Region, room: &Room) -> Self {
        let (w, h) = (room.width, room.height);
        let mut squares = Vec::with_capacity(w as usize * h as usize);
        for y in 0..h {
            for x in 0..w {
                let edge = x == 0 || y == 0 || x + 1 == w || y + 1 == h;
                squares.push(Square {
                    base: if edge { Base::Wall } else { Base::Floor },
                    items: Vec::new(),
                    residents: Vec::new(),
                });
            }
        }
        let mut scene = RoomScene {
            width: w,
            height: h,
            squares,
            off_room: Vec::new(),
        };
        for (i, item) in room.items.iter().enumerate() {
            match scene.index(item.x, item.y) {
                Some(k) => scene.squares[k].items.push(i),
                None => scene.off_room.push(i),
            }
        }
        if !room.deleted {
            for (i, c) in region.residents.iter().enumerate() {
                if c.in_use && c.room == room.number {
                    if let Some(k) = scene.index(c.x, c.y) {
                        scene.squares[k].residents.push(i);
                    }
                }
            }
        }
        scene
    }

    fn index(&self, x: u8, y: u8) -> Option<usize> {
        (x < self.width && y < self.height).then(|| y as usize * self.width as usize + x as usize)
    }

    pub fn square(&self, x: u8, y: u8) -> Option<&Square> {
        self.index(x, y).map(|k| &self.squares[k])
    }
}

/// What a square shows, from the top down.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shown {
    /// Index into [`Region::residents`].
    Resident(usize),
    /// Index into [`Room::items`].
    Item(usize),
    Wall,
    Floor,
}

/// The visible layer of a square and the picture it shows (`None` when the
/// visible thing or creature has no picture, e.g. a floor thing missing from
/// the master list).
pub fn shown(adv: &Adventure, region: &Region, room: &Room, sq: &Square) -> (Shown, Option<u8>) {
    if let Some(&r) = sq.residents.first() {
        return (Shown::Resident(r), region.residents[r].picture);
    }
    // Things without a picture (spells) are skipped; the next one shows.
    for &i in &sq.items {
        if let Some(p) = thing_picture(adv, room.items[i].thing) {
            return (Shown::Item(i), Some(p));
        }
    }
    match sq.base {
        Base::Wall => (Shown::Wall, Some(room.wall_picture)),
        Base::Floor => (Shown::Floor, floor_picture(adv)),
    }
}

/// Picture of master thing `number` (1-based), if it has one.
pub fn thing_picture(adv: &Adventure, number: u8) -> Option<u8> {
    adv.things
        .get((number as usize).checked_sub(1)?)
        .and_then(|t| t.kind.picture())
}

/// The floor picture: master thing 127's.
pub fn floor_picture(adv: &Adventure) -> Option<u8> {
    thing_picture(adv, FLOOR_THING)
}

/// A square-unit rectangle on the region grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridRect {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

impl GridRect {
    pub fn contains(&self, x: u16, y: u16) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
}

/// Where each live room lies on the region grid, plus the bounding box of all
/// of them (`None` for a region without live rooms).
pub fn region_layout(region: &Region) -> (Vec<(usize, GridRect)>, Option<GridRect>) {
    let rooms: Vec<(usize, GridRect)> = region
        .rooms
        .iter()
        .enumerate()
        .filter(|(_, r)| !r.deleted)
        .map(|(i, r)| {
            (
                i,
                GridRect {
                    x: r.x as u16,
                    y: r.y as u16,
                    w: r.width as u16,
                    h: r.height as u16,
                },
            )
        })
        .collect();
    let bounds = rooms.iter().map(|(_, r)| *r).reduce(|a, b| {
        let x = a.x.min(b.x);
        let y = a.y.min(b.y);
        GridRect {
            x,
            y,
            w: (a.x + a.w).max(b.x + b.w) - x,
            h: (a.y + a.h).max(b.y + b.h) - y,
        }
    });
    (rooms, bounds)
}
