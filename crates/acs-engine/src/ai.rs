// SPDX-License-Identifier: GPL-3.0-or-later

//! One creature's turn: the overburdened drop, readying, choosing a goal
//! and walking towards it. Spec: `docs/rules/creatures.md` sections 5-8.

use crate::creatures::{self, is_fighter, is_foe, is_wounded, Other};
use crate::game::Game;
use crate::inventory::{self, GOLD, GOLD_LIMIT, PILE_LIMIT};
use crate::{Dir, Event, Location, RemovalReason, Who};
use acs_core::model::{Access, Alignment, Creature, Destination, Possession, SpellActionKind, ThingKind};
use std::collections::VecDeque;

use Alignment::{Enemy, Friend, Neutral, Thief};

/// A goal category from the manual's lists (6.1).
#[derive(Debug, Clone, Copy)]
enum Cat {
    Player,
    /// Another creature of one of these alignments, Aggressive or not.
    Ind(&'static [Alignment], bool),
    Exit,
    Gold,
    Treasure,
}

/// Each type's list: ranks in order, categories within a rank equal.
type List = &'static [&'static [Cat]];

const FIGHTER_ENEMY: List = &[
    &[Cat::Player],
    &[Cat::Ind(&[Friend], true)],
    &[Cat::Ind(&[Friend], false)],
    &[Cat::Exit],
    &[Cat::Ind(&[Neutral, Thief], true)],
    &[Cat::Ind(&[Neutral, Thief], false)],
    &[Cat::Gold],
];
const SLINKER_ENEMY: List = &[
    &[Cat::Ind(&[Enemy], false)],
    &[Cat::Ind(&[Enemy], true)],
    &[Cat::Exit],
    &[Cat::Gold],
];
const FIGHTER_FRIEND: List = &[
    &[Cat::Ind(&[Enemy], true)],
    &[Cat::Ind(&[Neutral], true)],
    &[Cat::Ind(&[Enemy], false)],
    &[Cat::Ind(&[Thief], true)],
    &[Cat::Player],
    &[Cat::Exit],
];
const SLINKER_FRIEND: List = &[&[Cat::Player], &[Cat::Exit]];
const FIGHTER_NEUTRAL: List = &[
    &[Cat::Ind(&[Friend, Enemy], true)],
    &[Cat::Ind(&[Thief], true)],
    &[Cat::Ind(&[Friend, Enemy], false)],
    &[Cat::Ind(&[Thief], false), Cat::Ind(&[Neutral], true)],
    &[Cat::Ind(&[Neutral], false)],
    &[Cat::Gold],
    &[Cat::Exit],
];
const SLINKER_NEUTRAL: List = &[&[Cat::Exit]];
const THIEF: List = &[&[Cat::Treasure, Cat::Gold], &[Cat::Exit]];

fn list(c: &Creature) -> List {
    match (c.strategy.alignment, is_fighter(c)) {
        (Enemy, true) => FIGHTER_ENEMY,
        (Enemy, false) => SLINKER_ENEMY,
        (Friend, true) => FIGHTER_FRIEND,
        (Friend, false) => SLINKER_FRIEND,
        (Neutral, true) => FIGHTER_NEUTRAL,
        (Neutral, false) => SLINKER_NEUTRAL,
        (Thief, _) => THIEF,
    }
}

/// What a square is to a creature (5.1-5.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sq {
    Open,
    /// Gold or a treasure it picks up (room item index).
    Take(usize),
    /// A door it may take (room item index).
    Door(usize),
    Closed,
}

/// What the creature is heading for.
#[derive(Debug, Clone, Copy)]
enum Target {
    Individual(Other),
    /// A door or a thing to pick up: reached by stepping on it.
    Square,
}

struct Choice {
    target: Target,
    dist: u16,
    /// Squares that count as reaching the goal.
    ends: Vec<(u8, u8)>,
    /// The goal was a player (for 7.3).
    player: bool,
}

const FAR: u16 = u16::MAX;

impl Game {
    /// One creature's turn in a creature phase (2.5, 8.1, 8.2, 6.6).
    pub(crate) fn creature_turn(&mut self, region: u8, slot: usize, ev: &mut Vec<Event>) {
        self.overburdened_drop(region, slot, ev);
        self.creature_ready(region, slot, ev);
        let Some(c) = self.creature(region, slot) else {
            return;
        };
        let moves = creatures::effective_speed(&self.adv, c);
        ev.push(Event::CreatureTurn { region, slot, moves });
        for _ in 0..moves {
            if !self.creature_step(region, slot, ev) {
                break;
            }
        }
    }

    /// Classifies every square of the creature's room for it.
    fn squares(&self, region: u8, slot: usize) -> Option<(Vec<Sq>, u8, u8)> {
        let c = self.creature(region, slot)?;
        let r = self.room(region, c.room)?;
        let (w, h) = (r.width, r.height);
        let over = creatures::load(&self.adv, c) > creatures::capacity(c);
        let mut grid = vec![Sq::Closed; w as usize * h as usize];
        let me = Who::Creature { region, slot };
        let others = self.others_in_room(region, c.room, Some(me));
        for y in 0..h {
            for x in 0..w {
                let edge = x == 0 || y == 0 || x == w - 1 || y == h - 1;
                let k = y as usize * w as usize + x as usize;
                if others.iter().any(|o| o.x == x && o.y == y) {
                    continue;
                }
                let top = r.items.iter().position(|it| it.x == x && it.y == y);
                let Some(i) = top else {
                    if !edge {
                        grid[k] = Sq::Open;
                    }
                    continue;
                };
                let it = &r.items[i];
                let Some(t) = self.thing(it.thing) else {
                    if !edge {
                        grid[k] = Sq::Open;
                    }
                    continue;
                };
                grid[k] = match &t.kind {
                    kind if inventory::carriable(kind) => {
                        let room_for = if it.thing == GOLD {
                            c.wealth < GOLD_LIMIT
                        } else {
                            creatures::count(c, it.thing) < inventory::limit(it.thing)
                        };
                        if !over && room_for && creatures::picks_up(c, it.thing, kind) {
                            Sq::Take(i)
                        } else {
                            Sq::Closed
                        }
                    }
                    ThingKind::Portal { access, .. }
                        if self.door_open_to(c, region, access, it.destination) =>
                    {
                        Sq::Door(i)
                    }
                    ThingKind::RoomFloor { .. } | ThingKind::Undefined | ThingKind::Unknown { .. }
                        if !edge =>
                    {
                        Sq::Open
                    }
                    _ => Sq::Closed,
                };
            }
        }
        // The creature's own square is where it starts, whatever lies there.
        grid[c.y as usize * w as usize + c.x as usize] = Sq::Open;
        Some((grid, w, h))
    }

    /// 5.3: same region, no forbidden spell, access as for players.
    fn door_open_to(&self, c: &Creature, region: u8, access: &Access, dest: Option<Destination>) -> bool {
        if !matches!(dest, Some(Destination::Room { region: r, .. }) if r == region) {
            return false;
        }
        match *access {
            Access::Anyone => true,
            Access::InvokeSpell { action } => !matches!(
                action.kind,
                SpellActionKind::ActivateAll | SpellActionKind::RidRoomOf | SpellActionKind::AddToRoom
            ),
            Access::OwnersOf { thing } => creatures::count(c, thing) > 0,
            Access::NonOwnersOf { thing } => creatures::count(c, thing) == 0,
        }
    }

    fn dirs(&self) -> impl Iterator<Item = Dir> + '_ {
        Dir::ALL
            .into_iter()
            .filter(|d| self.rules.diagonals || !d.is_diagonal())
    }

    /// Breadth-first distances from `sources` over open squares; door and
    /// take squares are entered but doors are not passed through.
    fn bfs(&self, grid: &[Sq], w: u8, h: u8, sources: &[(u8, u8)]) -> Vec<u16> {
        let mut dist = vec![FAR; grid.len()];
        let mut q = VecDeque::new();
        for &(x, y) in sources {
            let k = y as usize * w as usize + x as usize;
            if dist[k] == FAR {
                dist[k] = 0;
                q.push_back((x, y));
            }
        }
        while let Some((x, y)) = q.pop_front() {
            let k = y as usize * w as usize + x as usize;
            if matches!(grid[k], Sq::Door(_)) && dist[k] > 0 {
                continue;
            }
            for d in self.dirs() {
                let (dx, dy) = d.delta();
                let (nx, ny) = (x as i16 + dx as i16, y as i16 + dy as i16);
                if !(0..w as i16).contains(&nx) || !(0..h as i16).contains(&ny) {
                    continue;
                }
                let n = ny as usize * w as usize + nx as usize;
                if grid[n] != Sq::Closed && dist[n] == FAR {
                    dist[n] = dist[k] + 1;
                    q.push_back((nx as u8, ny as u8));
                }
            }
        }
        dist
    }

    /// 6.1-6.5: the nearest goal on the creature's list.
    fn choose(&self, region: u8, slot: usize, grid: &[Sq], w: u8, h: u8) -> Option<Choice> {
        let c = self.creature(region, slot)?;
        let me = Who::Creature { region, slot };
        let here = (c.x, c.y);
        let dist = self.bfs(grid, w, h, &[here]);
        let at = |x: u8, y: u8| y as usize * w as usize + x as usize;
        let others = self.others_in_room(region, c.room, Some(me));
        let r = self.room(region, c.room)?;

        let mut ranks: Vec<&[Cat]> = list(c).to_vec();
        if !is_fighter(c) && is_wounded(c) {
            // 6.2: a wounded slinker heads for an exit first.
            ranks.insert(0, &[Cat::Exit]);
        }
        // (distance, rank, order, is an exit, choice)
        let mut found: Vec<(u16, usize, usize, bool, Choice)> = Vec::new();
        let mut offer = |rank: usize, order: usize, exit: bool, choice: Choice| {
            if choice.dist != FAR {
                found.push((choice.dist, rank, order, exit, choice));
            }
        };
        let individual = |o: &Other| -> (u16, Vec<(u8, u8)>) {
            let mut ends = Vec::new();
            let mut d = FAR;
            for dir in Dir::ALL {
                let (dx, dy) = dir.delta();
                let (nx, ny) = (o.x as i16 + dx as i16, o.y as i16 + dy as i16);
                if !(0..w as i16).contains(&nx) || !(0..h as i16).contains(&ny) {
                    continue;
                }
                let (nx, ny) = (nx as u8, ny as u8);
                if !matches!(grid[at(nx, ny)], Sq::Open | Sq::Take(_)) {
                    continue;
                }
                ends.push((nx, ny));
                d = d.min(dist[at(nx, ny)]);
            }
            (d, ends)
        };
        let mut order = 0;
        for (rank, cats) in ranks.iter().enumerate() {
            for cat in cats.iter() {
                match *cat {
                    Cat::Player | Cat::Ind(..) => {
                        for o in &others {
                            let wanted = match (*cat, o.alignment) {
                                (Cat::Player, None) => true,
                                (Cat::Ind(al, aggr), Some(a)) => al.contains(&a) && aggr == o.aggressive,
                                _ => false,
                            };
                            order += 1;
                            if wanted {
                                let (d, ends) = individual(o);
                                offer(
                                    rank,
                                    order,
                                    false,
                                    Choice {
                                        target: Target::Individual(*o),
                                        dist: d,
                                        ends,
                                        player: o.alignment.is_none(),
                                    },
                                );
                            }
                        }
                        if matches!(cat, Cat::Player) {
                            // 6.4: a player who left by a door is sought there.
                            for &(rg, rm, x, y) in &self.turn_exits {
                                order += 1;
                                if (rg, rm) == (region, c.room) && matches!(grid[at(x, y)], Sq::Door(_)) {
                                    offer(
                                        rank,
                                        order,
                                        false,
                                        Choice {
                                            target: Target::Square,
                                            dist: dist[at(x, y)],
                                            ends: vec![(x, y)],
                                            player: false,
                                        },
                                    );
                                }
                            }
                        }
                    }
                    Cat::Exit | Cat::Gold | Cat::Treasure => {
                        for y in 0..h {
                            for x in 0..w {
                                order += 1;
                                let wanted = match (*cat, grid[at(x, y)]) {
                                    (Cat::Exit, Sq::Door(_)) => true,
                                    (Cat::Gold, Sq::Take(i)) => r.items[i].thing == GOLD,
                                    (Cat::Treasure, Sq::Take(i)) => r.items[i].thing != GOLD,
                                    _ => false,
                                };
                                if wanted {
                                    offer(
                                        rank,
                                        order,
                                        matches!(cat, Cat::Exit),
                                        Choice {
                                            target: Target::Square,
                                            dist: dist[at(x, y)],
                                            ends: vec![(x, y)],
                                            player: false,
                                        },
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
        // 6.3: an exit counts only when nothing ranked above it is reachable.
        let exit_rank = ranks
            .iter()
            .position(|r| r.iter().any(|c| matches!(c, Cat::Exit)));
        let above = found.iter().any(|f| !f.3 && exit_rank.is_some_and(|e| f.1 < e));
        found
            .into_iter()
            .filter(|f| !(above && f.3))
            .min_by_key(|f| (f.0, f.1, f.2))
            .map(|f| f.4)
    }

    /// One step (or the end of the turn). True if the creature moved and may
    /// go on.
    fn creature_step(&mut self, region: u8, slot: usize, ev: &mut Vec<Event>) -> bool {
        let Some((grid, w, h)) = self.squares(region, slot) else {
            return false;
        };
        let Some(choice) = self.choose(region, slot, &grid, w, h) else {
            return false; // 6.6: nothing to seek; stays put.
        };
        let Some(c) = self.creature(region, slot).cloned() else {
            return false;
        };
        if choice.dist == 0 {
            if let Target::Individual(o) = choice.target {
                // 7.3: a friend next to the player it sought.
                if choice.player && c.strategy.alignment == Friend {
                    self.friend_drop(region, slot, o, ev);
                }
                // 7.1: a fighter next to a foe.
                if is_fighter(&c) && is_foe(&c, &o) {
                    ev.push(Event::CreatureConfronts {
                        region,
                        slot,
                        target: o.who,
                    });
                }
            }
            return false; // 7.2
        }
        let back = self.bfs(&grid, w, h, &choice.ends);
        let at = |x: u8, y: u8| y as usize * w as usize + x as usize;
        let step = self.dirs().find_map(|d| {
            let (dx, dy) = d.delta();
            let (nx, ny) = (c.x as i16 + dx as i16, c.y as i16 + dy as i16);
            if !(0..w as i16).contains(&nx) || !(0..h as i16).contains(&ny) {
                return None;
            }
            let (nx, ny) = (nx as u8, ny as u8);
            let k = at(nx, ny);
            (grid[k] != Sq::Closed && back[k] == choice.dist - 1).then_some((nx, ny, grid[k]))
        });
        let Some((nx, ny, sq)) = step else {
            return false;
        };
        let from = Location::Room {
            region,
            room: c.room,
            x: c.x,
            y: c.y,
        };
        let to = Location::Room {
            region,
            room: c.room,
            x: nx,
            y: ny,
        };
        match sq {
            Sq::Door(i) => {
                self.creature_take_door(region, slot, to, i, ev);
                false
            }
            Sq::Take(i) => {
                self.creature_pick_up(region, slot, i, ev);
                self.set_creature_xy(region, slot, nx, ny);
                ev.push(Event::CreatureMoved {
                    region,
                    slot,
                    from,
                    to,
                });
                true
            }
            _ => {
                self.set_creature_xy(region, slot, nx, ny);
                ev.push(Event::CreatureMoved {
                    region,
                    slot,
                    from,
                    to,
                });
                true
            }
        }
    }

    fn set_creature_xy(&mut self, region: u8, slot: usize, x: u8, y: u8) {
        if let Some(c) = self.creature_mut(region, slot) {
            c.x = x;
            c.y = y;
        }
    }

    /// 7.5: gold adds to wealth (the rest stays); a treasure is taken.
    fn creature_pick_up(&mut self, region: u8, slot: usize, index: usize, ev: &mut Vec<Event>) {
        let Some(room) = self.creature(region, slot).map(|c| c.room) else {
            return;
        };
        let Some(item) = self.room(region, room).and_then(|r| r.items.get(index)).cloned() else {
            return;
        };
        if item.thing == GOLD {
            let pile = (item.parameter as u16).max(1);
            let wealth = self.creature(region, slot).map_or(0, |c| c.wealth);
            let amount = pile.min(GOLD_LIMIT - wealth);
            if let Some(c) = self.creature_mut(region, slot) {
                c.wealth += amount;
            }
            if amount < pile {
                if let Some(it) = self
                    .region_mut(region)
                    .and_then(|r| r.rooms.get_mut(room as usize))
                    .and_then(|r| r.items.get_mut(index))
                {
                    it.parameter = (pile - amount) as u8;
                }
            } else {
                self.remove_item(region, room, index);
            }
            ev.push(Event::CreatureGoldPickedUp { region, slot, amount });
        } else {
            self.remove_item(region, room, index);
            if let Some(c) = self.creature_mut(region, slot) {
                add_possession(c, item.thing);
            }
            ev.push(Event::CreaturePickedUp {
                region,
                slot,
                thing: item.thing,
            });
        }
    }

    /// 5.4, 5.5: through a door to a room of the same region.
    fn creature_take_door(
        &mut self,
        region: u8,
        slot: usize,
        door: Location,
        index: usize,
        ev: &mut Vec<Event>,
    ) {
        let Some(c) = self.creature(region, slot).cloned() else {
            return;
        };
        let Some(item) = self
            .room(region, c.room)
            .and_then(|r| r.items.get(index))
            .cloned()
        else {
            return;
        };
        let Some(Destination::Room { room, x, y, .. }) = item.destination else {
            return;
        };
        let Some(r) = self.room(region, room) else {
            self.remove_creature(region, slot, RemovalReason::Banished, ev);
            return;
        };
        if x >= r.width || y >= r.height {
            self.remove_creature(region, slot, RemovalReason::Banished, ev);
            return;
        }
        let Some((lx, ly)) = self.landing(region, room, x, y) else {
            return; // Nowhere to stand: it stays and its turn ends.
        };
        // Access "owners of" with "destroy needed thing" uses one up.
        if let Some(ThingKind::Portal {
            access: Access::OwnersOf { thing },
            destroy_needed_thing: true,
            ..
        }) = self.thing(item.thing).map(|t| t.kind.clone())
        {
            if let Some(cm) = self.creature_mut(region, slot) {
                if take_possession(cm, thing) {
                    ev.push(Event::CreatureUsedUp { region, slot, thing });
                }
            }
        }
        if let Some(cm) = self.creature_mut(region, slot) {
            cm.room = room;
            cm.x = lx;
            cm.y = ly;
        }
        ev.push(Event::CreatureTookDoor {
            region,
            slot,
            from: door,
            to: Location::Room {
                region,
                room,
                x: lx,
                y: ly,
            },
        });
    }

    /// 5.4: the destination square if no one stands there, else the nearest
    /// inside square with no individual and no thing.
    pub(crate) fn landing(&self, region: u8, room: u8, x: u8, y: u8) -> Option<(u8, u8)> {
        if !self.occupied(region, room, x, y) {
            return Some((x, y));
        }
        let r = self.room(region, room)?;
        let (w, h) = (r.width as i16, r.height as i16);
        for d in 1..w.max(h) {
            for ny in (y as i16 - d)..=(y as i16 + d) {
                for nx in (x as i16 - d)..=(x as i16 + d) {
                    if (nx - x as i16).abs().max((ny - y as i16).abs()) != d {
                        continue;
                    }
                    if nx <= 0 || ny <= 0 || nx >= w - 1 || ny >= h - 1 {
                        continue;
                    }
                    let (nx, ny) = (nx as u8, ny as u8);
                    let has_item = r.items.iter().any(|it| it.x == nx && it.y == ny);
                    if !has_item && !self.occupied(region, room, nx, ny) {
                        return Some((nx, ny));
                    }
                }
            }
        }
        None
    }

    /// 8.1: one drop a turn while load is above capacity.
    fn overburdened_drop(&mut self, region: u8, slot: usize, ev: &mut Vec<Event>) {
        let Some(c) = self.creature(region, slot).cloned() else {
            return;
        };
        let (load, cap) = (creatures::load(&self.adv, &c), creatures::capacity(&c));
        if load <= cap {
            return;
        }
        let mut owned: Vec<u8> = c
            .possessions
            .iter()
            .filter(|p| p.count > 0)
            .map(|p| p.thing)
            .collect();
        owned.sort_unstable();
        let of_kind = |magic: bool| {
            owned.iter().copied().find(|&t| {
                matches!(
                    (self.thing(t).map(|t| &t.kind), magic),
                    (Some(ThingKind::MagicItem { .. }), true) | (Some(ThingKind::Treasure { .. }), false)
                ) && t != GOLD
            })
        };
        let gold_weight = inventory::weight(&self.adv, GOLD);
        let (thing, amount) = if let Some(t) = of_kind(true).or_else(|| of_kind(false)) {
            (t, 1)
        } else if c.wealth > 0 && gold_weight > 0 {
            let need = ((load - cap) + gold_weight - 1) / gold_weight;
            let n = (need.max(1) as u16).min(PILE_LIMIT).min(c.wealth);
            (GOLD, n)
        } else {
            return;
        };
        if let Some(cm) = self.creature_mut(region, slot) {
            if thing == GOLD {
                cm.wealth -= amount;
            } else {
                take_possession(cm, thing);
            }
        }
        let vanishes = matches!(
            self.thing(thing).map(|t| &t.kind),
            Some(ThingKind::Treasure {
                disappears_when_dropped: true,
                ..
            })
        );
        let at = Location::Room {
            region,
            room: c.room,
            x: c.x,
            y: c.y,
        };
        if !vanishes {
            self.put_on_top(region, c.room, c.x, c.y, thing, amount as u8);
        }
        ev.push(Event::CreatureDropped {
            region,
            slot,
            thing,
            amount,
            at: (!vanishes).then_some(at),
        });
    }

    /// 8.2: a missile weapon when safe, else a melee weapon; lowest number.
    fn creature_ready(&mut self, region: u8, slot: usize, ev: &mut Vec<Event>) {
        let Some(c) = self.creature(region, slot).cloned() else {
            return;
        };
        let me = Who::Creature { region, slot };
        let safe = !self.others_in_room(region, c.room, Some(me)).iter().any(|o| {
            (o.x as i16 - c.x as i16).abs() <= 1 && (o.y as i16 - c.y as i16).abs() <= 1 && is_foe(&c, o)
        });
        let mut weapons: Vec<u8> = c
            .possessions
            .iter()
            .filter(|p| p.count > 0)
            .map(|p| p.thing)
            .collect();
        if c.weapon != 0 {
            weapons.push(c.weapon);
        }
        weapons.sort_unstable();
        weapons.dedup();
        let find = |missile: bool| {
            weapons.iter().copied().find(|&t| {
                matches!(
                    (self.thing(t).map(|t| &t.kind), missile),
                    (Some(ThingKind::MissileWeapon { .. }), true)
                        | (Some(ThingKind::MeleeWeapon { .. }), false)
                )
            })
        };
        let pick = if safe {
            find(true).or_else(|| find(false))
        } else {
            find(false)
        };
        if let Some(t) = pick {
            if t != c.weapon {
                if let Some(cm) = self.creature_mut(region, slot) {
                    cm.weapon = t;
                }
                ev.push(Event::CreatureReadied {
                    region,
                    slot,
                    thing: t,
                });
            }
        }
    }

    /// 7.3: the lowest-numbered magic item, next to the player if possible.
    fn friend_drop(&mut self, region: u8, slot: usize, player: Other, ev: &mut Vec<Event>) {
        let Some(c) = self.creature(region, slot).cloned() else {
            return;
        };
        let mut items: Vec<u8> = c
            .possessions
            .iter()
            .filter(|p| p.count > 0)
            .map(|p| p.thing)
            .filter(|&t| matches!(self.thing(t).map(|t| &t.kind), Some(ThingKind::MagicItem { .. })))
            .collect();
        items.sort_unstable();
        let Some(&thing) = items.first() else {
            return;
        };
        let Some(r) = self.room(region, c.room) else {
            return;
        };
        let (w, h) = (r.width, r.height);
        let accepts = |x: u8, y: u8| {
            if x == 0 || y == 0 || x >= w - 1 || y >= h - 1 || self.occupied(region, c.room, x, y) {
                return false;
            }
            match r.items.iter().find(|it| it.x == x && it.y == y) {
                None => true,
                Some(it) => !matches!(
                    self.thing(it.thing).map(|t| &t.kind),
                    Some(
                        ThingKind::Portal { .. }
                            | ThingKind::Obstacle { .. }
                            | ThingKind::CustomObstacle { .. }
                            | ThingKind::Store { .. }
                    )
                ),
            }
        };
        let spot = Dir::ALL
            .into_iter()
            .filter_map(|d| self.neighbour(region, c.room, player.x, player.y, d))
            .find(|&(x, y)| accepts(x, y))
            .unwrap_or((c.x, c.y));
        if let Some(cm) = self.creature_mut(region, slot) {
            take_possession(cm, thing);
        }
        self.put_on_top(region, c.room, spot.0, spot.1, thing, 1);
        ev.push(Event::CreatureDropped {
            region,
            slot,
            thing,
            amount: 1,
            at: Some(Location::Room {
                region,
                room: c.room,
                x: spot.0,
                y: spot.1,
            }),
        });
    }
}

fn add_possession(c: &mut Creature, thing: u8) {
    if let Some(p) = c.possessions.iter_mut().find(|p| p.thing == thing) {
        p.count += 1;
    } else {
        c.possessions.push(Possession { thing, count: 1 });
    }
}

/// Removes one; clears the readied slot when the last goes. False if none.
fn take_possession(c: &mut Creature, thing: u8) -> bool {
    if thing == GOLD {
        if c.wealth == 0 {
            return false;
        }
        c.wealth -= 1;
        return true;
    }
    let Some(i) = c.possessions.iter().position(|p| p.thing == thing && p.count > 0) else {
        return false;
    };
    c.possessions[i].count -= 1;
    if c.possessions[i].count == 0 {
        c.possessions.remove(i);
        if c.weapon == thing {
            c.weapon = 0;
        }
        if c.armor == thing {
            c.armor = 0;
        }
    }
    true
}
