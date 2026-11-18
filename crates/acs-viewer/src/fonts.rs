// SPDX-License-Identifier: GPL-3.0-or-later

//! The kit's UI font (docs/client.md 7.1; the glyphs are
//! docs/art/pirates-font.txt, built by scripts/font.py). It is a pixel
//! font: each glyph pixel is a square in the outline, and the em is the
//! 11-row cell, so it draws crisp when its size is a whole multiple of 11
//! physical pixels. `kit` picks that size.

use egui::epaint::text::{FontData, FontDefinitions, FontTweak};
use egui::{Context, FontFamily, FontId};
use std::sync::Arc;

const REGULAR: &str = "kit";
const BOLD: &str = "kit-bold";
/// Rows in a glyph cell: one font pixel is a size's 1/11th.
const CELL: f32 = 11.0;

/// Adds the kit font, regular and bold, as their own families, each
/// falling back to egui's fonts for characters it lacks.
pub fn install(ctx: &Context) {
    let mut defs = FontDefinitions::default();
    let tweak = FontTweak {
        hinting: Some(false),
        subpixel_binning: Some(false),
        ..FontTweak::default()
    };
    for (name, bytes) in [
        (REGULAR, include_bytes!("../assets/font.ttf").as_slice()),
        (BOLD, include_bytes!("../assets/font-bold.ttf").as_slice()),
    ] {
        defs.font_data.insert(
            name.into(),
            Arc::new(FontData::from_static(bytes).tweak(tweak.clone())),
        );
        let mut list = vec![name.to_string()];
        list.extend(
            defs.families
                .get(&FontFamily::Proportional)
                .cloned()
                .unwrap_or_default(),
        );
        defs.families.insert(FontFamily::Name(name.into()), list);
    }
    ctx.set_fonts(defs);
}

/// The kit font at about `points`, times the text-size option, rounded to
/// a whole multiple of the font's pixel on this screen (at least 1x).
pub fn kit(ctx: &Context, points: f32, bold: bool) -> FontId {
    let text = crate::access::Access::get(ctx).text;
    FontId::new(
        size(points * text, ctx.pixels_per_point()),
        FontFamily::Name(if bold { BOLD } else { REGULAR }.into()),
    )
}

/// `points` rounded so that it is a whole multiple of 11 physical pixels
/// at `ppp` pixels a point, at least one.
fn size(points: f32, ppp: f32) -> f32 {
    (points * ppp / CELL).round().max(1.0) * CELL / ppp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_are_whole_font_pixels_on_the_screen() {
        for ppp in [1.0, 1.25, 1.5, 2.0, 2.5] {
            for pt in [4.0, 11.0, 16.0, 22.0, 30.0, 44.0] {
                let px = size(pt, ppp) * ppp;
                assert!(
                    (px / CELL - (px / CELL).round()).abs() < 1e-3,
                    "{pt} pt at {ppp}: {px} px"
                );
                assert!(px >= CELL - 1e-3, "{pt} pt at {ppp}");
                assert!((size(pt, ppp) - pt).abs() <= CELL / 2.0 / ppp + 1e-3 || px <= CELL + 1e-3);
            }
        }
        assert_eq!(size(22.0, 1.0), 22.0);
        assert_eq!(size(22.0, 1.5), 22.0);
    }

    #[test]
    fn the_fonts_load() {
        let ctx = Context::default();
        install(&ctx);
        assert!(kit(&ctx, 22.0, true).size > 0.0);
    }
}
