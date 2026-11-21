// SPDX-License-Identifier: GPL-3.0-or-later

//! World map encounters: the roll after a map step, the four encounter
//! areas (rooms 1..4 of the map's own region 0), entering, leaving and
//! closing them. Spec: `docs/rules/encounters.md` (section numbers in the
//! comments refer to it).

use crate::game::Game;
use crate::{Event, Location, RemovalReason};
use acs_core::model::{
    Adventure, Alignment, Creature, MagicDefense, Possession, Region, Room, Strategy, ThingId,
};

/// Encounter areas, rooms 1..=AREAS of the map region (3.1).
pub const AREAS: u8 = 4;
/// An area's size, walls included (3.2).
pub const AREA_WIDTH: u8 = 15;
pub const AREA_HEIGHT: u8 = 10;
/// The thing whose items are an area's exit doors (3.5).
pub const AREA_DOOR_THING: ThingId = acs_core::layout::world_map::AREA_DOOR_THING as ThingId;
/// Where a player is put on entering (3.6).
pub const AREA_ENTRY: (u8, u8) = (12, 4);
/// The map region's slots that take encounter copies (2.6).
pub const FIRST_COPY_SLOT: usize = 8;
/// The map region's creature slots.
const MAP_SLOTS: usize = 16;

/// Record `r`'s square in an area (3.6).
pub fn creature_square(r: usize) -> (u8, u8) {
    (2 + (r % 2) as u8, 3 + (r / 2) as u8)
}

fn blank_creature(number: u16) -> Creature {
    Creature {
        number,
        in_use: false,
        name: String::new(),
        class: None,
        picture: None,
        constitution: 0,
        life_force: 0,
        strength: 0,
        dexterity: 0,
        speed: 0,
        wisdom: 0,
        power: 0,
        size: 0,
        dodge: 0,
        parry: 0,
        armor_skill: 0,
        melee: 0,
        missile: 0,
        strategy: Strategy {
            alignment: Alignment::Neutral,
            aggressive: false,
            brave: false,
        },
        magic_defense: MagicDefense::None,
        mimic: false,
        wealth: 0,
        weapon: 0,
        armor: 0,
        x: 0,
        y: 0,
        region: 0,
        room: 0,
        possessions: Vec::new(),
        unknown_0b: 0,
        last_turn_round: 0,
    }
}

fn free_room(number: u8) -> Room {
    Room {
        number,
        name: String::new(),
        deleted: true,
        wall_picture: 0,
        width: 0,
        height: 0,
        x: 0,
        y: 0,
        random_chance: 0,
        random_creature: 0,
        items: Vec::new(),
    }
}

/// The world map's own region (3.1): room 0 stands for the open map and is
/// never a room; rooms 1..4 are the areas, free (deleted) until used; 16
/// creature slots, the copies in 8..15; no stock yet (`seeded_map_region`
/// builds it). With no lists (a game saved before them) the areas start
/// empty.
pub(crate) fn map_region() -> Region {
    Region {
        number: 0,
        name: String::new(),
        rooms: (0..=AREAS).map(free_room).collect(),
        random_creatures: Vec::new(),
        residents: (0..MAP_SLOTS).map(|i| blank_creature(i as u16 + 1)).collect(),
        store: Vec::new(),
        portal_table: Vec::new(),
        unknown_header_03: 0,
        unknown_59: Vec::new(),
    }
}

/// The map region with its areas seeded from the world map's lists
/// (3.4): area n gets list n, once, at the start of the game. Its stock
/// (stores 2.7) is every thing 1..80 a store can stock (treasures, magic
/// items, weapons and armor, never spells), value 0 included, each up to
/// the stock limit (stores 2.6); buying and selling there never change it.
pub(crate) fn seeded_map_region(adv: &Adventure) -> Region {
    let mut region = map_region();
    for (n, items) in adv.world.area_items.iter().enumerate().take(AREAS as usize) {
        region.rooms[n + 1].items = items.clone();
    }
    region.store = (1..=80)
        .filter(|&t| crate::stores::value(adv, t).is_some())
        .map(|thing| Possession {
            thing,
            count: crate::inventory::limit(thing) as u8,
        })
        .collect();
    region
}

impl Game {
    /// The exit door (3.5) at `at`, if it is one: an item of thing 82 on
    /// that square of an area, the first in the list giving the parameter
    /// n, which picks its entry in the area door table (7.1). Front ends
    /// draw it as an opening on a wall square, as floor inside.
    pub fn area_door(&self, at: Location) -> Option<u8> {
        let Location::Room {
            region: 0,
            room,
            x,
            y,
        } = at
        else {
            return None;
        };
        if !(1..=AREAS).contains(&room) {
            return None;
        }
        self.map
            .rooms
            .get(room as usize)?
            .items
            .iter()
            .find(|it| it.thing == AREA_DOOR_THING && (it.x, it.y) == (x, y))
            .map(|it| it.parameter)
    }

    /// The first free area, 1..=4.
    fn free_area(&self) -> Option<u8> {
        (1..=AREAS).find(|&n| self.map.rooms.get(n as usize).is_some_and(|r| r.deleted))
    }

    /// The open area whose entrance is map square (x, y).
    fn area_at(&self, x: u8, y: u8) -> Option<u8> {
        (1..=AREAS).find(|&n| self.entrances.get(n as usize - 1) == Some(&Some((x, y))))
    }

    /// After a completed step onto map square (x, y), the move used:
    /// joining an open area on its entrance (5.2), else the roll (2). True
    /// when the player is now in an area; its turn is over then (5.1).
    pub(crate) fn after_map_step(&mut self, p: usize, x: u8, y: u8, ev: &mut Vec<Event>) -> bool {
        if let Some(area) = self.area_at(x, y) {
            self.enter_area(p, area, ev);
            return true;
        }
        // 2.4: no rolls at all while every area is in use.
        let Some(area) = self.free_area() else {
            return false;
        };
        let terrain = self.tile(x, y);
        // 2.5: every record on the terrain rolls, in record order, and
        // every hit appears.
        let rolls: Vec<(usize, u8)> = self
            .adv
            .world
            .random_creatures
            .iter()
            .enumerate()
            .filter(|(_, c)| c.terrain == terrain)
            .map(|(i, c)| (i, c.chance))
            .collect();
        let hits: Vec<usize> = rolls
            .into_iter()
            .filter(|&(_, chance)| self.rng.percent(chance))
            .map(|(i, _)| i)
            .collect();
        let mut placed = false;
        for r in hits {
            let slot = self.place_copy(area, r);
            placed |= slot.is_some();
            ev.push(Event::Encounter {
                player: p,
                creature: r,
                terrain,
                area,
                slot,
            });
        }
        if !placed {
            return false;
        }
        // 3.1-3.4: the area opens, walled with the entered terrain's picture.
        let picture = self
            .adv
            .world
            .terrain
            .get(terrain as usize)
            .map_or(0, |t| t.picture);
        let room = &mut self.map.rooms[area as usize];
        room.deleted = false;
        room.width = AREA_WIDTH;
        room.height = AREA_HEIGHT;
        room.wall_picture = picture;
        self.entrances[area as usize - 1] = Some((x, y));
        self.enter_area(p, area, ev);
        true
    }

    /// 2.6, 4: a copy of map record `r` in the first free slot 8..15, on its
    /// square of `area`, marked random.
    fn place_copy(&mut self, area: u8, r: usize) -> Option<usize> {
        let rec = self.adv.world.random_creatures.get(r)?;
        let (x, y) = creature_square(r);
        if !rec.creature.in_use || x >= AREA_WIDTH - 1 || y >= AREA_HEIGHT - 1 {
            return None;
        }
        let slot = (FIRST_COPY_SLOT..self.map.residents.len()).find(|&s| !self.map.residents[s].in_use)?;
        let mut c = rec.creature.clone();
        c.in_use = true;
        c.number = slot as u16 + 1;
        c.region = 0;
        c.room = area;
        c.x = x;
        c.y = y;
        self.map.residents[slot] = c;
        self.fill_slot(0, slot, true);
        Some(slot)
    }

    /// 5.1, 5.2: the player stands on 12,4 of `area`; its turn ends (the
    /// caller ends it).
    fn enter_area(&mut self, p: usize, area: u8, ev: &mut Vec<Event>) {
        let from = self.players[p].at;
        let (x, y) = AREA_ENTRY;
        let to = Location::Room {
            region: 0,
            room: area,
            x,
            y,
        };
        self.players[p].at = to;
        self.players[p].moves_left = 0;
        if !self.turn_rooms.contains(&(0, area)) {
            self.turn_rooms.push((0, area));
        }
        ev.push(Event::EnteredArea {
            player: p,
            area,
            from,
            to,
        });
    }

    /// 3.5: entry `n` of the area door table, two bytes in the room door
    /// format: n 0..3 from `unknown_806`, n 4 the word `unknown_80e` (low
    /// byte first); none above 4.
    pub fn area_door_entry(&self, n: u8) -> Option<[u8; 2]> {
        let world = &self.adv.world;
        match n {
            0..=3 => {
                let i = n as usize * 2;
                Some([*world.unknown_806.get(i)?, *world.unknown_806.get(i + 1)?])
            }
            4 => Some(world.unknown_80e.to_le_bytes()),
            _ => None,
        }
    }

    /// 3.5: where a door of parameter `n` leads, `None` for the start
    /// point: region 0 with code 0x41..0x44 that area's entrance (the start
    /// while it is free), region 0 with point 0..63 that map point, region
    /// 1 or above that room's square if the adventure has it. Front ends
    /// and the script player ask it where an area's exit leads.
    pub fn area_door_destination(&self, n: u8) -> Option<Location> {
        let [b0, b1] = self.area_door_entry(n)?;
        let region = b0 >> 4;
        if region == 0 {
            use acs_core::layout::world_map as wm;
            let world = &self.adv.world;
            let (x, y) = match b1 {
                0x41..=0x44 => self.area_entrance(b1 - 0x40)?,
                p if (p as usize) < wm::DOOR_COUNT => world
                    .doors
                    .iter()
                    .find(|d| d.index == u16::from(p))
                    .map(|d| (d.at.x, d.at.y))?,
                p if (p as usize) < wm::POINT_COUNT => world
                    .exit_points
                    .get(p as usize - wm::DOOR_COUNT)
                    .copied()
                    .flatten()
                    .map(|pt| (pt.x, pt.y))?,
                _ => return None,
            };
            return self.on_map(x, y).then_some(Location::World { x, y });
        }
        let (room, x, y) = (b0 & 0x0F, b1 >> 4, b1 & 0x0F);
        self.room(region, room)
            .filter(|r| x < r.width && y < r.height)
            .map(|_| Location::Room { region, room, x, y })
    }

    /// 7.1, 3.5: through a door of parameter `n`, to where its area door
    /// table entry leads, or the start point; then `area` closes if no
    /// living player is left in it (7.2). Arriving is not a step: no map
    /// roll, no door taken; into another region it is a region change, with
    /// the room's encounter roll (movement 7.2). The caller spends the step.
    pub(crate) fn leave_area(&mut self, p: usize, area: u8, n: u8, ev: &mut Vec<Event>) {
        let from = self.players[p].at;
        let to = self.area_door_destination(n).unwrap_or_else(|| {
            let s = self.adv.world.start;
            Location::World { x: s.x, y: s.y }
        });
        self.players[p].at = to;
        ev.push(Event::LeftArea {
            player: p,
            area,
            from,
            to,
        });
        self.close_if_empty(area, ev);
        if to.room().is_some() {
            self.arrive(p, from, to, ev);
        }
    }

    /// 7.2, 7.4: with no living player left in `area`, it is freed, its
    /// entrance forgotten, its copies removed and its items deleted.
    pub(crate) fn close_if_empty(&mut self, area: u8, ev: &mut Vec<Event>) {
        if !(1..=AREAS).contains(&area) || self.map.rooms[area as usize].deleted {
            return;
        }
        let occupied = self
            .players
            .iter()
            .any(|p| p.alive && p.at.room() == Some((0, area)));
        if occupied {
            return;
        }
        for slot in 0..self.map.residents.len() {
            let c = &self.map.residents[slot];
            if c.in_use && c.room == area {
                self.remove_creature(0, slot, RemovalReason::AreaClosed, ev);
            }
        }
        // 7.2: the doors stay, every other item is deleted.
        let room = &mut self.map.rooms[area as usize];
        room.items.retain(|it| it.thing == AREA_DOOR_THING);
        room.deleted = true;
        room.width = 0;
        room.height = 0;
        self.entrances[area as usize - 1] = None;
        ev.push(Event::AreaClosed { area });
    }

    /// The map square an open area was entered from (7.1).
    pub fn area_entrance(&self, area: u8) -> Option<(u8, u8)> {
        self.entrances
            .get((area as usize).checked_sub(1)?)
            .copied()
            .flatten()
    }
}
