// SPDX-License-Identifier: GPL-3.0-or-later

//! HD packs (client 5): replacement art for original pictures, one PNG
//! atlas with its manifest in an iTXt chunk, keyed by a hash of each
//! picture's pixels as shown.

use acs_core::Adventure;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The iTXt keyword and the manifest's `format`.
pub const KEYWORD: &str = "acsvm-hd-pack";
pub const VERSION: u32 = 1;
const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// FNV-1a, 64-bit, over a picture's RGBA bytes at display size (client 5).
pub fn hash(rgba: &[u8]) -> u64 {
    rgba.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// A hash as the manifest writes it: 16 lowercase hex digits.
pub fn hex(h: u64) -> String {
    format!("{h:016x}")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub format: String,
    pub version: u32,
    pub name: String,
    #[serde(default)]
    pub license: String,
    pub tile: u32,
    pub tiles: Vec<Entry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub hash: String,
    pub at: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Why a file was not loaded as a pack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackError {
    /// Not a PNG, or a PNG without the pack's chunk: some other file.
    NotAPack,
    /// A pack that can't be used, and why.
    Broken(String),
}

/// A loaded pack.
pub struct Pack {
    pub name: String,
    pub license: String,
    /// Tile side in pixels.
    pub tile: u32,
    /// Entries in the manifest.
    pub entries: usize,
    places: HashMap<u64, u32>,
    width: u32,
    rgba: Vec<u8>,
}

impl Pack {
    pub fn read(bytes: &[u8]) -> Result<Pack, PackError> {
        if !bytes.starts_with(&PNG_SIGNATURE) {
            return Err(PackError::NotAPack);
        }
        let broken = |e: &dyn std::fmt::Display| PackError::Broken(e.to_string());
        let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        decoder.set_transformations(png::Transformations::normalize_to_color8());
        let mut reader = decoder.read_info().map_err(|e| broken(&e))?;
        let text = reader
            .info()
            .utf8_text
            .iter()
            .find(|c| c.keyword == KEYWORD)
            .map(|c| c.get_text())
            .ok_or(PackError::NotAPack)?
            .map_err(|e| broken(&e))?;
        let m: Manifest = serde_json::from_str(&text).map_err(|e| broken(&format!("manifest: {e}")))?;
        if m.format != KEYWORD || m.version != VERSION {
            return Err(broken(&format!(
                "format {} version {}, not {KEYWORD} {VERSION}",
                m.format, m.version
            )));
        }
        acs_bundle::png_fits(reader.info()).map_err(|e| broken(&e))?;
        let mut buf = vec![0; reader.output_buffer_size()];
        let frame = reader.next_frame(&mut buf).map_err(|e| broken(&e))?;
        let (w, h) = (frame.width, frame.height);
        let rgba: Vec<u8> = match frame.color_type {
            png::ColorType::Rgba => buf[..frame.buffer_size()].to_vec(),
            png::ColorType::Rgb => buf[..frame.buffer_size()]
                .as_chunks::<3>()
                .0
                .iter()
                .flat_map(|c| [c[0], c[1], c[2], 255])
                .collect(),
            png::ColorType::GrayscaleAlpha => buf[..frame.buffer_size()]
                .as_chunks::<2>()
                .0
                .iter()
                .flat_map(|c| [c[0], c[0], c[0], c[1]])
                .collect(),
            png::ColorType::Grayscale => buf[..frame.buffer_size()]
                .iter()
                .flat_map(|&g| [g, g, g, 255])
                .collect(),
            other => return Err(broken(&format!("colour type {other:?}"))),
        };
        if !(16..=512).contains(&m.tile) || w % m.tile != 0 || h % m.tile != 0 {
            return Err(broken(&format!(
                "tile {} doesn't divide the {w} x {h} atlas",
                m.tile
            )));
        }
        let room = (w / m.tile) * (h / m.tile);
        let mut places = HashMap::new();
        for e in &m.tiles {
            let hash = u64::from_str_radix(&e.hash, 16)
                .ok()
                .filter(|_| e.hash.len() == 16)
                .ok_or_else(|| broken(&format!("hash {:?} is not 16 hex digits", e.hash)))?;
            if e.at >= room {
                return Err(broken(&format!("place {} is past the atlas's {room}", e.at)));
            }
            places.entry(hash).or_insert(e.at);
        }
        Ok(Pack {
            name: m.name,
            license: m.license,
            tile: m.tile,
            entries: m.tiles.len(),
            places,
            width: w,
            rgba,
        })
    }

    /// The atlas place for a picture hash.
    pub fn place(&self, hash: u64) -> Option<u32> {
        self.places.get(&hash).copied()
    }

    /// The RGBA pixels of the tile at `at`, `tile` x `tile`.
    pub fn tile_rgba(&self, at: u32) -> Vec<u8> {
        let (t, cols) = (self.tile as usize, (self.width / self.tile) as usize);
        let (cx, cy) = (at as usize % cols, at as usize / cols);
        let stride = self.width as usize * 4;
        (0..t)
            .flat_map(|y| {
                let start = (cy * t + y) * stride + cx * t * 4;
                self.rgba[start..start + t * 4].iter().copied()
            })
            .collect()
    }
}

/// A pack's coverage of an adventure (client 5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Coverage {
    /// Pictures that aren't blank.
    pub counted: usize,
    /// Of those, the ones the pack replaces.
    pub matched: usize,
    /// Pictures of one colour, not counted.
    pub blank: usize,
    /// The counted pictures the pack misses: number and hash.
    pub missing: Vec<(usize, u64)>,
}

impl Coverage {
    pub fn percent(&self) -> f32 {
        if self.counted == 0 {
            100.0
        } else {
            100.0 * self.matched as f32 / self.counted as f32
        }
    }

    /// The missing pictures as manifest entries: the start of a pack.
    pub fn missing_entries(&self) -> String {
        let entries: Vec<Entry> = self
            .missing
            .iter()
            .map(|&(n, h)| Entry {
                hash: hex(h),
                at: 0,
                note: Some(format!("picture {n}")),
            })
            .collect();
        serde_json::to_string_pretty(&entries).unwrap_or_default()
    }
}

/// Each picture's hash, by picture number.
pub fn picture_hashes(adv: &Adventure) -> Vec<u64> {
    adv.pictures
        .iter()
        .map(|p| hash(&p.to_rgba(&adv.palette)))
        .collect()
}

pub fn coverage(pack: &Pack, adv: &Adventure) -> Coverage {
    let mut c = Coverage {
        counted: 0,
        matched: 0,
        blank: 0,
        missing: Vec::new(),
    };
    for (n, (p, h)) in adv.pictures.iter().zip(picture_hashes(adv)).enumerate() {
        if p.pixels.iter().all(|&v| v == p.pixels[0]) {
            c.blank += 1;
            continue;
        }
        c.counted += 1;
        if pack.place(h).is_some() {
            c.matched += 1;
        } else {
            c.missing.push((n, h));
        }
    }
    c
}

/// Writes a pack: `rgba` is the `w` x `h` atlas (tests, and a pack made in
/// code).
#[cfg(test)]
pub fn write(m: &Manifest, w: u32, h: u32, rgba: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut e = png::Encoder::new(&mut out, w, h);
        e.set_color(png::ColorType::Rgba);
        e.set_depth(png::BitDepth::Eight);
        e.add_itxt_chunk(KEYWORD.into(), serde_json::to_string(m).unwrap())
            .unwrap();
        let mut wr = e.write_header().unwrap();
        wr.write_image_data(rgba).unwrap();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixture as common;

    #[test]
    fn hashes_are_fnv1a_64() {
        assert_eq!(hash(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(hash(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(hex(0xab), "00000000000000ab");
    }

    /// A test adventure of our own: three pictures with shapes, one blank.
    fn adventure() -> Adventure {
        let mut f = common::Fixture::new(acs_core::Platform::Pc);
        f.header("PACKED", "", "");
        let mut a = acs_core::decode(&f.build()).expect("decodes").adventure;
        a.pictures.truncate(4);
        assert_eq!(a.pictures.len(), 4, "the fixture has pictures");
        // Four colours told apart, so shapes show in the RGBA.
        for (i, c) in a.palette.colors.iter_mut().enumerate() {
            c.rgb = [60 * i as u8, 0, 255 - 60 * i as u8];
        }
        for (n, p) in a.pictures.iter_mut().enumerate() {
            let len = p.pixels.len();
            p.pixels = (0..len)
                .map(|i| if n == 3 { 0 } else { ((i * (n + 1)) % 4) as u8 })
                .collect();
        }
        a
    }

    fn manifest(tile: u32, tiles: Vec<Entry>) -> Manifest {
        Manifest {
            format: KEYWORD.into(),
            version: VERSION,
            name: "Test".into(),
            license: "CC0".into(),
            tile,
            tiles,
        }
    }

    fn entry(h: u64, at: u32) -> Entry {
        Entry {
            hash: hex(h),
            at,
            note: None,
        }
    }

    /// Client 10's gate: a pack loads and its coverage of a test adventure
    /// of our own is right.
    #[test]
    fn a_pack_loads_and_reports_its_coverage() {
        let a = adventure();
        let h = picture_hashes(&a);
        // A 2 x 1 atlas of 16 px tiles: place 0 red, place 1 green.
        let (w, t) = (32u32, 16u32);
        let rgba: Vec<u8> = (0..t * w)
            .flat_map(|i| {
                if i % w < t {
                    [255, 0, 0, 255]
                } else {
                    [0, 255, 0, 255]
                }
            })
            .collect();
        let bytes = write(
            &manifest(t, vec![entry(h[0], 1), entry(h[2], 0), entry(h[0], 0)]),
            w,
            t,
            &rgba,
        );
        let pack = Pack::read(&bytes).expect("a pack");
        assert_eq!((pack.name.as_str(), pack.tile, pack.entries), ("Test", 16, 3));
        // A later entry for the same hash is ignored.
        assert_eq!(pack.place(h[0]), Some(1));
        assert_eq!(&pack.tile_rgba(1)[..4], [0, 255, 0, 255]);
        assert_eq!(pack.tile_rgba(0).len(), 16 * 16 * 4);
        let c = coverage(&pack, &a);
        assert_eq!((c.counted, c.matched, c.blank), (3, 2, 1));
        assert_eq!(c.missing, [(1, h[1])]);
        assert!((c.percent() - 200.0 / 3.0).abs() < 1e-3);
        let start: Vec<Entry> = serde_json::from_str(&c.missing_entries()).unwrap();
        assert_eq!(start[0].hash, hex(h[1]));
    }

    #[test]
    fn other_files_and_broken_packs_are_told_apart() {
        assert!(matches!(Pack::read(b"not a png"), Err(PackError::NotAPack)));
        // A PNG without the chunk is some other picture.
        let mut plain = Vec::new();
        {
            let mut e = png::Encoder::new(&mut plain, 1, 1);
            e.set_color(png::ColorType::Rgba);
            e.write_header().unwrap().write_image_data(&[0; 4]).unwrap();
        }
        assert!(matches!(Pack::read(&plain), Err(PackError::NotAPack)));
        let rgba = vec![0u8; 16 * 16 * 4];
        let broken = |m: Manifest| match Pack::read(&write(&m, 16, 16, &rgba)) {
            Err(PackError::Broken(why)) => why,
            _ => panic!("not refused"),
        };
        assert!(broken(manifest(12, vec![])).contains("tile"));
        assert!(broken(manifest(16, vec![entry(1, 1)])).contains("place"));
        let mut bad = manifest(16, vec![]);
        bad.tiles.push(Entry {
            hash: "xyz".into(),
            at: 0,
            note: None,
        });
        assert!(broken(bad).contains("hash"));
        let mut old = manifest(16, vec![]);
        old.version = 9;
        assert!(broken(old).contains("version"));
    }
}
