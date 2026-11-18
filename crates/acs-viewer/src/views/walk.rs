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
use acs_engine::characters::{Mark, Rating};
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

/// What a target cursor sends (client 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Aim {
    /// FIRE with the weapon chosen from the menu (`None`: the one held).
    Fire(Option<u16>),
    Cast(u16),
    Use(u16),
}

impl Aim {
    fn command(self, at: (u8, u8)) -> Command {
        match self {
            Aim::Fire(weapon) => Command::Fire {
                target: Some(at),
                weapon,
            },
            Aim::Cast(thing) => Command::Cast {
                thing,
                target: None,
                at: Some(at),
            },
            Aim::Use(thing) => Command::Use {
                thing,
                target: None,
                at: Some(at),
            },
        }
    }
}

/// Where `to` lies from `from`: in the same room, or both on a map `n`
/// squares each way (the shorter way round where it wraps).
fn offset(from: Location, to: Location, (w, h): (i16, i16), wraps: bool) -> Option<(i16, i16)> {
    let near = |d: i16, n: i16| {
        if wraps && n > 0 {
            let d = d.rem_euclid(n);
            if d > n / 2 {
                d - n
            } else {
                d
            }
        } else {
            d
        }
    };
    match (from, to) {
        (Location::World { x, y }, Location::World { x: tx, y: ty }) => {
            Some((near(tx as i16 - x as i16, w), near(ty as i16 - y as i16, h)))
        }
        (
            Location::Room { region, room, x, y },
            Location::Room {
                region: r2,
                room: m2,
                x: tx,
                y: ty,
            },
        ) if (region, room) == (r2, m2) => Some((tx as i16 - x as i16, ty as i16 - y as i16)),
        _ => None,
    }
}

pub struct Walk {
    pub game: Game,
    log: Vec<(Kind, String)>,
    /// A drop waiting for its square: the next direction key (or Space for
    /// the player's own square) sends it; Escape cancels.
    pending_drop: Option<u16>,
    /// Pieces per gold drop.
    gold_amount: u16,
    /// A target cursor: FIRE's (combat 8.8.3) or a cast's or use's at a
    /// square (client 4), and the square under it. Direction keys or the
    /// mouse move it, Space, Enter or a click sends it, Escape cancels.
    pending_aim: Option<(Aim, (u8, u8))>,
    /// The last input was a touch: the on-screen pad shows (client 4).
    pub touch: bool,
    /// Play Music cues not yet played (client 6).
    cues: Vec<u8>,
    /// Log lines dropped from the front, and lines ever logged that the
    /// screen reader region has had (client 7).
    trimmed: usize,
    announced: usize,
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
            pending_aim: None,
            touch: false,
            cues: Vec::new(),
            trimmed: 0,
            announced: 0,
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
        // A replay's cues are not played (client 6), nor its lines read.
        w.cues.clear();
        w.take_unannounced();
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
        self.cancel();
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
        if let Some((aim, (x, y))) = self.pending_aim {
            match cmd {
                Command::Move { dir } => {
                    let (w, h) = self.room_size();
                    let (dx, dy) = dir.delta();
                    let nx = (x as i16 + dx as i16).clamp(1, (w as i16 - 2).max(1)) as u8;
                    let ny = (y as i16 + dy as i16).clamp(1, (h as i16 - 2).max(1)) as u8;
                    self.pending_aim = Some((aim, (nx, ny)));
                }
                _ => {
                    self.pending_aim = None;
                    self.apply(aim.command((x, y)));
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
        if !(aim && self.start_aim(Aim::Fire(weapon))) {
            self.apply(Command::Fire { target: None, weapon });
        }
    }

    /// Raises the target cursor on the player's square; false on the map,
    /// where there is no square to aim at.
    fn start_aim(&mut self, aim: Aim) -> bool {
        let Location::Room { x, y, .. } = self.game.current_player().at else {
            return false;
        };
        self.pending_drop = None;
        self.pending_aim = Some((aim, (x, y)));
        true
    }

    /// Escape: drops the waiting drop or cursor.
    fn cancel(&mut self) {
        self.pending_drop = None;
        self.pending_aim = None;
    }

    /// The log lines since the last call, for the screen reader region
    /// (client 7).
    pub fn take_unannounced(&mut self) -> Vec<String> {
        let start = self.announced.saturating_sub(self.trimmed).min(self.log.len());
        self.announced = self.trimmed + self.log.len();
        self.log[start..].iter().map(|(_, l)| l.clone()).collect()
    }

    /// The Play Music cues since the last call (client 6).
    pub fn take_cues(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.cues)
    }

    /// FIRE goes without a menu: on the map, or holding a missile weapon.
    fn fire_ready(&self) -> bool {
        let p = self.game.current_player();
        let missile = p
            .weapon
            .and_then(|t| self.game.adventure().things.get((t as usize).wrapping_sub(1)))
            .is_some_and(|t| matches!(t.kind, ThingKind::MissileWeapon { .. }));
        missile || matches!(p.at, Location::World { .. })
    }

    /// Whether a drop or a cursor waits.
    fn waiting(&self) -> bool {
        self.pending_drop.is_some() || self.pending_aim.is_some()
    }

    /// The square `(x, y)` of the current room is inside its walls.
    fn inside(&self, x: u8, y: u8) -> bool {
        let (w, h) = self.room_size();
        (1..w.saturating_sub(1)).contains(&x) && (1..h.saturating_sub(1)).contains(&y)
    }

    /// The pointer is over `at` with a cursor up: the cursor follows it
    /// onto inside squares (client 4).
    fn hover(&mut self, at: Location) {
        if let (Some((aim, _)), Location::Room { x, y, .. }) = (self.pending_aim, at) {
            if self.offset(at).is_some() && self.inside(x, y) {
                self.pending_aim = Some((aim, (x, y)));
            }
        }
    }

    /// A click or tap on the board square `at` (client 4).
    pub fn click(&mut self, at: Location) {
        let Some((dx, dy)) = self.offset(at) else {
            return;
        };
        if let Some((aim, _)) = self.pending_aim {
            if let Location::Room { x, y, .. } = at {
                if self.inside(x, y) {
                    self.pending_aim = None;
                    self.apply(aim.command((x, y)));
                }
            }
            return;
        }
        if (dx, dy) == (0, 0) {
            if self.pending_drop.is_some() {
                self.key(Command::Wait);
            }
            return;
        }
        let step = i8::try_from(dx).ok().zip(i8::try_from(dy).ok());
        if let Some(dir) = step.and_then(|(a, b)| Dir::from_delta(a, b)) {
            self.key(Command::Move { dir });
        }
    }

    /// Where `at` lies from the current player, if it is in the same room
    /// or both are on the map (across the wraparound where it wraps).
    fn offset(&self, at: Location) -> Option<(i16, i16)> {
        let world = &self.game.adventure().world;
        offset(self.game.current_player().at, at, map_size(world), world.wraps)
    }

    fn note(&mut self, s: String) {
        self.log.push((Kind::Note, s));
    }

    fn record(&mut self, events: &[Event]) {
        for e in events {
            if let Event::Music { number } = e {
                self.cues.push(*number);
            }
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
            self.trimmed += self.log.len() - LOG_MAX;
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
                || format!("unknown walk step {t:?} (n, ne, ..., wait, drop:T[:dir], gold:N[:dir], ready:T, cast:T[:me|pN|cS|X:Y], use:T[:me|pN|cS|X:Y], fire[:X:Y[:T]], retire[:save|destroy], save, buy:T, sell:T, leave, add[:NAME])");
            let mut parts = t.split(':');
            let head = parts.next().unwrap_or_default();
            let num = |p: Option<&str>| p.and_then(|n| n.parse::<u16>().ok()).ok_or_else(bad);
            let dir = |p: Option<&str>| p.map(|d| parse_dir(d).ok_or_else(bad)).transpose();
            // me, pN (player N) or cS (creature slot S in the player's
            // region); for a cast or use, X:Y is a square instead.
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
                "cast" | "use" => {
                    let thing = num(parts.next())?;
                    let rest: Vec<&str> = parts.by_ref().collect();
                    let (at, target) = match rest.as_slice() {
                        [x, y] => {
                            let n = |v: &str| v.parse::<u8>().map_err(|_| bad());
                            (Some((n(x)?, n(y)?)), None)
                        }
                        [] => (None, None),
                        [w] => (None, target(Some(w))?),
                        _ => return Err(bad()),
                    };
                    if head == "cast" {
                        Command::Cast { thing, at, target }
                    } else {
                        Command::Use { thing, at, target }
                    }
                }
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
        if walk.waiting()
            && ui
                .ctx()
                .input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
        {
            walk.cancel();
        }
        // The pad follows the last input: a touch shows it, a key hides
        // it (client 4).
        let (touched, keyed) = ui.ctx().input(|i| {
            let any = |f: fn(&egui::Event) -> bool| i.events.iter().any(f);
            (
                any(|e| matches!(e, egui::Event::Touch { .. })),
                any(|e| matches!(e, egui::Event::Key { pressed: true, .. })),
            )
        });
        if touched {
            walk.touch = true;
        } else if keyed {
            walk.touch = false;
        }
        // Sound may start once the player has done something (client 6).
        if touched || keyed || ui.ctx().input(|i| i.pointer.any_pressed()) {
            doc.audio.unlock();
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
    let mut aim: Option<Aim> = None;
    let mut load_game = false;
    let walk = doc.walk.as_mut().unwrap();

    let mut show_info = doc.show_info;
    egui::Panel::right("walk-info")
        .resizable(true)
        .default_size(340.0)
        .size_range(panel_width(ui))
        .show_collapsible(ui, &mut show_info, |ui| {
            // Taller than the window on a short screen: it scrolls
            // (client 2).
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            let g = &walk.game;
            let p = g.current_player();
            let heading = if g.players().len() > 1 {
                format!("{}'s turn", p.name)
            } else {
                p.name.clone()
            };
            ui.heading(RichText::new(heading).color(player_colour(ui.ctx(), g.current())));
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
                    "The original saves when a visit to another region begins, and at the end of a round in which SAVE GAME was chosen or a player retired with Save.",
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
            actions_panel(ui, g, &walk.retired, walk.pending_aim.is_some(), &mut fire, &mut click);
            ui.separator();
            stats_panel(ui, g);
            inventory_panel(
                ui,
                g,
                &mut walk.pending_drop,
                &mut walk.gold_amount,
                &mut aim,
                &mut click,
            );
            creatures_panel(ui, g);
            ui.horizontal_wrapped(|ui| {
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
                let mut pad = doc.pad.unwrap_or(walk.touch);
                if ui
                    .checkbox(&mut pad, "On-screen pad")
                    .on_hover_text("Arrows and Wait on the board; shows by itself after a touch.")
                    .changed()
                {
                    doc.pad = Some(pad);
                }
                ui.checkbox(&mut doc.turn_based, "Turn-based").on_hover_text(
                    "The bar waits for you. Off: it drains while you do nothing, a step about every 7 seconds, as in the original.",
                );
                if ui.button("Restart").clicked() {
                    restart = Some(g.rules());
                }
            });
            ui.horizontal_wrapped(|ui| {
                let audio = &mut doc.audio;
                let mut changed = ui
                    .checkbox(&mut audio.on, "Sound")
                    .on_hover_text("Play Music's cues: placeholders until the finished set.")
                    .changed();
                let mut pct = (audio.volume * 100.0).round();
                changed |= ui
                    .add_enabled(audio.on, egui::Slider::new(&mut pct, 0.0..=100.0).suffix("%"))
                    .changed();
                audio.volume = pct / 100.0;
                if changed {
                    audio.apply_controls();
                }
            });
            ui.horizontal_wrapped(|ui| {
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
            });
        });

    // The on-screen pad: its own strip at the bottom, under the thumbs,
    // never over the board (client 4).
    let mut pad_key = None;
    if doc.pad.unwrap_or(walk.touch) {
        let fire = !walk.waiting() && walk.fire_ready();
        let waiting = walk.waiting();
        egui::Panel::bottom("walk-pad").show(ui, |ui| {
            pad_key = touch_pad(ui, waiting, fire);
        });
    }

    // The message log: always shown in walk mode, whatever else is hidden
    // (client 2, 3).
    let height = ui.ctx().content_rect().height();
    let narrow = ui.ctx().content_rect().width() < crate::app::NARROW;
    // The board is 15 x 10 squares (3:2): on a tall window the log also
    // takes the height the board can't use.
    let left = ui.available_rect_before_wrap();
    let spare = left.height() - left.width() * 2.0 / 3.0 - 8.0;
    let share = height * if narrow { 0.16 } else { 0.22 };
    egui::Panel::bottom("walk-log")
        .resizable(true)
        .default_size(share.max(spare).min(height * 0.6))
        .size_range(48.0..=height * 0.6)
        .show(ui, |ui| {
            // Both ways: a long message's lines are never rewrapped.
            // Held to the panel: content taller than it would otherwise
            // grow the panel to its largest size.
            egui::ScrollArea::both()
                .max_height(ui.available_height())
                .stick_to_bottom(true)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let access = crate::access::Access::get(ui.ctx());
                    // The kit font at 2x (client 7.1).
                    let font = crate::fonts::kit(ui.ctx(), 22.0, false);
                    for (kind, line) in &walk.log {
                        // A symbol as well as a colour (client 7).
                        let t = match kind_mark(*kind, &access) {
                            Some((sym, colour)) => RichText::new(format!("{sym} {line}")).color(colour),
                            None => RichText::new(line),
                        };
                        let t = match kind {
                            Kind::Turn | Kind::Step => t.weak(),
                            // Not italics: a slanted pixel font blurs.
                            Kind::Message => t.color(Color32::from_rgb(251, 241, 207)),
                            _ => t,
                        };
                        let t = t.font(font.clone());
                        if *kind == Kind::Message {
                            // A long message keeps its stored lines (magic
                            // 3.10): shown as they are, never rewrapped.
                            ui.add(egui::Label::new(t).extend());
                        } else {
                            ui.label(t);
                        }
                    }
                });
        });

    if walk.game.store_visit().is_some() {
        store_window(ui.ctx(), &walk.game, &mut click);
    }

    let board = egui::CentralPanel::no_frame().show(ui, |ui| {
        let g = &walk.game;
        let at = g.current_player().at;
        match at {
            Location::World { x, y } => draw_map(&doc.tiles, ui, g, x, y),
            Location::Room { region, room, .. } => {
                let cursor = walk.pending_aim.map(|(_, xy)| xy);
                draw_room(&doc.tiles, ui, g, region, room, cursor)
            }
        }
    });
    // Clicks and taps on the board (client 4); the cursor follows the
    // pointer only while it moves, so the keys can still move it.
    match board.inner {
        Some((at, true)) => walk.click(at),
        Some((at, false)) if ui.ctx().input(|i| i.pointer.delta() != egui::Vec2::ZERO) => walk.hover(at),
        _ => {}
    }

    if let Some(cmd) = click {
        walk.apply(cmd);
    }
    if let Some((weapon, aim)) = fire {
        walk.start_fire(weapon, aim);
    }
    if let Some(a) = aim {
        walk.start_aim(a);
    }
    match pad_key {
        Some(PadKey::Key(cmd)) => walk.key(cmd),
        Some(PadKey::Cancel) => walk.cancel(),
        Some(PadKey::Fire) => walk.start_fire(None, true),
        None => {}
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
    // Play Music (client 6): what this frame's commands played.
    for n in walk.take_cues() {
        doc.audio.play(n);
    }
    // New log lines for a screen reader (client 7).
    crate::live::announce(&walk.take_unannounced());
    if let Some(rules) = restart {
        doc.walk = Some(Walk::new(&doc.decoded.adventure, fresh_seed(), rules));
    }
}

/// The open store (stores 3-5): its stock with the buying prices, the
/// player's goods with the store's offers, and the way out. How the
/// original lays the store out is not yet known (docs/research/stores.md),
/// so this is a plain list.
fn store_window(ctx: &egui::Context, g: &Game, click: &mut Option<Command>) {
    let Some(offers) = g.store_offers() else { return };
    let a = g.adventure();
    let p = &g.players()[offers.player];
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
            ui.label(format!("{} has {} gold to spend.", p.name, offers.gold));
            ui.separator();
            ui.label(RichText::new("For sale").strong());
            if offers.buy.is_empty() {
                ui.label(RichText::new("Nothing in stock.").weak());
            }
            for o in &offers.buy {
                ui.horizontal(|ui| {
                    let count = if o.count > 1 {
                        format!("{} x ", o.count)
                    } else {
                        String::new()
                    };
                    ui.label(format!("{count}{}: {} gold", name(o.thing), o.price));
                    let button = ui.add_enabled(o.refusal.is_none(), egui::Button::new("Buy").small());
                    let button = if o.kept {
                        button
                    } else {
                        button.on_hover_text("You carry as many as you can: this one would be lost.")
                    };
                    if button.clicked() {
                        *click = Some(Command::Buy { thing: o.thing });
                    }
                });
            }
            ui.separator();
            ui.label(RichText::new("To sell").strong());
            if offers.sell.is_empty() {
                ui.label(RichText::new("Nothing the store would buy.").weak());
            }
            for o in &offers.sell {
                ui.horizontal(|ui| {
                    ui.label(format!("{}: the store offers {}", name(o.thing), o.offer));
                    if ui.small_button("Sell").clicked() {
                        *click = Some(Command::Sell { thing: o.thing });
                    }
                });
            }
            ui.separator();
            if ui.button("Leave the store").clicked() {
                *click = Some(Command::LeaveStore);
            }
        });
}

/// What the on-screen pad asked for.
enum PadKey {
    /// A key, as the keyboard's: a step or Wait.
    Key(Command),
    Cancel,
    Fire,
}

/// The on-screen pad (client 4): eight arrows round Wait on the left,
/// and on the right Cancel while a drop or cursor waits and Fire when it
/// needs no menu.
fn touch_pad(ui: &mut egui::Ui, waiting: bool, fire: bool) -> Option<PadKey> {
    const KEY: f32 = 52.0;
    const GAP: f32 = 4.0;
    let mut out = None;
    ui.add_space(GAP);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(GAP, GAP);
        ui.vertical(|ui| {
            use Dir::*;
            for row in [
                [Some(NW), Some(N), Some(NE)],
                [Some(W), None, Some(E)],
                [Some(SW), Some(S), Some(SE)],
            ] {
                ui.horizontal(|ui| {
                    for d in row {
                        if pad_key(ui, KEY, d) {
                            out = Some(PadKey::Key(d.map_or(Command::Wait, |dir| Command::Move { dir })));
                        }
                    }
                });
            }
        });
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Max), |ui| {
            let mut big = |ui: &mut egui::Ui, label: &str, key: PadKey| {
                let b = egui::Button::new(RichText::new(label).size(16.0)).min_size(egui::vec2(104.0, KEY));
                if ui.add(b).clicked() {
                    out = Some(key);
                }
            };
            if fire {
                big(ui, "Fire", PadKey::Fire);
            }
            if waiting {
                big(ui, "Cancel", PadKey::Cancel);
            }
        });
    });
    ui.add_space(GAP);
    out
}

/// One of the pad's square keys: an arrow, or Wait for `None`.
fn pad_key(ui: &mut egui::Ui, size: f32, dir: Option<Dir>) -> bool {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::click());
    let fill = if resp.is_pointer_button_down_on() {
        ui.visuals().selection.bg_fill
    } else {
        Color32::from_black_alpha(160)
    };
    let painter = ui.painter();
    painter.rect_filled(rect, 8.0, fill);
    painter.rect_stroke(
        rect,
        8.0,
        Stroke::new(1.0, Color32::from_white_alpha(70)),
        egui::StrokeKind::Inside,
    );
    let ink = Color32::from_white_alpha(230);
    match dir {
        Some(d) => {
            let (dx, dy) = d.delta();
            let v = egui::vec2(dx as f32, dy as f32).normalized() * size * 0.28;
            painter.arrow(rect.center() - v, v * 2.0, Stroke::new(3.0, ink));
        }
        None => {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Wait",
                egui::FontId::proportional(14.0),
                ink,
            );
        }
    }
    let label = match dir {
        Some(d) => format!("{d:?}"),
        None => "Wait".into(),
    };
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &label));
    resp.clicked()
}

/// Marker colour of each of the four players (movement 1.4), in the
/// palette chosen (client 7).
fn player_colour(ctx: &egui::Context, player: usize) -> Color32 {
    crate::access::Access::get(ctx).player_colour(player)
}

/// A log kind's symbol and colour (client 7); turns, steps and notes have
/// neither.
fn kind_mark(kind: Kind, a: &crate::access::Access) -> Option<(&'static str, Color32)> {
    use crate::access::okabe::*;
    let pick = |plain: Color32, safe: Color32| if a.colour_blind { safe } else { plain };
    Some(match kind {
        Kind::Block => ("×", pick(Color32::from_rgb(255, 170, 120), ORANGE)),
        Kind::Door => ("»", pick(Color32::from_rgb(120, 220, 255), SKY_BLUE)),
        Kind::Spell => ("*", pick(Color32::from_rgb(230, 160, 255), PURPLE)),
        Kind::Item => ("+", pick(Color32::from_rgb(150, 230, 140), GREEN)),
        Kind::Creature => ("~", pick(Color32::from_rgb(235, 220, 150), YELLOW)),
        Kind::Foe => ("!", pick(Color32::from_rgb(255, 110, 110), VERMILLION)),
        Kind::Message => ("¶", Color32::from_rgb(255, 240, 200)),
        Kind::Turn | Kind::Step | Kind::Note => return None,
    })
}

/// Everyone playing, whose turn it is, and "Add Someone" (movement 1.4):
/// the new character joins at the start point on the next round.
fn party_panel(ui: &mut egui::Ui, g: &Game, new_name: &mut String, click: &mut Option<Command>) {
    let players = g.players();
    if players.len() > 1 {
        for (i, p) in players.iter().enumerate() {
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                ui.painter()
                    .circle_filled(rect.center(), 5.0, player_colour(ui.ctx(), i));
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
    ui.horizontal_wrapped(|ui| {
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
            RichText::new("Aim: arrows or the mouse move the cursor; Space or a click sends it, Esc cancels")
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
    let p = g.current_player();
    let m = |r: Rating| match p.mark(r) {
        Some(Mark::Up) => "+",
        Some(Mark::Down) => "-",
        None => "",
    };
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
                m(LifeForce),
                p.constitution,
                m(Constitution)
            )),
    );
    ui.add(
        egui::ProgressBar::new(p.power as f32 / 127.0)
            .fill(Color32::from_rgb(60, 80, 190))
            .text(format!(
                "Power {}{} (drifts toward Wisdom {}{})",
                p.power,
                m(Power),
                p.wisdom,
                m(Wisdom)
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
            m(Strength),
            p.dexterity,
            m(Dexterity),
            p.size,
            m(Size),
            p.speed,
            m(Speed),
            p.melee,
            m(Melee),
            p.missile,
            m(Missile),
            p.dodge,
            m(Dodge),
            p.parry,
            m(Parry),
            p.armor_skill,
            m(ArmorSkill)
        ))
        .weak(),
    )
    .on_hover_text("+ above average, - below, as the original's profile marks them.");
}

/// "Cast" / "Use" with a menu of targets: the caster and everyone in the
/// room (magic 2.4).
/// The cast or use goes at the square of the one picked (magic 2.4);
/// "yourself" leaves it on the caster's own.
fn target_menu(
    ui: &mut egui::Ui,
    g: &Game,
    label: &str,
    make: impl Fn(Option<(u8, u8)>) -> Command,
    (aim, start): (Aim, &mut Option<Aim>),
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
        // Client 4: the target cursor, as FIRE's.
        if ui.button("a square…").clicked() {
            *start = Some(aim);
            ui.close();
        }
        for (i, p) in g.players().iter().enumerate() {
            let who = Who::Player { player: i };
            let Location::Room { x, y, .. } = p.at else {
                continue;
            };
            if who != me && p.alive && p.at.room() == Some((region, room)) && ui.button(&p.name).clicked() {
                *click = Some(make(Some((x, y))));
                ui.close();
            }
        }
        for (_, c) in g.residents_in(region, room) {
            let name = format!("{} · {},{} · LF {}", c.name.trim(), c.x, c.y, c.life_force);
            if ui.button(name).clicked() {
                *click = Some(make(Some((c.x, c.y))));
                ui.close();
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
    aim: &mut Option<Aim>,
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
                    let chance = g.cast_chance(g.current(), t).unwrap_or(0);
                    ui.label(RichText::new(format!("cost {power}, {chance}%")).weak());
                    target_menu(
                        ui,
                        g,
                        "Cast",
                        |at| Command::Cast {
                            thing: t,
                            at,
                            target: None,
                        },
                        (Aim::Cast(t), &mut *aim),
                        click,
                    );
                }
                Some(ThingKind::MagicItem { on_use: true, .. }) => {
                    target_menu(
                        ui,
                        g,
                        "Use",
                        |at| Command::Use {
                            thing: t,
                            at,
                            target: None,
                        },
                        (Aim::Use(t), &mut *aim),
                        click,
                    );
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
/// An alignment's mark: its shape in the palette's colour (client 7).
fn alignment_mark(painter: &egui::Painter, c: egui::Pos2, r: f32, a: Alignment) {
    let colour = crate::access::Access::get(painter.ctx()).alignment_colour(a);
    crate::access::alignment_mark(painter, c, r, a, colour);
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
    let here: Vec<(usize, &Creature)> = g.residents_in(region, room).collect();
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
            alignment_mark(ui.painter(), rect.center(), 5.0, c.strategy.alignment);
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
    let colour = player_colour(painter.ctx(), player);
    painter.circle_stroke(c, r, Stroke::new((r * 0.25).max(1.0), colour));
    // The player's number, so colour isn't the only mark (client 7).
    if r >= 5.0 {
        let font = egui::FontId::proportional(r * 1.15);
        let n = (player + 1).to_string();
        painter.text(
            c + egui::vec2(1.0, 1.0),
            egui::Align2::CENTER_CENTER,
            &n,
            font.clone(),
            Color32::BLACK,
        );
        painter.text(c, egui::Align2::CENTER_CENTER, &n, font, colour);
    }
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

fn draw_map(
    tiles: &crate::tiles::Tiles,
    ui: &mut egui::Ui,
    g: &Game,
    px: u8,
    py: u8,
) -> Option<(Location, bool)> {
    let map = &g.adventure().world;
    let win = MapWindow::new(map_size(map), map.wraps, (px, py), MAP_VIEW);
    let (cols, rows) = (MAP_VIEW.0 as u32, MAP_VIEW.1 as u32);
    let (resp, painter, grid) = canvas(ui, cols, rows, tiles);
    painter.rect_filled(resp.rect, 0.0, ui.visuals().extreme_bg_color);
    for j in 0..rows {
        for i in 0..cols {
            let Some((mx, my)) = win.square(i as i16, j as i16) else {
                continue;
            };
            let t = map.tiles[my as usize][mx as usize];
            let pic = map.terrain.get(t as usize).map(|t| t.picture);
            tiles.paint(&painter, pic, grid.cell_rect(i, j));
        }
    }
    markers(&painter, g, |at| {
        let Location::World { x, y } = at else { return None };
        win.cell((x, y)).map(|(i, j)| grid.cell_rect(i as u32, j as u32))
    });
    // The square under the pointer, and whether it was clicked (client 4).
    hovered_cell(&resp, &grid).and_then(|(i, j)| {
        let (mx, my) = win.square(i as i16, j as i16)?;
        Some((Location::World { x: mx, y: my }, resp.clicked()))
    })
}

/// The world map's width and height in squares (it need not be square).
fn map_size(map: &acs_core::model::WorldMap) -> (i16, i16) {
    let h = map.tiles.len() as i16;
    let w = map.tiles.first().map_or(0, Vec::len) as i16;
    (w, h)
}

/// Which map square each cell of walk mode's map window shows (client 4).
/// A map that wraps is centred on the player and repeats across its edges;
/// one that doesn't stops at its edges, and is centred in the window, with
/// empty cells around it, where it is smaller than the window.
#[derive(Debug, Clone, Copy)]
struct MapWindow {
    size: (i16, i16),
    wraps: bool,
    /// The window's columns and rows.
    view: (i16, i16),
    /// The map square under the window's top-left cell (may lie outside).
    origin: (i16, i16),
}

impl MapWindow {
    fn new(size: (i16, i16), wraps: bool, (px, py): (u8, u8), (cols, rows): (u8, u8)) -> Self {
        let axis = |n: i16, p: u8, view: u8| {
            let (p, view) = (p as i16, view as i16);
            if wraps {
                p - view / 2
            } else if n <= view {
                -((view - n) / 2)
            } else {
                (p - view / 2).clamp(0, n - view)
            }
        };
        let origin = (axis(size.0, px, cols), axis(size.1, py, rows));
        MapWindow {
            size,
            wraps,
            view: (cols as i16, rows as i16),
            origin,
        }
    }

    /// The map square shown in cell `(i, j)`, if any.
    fn square(&self, i: i16, j: i16) -> Option<(u8, u8)> {
        let (w, h) = self.size;
        if w <= 0 || h <= 0 {
            return None;
        }
        let (x, y) = (self.origin.0 + i, self.origin.1 + j);
        if self.wraps {
            Some((x.rem_euclid(w) as u8, y.rem_euclid(h) as u8))
        } else {
            ((0..w).contains(&x) && (0..h).contains(&y)).then_some((x as u8, y as u8))
        }
    }

    /// The window cell that shows map square `(x, y)`, if it is in view
    /// (where the map wraps, the copy nearest the window's top left).
    fn cell(&self, (x, y): (u8, u8)) -> Option<(i16, i16)> {
        let (w, h) = self.size;
        let (mut i, mut j) = (x as i16 - self.origin.0, y as i16 - self.origin.1);
        if self.wraps && w > 0 && h > 0 {
            i = i.rem_euclid(w);
            j = j.rem_euclid(h);
        }
        let (cols, rows) = self.view;
        ((0..cols).contains(&i) && (0..rows).contains(&j)).then_some((i, j))
    }
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
) -> Option<(Location, bool)> {
    let a = g.adventure();
    let reg = g.region(region)?;
    let r = reg.rooms.get(room as usize)?;
    let scene = RoomScene::new(reg, r);
    let door = |x: u8, y: u8| g.area_door(Location::Room { region, room, x, y }).is_some();
    let (resp, painter, grid) = canvas(ui, r.width as u32, r.height as u32, tiles);
    painter.rect_filled(resp.rect, 0.0, ui.visuals().extreme_bg_color);
    for (k, sq) in scene.squares.iter().enumerate() {
        let (x, y) = ((k % r.width as usize) as u32, (k / r.width as usize) as u32);
        let cell = grid.cell_rect(x, y);
        match square_picture(a, reg, r, sq, door(x as u8, y as u8)) {
            // An area door drawn as an opening or floor: that alone.
            Some(pic) if Some(pic) != scene::shown(a, reg, r, sq).1 => tiles.paint(&painter, Some(pic), cell),
            None => tiles.paint(&painter, None, cell),
            Some(_) => {
                for pic in scene::layers(a, reg, r, sq) {
                    tiles.paint(&painter, Some(pic), cell);
                }
            }
        }
    }
    // A mark in the corner of each creature's square shows its alignment.
    for c in reg.residents.iter().filter(|c| c.in_use && c.room == room) {
        let cell = grid.cell_rect(c.x as u32, c.y as u32);
        let rad = (cell.width().min(cell.height()) * 0.12).max(2.0);
        let at = cell.right_top() + egui::vec2(-rad * 1.4, rad * 1.4);
        alignment_mark(&painter, at, rad, c.strategy.alignment);
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
    // The square under the pointer, and whether it was clicked (client 4).
    hovered_cell(&resp, &grid).map(|(x, y)| {
        let at = Location::Room {
            region,
            room,
            x: x as u8,
            y: y as u8,
        };
        (at, resp.clicked())
    })
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

    #[test]
    fn new_log_lines_are_announced_once_even_past_the_trim() {
        let a = adventure();
        let mut w = Walk::new(&a, 3, Rules::default());
        w.take_unannounced();
        w.note("one".into());
        w.note("two".into());
        assert_eq!(w.take_unannounced(), ["one", "two"]);
        assert!(w.take_unannounced().is_empty());
        for i in 0..LOG_MAX + 3 {
            w.note(format!("line {i}"));
        }
        w.record(&[]);
        let got = w.take_unannounced();
        assert_eq!(
            got.last().map(String::as_str),
            Some(format!("line {}", LOG_MAX + 2).as_str())
        );
        assert_eq!(got.len(), LOG_MAX);
    }

    #[test]
    fn music_events_become_cues_and_a_replays_are_dropped() {
        let a = adventure();
        let mut w = Walk::new(&a, 3, Rules::default());
        w.record(&[Event::Music { number: 14 }, Event::Music { number: 3 }]);
        assert_eq!(w.take_cues(), [14, 3]);
        assert!(w.take_cues().is_empty());
        w.record(&[Event::Music { number: 20 }]);
        let (_, text) = w.save_file(&a);
        assert!(Walk::from_save(&a, &text).unwrap().take_cues().is_empty());
    }

    #[test]
    fn casts_parse_a_target_or_a_square() {
        let cast = |at, target| Command::Cast {
            thing: 33,
            at,
            target,
        };
        assert_eq!(
            parse_commands("cast:33,cast:33:c2,cast:33:4:5").unwrap(),
            [
                cast(None, None),
                cast(None, Some(Who::Creature { region: 0, slot: 1 })),
                cast(Some((4, 5)), None),
            ]
        );
        assert_eq!(
            parse_commands("use:40:2:3").unwrap(),
            [Command::Use {
                thing: 40,
                at: Some((2, 3)),
                target: None
            }]
        );
        assert!(parse_commands("cast:33:4").is_err());
    }

    /// One region with one room 10 x 7 (inside 1..8 x 1..5); the player
    /// on its square 3,3.
    fn in_a_room() -> Walk {
        use acs_core::layout::world_map as wm;
        let mut f = common::Fixture::new(acs_core::Platform::Pc);
        f.header("CLICKS", "", "");
        f.thing(127, "EMPTY FLOOR", [0x00, 1, 0, 0]);
        f.region(common::RegionSpec {
            name: "HALL",
            rooms: vec![common::RoomSpec {
                name: "HALL",
                w: 10,
                h: 7,
                wall: 12,
                ..Default::default()
            }],
            ..Default::default()
        });
        let w = f.l.world_map;
        f.put(w + wm::TERRAIN, &[1 | 1 << 6, 0]);
        let a = acs_core::decode(&f.build()).expect("decodes").adventure;
        let mut walk = Walk::new(&a, 1, Rules::default());
        walk.place(sq(3, 3));
        walk
    }

    fn sq(x: u8, y: u8) -> Location {
        Location::Room {
            region: 1,
            room: 0,
            x,
            y,
        }
    }

    fn last(w: &Walk) -> Option<&Command> {
        w.game.log().last()
    }

    /// Client 4: a click beside the player is a step; its own square and
    /// farther squares, and squares of another room, send nothing.
    #[test]
    fn a_click_beside_the_player_steps_there() {
        let mut w = in_a_room();
        for at in [
            sq(3, 3),
            sq(5, 3),
            Location::World { x: 4, y: 3 },
            Location::Room {
                region: 1,
                room: 1,
                x: 4,
                y: 3,
            },
        ] {
            w.click(at);
            assert_eq!(last(&w), None, "{at:?}");
        }
        w.click(sq(4, 2));
        assert_eq!(last(&w), Some(&Command::Move { dir: Dir::NE }));
    }

    /// Client 4: with a drop waiting, the own square drops it there and a
    /// square beside it that way.
    #[test]
    fn a_click_places_a_waiting_drop() {
        let mut w = in_a_room();
        w.pending_drop = Some(5);
        w.click(sq(3, 3));
        assert_eq!(last(&w), Some(&Command::Drop { thing: 5, dir: None }));
        w.pending_drop = Some(5);
        w.click(sq(2, 3));
        assert_eq!(
            last(&w),
            Some(&Command::Drop {
                thing: 5,
                dir: Some(Dir::W)
            })
        );
        assert!(!w.waiting());
    }

    /// Client 4: the cursor follows the pointer onto inside squares only;
    /// a click on one sends the command there, a wall does nothing.
    #[test]
    fn the_target_cursor_follows_the_pointer_and_a_click_sends_it() {
        let mut w = in_a_room();
        assert!(w.start_aim(Aim::Cast(33)));
        w.hover(sq(8, 5));
        assert_eq!(w.pending_aim, Some((Aim::Cast(33), (8, 5))));
        w.hover(sq(9, 5));
        assert_eq!(w.pending_aim, Some((Aim::Cast(33), (8, 5))));
        w.click(sq(0, 0));
        assert_eq!(last(&w), None);
        w.click(sq(7, 1));
        assert_eq!(
            last(&w),
            Some(&Command::Cast {
                thing: 33,
                target: None,
                at: Some((7, 1))
            })
        );
        assert!(!w.waiting());
        // FIRE's cursor is the same one.
        w.start_fire(None, true);
        w.click(sq(1, 5));
        assert_eq!(
            last(&w),
            Some(&Command::Fire {
                target: Some((1, 5)),
                weapon: None
            })
        );
    }

    /// Client 4: on the map a step can cross the wraparound.
    #[test]
    fn map_offsets_take_the_short_way_round_where_it_wraps() {
        let at = |x, y| Location::World { x, y };
        assert_eq!(offset(at(0, 0), at(39, 1), (40, 40), true), Some((-1, 1)));
        assert_eq!(offset(at(0, 0), at(39, 1), (40, 40), false), Some((39, 1)));
        assert_eq!(offset(at(39, 39), at(0, 0), (40, 40), true), Some((1, 1)));
        assert_eq!(offset(at(5, 5), sq(5, 5), (40, 40), true), None);
        // A map wider than it is tall wraps on each axis by its own size.
        assert_eq!(offset(at(0, 0), at(9, 5), (10, 6), true), Some((-1, -1)));
    }

    /// Client 4: a map that doesn't wrap and is smaller than the window is
    /// shown once, centred, its own width wide (First Voyage's 10 x 6 map
    /// was drawn 6 wide and repeated, hiding the cave at 7,1).
    #[test]
    fn a_small_map_that_does_not_wrap_is_shown_once_and_whole() {
        let win = MapWindow::new((10, 6), false, (6, 2), (15, 10));
        let shown: Vec<(u8, u8)> = (0..10)
            .flat_map(|j| (0..15).map(move |i| (i, j)))
            .filter_map(|(i, j)| win.square(i, j))
            .collect();
        assert_eq!(shown.len(), 60, "every square once");
        assert!(shown.contains(&(7, 1)));
        // Centred: 2 empty columns left of it, 2 empty rows above.
        assert_eq!(win.square(2, 2), Some((0, 0)));
        assert_eq!(win.square(1, 2), None);
        assert_eq!(win.square(12, 7), None);
        // The player at 6,2 is drawn on 6,2, and clicking there finds it.
        let (i, j) = win.cell((6, 2)).unwrap();
        assert_eq!(win.square(i, j), Some((6, 2)));
    }

    /// Client 4: a large map that doesn't wrap scrolls with the player and
    /// stops at its edges; one that wraps centres on the player.
    #[test]
    fn the_map_window_follows_the_player() {
        let edge = MapWindow::new((40, 30), false, (39, 0), (15, 10));
        assert_eq!(edge.square(14, 0), Some((39, 0)));
        assert_eq!(edge.cell((39, 0)), Some((14, 0)));
        let wraps = MapWindow::new((40, 30), true, (0, 0), (15, 10));
        assert_eq!(wraps.cell((0, 0)), Some((7, 5)));
        assert_eq!(wraps.square(0, 0), Some((33, 25)));
    }
}
