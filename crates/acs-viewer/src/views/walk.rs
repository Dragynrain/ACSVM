// SPDX-License-Identifier: GPL-3.0-or-later

//! Walk mode: a player moved by the keyboard through the rules engine.
//!
//! All rules live in `acs-engine`; this view only turns keys and buttons into
//! commands, draws where the engine says the player is, shows what it
//! carries, and lists the engine's events in a message log. It also keeps
//! the game as it stood at the engine's save points (creatures 2.2.7), so
//! a death can be undone the original's way.

use super::*;
use acs_core::model::{Alignment, Creature, MagicDefense, ThingKind};
use acs_core::scene::{self, RoomScene};
use acs_engine::characters::{mark, Mark, Rating};
use acs_engine::creatures as cr;
use acs_engine::inventory::{self, GOLD};
use acs_engine::{
    describe, Command, Dir, Event, Game, Location, Newcomer, Player, Rules, SaveFile, Stat, Who,
};
use egui::{Key, Modifiers, RichText};

/// The world map window the original shows: 15 wide, 10 high.
const MAP_VIEW: (u8, u8) = (15, 10);
/// Log lines kept.
const LOG_MAX: usize = 500;
/// The real-time drain (movement 2.4): frames of idle time per step of the
/// bar, the turn's extra frames before the first, and the PAL C64's frame
/// rate.
const DRAIN_FRAMES: f64 = 354.0;
const DRAIN_FIRST: f64 = 17.0;
const PAL_HZ: f64 = 50.125;
/// Longest frame the drain clock counts, so a stall doesn't drain a turn.
const DRAIN_DT_MAX: f64 = 0.25;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Turn,
    Step,
    Note,
    Block,
    Door,
    Spell,
    Item,
    Creature,
    Foe,
    /// A long message: the designer's text, shown in full.
    Message,
}

pub struct Walk {
    pub game: Game,
    log: Vec<(Kind, String)>,
    /// A drop waiting for its square: the next direction key (or Space for
    /// the player's own square) sends it; Escape cancels.
    pending_drop: Option<u16>,
    /// Pieces per gold drop.
    gold_amount: u16,
    /// FIRE's target cursor (combat 8.8.3): the weapon chosen from the
    /// menu (`None` when one is held) and the square under the cursor.
    /// Direction keys move it, Space or Enter fires, Escape cancels.
    pending_fire: Option<(Option<u16>, (u8, u8))>,
    /// Characters retired with Save (characters 7.3): the engine has no
    /// roster yet, so the front end keeps them.
    retired: Vec<Player>,
    /// The current turn's bar was halved by a creature (creatures 2.6).
    hindered: bool,
    /// The game at the engine's last save point (`Game::save_point`), and
    /// at the one before it: after a death in a round that also saved, the
    /// button restores the earlier one (`Game::restore_point`).
    save: Game,
    older: Game,
    /// The name typed for the next "Add Someone".
    new_name: String,
    /// Idle seconds this turn, and the steps they have drained so far.
    idle: f64,
    drained: u32,
}

impl Walk {
    pub fn new(adv: &Adventure, seed: u64, rules: Rules) -> Self {
        // Characters 1: the first character is rolled like any other.
        let first = Newcomer::Create {
            create: "PLAYER 1".into(),
        };
        let (game, ev) = Game::new(adv.clone(), seed, rules, first);
        let mut w = Walk::with(game.clone(), game);
        w.note(format!(
            "Seed {seed}. Arrows/WASD move, Q E Z C or Home PgUp End PgDn go diagonally, Space waits. \
             To drop, click Drop, then a direction (Space = here, Esc = cancel). \
             Cast and Use ask for a target. Add Someone brings in up to 4 characters, who take turns."
        ));
        w.record(&ev);
        w
    }

    fn with(game: Game, save: Game) -> Self {
        Walk {
            game,
            older: save.clone(),
            save,
            log: Vec::new(),
            pending_drop: None,
            gold_amount: 1,
            pending_fire: None,
            retired: Vec::new(),
            hindered: false,
            new_name: String::new(),
            idle: 0.0,
            drained: 0,
        }
    }

    /// Idle seconds at which the next step drains (movement 2.4).
    fn next_drain(&self) -> f64 {
        (DRAIN_FIRST + DRAIN_FRAMES * f64::from(self.drained + 1)) / PAL_HZ
    }

    /// The share of the next step already drained, 0 to 1.
    fn drain_part(&self) -> f64 {
        let start = if self.drained == 0 {
            0.0
        } else {
            (DRAIN_FIRST + DRAIN_FRAMES * f64::from(self.drained)) / PAL_HZ
        };
        ((self.idle - start) / (self.next_drain() - start)).clamp(0.0, 1.0)
    }

    /// Runs the drain clock for `dt` seconds: an `Idle` for each step that
    /// runs out. Returns the seconds until the next one.
    fn drain(&mut self, dt: f64) -> f64 {
        self.idle += dt.min(DRAIN_DT_MAX);
        while self.idle >= self.next_drain() && !self.game.is_over() {
            let turn = self.game.log().len();
            self.drained += 1;
            self.apply(Command::Idle);
            // A new turn (record() resets the clock) or nothing happened.
            if self.drained == 0 || self.game.log().len() == turn {
                break;
            }
        }
        (self.next_drain() - self.idle).max(0.0)
    }

    /// A walk from a save file's text (docs/saves.md), replayed over `adv`.
    pub fn from_save(adv: &Adventure, text: &str) -> Result<Self, String> {
        let save = SaveFile::from_json(text).map_err(|e| e.to_string())?;
        let r = save.restore(adv).map_err(|e| e.to_string())?;
        let mut w = Walk::with(r.game, r.autosave);
        w.note(format!(
            "Loaded a save: seed {}, {} commands replayed, the autosave after {}.",
            save.seed,
            save.commands.len(),
            save.autosave
        ));
        if r.adventure_changed {
            w.log.push((
                Kind::Foe,
                "Warning: this adventure's data differs from the one the save was made with \
                 (another copy, or a decoder change)."
                    .into(),
            ));
        }
        if r.diverged {
            w.log.push((
                Kind::Foe,
                "Warning: the replay came out differently from the saved game \
                 (the rules have changed since it was saved)."
                    .into(),
            ));
        }
        let p = w.game.current_player();
        let line = format!("{}'s turn, {}.", p.name, place_name(&w.game, p.at));
        w.note(line);
        Ok(w)
    }

    /// The save file of this walk: a file name for it and its text.
    pub fn save_file(&self, original: &Adventure) -> (String, String) {
        let text = SaveFile::new(&self.game, original, self.restore_target()).to_json();
        let stem: String = original
            .name
            .trim()
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_lowercase()
                } else {
                    '-'
                }
            })
            .collect();
        let stem = stem.trim_matches('-');
        let stem = if stem.is_empty() { "game" } else { stem };
        (format!("{stem}.acsave.json"), text)
    }

    /// A `Cast`/`Use` target written as creature slot `S` of region 0 (the
    /// `--walk` form) means that slot in the current player's region.
    pub fn apply(&mut self, mut cmd: Command) {
        if let Command::Cast { target, .. } | Command::Use { target, .. } = &mut cmd {
            if let Some(Who::Creature { region: r @ 0, .. }) = target {
                *r = self.game.current_player().at.region();
            }
        }
        let ev = self.game.apply(cmd);
        self.record(&ev);
        // The engine saved during this command: keep the game as it is now.
        let point = self.game.save_point();
        if point == self.game.log().len() && point != self.save.log().len() {
            self.older = std::mem::replace(&mut self.save, self.game.clone());
        }
        // Characters 7.3: the front end keeps characters retired with Save.
        for e in &ev {
            if let Event::Retired { saved: Some(p), .. } = e {
                self.retired.push(p.as_ref().clone());
            }
        }
    }

    /// The game restoring goes back to now: the kept game at the engine's
    /// restore point (a save loaded from an older file keeps its own).
    fn restore_target(&self) -> &Game {
        let point = self.game.restore_point();
        if point != self.save.log().len() && point == self.older.log().len() {
            &self.older
        } else {
            &self.save
        }
    }

    /// Tool shortcut: the player stands at `at` (not a command, so not in
    /// the replay log); the autosave starts from there.
    pub fn place(&mut self, at: Location) {
        self.game.place(self.game.current(), at);
        self.save = self.game.clone();
        self.older = self.game.clone();
        self.note("Placed by --place (a tool shortcut, not replayed).".into());
    }

    /// Back to the last autosave (after a death, as in the original).
    /// The engine's button (`Command::Restore`) on the kept game.
    fn resume(&mut self) {
        self.game = self.restore_target().clone();
        self.pending_drop = None;
        self.pending_fire = None;
        self.note("Resumed from the last save.".into());
        let ev = self.game.apply(Command::Restore);
        self.save = self.game.clone();
        self.older = self.game.clone();
        self.record(&ev);
    }

    /// While a drop is pending, a direction key or Wait sends the drop;
    /// while FIRE's cursor is up, a direction key moves it inside the
    /// room's walls and Wait fires.
    fn key(&mut self, cmd: Command) {
        if let Some((weapon, (x, y))) = self.pending_fire {
            match cmd {
                Command::Move { dir } => {
                    let (w, h) = self.room_size();
                    let (dx, dy) = dir.delta();
                    let nx = (x as i16 + dx as i16).clamp(1, (w as i16 - 2).max(1)) as u8;
                    let ny = (y as i16 + dy as i16).clamp(1, (h as i16 - 2).max(1)) as u8;
                    self.pending_fire = Some((weapon, (nx, ny)));
                }
                _ => {
                    self.pending_fire = None;
                    self.apply(Command::Fire {
                        target: Some((x, y)),
                        weapon,
                    });
                }
            }
            return;
        }
        let Some(thing) = self.pending_drop.take() else {
            self.apply(cmd);
            return;
        };
        let dir = match cmd {
            Command::Move { dir } => Some(dir),
            _ => None,
        };
        self.apply(if thing == GOLD {
            Command::DropGold {
                amount: self.gold_amount,
                dir,
            }
        } else {
            Command::Drop { thing, dir }
        });
    }

    /// The width and height of the current player's room (1 x 1 on the map).
    fn room_size(&self) -> (u8, u8) {
        let Location::Room { region, room, .. } = self.game.current_player().at else {
            return (1, 1);
        };
        self.game
            .region(region)
            .and_then(|r| r.rooms.get(room as usize))
            .map_or((1, 1), |r| (r.width, r.height))
    }

    /// FIRE (combat 8.8): holding a missile weapon, or with `weapon` chosen
    /// from the menu, the cursor comes up on the player's square; on the
    /// map, or with NONE chosen, the command goes at once.
    fn start_fire(&mut self, weapon: Option<u16>, aim: bool) {
        let p = self.game.current_player();
        match p.at {
            Location::Room { x, y, .. } if aim => {
                self.pending_drop = None;
                self.pending_fire = Some((weapon, (x, y)));
            }
            _ => self.apply(Command::Fire { target: None, weapon }),
        }
    }

    fn note(&mut self, s: String) {
        self.log.push((Kind::Note, s));
    }

    fn record(&mut self, events: &[Event]) {
        for e in events {
            if let Event::TurnStarted { hindered, .. } = e {
                self.hindered = *hindered;
                self.idle = 0.0;
                self.drained = 0;
            }
            let kind = match e {
                Event::TurnStarted { .. } | Event::TurnEnded { .. } => Kind::Turn,
                Event::Moved { .. } => Kind::Step,
                Event::Blocked { .. }
                | Event::BumpedCreature { .. }
                | Event::BumpedPlayer { .. }
                | Event::DropRefused { .. }
                | Event::ReadyRefused { .. }
                | Event::CannotFire { .. }
                | Event::RetireRefused { .. }
                | Event::AddRefused => Kind::Block,
                Event::Retired { .. } => Kind::Door,
                Event::PlayerAdded { .. } => Kind::Door,
                Event::PickedUp { .. }
                | Event::GoldPickedUp { .. }
                | Event::Dropped { .. }
                | Event::GoldDropped { .. }
                | Event::LostOnMap { .. }
                | Event::VanishedWhenDropped { .. }
                | Event::Readied { .. }
                | Event::ThingUsedUp { .. }
                | Event::StoreOpened { .. }
                | Event::StoreClosed { .. }
                | Event::Bought { .. }
                | Event::Sold { .. } => Kind::Item,
                Event::StoreRefused { .. } => Kind::Block,
                Event::TookDoor { .. }
                | Event::EnteredRoom { .. }
                | Event::EnteredRegion { .. }
                | Event::LeftRegion { .. }
                | Event::EnteredWorldMap { .. }
                | Event::SentToStart { .. }
                | Event::EnteredArea { .. }
                | Event::LeftArea { .. }
                | Event::AreaClosed { .. } => Kind::Door,
                Event::SpellTriggered { .. }
                | Event::SpellCast { .. }
                | Event::ItemUsed { .. }
                | Event::SpellGone { .. }
                | Event::StatChanged { .. }
                | Event::Given { .. }
                | Event::Music { .. }
                | Event::RoomRidOf { .. }
                | Event::ThingAdded { .. }
                | Event::CreatureSummoned { .. }
                | Event::SummonUnanswered { .. }
                | Event::Encounter { .. }
                | Event::RoomEncounter { .. } => Kind::Spell,
                Event::LongMessage { .. } => Kind::Message,
                Event::CastRefused { .. } => Kind::Block,
                Event::Attacked { .. }
                | Event::Fired { .. }
                | Event::Shot { .. }
                | Event::Dodged { .. }
                | Event::MissileHit { .. }
                | Event::MissileStopped { .. }
                | Event::Blow { .. }
                | Event::Broke { .. }
                | Event::CreatureTurned { .. }
                | Event::CreatureRemoved { .. }
                | Event::Died { .. }
                | Event::AllPlayersDead => Kind::Foe,
                Event::CreatureTurn { .. }
                | Event::CreatureMoved { .. }
                | Event::CreatureTookDoor { .. }
                | Event::CreaturePickedUp { .. }
                | Event::CreatureGoldPickedUp { .. }
                | Event::CreatureDropped { .. }
                | Event::CreatureReadied { .. }
                | Event::CreatureUsedUp { .. } => Kind::Creature,
                _ => Kind::Note,
            };
            self.log.push((kind, describe(&self.game, e)));
        }
        if self.log.len() > LOG_MAX {
            self.log.drain(..self.log.len() - LOG_MAX);
        }
    }
}

/// A seed that differs between sessions (the engine itself never reads a clock).
pub fn fresh_seed() -> u64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(1)
    }
    #[cfg(target_arch = "wasm32")]
    {
        (js_sys::Date::now() * 1000.0) as u64 ^ (js_sys::Math::random() * u32::MAX as f64) as u64
    }
}

/// Parses `n,ne,e,...,wait` (the desktop `--walk` option).
#[cfg(not(target_arch = "wasm32"))]
pub fn parse_commands(s: &str) -> Result<Vec<Command>, String> {
    s.split(',')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(|t| {
            let t = t.to_ascii_lowercase();
            let bad =
                || format!("unknown walk step {t:?} (n, ne, ..., wait, drop:T[:dir], gold:N[:dir], ready:T, cast:T[:me|pN|cS], use:T[:me|pN|cS], fire[:X:Y[:T]], retire[:save|destroy], save, buy:T, sell:T, leave, add[:NAME])");
            let mut parts = t.split(':');
            let head = parts.next().unwrap_or_default();
            let num = |p: Option<&str>| p.and_then(|n| n.parse::<u16>().ok()).ok_or_else(bad);
            let dir = |p: Option<&str>| p.map(|d| parse_dir(d).ok_or_else(bad)).transpose();
            // me, pN (player N) or cS (creature slot S in the player's region).
            let target = |p: Option<&str>| -> Result<Option<Who>, String> {
                let Some(p) = p else { return Ok(None) };
                let n = |s: &str| s.parse::<usize>().ok().filter(|&n| n > 0).ok_or_else(bad);
                Ok(Some(match p {
                    "me" => return Ok(None),
                    _ if p.starts_with('p') => Who::Player { player: n(&p[1..])? - 1 },
                    _ if p.starts_with('c') => Who::Creature {
                        region: 0,
                        slot: n(&p[1..])? - 1,
                    },
                    _ => return Err(bad()),
                }))
            };
            Ok(match head {
                "wait" => Command::Wait,
                "idle" => Command::Idle,
                "drop" => Command::Drop {
                    thing: num(parts.next())?,
                    dir: dir(parts.next())?,
                },
                "gold" => Command::DropGold {
                    amount: num(parts.next())?,
                    dir: dir(parts.next())?,
                },
                "ready" => Command::Ready {
                    thing: num(parts.next())?,
                },
                "cast" => Command::Cast {
                    thing: num(parts.next())?,
                    target: target(parts.next())?,
                },
                "use" => Command::Use {
                    thing: num(parts.next())?,
                    target: target(parts.next())?,
                },
                "buy" => Command::Buy {
                    thing: num(parts.next())?,
                },
                "sell" => Command::Sell {
                    thing: num(parts.next())?,
                },
                "leave" => Command::LeaveStore,
                "save" => Command::SaveGame,
                "fire" => {
                    let target = match parts.next() {
                        None => None,
                        Some(x) => {
                            let x = num(Some(x))?;
                            let y = num(parts.next())?;
                            Some((
                                u8::try_from(x).map_err(|_| bad())?,
                                u8::try_from(y).map_err(|_| bad())?,
                            ))
                        }
                    };
                    let weapon = parts.next().map(|w| num(Some(w))).transpose()?;
                    Command::Fire { target, weapon }
                }
                "retire" => Command::Retire {
                    save: match parts.next() {
                        None | Some("save") => true,
                        Some("destroy") => false,
                        Some(_) => return Err(bad()),
                    },
                },
                "add" => Command::CreatePlayer {
                    name: parts.next().unwrap_or("someone").to_ascii_uppercase(),
                },
                d => Command::Move {
                    dir: parse_dir(d).ok_or_else(bad)?,
                },
            })
        })
        .collect()
}

#[cfg(not(target_arch = "wasm32"))]
fn parse_dir(s: &str) -> Option<Dir> {
    Some(match s {
        "n" => Dir::N,
        "ne" => Dir::NE,
        "e" => Dir::E,
        "se" => Dir::SE,
        "s" => Dir::S,
        "sw" => Dir::SW,
        "w" => Dir::W,
        "nw" => Dir::NW,
        _ => return None,
    })
}

fn key_command(ctx: &egui::Context) -> Option<Command> {
    const KEYS: [(Key, Option<Dir>); 23] = [
        (Key::ArrowUp, Some(Dir::N)),
        (Key::ArrowDown, Some(Dir::S)),
        (Key::ArrowLeft, Some(Dir::W)),
        (Key::ArrowRight, Some(Dir::E)),
        (Key::W, Some(Dir::N)),
        (Key::S, Some(Dir::S)),
        (Key::A, Some(Dir::W)),
        (Key::D, Some(Dir::E)),
        (Key::Q, Some(Dir::NW)),
        (Key::E, Some(Dir::NE)),
        (Key::Z, Some(Dir::SW)),
        (Key::C, Some(Dir::SE)),
        (Key::Home, Some(Dir::NW)),
        (Key::PageUp, Some(Dir::NE)),
        (Key::End, Some(Dir::SW)),
        (Key::PageDown, Some(Dir::SE)),
        (Key::Num8, Some(Dir::N)),
        (Key::Num2, Some(Dir::S)),
        (Key::Num4, Some(Dir::W)),
        (Key::Num6, Some(Dir::E)),
        (Key::Space, None),
        (Key::Enter, None),
        (Key::Num5, None),
    ];
    const DIAGONAL_DIGITS: [(Key, Dir); 4] = [
        (Key::Num7, Dir::NW),
        (Key::Num9, Dir::NE),
        (Key::Num1, Dir::SW),
        (Key::Num3, Dir::SE),
    ];
    // Text fields keep their keys.
    if ctx.memory(|m| m.focused().is_some()) {
        return None;
    }
    ctx.input_mut(|i| {
        for (k, d) in KEYS {
            if i.consume_key(Modifiers::NONE, k) {
                return Some(match d {
                    Some(dir) => Command::Move { dir },
                    None => Command::Wait,
                });
            }
        }
        for (k, dir) in DIAGONAL_DIGITS {
            if i.consume_key(Modifiers::NONE, k) {
                return Some(Command::Move { dir });
            }
        }
        None
    })
}

pub fn show(doc: &mut Doc, ui: &mut egui::Ui) {
    if doc.walk.is_none() {
        doc.walk = Some(Walk::new(&doc.decoded.adventure, fresh_seed(), Rules::default()));
    }
    {
        let walk = doc.walk.as_mut().unwrap();
        if (walk.pending_drop.is_some() || walk.pending_fire.is_some())
            && ui
                .ctx()
                .input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
        {
            walk.pending_drop = None;
            walk.pending_fire = None;
        }
        if let Some(cmd) = key_command(ui.ctx()) {
            walk.key(cmd);
        }
        // The original's bar drains while the player does nothing
        // (movement 2.4); the clock stops in a store, while typing, out of
        // focus, and in turn-based play.
        let ctx = ui.ctx();
        let paused = doc.turn_based
            || walk.game.is_over()
            || walk.game.store_visit().is_some()
            || ctx.memory(|m| m.focused().is_some())
            || !ctx.input(|i| i.focused);
        if !paused {
            let dt = f64::from(ctx.input(|i| i.stable_dt));
            let wait = walk.drain(dt);
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(wait.min(0.1)));
        }
    }
    let mut click: Option<Command> = None;
    let mut fire: Option<(Option<u16>, bool)> = None;
    let mut restart: Option<Rules> = None;
    let mut resume = false;
    let mut save_game = false;
    let mut load_game = false;
    let walk = doc.walk.as_mut().unwrap();

    let mut show_info = doc.show_info;
    egui::Panel::right("walk-info")
        .resizable(true)
        .default_size(340.0)
        .size_range(PANEL_WIDTH)
        .show_collapsible(ui, &mut show_info, |ui| {
            let g = &walk.game;
            let p = g.current_player();
            let heading = if g.players().len() > 1 {
                format!("{}'s turn", p.name)
            } else {
                p.name.clone()
            };
            ui.heading(RichText::new(heading).color(player_colour(g.current())));
            ui.label(place_name(g, p.at));
            if g.is_over() || !p.alive {
                ui.label(
                    RichText::new("Nobody in play. Resume from the last save, or add someone.")
                        .color(Color32::from_rgb(255, 110, 110)),
                );
            }
            if ui
                .add_enabled(!g.log().is_empty(), egui::Button::new("Resume from last save"))
                .on_hover_text(
                    "The original saves when a visit to another region begins, and at the end of                      a round in which SAVE GAME was chosen or a player retired with Save.",
                )
                .clicked()
            {
                resume = true;
            }
            let slowed = match (p.turn_moves < p.speed, walk.hindered) {
                (true, true) => format!(" (slowed from {}, hindered)", p.speed),
                (true, false) => format!(" (slowed from {})", p.speed),
                _ => String::new(),
            };
            let drained = if doc.turn_based || p.moves_left == 0 {
                0.0
            } else {
                walk.drain_part() as f32
            };
            ui.add(
                egui::ProgressBar::new((p.moves_left as f32 - drained) / p.turn_moves.max(1) as f32)
                    .text(format!("moves {} of {}{slowed}", p.moves_left, p.turn_moves)),
            );
            party_panel(ui, g, &mut walk.new_name, &mut click);
            actions_panel(ui, g, &walk.retired, walk.pending_fire.is_some(), &mut fire, &mut click);
            ui.separator();
            stats_panel(ui, g);
            inventory_panel(ui, g, &mut walk.pending_drop, &mut walk.gold_amount, &mut click);
            creatures_panel(ui, g);
            ui.horizontal(|ui| {
                let mut rules = g.rules();
                if ui
                    .checkbox(&mut rules.diagonals, "Diagonal moves")
                    .on_hover_text(
                        "Off in the original (measured on the C64); changing it restarts the walk.",
                    )
                    .changed()
                {
                    restart = Some(rules);
                }
                ui.checkbox(&mut doc.turn_based, "Turn-based").on_hover_text(
                    "The bar waits for you. Off: it drains while you do nothing,                      a step about every 7 seconds, as in the original.",
                );
                if ui.button("Restart").clicked() {
                    restart = Some(g.rules());
                }
            });
            ui.horizontal(|ui| {
                if ui
                    .button("Save game")
                    .on_hover_text(
                        "Writes the seed and the commands played (docs/saves.md); \
                         load it over the same adventure.",
                    )
                    .clicked()
                {
                    save_game = true;
                }
                if ui
                    .button("Load game…")
                    .on_hover_text("Opens a save file; dropping one on the window works too.")
                    .clicked()
                {
                    load_game = true;
                }
            });
            ui.label(RichText::new(format!("seed {} · {} commands", g.seed(), g.log().len())).weak());
            ui.separator();
            ui.label(RichText::new("Log").strong());
            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for (kind, line) in &walk.log {
                        let t = RichText::new(line);
                        let t = match kind {
                            Kind::Turn | Kind::Step => t.weak(),
                            Kind::Block => t.color(Color32::from_rgb(255, 170, 120)),
                            Kind::Door => t.color(Color32::from_rgb(120, 220, 255)),
                            Kind::Spell => t.color(Color32::from_rgb(230, 160, 255)),
                            Kind::Item => t.color(Color32::from_rgb(150, 230, 140)),
                            Kind::Creature => t.color(Color32::from_rgb(235, 220, 150)),
                            Kind::Foe => t.color(Color32::from_rgb(255, 110, 110)),
                            Kind::Message => t.italics().color(Color32::from_rgb(255, 240, 200)),
                            Kind::Note => t,
                        };
                        ui.label(t);
                    }
                });
        });

    if walk.game.store_visit().is_some() {
        store_window(ui.ctx(), &walk.game, &mut click);
    }

    egui::CentralPanel::no_frame().show(ui, |ui| {
        let g = &walk.game;
        let at = g.current_player().at;
        match at {
            Location::World { x, y } => draw_map(&doc.tiles, ui, g, x, y),
            Location::Room { region, room, .. } => {
                let cursor = walk.pending_fire.map(|(_, xy)| xy);
                draw_room(&doc.tiles, ui, g, region, room, cursor)
            }
        }
    });

    if let Some(cmd) = click {
        walk.apply(cmd);
    }
    if let Some((weapon, aim)) = fire {
        walk.start_fire(weapon, aim);
    }
    if resume {
        walk.resume();
    }
    if save_game {
        let (name, text) = walk.save_file(&doc.decoded.adventure);
        crate::savefile::offer(name, text);
    }
    if load_game {
        doc.pick = true;
    }
    if let Some(rules) = restart {
        doc.walk = Some(Walk::new(&doc.decoded.adventure, fresh_seed(), rules));
    }
}

/// The open store (stores 3-5): its stock with the buying prices, the
/// player's goods with the store's offers, and the way out. How the
/// original lays the store out is not yet known (docs/research/stores.md),
/// so this is a plain list.
fn store_window(ctx: &egui::Context, g: &Game, click: &mut Option<Command>) {
    use acs_engine::stores;
    let Some(visit) = g.store_visit() else { return };
    let a = g.adventure();
    let p = &g.players()[visit.player];
    let name = |t: u16| {
        a.things
            .get(t as usize - 1)
            .map(|t| t.name.trim().to_string())
            .unwrap_or_else(|| format!("thing {t}"))
    };
    egui::Window::new("Store")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label(format!("{} has {} gold to spend.", p.name, p.gold));
            ui.separator();
            ui.label(RichText::new("For sale").strong());
            let stock = g.stock(visit.region);
            if stock.is_empty() {
                ui.label(RichText::new("Nothing in stock.").weak());
            }
            for s in stock {
                let price = stores::buy_price(a, s.thing).unwrap_or(0);
                ui.horizontal(|ui| {
                    let count = if s.count > 1 {
                        format!("{} x ", s.count)
                    } else {
                        String::new()
                    };
                    ui.label(format!("{count}{}: {price} gold", name(s.thing)));
                    let can = p.gold as i32 >= price && p.count(s.thing) < inventory::limit(s.thing);
                    if ui.add_enabled(can, egui::Button::new("Buy").small()).clicked() {
                        *click = Some(Command::Buy { thing: s.thing });
                    }
                });
            }
            ui.separator();
            ui.label(RichText::new("To sell").strong());
            let mut goods = p.possessions.clone();
            goods.sort_unstable();
            goods.dedup();
            let mut any = false;
            for t in goods {
                let Some(value) = stores::value(a, t) else {
                    continue;
                };
                any = true;
                let offer = stores::sell_offer(value, p.wisdom);
                ui.horizontal(|ui| {
                    ui.label(format!("{}: the store offers {offer}", name(t)));
                    if ui.small_button("Sell").clicked() {
                        *click = Some(Command::Sell { thing: t });
                    }
                });
            }
            if !any {
                ui.label(RichText::new("Nothing the store would buy.").weak());
            }
            ui.separator();
            if ui.button("Leave the store").clicked() {
                *click = Some(Command::LeaveStore);
            }
        });
}

/// Marker colour of each of the four players (movement 1.4).
fn player_colour(player: usize) -> Color32 {
    const COLOURS: [Color32; 4] = [
        Color32::from_rgb(255, 215, 0),
        Color32::from_rgb(80, 200, 255),
        Color32::from_rgb(255, 120, 220),
        Color32::from_rgb(140, 240, 110),
    ];
    COLOURS[player % COLOURS.len()]
}

/// Everyone playing, whose turn it is, and "Add Someone" (movement 1.4):
/// the new character joins at the start point on the next round.
fn party_panel(ui: &mut egui::Ui, g: &Game, new_name: &mut String, click: &mut Option<Command>) {
    let players = g.players();
    if players.len() > 1 {
        for (i, p) in players.iter().enumerate() {
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                ui.painter().circle_filled(rect.center(), 5.0, player_colour(i));
                let mut name = RichText::new(&p.name);
                if i == g.current() {
                    name = name.strong();
                }
                if !p.alive {
                    name = name.strikethrough().weak();
                }
                ui.label(name);
                let state = if p.alive {
                    format!("LF {} · {}", p.life_force, place_name(g, p.at))
                } else {
                    "dead".into()
                };
                ui.label(RichText::new(state).weak());
            });
        }
    }
    let living = players.iter().filter(|p| p.alive).count();
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(new_name)
                .char_limit(acs_engine::NAME_MAX)
                .hint_text(format!("PLAYER {}", players.len() + 1))
                .desired_width(130.0),
        );
        let can = living < acs_engine::MAX_PLAYERS;
        if ui
            .add_enabled(can, egui::Button::new("Add Someone"))
            .on_hover_text(
                "Up to 4 characters play at once; a new one is rolled and joins at the start point next round.",
            )
            .on_disabled_hover_text("4 characters already play.")
            .clicked()
        {
            let typed = new_name.trim().to_uppercase();
            let name = if typed.is_empty() {
                format!("PLAYER {}", players.len() + 1)
            } else {
                typed
            };
            *click = Some(Command::CreatePlayer { name });
            new_name.clear();
        }
    });
}

/// The menu's FIRE and RETIRE (combat 8.8, characters 7), and the
/// characters retired with Save. FIRE holding a missile weapon brings the
/// cursor; holding none, the menu of NONE and the missile weapons owned
/// (the engine takes FIRE's move, and the readying's, when it is sent).
fn actions_panel(
    ui: &mut egui::Ui,
    g: &Game,
    retired: &[Player],
    aiming: bool,
    fire: &mut Option<(Option<u16>, bool)>,
    click: &mut Option<Command>,
) {
    let a = g.adventure();
    let p = g.current_player();
    let missile = |t: u16| {
        matches!(
            a.things.get(t as usize - 1).map(|t| &t.kind),
            Some(ThingKind::MissileWeapon { .. })
        )
    };
    let name = |t: u16| {
        a.things
            .get(t as usize - 1)
            .map(|t| t.name.trim().to_string())
            .unwrap_or_else(|| format!("thing {t}"))
    };
    if aiming {
        ui.label(
            RichText::new("Fire: move the cursor, Space = fire, Esc = cancel")
                .color(Color32::from_rgb(255, 215, 0)),
        );
    }
    ui.horizontal(|ui| {
        let in_room = matches!(p.at, Location::Room { .. });
        if p.weapon.is_some_and(missile) || !in_room {
            if ui.button("Fire").clicked() {
                *fire = Some((None, true));
            }
        } else {
            ui.menu_button("Fire", |ui| {
                ui.label(RichText::new("Select a missile weapon").weak());
                if ui.button("NONE").clicked() {
                    *fire = Some((None, false));
                    ui.close();
                }
                let mut owned: Vec<u16> = p.possessions.iter().copied().filter(|&t| missile(t)).collect();
                owned.sort_unstable();
                owned.dedup();
                for t in owned {
                    if ui.button(name(t)).clicked() {
                        *fire = Some((Some(t), true));
                        ui.close();
                    }
                }
            });
        }
        if ui
            .button("SAVE GAME")
            .on_hover_text(
                "The menu's SAVE GAME: the game is saved at the end of this round,                  and a death goes back there. The turn goes on.",
            )
            .clicked()
        {
            *click = Some(Command::SaveGame);
        }
        ui.menu_button("Retire", |ui| {
            ui.label(RichText::new("At the end of the round:").weak());
            if ui.button("Save for later adventures").clicked() {
                *click = Some(Command::Retire { save: true });
                ui.close();
            }
            if ui
                .button("Destroy (never seen again)")
                .on_hover_text("In a room everything carried is left on the square.")
                .clicked()
            {
                *click = Some(Command::Retire { save: false });
                ui.close();
            }
        });
    });
    if !retired.is_empty() {
        let names: Vec<&str> = retired.iter().map(|p| p.name.trim()).collect();
        ui.label(RichText::new(format!("Retired, saved: {}", names.join(", "))).weak());
    }
}

/// Life Force, Power and the other ratings spells change (magic 3, 5).
fn stats_panel(ui: &mut egui::Ui, g: &Game) {
    use Rating::*;
    // The profile's marks (characters 4): above or below average.
    let m = |r: Rating, v: u8| match mark(r, v.into()) {
        Some(Mark::Up) => "+",
        Some(Mark::Down) => "-",
        None => "",
    };
    let p = g.current_player();
    let me = Who::Player { player: g.current() };
    let temp = g.temporary_life(me);
    let temp = match temp {
        0 => String::new(),
        t => format!(" ({t:+} wearing off)"),
    };
    ui.add(
        egui::ProgressBar::new(p.life_force as f32 / 63.0)
            .fill(Color32::from_rgb(170, 40, 40))
            .text(format!(
                "Life Force {}{} (Constitution {}{}){temp}",
                p.life_force,
                m(LifeForce, p.life_force),
                p.constitution,
                m(Constitution, p.constitution)
            )),
    );
    ui.add(
        egui::ProgressBar::new(p.power as f32 / 127.0)
            .fill(Color32::from_rgb(60, 80, 190))
            .text(format!(
                "Power {}{} (drifts toward Wisdom {}{})",
                p.power,
                m(Power, p.power),
                p.wisdom,
                m(Wisdom, p.wisdom)
            )),
    );
    let defense = match p.magic_defense {
        MagicDefense::None => "none",
        MagicDefense::NonMagicHalfDamage => "non-magic half",
        MagicDefense::OnlyMagicDamages => "only magic hurts",
        MagicDefense::MagicHalfDamage => "magic half",
    };
    ui.label(
        RichText::new(format!(
            "Str {}{} · Dex {}{} · Size {}{} · Speed {}{} · Melee {}{} · Missile {}{} · Dodge {}{} · Parry {}{} · Armor {}{} · magic defense {defense}",
            p.strength,
            m(Strength, p.strength),
            p.dexterity,
            m(Dexterity, p.dexterity),
            p.size,
            m(Size, p.size),
            p.speed,
            m(Speed, p.speed),
            p.melee,
            m(Melee, p.melee),
            p.missile,
            m(Missile, p.missile),
            p.dodge,
            m(Dodge, p.dodge),
            p.parry,
            m(Parry, p.parry),
            p.armor_skill,
            m(ArmorSkill, p.armor_skill)
        ))
        .weak(),
    )
    .on_hover_text("+ above average, - below, as the original's profile marks them.");
}

/// "Cast" / "Use" with a menu of targets: the caster and everyone in the
/// room (magic 2.4).
fn target_menu(
    ui: &mut egui::Ui,
    g: &Game,
    label: &str,
    make: impl Fn(Option<Who>) -> Command,
    click: &mut Option<Command>,
) {
    let me = Who::Player { player: g.current() };
    ui.menu_button(label, |ui| {
        if ui.button("yourself").clicked() {
            *click = Some(make(None));
            ui.close();
        }
        let Location::Room { region, room, .. } = g.current_player().at else {
            return;
        };
        for (i, p) in g.players().iter().enumerate() {
            let who = Who::Player { player: i };
            if who != me && p.alive && p.at.room() == Some((region, room)) && ui.button(&p.name).clicked() {
                *click = Some(make(Some(who)));
                ui.close();
            }
        }
        for slot in 0..g.slots(region) {
            if let Some(c) = g.creature(region, slot).filter(|c| c.room == room) {
                let name = format!("{} · {},{} · LF {}", c.name.trim(), c.x, c.y, c.life_force);
                if ui.button(name).clicked() {
                    *click = Some(make(Some(Who::Creature { region, slot })));
                    ui.close();
                }
            }
        }
    });
}

/// Load, gold and the carried things, with Ready and Drop buttons.
fn inventory_panel(
    ui: &mut egui::Ui,
    g: &Game,
    pending: &mut Option<u16>,
    gold_amount: &mut u16,
    click: &mut Option<Command>,
) {
    let a = g.adventure();
    let p = g.current_player();
    let (load, cap) = (p.load(a), p.capacity());
    let state = if load > cap {
        " · overloaded"
    } else if load * 2 > cap {
        " · over half: slowed"
    } else {
        ""
    };
    ui.label(format!("Load {load} of {cap}{state}"));
    let name = |t: u16| {
        a.things
            .get(t as usize - 1)
            .map(|t| t.name.trim().to_string())
            .unwrap_or_else(|| format!("thing {t}"))
    };
    if let Some(t) = *pending {
        ui.label(
            RichText::new(format!(
                "Drop {}: pick a direction, Space = here, Esc = cancel",
                name(t)
            ))
            .color(Color32::from_rgb(255, 215, 0)),
        );
    }
    if p.gold > 0 {
        ui.horizontal(|ui| {
            ui.label(format!("{} {}", p.gold, name(GOLD)));
            ui.add(egui::DragValue::new(gold_amount).range(1..=inventory::PILE_LIMIT.min(p.gold)));
            if ui.small_button("Drop").clicked() {
                *pending = Some(GOLD);
            }
        });
    }
    let mut things = p.possessions.clone();
    things.sort_unstable();
    things.dedup();
    if things.is_empty() && p.gold == 0 {
        ui.label(RichText::new("Carrying nothing.").weak());
    }
    for t in things {
        let kind = a.things.get(t as usize - 1).map(|t| &t.kind);
        ui.horizontal(|ui| {
            let count = p.count(t);
            let mut label = if count > 1 {
                format!("{count} x {}", name(t))
            } else {
                name(t)
            };
            if p.weapon == Some(t) {
                label.push_str(" (readied)");
            }
            if p.armor == Some(t) {
                label.push_str(" (worn)");
            }
            ui.label(label);
            let readiable = matches!(
                kind,
                Some(
                    ThingKind::MeleeWeapon { .. } | ThingKind::MissileWeapon { .. } | ThingKind::Armor { .. }
                )
            );
            if readiable
                && p.weapon != Some(t)
                && p.armor != Some(t)
                && ui
                    .small_button("Ready")
                    .on_hover_text("Readying armor takes the rest of the turn.")
                    .clicked()
            {
                *click = Some(Command::Ready { thing: t });
            }
            match kind {
                Some(ThingKind::MagicSpell { power, .. }) => {
                    let chance = acs_engine::magic::cast_chance(p.power, *power);
                    ui.label(RichText::new(format!("cost {power}, {chance}%")).weak());
                    target_menu(ui, g, "Cast", |target| Command::Cast { thing: t, target }, click);
                }
                Some(ThingKind::MagicItem { on_use: true, .. }) => {
                    target_menu(ui, g, "Use", |target| Command::Use { thing: t, target }, click);
                }
                _ => {}
            }
            if !matches!(kind, Some(ThingKind::MagicSpell { .. })) && ui.small_button("Drop").clicked() {
                *pending = Some(t);
            }
        });
    }
}

/// Colour of a creature's alignment marker.
fn alignment_colour(a: Alignment) -> Color32 {
    match a {
        Alignment::Enemy => Color32::from_rgb(235, 60, 60),
        Alignment::Friend => Color32::from_rgb(70, 210, 90),
        Alignment::Neutral => Color32::from_rgb(230, 200, 60),
        Alignment::Thief => Color32::from_rgb(190, 110, 255),
    }
}

/// "fighter enemy, aggressive, brave" (creatures 4).
fn personality(c: &Creature) -> String {
    let kind = if cr::is_fighter(c) { "fighter" } else { "slinker" };
    let s = c.strategy;
    format!(
        "{kind} {}, {}, {}",
        format!("{:?}", s.alignment).to_lowercase(),
        if s.aggressive { "aggressive" } else { "peaceful" },
        if s.brave { "brave" } else { "cautious" }
    )
}

/// The creatures in the player's room, with what the AI uses.
fn creatures_panel(ui: &mut egui::Ui, g: &Game) {
    let Location::Room { region, room, .. } = g.current_player().at else {
        return;
    };
    let a = g.adventure();
    let here: Vec<(usize, &Creature)> = (0..g.slots(region))
        .filter_map(|s| g.creature(region, s).map(|c| (s, c)))
        .filter(|(_, c)| c.room == room)
        .collect();
    ui.separator();
    ui.label(RichText::new("Creatures here").strong());
    if here.is_empty() {
        ui.label(RichText::new("None.").weak());
    }
    for (slot, c) in here {
        let name = c.name.trim();
        let random = if g.is_random(region, slot) {
            " (random)"
        } else {
            ""
        };
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
            ui.painter()
                .circle_filled(rect.center(), 5.0, alignment_colour(c.strategy.alignment));
            ui.label(format!("{name}{random} · {},{}", c.x, c.y));
        });
        let weapon = a
            .things
            .get((c.weapon as usize).wrapping_sub(1))
            .map(|t| format!(" · {}", t.name.trim()))
            .unwrap_or_default();
        let wealth = if c.wealth > 0 {
            format!(" · {} gold", c.wealth)
        } else {
            String::new()
        };
        let wits = match c.wisdom {
            w if w > 14 => "smart",
            w if w < 6 => "dumb",
            _ => "sometimes smart",
        };
        let spells = c
            .possessions
            .iter()
            .filter(|p| {
                matches!(
                    a.things.get((p.thing as usize).wrapping_sub(1)).map(|t| &t.kind),
                    Some(ThingKind::MagicSpell { .. })
                )
            })
            .count();
        let spells = match spells {
            0 => String::new(),
            1 => " · 1 spell".into(),
            n => format!(" · {n} spells"),
        };
        let lf = g
            .stat(Who::Creature { region, slot }, Stat::LifeForce)
            .unwrap_or(0);
        ui.label(
            RichText::new(format!(
                "    {} · speed {} · LF {lf}/{} · Power {}/{} ({wits}){spells}{weapon}{wealth}",
                personality(c),
                cr::effective_speed(a, c),
                c.constitution,
                c.power,
                c.wisdom
            ))
            .weak(),
        );
    }
}

fn place_name(g: &Game, at: Location) -> String {
    let a = g.adventure();
    match at {
        Location::World { x, y } => format!("{} · {x},{y}", a.world.name.trim()),
        Location::Room { region: 0, x, y, .. } => {
            format!("{} · encounter · {x},{y}", a.world.name.trim())
        }
        Location::Room { region, room, x, y } => {
            let reg = g.region(region);
            let rname = reg.map(|r| r.name.trim()).unwrap_or("?");
            let room_name = reg
                .and_then(|r| r.rooms.get(room as usize))
                .map(|r| r.name.trim())
                .unwrap_or("?");
            format!("{rname} · {room_name} · {x},{y}")
        }
    }
}

/// A player marker: a ring in the player's colour that stays visible on any
/// picture. With more than one player, the one whose turn it is gets a
/// white outer ring. `nest` shrinks the ring for players sharing a square
/// (0 the outermost).
fn marker(painter: &egui::Painter, rect: Rect, player: usize, current: bool, nest: usize) {
    let r = rect.width().min(rect.height()) * (0.36 - 0.09 * nest.min(3) as f32);
    let c = rect.center();
    if current {
        let outer = r + (r * 0.4).max(2.0);
        painter.circle_stroke(c, outer, Stroke::new((r * 0.35).max(2.0), Color32::BLACK));
        painter.circle_stroke(c, outer, Stroke::new((r * 0.15).max(1.0), Color32::WHITE));
    }
    painter.circle_stroke(c, r, Stroke::new((r * 0.45).max(2.0), Color32::BLACK));
    painter.circle_stroke(c, r, Stroke::new((r * 0.25).max(1.0), player_colour(player)));
}

/// Draws every living player for which `cell` gives a square in view.
/// Players on one square (all of them at the start, movement 1.2) nest:
/// the current player outermost, the others inside in joining order.
fn markers(painter: &egui::Painter, g: &Game, cell: impl Fn(Location) -> Option<Rect>) {
    let many = g.players().len() > 1;
    let mut order = vec![g.current()];
    order.extend((0..g.players().len()).filter(|&i| i != g.current()));
    let mut drawn: Vec<Location> = Vec::new();
    for i in order {
        let p = &g.players()[i];
        if let Some(rect) = cell(p.at).filter(|_| p.alive) {
            let nest = drawn.iter().filter(|&&at| at == p.at).count();
            drawn.push(p.at);
            marker(painter, rect, i, many && i == g.current(), nest);
        }
    }
}

fn draw_map(tiles: &crate::tiles::Tiles, ui: &mut egui::Ui, g: &Game, px: u8, py: u8) {
    let map = &g.adventure().world;
    let n = map.tiles.len() as i16;
    let (cols, rows) = (MAP_VIEW.0 as i16, MAP_VIEW.1 as i16);
    let (cx, cy) = (px as i16 - cols / 2, py as i16 - rows / 2);
    // Without wraparound the window stops at the map edges.
    let (ox, oy) = if map.wraps {
        (cx, cy)
    } else {
        (cx.clamp(0, (n - cols).max(0)), cy.clamp(0, (n - rows).max(0)))
    };
    let (resp, painter, grid) = canvas(ui, cols as u32, rows as u32, tiles.size);
    painter.rect_filled(resp.rect, 0.0, ui.visuals().extreme_bg_color);
    for j in 0..rows {
        for i in 0..cols {
            let (mx, my) = ((ox + i).rem_euclid(n.max(1)), (oy + j).rem_euclid(n.max(1)));
            let t = map.tiles[my as usize][mx as usize];
            let pic = map.terrain.get(t as usize).map(|t| t.picture);
            tiles.paint(&painter, pic, grid.cell_rect(i as u32, j as u32));
        }
    }
    markers(&painter, g, |at| {
        let Location::World { x, y } = at else { return None };
        let (mi, mj) = (
            (x as i16 - ox).rem_euclid(n.max(1)),
            (y as i16 - oy).rem_euclid(n.max(1)),
        );
        (mi < cols && mj < rows).then(|| grid.cell_rect(mi as u32, mj as u32))
    });
}

/// The picture a room square shows. An encounter area's door item on top
/// of its square (`door`: the square is an area door) is drawn as an
/// opening on a wall square and as plain floor on an inside one, never as
/// its own picture (encounters 3.5); the opening shows the floor through
/// the wall.
fn square_picture(
    a: &Adventure,
    reg: &acs_core::model::Region,
    r: &acs_core::model::Room,
    sq: &scene::Square,
    door: bool,
) -> Option<acs_core::model::PictureId> {
    let top_is_door = sq.items.first().and_then(|&i| r.items.get(i)).map(|it| it.thing)
        == Some(acs_engine::encounters::AREA_DOOR_THING);
    if !(door && top_is_door && sq.residents.is_empty()) {
        return scene::shown(a, reg, r, sq).1;
    }
    // On a wall square the floor through the opening, inside plain floor.
    scene::floor_picture(a)
}

fn draw_room(
    tiles: &crate::tiles::Tiles,
    ui: &mut egui::Ui,
    g: &Game,
    region: u8,
    room: u8,
    cursor: Option<(u8, u8)>,
) {
    let a = g.adventure();
    let Some(reg) = g.region(region) else {
        return;
    };
    let Some(r) = reg.rooms.get(room as usize) else {
        return;
    };
    let scene = RoomScene::new(reg, r);
    let door = |x: u8, y: u8| g.area_door(Location::Room { region, room, x, y }).is_some();
    let (resp, painter, grid) = canvas(ui, r.width as u32, r.height as u32, tiles.size);
    painter.rect_filled(resp.rect, 0.0, ui.visuals().extreme_bg_color);
    for (k, sq) in scene.squares.iter().enumerate() {
        let (x, y) = ((k % r.width as usize) as u32, (k / r.width as usize) as u32);
        let pic = square_picture(a, reg, r, sq, door(x as u8, y as u8));
        tiles.paint(&painter, pic, grid.cell_rect(x, y));
    }
    // A dot in the corner of each creature's square shows its alignment.
    for c in reg.residents.iter().filter(|c| c.in_use && c.room == room) {
        let cell = grid.cell_rect(c.x as u32, c.y as u32);
        let rad = (cell.width().min(cell.height()) * 0.12).max(2.0);
        let at = cell.right_top() + egui::vec2(-rad * 1.4, rad * 1.4);
        painter.circle_filled(at, rad + 1.0, Color32::BLACK);
        painter.circle_filled(at, rad, alignment_colour(c.strategy.alignment));
    }
    markers(&painter, g, |at| match at {
        Location::Room {
            region: rg,
            room: rm,
            x,
            y,
        } if (rg, rm) == (region, room) => Some(grid.cell_rect(x as u32, y as u32)),
        _ => None,
    });
    // FIRE's target cursor (combat 8.8.3).
    if let Some((x, y)) = cursor {
        let cell = grid.cell_rect(x as u32, y as u32).shrink(1.0);
        painter.rect_stroke(
            cell,
            0.0,
            Stroke::new(3.0, Color32::BLACK),
            egui::StrokeKind::Inside,
        );
        painter.rect_stroke(
            cell.shrink(1.0),
            0.0,
            Stroke::new(1.5, Color32::from_rgb(255, 80, 60)),
            egui::StrokeKind::Inside,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::test_fixture as common;

    fn adventure() -> Adventure {
        let mut f = common::Fixture::new(acs_core::Platform::C64);
        f.header("DRAINS", "", "");
        acs_core::decode(&f.build()).expect("decodes").adventure
    }

    /// Encounters 3.5: a thing-82 door is drawn as an opening (the floor
    /// through it) on a wall square and as floor inside, never as its own
    /// picture; other items and walls as usual.
    #[test]
    fn an_area_door_is_an_opening_on_a_wall_and_floor_inside() {
        let mut f = common::Fixture::new(acs_core::Platform::C64);
        f.header("DOORS", "", "");
        f.thing(1, "GEM", [0x01, 3, 50, 5]);
        f.thing(82, "EXIT", [0x01, 4, 0, 0]);
        f.thing(127, "EMPTY FLOOR", [0x00, 1, 0, 0]);
        f.region(common::RegionSpec {
            name: "R",
            rooms: vec![common::RoomSpec {
                name: "A",
                w: 15,
                h: 10,
                wall: 9,
                items: vec![
                    (0, 5, 82, 0),
                    (7, 5, 82, 0),
                    (3, 3, 1, 0),
                    (6, 6, 1, 0),
                    (6, 6, 82, 0),
                ],
                ..Default::default()
            }],
            ..Default::default()
        });
        let a = acs_core::decode(&f.build()).expect("decodes").adventure;
        let reg = &a.regions[0];
        let r = &reg.rooms[0];
        let sc = RoomScene::new(reg, r);
        let pic = |x, y, door| square_picture(&a, reg, r, sc.square(x, y).unwrap(), door);
        let floor = scene::floor_picture(&a);
        let exit = scene::thing_picture(&a, 82);
        assert_ne!(floor, exit);
        // Wall square: an opening; inside: floor.
        assert_eq!(pic(0, 5, true), floor);
        assert_eq!(pic(7, 5, true), floor);
        // Not a door (outside an area): the item as usual.
        assert_eq!(pic(7, 5, false), exit);
        // A thing above the door shows; plain walls stay walls.
        assert_eq!(pic(6, 6, true), scene::thing_picture(&a, 1));
        assert_eq!(pic(0, 3, false), Some(r.wall_picture));
    }

    /// Feeds the clock `secs` seconds in frames of 1/50 s.
    fn run(w: &mut Walk, secs: f64) {
        for _ in 0..(secs * 50.0).round() as u32 {
            w.drain(0.02);
        }
    }

    #[test]
    fn the_bar_drains_a_step_every_354_frames() {
        let a = adventure();
        let mut w = Walk::new(&a, 3, Rules::default());
        let full = w.game.current_player().moves_left;
        assert!(full > 1);
        // Movement 2.4: the first step after 371 frames, then every 354.
        run(&mut w, 7.3);
        assert_eq!(w.game.current_player().moves_left, full);
        run(&mut w, 0.2);
        assert_eq!(w.game.current_player().moves_left, full - 1);
        assert_eq!(w.game.log(), [Command::Idle]);
        run(&mut w, 354.0 / PAL_HZ);
        assert_eq!(w.game.current_player().moves_left, full - 2);
        // A stall counts at most a quarter second.
        w.drain(60.0);
        assert_eq!(w.game.current_player().moves_left, full - 2);
        // The rest of the turn drains away and the next turn starts afresh.
        let turns = |w: &Walk| w.log.iter().filter(|(k, _)| *k == Kind::Turn).count();
        let before = turns(&w);
        run(&mut w, f64::from(full - 2) * 354.0 / PAL_HZ + 0.1);
        assert!(turns(&w) > before, "a new turn");
        assert_eq!(w.drained, 0);
        // The drain is in the log, so a save brings the same game back.
        let (_, text) = w.save_file(&a);
        assert_eq!(Walk::from_save(&a, &text).unwrap().game, w.game);
    }

    /// Creatures 2.2.7: SAVE GAME moves the kept save to the round's end;
    /// resuming goes back there and logs the button (2.2.6), which saves
    /// again at the new cycle's first visit.
    #[test]
    fn save_game_moves_the_restore_point() {
        let a = adventure();
        let mut w = Walk::new(&a, 3, Rules::default());
        // The turn goes on: the save comes at the round's end.
        w.apply(Command::SaveGame);
        assert!(w.save.log().is_empty());
        w.apply(Command::Wait);
        let kept = [Command::SaveGame, Command::Wait];
        assert_eq!(w.save.log(), kept);
        assert_eq!(w.restore_target().log().len(), 2);
        w.apply(Command::Wait);
        w.resume();
        assert_eq!(w.game.log(), [Command::SaveGame, Command::Wait, Command::Restore]);
        assert_eq!(w.game.save_point(), 3);
        assert_eq!(w.restore_target().log().len(), 3);
        // A save file of the resumed walk replays to it.
        let (_, text) = w.save_file(&a);
        assert_eq!(Walk::from_save(&a, &text).unwrap().game, w.game);
        assert_eq!(parse_commands("save").unwrap(), [Command::SaveGame]);
    }
}
