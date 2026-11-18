// SPDX-License-Identifier: GPL-3.0-or-later

//! The opening (client 3.3): the title screen, then the intro page, as the
//! original opens play (capture.md 10.48).

use crate::app::Doc;
use acs_core::limits::Profile;
use acs_core::model::Adventure;
use eframe::egui::{self, Color32, Key, Modifiers, RichText, Sense};

/// Which page of the opening shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opening {
    Title,
    Intro,
}

impl Opening {
    /// The page after this one, if any: the intro page unless the
    /// adventure has none (`no_intro_page`, or an Extended one with no
    /// intro text).
    fn next(self, adv: &Adventure) -> Option<Opening> {
        match self {
            Opening::Title if has_intro_page(adv) => Some(Opening::Intro),
            _ => None,
        }
    }
}

/// Whether the adventure opens with its intro page (client 3.3).
pub fn has_intro_page(adv: &Adventure) -> bool {
    !adv.no_intro_page && !(adv.profile == Profile::Extended && adv.intro.trim().is_empty())
}

const GOLD: Color32 = Color32::from_rgb(246, 222, 74);
const PAPER: Color32 = Color32::from_rgb(251, 241, 207);

/// Shows the page `doc.opening` names; Space, Enter, a click or a tap goes
/// on to the next, and past the last, clears it.
pub fn show(doc: &mut Doc, ui: &mut egui::Ui) {
    let Some(page) = doc.opening else { return };
    let adv = &doc.decoded.adventure;
    let ctx = ui.ctx().clone();
    let key = ctx.input_mut(|i| {
        i.consume_key(Modifiers::NONE, Key::Space) || i.consume_key(Modifiers::NONE, Key::Enter)
    });
    let big = crate::fonts::kit(&ctx, 33.0, true);
    let text = crate::fonts::kit(&ctx, 22.0, false);
    let panel = egui::CentralPanel::default_margins().show(ui, |ui| {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.set_max_width(640.0f32.min(ui.available_width()));
                    match page {
                        Opening::Title => {
                            ui.add_space((ui.available_height() * 0.25).max(24.0));
                            ui.label(RichText::new(adv.name.trim()).font(big.clone()).color(GOLD));
                            if !adv.byline.trim().is_empty() {
                                ui.add_space(8.0);
                                ui.label(RichText::new("by").font(text.clone()).color(PAPER));
                                ui.add_space(4.0);
                                ui.label(RichText::new(adv.byline.trim()).font(text.clone()).color(PAPER));
                            }
                        }
                        Opening::Intro => {
                            ui.add_space(16.0);
                            egui::Frame::group(ui.style()).show(ui, |ui| {
                                ui.label(RichText::new(adv.name.trim()).font(text.clone()).color(GOLD));
                            });
                            ui.add_space(12.0);
                            let body = intro_text(adv);
                            egui::Frame::group(ui.style())
                                .stroke(egui::Stroke::new(3.0, PAPER))
                                .inner_margin(16.0)
                                .show(ui, |ui| {
                                    // Prose reads from the left, not centred line by line.
                                    ui.set_min_width(ui.available_width());
                                    ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                                        ui.label(RichText::new(body).font(text.clone()).color(PAPER));
                                    });
                                });
                        }
                    }
                    ui.add_space(24.0);
                    ui.label(RichText::new("Press Space or click to continue.").weak());
                });
            });
    });
    let clicked = panel.response.interact(Sense::click()).clicked();
    if key || clicked {
        doc.opening = page.next(&doc.decoded.adventure);
    }
}

/// The intro as shown: an Extended one as paragraphs, a Classic one line
/// for line, its 32-character lines as stored, trailing blanks dropped.
fn intro_text(adv: &Adventure) -> String {
    if adv.profile == Profile::Extended {
        return super::walk::paragraphs(&adv.intro);
    }
    let lines: Vec<&str> = adv.intro.lines().map(str::trim_end).collect();
    let end = lines.iter().rposition(|l| !l.is_empty()).map_or(0, |i| i + 1);
    lines[..end].join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn adventure(profile: Profile, intro: &str, no_page: bool) -> Adventure {
        let mut f = crate::views::walk::tests::in_a_room().game.adventure().clone();
        f.profile = profile;
        f.intro = intro.into();
        f.no_intro_page = no_page;
        f
    }

    /// Client 3.3: the title, then the intro page unless the adventure
    /// says not to.
    #[test]
    fn the_intro_page_follows_the_title_when_the_adventure_has_one() {
        let a = adventure(Profile::Classic, "", false);
        assert_eq!(
            Opening::Title.next(&a),
            Some(Opening::Intro),
            "a Classic flag set, even blank"
        );
        let a = adventure(Profile::Classic, "SOME TEXT", true);
        assert_eq!(Opening::Title.next(&a), None, "the flag clear");
        let a = adventure(Profile::Extended, "  ", false);
        assert_eq!(Opening::Title.next(&a), None, "ours, with no text");
        let a = adventure(Profile::Extended, "A story.", false);
        assert_eq!(Opening::Title.next(&a), Some(Opening::Intro));
        assert_eq!(Opening::Intro.next(&a), None);
    }
}
