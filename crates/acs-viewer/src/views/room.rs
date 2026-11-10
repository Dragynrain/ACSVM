// SPDX-License-Identifier: GPL-3.0-or-later

//! One room: walls, floor, the top item of every stack and the residents.

use super::*;
use acs_core::model::{Region, Room};
use acs_core::scene::{self, Base, RoomScene, Square};
use egui::RichText;

pub fn show(doc: &mut Doc, ui: &mut egui::Ui, r: usize, i: usize) {
    let adv = &doc.decoded.adventure;
    let Some(region) = adv.regions.get(r) else { return };
    let Some(room) = region.rooms.get(i) else { return };
    let scene = RoomScene::new(region, room);
    let mut action = None;
    let focus = doc.selected.or(doc.hover);

    let mut show_info = doc.show_info;
    egui::Panel::right("room-info")
        .resizable(true)
        .default_size(300.0)
        .size_range(PANEL_WIDTH)
        .show_collapsible(ui, &mut show_info, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading(room.name.trim());
                ui.label(format!(
                    "Region {} {} · room {} · {}×{} squares · at {},{} on the region grid",
                    r + 1,
                    region.name.trim(),
                    room.number,
                    room.width,
                    room.height,
                    room.x,
                    room.y
                ));
                let random = region
                    .random_creatures
                    .get(room.random_creature as usize)
                    .filter(|c| c.in_use)
                    .map(|c| c.name.trim().to_string())
                    .unwrap_or_else(|| format!("slot {}", room.random_creature + 1));
                ui.label(format!(
                    "Wall picture {} · random creature {}% ({random})",
                    room.wall_picture, room.random_chance
                ));
                if ui.small_button("Show region").clicked() {
                    action = Some(Action::Go(View::Region(r)));
                }
                ui.separator();
                match focus.and_then(|(x, y)| scene.square(x, y).map(|s| (x, y, s))) {
                    Some((x, y, sq)) => {
                        if let Some(d) = square_info(ui, adv, region, room, x, y, sq) {
                            action = Some(Action::Follow(d));
                        }
                    }
                    None => {
                        ui.label(
                            RichText::new("Hover or click a square for details; click a door to follow it.")
                                .weak(),
                        );
                    }
                }
                ui.separator();
                let residents: Vec<_> = scene
                    .squares
                    .iter()
                    .flat_map(|s| s.residents.iter())
                    .map(|&k| &region.residents[k])
                    .collect();
                ui.label(RichText::new(format!("Residents ({})", residents.len())).strong());
                for c in residents {
                    ui.label(format!("{},{}  {}", c.x, c.y, creature_label(c)));
                }
                if !scene.off_room.is_empty() {
                    ui.add_space(4.0);
                    ui.label(RichText::new(format!("Off the room ({})", scene.off_room.len())).strong());
                    for &k in &scene.off_room {
                        let it = &room.items[k];
                        ui.label(format!("{},{}  {}", it.x, it.y, item_label(adv, it)));
                    }
                }
            });
        });

    let mut hover = None;
    egui::CentralPanel::no_frame().show(ui, |ui| {
        let (resp, painter, grid) = canvas(ui, room.width as u32, room.height as u32, doc.tiles.size);
        painter.rect_filled(resp.rect, 0.0, ui.visuals().extreme_bg_color);
        for (k, sq) in scene.squares.iter().enumerate() {
            let (x, y) = ((k % room.width as usize) as u32, (k / room.width as usize) as u32);
            let (_, pic) = scene::shown(adv, region, room, sq);
            doc.tiles.paint(&painter, pic, grid.cell_rect(x, y));
        }
        hover = hovered_cell(&resp, &grid).map(|(x, y)| (x as u8, y as u8));
        if let Some((x, y)) = hover {
            outline(
                &painter,
                grid.cell_rect(x as u32, y as u32),
                Color32::from_white_alpha(160),
                1.5,
            );
            if scene.square(x, y).and_then(|sq| door(room, sq)).is_some() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
        }
        if let Some((x, y)) = doc.selected {
            outline(&painter, grid.cell_rect(x as u32, y as u32), Color32::WHITE, 2.0);
        }
        if let Some((v, x, y)) = doc.highlight {
            if v == View::Room(r, i) && x < room.width && y < room.height {
                pulse(ui, &painter, grid.cell_rect(x as u32, y as u32));
            }
        }
        if resp.clicked() {
            match hover.and_then(|(x, y)| scene.square(x, y).and_then(|sq| door(room, sq))) {
                Some(d) => action = Some(Action::Follow(d)),
                None => action = Some(Action::Select(hover)),
            }
        }
    });
    doc.hover = hover;
    apply(doc, action);
}

/// The destination of the first door in a square's stack.
pub fn door(room: &Room, sq: &Square) -> Option<Destination> {
    sq.items.iter().find_map(|&k| room.items[k].destination)
}

fn square_info(
    ui: &mut egui::Ui,
    adv: &Adventure,
    region: &Region,
    room: &Room,
    x: u8,
    y: u8,
    sq: &Square,
) -> Option<Destination> {
    let mut follow = None;
    let base = match sq.base {
        Base::Wall => "wall",
        Base::Floor => "floor",
    };
    ui.label(RichText::new(format!("Square {x},{y} ({base})")).strong());
    for &k in &sq.residents {
        ui.label(format!("creature: {}", creature_label(&region.residents[k])));
    }
    if sq.items.is_empty() {
        ui.label(RichText::new("No things").weak());
    }
    for (n, &k) in sq.items.iter().enumerate() {
        let it = &room.items[k];
        let top = if n == 0 { "top: " } else { "" };
        ui.label(format!("{top}{}", item_label(adv, it)));
        if let Some(d) = &it.destination {
            if let Some(d) = follow_row(ui, adv, d) {
                follow = Some(d);
            }
        }
    }
    follow
}
