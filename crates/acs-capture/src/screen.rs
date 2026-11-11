// SPDX-License-Identifier: GPL-3.0-or-later

//! Frames from the emulator: PNG output and simple pixel queries.

use crate::monitor::Display;
use std::path::Path;

/// A frame plus the palette to show it with.
pub struct Frame {
    pub display: Display,
    pub palette: Vec<[u8; 3]>,
}

impl Frame {
    /// The inner (non-border) area as (x, y, w, h).
    pub fn inner(&self) -> (usize, usize, usize, usize) {
        self.display.inner
    }

    /// Palette index at a point of the inner area.
    pub fn at(&self, x: usize, y: usize) -> u8 {
        let (ox, oy, _, _) = self.display.inner;
        self.display.pixels[(oy + y) * self.display.width + ox + x]
    }

    /// Writes the inner area (or the whole buffer) as an RGB PNG, scaled.
    pub fn save_png(&self, path: &Path, whole: bool, scale: usize) -> std::io::Result<()> {
        let (x0, y0, w, h) = if whole {
            (0, 0, self.display.width, self.display.height)
        } else {
            self.display.inner
        };
        let s = scale.max(1);
        let mut rgb = Vec::with_capacity(w * h * 3 * s * s);
        for y in 0..h * s {
            for x in 0..w * s {
                let i = self.display.pixels[(y0 + y / s) * self.display.width + x0 + x / s];
                let c = self.palette.get(i as usize).copied().unwrap_or([255, 0, 255]);
                rgb.extend_from_slice(&c);
            }
        }
        let file = std::fs::File::create(path)?;
        let mut enc = png::Encoder::new(std::io::BufWriter::new(file), (w * s) as u32, (h * s) as u32);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        let mut wr = enc.write_header().map_err(std::io::Error::other)?;
        wr.write_image_data(&rgb).map_err(std::io::Error::other)?;
        Ok(())
    }

    /// A short hash of the inner area, to tell whether the screen changed.
    pub fn hash(&self) -> u64 {
        let (x0, y0, w, h) = self.display.inner;
        let mut hsh: u64 = 0xcbf2_9ce4_8422_2325;
        for y in y0..y0 + h {
            for &p in &self.display.pixels[y * self.display.width + x0..y * self.display.width + x0 + w] {
                hsh ^= p as u64;
                hsh = hsh.wrapping_mul(0x0100_0000_01b3);
            }
        }
        hsh
    }
}
