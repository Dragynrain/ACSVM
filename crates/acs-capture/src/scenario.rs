// SPDX-License-Identifier: GPL-3.0-or-later

//! Scenario adventures, written byte by byte from our layout constants with
//! the same builder the synthetic tests use. Nothing here comes from a real
//! adventure; the pictures are plain marker patterns of our own.

#[path = "../../acs-core/tests/common/mod.rs"]
mod common;

use acs_core::layout::{creatures as cr, world_map as wm, Layout};
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
/// The map door: a solid colour-1 square.
pub const PIC_DOOR: usize = 3;
/// A second landmark: colour 2 in the top half only (a 16 x 8 box), so the
/// observer can tell it from the first.
pub const PIC_MARKER2: usize = 4;
/// A third landmark: colour 2 in the left half only (an 8 x 16 box).
pub const PIC_MARKER3: usize = 5;

/// A player of our own, written as retired player 1 so the start-up screen
/// offers it (docs/capture.md section 8). The program rolls the stats of a
/// new player; this one has the stats we choose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Player {
    pub name: String,
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
    pub melee: u8,
    pub missile: u8,
}

impl Default for Player {
    /// The stats the program rolled for move-basic's player.
    fn default() -> Self {
        Player {
            name: "T".into(),
            constitution: 11,
            life_force: 11,
            strength: 9,
            dexterity: 13,
            speed: 6,
            wisdom: 13,
            power: 13,
            size: 9,
            dodge: 30,
            parry: 26,
            melee: 31,
            missile: 22,
        }
    }
}

impl Player {
    /// Applies `key=value` pairs separated by commas (con, lf, str, dex,
    /// spd, wis, pow, size, dodge, parry, melee, missile, name).
    pub fn with(mut self, spec: &str) -> Result<Self, String> {
        for kv in spec.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            let (k, v) = kv.split_once('=').ok_or(format!("{kv}: want key=value"))?;
            if k == "name" {
                self.name = v.to_string();
                continue;
            }
            let n: u8 = v.parse().map_err(|_| format!("{kv}: bad number"))?;
            let (field, max) = match k {
                "con" => (&mut self.constitution, 63),
                "lf" => (&mut self.life_force, 63),
                "str" => (&mut self.strength, 31),
                "dex" => (&mut self.dexterity, 31),
                "spd" => (&mut self.speed, 15),
                "wis" => (&mut self.wisdom, 31),
                "pow" => (&mut self.power, 127),
                "size" => (&mut self.size, 31),
                "dodge" => (&mut self.dodge, 127),
                "parry" => (&mut self.parry, 127),
                "melee" => (&mut self.melee, 127),
                "missile" => (&mut self.missile, 127),
                _ => return Err(format!("{k}: unknown stat")),
            };
            if n > max {
                return Err(format!("{kv}: at most {max}"));
            }
            *field = n;
        }
        Ok(self)
    }

    /// The 37-byte record, laid out as the decoder reads creature records.
    /// Byte 1 bit 6 is set as in the records the program wrote for its own
    /// players (meaning unknown). The player stands on the map start cell
    /// with nothing carried; the first master creature picture.
    pub fn record(&self, start: (u8, u8)) -> [u8; 37] {
        let mut r = [0u8; 37];
        r[0x00] = self.constitution;
        r[0x01] = 0x40 | self.strength;
        r[0x02] = self.dexterity;
        r[0x03] = self.life_force;
        r[0x04] = self.speed; // alignment 0 (thief), not aggressive or brave
        r[0x05] = self.power;
        r[0x06] = self.dodge;
        r[0x07] = self.parry;
        r[0x09] = self.melee;
        r[0x0A] = self.missile;
        r[0x0D] = start.0;
        r[0x0E] = start.1;
        r[0x0F] = self.size;
        r[0x11] = self.wisdom;
        r[0x16] = 64 + 2;
        r
    }
}

/// The box (width, height in screen pixels) of resident 0, 1, ..
/// pictures, `RESIDENT_INSET` pixels in from the square's top-left corner
/// and short of its other edges, so it never touches a neighbour's; past
/// the list they are solid 16 x 16.
pub const RESIDENT_BOXES: [(usize, usize); 6] = [(4, 12), (12, 4), (4, 4), (8, 12), (12, 8), (8, 8)];
pub const RESIDENT_INSET: usize = 2;

/// A creature of ours living in the scenario's room (a resident slot of
/// region 1), for the creature captures (docs/capture.md 10.10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resident {
    /// Name and stats, as for a player (`name` up to 10 letters).
    pub stats: Player,
    /// Its room square, counted from the room's top-left wall square.
    pub at: (u8, u8),
    /// 0 thief, 1 neutral, 2 enemy, 3 friend (as the decoder reads them).
    pub alignment: u8,
    pub aggressive: bool,
    pub brave: bool,
    /// Its room in region 1 (0 the test room, 1 the side room).
    pub room: u8,
    /// Gold carried (the wealth count).
    pub wealth: u16,
    /// Treasures of ours it owns, by weight (things `CARRY`.., numbered
    /// across all residents in slot order).
    pub carry: Vec<u16>,
}

impl Resident {
    /// `key=value` pairs separated by commas: `x`, `y`, `align` (thief,
    /// neutral, enemy, friend), `aggressive` and `brave` (0 or 1), `room`
    /// (0 or 1), `gold` (wealth), `carry` (treasure weights joined by `+`),
    /// and any stat `Player::with` takes. Default: a Speed 3 peaceful, cautious
    /// friend named A with Constitution and Life Force 10.
    pub fn parse(spec: &str) -> Result<Self, String> {
        let mut r = Resident {
            stats: Player::default().with("name=A,spd=3,con=10,lf=10")?,
            at: (8, 2),
            alignment: 3,
            aggressive: false,
            brave: false,
            room: 0,
            wealth: 0,
            carry: Vec::new(),
        };
        let mut rest = Vec::new();
        for kv in spec.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            let (k, v) = kv.split_once('=').ok_or(format!("{kv}: want key=value"))?;
            let flag = || match v {
                "0" => Ok(false),
                "1" => Ok(true),
                _ => Err(format!("{kv}: 0 or 1")),
            };
            let num = || v.parse::<u8>().map_err(|_| format!("{kv}: bad number"));
            match k {
                "x" => r.at.0 = num()?,
                "y" => r.at.1 = num()?,
                "align" => {
                    r.alignment = match v {
                        "thief" => 0,
                        "neutral" => 1,
                        "enemy" => 2,
                        "friend" => 3,
                        _ => return Err(format!("{kv}: thief, neutral, enemy or friend")),
                    }
                }
                "aggressive" => r.aggressive = flag()?,
                "brave" => r.brave = flag()?,
                "room" => r.room = num()?,
                "gold" => r.wealth = v.parse().map_err(|_| format!("{kv}: bad number"))?,
                "carry" => {
                    r.carry = v
                        .split('+')
                        .map(|w| w.parse::<u16>().map_err(|_| format!("{kv}: bad weight")))
                        .collect::<Result<_, _>>()?
                }
                _ => rest.push(kv),
            }
        }
        r.stats = r.stats.with(&rest.join(","))?;
        Ok(r)
    }

    /// The resident record: the player record's stats on its square, the
    /// strategy in byte 4 with the Speed, its room of region 1, picture
    /// `picture` (64..109, all solid in our scenarios), its gold, and
    /// things `first_carry`.. for its `carry` list.
    pub fn record(&self, picture: u8, first_carry: u8) -> [u8; 37] {
        let mut r = self.stats.record(self.at);
        r[0x01] = self.stats.strength;
        r[0x04] = self.stats.speed
            | self.alignment << 4
            | u8::from(self.aggressive) << 6
            | u8::from(self.brave) << 7;
        r[0x10] = 1 << 4 | (self.room & 0x0F);
        r[0x12..0x14].copy_from_slice(&self.wealth.to_le_bytes());
        r[0x16] = picture + 2;
        for k in 0..self.carry.len() {
            let i = (first_carry as usize + k) - 33;
            r[0x17 + 8 + i / 8] |= 0x80 >> (i % 8);
        }
        r
    }
}

/// The map cell the player starts on, and the open field around it.
pub struct MoveBasic {
    pub start: (u8, u8),
    /// Inclusive bounds of the open field; the ring around it is wall.
    pub field: (u8, u8, u8, u8),
    /// An open marker cell inside the field (terrain 2, a 16 x 16 box).
    pub marker: (u8, u8),
    /// A second marker (terrain 3, a 16 x 8 box), so one stays in view
    /// along a walk east of more than 12 squares.
    pub marker2: (u8, u8),
    /// A third marker (terrain 4, an 8 x 16 box), in view near the west
    /// wall.
    pub marker3: (u8, u8),
    /// A map door (terrain 12, door 0) into region 1, room 0.
    pub door: (u8, u8),
    /// Where the door leads: a square of that room, counted from its
    /// top-left wall square.
    pub room_entry: (u8, u8),
    /// A retired player of our own, offered at start-up.
    pub player: Option<Player>,
    /// An obstacle (thing `OBSTACLE`, type 10) on this room square.
    pub obstacle: Option<(u8, u8)>,
    /// Treasures of these weights (things `FIRST_WEIGHT`..), in a row east
    /// of the room's entry square, one a square.
    pub weights: Vec<u16>,
    /// Magic spells of ours (things `SPELL`, `SPELL` - 1, ..); the retired
    /// player owns them.
    pub spells: Vec<Spell>,
    /// A magic item of ours (thing `ITEM`) that acts when used, with this
    /// action and parameter; the retired player owns it.
    pub item: Option<(u8, u8)>,
    /// A second retired player of ours, in the second slot.
    pub player2: Option<Player>,
    /// Treasures (things `TAKE`, `TAKE` + 1, ..) on these room squares, of
    /// weight 1 unless `take_weights` gives their first two property bytes.
    pub takes: Vec<(u8, u8)>,
    pub take_weights: Vec<(u8, u8)>,
    /// The `takes` show no message when taken (the `loose` ones keep the
    /// template's, which the runs answer).
    pub quiet_takes: bool,
    /// A one-way portal (thing `EXIT`) on this room square, to world map
    /// point 32 at the second cell (door table entry 1).
    pub exit: Option<((u8, u8), (u8, u8))>,
    /// A portal open only to owners of the key (thing `LOCK`, door table
    /// entry 2).
    pub lock: Option<Lock>,
    /// Creatures of ours in the room, resident slots 0, 1, ..
    pub residents: Vec<Resident>,
    /// A second room (room 1, 10 x 8, SIDE ROOM) and a one-way portal
    /// (thing `SIDE`, door table entry 3) on this room 0 square leading to
    /// the second square, in room 1.
    pub side: Option<((u8, u8), (u8, u8))>,
    /// The region's random creature 0 (picture 70, `RESIDENT_BOXES[5]`)
    /// and room 0's chance for it, 0..15 (5% each).
    pub random_creature: Option<Resident>,
    pub room_chance: u8,
    /// The side room's chance for random creature 0.
    pub side_chance: u8,
    /// Piles of gold (thing `GOLD`) on room 0 squares: x, y, amount.
    pub gold: Vec<(u8, u8, u8)>,
}

/// A locked one-way room door of ours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lock {
    /// The portal's room square.
    pub at: (u8, u8),
    /// Where it leads, in the same room.
    pub to: (u8, u8),
    /// "Destroy needed thing": passing takes the key.
    pub destroy: bool,
    /// Whether the retired player owns the key (thing `KEY`).
    pub key_owned: bool,
}

/// The first master thing slot of the `takes` treasures.
pub const TAKE: u8 = 60;
/// The key, a treasure (carriable: 1-80).
pub const KEY: u8 = 69;
/// The exit portal and the locked portal.
pub const EXIT: u8 = 101;
pub const LOCK: u8 = 102;
/// The portal into the side room.
pub const SIDE: u8 = 103;

/// One spell of ours: its cost and what it does (action code, parameter).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spell {
    pub cost: u8,
    pub action: u8,
    pub param: u8,
}

/// Action code 10: display long message.
pub const LONG_MESSAGE: u8 = 10;

impl Spell {
    /// `COST` (shows long message 1) or `COST:ACTION:PARAM`.
    pub fn parse(spec: &str) -> Result<Self, String> {
        let n: Vec<&str> = spec.split(':').collect();
        let num = |t: &str| -> Result<u8, String> {
            let t = t.trim();
            let v = if let Some(h) = t.strip_prefix("0x") {
                u8::from_str_radix(h, 16)
            } else {
                t.parse()
            };
            v.map_err(|_| format!("{spec}: bad number {t}"))
        };
        match n.as_slice() {
            [c] => Ok(Spell {
                cost: num(c)?,
                action: LONG_MESSAGE,
                param: 1,
            }),
            [c, a, p] => Ok(Spell {
                cost: num(c)?,
                action: num(a)?,
                param: num(p)?,
            }),
            _ => Err(format!("{spec}: want COST or COST:ACTION:PARAM")),
        }
    }
}

/// The master thing slot of our first spell (1-80, so a player can own it);
/// further spells take the slots below it.
pub const SPELL: u8 = 80;
/// Long message 1, shown when the spell works.
pub const SPELL_MESSAGE: &str = "THE SPELL WORKED";
/// The master thing slot of our magic item, below the weight row.
pub const ITEM: u8 = 70;
/// Long message 2, for an item that displays a message.
pub const ITEM_MESSAGE: &str = "THE ITEM WORKED";

/// The first master thing slot of the weight row (carriable things must be
/// 1-80: possessions are a bitmap over them).
pub const FIRST_WEIGHT: u8 = 71;

/// A treasure's first two property bytes for a weight: x1 up to 255, else
/// x20 (bit 6) for multiples of 20 up to 5,100.
pub fn weight_bytes(w: u16) -> Result<(u8, u8), String> {
    if w <= 255 {
        Ok((0x01, w as u8))
    } else if w % 20 == 0 && w / 20 <= 255 {
        Ok((0x01 | 0x40, (w / 20) as u8))
    } else {
        Err(format!(
            "weight {w}: above 255 it must be a multiple of 20, at most 5,100"
        ))
    }
}

/// A take weight: `N` (as `weight_bytes`) or `x20:N`, stored as N / 20
/// with the x20 bit even when N fits in a byte.
pub fn take_weight(spec: &str) -> Result<(u8, u8), String> {
    match spec.strip_prefix("x20:") {
        Some(n) => {
            let w: u16 = n.parse().map_err(|_| format!("{spec}: bad weight"))?;
            if w % 20 != 0 || w / 20 > 255 {
                return Err(format!("{spec}: a multiple of 20, at most 5,100"));
            }
            Ok((0x01 | 0x40, (w / 20) as u8))
        }
        None => weight_bytes(spec.parse().map_err(|_| format!("{spec}: bad weight"))?),
    }
}

/// The first master thing slot of the treasures residents carry (up to
/// `TAKE`).
pub const CARRY: u8 = 50;
/// Gold: thing 81 in every corpus adventure (docs/format-notes.md).
pub const GOLD: u8 = 81;

/// The master thing slot our obstacle takes (over the builder set's).
pub const OBSTACLE: u8 = 100;

impl Default for MoveBasic {
    fn default() -> Self {
        MoveBasic {
            start: (12, 12),
            field: (6, 6, 34, 18),
            marker: (15, 14),
            marker2: (27, 14),
            marker3: (9, 14),
            door: (12, 16),
            room_entry: (4, 4),
            player: None,
            obstacle: None,
            weights: Vec::new(),
            spells: Vec::new(),
            item: None,
            player2: None,
            takes: Vec::new(),
            take_weights: Vec::new(),
            quiet_takes: false,
            exit: None,
            lock: None,
            residents: Vec::new(),
            side: None,
            random_creature: None,
            room_chance: 0,
            side_chance: 0,
            gold: Vec::new(),
        }
    }
}

impl MoveBasic {
    /// The `loose` scenario (docs/capture.md 10.8): a Speed 6 player of
    /// ours with Wisdom 20 (and `player` over that) owning the item, two
    /// things to take west of the room's entry, a door out to the map
    /// south of it and a locked door north of it.
    pub fn loose(player: &str, key: bool, destroy: bool) -> Result<Self, String> {
        Ok(MoveBasic {
            player: Some(Player::default().with("spd=6,wis=20")?.with(player)?),
            item: Some((LONG_MESSAGE, 2)),
            takes: vec![(3, 4), (2, 4)],
            exit: Some(((4, 5), (20, 12))),
            lock: Some(Lock {
                at: (4, 3),
                to: (6, 7),
                destroy,
                key_owned: key,
            }),
            ..MoveBasic::default()
        })
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

/// Clears a thing's two message references: the template's builder-set
/// thing in that slot may have had one, which the program would show.
fn no_messages(f: &mut Fixture, n: usize) {
    f.thing_message(n, 0, 0);
    f.thing_message(n, 1, 0);
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
    picture(&mut f, PIC_DOOR, [0x55; 16], [0x55; 16]);
    let half: [u8; 16] = std::array::from_fn(|r| if r < 8 { 0xAA } else { 0x00 });
    picture(&mut f, PIC_MARKER2, half, half);
    picture(&mut f, PIC_MARKER3, [0xAA; 16], [0; 16]);
    // Room floors show master thing 127's picture: make it the blank one.
    let floor = l.things + acs_core::layout::things::PROPS + 126 * 4 + 1;
    f.data[floor] = (f.data[floor] & 0xC0) | PIC_FLOOR as u8;
    // Creature pictures 64..109: solid colour 3. Residents get 65.. with
    // their own box shapes (`RESIDENT_BOXES`), so the observer can tell
    // them from the players (64, 16 x 16) and from each other.
    for n in 64..110 {
        picture(&mut f, n, [0xFF; 16], [0xFF; 16]);
    }
    for (i, &(w, h)) in RESIDENT_BOXES.iter().enumerate() {
        // A byte is 4 double-width pixels: 8 screen pixels of one half,
        // the first in the top two bits. Colour 3 is bit pair 11.
        let (a, b) = (RESIDENT_INSET, RESIDENT_INSET + w);
        let byte = |half: usize| -> u8 {
            (0..4)
                .filter(|k| (a..b).contains(&(half * 8 + k * 2)))
                .fold(0, |m, k| m | 0b11 << (6 - 2 * k))
        };
        let on = |r: usize| (RESIDENT_INSET..RESIDENT_INSET + h).contains(&r);
        let rows: [u8; 16] = std::array::from_fn(|r| if on(r) { byte(0) } else { 0 });
        let right: [u8; 16] = std::array::from_fn(|r| if on(r) { byte(1) } else { 0 });
        picture(&mut f, 65 + i, rows, right);
    }

    let w = l.world_map;
    // Terrain 0: open field, blank picture. Terrain 1: impassable wall.
    f.put(w + wm::TERRAIN, &[0x40 | PIC_FLOOR as u8, 0]);
    f.put(w + wm::TERRAIN + 2, &[PIC_WALL as u8, 0]);
    f.put(w + wm::TERRAIN + 4, &[0x40 | PIC_MARKER as u8, 0]);
    f.put(w + wm::TERRAIN + 6, &[0x40 | PIC_MARKER2 as u8, 0]);
    f.put(w + wm::TERRAIN + 8, &[0x40 | PIC_MARKER3 as u8, 0]);
    // Terrain 12 (the first of the four door terrains), open.
    f.put(w + wm::TERRAIN + 24, &[0x40 | PIC_DOOR as u8, 0]);
    f.ascii(
        w + wm::TERRAIN_NAMES + 12 * wm::TERRAIN_NAME_LEN,
        "DOOR",
        wm::TERRAIN_NAME_LEN,
    );
    // Door 0 at s.door, leading to region 1, room 0, s.room_entry.
    f.put(w + wm::POINTS, &[s.door.0, s.door.1]);
    f.put(
        w + wm::DOOR_DESTINATIONS,
        &[1 << 4, s.room_entry.1 | s.room_entry.0 << 4],
    );
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
            } else if (xu, yu) == s.marker2 {
                3
            } else if (xu, yu) == s.marker3 {
                4
            } else if (xu, yu) == s.door {
                12
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
        w + wm::TERRAIN_NAMES + 3 * wm::TERRAIN_NAME_LEN,
        "MARKER 2",
        wm::TERRAIN_NAME_LEN,
    );
    f.ascii(
        w + wm::TERRAIN_NAMES + 4 * wm::TERRAIN_NAME_LEN,
        "MARKER 3",
        wm::TERRAIN_NAME_LEN,
    );
    for (i, sp) in s.spells.iter().enumerate() {
        // Type 6, cost, action, parameter.
        let n = SPELL as usize - i;
        f.thing(
            n,
            &format!("TESTSPELL{}", i + 1),
            [0x06, sp.cost, sp.action, sp.param],
        );
        no_messages(&mut f, n);
    }
    f.long_message(1, SPELL_MESSAGE);
    if let Some((action, param)) = s.item {
        // Type 2, weight 1, value 1, extended record 0 of the magic part
        // (the template's offset kept): action, acts when used (bit 7);
        // the door picture; the parameter.
        let off = f.data[l.things + acs_core::layout::things::MAGIC_ITEM_OFFSET];
        f.thing(ITEM as usize, "TESTITEM", [0x02, 1, 1, 0]);
        no_messages(&mut f, ITEM as usize);
        f.extended(Some(off), 0, [0x80 | (action & 0x0F), PIC_DOOR as u8, param]);
        f.long_message(2, ITEM_MESSAGE);
    }
    if let Some(p) = &s.player {
        let c = l.creatures;
        f.packed(c + cr::RETIRED_NAMES, &p.name);
        let mut r = p.record(s.start);
        // Possessions from byte 0x17: things 33-80 one bit each, eight to a
        // byte, high bit first (as the decoder reads them).
        let owned = (0..s.spells.len())
            .map(|i| SPELL as usize - i)
            .chain(s.item.map(|_| ITEM as usize))
            .chain(s.lock.filter(|k| k.key_owned).map(|_| KEY as usize));
        for n in owned {
            let i = n - 33;
            r[0x17 + 8 + i / 8] |= 0x80 >> (i % 8);
        }
        f.put(c + cr::RETIRED_RECORDS, &r);
    }
    if let Some(p) = &s.player2 {
        // The second slot, as the decoder reads it: the next name and the
        // next record. Owns nothing.
        let c = l.creatures;
        f.packed(c + cr::RETIRED_NAMES + 10, &p.name);
        f.put(c + cr::RETIRED_RECORDS + cr::RECORD_LEN, &p.record(s.start));
    }
    // The door table: entry 0 unused (as C64 adventures leave it), 1 the
    // exit to map point 32, 2 the locked door's square in this room.
    // 3 the side room's square.
    let mut portals = Vec::new();
    if s.exit.is_some() || s.lock.is_some() || s.side.is_some() {
        portals.push([0, 0]);
        portals.push([0, 0x20]);
        portals.push(s.lock.map_or([0, 0], |k| [1 << 4, k.to.0 << 4 | k.to.1]));
    }
    if let Some((_, to)) = s.side {
        portals.push([1 << 4 | 1, to.0 << 4 | to.1]);
        f.thing(SIDE as usize, "SIDEDOOR", [0x07, PIC_DOOR as u8, 0, 0]);
        no_messages(&mut f, SIDE as usize);
    }
    if let Some((_, cell)) = s.exit {
        f.put(w + wm::POINTS + 32 * 2, &[cell.0, cell.1]);
        // Type 7, one-way (bit 5 clear); the door picture, open to anyone.
        f.thing(EXIT as usize, "EXIT", [0x07, PIC_DOOR as u8, 0, 0]);
        no_messages(&mut f, EXIT as usize);
    }
    if let Some(k) = s.lock {
        f.thing(KEY as usize, "KEY", [0x01, 1, 1, PIC_DOOR as u8]);
        // Access 2 (bits 6-7 of byte 1): open only to owners of byte 3's
        // thing; byte 2 bit 7: destroy it on passing.
        let destroy = if k.destroy { 0x80 } else { 0 };
        f.thing(LOCK as usize, "LOCK", [0x07, 0x80 | PIC_DOOR as u8, destroy, KEY]);
        no_messages(&mut f, LOCK as usize);
    }
    f.ascii(
        l.adventure + acs_core::layout::adventure::MAP_NAME,
        "TEST FIELD",
        20,
    );

    // An obstacle: type 10, the wall picture, no spell on a bump.
    if s.obstacle.is_some() {
        f.thing(OBSTACLE as usize, "BLOCK", [0x0A, PIC_WALL as u8, 0, 0]);
    }
    let mut items = s
        .obstacle
        .map(|(x, y)| vec![(x, y, OBSTACLE, 0)])
        .unwrap_or_default();
    // Treasures (type 1), value 1, the door picture.
    for (i, &w) in s.weights.iter().enumerate() {
        let n = FIRST_WEIGHT + i as u8;
        let (b0, b1) = weight_bytes(w).expect("weight");
        f.thing(n as usize, &format!("LOAD{}", i + 1), [b0, b1, 1, PIC_DOOR as u8]);
        items.push((s.room_entry.0 + 1 + i as u8, s.room_entry.1, n, 1));
    }
    for (i, &(x, y)) in s.takes.iter().enumerate() {
        let n = TAKE + i as u8;
        let (b0, b1) = s.take_weights.get(i).copied().unwrap_or((0x01, 1));
        f.thing(
            n as usize,
            &format!("THING{}", i + 1),
            [b0, b1, 1, PIC_DOOR as u8],
        );
        if s.quiet_takes {
            no_messages(&mut f, n as usize);
        }
        items.push((x, y, n, 1));
    }
    let wealthy = s
        .residents
        .iter()
        .chain(s.random_creature.iter())
        .any(|c| c.wealth > 0);
    if !s.gold.is_empty() || wealthy {
        // A treasure of weight 1 and value 1, the door picture; a placed
        // pile's item parameter is its size.
        f.thing(GOLD as usize, "GOLD", [0x01, 1, 1, PIC_DOOR as u8]);
        no_messages(&mut f, GOLD as usize);
    }
    for &(x, y, n) in &s.gold {
        items.push((x, y, GOLD, n));
    }
    // Treasures residents carry: things CARRY.., never placed.
    let mut next = CARRY;
    let mut first_carry = Vec::new();
    for c in s.residents.iter().chain(s.random_creature.iter()) {
        first_carry.push(next);
        for &w in &c.carry {
            assert!(next < TAKE, "at most {} carried treasures", TAKE - CARRY);
            let (b0, b1) = weight_bytes(w).expect("weight");
            f.thing(
                next as usize,
                &format!("BAG{}", next - CARRY + 1),
                [b0, b1, 1, PIC_DOOR as u8],
            );
            no_messages(&mut f, next as usize);
            next += 1;
        }
    }
    if let Some((at, _)) = s.exit {
        items.push((at.0, at.1, EXIT, 1));
    }
    if let Some(k) = s.lock {
        items.push((k.at.0, k.at.1, LOCK, 2));
    }
    if let Some((at, _)) = s.side {
        items.push((at.0, at.1, SIDE, 3));
    }
    // Residents: pictures 65, 66, .. (the players' is 64).
    let residents = s
        .residents
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let name: &'static str = Box::leak(c.stats.name.clone().into_boxed_str());
            (i, name, c.record(65 + i as u8, first_carry[i]))
        })
        .collect();
    // One region with one full-size room behind the map door.
    let mut rooms = vec![RoomSpec {
        name: "TEST ROOM",
        w: 15,
        h: 10,
        wall: PIC_WALL as u8,
        items,
        // Low 4 bits the chance, the next 3 which random creature (0).
        random: s.room_chance & 0x0F,
        ..RoomSpec::default()
    }];
    let random_creatures = s
        .random_creature
        .iter()
        .map(|c| {
            let name: &'static str = Box::leak(c.stats.name.clone().into_boxed_str());
            (0, name, c.record(70, first_carry[s.residents.len()]))
        })
        .collect();
    if s.side.is_some() {
        rooms.push(RoomSpec {
            name: "SIDE ROOM",
            w: 10,
            h: 8,
            wall: PIC_WALL as u8,
            random: s.side_chance & 0x0F,
            ..RoomSpec::default()
        });
    }
    f.region(RegionSpec {
        name: "TEST REGION",
        rooms,
        portals,
        residents,
        random_creatures,
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
        assert_eq!(m.doors.len(), 1);
        let room = &adv.regions[0].rooms[0];
        assert_eq!((room.width, room.height), (15, 10));
        assert_eq!(adv.pictures[70].pixel(3, 9), 3);
        assert!(adv.pictures[0].is_blank());
        assert_eq!(m.tiles[14][27], 3);
        assert_eq!(m.tiles[14][9], 4);
        assert!(adv.retired_players.is_empty());
    }

    #[test]
    fn retired_player_decodes_with_its_stats() {
        let s = MoveBasic {
            player: Some(Player::default().with("spd=15,str=20,pow=40").unwrap()),
            ..MoveBasic::default()
        };
        let adv = acs_core::decode(&move_basic(&s, None))
            .expect("decodes")
            .adventure;
        let [p] = adv.retired_players.as_slice() else {
            panic!("one retired player");
        };
        assert_eq!(p.name, "T");
        assert_eq!((p.speed, p.strength, p.power, p.constitution), (15, 20, 40, 11));
        assert_eq!((p.x, p.y, p.region), (12, 12, 0));
        assert_eq!(p.picture, Some(64));
        assert!(Player::default().with("spd=16").is_err());
    }

    #[test]
    fn obstacle_is_placed_in_the_room() {
        let s = MoveBasic {
            obstacle: Some((5, 4)),
            ..MoveBasic::default()
        };
        let adv = acs_core::decode(&move_basic(&s, None))
            .expect("decodes")
            .adventure;
        let room = &adv.regions[0].rooms[0];
        assert_eq!(room.items.len(), 1);
        let it = &room.items[0];
        assert_eq!((it.x, it.y, it.thing), (5, 4, OBSTACLE as u16));
        let t = &adv.things[OBSTACLE as usize - 1];
        assert!(matches!(t.kind, acs_core::model::ThingKind::Obstacle { .. }));
    }

    #[test]
    fn weight_row_decodes_with_its_weights() {
        let s = MoveBasic {
            weights: vec![400, 100, 1],
            ..MoveBasic::default()
        };
        let adv = acs_core::decode(&move_basic(&s, None))
            .expect("decodes")
            .adventure;
        let room = &adv.regions[0].rooms[0];
        let got: Vec<(u8, i32)> = room
            .items
            .iter()
            .map(|it| match &adv.things[it.thing as usize - 1].kind {
                acs_core::model::ThingKind::Treasure { carry, .. } => (it.x, carry.weight),
                k => panic!("{k:?}"),
            })
            .collect();
        assert_eq!(got, vec![(5, 400), (6, 100), (7, 1)]);
        assert!(weight_bytes(401).is_err());
    }

    #[test]
    fn residents_decode_where_and_as_written() {
        use acs_core::model::Alignment;
        let s = MoveBasic {
            residents: vec![
                Resident::parse("name=A,x=8,y=2").unwrap(),
                Resident::parse("name=BB,x=2,y=7,spd=5,align=enemy,aggressive=1,brave=1").unwrap(),
            ],
            ..MoveBasic::default()
        };
        let adv = acs_core::decode(&move_basic(&s, None))
            .expect("decodes")
            .adventure;
        let live: Vec<_> = adv.regions[0].residents.iter().filter(|c| c.in_use).collect();
        let [a, b] = live.as_slice() else {
            panic!("two residents, got {}", live.len());
        };
        assert_eq!((a.name.as_str(), a.x, a.y, a.room, a.region), ("A", 8, 2, 0, 1));
        assert_eq!(
            (a.speed, a.strategy.alignment, a.strategy.aggressive),
            (3, Alignment::Friend, false)
        );
        assert_eq!((b.name.as_str(), b.x, b.y, b.speed), ("BB", 2, 7, 5));
        assert_eq!(b.strategy.alignment, Alignment::Enemy);
        assert!(b.strategy.aggressive && b.strategy.brave);
        assert_eq!((a.constitution, a.life_force), (10, 10));
        assert!(Resident::parse("align=pirate").is_err());
        // Resident 0's picture is 4 x 12 screen pixels from 2,2, resident
        // 1's 12 x 4 (pixels are double width: 8 a row).
        let pa = &adv.pictures[65];
        assert_eq!(
            (pa.pixel(0, 5), pa.pixel(1, 5), pa.pixel(2, 5), pa.pixel(3, 5)),
            (0, 3, 3, 0)
        );
        assert_eq!(
            (pa.pixel(1, 1), pa.pixel(1, 2), pa.pixel(1, 13), pa.pixel(1, 14)),
            (0, 3, 3, 0)
        );
        let pb = &adv.pictures[66];
        assert_eq!(
            (pb.pixel(1, 3), pb.pixel(6, 3), pb.pixel(7, 3), pb.pixel(6, 6)),
            (3, 3, 0, 0)
        );
    }

    #[test]
    fn residents_carry_gold_and_treasures_and_gold_lies_in_the_room() {
        let s = MoveBasic {
            residents: vec![
                Resident::parse("name=A,gold=300,carry=400+1").unwrap(),
                Resident::parse("name=B,room=1,x=3,y=3,carry=20").unwrap(),
            ],
            side: Some(((7, 4), (2, 4))),
            gold: vec![(2, 7, 10)],
            takes: vec![(12, 2)],
            ..MoveBasic::default()
        };
        let adv = acs_core::decode(&move_basic(&s, None))
            .expect("decodes")
            .adventure;
        let live: Vec<_> = adv.regions[0].residents.iter().filter(|c| c.in_use).collect();
        let [a, b] = live.as_slice() else {
            panic!("two residents, got {}", live.len());
        };
        assert_eq!(a.wealth, 300);
        let owned =
            |c: &acs_core::model::Creature| -> Vec<u16> { c.possessions.iter().map(|p| p.thing).collect() };
        assert_eq!(owned(a), vec![50, 51]);
        assert_eq!((b.room, owned(b)), (1, vec![52]));
        assert_eq!(adv.things[49].name, "BAG1");
        assert_eq!(adv.things[80].name, "GOLD");
        let items = &adv.regions[0].rooms[0].items;
        assert!(items.iter().any(|i| (i.x, i.y, i.thing) == (2, 7, 81)));
        assert!(items
            .iter()
            .any(|i| (i.x, i.y, i.thing) == (12, 2, u16::from(TAKE))));
    }

    #[test]
    fn random_creature_and_room_chance() {
        let s = MoveBasic {
            random_creature: Some(Resident::parse("name=RR,spd=2").unwrap()),
            room_chance: 15,
            ..MoveBasic::default()
        };
        let adv = acs_core::decode(&move_basic(&s, None))
            .expect("decodes")
            .adventure;
        let r = &adv.regions[0];
        assert_eq!((r.rooms[0].random_chance, r.rooms[0].random_creature), (75, 0));
        let c = &r.random_creatures[0];
        assert!(c.in_use);
        assert_eq!((c.name.as_str(), c.speed), ("RR", 2));
    }

    #[test]
    fn side_room_and_its_portal() {
        use acs_core::model::ThingKind;
        let s = MoveBasic {
            side: Some(((7, 4), (2, 4))),
            ..MoveBasic::default()
        };
        let adv = acs_core::decode(&move_basic(&s, None))
            .expect("decodes")
            .adventure;
        let r = &adv.regions[0];
        assert!(!r.rooms[1].deleted && r.rooms[2].deleted);
        assert_eq!(
            (r.rooms[1].name.as_str(), r.rooms[1].width, r.rooms[1].height),
            ("SIDE ROOM", 10, 8)
        );
        let door = r.rooms[0]
            .items
            .iter()
            .find(|it| (it.x, it.y) == (7, 4))
            .expect("portal");
        assert_eq!(door.thing, u16::from(SIDE));
        assert!(
            matches!(adv.things[SIDE as usize - 1].kind, ThingKind::Portal { .. }),
            "{:?}",
            adv.things[SIDE as usize - 1].kind
        );
        // Region 1, room 1; square 2,4.
        assert_eq!(r.portal_table[3], [0x11, 0x24]);
    }

    #[test]
    fn loose_has_its_doors_things_and_two_players() {
        use acs_core::model::{Access, ThingKind};
        let mut s = MoveBasic::loose("", true, true).unwrap();
        s.player2 = Some(Player::default().with("name=U,spd=3,wis=9").unwrap());
        let adv = acs_core::decode(&move_basic(&s, None))
            .expect("decodes")
            .adventure;
        let [p, q] = adv.retired_players.as_slice() else {
            panic!("two retired players");
        };
        assert_eq!((p.name.as_str(), p.speed, p.wisdom), ("T", 6, 20));
        assert_eq!((q.name.as_str(), q.speed, q.wisdom), ("U", 3, 9));
        let mut owned: Vec<u16> = p.possessions.iter().map(|o| o.thing).collect();
        owned.sort();
        assert_eq!(owned, vec![KEY as u16, ITEM as u16]);
        assert!(q.possessions.is_empty());
        let room = &adv.regions[0].rooms[0];
        let at = |x, y| {
            room.items
                .iter()
                .find(|it| (it.x, it.y) == (x, y))
                .map(|it| it.thing)
        };
        assert_eq!(at(3, 4), Some(TAKE as u16));
        assert_eq!(at(2, 4), Some(TAKE as u16 + 1));
        assert_eq!(at(4, 5), Some(EXIT as u16));
        assert_eq!(at(4, 3), Some(LOCK as u16));
        match &adv.things[LOCK as usize - 1].kind {
            ThingKind::Portal {
                access,
                destroy_needed_thing,
                two_way,
                ..
            } => {
                assert!(matches!(access, Access::OwnersOf { thing } if *thing == KEY as u16));
                assert!(*destroy_needed_thing);
                assert!(!two_way);
            }
            k => panic!("{k:?}"),
        }
        let dest = |x, y| {
            room.items
                .iter()
                .find(|it| (it.x, it.y) == (x, y))
                .and_then(|it| it.destination)
        };
        use acs_core::model::Destination as D;
        assert_eq!(dest(4, 5), Some(D::WorldMap { x: 20, y: 12 }));
        assert!(matches!(
            dest(4, 3),
            Some(D::Room {
                region: 1,
                room: 0,
                x: 6,
                y: 7
            })
        ));
    }

    #[test]
    fn take_weights_decode() {
        let mut s = MoveBasic::loose("", false, false).unwrap();
        s.take_weights = vec![take_weight("x20:240").unwrap(), take_weight("240").unwrap()];
        let adv = acs_core::decode(&move_basic(&s, None))
            .expect("decodes")
            .adventure;
        for (n, raw) in [(TAKE, (0x41, 12)), (TAKE + 1, (0x01, 240))] {
            match &adv.things[n as usize - 1].kind {
                acs_core::model::ThingKind::Treasure { carry, .. } => assert_eq!(carry.weight, 240),
                k => panic!("{k:?}"),
            }
            assert_eq!(s.take_weights[(n - TAKE) as usize], raw);
        }
        assert!(take_weight("x20:250").is_err());
    }

    #[test]
    fn the_player_owns_the_spell() {
        let s = MoveBasic {
            player: Some(Player::default()),
            spells: vec![Spell::parse("20").unwrap(), Spell::parse("0:7:0x8A").unwrap()],
            item: Some((LONG_MESSAGE, 2)),
            ..MoveBasic::default()
        };
        let adv = acs_core::decode(&move_basic(&s, None))
            .expect("decodes")
            .adventure;
        let p = &adv.retired_players[0];
        let mut owned: Vec<u16> = p.possessions.iter().map(|q| q.thing).collect();
        owned.sort();
        assert_eq!(owned, vec![ITEM as u16, SPELL as u16 - 1, SPELL as u16]);
        match &adv.things[SPELL as usize - 2].kind {
            acs_core::model::ThingKind::MagicSpell { power, action, .. } => {
                assert_eq!(*power, 0);
                assert_eq!(action.kind, acs_core::model::SpellActionKind::ChangePower);
                assert_eq!(acs_engine::magic::amount(action.parameter), (-10, false));
            }
            k => panic!("{k:?}"),
        }
        match &adv.things[ITEM as usize - 1].kind {
            acs_core::model::ThingKind::MagicItem { action, on_use, .. } => {
                assert!(*on_use);
                assert_eq!(action.kind, acs_core::model::SpellActionKind::LongMessage);
                assert_eq!(action.parameter, 2);
            }
            k => panic!("{k:?}"),
        }
        match &adv.things[SPELL as usize - 1].kind {
            acs_core::model::ThingKind::MagicSpell { power, action, .. } => {
                assert_eq!(*power, 20);
                assert_eq!(action.kind, acs_core::model::SpellActionKind::LongMessage);
            }
            k => panic!("{k:?}"),
        }
    }
}
