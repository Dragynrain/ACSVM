// SPDX-License-Identifier: GPL-3.0-or-later

//! Thing and creature browsers: a list on the left, details on the right.

use super::*;
use acs_core::model::{Creature, ThingKind};
use egui::RichText;

pub fn things(doc: &mut Doc, ui: &mut egui::Ui) {
    let adv = &doc.decoded.adventure;
    let mut pick = None;
    egui::Panel::left("thing-list")
        .resizable(true)
        .default_size(280.0)
        .size_range(PANEL_WIDTH)
        .show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (i, t) in adv.things.iter().enumerate() {
                    if matches!(t.kind, ThingKind::Undefined) {
                        continue;
                    }
                    ui.horizontal(|ui| {
                        picture(ui, doc, t.kind.picture(), 24.0);
                        let text = format!("{:>3} {} · {}", t.number, t.name.trim(), t.kind.type_name());
                        if ui.selectable_label(doc.thing == i, text).clicked() {
                            pick = Some(i);
                        }
                    });
                }
            });
        });
    let mut action = None;
    egui::CentralPanel::default_margins().show(ui, |ui| {
        let Some(t) = adv.things.get(doc.thing) else {
            return;
        };
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal(|ui| {
                picture(ui, doc, t.kind.picture(), 96.0);
                ui.vertical(|ui| {
                    ui.heading(format!("#{} {}", t.number, t.name.trim()));
                    ui.label(t.kind.type_name());
                    if let Some(p) = t.kind.picture() {
                        ui.label(format!("Picture {p}"));
                    }
                    for (label, m) in [
                        ("Blocked message", &t.blocked_message),
                        ("Spell message", &t.spell_message),
                    ] {
                        if let Some(m) = m {
                            let text = adv
                                .short_messages
                                .get(m.slot as usize - 1)
                                .map(|s| s.as_str())
                                .unwrap_or("?");
                            let hidden = if m.shown { "" } else { " (hidden)" };
                            ui.label(format!("{label} {}{hidden}: {}", m.slot, text.replace('\n', " ")));
                        }
                    }
                });
            });
            ui.separator();
            let places = placements(adv, t.number);
            ui.label(RichText::new(format!("Placed in rooms: {}", places.len())).strong());
            for (r, i, x, y) in places.iter().take(200) {
                let room = &adv.regions[*r].rooms[*i];
                let text = format!(
                    "region {} {} · room {i} {} · {x},{y}",
                    r + 1,
                    adv.regions[*r].name.trim(),
                    room.name.trim()
                );
                if ui.link(text).clicked() {
                    action = Some((View::Room(*r, *i), *x, *y));
                }
            }
            if places.len() > 200 {
                ui.label(format!("… and {} more", places.len() - 200));
            }
            ui.separator();
            json(ui, t);
        });
    });
    if let Some(i) = pick {
        doc.thing = i;
    }
    if let Some((v, x, y)) = action {
        doc.go(v);
        doc.highlight = Some((v, x, y));
    }
}

/// Every room square where master thing `number` is placed: (region, room, x, y).
fn placements(adv: &Adventure, number: u16) -> Vec<(usize, usize, u8, u8)> {
    let mut out = Vec::new();
    for (r, region) in adv.regions.iter().enumerate() {
        for (i, room) in region.rooms.iter().enumerate().filter(|(_, rm)| !rm.deleted) {
            for it in room.items.iter().filter(|it| it.thing == number) {
                out.push((r, i, it.x, it.y));
            }
        }
    }
    out
}

pub fn creatures(doc: &mut Doc, ui: &mut egui::Ui) {
    let adv = &doc.decoded.adventure;
    let mut pick = None;
    egui::Panel::left("creature-list")
        .resizable(true)
        .default_size(280.0)
        .size_range(PANEL_WIDTH)
        .show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (i, c) in adv.creatures.iter().enumerate().filter(|(_, c)| c.in_use) {
                    ui.horizontal(|ui| {
                        picture(ui, doc, c.picture, 24.0);
                        if ui
                            .selectable_label(doc.creature == i, format!("{:>3} {}", c.number, c.name.trim()))
                            .clicked()
                        {
                            pick = Some(i);
                        }
                    });
                }
            });
        });
    egui::CentralPanel::default_margins().show(ui, |ui| {
        let Some(c) = adv.creatures.get(doc.creature) else {
            return;
        };
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal(|ui| {
                picture(ui, doc, c.picture, 96.0);
                ui.vertical(|ui| {
                    ui.heading(format!("#{} {}", c.number, c.name.trim()));
                    let class = c
                        .class
                        .and_then(|k| adv.creature_classes.get(k as usize))
                        .map(|s| s.trim().to_string())
                        .unwrap_or_default();
                    ui.label(format!("{class} · {}", creature_label(c)));
                    if let Some(p) = c.picture {
                        ui.label(format!("Picture {p}"));
                    }
                });
            });
            ui.separator();
            stats(ui, adv, c);
            ui.separator();
            json(ui, c);
        });
    });
    if let Some(i) = pick {
        doc.creature = i;
    }
}

fn stats(ui: &mut egui::Ui, adv: &Adventure, c: &Creature) {
    egui::Grid::new("stats").striped(true).show(ui, |ui| {
        let rows: [(&str, String); 10] = [
            (
                "Constitution / Life Force",
                format!("{} / {}", c.constitution, c.life_force),
            ),
            (
                "Strength / Dexterity",
                format!("{} / {}", c.strength, c.dexterity),
            ),
            ("Speed / Wisdom", format!("{} / {}", c.speed, c.wisdom)),
            ("Power / Size", format!("{} / {}", c.power, c.size)),
            ("Dodge / Parry", format!("{} / {}", c.dodge, c.parry)),
            (
                "Melee / Missile / Armor skill",
                format!("{} / {} / {}", c.melee, c.missile, c.armor_skill),
            ),
            ("Magic defense", format!("{:?}", c.magic_defense)),
            ("Wealth", c.wealth.to_string()),
            (
                "Weapon",
                if c.weapon == 0 {
                    "none".into()
                } else {
                    thing_label(adv, c.weapon)
                },
            ),
            (
                "Armor",
                if c.armor == 0 {
                    "none".into()
                } else {
                    thing_label(adv, c.armor)
                },
            ),
        ];
        for (k, v) in rows {
            ui.label(k);
            ui.label(v);
            ui.end_row();
        }
        ui.label("Carries");
        ui.vertical(|ui| {
            if c.possessions.is_empty() {
                ui.label("nothing");
            }
            for p in &c.possessions {
                ui.label(format!("{} × {}", p.count, thing_label(adv, p.thing)));
            }
        });
        ui.end_row();
    });
}

/// The decoded record as JSON, collapsed by default.
fn json<T: serde::Serialize>(ui: &mut egui::Ui, value: &T) {
    egui::CollapsingHeader::new("Decoded record (JSON)").show(ui, |ui| {
        let text = serde_json::to_string_pretty(value).unwrap_or_else(|e| e.to_string());
        ui.label(RichText::new(text).monospace());
    });
}
