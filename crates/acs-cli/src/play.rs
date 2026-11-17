// SPDX-License-Identifier: GPL-3.0-or-later

//! `acs play`: plays an adventure with a scripted walkthrough
//! (`docs/play.md`). A front end like the viewer's walk mode: it only
//! issues [`Command`]s and reads [`Event`]s and state. Walking is a
//! breadth-first search over moves tried on clones of the game, so what
//! a square does is whatever the engine says it does.

use crate::script::{sturdy, Action, Expect, Script, Sel, Step};
use acs_core::model::{Adventure, Destination, ThingId};
use acs_engine::{describe, Command, Dir, Event, Game, Location, Rules, SaveFile, Stat, Who};
use serde_json::json;
use std::collections::{HashMap, HashSet, VecDeque};

/// Lines of the engine's log kept for a failure report.
const RECENT: usize = 30;
/// Squares a walk's search visits at most.
const SEARCH_MAX: usize = 20_000;
/// Turns one walk may take.
const WALK_TURNS: usize = 400;
/// Commands played for others while waiting for a player's turn.
const WAIT_MAX: usize = 2_000;
/// Door hops a `goto` tries at most.
const HOPS_MAX: usize = 64;
/// Turns an `attack` lasts unless told.
const ATTACK_TURNS: u32 = 20;

/// What a run made: the game as it ended, the save, the trace and, when a
/// step failed, the report.
pub struct Outcome {
    pub game: Game,
    pub save: SaveFile,
    /// One JSON object a line.
    pub trace: Vec<String>,
    /// Human-readable lines, one per step.
    pub progress: Vec<String>,
    pub failure: Option<String>,
}

/// Plays `script` on `adv` (as decoded) with `seed` and at least
/// `players` players.
pub fn play(adv: &Adventure, script: &Script, seed: u64, players: usize) -> Outcome {
    let declared = script.players.iter().map(|(n, _)| n + 1).max().unwrap_or(1);
    let count = players.max(declared).clamp(1, acs_engine::MAX_PLAYERS);
    let setup = |i: usize| {
        script
            .players
            .iter()
            .find(|(n, _)| *n == i)
            .map(|(_, s)| s.clone())
            .unwrap_or_else(|| sturdy(i))
    };
    let (game, ev) = Game::new(adv.clone(), seed, Rules::default(), setup(0));
    let mut d = Driver {
        original: adv,
        game,
        trace: Vec::new(),
        progress: Vec::new(),
        recent: VecDeque::new(),
        messages: Vec::new(),
        deaths: Vec::new(),
    };
    d.trace
        .push(json!({"start": {"seed": seed, "players": count}}).to_string());
    d.record(None, None, &ev);
    let mut failure = d.add_players(count, setup).err();
    if failure.is_none() {
        failure = d.run(&script.steps);
    }
    if let Some(f) = &failure {
        d.trace.push(json!({ "failure": f }).to_string());
    }
    let autosave = Game::replay(
        adv.clone(),
        d.game.seed(),
        d.game.rules(),
        d.game.first_player().clone(),
        &d.game.log()[..d.game.restore_point()],
    );
    let save = SaveFile::new(&d.game, adv, &autosave);
    Outcome {
        game: d.game,
        save,
        trace: d.trace,
        progress: d.progress,
        failure,
    }
}

/// Where a walk is headed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Goal {
    /// Stand on the square (a door square counts as reached when taken).
    Square(u8, u8),
    /// Step onto the square and through the door or portal there.
    Door(u8, u8),
    /// Stand beside the square.
    Beside(u8, u8),
    /// Step onto the square and take what lies there.
    Take(u8, u8),
}

/// A door in the graph `goto` searches: on `at` of `from`, into `to`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Edge {
    from: (u8, u8),
    at: (u8, u8),
    to: (u8, u8),
}

struct Checkpoint {
    game: Game,
    trace: usize,
    progress: usize,
    recent: VecDeque<String>,
    messages: Vec<u16>,
    deaths: Vec<usize>,
}

struct Driver<'a> {
    original: &'a Adventure,
    game: Game,
    trace: Vec<String>,
    progress: Vec<String>,
    recent: VecDeque<String>,
    /// Long messages shown since the last `expect`.
    messages: Vec<u16>,
    /// Players who died and haven't been accounted for.
    deaths: Vec<usize>,
}

/// `(region, room)` of a place; the world map is (0, 0).
fn place_key(at: Location) -> (u8, u8) {
    match at {
        Location::World { .. } => (0, 0),
        Location::Room { region, room, .. } => (region, room),
    }
}

fn place_name(game: &Game, at: Location) -> String {
    match at {
        Location::World { x, y } => format!("the world map at {x},{y}"),
        Location::Room {
            region: 0,
            room,
            x,
            y,
        } => format!("encounter area {room} at {x},{y}"),
        Location::Room { region, room, x, y } => {
            let r = game.region(region);
            let rname = r.map(|r| r.name.trim().to_string()).unwrap_or_default();
            let mname = r
                .and_then(|r| r.rooms.get(room as usize))
                .map(|m| m.name.trim().to_string())
                .unwrap_or_default();
            format!("{mname} (region {region} {rname}, room {room}) at {x},{y}")
        }
    }
}

fn thing_name(game: &Game, t: ThingId) -> String {
    game.adventure()
        .things
        .get((t as usize).wrapping_sub(1))
        .map(|t| t.name.trim().to_string())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| format!("thing {t}"))
}

fn dirs(game: &Game) -> &'static [Dir] {
    if game.rules().diagonals {
        &Dir::ALL
    } else {
        &[Dir::N, Dir::E, Dir::S, Dir::W]
    }
}

/// The square one step from `at` in `dir`, inside the room's walls or on
/// the map (wrapping as the map does); `None` off the edge.
fn neighbour(game: &Game, at: Location, dir: Dir) -> Option<Location> {
    let (dx, dy) = dir.delta();
    match at {
        Location::World { x, y } => {
            let tiles = &game.adventure().world.tiles;
            let (w, h) = (tiles.first().map_or(0, Vec::len) as i16, tiles.len() as i16);
            let (nx, ny) = (x as i16 + dx as i16, y as i16 + dy as i16);
            if w == 0 || h == 0 {
                None
            } else if game.adventure().world.wraps {
                Some(Location::World {
                    x: nx.rem_euclid(w) as u8,
                    y: ny.rem_euclid(h) as u8,
                })
            } else if (0..w).contains(&nx) && (0..h).contains(&ny) {
                Some(Location::World {
                    x: nx as u8,
                    y: ny as u8,
                })
            } else {
                None
            }
        }
        Location::Room { region, room, x, y } => {
            let r = game.region(region)?.rooms.get(room as usize)?;
            let (nx, ny) = (x as i16 + dx as i16, y as i16 + dy as i16);
            ((0..r.width as i16).contains(&nx) && (0..r.height as i16).contains(&ny)).then_some(
                Location::Room {
                    region,
                    room,
                    x: nx as u8,
                    y: ny as u8,
                },
            )
        }
    }
}

/// The same place with the square `(x, y)`.
fn with_xy(at: Location, x: u8, y: u8) -> Location {
    match at {
        Location::World { .. } => Location::World { x, y },
        Location::Room { region, room, .. } => Location::Room { region, room, x, y },
    }
}

/// The direction from `a` to the square beside it `b`, if they are beside
/// each other as moves go.
fn dir_to(game: &Game, a: Location, b: Location) -> Option<Dir> {
    dirs(game)
        .iter()
        .copied()
        .find(|&d| neighbour(game, a, d) == Some(b))
}

/// The events of `ev` up to the end of `me`'s turn (the step's own).
fn own_part(ev: &[Event], me: usize) -> &[Event] {
    let end = ev
        .iter()
        .position(|e| matches!(e, Event::TurnEnded { player } if *player == me))
        .map_or(ev.len(), |i| i + 1);
    &ev[..end]
}

fn turn_ended(ev: &[Event], me: usize) -> bool {
    ev.iter()
        .any(|e| matches!(e, Event::TurnEnded { player } if *player == me))
}

/// Plays `Wait` for everyone else until it is `me`'s turn, on a game no
/// one records (a search's clone). The commands played, or `None` when
/// `me` died or nobody is left in play.
fn advance(g: &mut Game, me: usize) -> Option<Vec<Command>> {
    let mut cmds = Vec::new();
    for _ in 0..WAIT_MAX {
        if g.is_over() || !g.players().get(me).is_some_and(|p| p.alive) {
            return None;
        }
        if g.current() == me {
            return Some(cmds);
        }
        g.apply(Command::Wait);
        cmds.push(Command::Wait);
    }
    None
}

/// Why a tried step is no good for a walk, or `None` when it is fine.
fn objection(game: &Game, before: Location, after: Location, ev: &[Event], me: usize) -> Option<String> {
    if let Some(why) = harm(ev, me) {
        return Some(why);
    }
    if place_key(after) != place_key(before) {
        return Some(format!("it takes a door to {}", place_name(game, after)));
    }
    if after == before {
        let why: Vec<String> = own_part(ev, me)
            .iter()
            .filter(|e| !matches!(e, Event::TurnEnded { .. } | Event::TurnStarted { .. }))
            .map(|e| describe(game, e))
            .collect();
        return Some(if why.is_empty() {
            "the player doesn't move".into()
        } else {
            why.join(" ")
        });
    }
    None
}

/// Whether `me` took something in the step's own events.
fn took(ev: &[Event], me: usize) -> bool {
    own_part(ev, me).iter().any(
        |e| matches!(e, Event::PickedUp { player, .. } | Event::GoldPickedUp { player, .. } if *player == me),
    )
}

/// What in a step's own events a walk keeps away from: a death or lost
/// Life Force, a store, a map encounter.
fn harm(ev: &[Event], me: usize) -> Option<String> {
    let mine = |w: &Who| *w == Who::Player { player: me };
    for e in own_part(ev, me) {
        match e {
            Event::Died { who, .. } if mine(who) => return Some("the step kills the player".into()),
            Event::StatChanged {
                who,
                stat: Stat::LifeForce,
                from,
                to,
            } if mine(who) && to < from => return Some(format!("the step lowers Life Force {from} -> {to}")),
            Event::StoreOpened { .. } => return Some("it opens a store".into()),
            Event::Encounter { .. } => return Some("a map encounter".into()),
            Event::EnteredArea { .. } | Event::LeftArea { .. } => return Some("an encounter area".into()),
            _ => {}
        }
    }
    None
}

/// Whether `a` and `b` are beside each other in the same place.
fn beside(game: &Game, a: Location, b: Location) -> bool {
    dir_to(game, a, b).is_some()
}

/// A breadth-first search over `me`'s moves, each tried on a clone, until
/// `goal`; turns that end on the way go on after the others wait. The
/// commands to play (moves and the others' waits), or why it failed.
fn plan(game: &Game, me: usize, goal: Goal) -> Result<Vec<Command>, String> {
    struct Node {
        parent: Option<usize>,
        cmds: Vec<Command>,
    }
    let start = game.players()[me].at;
    let target = match goal {
        Goal::Square(x, y) | Goal::Door(x, y) | Goal::Beside(x, y) | Goal::Take(x, y) => with_xy(start, x, y),
    };
    let done_at = |at: Location| match goal {
        Goal::Square(..) => at == target,
        Goal::Door(..) | Goal::Take(..) => false,
        Goal::Beside(..) => beside(game, at, target),
    };
    if done_at(start) {
        return Ok(Vec::new());
    }
    let mut nodes = vec![Node {
        parent: None,
        cmds: Vec::new(),
    }];
    let mut reached: HashMap<Location, usize> = HashMap::from([(start, 0)]);
    let mut refused: HashMap<Location, String> = HashMap::new();
    let mut frontier: Vec<(usize, Location, Game)> = vec![(0, start, game.clone())];
    let path = |nodes: &[Node], mut i: usize, last: Vec<Command>| {
        let mut parts = vec![last];
        while let Some(p) = nodes[i].parent {
            parts.push(nodes[i].cmds.clone());
            i = p;
        }
        parts.into_iter().rev().flatten().collect::<Vec<_>>()
    };
    while !frontier.is_empty() && nodes.len() < SEARCH_MAX {
        let mut next = Vec::new();
        for (i, at, g) in frontier {
            for &d in dirs(&g) {
                let Some(sq) = neighbour(&g, at, d) else { continue };
                if reached.contains_key(&sq) && sq != target {
                    continue;
                }
                let into_target = sq == target;
                let mut c = g.clone();
                let mut cmds = vec![Command::Move { dir: d }];
                let mut ev = c.apply(Command::Move { dir: d });
                if matches!(goal, Goal::Take(..)) && into_target {
                    if took(&ev, me) {
                        return Ok(path(&nodes, i, cmds));
                    }
                    let after = c.players()[me].at;
                    let why =
                        objection(&c, at, after, &ev, me).unwrap_or_else(|| "nothing is taken there".into());
                    refused.insert(sq, why);
                    continue;
                }
                // A take on the way: the same step again goes on (a stack
                // gives up one thing a step).
                let mut harmed = None;
                for _ in 0..16 {
                    if !(took(&ev, me) && c.players()[me].at == at) {
                        break;
                    }
                    harmed = harm(&ev, me);
                    if harmed.is_some() {
                        break;
                    }
                    if turn_ended(&ev, me) || c.current() != me {
                        match advance(&mut c, me) {
                            Some(w) => cmds.extend(w),
                            None => {
                                harmed = Some("the player dies, or the game ends, after it".into());
                                break;
                            }
                        }
                    }
                    ev = c.apply(Command::Move { dir: d });
                    cmds.push(Command::Move { dir: d });
                }
                if let Some(why) = harmed {
                    refused.entry(sq).or_insert(why);
                    continue;
                }
                let after = c.players()[me].at;
                let objected = objection(&c, at, after, &ev, me);
                let ok = match goal {
                    // A walk's own target may hurt or be a door.
                    Goal::Square(..) if into_target && after != at => Some(true),
                    Goal::Door(..) if into_target => {
                        if place_key(after) != place_key(at) {
                            Some(true)
                        } else {
                            refused.insert(
                                sq,
                                objected
                                    .clone()
                                    .unwrap_or_else(|| "no door is taken there".into()),
                            );
                            Some(false)
                        }
                    }
                    Goal::Beside(..) if objected.is_none() && beside(&c, after, target) => Some(true),
                    _ => None,
                };
                if ok == Some(true) {
                    return Ok(path(&nodes, i, cmds));
                }
                if ok == Some(false) {
                    continue;
                }
                if let Some(why) = objected {
                    refused.entry(sq).or_insert(why);
                    continue;
                }
                if reached.contains_key(&after) {
                    continue;
                }
                if turn_ended(&ev, me) || c.current() != me {
                    match advance(&mut c, me) {
                        Some(w) => cmds.extend(w),
                        None => {
                            refused
                                .entry(sq)
                                .or_insert_with(|| "the player dies, or the game ends, after it".into());
                            continue;
                        }
                    }
                }
                let n = nodes.len();
                nodes.push(Node {
                    parent: Some(i),
                    cmds,
                });
                reached.insert(after, n);
                next.push((n, after, c));
            }
        }
        frontier = next;
    }
    // Unreachable: say what was reachable, nearest first.
    let (tx, ty) = target.xy();
    let mut near: Vec<Location> = reached.keys().copied().collect();
    near.sort_by_key(|l| {
        let (x, y) = l.xy();
        let (dx, dy) = (x as i32 - tx as i32, y as i32 - ty as i32);
        (dx * dx + dy * dy, y, x)
    });
    let list: Vec<String> = near
        .iter()
        .take(12)
        .map(|l| {
            let (x, y) = l.xy();
            format!("{x},{y}")
        })
        .collect();
    let what = match goal {
        Goal::Square(..) => "square",
        Goal::Door(..) => "door",
        Goal::Beside(..) => "a square beside",
        Goal::Take(..) => "a take at",
    };
    let mut msg = format!("{what} {tx},{ty} is unreachable from {}", place_name(game, start));
    if let Some(why) = refused.get(&target) {
        msg.push_str(&format!(" (stepping onto it: {why})"));
    }
    if nodes.len() >= SEARCH_MAX {
        msg.push_str(&format!(" (search stopped after {SEARCH_MAX} squares)"));
    }
    msg.push_str(&format!(
        "; reachable: {} squares, nearest first: {}{}",
        near.len(),
        list.join(" "),
        if near.len() > list.len() { " ..." } else { "" }
    ));
    Err(msg)
}

impl Driver<'_> {
    fn me_name(&self, p: usize) -> String {
        self.game
            .players()
            .get(p)
            .map(|pl| pl.name.clone())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| format!("player {}", p + 1))
    }

    /// Writes a command (or the start) and its events to the trace.
    fn record(&mut self, player: Option<usize>, cmd: Option<&Command>, ev: &[Event]) {
        let log: Vec<String> = ev.iter().map(|e| describe(&self.game, e)).collect();
        for l in &log {
            self.recent.push_back(l.clone());
        }
        while self.recent.len() > RECENT {
            self.recent.pop_front();
        }
        for e in ev {
            match e {
                Event::LongMessage { number, .. } => self.messages.push(*number),
                Event::Died {
                    who: Who::Player { player },
                    ..
                } => self.deaths.push(*player),
                _ => {}
            }
        }
        self.trace.push(
            json!({
                "player": player.map(|p| p + 1),
                "command": cmd,
                "events": ev,
                "log": log,
            })
            .to_string(),
        );
    }

    fn apply(&mut self, cmd: Command) -> Vec<Event> {
        let p = self.game.current();
        let ev = self.game.apply(cmd.clone());
        self.record(Some(p), Some(&cmd), &ev);
        ev
    }

    fn checkpoint(&self) -> Checkpoint {
        Checkpoint {
            game: self.game.clone(),
            trace: self.trace.len(),
            progress: self.progress.len(),
            recent: self.recent.clone(),
            messages: self.messages.clone(),
            deaths: self.deaths.clone(),
        }
    }

    fn rollback(&mut self, c: Checkpoint) {
        self.game = c.game;
        self.trace.truncate(c.trace);
        self.progress.truncate(c.progress);
        self.recent = c.recent;
        self.messages = c.messages;
        self.deaths = c.deaths;
    }

    /// Players 2.. join through "Add Someone" at the start; player 1 waits
    /// out the round that brings them in.
    fn add_players(
        &mut self,
        count: usize,
        setup: impl Fn(usize) -> acs_engine::PlayerSetup,
    ) -> Result<(), String> {
        for i in 1..count {
            self.apply(Command::AddPlayer { setup: setup(i) });
            let mut n = 0;
            while self.game.players().len() <= i {
                if n == WAIT_MAX || self.game.is_over() {
                    return Err(format!("player {} never joined", i + 1));
                }
                self.apply(Command::Wait);
                n += 1;
            }
        }
        Ok(())
    }

    /// Waits for `me`'s turn, playing `Wait` for everyone else.
    fn turn_for(&mut self, me: usize) -> Result<(), String> {
        for _ in 0..WAIT_MAX {
            if self.game.is_over() {
                return Err("nobody is left in play".into());
            }
            let Some(p) = self.game.players().get(me) else {
                return Err(format!("player {} is not in the game", me + 1));
            };
            if !p.alive {
                return Err(format!("{} is dead", self.me_name(me)));
            }
            if self.game.current() == me {
                return Ok(());
            }
            self.apply(Command::Wait);
        }
        Err(format!("{}'s turn never came", self.me_name(me)))
    }

    /// Walks `me` to `goal`, planning afresh at each of its turns. The
    /// events of the last step.
    fn walk(&mut self, me: usize, goal: Goal) -> Result<Vec<Event>, String> {
        let mut last = Vec::new();
        for _ in 0..WALK_TURNS {
            self.turn_for(me)?;
            let cmds = plan(&self.game, me, goal)?;
            if cmds.is_empty() {
                return Ok(last);
            }
            let n = cmds.len();
            for (i, cmd) in cmds.into_iter().enumerate() {
                let mine = matches!(cmd, Command::Move { .. });
                let ev = self.apply(cmd);
                if !self.deaths.is_empty() {
                    return Err("a player died on the way".into());
                }
                if mine {
                    last = ev.clone();
                }
                if i + 1 == n {
                    if matches!(goal, Goal::Door(..) | Goal::Take(..)) {
                        return Ok(last);
                    }
                    break;
                }
                if mine && turn_ended(&ev, me) {
                    break;
                }
            }
        }
        Err(format!("still walking after {WALK_TURNS} turns"))
    }

    fn at(&self, me: usize) -> Location {
        self.game.players()[me].at
    }

    /// Fails on a refusal event in `ev`.
    fn refused(&self, ev: &[Event]) -> Result<(), String> {
        for e in ev {
            if matches!(
                e,
                Event::CastRefused { .. }
                    | Event::DropRefused { .. }
                    | Event::ReadyRefused { .. }
                    | Event::StoreRefused { .. }
                    | Event::RetireRefused { .. }
                    | Event::CannotFire { .. }
            ) {
                return Err(format!("refused: {}", describe(&self.game, e)));
            }
        }
        Ok(())
    }

    /// Who stands on `(x, y)` of `me`'s room, `me` aside.
    fn who_at(&self, me: usize, x: u8, y: u8) -> Option<Who> {
        let sq = with_xy(self.at(me), x, y);
        for (i, p) in self.game.players().iter().enumerate() {
            if i != me && p.alive && p.at == sq {
                return Some(Who::Player { player: i });
            }
        }
        let region = sq.region();
        (0..self.game.slots(region))
            .map(|slot| Who::Creature { region, slot })
            .find(|&w| self.game.location(w) == Some(sq) && matches!(sq, Location::Room { .. }))
    }

    /// Pushes `me` into the square `sq` beside it (one Move).
    fn push(&mut self, me: usize, sq: Location) -> Result<Vec<Event>, String> {
        self.turn_for(me)?;
        let at = self.at(me);
        let dir = dir_to(&self.game, at, sq).ok_or_else(|| {
            let (x, y) = sq.xy();
            format!("{x},{y} is not beside {}", place_name(&self.game, at))
        })?;
        Ok(self.apply(Command::Move { dir }))
    }

    fn goto(&mut self, me: usize, region: u8, room: u8) -> Result<(), String> {
        let want = (region, room);
        let mut bad: HashMap<Edge, String> = HashMap::new();
        for _ in 0..HOPS_MAX {
            let here = place_key(self.at(me));
            if here == want {
                return Ok(());
            }
            let Some(route) = route(&self.game, here, want, &bad) else {
                let mut msg = format!(
                    "no door route from {} to region {region} room {room}",
                    place_name(&self.game, self.at(me))
                );
                if !bad.is_empty() {
                    let tried: Vec<String> = bad
                        .iter()
                        .map(|(e, why)| {
                            format!("door {},{} of {:?} to {:?}: {why}", e.at.0, e.at.1, e.from, e.to)
                        })
                        .collect();
                    msg.push_str(&format!("; doors that didn't work: {}", tried.join("; ")));
                }
                return Err(msg);
            };
            let hop = route[0];
            let cp = self.checkpoint();
            match self.walk(me, Goal::Door(hop.at.0, hop.at.1)) {
                Ok(_) if place_key(self.at(me)) == hop.to => {}
                Ok(_) => {
                    let why = format!("it led to {}", place_name(&self.game, self.at(me)));
                    self.rollback(cp);
                    bad.insert(hop, why);
                }
                Err(e) => {
                    self.rollback(cp);
                    bad.insert(hop, e);
                }
            }
        }
        Err(format!("not there after {HOPS_MAX} doors"))
    }

    fn state(&self, me: usize) -> String {
        let Some(p) = self.game.players().get(me) else {
            return format!("player {} is not in the game", me + 1);
        };
        let mut counts: Vec<(ThingId, usize)> = Vec::new();
        for &t in &p.possessions {
            match counts.iter_mut().find(|(u, _)| *u == t) {
                Some(c) => c.1 += 1,
                None => counts.push((t, 1)),
            }
        }
        let things: Vec<String> = counts
            .iter()
            .map(|&(t, n)| {
                let name = thing_name(&self.game, t);
                if n > 1 {
                    format!("{name} x{n} ({t})")
                } else {
                    format!("{name} ({t})")
                }
            })
            .collect();
        let held = |t: Option<ThingId>| t.map_or("-".into(), |t| thing_name(&self.game, t));
        format!(
            "{} ({}): {}; Life Force {}/{}, moves {}/{}, gold {}, weapon {}, armor {}; carries: {}",
            p.name,
            if p.alive { "alive" } else { "dead" },
            place_name(&self.game, p.at),
            p.life_force,
            p.constitution,
            p.moves_left,
            p.turn_moves,
            p.gold,
            held(p.weapon),
            held(p.armor),
            if things.is_empty() {
                "nothing".into()
            } else {
                things.join(", ")
            }
        )
    }

    fn expect(&mut self, me: usize, e: &Expect) -> Result<(), String> {
        let p = self.game.players().get(me).cloned().ok_or("no such player")?;
        let name = self.me_name(me);
        match *e {
            Expect::At(region, room, x, y) => {
                let want = if region == 0 && room == 0 {
                    Location::World { x, y }
                } else {
                    Location::Room { region, room, x, y }
                };
                if p.at != want {
                    return Err(format!(
                        "expected {name} at {}, but at {}",
                        place_name(&self.game, want),
                        place_name(&self.game, p.at)
                    ));
                }
            }
            Expect::Owns(t, yes) => {
                if p.owns(t) != yes {
                    return Err(format!(
                        "expected {name} {}to own {}",
                        if yes { "" } else { "not " },
                        thing_name(&self.game, t)
                    ));
                }
            }
            Expect::Message(n) => {
                if !self.messages.contains(&n) {
                    return Err(format!(
                        "expected long message {n} since the last expect; shown: {:?}",
                        self.messages
                    ));
                }
            }
            Expect::Alive => {
                if !p.alive {
                    return Err(format!("expected {name} alive"));
                }
            }
            Expect::Dead => {
                if p.alive {
                    return Err(format!("expected {name} dead"));
                }
                self.deaths.retain(|&d| d != me);
            }
            Expect::Life(op, n) => {
                if !op.holds(p.life_force, n) {
                    return Err(format!(
                        "expected {name}'s Life Force {} {n}, but it is {}",
                        op.symbol(),
                        p.life_force
                    ));
                }
            }
        }
        Ok(())
    }

    /// One step for one player.
    fn step_for(&mut self, me: usize, action: &Action) -> Result<(), String> {
        match *action {
            Action::As(_) => {}
            Action::Walk(x, y) => {
                self.walk(me, Goal::Square(x, y))?;
            }
            Action::Door(x, y) => {
                let before = place_key(self.at(me));
                self.walk(me, Goal::Door(x, y))?;
                if place_key(self.at(me)) == before {
                    return Err(format!("no door was taken at {x},{y}"));
                }
            }
            Action::Goto { region, room, at } => {
                self.goto(me, region, room)?;
                if let Some((x, y)) = at {
                    self.walk(me, Goal::Square(x, y))?;
                }
            }
            Action::Take(x, y) => {
                if self.at(me).xy() == (x, y) {
                    return Err(format!("already standing on {x},{y}: step off it first"));
                }
                let ev = self.walk(me, Goal::Take(x, y))?;
                if !took(&ev, me) {
                    return Err(format!("nothing was taken at {x},{y}"));
                }
            }
            Action::Bump(x, y) => {
                let sq = with_xy(self.at(me), x, y);
                if !beside(&self.game, self.at(me), sq) {
                    self.walk(me, Goal::Beside(x, y))?;
                }
                self.push(me, sq)?;
            }
            Action::Attack(x, y, turns) => self.attack(me, x, y, turns.unwrap_or(ATTACK_TURNS))?,
            Action::Use(thing, at) => {
                self.turn_for(me)?;
                let ev = self.apply(Command::Use {
                    thing,
                    target: None,
                    at,
                });
                self.refused(&ev)?;
            }
            Action::Cast(thing, at) => {
                self.turn_for(me)?;
                let ev = self.apply(Command::Cast {
                    thing,
                    target: None,
                    at,
                });
                self.refused(&ev)?;
            }
            Action::Drop(thing, dir) => self.single(me, Command::Drop { thing, dir })?,
            Action::Ready(thing) => self.single(me, Command::Ready { thing })?,
            Action::Buy(thing) => {
                self.require_store(me)?;
                self.single(me, Command::Buy { thing })?;
            }
            Action::Sell(thing) => {
                self.require_store(me)?;
                self.single(me, Command::Sell { thing })?;
            }
            Action::Leave => self.single(me, Command::LeaveStore)?,
            Action::Save => self.single(me, Command::SaveGame)?,
            Action::Retire(save) => self.single(me, Command::Retire { save })?,
            Action::Wait(n) => {
                for _ in 0..n {
                    self.single(me, Command::Wait)?;
                }
            }
            Action::Idle(n) => {
                for _ in 0..n {
                    self.single(me, Command::Idle)?;
                }
            }
            Action::Restore => self.restore(),
            Action::Expect(ref e) => self.expect(me, e)?,
        }
        Ok(())
    }

    fn require_store(&self, me: usize) -> Result<(), String> {
        match self.game.store_visit() {
            Some(_) if self.game.current() == me => Ok(()),
            _ => Err(format!("{} is not in a store (bump one first)", self.me_name(me))),
        }
    }

    fn single(&mut self, me: usize, cmd: Command) -> Result<(), String> {
        self.turn_for(me)?;
        let ev = self.apply(cmd);
        self.refused(&ev)
    }

    /// The death message's button: the game rebuilt at the restore point,
    /// then `Restore` (as the viewer does).
    fn restore(&mut self) {
        let point = self.game.restore_point();
        self.game = Game::replay(
            self.original.clone(),
            self.game.seed(),
            self.game.rules(),
            self.game.first_player().clone(),
            &self.game.log()[..point],
        );
        self.apply(Command::Restore);
        self.deaths.clear();
    }

    fn attack(&mut self, me: usize, x: u8, y: u8, turns: u32) -> Result<(), String> {
        let who = self
            .who_at(me, x, y)
            .ok_or_else(|| format!("no one stands on {x},{y}"))?;
        let mut used = 0;
        for _ in 0..WAIT_MAX {
            if !self.game.alive(who) {
                return Ok(());
            }
            if used >= turns {
                self.progress.push(format!(
                    "  (attack stopped after {turns} turns; the foe still lives)"
                ));
                return Ok(());
            }
            let foe = self.game.location(who).ok_or("the foe is gone")?;
            if place_key(foe) != place_key(self.at(me)) {
                return Err(format!("the foe left for {}", place_name(&self.game, foe)));
            }
            if !beside(&self.game, self.at(me), foe) {
                let (fx, fy) = foe.xy();
                self.walk(me, Goal::Beside(fx, fy))?;
                continue;
            }
            let ev = self.push(me, foe)?;
            if !ev
                .iter()
                .any(|e| matches!(e, Event::Attacked { by, .. } if *by == (Who::Player { player: me })))
            {
                let why: Vec<String> = ev.iter().map(|e| describe(&self.game, e)).collect();
                return Err(format!("pushing into it didn't attack: {}", why.join(" ")));
            }
            if turn_ended(&ev, me) {
                used += 1;
            }
        }
        Err("the attack never ended".into())
    }

    fn players_for(&self, sel: Sel) -> Vec<usize> {
        match sel {
            Sel::One(p) => vec![p],
            Sel::Both => (0..self.game.players().len()).collect(),
        }
    }

    fn report(&self, step: &Step, me: Option<usize>, why: &str) -> String {
        let mut s = format!("line {}: {}\n  failed: {why}\n", step.line, step.text);
        let who: Vec<usize> = match me {
            Some(p) => vec![p],
            None => (0..self.game.players().len()).collect(),
        };
        for p in who {
            s.push_str(&format!("  {}\n", self.state(p)));
        }
        s.push_str("  last events:\n");
        for l in &self.recent {
            s.push_str(&format!("    {l}\n"));
        }
        s
    }

    /// Runs the steps; the failure report of the first that failed.
    fn run(&mut self, steps: &[Step]) -> Option<String> {
        let mut sel = Sel::One(0);
        for (i, step) in steps.iter().enumerate() {
            if let Action::As(s) = step.action {
                sel = s;
            }
            let players = self.players_for(sel);
            self.trace.push(
                json!({
                    "step": i + 1,
                    "line": step.line,
                    "text": step.text,
                    "players": players.iter().map(|p| p + 1).collect::<Vec<_>>(),
                })
                .to_string(),
            );
            let next_expects_death = matches!(
                steps.get(i + 1).map(|s| &s.action),
                Some(Action::Expect(Expect::Dead))
            );
            for &me in &players {
                if me >= self.game.players().len() {
                    return Some(self.report(step, None, &format!("player {} is not in the game", me + 1)));
                }
                let result = self.step_for(me, &step.action);
                if !self.deaths.is_empty() && !matches!(step.action, Action::Expect(_)) {
                    let names: Vec<String> = self.deaths.iter().map(|&d| self.me_name(d)).collect();
                    if next_expects_death {
                        self.deaths.clear();
                    } else {
                        let why = match &result {
                            Err(e) => format!("{} died ({e})", names.join(", ")),
                            Ok(()) => format!("{} died", names.join(", ")),
                        };
                        return Some(self.report(step, Some(me), &why));
                    }
                }
                if let Err(e) = result {
                    return Some(self.report(step, Some(me), &e));
                }
                if !matches!(step.action, Action::As(_)) {
                    let line = format!(
                        "line {}: {} -> {}: {}",
                        step.line,
                        step.text,
                        self.me_name(me),
                        place_name(&self.game, self.at(me))
                    );
                    self.progress.push(line);
                }
            }
            if matches!(step.action, Action::Expect(_)) {
                self.messages.clear();
            }
        }
        None
    }
}

/// The shortest door route from `from` to `to` over the doors the rooms
/// and the world map hold now, leaving out `bad` ones.
fn route(game: &Game, from: (u8, u8), to: (u8, u8), bad: &HashMap<Edge, String>) -> Option<Vec<Edge>> {
    let mut prev: HashMap<(u8, u8), Edge> = HashMap::new();
    let mut seen: HashSet<(u8, u8)> = HashSet::from([from]);
    let mut queue = VecDeque::from([from]);
    while let Some(here) = queue.pop_front() {
        if here == to {
            let mut path = Vec::new();
            let mut at = to;
            while at != from {
                let e = prev[&at];
                path.push(e);
                at = e.from;
            }
            path.reverse();
            return Some(path);
        }
        for e in doors(game, here) {
            if bad.contains_key(&e) || !seen.insert(e.to) {
                continue;
            }
            prev.insert(e.to, e);
            queue.push_back(e.to);
        }
    }
    None
}

fn dest_key(game: &Game, d: Destination) -> Option<(u8, u8)> {
    match d {
        Destination::WorldMap { .. } => Some((0, 0)),
        Destination::Room { region, room, .. } => {
            let live = game
                .region(region)
                .and_then(|r| r.rooms.get(room as usize))
                .is_some_and(|r| !r.deleted);
            live.then_some((region, room))
        }
    }
}

/// The doors out of a place: the world map's doors, or a room's items
/// that lead somewhere.
fn doors(game: &Game, from: (u8, u8)) -> Vec<Edge> {
    let adv = game.adventure();
    if from == (0, 0) {
        return adv
            .world
            .doors
            .iter()
            .filter_map(|d| {
                Some(Edge {
                    from,
                    at: (d.at.x, d.at.y),
                    to: dest_key(game, d.destination)?,
                })
            })
            .collect();
    }
    let Some(room) = game.region(from.0).and_then(|r| r.rooms.get(from.1 as usize)) else {
        return Vec::new();
    };
    room.items
        .iter()
        .filter_map(|i| {
            Some(Edge {
                from,
                at: (i.x, i.y),
                to: dest_key(game, i.destination?)?,
            })
        })
        .collect()
}
