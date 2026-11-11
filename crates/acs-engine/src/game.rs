// SPDX-License-Identifier: GPL-3.0-or-later

//! Game state, turns and the command loop.

use crate::rng::Rng;
use crate::{Command, Event, Location, Rules};
use acs_core::model::{Adventure, Region, Room, Thing};
use serde::{Deserialize, Serialize};

/// Speed of a character made by [`PlayerSetup::default`]: the top of the
/// manual's normal starting range (6-7).
pub const DEFAULT_SPEED: u8 = 7;

/// Strength, Constitution and Life Force of a character made by
/// [`PlayerSetup::default`]: the top of the manual's normal range (10-11).
pub const DEFAULT_TRAIT: u8 = 11;

fn default_trait() -> u8 {
    DEFAULT_TRAIT
}

/// A new character. Trait generation is a later rule set; for now a player
/// is a name, the traits movement and carrying need, and what it carries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerSetup {
    pub name: String,
    pub speed: u8,
    #[serde(default = "default_trait")]
    pub strength: u8,
    #[serde(default = "default_trait")]
    pub constitution: u8,
    /// Starts equal to Constitution for new characters (inventory 6.1).
    #[serde(default = "default_trait")]
    pub life_force: u8,
    /// Master thing numbers carried at the start, one entry per item
    /// (thing 81, Gold, goes in `gold` instead).
    #[serde(default)]
    pub possessions: Vec<u8>,
    #[serde(default)]
    pub gold: u16,
}

impl Default for PlayerSetup {
    fn default() -> Self {
        PlayerSetup {
            name: "PLAYER 1".into(),
            speed: DEFAULT_SPEED,
            strength: DEFAULT_TRAIT,
            constitution: DEFAULT_TRAIT,
            life_force: DEFAULT_TRAIT,
            possessions: Vec::new(),
            gold: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Player {
    pub name: String,
    pub speed: u8,
    pub strength: u8,
    pub constitution: u8,
    pub life_force: u8,
    /// What is left of the movement bar this turn.
    pub moves_left: u8,
    /// Where the bar started this turn (Speed after any slowdown).
    pub turn_moves: u8,
    pub at: Location,
    /// Master thing numbers, one entry per item carried, in pick-up order.
    pub possessions: Vec<u8>,
    /// Pieces of Gold (thing 81), 0..=25,000.
    pub gold: u16,
    /// Readied weapon (melee or missile) and armor, master thing numbers.
    pub weapon: Option<u8>,
    pub armor: Option<u8>,
}

impl Player {
    pub fn owns(&self, thing: u8) -> bool {
        self.count(thing) > 0
    }

    /// How many of `thing` the player owns (gold pieces for thing 81).
    pub fn count(&self, thing: u8) -> u16 {
        if thing == crate::inventory::GOLD {
            self.gold
        } else {
            self.possessions.iter().filter(|&&t| t == thing).count() as u16
        }
    }
}

/// A game in progress. Cloning it is a snapshot (what an autosave keeps).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Game {
    /// The adventure as play has changed it (things picked up, obstacles gone).
    pub(crate) adv: Adventure,
    pub(crate) rules: Rules,
    pub(crate) rng: Rng,
    pub(crate) players: Vec<Player>,
    pub(crate) current: usize,
    seed: u64,
    first: PlayerSetup,
    log: Vec<Command>,
}

impl Game {
    /// Starts a game with one character at the adventure's start; returns the
    /// game and the opening events (the first turn starting).
    pub fn new(adv: Adventure, seed: u64, rules: Rules, first: PlayerSetup) -> (Self, Vec<Event>) {
        let mut g = Game {
            adv,
            rules,
            rng: Rng::new(seed),
            players: Vec::new(),
            current: 0,
            seed,
            first: first.clone(),
            log: Vec::new(),
        };
        let mut ev = Vec::new();
        g.add_player(first, &mut ev);
        g.start_turn(&mut ev);
        (g, ev)
    }

    /// Rebuilds a game from its seed, first player and command log.
    pub fn replay(adv: Adventure, seed: u64, rules: Rules, first: PlayerSetup, commands: &[Command]) -> Self {
        let (mut g, _) = Game::new(adv, seed, rules, first);
        for c in commands {
            g.apply(c.clone());
        }
        g
    }

    /// Runs one command for the player whose turn it is.
    pub fn apply(&mut self, cmd: Command) -> Vec<Event> {
        self.log.push(cmd.clone());
        let mut ev = Vec::new();
        match cmd {
            Command::Wait => self.end_turn(&mut ev),
            Command::Move { dir } => self.step(dir, &mut ev),
            Command::AddPlayer { setup } => self.add_player(setup, &mut ev),
            Command::Drop { thing, dir } => self.drop_thing(thing, dir, &mut ev),
            Command::DropGold { amount, dir } => self.drop_gold(amount, dir, &mut ev),
            Command::Ready { thing } => self.ready(thing, &mut ev),
        }
        ev
    }

    pub fn adventure(&self) -> &Adventure {
        &self.adv
    }
    pub fn rules(&self) -> Rules {
        self.rules
    }
    pub fn seed(&self) -> u64 {
        self.seed
    }
    pub fn first_player(&self) -> &PlayerSetup {
        &self.first
    }
    pub fn log(&self) -> &[Command] {
        &self.log
    }
    pub fn players(&self) -> &[Player] {
        &self.players
    }
    /// Index of the player whose turn it is.
    pub fn current(&self) -> usize {
        self.current
    }
    pub fn current_player(&self) -> &Player {
        &self.players[self.current]
    }

    /// Test and tool hook: puts a player somewhere without a move. Not a
    /// command, so it is not replayed.
    pub fn place(&mut self, player: usize, at: Location) {
        self.players[player].at = at;
    }

    /// Test and tool hook: gives a player a thing. Not replayed.
    pub fn give(&mut self, player: usize, thing: u8) {
        let p = &mut self.players[player];
        if thing == crate::inventory::GOLD {
            p.gold = p.gold.saturating_add(1).min(crate::inventory::GOLD_LIMIT);
        } else {
            p.possessions.push(thing);
        }
    }

    fn add_player(&mut self, setup: PlayerSetup, ev: &mut Vec<Event>) {
        let s = self.adv.world.start;
        let at = Location::World { x: s.x, y: s.y };
        let mut possessions = setup.possessions;
        let mut gold = setup.gold;
        possessions.retain(|&t| {
            if t == crate::inventory::GOLD {
                gold = gold.saturating_add(1);
            }
            t != crate::inventory::GOLD
        });
        self.players.push(Player {
            name: setup.name,
            speed: setup.speed,
            strength: setup.strength,
            constitution: setup.constitution,
            life_force: setup.life_force,
            moves_left: setup.speed,
            turn_moves: setup.speed,
            at,
            possessions,
            gold: gold.min(crate::inventory::GOLD_LIMIT),
            weapon: None,
            armor: None,
        });
        ev.push(Event::PlayerAdded {
            player: self.players.len() - 1,
            at,
        });
    }

    /// Uses one square of the current player's bar; ends the turn when empty.
    pub(crate) fn spend(&mut self, ev: &mut Vec<Event>) {
        let p = &mut self.players[self.current];
        p.moves_left = p.moves_left.saturating_sub(1);
        if p.moves_left == 0 {
            self.end_turn(ev);
        }
    }

    pub(crate) fn end_turn(&mut self, ev: &mut Vec<Event>) {
        ev.push(Event::TurnEnded { player: self.current });
        self.current = (self.current + 1) % self.players.len();
        self.start_turn(ev);
    }

    /// Fills the current player's bar from Speed after any slowdown
    /// (inventory 3.3, 3.4).
    fn start_turn(&mut self, ev: &mut Vec<Event>) {
        let moves = self.players[self.current].effective_speed(&self.adv);
        let p = &mut self.players[self.current];
        p.moves_left = moves;
        p.turn_moves = moves;
        ev.push(Event::TurnStarted {
            player: self.current,
            moves,
        });
    }

    // ---- lookups ---------------------------------------------------------

    pub(crate) fn region(&self, number: u8) -> Option<&Region> {
        self.adv.regions.iter().find(|r| r.number == number)
    }

    pub(crate) fn region_mut(&mut self, number: u8) -> Option<&mut Region> {
        self.adv.regions.iter_mut().find(|r| r.number == number)
    }

    /// A live room.
    pub(crate) fn room(&self, region: u8, room: u8) -> Option<&Room> {
        self.region(region)?
            .rooms
            .get(room as usize)
            .filter(|r| !r.deleted)
    }

    pub(crate) fn thing(&self, number: u8) -> Option<&Thing> {
        self.adv.things.get((number as usize).checked_sub(1)?)
    }

    /// Text of a short message slot as one line. The stored lines are split
    /// by position (words run across line ends), so they are joined back
    /// without a separator and runs of spaces collapsed.
    pub(crate) fn short_message(&self, slot: u8) -> Option<String> {
        let s = self.adv.short_messages.get((slot as usize).checked_sub(1)?)?;
        let joined: String = s.lines().collect();
        let t = joined.split_whitespace().collect::<Vec<_>>().join(" ");
        (!t.is_empty()).then_some(t)
    }

    /// A thing's message from a [`MessageRef`](acs_core::model::MessageRef),
    /// if it is set to be shown.
    pub(crate) fn message(&self, m: &Option<acs_core::model::MessageRef>) -> Option<String> {
        m.as_ref()
            .filter(|m| m.shown)
            .and_then(|m| self.short_message(m.slot))
    }
}
