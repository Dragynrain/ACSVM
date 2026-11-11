// SPDX-License-Identifier: GPL-3.0-or-later

//! Picture textures and the tile grid.
//!
//! Every picture becomes one texture at its display size (16 x 16; C64
//! pixels are already doubled in width by `Picture::to_rgba`) with nearest
//! filtering. The tile size is recomputed from the available area every
//! frame, in physical pixels, so resizing, fullscreen and high-DPI screens
//! all come from one calculation: whole multiples of the original pixels
//! whenever the grid fits at 1x or more, a fractional fit only below that.

use acs_core::Adventure;
use egui::{Color32, ColorImage, Context, Pos2, Rect, TextureHandle, TextureOptions, Vec2};

pub struct Tiles {
    textures: Vec<TextureHandle>,
    /// Display size of one picture in original pixels (16 x 16).
    pub size: [u32; 2],
    /// Palette colour 0, the background of every picture.
    pub background: Color32,
}

impl Tiles {
    pub fn new(ctx: &Context, adv: &Adventure) -> Self {
        let textures: Vec<TextureHandle> = adv
            .pictures
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let (w, h) = p.display_size();
                let image =
                    ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &p.to_rgba(&adv.palette));
                ctx.load_texture(format!("picture-{i}"), image, TextureOptions::NEAREST)
            })
            .collect();
        let size = adv
            .pictures
            .first()
            .map(|p| {
                let (w, h) = p.display_size();
                [w, h]
            })
            .unwrap_or([16, 16]);
        let background = adv
            .palette
            .colors
            .first()
            .map(|c| Color32::from_rgb(c.rgb[0], c.rgb[1], c.rgb[2]))
            .unwrap_or(Color32::BLACK);
        Tiles {
            textures,
            size,
            background,
        }
    }

    pub fn get(&self, picture: u16) -> Option<&TextureHandle> {
        self.textures.get(picture as usize)
    }

    /// Paints `picture` into `rect`, or the background colour if there is none.
    pub fn paint(&self, painter: &egui::Painter, picture: Option<u16>, rect: Rect) {
        match picture.and_then(|p| self.get(p)) {
            Some(t) => {
                painter.image(
                    t.id(),
                    rect,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            None => {
                painter.rect_filled(rect, 0.0, self.background);
            }
        }
    }
}

/// Scale (physical pixels per original pixel) that fits `cells` tiles of
/// `tile_px` original pixels into `avail_px` physical pixels: a whole number
/// when at least 1x fits, otherwise the largest fraction that fits.
pub fn fit_scale(avail_px: [f32; 2], cells: [u32; 2], tile_px: [u32; 2]) -> f32 {
    let need_w = (cells[0].max(1) * tile_px[0].max(1)) as f32;
    let need_h = (cells[1].max(1) * tile_px[1].max(1)) as f32;
    let s = (avail_px[0] / need_w).min(avail_px[1] / need_h);
    if s >= 1.0 {
        s.floor()
    } else {
        s.max(0.05)
    }
}

/// A grid of tiles placed in a rectangle, in egui points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TileGrid {
    pub origin: Pos2,
    pub tile: Vec2,
    pub cols: u32,
    pub rows: u32,
    /// Physical pixels per original pixel.
    pub scale: f32,
}

impl TileGrid {
    /// Fits a `cols` x `rows` grid into `avail`, centred, with its origin on
    /// a physical pixel so integer scales stay crisp.
    pub fn fit(avail: Rect, pixels_per_point: f32, cols: u32, rows: u32, tile_px: [u32; 2]) -> Self {
        let ppp = pixels_per_point.max(0.01);
        let scale = fit_scale([avail.width() * ppp, avail.height() * ppp], [cols, rows], tile_px);
        let tile = Vec2::new(tile_px[0] as f32 * scale / ppp, tile_px[1] as f32 * scale / ppp);
        let size = Vec2::new(tile.x * cols as f32, tile.y * rows as f32);
        let min = avail.center() - size / 2.0;
        let origin = Pos2::new((min.x * ppp).round() / ppp, (min.y * ppp).round() / ppp);
        TileGrid {
            origin,
            tile,
            cols,
            rows,
            scale,
        }
    }

    #[cfg(test)]
    pub fn rect(&self) -> Rect {
        Rect::from_min_size(
            self.origin,
            Vec2::new(self.tile.x * self.cols as f32, self.tile.y * self.rows as f32),
        )
    }

    pub fn cell_rect(&self, x: u32, y: u32) -> Rect {
        Rect::from_min_size(
            self.origin + Vec2::new(self.tile.x * x as f32, self.tile.y * y as f32),
            self.tile,
        )
    }

    pub fn cell_at(&self, pos: Pos2) -> Option<(u32, u32)> {
        let d = pos - self.origin;
        if d.x < 0.0 || d.y < 0.0 {
            return None;
        }
        let (x, y) = ((d.x / self.tile.x) as u32, (d.y / self.tile.y) as u32);
        (x < self.cols && y < self.rows).then_some((x, y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_scale_when_it_fits() {
        // 15 x 10 room of 16 px tiles in 1000 x 700 physical pixels: 4x.
        assert_eq!(fit_scale([1000.0, 700.0], [15, 10], [16, 16]), 4.0);
        // Exactly 1x.
        assert_eq!(fit_scale([640.0, 640.0], [40, 40], [16, 16]), 1.0);
        // Height-bound.
        assert_eq!(fit_scale([5000.0, 330.0], [10, 10], [16, 16]), 2.0);
    }

    #[test]
    fn fractional_below_one() {
        let s = fit_scale([320.0, 320.0], [40, 40], [16, 16]);
        assert!((s - 0.5).abs() < 1e-6);
    }

    #[test]
    fn high_dpi_counts_physical_pixels() {
        // 500 x 500 points at 2x DPI is 1000 physical pixels: 6x for 10 tiles.
        let avail = Rect::from_min_size(Pos2::ZERO, Vec2::splat(500.0));
        let g = TileGrid::fit(avail, 2.0, 10, 10, [16, 16]);
        assert_eq!(g.scale, 6.0);
        assert_eq!(g.tile, Vec2::splat(48.0)); // 96 physical pixels
                                               // Centred, on a physical pixel.
        assert_eq!(g.origin, Pos2::new(10.0, 10.0));
    }

    #[test]
    fn cells_round_trip() {
        let avail = Rect::from_min_size(Pos2::new(3.0, 7.0), Vec2::new(321.0, 161.0));
        let g = TileGrid::fit(avail, 1.0, 20, 10, [16, 16]);
        assert_eq!(g.scale, 1.0);
        for (x, y) in [(0, 0), (19, 9), (7, 3)] {
            assert_eq!(g.cell_at(g.cell_rect(x, y).center()), Some((x, y)));
        }
        assert_eq!(g.cell_at(g.rect().max + Vec2::splat(1.0)), None);
        assert_eq!(g.cell_at(g.origin - Vec2::splat(1.0)), None);
    }
}
