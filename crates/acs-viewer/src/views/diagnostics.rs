// SPDX-License-Identifier: GPL-3.0-or-later

//! What the decoder found odd about the file.

use crate::app::Doc;
use acs_core::Severity;
use egui::{Color32, RichText};

pub fn show(doc: &mut Doc, ui: &mut egui::Ui) {
    egui::CentralPanel::default_margins().show(ui, |ui| {
        let diags = &doc.decoded.diagnostics;
        ui.heading("Diagnostics");
        ui.label(
            RichText::new(
                "Warnings are unusual but representable data; errors are data the decoder could not \
                 interpret (the model keeps a raw fallback).",
            )
            .weak(),
        );
        ui.separator();
        if diags.is_empty() {
            ui.label("The decoder found nothing unusual in this file.");
            return;
        }
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("diags")
                .striped(true)
                .num_columns(4)
                .show(ui, |ui| {
                    for h in ["Severity", "Kind", "Where", "What"] {
                        ui.label(RichText::new(h).strong());
                    }
                    ui.end_row();
                    for d in diags {
                        let (text, color) = match d.severity {
                            Severity::Warning => ("warning", Color32::YELLOW),
                            Severity::Error => ("error", Color32::LIGHT_RED),
                        };
                        ui.colored_label(color, text);
                        ui.label(format!("{:?}", d.kind));
                        ui.label(&d.location);
                        ui.label(&d.message);
                        ui.end_row();
                    }
                });
        });
    });
}
