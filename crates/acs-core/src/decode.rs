// SPDX-License-Identifier: GPL-3.0-or-later
//
// Field decoding is ported from ACS Viewer's AcsLib GameLoader (Copyright (C)
// 2016 Dan Boris, GPL-3.0-or-later, https://github.com/danlb2000/ACSViewer),
// with corrections found against the C64 corpus (region heap pointers,
// door destinations, palette flags). See docs/provenance.md and
// docs/format-notes.md.

//! Decodes an [`Image`] into the canonical [`Adventure`] model.

use crate::bits::{bit, field, Bytes};
use crate::disk::Image;
use crate::error::{Diagnostic, DiagnosticKind as K, Diagnostics, Error, Result};
use crate::graphics::{decode_palette, decode_pictures};
use crate::layout::{
    self, adventure as adv, creatures as cr, region as rg, things as th, world_map as wm, Layout, NAME_LEN,
};
use crate::limits::{Limits, Profile};
use crate::model::*;
use crate::text::{decode_ascii, decode_ascii_byte, decode_packed};

/// Gold's thing number (docs/format-notes.md).
const GOLD_THING: u8 = 81;
/// The square byte of the record holding a gold pile's size (x 15, y 14).
const AMOUNT_SQUARE: u8 = 0xFE;

/// A decoded adventure plus everything odd found on the way.
#[derive(Debug, Clone)]
pub struct Decoded {
    pub adventure: Adventure,
    pub diagnostics: Vec<Diagnostic>,
}

impl Decoded {
    pub fn errors(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == crate::error::Severity::Error)
    }
}

/// Decodes an adventure image.
pub fn decode(image: &Image) -> Result<Decoded> {
    let mut d = Decoder {
        b: Bytes::new(&image.data),
        l: Layout::for_platform(image.platform),
        diags: Diagnostics::default(),
    };
    let mut adventure = d.adventure(image)?;
    let mut diagnostics = d.diags.0;
    resolve_portals(&mut adventure, &mut diagnostics);
    Ok(Decoded {
        adventure,
        diagnostics,
    })
}

struct Decoder<'a> {
    b: Bytes<'a>,
    l: &'static Layout,
    diags: Diagnostics,
}

/// Splits fixed-width text into lines, trimming trailing spaces and empty lines.
fn lines(text: &str, width: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out: Vec<String> = chars
        .chunks(width)
        .map(|c| c.iter().collect::<String>().trim_end().to_string())
        .collect();
    while out.last().is_some_and(|l| l.is_empty()) {
        out.pop();
    }
    out.join("\n")
}

impl Decoder<'_> {
    fn ascii(&mut self, addr: usize, len: usize, what: &'static str, location: &str) -> Result<String> {
        let raw = self.b.block(addr, len, what)?;
        if let Some(pos) = raw.iter().position(|&c| decode_ascii_byte(c).is_none()) {
            self.diags.warn(
                K::BadText,
                location,
                format!("byte 0x{:02X} at offset {pos} is not text", raw[pos]),
            );
        }
        Ok(decode_ascii(raw))
    }

    fn packed(&mut self, addr: usize, what: &'static str, location: &str) -> Result<String> {
        let (s, ok) = decode_packed(self.b.block(addr, NAME_LEN, what)?);
        if !ok {
            self.diags
                .warn(K::BadText, location, "packed name ends with an invalid word");
        }
        Ok(s.trim().to_string())
    }

    fn adventure(&mut self, image: &Image) -> Result<Adventure> {
        let l = self.l;
        let a = l.adventure;
        let region_count = self.b.u8(a + adv::REGION_COUNT, "region count")? as usize;
        if region_count > rg::MAX_REGIONS {
            return Err(Error::NotAnAdventure(format!(
                "region count is {region_count} (at most {}); this looks like a program or data disk",
                rg::MAX_REGIONS
            )));
        }

        let name = self.ascii(a + adv::NAME, adv::NAME_LEN, "adventure name", "adventure name")?;
        let byline = self.ascii(a + adv::BYLINE, adv::BYLINE_LEN, "byline", "byline")?;
        let intro = self.ascii(l.intro, layout::INTRO_LEN, "intro", "intro")?;
        let theme = Theme::from_code(self.b.u8(a + adv::THEME, "theme")?);

        let palette = decode_palette(&self.b, l, &mut self.diags)?;
        let pictures = decode_pictures(&self.b, l)?;

        let short_messages = (0..layout::SHORT_MESSAGE_SLOTS)
            .map(|s| {
                let raw = self
                    .b
                    .block(l.short_message(s), layout::SHORT_MESSAGE_LEN, "short message")?;
                Ok(lines(&decode_ascii(raw), layout::SHORT_MESSAGE_LINE))
            })
            .collect::<Result<Vec<_>>>()?;
        let long_messages = (0..layout::LONG_MESSAGE_COUNT)
            .map(|i| {
                let raw = self
                    .b
                    .block(l.long_message(i), layout::LONG_MESSAGE_LEN, "long message")?;
                Ok(lines(&decode_ascii(raw), 32))
            })
            .collect::<Result<Vec<_>>>()?;

        let things = (0..th::COUNT)
            .map(|i| self.thing(i))
            .collect::<Result<Vec<_>>>()?;
        for t in &things {
            for m in [&t.blocked_message, &t.spell_message].into_iter().flatten() {
                if m.slot as usize > layout::SHORT_MESSAGE_SLOTS {
                    self.diags.warn(
                        K::BadReference,
                        format!("thing {}", t.number),
                        format!("short message slot {} is past the last slot", m.slot),
                    );
                } else if m.shown && short_messages[m.slot as usize - 1].contains(char::REPLACEMENT_CHARACTER)
                {
                    self.diags.warn(
                        K::BadText,
                        format!("thing {}", t.number),
                        format!("short message {} contains non-text bytes", m.slot),
                    );
                }
            }
        }

        let creature_classes = (0..cr::CLASS_COUNT)
            .map(|i| {
                let addr = l.creatures + cr::CLASS_NAMES + i * cr::CLASS_NAME_LEN;
                Ok(self
                    .ascii(addr, cr::CLASS_NAME_LEN, "class name", "creature classes")?
                    .trim_end()
                    .to_string())
            })
            .collect::<Result<Vec<_>>>()?;
        let creatures = (0..cr::COUNT)
            .map(|i| {
                let base = l.creatures;
                self.creature(
                    i + 1,
                    base + cr::NAMES + i * NAME_LEN,
                    base + cr::record(i),
                    Some(base + cr::CLASSES + i),
                    &format!("creature {}", i + 1),
                )
            })
            .collect::<Result<Vec<_>>>()?;

        let world = self.world_map()?;
        let regions = (0..region_count)
            .map(|r| self.region(r))
            .collect::<Result<Vec<_>>>()?;
        let (players, retired_players) = self.players()?;

        Ok(Adventure {
            model_version: MODEL_VERSION,
            platform: image.platform,
            profile: Profile::Classic,
            limits: Limits::CLASSIC,
            name: name.trim().to_string(),
            byline: byline.trim().to_string(),
            intro: lines(&intro, 32),
            theme,
            palette,
            pictures,
            things,
            creature_classes,
            creatures,
            players,
            retired_players,
            world,
            regions,
            long_messages,
            short_messages,
        })
    }

    // -----------------------------------------------------------------------
    // Things

    fn action(&mut self, code: u8, parameter: u8, location: &str) -> SpellAction {
        let kind = SpellActionKind::from_code(code);
        if let SpellActionKind::Invalid(c) = kind {
            self.diags
                .warn(K::UnknownSpellAction, location, format!("spell action code {c}"));
        }
        SpellAction {
            kind,
            parameter: parameter.into(),
        }
    }

    fn message(&mut self, number: usize, slot: usize) -> Result<Option<MessageRef>> {
        let addr = self.l.things + th::MESSAGES + (number - 1) + slot * th::MESSAGE_SLOT_STRIDE;
        let b = self.b.u8(addr, "thing message")?;
        let index = b & 0x7F;
        Ok((index != 0).then_some(MessageRef {
            slot: index,
            shown: !bit(b, 7),
        }))
    }

    fn carry(p: &[u8; 4]) -> Carry {
        // Multiplier order per AcsLib; the draft gives two conflicting orders.
        const WEIGHT_MULT: [i32; 4] = [1, -1, 20, -20];
        Carry {
            weight: p[1] as i32 * WEIGHT_MULT[field(p[0], 5, 6) as usize],
            value: p[2] as i32 * if bit(p[0], 7) { 20 } else { 1 },
        }
    }

    fn access(&mut self, p: &[u8; 4], location: &str) -> Access {
        match field(p[1], 6, 7) {
            0 => Access::Anyone,
            1 => {
                if p[2] & 0x70 != 0 {
                    self.diags.warn(
                        K::Other,
                        location,
                        format!("portal spell byte 0x{:02X} has bits 4..6 set", p[2]),
                    );
                }
                Access::InvokeSpell {
                    action: self.action(p[2] & 0x0F, p[3], location),
                }
            }
            2 => Access::OwnersOf { thing: p[3].into() },
            _ => Access::NonOwnersOf { thing: p[3].into() },
        }
    }

    fn extended(&mut self, index: u8, magic: bool) -> Result<[u8; 3]> {
        let t = self.l.things;
        let offset = if magic {
            self.b.u8(t + th::MAGIC_ITEM_OFFSET, "magic item table offset")? as usize
        } else {
            0
        };
        let e = self.b.block(
            t + th::EXTENDED + offset + index as usize * 3,
            3,
            "thing extended properties",
        )?;
        Ok([e[0], e[1], e[2]])
    }

    fn weapon(&mut self, p: &[u8; 4], missile: bool, armor: bool) -> Result<(u8, WeaponStats)> {
        let e = self.extended(p[3], false)?;
        let adj = field(e[1], 4, 7) as i8 * 5;
        let stats = WeaponStats {
            power: if missile {
                field(e[0], 0, 3)
            } else {
                field(e[0], 0, 4)
            },
            range: missile.then(|| field(e[0], 5, 7) + 2),
            break_chance: field(e[1], 0, 3),
            attack_adjustment: if armor { adj - 65 } else { adj - 35 },
            usable_by_anyone: bit(e[2], 6),
            magic: bit(e[2], 7),
        };
        Ok((field(e[2], 0, 5), stats))
    }

    fn thing(&mut self, i: usize) -> Result<Thing> {
        let number = i + 1;
        let loc = format!("thing {number}");
        let t = self.l.things;
        let name = self.packed(t + th::NAMES + i * NAME_LEN, "thing name", &loc)?;
        let raw = self.b.block(t + th::PROPS + i * 4, 4, "thing properties")?;
        let p: [u8; 4] = [raw[0], raw[1], raw[2], raw[3]];
        let acs_may_add = field(self.b.u8(t + th::ACS_ADDS + i, "acs adds")?, 4, 7);
        let disappears = bit(p[0], 4);
        let code = field(p[0], 0, 3);

        // (blocked message slot used, spell message slot used)
        let (mut m0, mut m1) = (false, false);
        let kind = match code {
            0 => ThingKind::RoomFloor {
                picture: (p[1] & 0x3F).into(),
            },
            1 => ThingKind::Treasure {
                picture: (p[3] & 0x3F).into(),
                disappears_when_dropped: disappears,
                carry: Self::carry(&p),
            },
            2 => {
                let e = self.extended(p[3], true)?;
                m1 = true;
                ThingKind::MagicItem {
                    picture: field(e[1], 0, 5).into(),
                    disappears_after_use: disappears,
                    carry: Self::carry(&p),
                    action: self.action(field(e[0], 0, 3), e[2], &loc),
                    on_use: bit(e[0], 7),
                    on_drop: bit(e[1], 6),
                    on_pickup: bit(e[1], 7),
                }
            }
            3..=5 => {
                let (picture, stats) = self.weapon(&p, code == 3, code == 5)?;
                let carry = Self::carry(&p);
                let disappears_after_use = disappears;
                match code {
                    3 => ThingKind::MissileWeapon {
                        picture: picture.into(),
                        disappears_after_use,
                        carry,
                        stats,
                    },
                    4 => ThingKind::MeleeWeapon {
                        picture: picture.into(),
                        disappears_after_use,
                        carry,
                        stats,
                    },
                    _ => ThingKind::Armor {
                        picture: picture.into(),
                        disappears_after_use,
                        carry,
                        stats,
                    },
                }
            }
            6 => {
                m1 = true;
                ThingKind::MagicSpell {
                    disappears_after_use: disappears,
                    power: p[1],
                    action: self.action(field(p[2], 0, 3), p[3], &loc),
                }
            }
            7 => {
                m0 = true;
                ThingKind::Portal {
                    picture: (p[1] & 0x3F).into(),
                    two_way: bit(p[0], 5),
                    access: self.access(&p, &loc),
                    destroy_needed_thing: bit(p[2], 7),
                }
            }
            8 => {
                (m0, m1) = (true, true);
                ThingKind::Space {
                    picture: (p[1] & 0x3F).into(),
                    access: self.access(&p, &loc),
                }
            }
            9 => {
                (m0, m1) = (true, true);
                ThingKind::CustomSpace {
                    picture: (p[1] & 0x3F).into(),
                    placement_choice: if bit(p[0], 7) {
                        PlacementChoice::SpellParameter
                    } else {
                        PlacementChoice::Thing
                    },
                    trigger: match field(p[1], 6, 7) {
                        0 => CustomTrigger::WhenDroppedHere,
                        1 => CustomTrigger::WhenMovedOnto,
                        2 => CustomTrigger::OpenToOwnersOf,
                        _ => CustomTrigger::OpenToNonOwnersOf,
                    },
                    action: self.action(field(p[2], 0, 3), p[3] & 0x7F, &loc),
                    destroy_needed_thing: bit(p[2], 7),
                }
            }
            10 => {
                (m0, m1) = (true, true);
                ThingKind::Obstacle {
                    picture: (p[1] & 0x3F).into(),
                    bump_invokes_spell: bit(p[1], 6),
                    action: self.action(field(p[2], 0, 3), p[3], &loc),
                }
            }
            11 => {
                m1 = true;
                ThingKind::CustomObstacle {
                    picture: (p[1] & 0x3F).into(),
                    action: self.action(field(p[2], 0, 3), p[3], &loc),
                }
            }
            12 => ThingKind::Store {
                picture: (p[1] & 0x3F).into(),
            },
            15 => ThingKind::Undefined,
            c => {
                self.diags.error(
                    K::UnknownThingType,
                    &loc,
                    format!("thing type code {c} ({name:?})"),
                );
                ThingKind::Unknown { code: c }
            }
        };
        Ok(Thing {
            number: number as u16,
            name,
            acs_may_add,
            blocked_message: if m0 { self.message(number, 0)? } else { None },
            spell_message: if m1 { self.message(number, 1)? } else { None },
            kind,
            raw: p,
        })
    }

    // -----------------------------------------------------------------------
    // Creatures

    fn possessions(bytes: &[u8]) -> Vec<Possession> {
        // Things 1..=32: 2-bit counts, four per byte, high bits first.
        // Things 33..=80: 1 bit each, eight per byte, high bit first.
        let mut out = Vec::new();
        for (i, &b) in bytes[..8].iter().enumerate() {
            for k in 0..4 {
                let count = (b >> (6 - 2 * k)) & 3;
                if count > 0 {
                    out.push(Possession {
                        thing: (i * 4 + k + 1) as u16,
                        count,
                    });
                }
            }
        }
        for (i, &b) in bytes[8..14].iter().enumerate() {
            for k in 0..8 {
                if bit(b, 7 - k as u8) {
                    out.push(Possession {
                        thing: (33 + i * 8 + k) as u16,
                        count: 1,
                    });
                }
            }
        }
        out
    }

    fn creature(
        &mut self,
        number: usize,
        name_addr: usize,
        props_addr: usize,
        class_addr: Option<usize>,
        loc: &str,
    ) -> Result<Creature> {
        let name = self.packed(name_addr, "creature name", loc)?;
        let r = self.b.block(props_addr, cr::RECORD_LEN, "creature record")?;
        let class = match class_addr {
            Some(a) => Some(field(self.b.u8(a, "creature class")?, 0, 3)),
            None => None,
        };
        let in_use = r[0x00] != 0 && r[0x16] != 0 && class != Some(8);
        let picture = if in_use {
            if r[0x16] < 2 || (r[0x16] - 2) as usize >= self.l.picture_count {
                self.diags.warn(
                    K::BadReference,
                    loc,
                    format!("picture byte {} is out of range", r[0x16]),
                );
            }
            Some(r[0x16].wrapping_sub(2))
        } else {
            None
        };
        Ok(Creature {
            number: number as u16,
            in_use,
            name,
            class: class.filter(|&c| c < 8),
            picture: picture.map(u16::from),
            constitution: field(r[0x00], 0, 5),
            magic_defense: MagicDefense::from_code(field(r[0x00], 6, 7)),
            strength: field(r[0x01], 0, 4),
            dexterity: field(r[0x02], 0, 4),
            life_force: field(r[0x03], 0, 5),
            speed: field(r[0x04], 0, 3),
            strategy: Strategy {
                alignment: match field(r[0x04], 4, 5) {
                    0 => Alignment::Thief,
                    1 => Alignment::Neutral,
                    2 => Alignment::Enemy,
                    _ => Alignment::Friend,
                },
                aggressive: bit(r[0x04], 6),
                brave: bit(r[0x04], 7),
            },
            power: field(r[0x05], 0, 6),
            dodge: field(r[0x06], 0, 6),
            parry: field(r[0x07], 0, 6),
            armor_skill: field(r[0x08], 0, 6),
            melee: field(r[0x09], 0, 6),
            mimic: bit(r[0x09], 7),
            missile: field(r[0x0A], 0, 6),
            unknown_0b: r[0x0B],
            unknown_0c: r[0x0C],
            x: r[0x0D],
            y: r[0x0E],
            size: field(r[0x0F], 0, 4),
            room: field(r[0x10], 0, 3),
            region: field(r[0x10], 4, 7),
            wisdom: field(r[0x11], 0, 4),
            wealth: u16::from_le_bytes([r[0x12], r[0x13]]),
            weapon: r[0x14].into(),
            armor: r[0x15].into(),
            possessions: Self::possessions(&r[0x17..0x25]),
        })
    }

    fn players(&mut self) -> Result<(Vec<Creature>, Vec<Creature>)> {
        let a = self.l.adventure;
        let mut players = Vec::new();
        for i in 0..4 {
            let c = self.creature(
                i + 1,
                a + adv::PLAYER_NAMES + i * NAME_LEN,
                a + adv::PLAYERS + i * cr::RECORD_LEN,
                None,
                &format!("player {}", i + 1),
            )?;
            if c.in_use {
                players.push(c);
            }
        }
        // Retired players are packed into pages; AcsLib reads 26 names and 7
        // records per page. That overruns a 256-byte page, so we read 25 and 6
        // (unverified: the corpus has no retired players).
        let base = self.l.creatures;
        let mut retired = Vec::new();
        for i in 0..cr::RETIRED_MAX {
            let name = base + cr::RETIRED_NAMES + (i / 25) * 0x100 + (i % 25) * NAME_LEN;
            let rec = base + cr::RETIRED_RECORDS + (i / 6) * 0x100 + (i % 6) * cr::RECORD_LEN;
            if rec + cr::RECORD_LEN > base + cr::CLASSES {
                break;
            }
            let c = self.creature(i + 1, name, rec, None, &format!("retired player {}", i + 1))?;
            if !c.in_use {
                break;
            }
            retired.push(c);
        }
        Ok((players, retired))
    }

    // -----------------------------------------------------------------------
    // World map

    fn world_map(&mut self) -> Result<WorldMap> {
        let w = self.l.world_map;
        let a = self.l.adventure;
        let name = self.ascii(a + adv::MAP_NAME, adv::MAP_NAME_LEN, "map name", "world map name")?;
        let wraps = self.b.u8(a + adv::MAP_WRAP, "map wrap")? != 0;
        let start = MapPoint {
            x: self.b.u8(w + wm::START_X, "map start")?,
            y: self.b.u8(w + wm::START_Y, "map start")?,
        };

        let mut terrain = Vec::with_capacity(16);
        for i in 0..16 {
            let t = self.b.block(w + wm::TERRAIN + i * 2, 2, "terrain")?;
            let (b0, b1) = (t[0], t[1]);
            let tname = self.ascii(
                w + wm::TERRAIN_NAMES + i * wm::TERRAIN_NAME_LEN,
                wm::TERRAIN_NAME_LEN,
                "terrain name",
                &format!("terrain {i}"),
            )?;
            terrain.push(Terrain {
                number: i as u8,
                name: tname.trim().to_string(),
                picture: field(b0, 0, 4).into(),
                passability: match field(b0, 6, 7) {
                    0 => Passability::Impassable,
                    1 => Passability::Open,
                    2 => Passability::OwnersOf,
                    _ => Passability::TriggersSpell,
                },
                parameter: b1.into(),
                unknown_bit5: bit(b0, 5),
            });
        }

        let map = self.b.block(w + wm::MAP, wm::SIZE * wm::SIZE / 2, "world map")?;
        let tiles = map
            .chunks(wm::SIZE / 2)
            .map(|row| row.iter().flat_map(|&b| [b >> 4, b & 0x0F]).collect())
            .collect();

        let pts = self.b.block(w + wm::POINTS, wm::POINT_COUNT * 2, "map points")?;
        let point = |k: usize| (pts[k * 2], pts[k * 2 + 1]);
        let mut doors = Vec::new();
        for k in 0..wm::DOOR_COUNT {
            let (x, y) = point(k);
            if x == 0xFF {
                continue;
            }
            let dd = self
                .b
                .block(w + wm::DOOR_DESTINATIONS + k * 2, 2, "door destination")?;
            let region = field(dd[0], 4, 7);
            let destination = if region == 0 {
                let (dx, dy) = point(wm::DOOR_COUNT + k);
                Destination::WorldMap { x: dx, y: dy & 0x7F }
            } else {
                Destination::Room {
                    region,
                    room: field(dd[0], 0, 3),
                    x: field(dd[1], 4, 7),
                    y: field(dd[1], 0, 3),
                }
            };
            doors.push(MapDoor {
                index: k as u16,
                at: MapPoint { x, y: y & 0x7F },
                destination,
                unknown_y_bit7: bit(y, 7),
            });
        }
        let exit_points = (wm::DOOR_COUNT..wm::POINT_COUNT)
            .map(|k| {
                let (x, y) = point(k);
                (x != 0xFF).then_some(MapPoint { x, y: y & 0x7F })
            })
            .collect();

        let mut random_creatures = Vec::new();
        for i in 0..wm::CREATURE_COUNT {
            let creature = self.creature(
                i + 1,
                w + wm::CREATURE_NAMES + i * NAME_LEN,
                w + wm::CREATURES + i * cr::RECORD_LEN,
                None,
                &format!("world map creature {}", i + 1),
            )?;
            let c = self.b.u8(w + wm::CREATURE_CHANCES + i, "creature chance")?;
            let code = field(c, 0, 3);
            random_creatures.push(WorldCreature {
                creature,
                terrain: field(c, 4, 7),
                chance: if code < 5 { code * 2 } else { (code - 3) * 5 },
            });
        }

        Ok(WorldMap {
            name: name.trim().to_string(),
            wraps,
            start,
            terrain,
            tiles,
            doors,
            exit_points,
            random_creatures,
        })
    }

    // -----------------------------------------------------------------------
    // Regions

    fn region(&mut self, r: usize) -> Result<Region> {
        let number = r + 1;
        let loc = format!("region {number}");
        let base = self.l.regions + r * rg::STRIDE;
        let a = self.l.adventure;
        let name = self.ascii(
            a + adv::REGION_NAMES + r * adv::REGION_NAME_LEN,
            adv::REGION_NAME_LEN,
            "region name",
            &loc,
        )?;
        let heap = base + rg::HEAP;
        let ptr = |s: &Self, off: usize, what| s.b.u16le(base + off, what).map(|v| v as usize);
        let room_count = self.b.u8(base + rg::ROOM_COUNT, "room count")? as usize;
        let items_end = ptr(self, rg::ITEMS_END, "items end")?;
        let header_03 = self.b.u16le(base + rg::HEADER_03, "region header 0x03")?;
        let names_ptr = ptr(self, rg::RESIDENT_NAMES_PTR, "resident name pointer")?;
        let items_ptr = ptr(self, rg::ITEMS_PTR, "item pointer")?;
        if room_count > rg::MAX_ROOMS {
            self.diags
                .error(K::Other, &loc, format!("room count {room_count} exceeds 16"));
        }
        let table_start = rg::DOOR_TABLE_HEAP_OFFSET;
        if !(table_start <= names_ptr
            && (names_ptr - table_start) % 2 == 0
            && names_ptr + 0xA0 <= items_ptr
            && heap + items_ptr <= base + items_end
            && items_end <= rg::STRIDE)
        {
            return Err(Error::NotAnAdventure(format!(
                "{loc}: heap pointers out of order (names {names_ptr:#x}, items {items_ptr:#x}, end {items_end:#x})"
            )));
        }

        let store_raw = self.b.block(base + rg::STORE, 14, "store")?;
        let store = Self::possessions(store_raw);

        // The door table always starts at heap + 0xF0 and ends at the
        // resident names. Header bytes 3-4 usually say 0xF0 too, but not
        // always (see format-notes), so they are not trusted.
        let portal_bytes = self
            .b
            .block(heap + table_start, names_ptr - table_start, "region door table")?;
        let portal_table: Vec<[u8; 2]> = portal_bytes.chunks_exact(2).map(|c| [c[0], c[1]]).collect();

        let random_creatures = (0..rg::RANDOM_COUNT)
            .map(|i| {
                self.creature(
                    i + 1,
                    base + rg::RANDOM_NAMES + i * NAME_LEN,
                    base + rg::RANDOM_CREATURES + i * cr::RECORD_LEN,
                    None,
                    &format!("{loc} random creature {}", i + 1),
                )
            })
            .collect::<Result<Vec<_>>>()?;
        let residents = (0..rg::RESIDENT_COUNT)
            .map(|i| {
                self.creature(
                    i + 1,
                    heap + names_ptr + i * NAME_LEN,
                    base + rg::RESIDENTS + i * cr::RECORD_LEN,
                    None,
                    &format!("{loc} resident {}", i + 1),
                )
            })
            .collect::<Result<Vec<_>>>()?;
        for c in residents.iter().filter(|c| c.in_use) {
            if c.region as usize != number {
                self.diags.warn(
                    K::BadReference,
                    &loc,
                    format!(
                        "resident {} ({}) says it is in region {}",
                        c.number, c.name, c.region
                    ),
                );
            }
        }

        let mut rooms = Vec::with_capacity(rg::MAX_ROOMS);
        let mut cursor = heap + items_ptr;
        let end = base + items_end;
        for i in 0..rg::MAX_ROOMS {
            let room_loc = format!("{loc} room {i}");
            let d = self
                .b
                .block(base + rg::ROOMS + i * rg::ROOM_LEN, rg::ROOM_LEN, "room")?;
            let (w, h) = (field(d[1], 4, 7), field(d[1], 0, 3));
            let room_name = self.packed(base + rg::ROOM_NAMES + i * NAME_LEN, "room name", &room_loc)?;
            let items = self.room_items(&mut cursor, end, &room_loc)?;
            rooms.push(Room {
                number: i as u8,
                name: room_name,
                deleted: w == 0 && h == 0,
                wall_picture: d[0].into(),
                width: w + 1,
                height: h + 1,
                x: d[2],
                y: d[3],
                random_chance: field(d[4], 0, 3) * 5,
                random_creature: field(d[4], 4, 6),
                items,
            });
        }
        if cursor != end {
            self.diags.error(
                K::BadRoomItems,
                &loc,
                format!(
                    "room item lists end at {:#x}, header says {:#x}",
                    cursor - base,
                    items_end
                ),
            );
        }
        // The count byte is the number of live rooms, not a slot limit:
        // deleted rooms keep their slot with a zero size.
        let live = rooms.iter().filter(|r| !r.deleted).count();
        if live != room_count {
            self.diags.warn(
                K::Other,
                &loc,
                format!("room count byte is {room_count} but {live} room slots are in use"),
            );
        }

        Ok(Region {
            number: number as u8,
            name: name.trim().to_string(),
            rooms,
            random_creatures,
            residents,
            store,
            portal_table,
            unknown_header_03: header_03,
            unknown_59: self
                .b
                .block(base + rg::UNKNOWN_59, rg::UNKNOWN_59_LEN, "region bytes 0x59")?
                .to_vec(),
        })
    }

    fn room_items(&mut self, cursor: &mut usize, end: usize, loc: &str) -> Result<Vec<RoomItem>> {
        let len = self.b.u16le(*cursor, "room item list length")? as usize;
        if len < 2 || (len - 2) % 3 != 0 || *cursor + len > end {
            return Err(Error::NotAnAdventure(format!(
                "{loc}: bad room item list length {len}"
            )));
        }
        let data = self.b.block(*cursor + 2, len - 2, "room items")?;
        *cursor += len;
        let mut items: Vec<RoomItem> = Vec::new();
        let records: Vec<&[u8]> = data.chunks_exact(3).collect();
        let mut n = 0;
        while n < records.len() {
            let it = records[n];
            n += 1;
            if it[0] == 0xFF {
                continue; // deleted
            }
            let thing = it[1] & 0x7F;
            let mut item = RoomItem {
                x: field(it[0], 4, 7),
                y: field(it[0], 0, 3),
                thing: thing.into(),
                parameter: it[2],
                flag: bit(it[1], 7),
                destination: None,
                gold: None,
            };
            if thing == 0 {
                self.diags
                    .warn(K::BadReference, loc, format!("item {} has thing number 0", n - 1));
            }
            // A gold pile's size is the next record, on square 15,14 (byte
            // 0xFE): its two other bytes, low first (all 112 corpus piles).
            if thing == GOLD_THING {
                match records.get(n) {
                    Some(next) if next[0] == AMOUNT_SQUARE => {
                        item.gold = Some(u16::from_le_bytes([next[1], next[2]]));
                        n += 1;
                    }
                    _ => self.diags.warn(
                        K::BadReference,
                        loc,
                        format!("gold pile (item {}) has no amount record", n - 1),
                    ),
                }
            } else if it[0] == AMOUNT_SQUARE {
                self.diags.warn(
                    K::BadReference,
                    loc,
                    format!("item {} on square 15,14 follows no gold pile", n - 1),
                );
            }
            // Portal destinations are filled in by `resolve_portals`.
            items.push(item);
        }
        Ok(items)
    }
}

/// Fills in door destinations for portal room items. Needs the thing list,
/// so it runs on a finished [`Adventure`].
pub fn resolve_portals(adv: &mut Adventure, diags: &mut Vec<Diagnostic>) {
    let two_way: Vec<Option<bool>> = adv
        .things
        .iter()
        .map(|t| match t.kind {
            ThingKind::Portal { two_way, .. } => Some(two_way),
            _ => None,
        })
        .collect();
    let world = adv.world.clone();
    for region in &mut adv.regions {
        let table = region.portal_table.clone();
        for room in &mut region.rooms {
            for (n, item) in room.items.iter_mut().enumerate() {
                let Some(Some(_two_way)) = two_way
                    .get(item.thing as usize - 1)
                    .copied()
                    .filter(|_| item.thing > 0)
                else {
                    continue;
                };
                let Some(entry) = table.get(item.parameter as usize) else {
                    diags.push(Diagnostic {
                        severity: crate::error::Severity::Warning,
                        kind: K::BadReference,
                        location: format!("region {} room {} item {n}", region.number, room.number),
                        message: format!(
                            "portal parameter {} is past the {}-entry door table",
                            item.parameter,
                            table.len()
                        ),
                    });
                    continue;
                };
                let dest_region = field(entry[0], 4, 7);
                let x = field(entry[1], 4, 7);
                let y = field(entry[1], 0, 3);
                item.destination = Some(if dest_region == 0 {
                    // A door to the world map: (y | x << 4) indexes the map point table.
                    let p = (y as usize) | (x as usize) << 4;
                    let pt = if p < wm::DOOR_COUNT {
                        world.doors.iter().find(|d| d.index as usize == p).map(|d| d.at)
                    } else {
                        world.exit_points.get(p - wm::DOOR_COUNT).copied().flatten()
                    };
                    match pt {
                        Some(pt) => Destination::WorldMap { x: pt.x, y: pt.y },
                        None => {
                            diags.push(Diagnostic {
                                severity: crate::error::Severity::Warning,
                                kind: K::BadReference,
                                location: format!("region {} room {} item {n}", region.number, room.number),
                                message: format!("door leads to unused world map point {p}"),
                            });
                            continue;
                        }
                    }
                } else {
                    Destination::Room {
                        region: dest_region,
                        room: field(entry[0], 0, 3),
                        x,
                        y,
                    }
                });
            }
        }
    }
}
