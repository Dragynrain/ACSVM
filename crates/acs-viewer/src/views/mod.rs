// SPDX-License-Identifier: GPL-3.0-or-later

//! The central views and the helpers they share.

pub mod diagnostics;
pub mod lists;
pub mod messages;
pub mod region;
pub mod room;
pub mod walk;
pub mod world;

use crate::app::{Doc, View};
use crate::tiles::TileGrid;
use acs_core::model::{Adventure, Alignment, Creature, Destination, RoomItem};
use egui::{Color32, Painter, Rect, Response, Sense, Stroke, StrokeKind};

/// Something a view asks for, applied after drawing.
pub enum Action {
    Follow(Destination),
    Go(View),
    /// Select a square of the current map or room view (`None` clears).
    Select(Option<(u8, u8)>),
}

pub fn apply(doc: &mut Doc, action: Option<Action>) {
    match action {
        Some(Action::Follow(d)) => doc.follow(d),
        Some(Action::Go(v)) => {
            doc.go(v);
            doc.highlight = None;
        }
        Some(Action::Select(s)) => doc.selected = s,
        None => {}
    }
}

pub fn show(doc: &mut Doc, ui: &mut egui::Ui) {
    match doc.view {
        View::Walk => walk::show(doc, ui),
        View::World => world::show(doc, ui),
        View::Region(r) => region::show(doc, ui, r),
        View::Room(r, i) => room::show(doc, ui, r, i),
        View::Things => lists::things(doc, ui),
        View::Creatures => lists::creatures(doc, ui),
        View::Messages => messages::show(doc, ui),
        View::Diagnostics => diagnostics::show(doc, ui),
    }
}

/// Allocates the whole area for a tile canvas and fits a grid into it.
pub fn canvas(ui: &mut egui::Ui, cols: u32, rows: u32, tile_px: [u32; 2]) -> (Response, Painter, TileGrid) {
    let (resp, painter) = ui.allocate_painter(ui.available_size(), Sense::click());
    let grid = TileGrid::fit(resp.rect, ui.ctx().pixels_per_point(), cols, rows, tile_px);
    (resp, painter, grid)
}

/// Square under the pointer, if it is over the grid.
pub fn hovered_cell(resp: &Response, grid: &TileGrid) -> Option<(u32, u32)> {
    resp.hover_pos().and_then(|p| grid.cell_at(p))
}

pub fn outline(painter: &Painter, rect: Rect, color: Color32, width: f32) {
    painter.rect_stroke(rect, 0.0, Stroke::new(width, color), StrokeKind::Inside);
}

/// A pulsing marker for the square a followed door leads to.
pub fn pulse(ui: &egui::Ui, painter: &Painter, rect: Rect) {
    let t = ui.input(|i| i.time) as f32;
    let a = 0.55 + 0.45 * (t * 5.0).sin();
    let c = Color32::from_rgba_unmultiplied(255, 230, 0, (a * 255.0) as u8);
    painter.rect_stroke(rect.expand(2.0), 0.0, Stroke::new(3.0, c), StrokeKind::Outside);
    ui.ctx().request_repaint();
}

pub fn thing_label(adv: &Adventure, number: u16) -> String {
    match adv.things.get((number as usize).wrapping_sub(1)) {
        Some(t) => format!("#{number} {} ({})", t.name.trim(), t.kind.type_name()),
        None => format!("#{number} (not in the master list)"),
    }
}

pub fn item_label(adv: &Adventure, item: &RoomItem) -> String {
    let mut s = thing_label(adv, item.thing);
    if item.parameter != 0 {
        s += &format!(", parameter {}", item.parameter);
    }
    if item.flag {
        s += ", flag bit set";
    }
    s
}

pub fn alignment(a: Alignment) -> &'static str {
    match a {
        Alignment::Thief => "thief",
        Alignment::Neutral => "neutral",
        Alignment::Enemy => "enemy",
        Alignment::Friend => "friend",
    }
}

pub fn creature_label(c: &Creature) -> String {
    let s = c.strategy;
    format!(
        "{} ({}{}, {})",
        c.name.trim(),
        if s.aggressive { "aggressive " } else { "peaceful " },
        alignment(s.alignment),
        if s.brave { "brave" } else { "cautious" }
    )
}

pub fn destination_label(adv: &Adventure, d: &Destination) -> String {
    match *d {
        Destination::WorldMap { x, y } => format!("world map at {x},{y}"),
        Destination::Room { region, room, x, y } => {
            let reg = adv.regions.get((region as usize).wrapping_sub(1));
            let rname = reg
                .map(|r| r.name.trim().to_string())
                .unwrap_or_else(|| "?".into());
            let room_name = reg
                .and_then(|r| r.rooms.get(room as usize))
                .map(|r| {
                    if r.deleted {
                        "(deleted)".to_string()
                    } else {
                        r.name.trim().to_string()
                    }
                })
                .unwrap_or_else(|| "?".into());
            format!("region {region} {rname}, room {room} {room_name}, at {x},{y}")
        }
    }
}

/// A destination row with a Follow button; returns the destination if clicked.
pub fn follow_row(ui: &mut egui::Ui, adv: &Adventure, d: &Destination) -> Option<Destination> {
    let mut out = None;
    ui.horizontal_wrapped(|ui| {
        if ui.small_button("Follow").clicked() {
            out = Some(*d);
        }
        ui.label(format!("to {}", destination_label(adv, d)));
    });
    out
}

/// Width cap for the info and list panels, so long lines wrap instead of
/// pushing the map aside.
pub const PANEL_WIDTH: std::ops::RangeInclusive<f32> = 200.0..=480.0;

/// A picture about `points` wide, at a whole-number multiple of its pixels.
pub fn picture(ui: &mut egui::Ui, doc: &Doc, picture: Option<u16>, points: f32) -> Response {
    let ppp = ui.ctx().pixels_per_point();
    let scale = (points * ppp / doc.tiles.size[0] as f32).floor().max(1.0);
    let size = egui::vec2(
        doc.tiles.size[0] as f32 * scale / ppp,
        doc.tiles.size[1] as f32 * scale / ppp,
    );
    let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
    doc.tiles.paint(ui.painter(), picture, rect);
    resp
}
