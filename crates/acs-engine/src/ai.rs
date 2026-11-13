// SPDX-License-Identifier: GPL-3.0-or-later

//! One creature's turn: the overburdened drop, readying, casting, choosing
//! a goal and walking towards it, and stepping into an individual to
//! attack it. Spec: `docs/rules/creatures.md` sections 5-8,
//! `docs/rules/magic.md` section 6 and `docs/rules/combat.md` section 2.

use crate::combat::strike_cost;
use crate::creatures::{self, is_fighter, is_foe, Other};
use crate::game::Game;
use crate::inventory::{self, GOLD, GOLD_LIMIT};
use crate::magic::{self, add_possession, take_possession, Spell};
use crate::{Dir, Event, Location, RemovalReason, SpellSource, Who};
use acs_core::model::{Access, Alignment, Creature, Destination, SpellActionKind, ThingKind};

use acs_core::model::ThingId;
use Alignment::{Enemy, Friend, Neutral, Thief};

/// A rank that never makes a goal (6.3, 6.4).
const NONE: u16 = u16::MAX;
/// A best score at or above this is no goal (6.2).
const NO_GOAL: u16 = 200;

/// 6.3: the rank of individual `o` for creature `c` (lower is more wanted).
fn rank_individual(c: &Creature, o: &Other) -> u16 {
    let fighter = is_fighter(c);
    // Columns: player, A friend, P friend, A enemy, P enemy, A neutral,
    // P neutral, A thief, P thief.
    let col = match o.alignment {
        None => 0,
        Some(a) => {
            let base = match a {
                Friend => 1,
                Enemy => 3,
                Neutral => 5,
                Thief => 7,
            };
            base + usize::from(!o.aggressive)
        }
    };
    const N: u16 = NONE;
    let row: [u16; 9] = match (c.strategy.alignment, fighter) {
        (Enemy, true) => [0, 4, 6, N, N, 80, 84, 80, 84],
        (Friend, true) => [32, N, N, 0, 4, 3, N, 5, N],
        (Neutral, true) => [1, 0, 4, 0, 4, 6, 8, 2, 6],
        (Enemy, false) => [N, N, N, 29, 25, N, N, N, N],
        (Friend, false) => [26, N, N, N, N, N, N, N, N],
        (Thief, _) | (Neutral, false) => [N; 9],
    };
    row[col]
}

/// 6.4: the ranks of an exit, gold and other treasure for creature `c`.
fn rank_things(c: &Creature) -> (u16, u16, u16) {
    match (c.strategy.alignment, is_fighter(c)) {
        (Enemy, true) => (96, 128, NONE),
        (Neutral, true) => (96, 96, NONE),
        (Friend, true) => (96, NONE, NONE),
        (Thief, _) => (96, 32, 32),
        (Enemy, false) => (32, 96, NONE),
        (Friend, false) | (Neutral, false) => (32, NONE, NONE),
    }
}

fn steps(a: (u8, u8), b: (u8, u8)) -> u16 {
    (a.0.abs_diff(b.0) + a.1.abs_diff(b.1)) as u16
}

/// What a square is to a creature (5.1-5.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sq {
    Open,
    /// Gold or a treasure it picks up (room item index).
    Take(usize),
    /// A door it may take (room item index).
    Door(usize),
    /// An individual a fighter would strike, at this step cost (combat
    /// 2.2).
    Strike(Who, u16),
    Closed,
}

/// A turn's goal (6.2): its square, and the individual when it is one.
#[derive(Debug, Clone, Copy)]
struct Goal {
    at: (u8, u8),
    individual: Option<Other>,
}

/// The 4 squares beside, in the order a step looks at them (6.5).
const STEP_DIRS: [Dir; 4] = [Dir::S, Dir::N, Dir::E, Dir::W];

impl Game {
    /// One creature's turn (2.5, 8.1, 8.2, 6.6), with drift and the healing
    /// die at its top and a healing roll for each move it leaves unused
    /// (magic 5.1, 5.3, 5.4).
    pub(crate) fn creature_turn(&mut self, region: u8, slot: usize, ev: &mut Vec<Event>) {
        let me = Who::Creature { region, slot };
        if let Some((left, full)) = self.creature_acts(region, slot, ev) {
            self.idle_moves(me, left, full, ev);
        }
    }

    /// The turn itself; the moves left unused and the turn's full moves,
    /// unless it cast or used an item (or died).
    fn creature_acts(&mut self, region: u8, slot: usize, ev: &mut Vec<Event>) -> Option<(u8, u8)> {
        let dropped = self.overburdened_drop(region, slot, ev);
        let c = self.creature(region, slot)?; // its drop spell may have killed it
        let moves = creatures::effective_speed(&self.adv, c);
        ev.push(Event::CreatureTurn { region, slot, moves });
        self.turn_top(Who::Creature { region, slot }, ev);
        self.creature(region, slot)?;
        // Combat 6.2: readying, a move for each change.
        let mut left = moves;
        if self.creature_ready(region, slot, ev) {
            left = left.saturating_sub(1);
        }
        // 8.1: a turn with a drop takes no steps; its moves are idled.
        if dropped {
            return Some((left, moves));
        }
        // Magic 6.3: casting (or using an item) takes the whole turn.
        if self.creature_magic(region, slot, ev) {
            return None;
        }
        // 6.2: one goal for the whole turn.
        let goal = self
            .squares(region, slot)
            .and_then(|(grid, w, h)| self.choose_goal(region, slot, &grid, w, h));
        let start = self.creature(region, slot).map(|c| (c.x, c.y))?;
        let mut stood = vec![start];
        while left > 0 {
            if !self.creature_step(region, slot, goal, start, &mut stood, &mut left, moves, ev) {
                break;
            }
            left -= 1;
        }
        self.creature(region, slot)?;
        Some((left, moves))
    }

    /// Magic 6.1-6.5: a smart fighter casts its lowest-numbered fitting
    /// spell at the individual it is after (Power at least Wisdom), or else
    /// uses a fitting magic item (Power at least half Wisdom). True if it
    /// did, which ends its turn.
    fn creature_magic(&mut self, region: u8, slot: usize, ev: &mut Vec<Event>) -> bool {
        let me = Who::Creature { region, slot };
        let Some(c) = self.creature(region, slot).cloned() else {
            return false;
        };
        if !is_fighter(&c) || c.power < c.wisdom / 2 {
            return false;
        }
        let Some((grid, w, h)) = self.squares(region, slot) else {
            return false;
        };
        let Some(Goal {
            individual: Some(goal),
            ..
        }) = self.choose_goal(region, slot, &grid, w, h)
        else {
            return false;
        };
        let foe = is_foe(&c, &goal);
        let mut owned: Vec<ThingId> = c
            .possessions
            .iter()
            .filter(|p| p.count > 0)
            .map(|p| p.thing)
            .collect();
        owned.sort_unstable();
        let find = |spell: bool| {
            owned
                .iter()
                .copied()
                .find(|&t| match self.thing(t).map(|t| &t.kind) {
                    Some(ThingKind::MagicSpell { action, .. }) => {
                        spell && magic::hurtful(*action) == Some(foe)
                    }
                    Some(ThingKind::MagicItem {
                        action, on_use: true, ..
                    }) => !spell && magic::hurtful(*action) == Some(foe),
                    _ => false,
                })
        };
        let pick = if c.power >= c.wisdom {
            find(true)
                .map(|t| (t, true))
                .or_else(|| find(false).map(|t| (t, false)))
        } else {
            find(false).map(|t| (t, false))
        };
        let Some((thing, is_spell)) = pick else {
            return false;
        };
        let smart = match c.wisdom {
            w if w > 14 => true,
            w if w < 6 => false,
            w => self.rng.percent((w - 5) * 10),
        };
        if !smart {
            return false;
        }
        if is_spell {
            self.cast(me, thing, goal.who, ev);
        } else {
            self.use_item(me, thing, goal.who, ev);
        }
        true
    }

    /// Magic 6.6: a magic item a creature drops on `at` casts its drop
    /// spell on the creature itself (it never lands on another creature).
    fn creature_drop_spell(
        &mut self,
        region: u8,
        slot: usize,
        thing: ThingId,
        at: Location,
        ev: &mut Vec<Event>,
    ) {
        let Some(t) = self.thing(thing).cloned() else {
            return;
        };
        if let ThingKind::MagicItem {
            action,
            on_drop: true,
            ..
        } = t.kind
        {
            let me = Who::Creature { region, slot };
            self.run_spell(
                Spell {
                    by: me,
                    target: me,
                    source: SpellSource::Dropped,
                    thing,
                    action,
                    message: self.message(&t.spell_message),
                    at,
                    stack: None,
                },
                ev,
            );
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
        // Combat 2.3: only a fighter owning a melee weapon strikes.
        let striker = is_fighter(c) && self.owned_melee(me).is_some();
        for y in 0..h {
            for x in 0..w {
                let edge = x == 0 || y == 0 || x == w - 1 || y == h - 1;
                let k = y as usize * w as usize + x as usize;
                if let Some(o) = others.iter().find(|o| o.x == x && o.y == y) {
                    if striker {
                        if let Some(cost) = strike_cost(c, o) {
                            grid[k] = Sq::Strike(o.who, cost);
                        }
                    }
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

    /// 6.2-6.4: the candidate with the lowest rank plus steps from the
    /// creature's square; individuals first (players, then creatures by
    /// slot), then the room's things in order; none under 200, no goal.
    fn choose_goal(&self, region: u8, slot: usize, grid: &[Sq], w: u8, _h: u8) -> Option<Goal> {
        let c = self.creature(region, slot)?;
        let me = Who::Creature { region, slot };
        let here = (c.x, c.y);
        let r = self.room(region, c.room)?;
        let mut best: Option<(u16, Goal)> = None;
        let mut offer = |score: u16, goal: Goal| {
            if best.is_none_or(|(b, _)| score < b) {
                best = Some((score, goal));
            }
        };
        for o in self.others_in_room(region, c.room, Some(me)) {
            let rank = rank_individual(c, &o);
            if rank != NONE {
                offer(
                    rank.saturating_add(steps(here, (o.x, o.y))),
                    Goal {
                        at: (o.x, o.y),
                        individual: Some(o),
                    },
                );
            }
        }
        let (exit, gold, treasure) = rank_things(c);
        for (i, it) in r.items.iter().enumerate() {
            let k = it.y as usize * w as usize + it.x as usize;
            let rank = match grid.get(k) {
                Some(Sq::Door(j)) if *j == i => exit,
                Some(Sq::Take(j)) if *j == i => {
                    if it.thing == GOLD {
                        gold
                    } else {
                        treasure
                    }
                }
                _ => NONE,
            };
            if rank != NONE {
                offer(
                    rank.saturating_add(steps(here, (it.x, it.y))),
                    Goal {
                        at: (it.x, it.y),
                        individual: None,
                    },
                );
            }
        }
        best.filter(|(score, _)| *score < NO_GOAL).map(|(_, g)| g)
    }

    /// One step (6.5) or an action that ends the turn (7, combat 2), with
    /// `left` of `full` moves left. True if the creature moved and may go
    /// on.
    #[allow(clippy::too_many_arguments)]
    fn creature_step(
        &mut self,
        region: u8,
        slot: usize,
        goal: Option<Goal>,
        start: (u8, u8),
        stood: &mut Vec<(u8, u8)>,
        left: &mut u8,
        full: u8,
        ev: &mut Vec<Event>,
    ) -> bool {
        let Some((grid, w, h)) = self.squares(region, slot) else {
            return false;
        };
        let Some(c) = self.creature(region, slot).cloned() else {
            return false;
        };
        // 7.3: a friend beside the player it chose.
        if let Some(Goal {
            individual: Some(o), ..
        }) = goal
        {
            if steps((c.x, c.y), (o.x, o.y)) == 1
                && o.alignment.is_none()
                && c.strategy.alignment == Friend
                && self.friend_drop(region, slot, o, ev)
            {
                return false;
            }
        }
        let target = goal.map_or(start, |g| g.at);
        let (exit_cost, ..) = if is_fighter(&c) { (3u16, 0, 0) } else { (1, 0, 0) };
        let (_, gold_rank, treasure_rank) = rank_things(&c);
        let r = self.room(region, c.room).cloned();
        let mut best: Option<(u16, u8, u8, Sq)> = None;
        for d in STEP_DIRS {
            let (dx, dy) = d.delta();
            let (nx, ny) = (c.x as i16 + dx as i16, c.y as i16 + dy as i16);
            if !(0..w as i16).contains(&nx) || !(0..h as i16).contains(&ny) {
                continue;
            }
            let (nx, ny) = (nx as u8, ny as u8);
            let sq = grid[ny as usize * w as usize + nx as usize];
            if sq == Sq::Closed {
                continue;
            }
            let roll = self.rng.below(3) as u16;
            let cost = if goal.is_some_and(|g| g.at == (nx, ny)) {
                1
            } else {
                match sq {
                    Sq::Strike(_, cost) => cost,
                    Sq::Door(_) => exit_cost,
                    Sq::Take(i) => {
                        let gold = r.as_ref().is_some_and(|r| r.items[i].thing == GOLD);
                        (if gold { gold_rank } else { treasure_rank }) / 32
                    }
                    _ if stood.contains(&(nx, ny)) => 4,
                    _ => 2,
                }
            };
            let score = 3 * steps((nx, ny), target) + 5 * cost + 4 * roll;
            if best.is_none_or(|b| score < b.0) {
                best = Some((score, nx, ny, sq));
            }
        }
        let Some((_, nx, ny, sq)) = best else {
            return false; // nowhere open: it stands (6.5)
        };
        if let Sq::Strike(target, _) = sq {
            // Combat 2.1, 2.6: an attack; a melee weapon owned but not
            // held is readied first, a move if the weapon in hand changes.
            let me = Who::Creature { region, slot };
            if self.held_melee(me).is_none() {
                if let Some(t) = self.owned_melee(me) {
                    if self.set_weapon(me, t) {
                        ev.push(Event::CreatureReadied {
                            region,
                            slot,
                            thing: t,
                        });
                        *left = left.saturating_sub(1);
                    }
                }
            }
            self.attack(me, target, *left, full, ev);
            // 2.5: attacking ends the turn.
            *left = 0;
            return false;
        }
        stood.push((nx, ny));
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
                // 7.5: it takes the thing from where it stands, and its
                // turn ends.
                self.creature_pick_up(region, slot, i, ev);
                false
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
    /// The creature stays where it is.
    fn creature_pick_up(&mut self, region: u8, slot: usize, index: usize, ev: &mut Vec<Event>) {
        let Some(room) = self.creature(region, slot).map(|c| c.room) else {
            return;
        };
        let Some(item) = self.room(region, room).and_then(|r| r.items.get(index)).cloned() else {
            return;
        };
        if item.thing == GOLD {
            let pile = Game::pile_size(&item);
            let wealth = self.creature(region, slot).map_or(0, |c| c.wealth);
            let amount = pile.min(GOLD_LIMIT - wealth);
            if let Some(c) = self.creature_mut(region, slot) {
                c.wealth += amount;
            }
            if amount < pile {
                self.set_pile(region, room, index, pile - amount);
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
        // Creatures 5.4: it lands on the destination, sharing it with
        // whoever stands there (capture.md 10.10.5).
        let (lx, ly) = (x, y);
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
        // Magic 6.6: a door's spell goes off for the creature passing.
        if let Some(t) = self.thing(item.thing).cloned() {
            if let ThingKind::Portal {
                access: Access::InvokeSpell { action },
                ..
            } = t.kind
            {
                let me = Who::Creature { region, slot };
                self.run_spell(
                    Spell {
                        by: me,
                        target: me,
                        source: SpellSource::Portal,
                        thing: item.thing,
                        action,
                        message: self.message(&t.spell_message),
                        at: door,
                        stack: Some(index),
                    },
                    ev,
                );
                if self.creature(region, slot).is_none() {
                    return;
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

    /// 8.1: one drop a turn while load is above capacity: all its gold,
    /// else its lowest-numbered treasure or magic item. True if it dropped.
    fn overburdened_drop(&mut self, region: u8, slot: usize, ev: &mut Vec<Event>) -> bool {
        let Some(c) = self.creature(region, slot).cloned() else {
            return false;
        };
        let (load, cap) = (creatures::load(&self.adv, &c), creatures::capacity(&c));
        if load <= cap {
            return false;
        }
        let mut owned: Vec<ThingId> = c
            .possessions
            .iter()
            .filter(|p| p.count > 0)
            .map(|p| p.thing)
            .filter(|&t| {
                t != GOLD
                    && matches!(
                        self.thing(t).map(|t| &t.kind),
                        Some(ThingKind::MagicItem { .. } | ThingKind::Treasure { .. })
                    )
            })
            .collect();
        owned.sort_unstable();
        let (thing, amount) = if c.wealth > 0 {
            (GOLD, c.wealth)
        } else if let Some(&t) = owned.first() {
            (t, 1)
        } else {
            return false;
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
            if thing == GOLD {
                self.put_gold(region, c.room, c.x, c.y, amount);
            } else {
                self.put_on_top(region, c.room, c.x, c.y, thing, 1);
            }
        }
        ev.push(Event::CreatureDropped {
            region,
            slot,
            thing,
            amount,
            at: (!vanishes).then_some(at),
        });
        self.creature_drop_spell(region, slot, thing, at, ev);
        true
    }

    /// Combat 6.2 (creatures 8.2): with an aggressive foe or a player foe
    /// on one of the 4 squares beside it, an owned melee weapon it doesn't
    /// hold; with none beside it, an owned missile weapon; the
    /// lowest-numbered. True if the weapon in hand changed (a move).
    fn creature_ready(&mut self, region: u8, slot: usize, ev: &mut Vec<Event>) -> bool {
        let Some(c) = self.creature(region, slot).cloned() else {
            return false;
        };
        let me = Who::Creature { region, slot };
        let threatened = self.others_in_room(region, c.room, Some(me)).iter().any(|o| {
            steps((o.x, o.y), (c.x, c.y)) == 1 && (o.alignment.is_none() || o.aggressive) && is_foe(&c, o)
        });
        let mut weapons: Vec<ThingId> = c
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
        let holds_melee = matches!(
            self.thing(c.weapon).map(|t| &t.kind),
            Some(ThingKind::MeleeWeapon { .. })
        );
        let pick = match (threatened, holds_melee) {
            (true, true) => None,
            (true, false) => find(false),
            (false, _) => find(true),
        };
        match pick {
            Some(t) if self.set_weapon(me, t) => {
                ev.push(Event::CreatureReadied {
                    region,
                    slot,
                    thing: t,
                });
                true
            }
            _ => false,
        }
    }

    /// 7.3: the lowest-numbered magic item, next to the player if possible.
    /// True if it dropped one (which ends its turn).
    fn friend_drop(&mut self, region: u8, slot: usize, player: Other, ev: &mut Vec<Event>) -> bool {
        let Some(c) = self.creature(region, slot).cloned() else {
            return false;
        };
        let mut items: Vec<ThingId> = c
            .possessions
            .iter()
            .filter(|p| p.count > 0)
            .map(|p| p.thing)
            .filter(|&t| t != GOLD)
            .collect();
        items.sort_unstable();
        // 7.3: its lowest-numbered magic item, else treasure.
        let of = |magic: bool| {
            items
                .iter()
                .copied()
                .find(|&t| match self.thing(t).map(|t| &t.kind) {
                    Some(ThingKind::MagicItem { .. }) => magic,
                    Some(ThingKind::Treasure { .. }) => !magic,
                    _ => false,
                })
        };
        let Some(thing) = of(true).or_else(|| of(false)) else {
            return false;
        };
        let Some(r) = self.room(region, c.room) else {
            return false;
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
        let at = Location::Room {
            region,
            room: c.room,
            x: spot.0,
            y: spot.1,
        };
        ev.push(Event::CreatureDropped {
            region,
            slot,
            thing,
            amount: 1,
            at: Some(at),
        });
        self.creature_drop_spell(region, slot, thing, at, ev);
        true
    }
}
