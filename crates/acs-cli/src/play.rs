// SPDX-License-Identifier: GPL-3.0-or-later

//! `acs play`: plays an adventure with a scripted walkthrough
//! (`docs/play.md`). A front end like the viewer's play mode: it only
//! issues [`Command`]s and reads [`Event`]s and state. Walking is a
//! breadth-first search over moves tried on clones of the game, so what
//! a square does is whatever the engine says it does.

use crate::script::{sturdy, Action, Expect, Script, Sel, Step};
use acs_core::model::{Adventure, Destination, ThingId};
use acs_engine::{describe, Command, Dir, Event, Game, Location, Rules, SaveFile, Stat, Who};
use serde_json::json;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::{Hash, Hasher};

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
/// Doors a `goto`'s fallback route may add to what the first route left.
const DETOUR: usize = 2;
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
        turns: vec![0; acs_engine::MAX_PLAYERS],
        fresh: vec![true; acs_engine::MAX_PLAYERS],
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
    /// Stand beside the square (never stepping onto it).
    Beside(u8, u8),
    /// Stand beside the individual, wherever it is as the search goes.
    Foe(Who),
    /// Be drawn into an encounter area on the way to the square, or stand
    /// on it (`walk ... encounter`, play.md 3.2).
    Encounter(u8, u8),
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
    turns: Vec<usize>,
    fresh: Vec<bool>,
}

struct Driver<'a> {
    original: &'a Adventure,
    game: Game,
    trace: Vec<String>,
    progress: Vec<String>,
    recent: VecDeque<String>,
    /// Long messages shown since the last action step began (`expect`s
    /// and `as` keep them).
    messages: Vec<u16>,
    /// Players who died and haven't been accounted for.
    deaths: Vec<usize>,
    /// Turns each player has ended.
    turns: Vec<usize>,
    /// Players whose script hasn't had a turn yet: a turn of theirs with
    /// no moves left (the first after joining) is waited out.
    fresh: Vec<bool>,
}

/// `(region, room)` of a place; the world map is (0, 0).
fn place_key(at: Location) -> (u8, u8) {
    match at {
        Location::World { .. } => (0, 0),
        Location::Room { region, room, .. } => (region, room),
    }
}

/// Whether `at` is in one of the world map's encounter areas (rooms 1.. of
/// region 0).
fn in_area(at: Location) -> bool {
    matches!(at, Location::Room { region: 0, room, .. } if room >= 1)
}

/// Moves from `a` to `b` in the same place with nothing in the way:
/// diagonals or not as the rules say, across the map's wrap when it
/// wraps.
fn distance(game: &Game, a: Location, b: Location) -> usize {
    let ((ax, ay), (bx, by)) = (a.xy(), b.xy());
    let (mut dx, mut dy) = ((ax as i32 - bx as i32).abs(), (ay as i32 - by as i32).abs());
    if matches!(a, Location::World { .. }) && game.adventure().world.wraps {
        let tiles = &game.adventure().world.tiles;
        let (w, h) = (tiles.first().map_or(0, Vec::len) as i32, tiles.len() as i32);
        (dx, dy) = (dx.min(w - dx), dy.min(h - dy));
    }
    if game.rules().diagonals {
        dx.max(dy) as usize
    } else {
        (dx + dy) as usize
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

/// Whether `me` went through a door or portal in the step's own events
/// (into another place or onto another square of the same room).
fn took_door(ev: &[Event], me: usize) -> bool {
    own_part(ev, me)
        .iter()
        .any(|e| matches!(e, Event::TookDoor { player, .. } if *player == me))
}

/// What `me` took in the step's own events, by name.
fn taken(game: &Game, ev: &[Event], me: usize) -> String {
    let names: Vec<String> = own_part(ev, me)
        .iter()
        .filter_map(|e| match e {
            Event::PickedUp { player, thing } if *player == me => Some(thing_name(game, *thing)),
            Event::GoldPickedUp { player, amount } if *player == me => Some(format!("{amount} gold")),
            _ => None,
        })
        .collect();
    names.join(", ")
}

/// What a walk's search tells states apart by besides the square: what
/// the player carries and holds, and what the room's squares hold.
fn fingerprint(game: &Game, me: usize) -> u64 {
    let mut h = DefaultHasher::new();
    let p = &game.players()[me];
    (&p.possessions, p.gold, p.weapon, p.armor).hash(&mut h);
    if let Location::Room { region, room, .. } = p.at {
        if let Some(r) = game.region(region).and_then(|r| r.rooms.get(room as usize)) {
            for it in &r.items {
                (it.x, it.y, it.thing, it.parameter, it.flag, it.gold).hash(&mut h);
            }
        }
    }
    h.finish()
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
/// `goal`; turns that end on the way go on after the others wait. A state
/// is the square plus a fingerprint of what the player carries and what
/// the room holds, so a square is visited again once a step changed the
/// room (a take, a space's spell). The commands to play (moves and the
/// others' waits), or why it failed.
fn plan(game: &Game, me: usize, goal: Goal, avoid_takes: bool) -> Result<Vec<Command>, String> {
    struct Node {
        parent: Option<usize>,
        cmds: Vec<Command>,
    }
    let start = game.players()[me].at;
    let fixed = match goal {
        Goal::Square(x, y) | Goal::Door(x, y) | Goal::Beside(x, y) | Goal::Encounter(x, y) => {
            Some(with_xy(start, x, y))
        }
        Goal::Foe(_) => None,
    };
    // Where the goal's square is in `g` (a foe's square moves).
    let target_in = |g: &Game| match goal {
        Goal::Foe(w) => g
            .location(w)
            .filter(|l| g.alive(w) && place_key(*l) == place_key(start)),
        _ => fixed,
    };
    let done = |g: &Game, at: Location| match goal {
        Goal::Square(..) => Some(at) == fixed,
        Goal::Door(..) => false,
        Goal::Beside(..) | Goal::Foe(_) => target_in(g).is_some_and(|t| beside(g, at, t)),
        Goal::Encounter(..) => in_area(at) || Some(at) == fixed,
    };
    let Some(first_target) = target_in(game) else {
        return Err("the foe is not in this room".into());
    };
    if done(game, start) {
        return Ok(Vec::new());
    }
    let mut nodes = vec![Node {
        parent: None,
        cmds: Vec::new(),
    }];
    let mut reached: HashMap<(Location, u64), usize> = HashMap::from([((start, fingerprint(game, me)), 0)]);
    let mut squares: HashSet<Location> = HashSet::from([start]);
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
    // An encounter walk's best end so far (play.md 3.2): moves to it plus
    // the moves left to the target, whether it is the target reached with
    // no encounter (which loses ties), and the commands.
    let mut best: Option<(usize, bool, Vec<Command>)> = None;
    let mut level = 0;
    while !frontier.is_empty() && nodes.len() < SEARCH_MAX {
        // Ends from here on are at least `level + 1` moves away: none can
        // beat what was found.
        if best.as_ref().is_some_and(|b| (b.0, b.1) <= (level + 1, false)) {
            break;
        }
        let mut next = Vec::new();
        for (i, at, g) in frontier {
            let target = target_in(&g);
            let print = fingerprint(&g, me);
            for &d in dirs(&g) {
                let Some(sq) = neighbour(&g, at, d) else { continue };
                let into_target = Some(sq) == target;
                // A walk beside the square never steps onto it (that would
                // take, trigger or attack what is there).
                if into_target && matches!(goal, Goal::Beside(..) | Goal::Foe(_)) {
                    continue;
                }
                if !into_target && reached.contains_key(&(sq, print)) {
                    continue;
                }
                let mut c = g.clone();
                let mut cmds = vec![Command::Move { dir: d }];
                let mut ev = c.apply(Command::Move { dir: d });
                // A take on the way: the same step again goes on (a stack
                // gives up one thing a step); with `avoid_takes` only on
                // the target.
                let mut harmed = None;
                for _ in 0..16 {
                    if !(took(&ev, me) && c.players()[me].at == at) {
                        break;
                    }
                    if avoid_takes && !into_target {
                        harmed = Some(format!("it would take {}", taken(&c, &ev, me)));
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
                if let (Goal::Encounter(..), Some(t)) = (goal, fixed) {
                    let end = if in_area(after) {
                        let dies = own_part(&ev, me).iter().any(
                            |e| matches!(e, Event::Died { who, .. } if *who == Who::Player { player: me }),
                        );
                        (!dies).then(|| (level + 1 + distance(&c, sq, t), false))
                    } else if into_target && after != at {
                        Some((level + 1, true))
                    } else {
                        None
                    };
                    if let Some(key) = end {
                        if best.as_ref().is_none_or(|b| key < (b.0, b.1)) {
                            best = Some((key.0, key.1, path(&nodes, i, cmds)));
                        }
                        continue;
                    }
                }
                match goal {
                    // A walk's own target may hurt or be a door.
                    Goal::Square(..) if into_target && after != at => return Ok(path(&nodes, i, cmds)),
                    Goal::Door(..) if into_target => {
                        if took_door(&ev, me) {
                            return Ok(path(&nodes, i, cmds));
                        }
                        refused.insert(sq, objected.unwrap_or_else(|| "no door is taken there".into()));
                        continue;
                    }
                    _ => {}
                }
                if let Some(why) = objected {
                    refused.entry(sq).or_insert(why);
                    continue;
                }
                if done(&c, after) {
                    return Ok(path(&nodes, i, cmds));
                }
                let key = (after, fingerprint(&c, me));
                if reached.contains_key(&key) {
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
                reached.insert(key, n);
                squares.insert(after);
                next.push((n, after, c));
            }
        }
        frontier = next;
        level += 1;
    }
    if let Some((_, _, cmds)) = best {
        return Ok(cmds);
    }
    // Unreachable: say what was reachable, nearest first.
    let (tx, ty) = first_target.xy();
    let mut near: Vec<Location> = squares.into_iter().collect();
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
        Goal::Square(..) | Goal::Encounter(..) => "square",
        Goal::Door(..) => "door",
        Goal::Beside(..) => "a square beside",
        Goal::Foe(_) => "a square beside the foe on",
    };
    let mut msg = format!("{what} {tx},{ty} is unreachable from {}", place_name(game, start));
    if avoid_takes {
        msg.push_str(" avoiding takes");
    }
    if let Some(why) = refused.get(&first_target) {
        msg.push_str(&format!(" (stepping onto it: {why})"));
    }
    if nodes.len() >= SEARCH_MAX {
        msg.push_str(&format!(" (search stopped after {SEARCH_MAX} states)"));
    }
    msg.push_str(&format!(
        "; reachable: {} squares, nearest first: {}{}",
        near.len(),
        list.join(" "),
        if near.len() > list.len() { " ..." } else { "" }
    ));
    let mut takes: Vec<String> = refused
        .iter()
        .filter_map(|(l, why)| {
            let what = why.strip_prefix("it would take ")?;
            let (x, y) = l.xy();
            Some(format!("{x},{y} ({what})"))
        })
        .collect();
    takes.sort();
    if !takes.is_empty() {
        msg.push_str(&format!("; takes left alone: {}", takes.join(", ")));
    }
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
                Event::TurnEnded { player } => {
                    if let Some(t) = self.turns.get_mut(*player) {
                        *t += 1;
                    }
                }
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
            turns: self.turns.clone(),
            fresh: self.fresh.clone(),
        }
    }

    fn rollback(&mut self, c: Checkpoint) {
        self.game = c.game;
        self.trace.truncate(c.trace);
        self.progress.truncate(c.progress);
        self.recent = c.recent;
        self.messages = c.messages;
        self.deaths = c.deaths;
        self.turns = c.turns;
        self.fresh = c.fresh;
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

    /// Waits for `me`'s turn, playing `Wait` for everyone else. The
    /// first turn of a player's script is one with moves: one without
    /// (the first after joining) is waited out.
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
                if !self.fresh[me] {
                    return Ok(());
                }
                if p.moves_left > 0 {
                    self.fresh[me] = false;
                    return Ok(());
                }
            }
            self.apply(Command::Wait);
        }
        Err(format!("{}'s turn never came", self.me_name(me)))
    }

    /// On `me`'s turn with no moves left (a wound or a load can leave one,
    /// inventory 3.2), ends it with WAIT, so an attack or a fire goes on
    /// at the next turn rather than being refused. Whether it did.
    fn zero_moves_waited(&mut self, me: usize) -> bool {
        let idle = self.game.current() == me
            && self
                .game
                .players()
                .get(me)
                .is_some_and(|p| p.alive && p.moves_left == 0);
        if idle {
            self.apply(Command::Wait);
        }
        idle
    }

    /// Walks `me` to `goal`, planning afresh at each of its turns. The
    /// events of the last step.
    fn walk(&mut self, me: usize, goal: Goal, avoid_takes: bool) -> Result<Vec<Event>, String> {
        self.walk_turns(me, goal, avoid_takes, WALK_TURNS)?
            .ok_or_else(|| format!("still walking after {WALK_TURNS} turns"))
    }

    /// `walk` for at most `turns` plans (one a turn); `None` when the goal
    /// isn't reached by then.
    fn walk_turns(
        &mut self,
        me: usize,
        goal: Goal,
        avoid_takes: bool,
        turns: usize,
    ) -> Result<Option<Vec<Event>>, String> {
        let mut last = Vec::new();
        for _ in 0..turns {
            self.turn_for(me)?;
            let cmds = plan(&self.game, me, goal, avoid_takes)?;
            if cmds.is_empty() {
                return Ok(Some(last));
            }
            let n = cmds.len();
            for (i, cmd) in cmds.into_iter().enumerate() {
                let mine = matches!(cmd, Command::Move { .. });
                let ev = self.apply(cmd);
                if !self.deaths.is_empty() {
                    return Err("a player died on the way".into());
                }
                if matches!(goal, Goal::Encounter(..)) && in_area(self.at(me)) {
                    return Ok(Some(ev));
                }
                if mine {
                    last = ev.clone();
                }
                if i + 1 == n {
                    if matches!(goal, Goal::Door(..)) {
                        return Ok(Some(last));
                    }
                    break;
                }
                if mine && turn_ended(&ev, me) {
                    break;
                }
            }
        }
        Ok(None)
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

    /// Walks through doors until in `(region, room)`. Each hop is a `door`
    /// walk on the real game, rolled back when it fails or leads
    /// elsewhere; the route is then searched again without that door. A
    /// fallback route may be no longer than what the first route left
    /// plus [`DETOUR`] doors, and never goes back through a place this
    /// `goto` passed or, once in the target region, out of it.
    fn goto(&mut self, me: usize, region: u8, room: u8, avoid_takes: bool) -> Result<(), String> {
        let want = (region, room);
        let mut bad: HashMap<Edge, String> = HashMap::new();
        let mut been: HashSet<(u8, u8)> = HashSet::new();
        let mut budget: Option<usize> = None;
        let tried = |bad: &HashMap<Edge, String>| {
            let mut t: Vec<String> = bad
                .iter()
                .map(|(e, why)| format!("door {},{} of {:?} to {:?}: {why}", e.at.0, e.at.1, e.from, e.to))
                .collect();
            t.sort();
            if t.is_empty() {
                String::new()
            } else {
                format!("; doors that didn't work: {}", t.join("; "))
            }
        };
        for _ in 0..HOPS_MAX {
            let here = place_key(self.at(me));
            if here == want {
                return Ok(());
            }
            been.insert(here);
            let stay = (want.0 != 0 && here.0 == want.0).then_some(want.0);
            let Some(route) = route(&self.game, here, want, &bad, &been, stay) else {
                let mut msg = format!(
                    "no door route from {} to region {region} room {room}",
                    place_name(&self.game, self.at(me))
                );
                if been.len() > 1 {
                    msg.push_str(" that keeps out of the places already passed");
                }
                if stay.is_some() {
                    msg.push_str(&format!(" without leaving region {region}"));
                }
                msg.push_str(&tried(&bad));
                return Err(msg);
            };
            match budget {
                Some(b) if route.len() > b => {
                    return Err(format!(
                        "the only way on from {} is {} doors, more than the {b} the first route left room for \
                         (it plus {DETOUR}){}",
                        place_name(&self.game, self.at(me)),
                        route.len(),
                        tried(&bad)
                    ));
                }
                Some(_) => {}
                None => budget = Some(route.len() + DETOUR),
            }
            let hop = route[0];
            let cp = self.checkpoint();
            match self.walk(me, Goal::Door(hop.at.0, hop.at.1), avoid_takes) {
                Ok(_) if place_key(self.at(me)) == hop.to => budget = budget.map(|b| b.saturating_sub(1)),
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
                        "expected long message {n} in the last action step; shown: {:?}",
                        self.messages
                    ));
                }
            }
            Expect::Alive => {
                if !p.alive {
                    return Err(format!("expected {name} alive"));
                }
            }
            Expect::Won => {
                if self.game.won().map(|w| w.player) != Some(me) {
                    return Err(format!("expected {name} to have won the adventure"));
                }
            }
            Expect::Dead => {
                if p.alive {
                    return Err(format!("expected {name} dead"));
                }
                self.deaths.retain(|&d| d != me);
            }
            Expect::Area(n) => {
                let here = match p.at {
                    Location::Room { region: 0, room, .. } if room >= 1 => Some(room),
                    _ => None,
                };
                if here.is_none() || n.is_some_and(|n| here != Some(n)) {
                    let want = n.map_or("an encounter area".into(), |n| format!("encounter area {n}"));
                    return Err(format!(
                        "expected {name} in {want}, but at {}",
                        place_name(&self.game, p.at)
                    ));
                }
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
            Action::Walk {
                x,
                y,
                avoid,
                encounter: false,
            } => {
                self.walk(me, Goal::Square(x, y), avoid)?;
            }
            Action::Walk {
                x,
                y,
                avoid,
                encounter: true,
            } => {
                self.walk(me, Goal::Encounter(x, y), avoid)?;
                if !in_area(self.at(me)) {
                    return Err(format!("the walk reached {x},{y} with no encounter on the way"));
                }
            }
            Action::Door(x, y, avoid) => {
                let ev = self.walk(me, Goal::Door(x, y), avoid)?;
                if !took_door(&ev, me) {
                    return Err(format!("no door was taken at {x},{y}"));
                }
            }
            Action::Goto {
                region,
                room,
                at,
                avoid,
            } => {
                self.goto(me, region, room, avoid)?;
                if let Some((x, y)) = at {
                    self.walk(me, Goal::Square(x, y), avoid)?;
                }
            }
            Action::Take(x, y, avoid) => {
                // A take leaves the taker on its own square: push into it
                // from beside.
                if self.at(me).xy() == (x, y) {
                    return Err(format!("already standing on {x},{y}: step off it first"));
                }
                let sq = with_xy(self.at(me), x, y);
                if !beside(&self.game, self.at(me), sq) {
                    self.walk(me, Goal::Beside(x, y), avoid)?;
                }
                let ev = self.push(me, with_xy(self.at(me), x, y))?;
                if !took(&ev, me) {
                    let why: Vec<String> = own_part(&ev, me)
                        .iter()
                        .filter(|e| !matches!(e, Event::TurnEnded { .. } | Event::TurnStarted { .. }))
                        .map(|e| describe(&self.game, e))
                        .collect();
                    return Err(format!("nothing was taken at {x},{y}: {}", why.join(" ")));
                }
            }
            Action::Bump(x, y, avoid) => {
                let sq = with_xy(self.at(me), x, y);
                if !beside(&self.game, self.at(me), sq) {
                    self.walk(me, Goal::Beside(x, y), avoid)?;
                }
                self.push(me, with_xy(self.at(me), x, y))?;
            }
            Action::Attack(x, y, turns) => self.attack(me, x, y, turns.unwrap_or(ATTACK_TURNS))?,
            Action::Fire(x, y, turns) => self.fire(me, x, y, turns.unwrap_or(ATTACK_TURNS))?,
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
        self.fresh.fill(true);
    }

    /// Attacks whoever stands on `(x, y)`: that individual (by its `Who`),
    /// followed wherever it goes in the room, one turn's walk at a time.
    fn attack(&mut self, me: usize, x: u8, y: u8, turns: u32) -> Result<(), String> {
        let who = self
            .who_at(me, x, y)
            .ok_or_else(|| format!("no one stands on {x},{y}"))?;
        let start = self.turns[me];
        for _ in 0..WAIT_MAX {
            if !self.game.alive(who) {
                return Ok(());
            }
            if self.turns[me] - start >= turns as usize {
                self.progress.push(format!(
                    "  (attack stopped after {turns} turns; the foe still lives)"
                ));
                return Ok(());
            }
            self.turn_for(me)?;
            if self.zero_moves_waited(me) {
                continue;
            }
            if !self.game.alive(who) {
                return Ok(());
            }
            let foe = self.game.location(who).ok_or("the foe is gone")?;
            if place_key(foe) != place_key(self.at(me)) {
                return Err(format!("the foe left for {}", place_name(&self.game, foe)));
            }
            if !beside(&self.game, self.at(me), foe) {
                self.walk_turns(me, Goal::Foe(who), false, 1)?;
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
        }
        Err("the attack never ended".into())
    }

    /// Fires the readied missile weapon at whoever stands on `(x, y)`: that
    /// individual, aimed at where it stands at each of `me`'s turns
    /// (play.md 3.1). The engine's refusals fail the step.
    fn fire(&mut self, me: usize, x: u8, y: u8, turns: u32) -> Result<(), String> {
        let who = self
            .who_at(me, x, y)
            .ok_or_else(|| format!("no one stands on {x},{y}"))?;
        let start = self.turns[me];
        for _ in 0..WAIT_MAX {
            if !self.game.alive(who) {
                return Ok(());
            }
            if self.turns[me] - start >= turns as usize {
                self.progress.push(format!(
                    "  (fire stopped after {turns} turns; the foe still lives)"
                ));
                return Ok(());
            }
            self.turn_for(me)?;
            if self.zero_moves_waited(me) {
                continue;
            }
            if !self.game.alive(who) {
                return Ok(());
            }
            let foe = self.game.location(who).ok_or("the foe is gone")?;
            if place_key(foe) != place_key(self.at(me)) {
                return Err(format!("the foe left for {}", place_name(&self.game, foe)));
            }
            let ev = self.apply(Command::Fire {
                target: Some(foe.xy()),
                weapon: None,
            });
            self.refused(&ev)?;
            if ev
                .iter()
                .any(|e| matches!(e, Event::FireCancelled { player } if *player == me))
            {
                return Err("no missile weapon is readied (FIRE chose NONE)".into());
            }
            if !ev
                .iter()
                .any(|e| matches!(e, Event::Fired { by, .. } if *by == (Who::Player { player: me })))
            {
                let why: Vec<String> = ev.iter().map(|e| describe(&self.game, e)).collect();
                return Err(format!("FIRE didn't shoot: {}", why.join(" ")));
            }
        }
        Err("the firing never ended".into())
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
            // Long messages are kept from the start of an action step
            // through the `expect`s after it.
            if !matches!(step.action, Action::Expect(_) | Action::As(_)) {
                self.messages.clear();
            }
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
        }
        None
    }
}

/// The shortest door route from `from` to `to` over the doors the rooms
/// and the world map hold now, leaving out `bad` ones, places in
/// `closed` and, with `stay`, places outside that region.
fn route(
    game: &Game,
    from: (u8, u8),
    to: (u8, u8),
    bad: &HashMap<Edge, String>,
    closed: &HashSet<(u8, u8)>,
    stay: Option<u8>,
) -> Option<Vec<Edge>> {
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
            if bad.contains_key(&e)
                || closed.contains(&e.to)
                || stay.is_some_and(|r| e.to.0 != r)
                || !seen.insert(e.to)
            {
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
