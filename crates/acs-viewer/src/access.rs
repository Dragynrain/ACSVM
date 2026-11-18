// SPDX-License-Identifier: GPL-3.0-or-later

//! Accessibility options (client 7): text size, the colour-blind palette,
//! reduced motion; and the marks that keep meaning from resting on colour
//! alone. The options live in egui's context so any view can read them.

use acs_core::model::Alignment;
use egui::{Color32, Context, Painter, Pos2, Stroke};

/// The Okabe-Ito colours, told apart under the common colour-vision
/// deficiencies.
pub mod okabe {
    use egui::Color32;
    pub const ORANGE: Color32 = Color32::from_rgb(230, 159, 0);
    pub const SKY_BLUE: Color32 = Color32::from_rgb(86, 180, 233);
    pub const GREEN: Color32 = Color32::from_rgb(0, 158, 115);
    pub const YELLOW: Color32 = Color32::from_rgb(240, 228, 66);
    pub const VERMILLION: Color32 = Color32::from_rgb(213, 94, 0);
    pub const PURPLE: Color32 = Color32::from_rgb(204, 121, 167);
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Access {
    /// Text size, 0.75..=2.0.
    pub text: f32,
    pub colour_blind: bool,
    pub reduce_motion: bool,
    /// Stores as a plain list instead of the original's store screen.
    pub simple_shops: bool,
}

impl Default for Access {
    fn default() -> Self {
        Access {
            text: 1.0,
            colour_blind: false,
            reduce_motion: false,
            simple_shops: false,
        }
    }
}

fn id() -> egui::Id {
    egui::Id::unique("acsvm-access")
}

impl Access {
    pub const TEXT_RANGE: std::ops::RangeInclusive<f32> = 0.75..=2.0;

    /// The options in force this frame.
    pub fn get(ctx: &Context) -> Access {
        ctx.data(|d| d.get_temp(id())).unwrap_or_default()
    }

    /// Puts the options in force; text size changes the fonts at once.
    pub fn set(self, ctx: &Context) {
        let old = Access::get(ctx);
        if old.text != self.text || ctx.data(|d| d.get_temp::<Access>(id())).is_none() {
            let scale = self
                .text
                .clamp(*Self::TEXT_RANGE.start(), *Self::TEXT_RANGE.end());
            let base = egui::Style::default().text_styles;
            ctx.all_styles_mut(|s| {
                for (style, font) in s.text_styles.iter_mut() {
                    if let Some(b) = base.get(style) {
                        font.size = b.size * scale;
                    }
                }
            });
        }
        ctx.data_mut(|d| d.insert_temp(id(), self));
    }

    /// Player `n`'s marker colour (movement 1.4).
    pub fn player_colour(&self, n: usize) -> Color32 {
        const PLAIN: [Color32; 4] = [
            Color32::from_rgb(255, 215, 0),
            Color32::from_rgb(80, 200, 255),
            Color32::from_rgb(255, 120, 220),
            Color32::from_rgb(140, 240, 110),
        ];
        const SAFE: [Color32; 4] = [okabe::ORANGE, okabe::SKY_BLUE, okabe::YELLOW, okabe::PURPLE];
        (if self.colour_blind { SAFE } else { PLAIN })[n % 4]
    }

    pub fn alignment_colour(&self, a: Alignment) -> Color32 {
        match (a, self.colour_blind) {
            (Alignment::Enemy, false) => Color32::from_rgb(235, 60, 60),
            (Alignment::Friend, false) => Color32::from_rgb(70, 210, 90),
            (Alignment::Neutral, false) => Color32::from_rgb(230, 200, 60),
            (Alignment::Thief, false) => Color32::from_rgb(190, 110, 255),
            (Alignment::Enemy, true) => okabe::VERMILLION,
            (Alignment::Friend, true) => okabe::GREEN,
            (Alignment::Neutral, true) => okabe::YELLOW,
            (Alignment::Thief, true) => okabe::PURPLE,
        }
    }

    /// The world map's door, exit point and start colours.
    pub fn map_colours(&self) -> [Color32; 3] {
        if self.colour_blind {
            [okabe::SKY_BLUE, okabe::ORANGE, okabe::GREEN]
        } else {
            [
                Color32::from_rgb(0, 220, 255),
                Color32::from_rgb(255, 140, 0),
                Color32::from_rgb(80, 255, 80),
            ]
        }
    }
}

/// An alignment's shape, filled at `c` with radius `r` and a dark edge so
/// it reads on any tile (client 7): enemy a triangle, friend a circle,
/// neutral a square, thief a diamond.
pub fn alignment_mark(painter: &Painter, c: Pos2, r: f32, a: Alignment, colour: Color32) {
    let edge = Stroke::new(1.0, Color32::BLACK);
    let poly = |pts: Vec<Pos2>| painter.add(egui::Shape::convex_polygon(pts, colour, edge));
    match a {
        Alignment::Friend => {
            painter.circle(c, r, colour, edge);
        }
        Alignment::Enemy => {
            poly(vec![
                c + egui::vec2(0.0, -r * 1.15),
                c + egui::vec2(r * 1.1, r * 0.8),
                c + egui::vec2(-r * 1.1, r * 0.8),
            ]);
        }
        Alignment::Neutral => {
            painter.rect(
                egui::Rect::from_center_size(c, egui::vec2(r * 1.7, r * 1.7)),
                0.0,
                colour,
                edge,
                egui::StrokeKind::Middle,
            );
        }
        Alignment::Thief => {
            poly(vec![
                c + egui::vec2(0.0, -r * 1.25),
                c + egui::vec2(r * 1.25, 0.0),
                c + egui::vec2(0.0, r * 1.25),
                c + egui::vec2(-r * 1.25, 0.0),
            ]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Client 7: every colour that carries meaning has a different one in
    /// each palette, and the palettes' sets don't repeat a colour.
    #[test]
    fn each_palette_tells_its_marks_apart() {
        for colour_blind in [false, true] {
            let a = Access {
                colour_blind,
                ..Default::default()
            };
            let players: Vec<_> = (0..4).map(|n| a.player_colour(n)).collect();
            let aligns: Vec<_> = [
                Alignment::Enemy,
                Alignment::Friend,
                Alignment::Neutral,
                Alignment::Thief,
            ]
            .into_iter()
            .map(|x| a.alignment_colour(x))
            .collect();
            for set in [players, aligns, a.map_colours().to_vec()] {
                for i in 0..set.len() {
                    for j in i + 1..set.len() {
                        assert_ne!(set[i], set[j], "{colour_blind}");
                    }
                }
            }
        }
        assert_eq!(
            Access::default().player_colour(4),
            Access::default().player_colour(0)
        );
    }

    #[test]
    fn text_size_scales_every_font_and_returns() {
        let ctx = Context::default();
        let body = |ctx: &Context| ctx.global_style().text_styles[&egui::TextStyle::Body].size;
        Access::default().set(&ctx);
        let base = body(&ctx);
        Access {
            text: 1.5,
            ..Default::default()
        }
        .set(&ctx);
        assert!((body(&ctx) - base * 1.5).abs() < 1e-4);
        assert_eq!(Access::get(&ctx).text, 1.5);
        Access::default().set(&ctx);
        assert!((body(&ctx) - base).abs() < 1e-4);
    }
}
