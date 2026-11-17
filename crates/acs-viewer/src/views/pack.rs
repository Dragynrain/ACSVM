// SPDX-License-Identifier: GPL-3.0-or-later

//! The HD pack view (client 5): the loaded pack and its coverage of the
//! open adventure, with every picture it misses.

use super::*;
use crate::hdpack;
use egui::RichText;

pub fn show(doc: &mut Doc, ui: &mut egui::Ui) {
    egui::CentralPanel::default_margins().show(ui, |ui| {
        let Some(pack) = doc.pack.clone() else {
            ui.heading("HD pack");
            ui.label(
                "No HD pack loaded. An HD pack is one PNG of replacement tiles with its manifest \
                 inside; drop it on the window or use Open…, then the Look menu offers it.",
            );
            ui.label(RichText::new("Its format is in docs/client.md, section 5.").weak());
            return;
        };
        let c = hdpack::coverage(&pack, &doc.decoded.adventure);
        ui.heading(format!("HD pack: {}", pack.name));
        let license = if pack.license.is_empty() {
            "no license given"
        } else {
            &pack.license
        };
        ui.label(format!(
            "{license} · {} px tiles · {} entries",
            pack.tile, pack.entries
        ));
        ui.separator();
        ui.add(egui::ProgressBar::new(c.percent() / 100.0).text(format!(
            "Covers {} of {} pictures ({:.0}%); {} blank pictures not counted",
            c.matched,
            c.counted,
            c.percent(),
            c.blank
        )));
        if c.missing.is_empty() {
            ui.label("Every picture is covered.");
            return;
        }
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("Missing ({})", c.missing.len())).strong());
            if ui
                .button("Copy missing")
                .on_hover_text("The missing pictures as manifest entries, the start of a pack.")
                .clicked()
            {
                ui.ctx().copy_text(c.missing_entries());
            }
        });
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("pack-missing").striped(true).show(ui, |ui| {
                    for &(n, h) in &c.missing {
                        picture(ui, doc, Some(n as u16), 32.0);
                        ui.label(format!("picture {n}"));
                        ui.add(egui::Label::new(RichText::new(hdpack::hex(h)).monospace()).selectable(true));
                        ui.end_row();
                    }
                });
            });
    });
}
