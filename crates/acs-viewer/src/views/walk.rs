// SPDX-License-Identifier: GPL-3.0-or-later

//! Walk mode: a player moved by the keyboard through the rules engine.
//!
//! All rules live in `acs-engine`; this view only turns keys and buttons into
//! commands, draws where the engine says the player is, shows what it
//! carries, and lists the engine's events in a message log. It also keeps
//! the autosave the original makes on entering or leaving a region, so a
//! death can be undone the original's way.

use super::*;
use acs_core::model::{Alignment, Creature, MagicDefense, ThingKind};
use acs_core::scene::{self, RoomScene};
use acs_engine::creatures as cr;
use acs_engine::inventory::{self, GOLD};
use acs_engine::{describe, Command, Dir, Event, Game, Location, PlayerSetup, Rules, Stat, Who};
use egui::{Key, Modifiers, RichText};

/// The world map window the original shows: 15 wide, 10 high.
const MAP_VIEW: (u8, u8) = (15, 10);
/// Log lines kept.
const LOG_MAX: usize = 500;

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
    /// The current turn's bar was halved by a creature (creatures 2.6).
    hindered: bool,
    /// The last autosave: the game as it stood after the last region change
    /// (or the start), what the original resumes from after a death.
    save: Game,
}

impl Walk {
    pub fn new(adv: &Adventure, seed: u64, rules: Rules) -> Self {
        let (game, ev) = Game::new(adv.clone(), seed, rules, PlayerSetup::default());
        let mut w = Walk {
            save: game.clone(),
            game,
            log: Vec::new(),
            pending_drop: None,
            gold_amount: 1,
            hindered: false,
        };
        w.note(format!(
            "Seed {seed}. Arrows/WASD move, Q E Z C or Home PgUp End PgDn go diagonally, Space waits. \
             To drop, click Drop, then a direction (Space = here, Esc = cancel). \
             Cast and Use ask for a target."
        ));
        w.record(&ev);
        w
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
        // The original autosaves on entering or leaving a region.
        let saves = ev
            .iter()
            .any(|e| matches!(e, Event::EnteredRegion { .. } | Event::LeftRegion { .. }));
        if saves && !self.game.is_over() {
            self.save = self.game.clone();
        }
    }

    /// Tool shortcut: the player stands at `at` (not a command, so not in
    /// the replay log); the autosave starts from there.
    pub fn place(&mut self, at: Location) {
        self.game.place(self.game.current(), at);
        self.save = self.game.clone();
        self.note("Placed by --place (a tool shortcut, not replayed).".into());
    }

    /// Back to the last autosave (after a death, as in the original).
    fn resume(&mut self) {
        self.game = self.save.clone();
        self.pending_drop = None;
        self.note("Resumed from the last save.".into());
    }

    /// While a drop is pending, a direction key or Wait sends the drop.
    fn key(&mut self, cmd: Command) {
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

    fn note(&mut self, s: String) {
        self.log.push((Kind::Note, s));
    }

    fn record(&mut self, events: &[Event]) {
        for e in events {
            if let Event::TurnStarted { hindered, .. } = e {
                self.hindered = *hindered;
            }
            let kind = match e {
                Event::TurnStarted { .. } | Event::TurnEnded { .. } => Kind::Turn,
                Event::Moved { .. } => Kind::Step,
                Event::Blocked { .. }
                | Event::BumpedCreature { .. }
                | Event::DropRefused { .. }
                | Event::ReadyRefused { .. } => Kind::Block,
                Event::PickedUp { .. }
                | Event::GoldPickedUp { .. }
                | Event::Dropped { .. }
                | Event::GoldDropped { .. }
                | Event::LostOnMap { .. }
                | Event::VanishedWhenDropped { .. }
                | Event::Readied { .. }
                | Event::ThingUsedUp { .. } => Kind::Item,
                Event::TookDoor { .. }
                | Event::EnteredRoom { .. }
                | Event::EnteredRegion { .. }
                | Event::LeftRegion { .. }
                | Event::EnteredWorldMap { .. }
                | Event::SentToStart { .. } => Kind::Door,
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
                Event::CreatureConfronts { .. }
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
                || format!("unknown walk step {t:?} (n, ne, ..., wait, drop:T[:dir], gold:N[:dir], ready:T, cast:T[:me|pN|cS], use:T[:me|pN|cS])");
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
        if walk.pending_drop.is_some()
            && ui
                .ctx()
                .input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
        {
            walk.pending_drop = None;
        }
        if let Some(cmd) = key_command(ui.ctx()) {
            walk.key(cmd);
        }
    }
    let mut click: Option<Command> = None;
    let mut restart: Option<Rules> = None;
    let mut resume = false;
    let walk = doc.walk.as_mut().unwrap();

    let mut show_info = doc.show_info;
    egui::Panel::right("walk-info")
        .resizable(true)
        .default_size(340.0)
        .size_range(PANEL_WIDTH)
        .show_collapsible(ui, &mut show_info, |ui| {
            let g = &walk.game;
            let p = g.current_player();
            ui.heading(&p.name);
            ui.label(place_name(g, p.at));
            if g.is_over() || !p.alive {
                ui.label(
                    RichText::new("Dead. Play resumes from the last save (the last region change).")
                        .color(Color32::from_rgb(255, 110, 110)),
                );
            }
            if ui
                .add_enabled(!g.log().is_empty(), egui::Button::new("Resume from last save"))
                .on_hover_text("The original autosaves on entering or leaving a region.")
                .clicked()
            {
                resume = true;
            }
            let slowed = match (p.turn_moves < p.speed, walk.hindered) {
                (true, true) => format!(" (slowed from {}, hindered)", p.speed),
                (true, false) => format!(" (slowed from {})", p.speed),
                _ => String::new(),
            };
            ui.add(
                egui::ProgressBar::new(p.moves_left as f32 / p.turn_moves.max(1) as f32)
                    .text(format!("moves {} of {}{slowed}", p.moves_left, p.turn_moves)),
            );
            stats_panel(ui, g);
            inventory_panel(ui, g, &mut walk.pending_drop, &mut walk.gold_amount, &mut click);
            creatures_panel(ui, g);
            ui.horizontal(|ui| {
                let mut rules = g.rules();
                if ui
                    .checkbox(&mut rules.diagonals, "Diagonal moves")
                    .on_hover_text("Unknown in the original; changing it restarts the walk.")
                    .changed()
                {
                    restart = Some(rules);
                }
                if ui.button("Restart").clicked() {
                    restart = Some(g.rules());
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

    egui::CentralPanel::no_frame().show(ui, |ui| {
        let g = &walk.game;
        let at = g.current_player().at;
        match at {
            Location::World { x, y } => draw_map(&doc.tiles, ui, g, x, y),
            Location::Room { region, room, x, y } => draw_room(&doc.tiles, ui, g, region, room, x, y),
        }
    });

    if let Some(cmd) = click {
        walk.apply(cmd);
    }
    if resume {
        walk.resume();
    }
    if let Some(rules) = restart {
        doc.walk = Some(Walk::new(&doc.decoded.adventure, fresh_seed(), rules));
    }
}

/// Life Force, Power and the other ratings spells change (magic 3, 5).
fn stats_panel(ui: &mut egui::Ui, g: &Game) {
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
                "Life Force {} (Constitution {}){temp}",
                p.life_force, p.constitution
            )),
    );
    ui.add(
        egui::ProgressBar::new(p.power as f32 / 127.0)
            .fill(Color32::from_rgb(60, 80, 190))
            .text(format!("Power {} (drifts toward Wisdom {})", p.power, p.wisdom)),
    );
    let defense = match p.magic_defense {
        MagicDefense::None => "none",
        MagicDefense::NonMagicHalfDamage => "non-magic half",
        MagicDefense::OnlyMagicDamages => "only magic hurts",
        MagicDefense::MagicHalfDamage => "magic half",
    };
    ui.label(
        RichText::new(format!(
            "Str {} · Dex {} · Size {} · Melee {} · Missile {} · Dodge {} · Parry {} · Armor {} · magic defense {defense}",
            p.strength, p.dexterity, p.size, p.melee, p.missile, p.dodge, p.parry, p.armor_skill
        ))
        .weak(),
    );
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
        Location::Room { region, room, x, y } => {
            let reg = a.regions.iter().find(|r| r.number == region);
            let rname = reg.map(|r| r.name.trim()).unwrap_or("?");
            let room_name = reg
                .and_then(|r| r.rooms.get(room as usize))
                .map(|r| r.name.trim())
                .unwrap_or("?");
            format!("{rname} · {room_name} · {x},{y}")
        }
    }
}

/// The player marker: a ring that stays visible on any picture.
fn marker(painter: &egui::Painter, rect: Rect) {
    let r = rect.width().min(rect.height()) * 0.36;
    let c = rect.center();
    painter.circle_stroke(c, r, Stroke::new((r * 0.45).max(2.0), Color32::BLACK));
    painter.circle_stroke(
        c,
        r,
        Stroke::new((r * 0.25).max(1.0), Color32::from_rgb(255, 215, 0)),
    );
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
    let (mi, mj) = (
        (px as i16 - ox).rem_euclid(n.max(1)),
        (py as i16 - oy).rem_euclid(n.max(1)),
    );
    if mi < cols && mj < rows {
        marker(&painter, grid.cell_rect(mi as u32, mj as u32));
    }
}

fn draw_room(tiles: &crate::tiles::Tiles, ui: &mut egui::Ui, g: &Game, region: u8, room: u8, px: u8, py: u8) {
    let a = g.adventure();
    let Some(reg) = a.regions.iter().find(|r| r.number == region) else {
        return;
    };
    let Some(r) = reg.rooms.get(room as usize) else {
        return;
    };
    let scene = RoomScene::new(reg, r);
    let (resp, painter, grid) = canvas(ui, r.width as u32, r.height as u32, tiles.size);
    painter.rect_filled(resp.rect, 0.0, ui.visuals().extreme_bg_color);
    for (k, sq) in scene.squares.iter().enumerate() {
        let (x, y) = ((k % r.width as usize) as u32, (k / r.width as usize) as u32);
        let (_, pic) = scene::shown(a, reg, r, sq);
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
    if g.current_player().alive {
        marker(&painter, grid.cell_rect(px as u32, py as u32));
    }
}
