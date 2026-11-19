// SPDX-License-Identifier: GPL-3.0-or-later
//
// Picture layouts follow ACS Viewer's AcsLib GameLoader (Copyright (C) 2016
// Dan Boris, GPL-3.0-or-later). The C64 colour handling was worked out against
// the test corpus. See docs/provenance.md and docs/format-notes.md.

//! Pictures and palettes.
//!
//! A picture is stored as colour indices into the adventure's palette: 2-bit
//! indices into 4 colours on the PC and C64, 5-bit into 32 on the Amiga.
//! PC and Amiga pictures are 16x16 square pixels. C64 pictures are 8x16
//! multicolor pixels, each twice as wide as it is tall, so they cover the same
//! screen area; `pixel_aspect` records that.

use crate::bits::Bytes;
use crate::error::{DiagnosticKind, Diagnostics, Result};
use crate::layout::{self, Layout, PictureFormat};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// One of the colours an adventure uses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaletteColor {
    /// PC and C64: index into the platform's 16-colour master palette.
    /// Amiga: the colour's own number, 0..31 (there is no master palette;
    /// `name` is then its RGB as `#rrggbb`).
    pub code: u8,
    pub name: String,
    pub rgb: [u8; 3],
}

/// The adventure's palette, indexed by a pixel's value: 4 colours on the PC
/// and C64, 32 on the Amiga.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Palette {
    pub colors: Vec<PaletteColor>,
}

/// A 16-colour master palette.
pub struct MasterPalette {
    pub names: [&'static str; 16],
    pub rgb: [[u8; 3]; 16],
}

/// The PC's 16 CGA/EGA text colours.
pub const PC_MASTER: MasterPalette = MasterPalette {
    names: [
        "black",
        "blue",
        "green",
        "cyan",
        "red",
        "magenta",
        "brown",
        "light gray",
        "dark gray",
        "light blue",
        "light green",
        "light cyan",
        "light red",
        "light magenta",
        "yellow",
        "white",
    ],
    rgb: [
        [0, 0, 0],
        [0, 0, 170],
        [0, 170, 0],
        [0, 170, 170],
        [170, 0, 0],
        [170, 0, 170],
        [170, 85, 0],
        [170, 170, 170],
        [85, 85, 85],
        [85, 85, 255],
        [85, 255, 85],
        [85, 255, 255],
        [255, 85, 85],
        [255, 85, 255],
        [255, 255, 85],
        [255, 255, 255],
    ],
};

/// The C64's 16 colours, using Philip "Pepto" Timmermann's measured PAL
/// values (pepto.de/projects/colorvic/2001).
pub const C64_MASTER: MasterPalette = MasterPalette {
    names: [
        "black",
        "white",
        "red",
        "cyan",
        "purple",
        "green",
        "blue",
        "yellow",
        "orange",
        "brown",
        "light red",
        "dark gray",
        "medium gray",
        "light green",
        "light blue",
        "light gray",
    ],
    rgb: [
        [0x00, 0x00, 0x00],
        [0xFF, 0xFF, 0xFF],
        [0x68, 0x37, 0x2B],
        [0x70, 0xA4, 0xB2],
        [0x6F, 0x3D, 0x86],
        [0x58, 0x8D, 0x43],
        [0x35, 0x28, 0x79],
        [0xB8, 0xC7, 0x6F],
        [0x6F, 0x4F, 0x25],
        [0x43, 0x39, 0x00],
        [0x9A, 0x67, 0x59],
        [0x44, 0x44, 0x44],
        [0x6C, 0x6C, 0x6C],
        [0x9A, 0xD2, 0x84],
        [0x6C, 0x5E, 0xB5],
        [0x95, 0x95, 0x95],
    ],
};

impl MasterPalette {
    pub fn color(&self, code: u8) -> PaletteColor {
        let c = (code & 0x0F) as usize;
        PaletteColor {
            code: c as u8,
            name: self.names[c].to_string(),
            rgb: self.rgb[c],
        }
    }
}

/// A decoded picture: `width * height` colour indices (0..=3, Amiga 0..=31),
/// row-major.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Picture {
    pub width: u8,
    pub height: u8,
    /// Horizontal stretch of each pixel when displayed (C64 = 2).
    pub pixel_aspect: u8,
    /// One string per row, one base-32 digit per pixel (`0`-`9`, `a`-`v`;
    /// PC and C64 pictures use only `0`-`3`).
    #[serde(serialize_with = "ser_rows", deserialize_with = "de_rows", rename = "rows")]
    pub pixels: Vec<u8>,
}

impl Picture {
    pub fn pixel(&self, x: usize, y: usize) -> u8 {
        self.pixels[y * self.width as usize + x]
    }

    /// Display size after applying `pixel_aspect`.
    pub fn display_size(&self) -> (u32, u32) {
        (self.width as u32 * self.pixel_aspect as u32, self.height as u32)
    }

    /// RGBA pixels at display size (C64 pixels doubled horizontally).
    pub fn to_rgba(&self, palette: &Palette) -> Vec<u8> {
        let (w, h) = self.display_size();
        let mut out = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..self.height as usize {
            for x in 0..self.width as usize {
                let rgb = palette
                    .colors
                    .get(self.pixel(x, y) as usize)
                    .map_or([0, 0, 0], |c| c.rgb);
                for _ in 0..self.pixel_aspect {
                    out.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 0xFF]);
                }
            }
        }
        out
    }

    /// True if every pixel has the same colour (typical of unused slots).
    pub fn is_blank(&self) -> bool {
        self.pixels.windows(2).all(|w| w[0] == w[1])
    }
}

fn ser_rows<S: Serializer>(pixels: &[u8], s: S) -> std::result::Result<S::Ok, S::Error> {
    // Width is not available here, so rows are re-derived from the length:
    // pictures are always 16 rows tall.
    let w = pixels.len() / 16;
    let rows: Vec<String> = pixels
        .chunks(w.max(1))
        .map(|r| {
            r.iter()
                .map(|&p| char::from_digit(u32::from(p), 32).unwrap_or('?'))
                .collect()
        })
        .collect();
    rows.serialize(s)
}

fn de_rows<'de, D: Deserializer<'de>>(d: D) -> std::result::Result<Vec<u8>, D::Error> {
    let rows = Vec::<String>::deserialize(d)?;
    let mut out = Vec::new();
    for r in rows {
        for c in r.chars() {
            let v = c
                .to_digit(32)
                .ok_or_else(|| serde::de::Error::custom(format!("bad pixel {c:?}")))?;
            out.push(v as u8);
        }
    }
    Ok(out)
}

pub const PICTURE_HEIGHT: usize = 16;

/// Reads the adventure palette.
pub fn decode_palette(bytes: &Bytes, layout: &Layout, diags: &mut Diagnostics) -> Result<Palette> {
    if layout.picture_format == PictureFormat::AmigaPlanar {
        return decode_amiga_palette(bytes, layout);
    }
    let raw = bytes.block(layout.palette, 4, "palette")?;
    let colors = match layout.picture_format {
        PictureFormat::AmigaPlanar => unreachable!(),
        PictureFormat::Pc => raw.iter().map(|&b| PC_MASTER.color(b & 0x0F)).collect(),
        PictureFormat::C64Multicolor => {
            // Bit pairs 00, 01, 10 come from the three shared background
            // registers (any of 16 colours). Bit pair 11 comes from colour
            // RAM, where bit 3 switches the character to multicolor mode and
            // only bits 0..2 pick the colour, so it is limited to colours 0..7.
            if raw[3] & 0x08 == 0 {
                diags.warn(
                    DiagnosticKind::BadPalette,
                    "palette",
                    format!(
                        "colour 3 byte 0x{:02X} lacks the multicolor flag (bit 3); pictures may be hi-res",
                        raw[3]
                    ),
                );
            }
            vec![
                C64_MASTER.color(raw[0] & 0x0F),
                C64_MASTER.color(raw[1] & 0x0F),
                C64_MASTER.color(raw[2] & 0x0F),
                C64_MASTER.color(raw[3] & 0x07),
            ]
        }
    };
    Ok(Palette { colors })
}

/// The Amiga's 32 colours, each a big-endian `0x0RGB` word (the colour
/// registers' form): 4 bits per channel, scaled to 8 bits as `n * 0x11`.
fn decode_amiga_palette(bytes: &Bytes, layout: &Layout) -> Result<Palette> {
    let raw = bytes.block(layout.palette, layout::AMIGA_COLOURS * 2, "palette")?;
    let colors = raw
        .chunks_exact(2)
        .enumerate()
        .map(|(i, w)| {
            let rgb = [(w[0] & 0x0F) * 0x11, (w[1] >> 4) * 0x11, (w[1] & 0x0F) * 0x11];
            PaletteColor {
                code: i as u8,
                name: format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]),
                rgb,
            }
        })
        .collect();
    Ok(Palette { colors })
}

/// Decodes every picture.
pub fn decode_pictures(bytes: &Bytes, layout: &Layout) -> Result<Vec<Picture>> {
    (0..layout.picture_count)
        .map(|n| decode_picture(bytes, layout, n))
        .collect()
}

/// Decodes picture `n`.
///
/// Both platforms store a picture as 16-byte cells (one per 4-pixel-wide
/// column pair, 8 rows each) at `base + n * 16`, with the bottom 8 rows a
/// further 0x800 bytes on. Each byte holds 4 pixels, most significant pair first.
pub fn decode_picture(bytes: &Bytes, layout: &Layout, n: usize) -> Result<Picture> {
    if layout.picture_format == PictureFormat::AmigaPlanar {
        return decode_amiga_picture(bytes, layout, n);
    }
    let base = layout.pictures + n * 16;
    let (width, aspect, columns): (usize, u8, &[usize]) = match layout.picture_format {
        // PC: columns of 4 pixels at +0, +0x2400, +8, +0x2408.
        PictureFormat::Pc => (16, 1, &[0x0000, 0x2400, 0x0008, 0x2408]),
        // C64: two 8x8 multicolor characters side by side (+0 and +8).
        PictureFormat::C64Multicolor => (8, 2, &[0x0000, 0x0008]),
        PictureFormat::AmigaPlanar => unreachable!(),
    };
    let mut pixels = vec![0u8; width * PICTURE_HEIGHT];
    for half in 0..2 {
        for row in 0..8 {
            let y = half * 8 + row;
            for (col, &off) in columns.iter().enumerate() {
                let b = bytes.u8(base + half * 0x800 + off + row, "picture")?;
                for p in 0..4 {
                    pixels[y * width + col * 4 + p] = (b >> (6 - 2 * p)) & 3;
                }
            }
        }
    }
    Ok(Picture {
        width: width as u8,
        height: PICTURE_HEIGHT as u8,
        pixel_aspect: aspect,
        pixels,
    })
}

/// Decodes Amiga picture `n`: plane `j` gives bit `j` of each pixel; each
/// row is a big-endian 16-bit word, leftmost pixel in the top bit. Pictures
/// 0..55 are the first strip of each plane, 55..110 the second.
fn decode_amiga_picture(bytes: &Bytes, layout: &Layout, n: usize) -> Result<Picture> {
    use layout::{AMIGA_PLANES, AMIGA_PLANE_STRIDE, AMIGA_ROW, AMIGA_STRIP, AMIGA_STRIP2};
    let strip = if n < AMIGA_STRIP {
        n * 2
    } else {
        AMIGA_STRIP2 + (n - AMIGA_STRIP) * 2
    };
    let mut pixels = vec![0u8; 16 * PICTURE_HEIGHT];
    for plane in 0..AMIGA_PLANES {
        for y in 0..PICTURE_HEIGHT {
            let at = layout.pictures + plane * AMIGA_PLANE_STRIDE + strip + y * AMIGA_ROW;
            let w = u16::from_be_bytes([bytes.u8(at, "picture")?, bytes.u8(at + 1, "picture")?]);
            for x in 0..16 {
                if (w >> (15 - x)) & 1 == 1 {
                    pixels[y * 16 + x] |= 1 << plane;
                }
            }
        }
    }
    Ok(Picture {
        width: 16,
        height: PICTURE_HEIGHT as u8,
        pixel_aspect: 1,
        pixels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{AMIGA, C64, PC};

    #[test]
    fn pc_picture_bank_order() {
        let mut img = vec![0u8; 0x2B920];
        let base = PC.pictures + 3 * 16; // picture 3
                                         // Top row: each 4-pixel column gets a distinct pattern.
        img[base] = 0b00_01_10_11; // x 0..3
        img[base + 0x2400] = 0b11_11_11_11; // x 4..7
        img[base + 8] = 0b10_10_10_10; // x 8..11
        img[base + 0x2408] = 0b01_00_01_00; // x 12..15
        img[base + 0x800 + 7] = 0b11_00_00_00; // row 15, x 0
        let pic = decode_picture(&Bytes::new(&img), &PC, 3).unwrap();
        assert_eq!((pic.width, pic.height, pic.pixel_aspect), (16, 16, 1));
        let row0: Vec<u8> = (0..16).map(|x| pic.pixel(x, 0)).collect();
        assert_eq!(row0, [0, 1, 2, 3, 3, 3, 3, 3, 2, 2, 2, 2, 1, 0, 1, 0]);
        assert_eq!(pic.pixel(0, 15), 3);
        assert_eq!(pic.pixel(1, 15), 0);
    }

    #[test]
    fn c64_picture_two_characters() {
        let mut img = vec![0u8; 174_848];
        let base = C64.pictures + 109 * 16; // last picture
        img[base + 2] = 0b11_10_01_00; // row 2, x 0..3
        img[base + 8 + 2] = 0b01_01_01_01; // row 2, x 4..7
        img[base + 0x800 + 8] = 0b10_00_00_10; // row 8, x 4..7
        let pic = decode_picture(&Bytes::new(&img), &C64, 109).unwrap();
        assert_eq!((pic.width, pic.height, pic.pixel_aspect), (8, 16, 2));
        let row2: Vec<u8> = (0..8).map(|x| pic.pixel(x, 2)).collect();
        assert_eq!(row2, [3, 2, 1, 0, 1, 1, 1, 1]);
        assert_eq!(pic.pixel(4, 8), 2);
        assert_eq!(pic.pixel(7, 8), 2);
        assert_eq!(pic.display_size(), (16, 16));
        assert_eq!(pic.to_rgba(&test_palette()).len(), 16 * 16 * 4);
    }

    #[test]
    fn c64_palette_colour3_masks_multicolor_flag() {
        let mut img = vec![0u8; 174_848];
        img[C64.palette..C64.palette + 4].copy_from_slice(&[0x00, 0x06, 0x02, 0x0B]);
        let mut d = Diagnostics::default();
        let pal = decode_palette(&Bytes::new(&img), &C64, &mut d).unwrap();
        let codes: Vec<u8> = pal.colors.iter().map(|c| c.code).collect();
        assert_eq!(codes, [0, 6, 2, 3]);
        assert_eq!(pal.colors[3].name, "cyan");
        assert!(d.0.is_empty());
    }

    #[test]
    fn amiga_picture_planes_and_strips() {
        let mut img = vec![0u8; 0x28300];
        // Picture 1 (first strip), row 0: plane 0 sets x 0, plane 4 sets x 0 and x 15.
        img[AMIGA.pictures + 2] = 0x80;
        img[AMIGA.pictures + 4 * 0xDC0 + 2] = 0x80;
        img[AMIGA.pictures + 4 * 0xDC0 + 3] = 0x01;
        // Picture 56 (second strip, second picture), row 15: plane 1 sets x 8.
        img[AMIGA.pictures + 0xDC0 + 0x6E0 + 2 + 15 * 110 + 1] = 0x80;
        let b = Bytes::new(&img);
        let p1 = decode_picture(&b, &AMIGA, 1).unwrap();
        assert_eq!((p1.width, p1.height, p1.pixel_aspect), (16, 16, 1));
        assert_eq!((p1.pixel(0, 0), p1.pixel(15, 0), p1.pixel(1, 0)), (17, 16, 0));
        let p56 = decode_picture(&b, &AMIGA, 56).unwrap();
        assert_eq!((p56.pixel(8, 15), p56.pixel(7, 15)), (2, 0));
        assert!(decode_picture(&b, &AMIGA, 0).unwrap().is_blank());
    }

    #[test]
    fn amiga_palette_is_0rgb_words() {
        let mut img = vec![0u8; 0x28300];
        img[0..4].copy_from_slice(&[0x0F, 0x80, 0x01, 0x2F]);
        let pal = decode_palette(&Bytes::new(&img), &AMIGA, &mut Diagnostics::default()).unwrap();
        assert_eq!(pal.colors.len(), 32);
        assert_eq!(pal.colors[0].rgb, [0xFF, 0x88, 0x00]);
        assert_eq!(pal.colors[1].rgb, [0x11, 0x22, 0xFF]);
        assert_eq!((pal.colors[1].code, pal.colors[1].name.as_str()), (1, "#1122ff"));
    }

    #[test]
    fn picture_json_rows_base32() {
        let pic = Picture {
            width: 16,
            height: 16,
            pixel_aspect: 1,
            pixels: (0..256).map(|i| (i % 32) as u8).collect(),
        };
        let json = serde_json::to_string(&pic).unwrap();
        assert!(json.contains("\"0123456789abcdef\""));
        assert!(json.contains("\"ghijklmnopqrstuv\""));
        let back: Picture = serde_json::from_str(&json).unwrap();
        assert_eq!(back, pic);
    }

    #[test]
    fn picture_json_rows() {
        let pic = Picture {
            width: 8,
            height: 16,
            pixel_aspect: 2,
            pixels: (0..128).map(|i| (i % 4) as u8).collect(),
        };
        let json = serde_json::to_string(&pic).unwrap();
        assert!(json.contains("\"01230123\""));
        let back: Picture = serde_json::from_str(&json).unwrap();
        assert_eq!(back, pic);
    }

    fn test_palette() -> Palette {
        Palette {
            colors: (0..4).map(|c| PC_MASTER.color(c)).collect(),
        }
    }
}
