// SPDX-License-Identifier: GPL-3.0-or-later

//! A whole region: every live room drawn in place on the region grid.

use super::*;
use acs_core::model::Region;
use acs_core::scene::{self, GridRect, RoomScene, Shown};
use egui::RichText;

pub fn show(doc: &mut Doc, ui: &mut egui::Ui, r: usize) {
    let adv = &doc.decoded.adventure;
    let Some(region) = adv.regions.get(r) else { return };
    let (rooms, bounds) = scene::region_layout(region);
    let mut action = None;
    let focus = doc.selected.or(doc.hover);
    let room_at = |x: u8, y: u8| -> Option<usize> { room_at(&rooms, bounds?, x, y) };

    let mut show_info = doc.show_info;
    egui::Panel::right("region-info")
        .resizable(true)
        .default_size(300.0)
        .size_range(PANEL_WIDTH)
        .show_collapsible(ui, &mut show_info, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading(format!("{} {}", r + 1, region.name.trim()));
                ui.label(format!("{} live rooms", rooms.len()));
                match focus.and_then(|(x, y)| room_at(x, y)) {
                    Some(i) => {
                        ui.label(
                            RichText::new(format!(
                                "Room {i}: {} (click to open)",
                                region.rooms[i].name.trim()
                            ))
                            .strong(),
                        );
                    }
                    None => {
                        ui.label(RichText::new("Click a room to open it.").weak());
                    }
                }
                ui.separator();
                region_details(ui, adv, region, &mut action, r);
            });
        });

    let mut hover = None;
    egui::CentralPanel::no_frame().show(ui, |ui| {
        let Some(b) = bounds else {
            ui.label("This region has no rooms.");
            return;
        };
        let (resp, painter, grid) = canvas(ui, b.w as u32, b.h as u32, doc.tiles.size);
        painter.rect_filled(resp.rect, 0.0, ui.visuals().extreme_bg_color);
        let scenes: Vec<(usize, GridRect, RoomScene)> = rooms
            .iter()
            .map(|&(i, g)| (i, g, RoomScene::new(region, &region.rooms[i])))
            .collect();
        // Walls and floors first, then everything standing on them, so a door
        // in one room's wall is not painted over by its neighbour's wall.
        for top_layer in [false, true] {
            for (i, g, s) in &scenes {
                let room = &region.rooms[*i];
                for (k, sq) in s.squares.iter().enumerate() {
                    let (shown, pic) = scene::shown(adv, region, room, sq);
                    if matches!(shown, Shown::Wall | Shown::Floor) == top_layer {
                        continue;
                    }
                    let (x, y) = ((k % room.width as usize) as u16, (k / room.width as usize) as u16);
                    let cell = grid.cell_rect((g.x - b.x + x) as u32, (g.y - b.y + y) as u32);
                    doc.tiles.paint(&painter, pic, cell);
                }
            }
        }
        hover = hovered_cell(&resp, &grid).map(|(x, y)| (x as u8, y as u8));
        let target = hover.and_then(|(x, y)| room_at(x, y));
        if let Some(i) = target {
            if let Some((_, g)) = rooms.iter().find(|(k, _)| *k == i) {
                let a = grid.cell_rect((g.x - b.x) as u32, (g.y - b.y) as u32);
                let z = grid.cell_rect((g.x - b.x + g.w - 1) as u32, (g.y - b.y + g.h - 1) as u32);
                outline(&painter, a.union(z), Color32::WHITE, 2.0);
            }
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if resp.clicked() {
            if let Some(i) = target {
                action = Some(Action::Go(View::Room(r, i)));
            }
        }
    });
    doc.hover = hover;
    apply(doc, action);
}

/// The room under a square of the drawn region (relative to `bounds`),
/// preferring one whose inside holds it, since neighbours share walls.
fn room_at(rooms: &[(usize, GridRect)], bounds: GridRect, x: u8, y: u8) -> Option<usize> {
    let (gx, gy) = (bounds.x + x as u16, bounds.y + y as u16);
    let inside = |g: &GridRect| gx > g.x && gy > g.y && gx + 1 < g.x + g.w && gy + 1 < g.y + g.h;
    rooms
        .iter()
        .find(|(_, g)| inside(g))
        .or_else(|| rooms.iter().find(|(_, g)| g.contains(gx, gy)))
        .map(|(i, _)| *i)
}

fn region_details(
    ui: &mut egui::Ui,
    adv: &Adventure,
    region: &Region,
    action: &mut Option<Action>,
    r: usize,
) {
    ui.label(RichText::new("Rooms").strong());
    for (i, room) in region.rooms.iter().enumerate().filter(|(_, rm)| !rm.deleted) {
        let text = format!(
            "{i:>2} {} · {}×{} at {},{}",
            room.name.trim(),
            room.width,
            room.height,
            room.x,
            room.y
        );
        if ui.link(text).clicked() {
            *action = Some(Action::Go(View::Room(r, i)));
        }
    }
    ui.add_space(4.0);
    let residents: Vec<_> = region.residents.iter().filter(|c| c.in_use).collect();
    ui.label(RichText::new(format!("Residents ({})", residents.len())).strong());
    for c in residents {
        ui.label(format!(
            "room {} at {},{}: {}",
            c.room,
            c.x,
            c.y,
            creature_label(c)
        ));
    }
    ui.add_space(4.0);
    let random: Vec<_> = region.random_creatures.iter().filter(|c| c.in_use).collect();
    ui.label(RichText::new(format!("Random creatures ({})", random.len())).strong());
    for c in random {
        ui.label(creature_label(c));
    }
    ui.add_space(4.0);
    ui.label(RichText::new(format!("Store stock ({})", region.store.len())).strong());
    for p in &region.store {
        ui.label(format!("{} × {}", p.count, thing_label(adv, p.thing)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_wall_goes_to_the_room_whose_inside_is_hovered() {
        let a = GridRect {
            x: 10,
            y: 3,
            w: 5,
            h: 4,
        };
        let b = GridRect {
            x: 14,
            y: 3,
            w: 4,
            h: 4,
        };
        let rooms = [(0, a), (1, b)];
        let bounds = GridRect {
            x: 10,
            y: 3,
            w: 8,
            h: 4,
        };
        assert_eq!(room_at(&rooms, bounds, 1, 1), Some(0)); // inside A
        assert_eq!(room_at(&rooms, bounds, 5, 1), Some(1)); // inside B
        assert_eq!(room_at(&rooms, bounds, 4, 1), Some(0)); // the shared wall: first room
        assert_eq!(room_at(&rooms, bounds, 7, 0), Some(1)); // B's corner
    }
}
