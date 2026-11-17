// SPDX-License-Identifier: GPL-3.0-or-later

//! The 40 x 40 world map with its doors, exit points and start.

use super::*;
use acs_core::model::{Passability, WorldMap};
use egui::RichText;

pub fn show(doc: &mut Doc, ui: &mut egui::Ui) {
    let adv = &doc.decoded.adventure;
    let map = &adv.world;
    let rows = map.tiles.len() as u32;
    let cols = map.tiles.first().map_or(0, |r| r.len()) as u32;
    let mut action = None;
    let focus = doc.selected.or(doc.hover);

    let mut show_info = doc.show_info;
    egui::Panel::right("world-info")
        .resizable(true)
        .default_size(300.0)
        .size_range(panel_width(ui))
        .show_collapsible(ui, &mut show_info, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading(map.name.trim());
                ui.label(format!(
                    "{cols}×{rows} · start {},{} · {}",
                    map.start.x,
                    map.start.y,
                    if map.wraps {
                        "wraps around"
                    } else {
                        "no wraparound"
                    }
                ));
                ui.horizontal_wrapped(|ui| {
                    // Each told apart by its outline too (client 7).
                    let [door, exit, start] = crate::access::Access::get(ui.ctx()).map_colours();
                    ui.colored_label(door, "door (outline)");
                    ui.colored_label(exit, "exit point (corners)");
                    ui.colored_label(start, "start (double)");
                });
                ui.separator();
                match focus {
                    Some((x, y)) => {
                        if let Some(d) = square_info(ui, adv, map, x, y) {
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
                ui.label(RichText::new("Terrain").strong());
                for t in &map.terrain {
                    ui.horizontal(|ui| {
                        picture(ui, doc, Some(t.picture), 24.0);
                        // Wraps beside the picture in a narrow column.
                        ui.add(
                            egui::Label::new(format!(
                                "{:>2} {} · {}",
                                t.number,
                                t.name.trim(),
                                passability(t.passability, t.parameter, adv)
                            ))
                            .wrap(),
                        );
                    });
                }
            });
        });

    let mut hover = None;
    egui::CentralPanel::no_frame().show(ui, |ui| {
        let (resp, painter, grid) = canvas(ui, cols, rows, &doc.tiles);
        painter.rect_filled(resp.rect, 0.0, ui.visuals().extreme_bg_color);
        for (y, row) in map.tiles.iter().enumerate() {
            for (x, &t) in row.iter().enumerate() {
                let pic = map.terrain.get(t as usize).map(|t| t.picture);
                doc.tiles.paint(&painter, pic, grid.cell_rect(x as u32, y as u32));
            }
        }
        let on_map = |x: u8, y: u8| (x as u32) < cols && (y as u32) < rows;
        let w = (grid.tile.x / 8.0).clamp(1.0, 3.0);
        let [door, exit, start] = crate::access::Access::get(ui.ctx()).map_colours();
        for p in map.exit_points.iter().flatten() {
            if on_map(p.x, p.y) {
                brackets(&painter, grid.cell_rect(p.x as u32, p.y as u32), exit, w + 1.0);
            }
        }
        for d in &map.doors {
            if on_map(d.at.x, d.at.y) {
                outline(&painter, grid.cell_rect(d.at.x as u32, d.at.y as u32), door, w);
            }
        }
        if on_map(map.start.x, map.start.y) {
            let cell = grid.cell_rect(map.start.x as u32, map.start.y as u32);
            outline(&painter, cell, start, w);
            outline(&painter, cell.shrink(2.0 * w + 1.0), start, w);
        }
        hover = hovered_cell(&resp, &grid).map(|(x, y)| (x as u8, y as u8));
        if let Some((x, y)) = hover {
            outline(
                &painter,
                grid.cell_rect(x as u32, y as u32),
                Color32::from_white_alpha(160),
                1.5,
            );
            if door_at(map, x, y).is_some() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
        }
        if let Some((x, y)) = doc.selected {
            outline(&painter, grid.cell_rect(x as u32, y as u32), Color32::WHITE, 2.0);
        }
        if let Some((View::World, x, y)) = doc.highlight {
            if on_map(x, y) {
                pulse(ui, &painter, grid.cell_rect(x as u32, y as u32));
            }
        }
        if resp.clicked() {
            match hover.and_then(|(x, y)| door_at(map, x, y)) {
                Some(d) => action = Some(Action::Follow(d)),
                None => action = Some(Action::Select(hover)),
            }
        }
    });
    doc.hover = hover;
    apply(doc, action);
}

fn door_at(map: &WorldMap, x: u8, y: u8) -> Option<Destination> {
    map.doors
        .iter()
        .find(|d| d.at.x == x && d.at.y == y)
        .map(|d| d.destination)
}

fn passability(p: Passability, parameter: u16, adv: &Adventure) -> String {
    match p {
        Passability::Impassable => "impassable".into(),
        Passability::Open => "open".into(),
        Passability::OwnersOf => format!("owners of {}", thing_label(adv, parameter)),
        Passability::TriggersSpell => format!("triggers {}", thing_label(adv, parameter)),
    }
}

fn square_info(ui: &mut egui::Ui, adv: &Adventure, map: &WorldMap, x: u8, y: u8) -> Option<Destination> {
    let mut follow = None;
    let t = map
        .tiles
        .get(y as usize)
        .and_then(|r| r.get(x as usize))
        .and_then(|&t| map.terrain.get(t as usize));
    ui.label(RichText::new(format!("Square {x},{y}")).strong());
    if let Some(t) = t {
        ui.label(format!(
            "{} · {}",
            t.name.trim(),
            passability(t.passability, t.parameter, adv)
        ));
        for c in map
            .random_creatures
            .iter()
            .filter(|c| c.creature.in_use && c.terrain == t.number)
        {
            ui.label(format!("{}% per move: {}", c.chance, creature_label(&c.creature)));
        }
    }
    for d in map.doors.iter().filter(|d| d.at.x == x && d.at.y == y) {
        let bit = if d.unknown_y_bit7 { " (y bit 7 set)" } else { "" };
        ui.label(format!("Door {}{bit}", d.index));
        if let Some(d) = follow_row(ui, adv, &d.destination) {
            follow = Some(d);
        }
    }
    for (k, _) in map
        .exit_points
        .iter()
        .enumerate()
        .filter(|(_, p)| p.is_some_and(|p| p.x == x && p.y == y))
    {
        ui.label(format!("Exit point {}", k + 32));
    }
    if (map.start.x, map.start.y) == (x, y) {
        ui.label("Start");
    }
    follow
}
