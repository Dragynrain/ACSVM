// SPDX-License-Identifier: GPL-3.0-or-later

//! Game state, turns and the command loop.

use crate::magic::{PendingSummon, Vitals};
use crate::rng::Rng;
use crate::{Command, Event, Location, Rules, Who};
use acs_core::model::ThingId;
use acs_core::model::{Adventure, MagicDefense, Region, Room, Thing};
use serde::{Deserialize, Serialize};

/// Speed of a character made by [`PlayerSetup::default`]: the top of the
/// manual's normal starting range (6-7).
pub const DEFAULT_SPEED: u8 = 7;

/// Strength, Constitution and Life Force of a character made by
/// [`PlayerSetup::default`]: the top of the manual's normal range (10-11).
pub const DEFAULT_TRAIT: u8 = 11;

/// Dodge, Parry, Melee and Armor skill of a default character: the middle
/// of the manual's 25-35 (magic 8).
pub const DEFAULT_SKILL: u8 = 30;
/// Living characters that can play at once (movement 1.4).
pub const MAX_PLAYERS: usize = 4;
/// Longest character name (movement 1.5).
pub const NAME_MAX: usize = 15;
/// Missile skill of a default character: the middle of 10-20.
pub const DEFAULT_MISSILE: u8 = 15;

fn default_trait() -> u8 {
    DEFAULT_TRAIT
}
fn default_skill() -> u8 {
    DEFAULT_SKILL
}
fn default_missile() -> u8 {
    DEFAULT_MISSILE
}
fn yes() -> bool {
    true
}
fn no_defense() -> MagicDefense {
    MagicDefense::None
}

/// A character joining with its ratings given: a saved character, or one
/// made by a test or tool. [`PlayerSetup::roll`] makes a new character
/// (characters 1, 2); the defaults are only for tests and tools.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerSetup {
    pub name: String,
    pub speed: u8,
    #[serde(default = "default_trait")]
    pub strength: u8,
    #[serde(default = "default_trait")]
    pub constitution: u8,
    /// Kept as given on joining (characters 3.1); a new character's is its
    /// Constitution.
    #[serde(default = "default_trait")]
    pub life_force: u8,
    /// Master thing numbers carried at the start, one entry per item
    /// (thing 81, Gold, goes in `gold` instead).
    #[serde(default)]
    pub possessions: Vec<ThingId>,
    #[serde(default)]
    pub gold: u16,
    #[serde(default = "default_trait")]
    pub dexterity: u8,
    #[serde(default = "default_trait")]
    pub wisdom: u8,
    /// Kept as given on joining (characters 3.1); `None` is its Wisdom, as
    /// a new character's.
    #[serde(default)]
    pub power: Option<u8>,
    #[serde(default = "default_trait")]
    pub size: u8,
    #[serde(default = "default_skill")]
    pub dodge: u8,
    #[serde(default = "default_skill")]
    pub parry: u8,
    #[serde(default = "default_skill")]
    pub armor_skill: u8,
    #[serde(default = "default_skill")]
    pub melee: u8,
    #[serde(default = "default_missile")]
    pub missile: u8,
    #[serde(default = "no_defense")]
    pub magic_defense: MagicDefense,
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
            dexterity: DEFAULT_TRAIT,
            wisdom: DEFAULT_TRAIT,
            power: None,
            size: DEFAULT_TRAIT,
            dodge: DEFAULT_SKILL,
            parry: DEFAULT_SKILL,
            armor_skill: DEFAULT_SKILL,
            melee: DEFAULT_SKILL,
            missile: DEFAULT_MISSILE,
            magic_defense: MagicDefense::None,
        }
    }
}

/// Who a game starts with: one with its ratings given, or a new character
/// rolled from the game's generator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Newcomer {
    Join(PlayerSetup),
    Create { create: String },
}

impl From<PlayerSetup> for Newcomer {
    fn from(s: PlayerSetup) -> Self {
        Newcomer::Join(s)
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
    pub possessions: Vec<ThingId>,
    /// Pieces of Gold (thing 81), 0..=25,000.
    pub gold: u16,
    /// Readied weapon (melee or missile) and armor, master thing numbers.
    pub weapon: Option<ThingId>,
    pub armor: Option<ThingId>,
    #[serde(default = "default_trait")]
    pub dexterity: u8,
    #[serde(default = "default_trait")]
    pub wisdom: u8,
    #[serde(default = "default_trait")]
    pub power: u8,
    #[serde(default = "default_trait")]
    pub size: u8,
    #[serde(default = "default_skill")]
    pub dodge: u8,
    #[serde(default = "default_skill")]
    pub parry: u8,
    #[serde(default = "default_skill")]
    pub armor_skill: u8,
    #[serde(default = "default_skill")]
    pub melee: u8,
    #[serde(default = "default_missile")]
    pub missile: u8,
    #[serde(default = "no_defense")]
    pub magic_defense: MagicDefense,
    /// False once Life Force reached 0 (magic 7.4).
    #[serde(default = "yes")]
    pub alive: bool,
}

impl Player {
    pub fn owns(&self, thing: ThingId) -> bool {
        self.count(thing) > 0
    }

    /// How many of `thing` the player owns (gold pieces for thing 81).
    pub fn count(&self, thing: ThingId) -> u16 {
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
    first: Newcomer,
    log: Vec<Command>,
    /// Region slots holding random creatures (creatures 2.4, 3.3).
    #[serde(default)]
    pub(crate) random: Vec<(u8, usize)>,
    /// Rooms any player has been in this round (creatures 2.2): the rooms
    /// whose creatures take the round's creature phase.
    #[serde(default)]
    pub(crate) turn_rooms: Vec<(u8, u8)>,
    /// Doors players left a room by this round: region, room, door square
    /// (creatures 6.4).
    #[serde(default)]
    pub(crate) turn_exits: Vec<(u8, u8, u8, u8)>,
    /// Temporary Life Force and turn counts per individual (magic 5).
    #[serde(default)]
    pub(crate) vitals: Vec<Vitals>,
    /// Summonses waiting for the next player turn (magic 3.2).
    #[serde(default)]
    pub(crate) summons: Vec<PendingSummon>,
    /// Every player is dead (magic 7.4).
    #[serde(default)]
    pub(crate) over: bool,
    /// Who moves when (creatures 2.2).
    #[serde(default)]
    pub(crate) sched: Schedule,
    /// Temporary changes still to be taken back (magic 5.2).
    #[serde(default)]
    pub(crate) reversals: Vec<crate::magic::Reversal>,
    /// The store visit in progress (stores 3).
    #[serde(default)]
    pub(crate) store: Option<crate::stores::StoreVisit>,
    /// Tool placements made so far, for the save (saves 1.3).
    #[serde(default)]
    placed: Vec<crate::save::Placement>,
    /// The world map's own region: its encounter areas and their
    /// creatures (encounters 3.1). Not in the adventure's region list.
    #[serde(default = "crate::encounters::map_region")]
    pub(crate) map: Region,
    /// Each encounter area's entrance square while it is open (7.1).
    #[serde(default)]
    pub(crate) entrances: [Option<(u8, u8)>; crate::encounters::AREAS as usize],
    /// Moves a player had left at a map door into a room, for the start of
    /// its next turn (creatures 2.2.4, movement 2.3).
    #[serde(default)]
    pub(crate) carried: Vec<(usize, u8)>,
    /// "Add Someone" requests waiting for the round's end, in the order
    /// asked (characters 6.2).
    #[serde(default)]
    pub(crate) adds: Vec<Newcomer>,
    /// The RETIRE waiting for the round's end (characters 7.1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) retire: Option<crate::characters::Retirement>,
}

/// The turn scheduler (creatures 2.2): cycles of region visits, each up to
/// `VISIT_ROUNDS` rounds, each round a list of individuals built at its
/// start.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Schedule {
    /// Regions still to visit this cycle, lowest first (0 the world map).
    pub marks: Vec<u8>,
    /// The region being visited.
    pub region: u8,
    /// Rounds of this visit so far.
    pub rounds: u8,
    /// This round's turn list and the next entry to go.
    pub list: Vec<Who>,
    pub pos: usize,
    /// Rounds since the game began (magic 5.2 counts in them).
    pub round: u32,
    /// The rooms on this round's list (creatures 2.2, 2.4).
    pub rooms: Vec<u8>,
    /// Living players (over every region) at the round's start, for the
    /// round-end message (creatures 2.2.5, characters 6.3).
    #[serde(default)]
    pub living: usize,
}

/// The rooms of `region` with a living player, at most `ROUND_ROOMS`, in
/// player order (creatures 2.2).
fn rooms_of(players: &[Player], region: u8) -> Vec<u8> {
    let mut rooms = Vec::new();
    for p in players.iter().filter(|p| p.alive) {
        if let Some((rg, rm)) = p.at.room() {
            if rg == region && !rooms.contains(&rm) && rooms.len() < ROUND_ROOMS {
                rooms.push(rm);
            }
        }
    }
    rooms
}

/// The newcomer an "Add Someone" command brings, if it is one.
fn newcomer(cmd: Command) -> Option<Newcomer> {
    match cmd {
        Command::AddPlayer { setup } => Some(Newcomer::Join(setup)),
        Command::CreatePlayer { name } => Some(Newcomer::Create { create: name }),
        _ => None,
    }
}

/// Rounds a region visit lasts at most (creatures 2.2).
pub const VISIT_ROUNDS: u8 = 10;
/// Rooms a round's list takes at most (creatures 2.2).
pub const ROUND_ROOMS: usize = 4;

impl Game {
    /// Starts a game with one character at the adventure's start; returns the
    /// game and the opening events (the first turn starting).
    pub fn new(adv: Adventure, seed: u64, rules: Rules, first: impl Into<Newcomer>) -> (Self, Vec<Event>) {
        let first = first.into();
        // Encounters 3.4: the areas' items, once, from the world map.
        let map = crate::encounters::seeded_map_region(&adv.world);
        let mut g = Game {
            adv,
            rules,
            rng: Rng::new(seed),
            players: Vec::new(),
            current: 0,
            seed,
            first: first.clone(),
            log: Vec::new(),
            random: Vec::new(),
            turn_rooms: Vec::new(),
            turn_exits: Vec::new(),
            vitals: Vec::new(),
            summons: Vec::new(),
            over: false,
            sched: Schedule::default(),
            reversals: Vec::new(),
            store: None,
            placed: Vec::new(),
            map,
            entrances: Default::default(),
            carried: Vec::new(),
            adds: Vec::new(),
            retire: None,
        };
        let mut ev = Vec::new();
        // The first player joins as given: the front end has asked again
        // for a refused name (characters 5.7) before starting the game.
        g.join(first, &mut ev);
        g.begin_cycle();
        g.next_turn(&mut ev);
        (g, ev)
    }

    /// Rebuilds a game from its seed, first player and command log.
    pub fn replay(
        adv: Adventure,
        seed: u64,
        rules: Rules,
        first: impl Into<Newcomer>,
        commands: &[Command],
    ) -> Self {
        let (mut g, _) = Game::new(adv, seed, rules, first);
        for c in commands {
            g.apply(c.clone());
        }
        g
    }

    /// Runs one command for the player whose turn it is.
    /// Once every player is dead (magic 7.4) only adding a player is
    /// accepted, and it starts a new cycle (creatures 2.2.6).
    pub fn apply(&mut self, cmd: Command) -> Vec<Event> {
        let mut ev = Vec::new();
        if self.over {
            if !matches!(cmd, Command::AddPlayer { .. } | Command::CreatePlayer { .. }) {
                return ev;
            }
            self.log.push(cmd.clone());
            // The add-a-player screen of nobody in play: at once.
            if let Some(n) = newcomer(cmd) {
                if self.name_ok(&n, &mut ev) {
                    self.join(n, &mut ev);
                }
            }
            // A refused name leaves nobody in play: still waiting.
            if self.players.iter().any(|p| p.alive) {
                self.over = false;
                self.begin_cycle();
                self.next_turn(&mut ev);
            }
            return ev;
        }
        self.log.push(cmd.clone());
        // Stores 3.8: anything but buying and selling ends a store visit.
        if !matches!(cmd, Command::Buy { .. } | Command::Sell { .. } | Command::Idle) {
            self.leave_store(&mut ev);
        }
        match cmd {
            Command::Wait => self.wait_out(&mut ev),
            Command::Idle => self.idle(&mut ev),
            Command::Move { dir } => self.step(dir, &mut ev),
            Command::AddPlayer { setup } => self.request_add(Newcomer::Join(setup), &mut ev),
            Command::CreatePlayer { name } => self.request_add(Newcomer::Create { create: name }, &mut ev),
            Command::Drop { thing, dir } => self.drop_thing(thing, dir, &mut ev),
            Command::DropGold { amount, dir } => self.drop_gold(amount, dir, &mut ev),
            Command::Ready { thing } => self.ready(thing, &mut ev),
            Command::Cast { thing, target } => self.cast_command(thing, target, &mut ev),
            Command::Use { thing, target } => self.use_command(thing, target, &mut ev),
            Command::Fire { target, weapon } => self.fire_command(target, weapon, &mut ev),
            Command::Retire { save } => self.request_retire(save, &mut ev),
            Command::Buy { thing } => self.buy(thing, &mut ev),
            Command::Sell { thing } => self.sell(thing, &mut ev),
            Command::LeaveStore => {}
        }
        // Magic 7.5: a player who died during their own turn ends it.
        if !self.over && !self.players[self.current].alive {
            self.end_turn(&mut ev);
        }
        ev
    }

    /// True once no player is left alive.
    pub fn is_over(&self) -> bool {
        self.over
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
    pub fn first_player(&self) -> &Newcomer {
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
    /// command, so [`Game::replay`] doesn't redo it, but it is kept for the
    /// save, whose replay does (saves 1.3). The schedule starts afresh from
    /// the current player's region, the current turn going on, as if a round
    /// had just begun with everyone where they now stand.
    pub fn place(&mut self, player: usize, at: Location) {
        self.placed.push(crate::save::Placement {
            after: self.log.len(),
            player,
            at,
        });
        self.players[player].at = at;
        self.reschedule();
    }

    /// The tool placements made so far (saves 1.3).
    pub fn placements(&self) -> &[crate::save::Placement] {
        &self.placed
    }

    /// See `place`.
    fn reschedule(&mut self) {
        let me = Who::Player { player: self.current };
        let region = self.players[self.current].at.region();
        let mut marks = self.player_regions();
        marks.retain(|&r| r != region);
        self.sched.marks = marks;
        self.sched.region = region;
        self.sched.rounds = 0;
        self.begin_round();
        self.sched.pos = self.sched.list.iter().position(|&w| w == me).map_or(0, |i| i + 1);
    }

    /// Regions where a living player stands, lowest first.
    fn player_regions(&self) -> Vec<u8> {
        let mut r: Vec<u8> = self
            .players
            .iter()
            .filter(|p| p.alive)
            .map(|p| p.at.region())
            .collect();
        r.sort_unstable();
        r.dedup();
        r
    }

    /// Creatures 2.2: a new cycle marks the regions with a living player and
    /// begins the first visit.
    fn begin_cycle(&mut self) {
        self.sched.marks = self.player_regions();
        self.next_visit();
    }

    /// Begins the next marked region's visit; false if none is left.
    fn next_visit(&mut self) -> bool {
        if self.sched.marks.is_empty() {
            return false;
        }
        self.sched.region = self.sched.marks.remove(0);
        self.sched.rounds = 0;
        self.begin_round();
        true
    }

    /// Builds the round's list for the region being visited: on the map its
    /// players; in a room region the rooms with a living player (at most 4,
    /// in player order), each with its players and then its creatures.
    fn begin_round(&mut self) {
        let region = self.sched.region;
        let mut list = Vec::new();
        if region == 0 {
            // Encounters 6.1: the open map's players, then each area in
            // use with its players and then its creatures.
            for (i, p) in self.players.iter().enumerate() {
                if p.alive && matches!(p.at, Location::World { .. }) {
                    list.push(Who::Player { player: i });
                }
            }
            for area in 1..=crate::encounters::AREAS {
                let here: Vec<usize> = (0..self.players.len())
                    .filter(|&i| self.players[i].alive && self.players[i].at.room() == Some((0, area)))
                    .collect();
                if here.is_empty() {
                    continue;
                }
                list.extend(here.into_iter().map(|player| Who::Player { player }));
                for (slot, c) in self.map.residents.iter().enumerate() {
                    if c.in_use && c.room == area {
                        list.push(Who::Creature { region: 0, slot });
                    }
                }
            }
        } else {
            let mut rooms: Vec<u8> = Vec::new();
            for p in self.players.iter().filter(|p| p.alive) {
                if let Some((rg, rm)) = p.at.room() {
                    if rg == region && !rooms.contains(&rm) && rooms.len() < ROUND_ROOMS {
                        rooms.push(rm);
                    }
                }
            }
            for &rm in &rooms {
                for (i, p) in self.players.iter().enumerate() {
                    if p.alive && p.at.room() == Some((region, rm)) {
                        list.push(Who::Player { player: i });
                    }
                }
                if let Some(reg) = self.region(region) {
                    for (slot, c) in reg.residents.iter().enumerate() {
                        if c.in_use && c.room == rm {
                            list.push(Who::Creature { region, slot });
                        }
                    }
                }
            }
        }
        self.sched.list = list;
        self.sched.rooms = if region == 0 {
            Vec::new()
        } else {
            rooms_of(&self.players, region)
        };
        self.sched.pos = 0;
        self.sched.living = self.living();
    }

    /// Living players over every region (characters 6.1: the engine's
    /// alive flag stands for the slot's first byte).
    fn living(&self) -> usize {
        self.players.iter().filter(|p| p.alive).count()
    }

    /// Runs the list on: creatures take their turns, and the next living
    /// player's turn starts (and waits for commands). A finished list ends
    /// the round.
    pub(crate) fn next_turn(&mut self, ev: &mut Vec<Event>) {
        // Every round with a living player has one on its list, so this
        // ends; the guard is for safety only.
        for _ in 0..10_000 {
            if self.over {
                return;
            }
            if self.sched.pos >= self.sched.list.len() {
                self.end_round(ev);
                continue;
            }
            let who = self.sched.list[self.sched.pos];
            self.sched.pos += 1;
            match who {
                Who::Player { player } => {
                    if self.players.get(player).is_some_and(|p| p.alive) {
                        self.current = player;
                        self.start_turn(ev);
                        return;
                    }
                }
                Who::Creature { region, slot } => {
                    if self.creature(region, slot).is_some() {
                        self.creature_turn(region, slot, ev);
                    }
                }
            }
        }
    }

    /// The end of a round (creatures 2.2.5): the round counter and what
    /// falls due on it (magic 5.2), random creatures left behind (2.4);
    /// the round-end message when the living count changed either way
    /// (characters 6.3); the retirement and then the add requests
    /// (characters 7.1, 6.2); with nobody in play the game waits for a new
    /// player (2.2.6, characters 7.6); then the next round of this
    /// visit, or the next visit (2.2.2). A death alone ends no visit; the
    /// message's answers are the front end's.
    fn end_round(&mut self, ev: &mut Vec<Event>) {
        self.sched.round += 1;
        self.reversals_due(ev);
        self.left_behind(ev);
        self.turn_exits.clear();
        self.turn_rooms.clear();
        self.sched.rounds += 1;
        let (before, after) = (self.sched.living, self.living());
        if before != after {
            ev.push(Event::LivingChanged { before, after });
        }
        self.take_retirement(ev);
        self.take_adds(ev);
        if !self.players.iter().any(|p| p.alive) {
            self.over = true;
            ev.push(Event::AllPlayersDead);
            return;
        }
        let region = self.sched.region;
        let here = self.players.iter().any(|p| p.alive && p.at.region() == region);
        if self.sched.rounds >= VISIT_ROUNDS || !here {
            if !self.next_visit() {
                self.begin_cycle();
            }
        } else {
            self.begin_round();
        }
    }

    /// Test and tool hook: sets a player's Power. Not replayed.
    pub fn set_power(&mut self, player: usize, power: u8) {
        self.players[player].power = power;
    }

    /// Test and tool hook: sets a player's Life Force (a joining player
    /// can't start below its Constitution, magic 5.1). Not replayed.
    pub fn set_life_force(&mut self, player: usize, life_force: u8) {
        self.players[player].life_force = life_force;
    }

    /// Test and tool hook: gives a player a thing. Not replayed.
    pub fn give(&mut self, player: usize, thing: ThingId) {
        let p = &mut self.players[player];
        if thing == crate::inventory::GOLD {
            p.gold = p.gold.saturating_add(1).min(crate::inventory::GOLD_LIMIT);
        } else {
            p.possessions.push(thing);
        }
    }

    /// False, with `NameRefused`, when name entry would ask again for the
    /// newcomer's name (characters 5.6, 5.7).
    fn name_ok(&self, n: &Newcomer, ev: &mut Vec<Event>) -> bool {
        let name = match n {
            Newcomer::Join(s) => &s.name,
            Newcomer::Create { create } => create,
        };
        let ok = !crate::characters::name_refused(name);
        if !ok {
            ev.push(Event::NameRefused);
        }
        ok
    }

    /// "Add Someone" in play (characters 6.2): taken at once, whoever
    /// plays, with no move used and no draw; handled at the round's end.
    fn request_add(&mut self, n: Newcomer, ev: &mut Vec<Event>) {
        if self.name_ok(&n, ev) {
            self.adds.push(n);
            ev.push(Event::AddRequested);
        }
    }

    /// The round's add requests, in the order asked (characters 6.2): each
    /// refused, with no draw, while `MAX_PLAYERS` live (6.1); otherwise
    /// rolled or set up then, and added.
    fn take_adds(&mut self, ev: &mut Vec<Event>) {
        for n in std::mem::take(&mut self.adds) {
            if self.living() >= MAX_PLAYERS {
                ev.push(Event::AddRefused);
            } else {
                self.join(n, ev);
            }
        }
    }

    /// Adds a newcomer: rolled from the game's generator (characters 1) or
    /// joining as given.
    fn join(&mut self, n: Newcomer, ev: &mut Vec<Event>) {
        let setup = match n {
            Newcomer::Join(setup) => setup,
            Newcomer::Create { create } => PlayerSetup::roll(&create, &mut self.rng),
        };
        self.add_player(setup, ev);
    }

    /// "Add Someone" (movement 1.2, 1.4, 1.5): the new character starts at
    /// the adventure's start and joins the next round (creatures 2.2),
    /// keeping its ratings as given (characters 3.1).
    fn add_player(&mut self, setup: PlayerSetup, ev: &mut Vec<Event>) {
        let mut setup = setup;
        setup.name = crate::characters::fold_name(&setup.name);
        let s = self.adv.world.start;
        let at = Location::World { x: s.x, y: s.y };
        // Characters 6.1, creatures 2.2.6: the newcomer takes the lowest
        // free slot, a dead player's included, and that slot's number.
        let slot = self.players.iter().position(|p| !p.alive);
        let mut possessions = setup.possessions;
        let mut gold = setup.gold;
        possessions.retain(|&t| {
            if t == crate::inventory::GOLD {
                gold = gold.saturating_add(1);
            }
            t != crate::inventory::GOLD
        });
        let player = Player {
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
            dexterity: setup.dexterity,
            wisdom: setup.wisdom,
            power: setup.power.unwrap_or(setup.wisdom),
            size: setup.size,
            dodge: setup.dodge,
            parry: setup.parry,
            armor_skill: setup.armor_skill,
            melee: setup.melee,
            missile: setup.missile,
            magic_defense: setup.magic_defense,
            alive: true,
        };
        let index = match slot {
            Some(i) => {
                // 6.1: the slot is cleared before anything is put in it.
                self.players[i] = player;
                self.carried.retain(|&(q, _)| q != i);
                self.forget_vitals(Who::Player { player: i });
                i
            }
            None => {
                self.players.push(player);
                self.players.len() - 1
            }
        };
        ev.push(Event::PlayerAdded { player: index, at });
    }

    /// Uses one square of the current player's bar; ends the turn when empty.
    pub(crate) fn spend(&mut self, ev: &mut Vec<Event>) {
        let p = &mut self.players[self.current];
        p.moves_left = p.moves_left.saturating_sub(1);
        if p.moves_left == 0 {
            self.end_turn(ev);
        }
    }

    /// "Wait": the rest of the turn's moves go as idle moves, each with its
    /// healing roll (magic 5.3), and the turn ends.
    fn wait_out(&mut self, ev: &mut Vec<Event>) {
        let p = &self.players[self.current];
        let (left, full) = (p.moves_left, p.turn_moves);
        self.idle_moves(Who::Player { player: self.current }, left, full, ev);
        if self.players[self.current].alive {
            self.end_turn(ev);
        }
    }

    /// The drain of one step (movement 2.4): an idle move with its healing
    /// roll (magic 5.3), the turn ending with the last move. A turn with
    /// no moves left ends at once; in a store nothing happens.
    fn idle(&mut self, ev: &mut Vec<Event>) {
        if self.store.is_some() {
            return;
        }
        let p = &self.players[self.current];
        if p.moves_left == 0 {
            self.end_turn(ev);
            return;
        }
        let full = p.turn_moves;
        self.idle_moves(Who::Player { player: self.current }, 1, full, ev);
        if self.players[self.current].alive {
            self.spend(ev);
        }
    }

    /// Ends the current player's turn; the schedule goes on, the round
    /// finishing even after a death (creatures 2.2.5, magic 7.4).
    pub(crate) fn end_turn(&mut self, ev: &mut Vec<Event>) {
        let p = self.current;
        ev.push(Event::TurnEnded { player: p });
        self.next_turn(ev);
    }

    /// Fills the current player's bar from Speed, halved (rounding up) next
    /// to an aggressive unfriendly creature (creatures 2.6), then the
    /// weight slowdown (inventory 3.3, 3.4).
    fn start_turn(&mut self, ev: &mut Vec<Event>) {
        // Magic 3.2: summonses are answered on the following turn.
        self.answer_summons(ev);
        // Creatures 2.2.4: the first turn after a map door into a room
        // starts with the moves left at the door. The bar is not refilled,
        // so it keeps its full size from the door turn.
        let me = self.current;
        let carried = self
            .carried
            .iter()
            .position(|&(q, _)| q == me)
            .map(|i| self.carried.remove(i).1);
        let (moves, hindered) = match carried {
            Some(left) => (left, false),
            None => {
                let hindered = self.players[me].speed > 0 && self.hindered(me);
                (self.players[me].turn_speed(&self.adv, hindered), hindered)
            }
        };
        if let Some(r) = self.players[self.current].at.room() {
            if !self.turn_rooms.contains(&r) {
                self.turn_rooms.push(r);
            }
        }
        let p = &mut self.players[self.current];
        p.moves_left = moves;
        if carried.is_none() {
            p.turn_moves = moves;
        }
        ev.push(Event::TurnStarted {
            player: self.current,
            moves,
            hindered,
        });
        // Magic 5.1, 5.3: drift and the healing die at the top of the turn.
        self.turn_top(Who::Player { player: self.current }, ev);
    }

    /// Test and tool hook: the live creature in a region slot.
    pub fn creature(&self, region: u8, slot: usize) -> Option<&acs_core::model::Creature> {
        self.region(region)?.residents.get(slot).filter(|c| c.in_use)
    }

    /// How many creature slots a region has (16 in Classic, creatures 1.1).
    pub fn slots(&self, region: u8) -> usize {
        self.region(region).map_or(0, |r| r.residents.len())
    }

    /// Whether a region slot holds a random creature (creatures 3.3).
    pub fn is_random(&self, region: u8, slot: usize) -> bool {
        self.random.contains(&(region, slot))
    }

    /// Test and tool hook: edits a region slot's record (not replayed).
    pub fn creature_mut(&mut self, region: u8, slot: usize) -> Option<&mut acs_core::model::Creature> {
        self.region_mut(region)?.residents.get_mut(slot)
    }

    // ---- lookups ---------------------------------------------------------

    /// A region by number; 0 is the world map's own region, which holds
    /// the encounter areas (encounters 3.1).
    pub fn region(&self, number: u8) -> Option<&Region> {
        if number == 0 {
            return Some(&self.map);
        }
        self.adv.regions.iter().find(|r| r.number == number)
    }

    pub(crate) fn region_mut(&mut self, number: u8) -> Option<&mut Region> {
        if number == 0 {
            return Some(&mut self.map);
        }
        self.adv.regions.iter_mut().find(|r| r.number == number)
    }

    /// A live room.
    pub(crate) fn room(&self, region: u8, room: u8) -> Option<&Room> {
        self.region(region)?
            .rooms
            .get(room as usize)
            .filter(|r| !r.deleted)
    }

    pub(crate) fn thing(&self, number: ThingId) -> Option<&Thing> {
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
