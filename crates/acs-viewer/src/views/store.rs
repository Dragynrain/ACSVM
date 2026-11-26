// SPDX-License-Identifier: GPL-3.0-or-later

//! The store screen (stores 3): the original's own map of shelves, the
//! player walking its aisles, a push toward a shelf offering what is on
//! it, the button (Space, Enter or the on-screen one) answering the line
//! at the bottom. The engine knows only Buy, Sell and Leave (stores 3.8);
//! walking about the screen is this front end's, from the engine's layout
//! (`stores::square`). With Options > Simplify shops the store is a list
//! instead (`store_window` in walk.rs).

use super::*;
use acs_engine::stores::{self, StoreSquare};
use acs_engine::StoreRefusal;

/// What the line at the bottom of the store screen says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Prompt {
    /// The first prompt: the player's gold; the button starts selling.
    Welcome,
    /// A push toward a stocked shelf: "purchase X for N" (4.1).
    Offer(u16),
    /// A push toward an empty place or off the screen (3.5).
    OutOfStock,
    /// A push into an exit (3.6): the button leaves.
    Exit,
    /// Selling (5.2): this thing offered, after `skipped` copies of it
    /// passed over with a push.
    Selling { thing: u16, skipped: u16 },
    /// A message the button clears (a purchase, a refusal).
    Said(String),
}

/// Where the player stands on the store screen and what it says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Screen {
    pub at: (u8, u8),
    pub prompt: Prompt,
}

impl Default for Screen {
    fn default() -> Self {
        Screen {
            at: stores::ENTRY,
            prompt: Prompt::Welcome,
        }
    }
}

/// What the screen's buttons and keys ask for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Ask {
    /// The button: whatever the prompt offers.
    Button,
    /// A push one square.
    Push(Dir),
    /// A click on a square: next to the player a push, a shelf further off
    /// offered from where the player stands.
    Square(u8, u8),
    /// Start selling, whatever was pushed before (the original only
    /// sells from its first prompt).
    Sell,
    /// Back to the first prompt (Not now, Done, Stay).
    Back,
    Leave,
}

fn plural(name: &str) -> String {
    if name.ends_with('S') || name.ends_with('s') {
        name.to_string()
    } else if name.chars().any(|c| c.is_ascii_lowercase()) {
        format!("{name}s")
    } else {
        format!("{name}S")
    }
}

impl Walk {
    /// The store screen while a store is open and shops aren't simplified.
    pub(super) fn screen_open(&self) -> bool {
        !self.simple_shops && self.game.store_visit().is_some()
    }

    fn thing_name(&self, t: u16) -> String {
        self.game
            .adventure()
            .things
            .get((t as usize).wrapping_sub(1))
            .map(|t| t.name.trim().to_string())
            .unwrap_or_else(|| format!("thing {t}"))
    }

    /// The first thing at or after `from` (in thing order) the player could
    /// sell (5.2).
    fn next_good(&self, from: u16) -> Option<u16> {
        let offers = self.game.store_offers()?;
        offers.sell.iter().map(|o| o.thing).filter(|&t| t >= from).min()
    }

    /// The next sale offer after `thing` with `skipped` copies passed: the
    /// same thing while copies are left, else the next good, else the
    /// first prompt (5.2).
    fn sell_from(&mut self, thing: u16, skipped: u16) {
        let owned = self.game.current_player().count(thing);
        self.screen.prompt = if owned > skipped {
            Prompt::Selling { thing, skipped }
        } else {
            match self.next_good(thing + 1) {
                Some(t) => Prompt::Selling { thing: t, skipped: 0 },
                None => Prompt::Welcome,
            }
        };
    }

    /// A push toward `(x, y)` from where the player stands (3.5).
    fn push_to(&mut self, x: i16, y: i16) {
        if self.game.store_visit().is_none() {
            return;
        }
        let sq = u8::try_from(x)
            .ok()
            .zip(u8::try_from(y).ok())
            .and_then(|(x, y)| stores::square(x, y).map(|s| (x, y, s)));
        self.screen.prompt = match sq {
            // The outer ring's dead ends are wall here (client 4.1).
            Some((x, y, StoreSquare::Floor)) if edge(x, y) => return,
            Some((x, y, StoreSquare::Floor)) => {
                self.screen.at = (x, y);
                Prompt::Welcome
            }
            // Only what the engine offers (stores 2.7): a stocked thing it
            // won't sell reads as out of stock.
            Some((_, _, StoreSquare::Shelf(t))) if self.buy_offer(t).is_some() => Prompt::Offer(t),
            Some((_, _, StoreSquare::Exit)) => Prompt::Exit,
            _ => Prompt::OutOfStock,
        };
    }

    /// The engine's offer of this thing, if it sells it now (client 1).
    fn buy_offer(&self, t: u16) -> Option<acs_engine::view::BuyOffer> {
        self.game.store_offers()?.buy.into_iter().find(|b| b.thing == t)
    }

    /// Acts on one of the store screen's asks.
    pub(super) fn store_ask(&mut self, ask: Ask) {
        if !self.screen_open() {
            return;
        }
        let prompt = self.screen.prompt.clone();
        match (ask, prompt) {
            (Ask::Leave, _) | (Ask::Button, Prompt::Exit) => {
                self.screen = Screen::default();
                self.apply(Command::LeaveStore);
            }
            (Ask::Back, _) => self.screen.prompt = Prompt::Welcome,
            (Ask::Sell, _) | (Ask::Button, Prompt::Welcome) => {
                self.screen.prompt = match self.next_good(1) {
                    Some(t) => Prompt::Selling { thing: t, skipped: 0 },
                    None => Prompt::Said(format!(
                        "{} has nothing the store will buy.",
                        self.game.current_player().name
                    )),
                }
            }
            (Ask::Button, Prompt::Offer(thing)) => {
                let ev = self.apply_events(Command::Buy { thing });
                let p = self.game.current_player();
                let said = ev.iter().find_map(|e| match e {
                    Event::Bought {
                        kept: true, price, ..
                    } => Some(format!(
                        "{} buys the {} for {price}, and has {} gold left.",
                        p.name,
                        self.thing_name(thing),
                        p.gold
                    )),
                    Event::Bought { kept: false, .. } => Some(format!(
                        "{} pays, but can't carry another {}: it is lost.",
                        p.name,
                        self.thing_name(thing)
                    )),
                    Event::StoreRefused { reason, .. } => Some(match reason {
                        StoreRefusal::NotEnoughGold => format!(
                            "{} does not have enough gold for the {}.",
                            p.name,
                            self.thing_name(thing)
                        ),
                        _ => "That item is out of stock.".into(),
                    }),
                    _ => None,
                });
                self.screen.prompt = Prompt::Said(said.unwrap_or_default());
            }
            (Ask::Button, Prompt::Selling { thing, skipped }) => {
                self.apply(Command::Sell { thing });
                self.sell_from(thing, skipped);
            }
            (Ask::Push(_) | Ask::Square(..), Prompt::Selling { thing, skipped }) => {
                // The stick skips to the next offer (5.2).
                self.sell_from(thing, skipped + 1);
            }
            (Ask::Button, Prompt::OutOfStock | Prompt::Said(_)) => self.screen.prompt = Prompt::Welcome,
            (Ask::Push(dir), _) => {
                let (dx, dy) = dir.delta();
                let (x, y) = self.screen.at;
                self.push_to(x as i16 + dx as i16, y as i16 + dy as i16);
            }
            (Ask::Square(x, y), _) => {
                let (px, py) = self.screen.at;
                let (dx, dy) = (x as i16 - px as i16, y as i16 - py as i16);
                if (dx, dy) == (0, 0) {
                    return;
                }
                if dx.abs() <= 1 && dy.abs() <= 1 {
                    self.push_to(x as i16, y as i16);
                } else if let Some(StoreSquare::Shelf(_) | StoreSquare::Exit) = stores::square(x, y) {
                    // A shelf across the shop: offered as if walked to.
                    self.push_to(x as i16, y as i16);
                }
            }
        }
    }

    /// The line at the bottom of the store screen.
    pub(super) fn prompt_text(&self) -> String {
        let p = self.game.current_player();
        match &self.screen.prompt {
            Prompt::Welcome => format!("{} has {} gold to spend.", p.name, p.gold),
            Prompt::Offer(t) => {
                let Some(o) = self.buy_offer(*t) else {
                    return "That item is out of stock.".into();
                };
                let mut s = format!("Purchase the {} for {}?", self.thing_name(*t), o.price);
                if o.count > 1 {
                    s.push_str(&format!(" ({} in stock)", o.count));
                }
                if !o.kept {
                    s.push_str(" You carry as many as you can: another would be lost.");
                } else if o.refusal == Some(StoreRefusal::NotEnoughGold) {
                    s.push_str(&format!(" {} has only {} gold.", p.name, p.gold));
                }
                s
            }
            Prompt::OutOfStock => "That item is out of stock.".into(),
            Prompt::Exit => format!("Thank you, {}. Please come back soon.", p.name),
            Prompt::Selling { thing, .. } => {
                let n = p.count(*thing);
                let name = self.thing_name(*thing);
                let offer = self
                    .game
                    .store_offers()
                    .and_then(|o| o.sell.iter().find(|s| s.thing == *thing).map(|s| s.offer))
                    .unwrap_or(0);
                if n > 1 {
                    format!("{} has {n} {}. Sell one for {offer}?", p.name, plural(&name))
                } else {
                    format!("{} has 1 {name}. Sell it for {offer}?", p.name)
                }
            }
            Prompt::Said(s) => s.clone(),
        }
    }
}

/// The store screen's board, as the PC original draws it (client 4.1):
/// floor in the aisles, the room's wall picture on every shelf place,
/// each stocked thing on floor cut into the shelf, and a door at each exit
/// (the picture of a portal in the store's room). The player on its
/// square (stores 3.3, 3.4). Returns the square clicked, if any.
pub(super) fn draw_store(tiles: &crate::tiles::Tiles, ui: &mut egui::Ui, walk: &Walk) -> Option<(u8, u8)> {
    let g = &walk.game;
    let a = g.adventure();
    let region = g.store_visit()?.region;
    let (w, h) = (stores::SCREEN_W as u32, stores::SCREEN_H as u32);
    let (resp, painter, grid) = canvas(ui, w, h, tiles);
    painter.rect_filled(resp.rect, 0.0, ui.visuals().extreme_bg_color);
    let floor = scene::floor_picture(a);
    let (wall, door) = store_room_pictures(walk);
    for y in 0..h {
        for x in 0..w {
            let cell = grid.cell_rect(x, y);
            match stores::square(x as u8, y as u8) {
                Some(StoreSquare::Shelf(t)) if g.on_shelf(region, t) > 0 => {
                    tiles.paint(&painter, floor, cell);
                    tiles.paint(&painter, scene::thing_picture(a, t), cell);
                }
                // An empty shelf place is wall; without a wall picture, a
                // dark board, so the aisles still read as the only floor.
                Some(StoreSquare::Shelf(_)) => match wall.filter(|&p| tiles.get(p).is_some()) {
                    Some(p) => tiles.paint(&painter, Some(p), cell),
                    None => {
                        painter.rect_filled(cell, 0.0, Color32::from_rgb(40, 24, 10));
                    }
                },
                Some(StoreSquare::Exit) => {
                    tiles.paint(&painter, floor, cell);
                    match door.filter(|&p| tiles.get(p).is_some()) {
                        Some(p) => tiles.paint(&painter, Some(p), cell),
                        None => brackets(&painter, cell.shrink(2.0), Color32::from_rgb(120, 220, 255), 2.0),
                    }
                }
                // The outer ring's dead ends, drawn as wall (client 4.1).
                Some(StoreSquare::Floor) if edge(x as u8, y as u8) => {
                    match wall.filter(|&p| tiles.get(p).is_some()) {
                        Some(p) => tiles.paint(&painter, Some(p), cell),
                        None => {
                            painter.rect_filled(cell, 0.0, Color32::from_rgb(40, 24, 10));
                        }
                    }
                }
                _ => tiles.paint(&painter, floor, cell),
            }
        }
    }
    let (px, py) = walk.screen.at;
    let cell = grid.cell_rect(px as u32, py as u32);
    let picture = acs_engine::characters::picture_of(a, g.current_player().picture);
    match picture.filter(|&p| tiles.get(p).is_some()) {
        Some(pic) => tiles.paint(&painter, Some(pic), cell),
        None => marker(&painter, cell, g.current(), false, 0),
    }
    // What is under the pointer, so a shelf can be read without a push.
    let hover = hovered_cell(&resp, &grid);
    if let Some((x, y)) = hover {
        if let Some(StoreSquare::Shelf(t)) = stores::square(x as u8, y as u8) {
            let offer = g
                .store_offers()
                .and_then(|o| o.buy.into_iter().find(|b| b.thing == t));
            if let Some(o) = offer {
                let (price, n) = (o.price, o.count);
                outline(
                    &painter,
                    grid.cell_rect(x, y),
                    Color32::from_rgb(255, 215, 0),
                    2.0,
                );
                resp.clone()
                    .on_hover_text(format!("{}: {price} gold ({n} in stock)", walk.thing_name(t)));
            }
        }
    }
    hover.filter(|_| resp.clicked()).map(|(x, y)| (x as u8, y as u8))
}

/// The prompt line and its buttons, under the store screen.
pub(super) fn prompt_bar(ui: &mut egui::Ui, walk: &Walk, ask: &mut Option<Ask>) {
    let font = crate::fonts::kit(ui.ctx(), 22.0, false);
    ui.add_space(4.0);
    ui.label(
        RichText::new(walk.prompt_text())
            .font(font)
            .color(Color32::from_rgb(251, 241, 207)),
    );
    ui.horizontal_wrapped(|ui| {
        let mut b = |ui: &mut egui::Ui, label: &str, a: Ask| {
            if ui.button(label).clicked() {
                *ask = Some(a);
            }
        };
        match &walk.screen.prompt {
            Prompt::Welcome => {
                b(ui, "Sell your goods", Ask::Sell);
                b(ui, "Leave the store", Ask::Leave);
            }
            Prompt::Offer(_) => {
                b(ui, "Buy", Ask::Button);
                b(ui, "Not now", Ask::Back);
            }
            Prompt::Selling { .. } => {
                b(ui, "Sell", Ask::Button);
                b(ui, "Skip", Ask::Push(Dir::N));
                b(ui, "Done", Ask::Back);
            }
            Prompt::Exit => {
                b(ui, "Leave", Ask::Button);
                b(ui, "Stay", Ask::Back);
            }
            Prompt::OutOfStock | Prompt::Said(_) => b(ui, "OK", Ask::Button),
        }
        ui.label(
            RichText::new("Arrows walk the aisles; walk into a shelf to see it. Space answers. Esc leaves.")
                .weak(),
        );
    });
    ui.add_space(4.0);
}

/// The wall picture of the room the store stands in, and the picture of
/// the first portal there, for the store screen's shelves and exits.
fn store_room_pictures(walk: &Walk) -> (Option<PictureId>, Option<PictureId>) {
    let g = &walk.game;
    let Some((rg, rm)) = g.current_player().at.room() else {
        return (None, None);
    };
    let Some(room) = g.region(rg).and_then(|r| r.rooms.get(rm as usize)) else {
        return (None, None);
    };
    let a = g.adventure();
    let door = room.items.iter().find_map(|i| {
        let t = a.things.get((i.thing as usize).checked_sub(1)?)?;
        match t.kind {
            acs_core::model::ThingKind::Portal { picture, .. } => Some(picture),
            _ => None,
        }
    });
    (Some(room.wall_picture), door)
}

/// A floor square of the outer ring (rows 0 and 9, between the shelf
/// places): open in the rules (stores 3.3) but reached only by a diagonal
/// step into a dead end, so the viewer draws it as wall and blocks it.
fn edge(x: u8, y: u8) -> bool {
    (y == 0 || y + 1 == stores::SCREEN_H) && stores::square(x, y) == Some(StoreSquare::Floor)
}
