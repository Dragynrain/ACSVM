// SPDX-License-Identifier: GPL-3.0-or-later

//! Long messages (8 x 32), short messages (3 x 40) and the intro text.

use crate::app::Doc;
use egui::RichText;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Intro,
    Long,
    Short,
}

pub fn show(doc: &mut Doc, ui: &mut egui::Ui) {
    egui::CentralPanel::default_margins().show(ui, |ui| {
        ui.horizontal(|ui| {
            for (t, name) in [
                (Tab::Intro, "Intro"),
                (Tab::Long, "Long messages"),
                (Tab::Short, "Short messages"),
            ] {
                ui.selectable_value(&mut doc.message_tab, t, name);
            }
        });
        ui.separator();
        let adv = &doc.decoded.adventure;
        egui::ScrollArea::vertical().show(ui, |ui| match doc.message_tab {
            Tab::Intro => {
                ui.label(RichText::new(format!("{}\n{}", adv.name, adv.byline)).strong());
                ui.add_space(6.0);
                ui.label(RichText::new(&adv.intro).monospace());
            }
            Tab::Long => list(ui, &adv.long_messages),
            Tab::Short => list(ui, &adv.short_messages),
        });
    });
}

fn list(ui: &mut egui::Ui, messages: &[String]) {
    let mut any = false;
    for (i, m) in messages.iter().enumerate() {
        if m.trim().is_empty() {
            continue;
        }
        any = true;
        ui.horizontal_top(|ui| {
            ui.label(RichText::new(format!("{:>3}", i + 1)).monospace().weak());
            ui.label(RichText::new(m.trim_end()).monospace());
        });
        ui.add_space(6.0);
    }
    if !any {
        ui.label(RichText::new("No messages.").weak());
    }
}
