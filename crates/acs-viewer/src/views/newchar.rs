// SPDX-License-Identifier: GPL-3.0-or-later

//! A new character (client 4.2, characters 1, 2.4): the rolled ratings,
//! a name and a picture, before the game begins; and the picture grid
//! Add Someone shares.

use super::*;
use acs_core::model::PictureId;
use acs_engine::characters::{fold_name, player_pictures};

/// The New character screen's state: the seed whose roll it shows, the
/// rules the game will have, and what has been chosen.
pub struct NewCharacter {
    seed: u64,
    rules: Rules,
    name: String,
    picture: Option<PictureId>,
    /// The roll for `seed`: the first character of the game it starts
    /// (characters 1.1: the name and picture change no roll).
    rolled: Player,
}

impl NewCharacter {
    pub fn new(adv: &Adventure, rules: Rules) -> Self {
        let seed = fresh_seed();
        NewCharacter {
            seed,
            rules,
            name: String::new(),
            picture: player_pictures(adv).first().copied(),
            rolled: rolled(adv, seed, rules),
        }
    }

    fn roll(&mut self, adv: &Adventure, seed: u64) {
        self.seed = seed;
        self.rolled = rolled(adv, seed, self.rules);
    }

    /// The walk this character begins.
    fn begin(&self, adv: &Adventure) -> Walk {
        let typed = self.name.trim();
        let name = if typed.is_empty() { "PLAYER 1" } else { typed };
        Walk::create(adv, self.seed, self.rules, name, self.picture)
    }
}

/// The first character a game with `seed` rolls.
fn rolled(adv: &Adventure, seed: u64, rules: Rules) -> Player {
    let first = Newcomer::Create {
        create: "PLAYER 1".into(),
        picture: None,
    };
    Game::new(adv.clone(), seed, rules, first).0.players()[0].clone()
}

/// A grid of the pictures a new character may take; a click chooses one.
/// True when one was clicked.
pub(super) fn picture_grid(
    ui: &mut egui::Ui,
    tiles: &crate::tiles::Tiles,
    adv: &Adventure,
    chosen: &mut Option<PictureId>,
    size: f32,
) -> bool {
    let mut clicked = false;
    let ppp = ui.ctx().pixels_per_point();
    // Whole physical pixels, like the board (client 5).
    let size = (size * ppp).round() / ppp;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
        for p in player_pictures(adv) {
            let (rect, resp) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::click());
            let painter = ui.painter();
            painter.rect_filled(rect, 2.0, ui.visuals().extreme_bg_color);
            tiles.paint(painter, scene::floor_picture(adv), rect.shrink(2.0));
            tiles.paint(painter, Some(p), rect.shrink(2.0));
            let on = *chosen == Some(p);
            let stroke = if on {
                Stroke::new(3.0, Color32::from_rgb(255, 215, 0))
            } else if resp.hovered() {
                Stroke::new(1.5, Color32::WHITE)
            } else {
                Stroke::NONE
            };
            painter.rect_stroke(rect, 2.0, stroke, egui::StrokeKind::Inside);
            if resp.on_hover_text(format!("picture {p}")).clicked() {
                *chosen = Some(p);
                clicked = true;
            }
        }
    });
    clicked
}

/// The rolled ratings, as the info column shows them.
fn ratings(ui: &mut egui::Ui, p: &Player) {
    let defense = match p.magic_defense {
        MagicDefense::None => "none",
        _ => "non-magical damage halved",
    };
    egui::Grid::new("new-char-ratings")
        .num_columns(4)
        .spacing([18.0, 4.0])
        .show(ui, |ui| {
            for row in [
                [("Strength", p.strength), ("Constitution", p.constitution)],
                [("Dexterity", p.dexterity), ("Wisdom", p.wisdom)],
                [("Size", p.size), ("Speed", p.speed)],
                [("Melee", p.melee), ("Missile", p.missile)],
                [("Dodge", p.dodge), ("Parry", p.parry)],
                [("Life Force", p.life_force), ("Power", p.power)],
            ] {
                for (label, v) in row {
                    ui.label(RichText::new(label).weak());
                    ui.label(RichText::new(v.to_string()).strong());
                }
                ui.end_row();
            }
        });
    ui.label(RichText::new(format!("Magic defense: {defense}. Gold 0, nothing carried.")).weak());
}

/// The New character screen (client 4.2): `Begin` puts the walk in `doc`.
pub fn show_new_character(doc: &mut Doc, ui: &mut egui::Ui) {
    let adv = doc.decoded.adventure.clone();
    let n = doc
        .creating
        .get_or_insert_with(|| NewCharacter::new(&adv, Rules::default()));
    let mut begin = false;
    egui::CentralPanel::default_margins().show(ui, |ui| {
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.set_max_width(560.0f32.min(ui.available_width()));
                ui.add_space(12.0);
                ui.heading(
                    RichText::new("New character")
                        .font(crate::fonts::kit(ui.ctx(), 33.0, true))
                        .color(Color32::from_rgb(246, 222, 74)),
                );
                ui.label(
                    RichText::new(format!(
                        "Your character for {}. The dice have been rolled; pick a name and a look, then begin.",
                        adv.name.trim()
                    ))
                    .weak(),
                );
            });
            ui.add_space(10.0);
            // A column at most 560 points wide, centred.
            let width = 560.0f32.min(ui.available_width());
            let margin = (ui.available_width() - width) / 2.0;
            ui.horizontal(|ui| {
            ui.add_space(margin.max(0.0));
            ui.vertical(|ui| { ui.group(|ui| {
                ui.set_width(width - 16.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Name").strong());
                    let edit = ui.add(
                        egui::TextEdit::singleline(&mut n.name)
                            .char_limit(acs_engine::NAME_MAX)
                            .hint_text("PLAYER 1")
                            .desired_width(180.0),
                    );
                    if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        begin = true;
                    }
                    let shown = fold_name(n.name.trim());
                    if !shown.is_empty() && shown != n.name.trim() {
                        ui.label(RichText::new(format!("kept as {shown}")).weak());
                    }
                });
                ui.add_space(8.0);
                ui.label(RichText::new("Ratings").strong());
                ratings(ui, &n.rolled);
                if ui
                    .button("Roll again")
                    .on_hover_text("New dice. The original rolls once; nothing has been played yet.")
                    .clicked()
                {
                    n.roll(&adv, fresh_seed());
                }
                ui.add_space(8.0);
                ui.label(RichText::new("Look").strong());
                picture_grid(ui, &doc.tiles, &adv, &mut n.picture, 48.0);
                ui.add_space(10.0);
                if ui
                    .button(RichText::new("Begin").font(crate::fonts::kit(ui.ctx(), 22.0, false)))
                    .clicked()
                {
                    begin = true;
                }
            });
            });
            });
        });
    });
    if begin {
        if let Some(n) = doc.creating.take() {
            doc.walk = Some(n.begin(&adv));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixture as common;

    /// Client 4.2: Begin starts the game with the roll shown, the name as
    /// folded, and the picture chosen; an empty name is PLAYER 1.
    #[test]
    fn begin_plays_the_roll_shown_with_the_name_and_picture() {
        let mut f = common::Fixture::new(acs_core::Platform::Pc);
        f.header("NEW", "", "");
        let a = acs_core::decode(&f.build()).expect("decodes").adventure;
        let mut n = NewCharacter::new(&a, Rules::default());
        n.roll(&a, 42);
        n.name = "anne bonny".into();
        n.picture = Some(5);
        let w = n.begin(&a);
        let p = &w.game.players()[0];
        assert_eq!((p.name.as_str(), p.picture), ("ANNE BONNY", Some(5)));
        assert_eq!(
            (p.strength, p.constitution, p.dexterity, p.wisdom, p.size),
            (
                n.rolled.strength,
                n.rolled.constitution,
                n.rolled.dexterity,
                n.rolled.wisdom,
                n.rolled.size
            )
        );
        assert_eq!(w.game.seed(), 42);
        n.name = "   ".into();
        assert_eq!(n.begin(&a).game.players()[0].name, "PLAYER 1");
    }
}
