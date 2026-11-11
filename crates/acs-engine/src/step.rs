// SPDX-License-Identifier: GPL-3.0-or-later

//! Movement: one step on the world map or in a room, doors, terrain and the
//! things a step can run into. Spec: `docs/rules/movement.md` (section
//! numbers in the comments refer to it).

use crate::game::Game;
use crate::{BlockReason, Dir, Event, Location, SpellSource};
use acs_core::model::{
    Access, CustomTrigger, Destination, Passability, PlacementChoice, SpellAction, ThingKind,
};

/// World map width and height (4.1).
pub const MAP_SIZE: u8 = 40;
/// Terrain numbers 12..16 (13-16 in the editor) are doors (4.4).
pub const FIRST_DOOR_TERRAIN: u8 = 12;
/// Most creatures a region holds at once (7.2).
pub const REGION_CREATURE_LIMIT: usize = 16;

/// Outcome of an access check on a door or space.
enum Pass {
    Yes {
        spell: Option<SpellAction>,
        used_up: Option<u8>,
    },
    No(BlockReason),
}

impl Game {
    pub(crate) fn step(&mut self, dir: Dir, ev: &mut Vec<Event>) {
        let p = self.current;
        let from = self.players[p].at;
        if dir.is_diagonal() && !self.rules.diagonals {
            ev.push(Event::Blocked {
                player: p,
                target: from,
                reason: BlockReason::Diagonal,
                message: None,
            });
            return;
        }
        if self.players[p].moves_left == 0 {
            ev.push(Event::Blocked {
                player: p,
                target: from,
                reason: BlockReason::NoMovesLeft,
                message: None,
            });
            return;
        }
        match from {
            Location::World { x, y } => self.step_on_map(p, x, y, dir, ev),
            Location::Room { region, room, x, y } => self.step_in_room(p, region, room, x, y, dir, ev),
        }
    }

    // ---- world map (section 4) -------------------------------------------

    fn tile(&self, x: u8, y: u8) -> u8 {
        self.adv
            .world
            .tiles
            .get(y as usize)
            .and_then(|row| row.get(x as usize))
            .copied()
            .unwrap_or(0)
    }

    fn map_target(&self, x: u8, y: u8, dir: Dir) -> Option<(u8, u8)> {
        let (dx, dy) = dir.delta();
        let (nx, ny) = (x as i16 + dx as i16, y as i16 + dy as i16);
        let n = MAP_SIZE as i16;
        if self.adv.world.wraps {
            Some((nx.rem_euclid(n) as u8, ny.rem_euclid(n) as u8))
        } else if (0..n).contains(&nx) && (0..n).contains(&ny) {
            Some((nx as u8, ny as u8))
        } else {
            None
        }
    }

    fn step_on_map(&mut self, p: usize, x: u8, y: u8, dir: Dir, ev: &mut Vec<Event>) {
        let from = Location::World { x, y };
        // 7.1: one roll before every map move, for the terrain stood on.
        self.roll_map_encounter(p, self.tile(x, y), ev);

        let Some((nx, ny)) = self.map_target(x, y, dir) else {
            ev.push(Event::Blocked {
                player: p,
                target: from,
                reason: BlockReason::MapEdge,
                message: None,
            });
            return;
        };
        let to = Location::World { x: nx, y: ny };
        let t = self.tile(nx, ny);

        if t >= FIRST_DOOR_TERRAIN {
            let door = self
                .adv
                .world
                .doors
                .iter()
                .find(|d| d.at.x == nx && d.at.y == ny)
                .map(|d| d.destination);
            if let Some(dest) = door {
                self.travel(p, from, Some(dest), None, ev);
                self.spend(ev);
                return;
            }
        }

        let Some(terrain) = self.adv.world.terrain.get(t as usize).cloned() else {
            self.move_to(p, from, to, ev);
            return;
        };
        match terrain.passability {
            Passability::Impassable => ev.push(Event::Blocked {
                player: p,
                target: to,
                reason: BlockReason::Terrain { terrain: t },
                message: None,
            }),
            Passability::Open => self.move_to(p, from, to, ev),
            Passability::OwnersOf => {
                if self.players[p].owns(terrain.parameter) {
                    self.move_to(p, from, to, ev);
                } else {
                    ev.push(Event::Blocked {
                        player: p,
                        target: to,
                        reason: BlockReason::NeedsThing {
                            thing: terrain.parameter,
                        },
                        message: None,
                    });
                }
            }
            Passability::TriggersSpell => {
                // 4.3: the spell runs once the character is there.
                self.players[p].at = to;
                ev.push(Event::Moved { player: p, from, to });
                if let Some((action, message)) = self.thing_spell(terrain.parameter) {
                    ev.push(Event::SpellTriggered {
                        player: p,
                        source: SpellSource::Terrain,
                        thing: terrain.parameter,
                        action,
                        message,
                    });
                }
                self.spend(ev);
            }
        }
    }

    /// The spell a carried thing (magic spell or magic item) holds.
    fn thing_spell(&self, thing: u8) -> Option<(SpellAction, Option<String>)> {
        let t = self.thing(thing)?;
        let action = match t.kind {
            ThingKind::MagicSpell { action, .. } | ThingKind::MagicItem { action, .. } => action,
            _ => return None,
        };
        Some((action, self.message(&t.spell_message)))
    }

    fn roll_map_encounter(&mut self, p: usize, terrain: u8, ev: &mut Vec<Event>) {
        let candidates: Vec<(usize, u8)> = self
            .adv
            .world
            .random_creatures
            .iter()
            .enumerate()
            .filter(|(_, c)| c.creature.in_use && c.terrain == terrain && c.chance > 0)
            .map(|(i, c)| (i, c.chance))
            .collect();
        for (i, chance) in candidates {
            if self.rng.percent(chance) {
                ev.push(Event::Encounter {
                    player: p,
                    creature: i,
                    terrain,
                });
                return;
            }
        }
    }

    // ---- rooms (section 5) -----------------------------------------------

    #[allow(clippy::too_many_arguments)]
    fn step_in_room(&mut self, p: usize, region: u8, room: u8, x: u8, y: u8, dir: Dir, ev: &mut Vec<Event>) {
        let from = Location::Room { region, room, x, y };
        let Some(r) = self.room(region, room) else {
            // Standing in a room that is gone: only a tool can do that.
            self.send_to_start(p, from, ev);
            return;
        };
        let (w, h) = (r.width as i16, r.height as i16);
        let (dx, dy) = dir.delta();
        let (nx, ny) = (x as i16 + dx as i16, y as i16 + dy as i16);
        if !(0..w).contains(&nx) || !(0..h).contains(&ny) {
            ev.push(Event::Blocked {
                player: p,
                target: from,
                reason: BlockReason::Wall,
                message: None,
            });
            return;
        }
        let (nx, ny) = (nx as u8, ny as u8);
        let to = Location::Room {
            region,
            room,
            x: nx,
            y: ny,
        };
        let edge = nx == 0 || ny == 0 || nx as i16 == w - 1 || ny as i16 == h - 1;

        // 5.4: residents stand above everything.
        let resident = self.region(region).and_then(|reg| {
            reg.residents
                .iter()
                .position(|c| c.in_use && c.room == room && c.x == nx && c.y == ny)
        });
        if let Some(resident) = resident {
            ev.push(Event::BumpedCreature {
                player: p,
                region,
                room,
                resident,
            });
            return;
        }

        // 5.3: only the top of the stack decides.
        let top = r.items.iter().position(|it| it.x == nx && it.y == ny);
        let Some(index) = top else {
            if edge {
                ev.push(Event::Blocked {
                    player: p,
                    target: to,
                    reason: BlockReason::Wall,
                    message: None,
                });
            } else {
                self.move_to(p, from, to, ev);
            }
            return;
        };
        let item = r.items[index].clone();
        let Some(thing) = self.thing(item.thing).cloned() else {
            self.move_to(p, from, to, ev);
            return;
        };
        let blocked_message = self.message(&thing.blocked_message);
        let spell_message = self.message(&thing.spell_message);
        let block = |reason| Event::Blocked {
            player: p,
            target: to,
            reason,
            message: blocked_message.clone(),
        };
        let spell = |source, action| Event::SpellTriggered {
            player: p,
            source,
            thing: item.thing,
            action,
            message: spell_message.clone(),
        };

        match thing.kind {
            // 5.5 / 5.10: pick it up and step onto the square.
            ThingKind::Treasure { .. }
            | ThingKind::MagicItem { .. }
            | ThingKind::MissileWeapon { .. }
            | ThingKind::MeleeWeapon { .. }
            | ThingKind::Armor { .. }
            | ThingKind::MagicSpell { .. } => {
                self.remove_item(region, room, index);
                self.players[p].possessions.push(item.thing);
                ev.push(Event::PickedUp {
                    player: p,
                    thing: item.thing,
                });
                if let ThingKind::MagicItem {
                    action,
                    on_pickup: true,
                    ..
                } = thing.kind
                {
                    ev.push(spell(SpellSource::PickedUp, action));
                }
                self.move_to(p, from, to, ev);
            }
            // Section 6.
            ThingKind::Portal {
                access,
                destroy_needed_thing,
                ..
            } => match self.check_access(p, &access, destroy_needed_thing) {
                Pass::No(reason) => ev.push(block(reason)),
                Pass::Yes { spell: s, used_up } => {
                    if let Some(a) = s {
                        ev.push(spell(SpellSource::Portal, a));
                    }
                    self.use_up(p, used_up, ev);
                    self.travel(p, from, item.destination, Some(item.thing), ev);
                    self.spend(ev);
                }
            },
            ThingKind::Space { access, .. } => match self.check_access(p, &access, false) {
                Pass::No(reason) => ev.push(block(reason)),
                Pass::Yes { spell: s, .. } => {
                    self.players[p].at = to;
                    ev.push(Event::Moved { player: p, from, to });
                    if let Some(a) = s {
                        ev.push(spell(SpellSource::Space, a));
                    }
                    self.spend(ev);
                }
            },
            // 5.8: the placement parameter is either the needed thing or
            // the spell's parameter.
            ThingKind::CustomSpace {
                placement_choice,
                trigger,
                action,
                destroy_needed_thing,
                ..
            } => {
                let (needed, parameter) = match placement_choice {
                    PlacementChoice::Thing => (item.parameter, action.parameter),
                    PlacementChoice::SpellParameter => (action.parameter, item.parameter),
                };
                let action = SpellAction {
                    kind: action.kind,
                    parameter,
                };
                let owns = self.players[p].owns(needed);
                let (passes, fires, used_up) = match trigger {
                    CustomTrigger::WhenMovedOnto => (true, true, None),
                    CustomTrigger::WhenDroppedHere => (true, false, None),
                    CustomTrigger::OpenToOwnersOf => {
                        (owns, owns, (owns && destroy_needed_thing).then_some(needed))
                    }
                    CustomTrigger::OpenToNonOwnersOf => (!owns, !owns, None),
                };
                if !passes {
                    let reason = if owns {
                        BlockReason::ClosedToOwnersOf { thing: needed }
                    } else {
                        BlockReason::NeedsThing { thing: needed }
                    };
                    ev.push(block(reason));
                    return;
                }
                self.players[p].at = to;
                ev.push(Event::Moved { player: p, from, to });
                if fires {
                    ev.push(spell(SpellSource::CustomSpace, action));
                }
                self.use_up(p, used_up, ev);
                self.spend(ev);
            }
            ThingKind::Obstacle {
                bump_invokes_spell,
                action,
                ..
            } => {
                ev.push(block(BlockReason::Obstacle { thing: item.thing }));
                if bump_invokes_spell {
                    ev.push(spell(SpellSource::Obstacle, action));
                }
                // 5.9: bit 4 of the first property byte = vanishes when bumped.
                if thing.raw[0] & 0x10 != 0 {
                    self.remove_item(region, room, index);
                    ev.push(Event::ObstacleVanished {
                        at: to,
                        thing: item.thing,
                    });
                }
            }
            ThingKind::CustomObstacle { action, .. } => {
                ev.push(block(BlockReason::Obstacle { thing: item.thing }));
                ev.push(spell(
                    SpellSource::CustomObstacle,
                    SpellAction {
                        kind: action.kind,
                        parameter: item.parameter,
                    },
                ));
            }
            ThingKind::Store { .. } => ev.push(block(BlockReason::Store { thing: item.thing })),
            ThingKind::RoomFloor { .. } | ThingKind::Undefined | ThingKind::Unknown { .. } => {
                if edge {
                    ev.push(block(BlockReason::Wall));
                } else {
                    self.move_to(p, from, to, ev);
                }
            }
        }
    }

    fn check_access(&self, p: usize, access: &Access, destroy: bool) -> Pass {
        let player = &self.players[p];
        match *access {
            Access::Anyone => Pass::Yes {
                spell: None,
                used_up: None,
            },
            Access::InvokeSpell { action } => Pass::Yes {
                spell: Some(action),
                used_up: None,
            },
            Access::OwnersOf { thing } => {
                if player.owns(thing) {
                    Pass::Yes {
                        spell: None,
                        used_up: destroy.then_some(thing),
                    }
                } else {
                    Pass::No(BlockReason::NeedsThing { thing })
                }
            }
            Access::NonOwnersOf { thing } => {
                if player.owns(thing) {
                    Pass::No(BlockReason::ClosedToOwnersOf { thing })
                } else {
                    Pass::Yes {
                        spell: None,
                        used_up: None,
                    }
                }
            }
        }
    }

    fn use_up(&mut self, p: usize, thing: Option<u8>, ev: &mut Vec<Event>) {
        let Some(thing) = thing else { return };
        let poss = &mut self.players[p].possessions;
        if let Some(i) = poss.iter().position(|&t| t == thing) {
            poss.remove(i);
            ev.push(Event::ThingUsedUp { player: p, thing });
        }
    }

    fn remove_item(&mut self, region: u8, room: u8, index: usize) {
        if let Some(r) = self
            .region_mut(region)
            .and_then(|reg| reg.rooms.get_mut(room as usize))
        {
            if index < r.items.len() {
                r.items.remove(index);
            }
        }
    }

    // ---- shared ------------------------------------------------------------

    /// An ordinary step onto `to`.
    fn move_to(&mut self, p: usize, from: Location, to: Location, ev: &mut Vec<Event>) {
        self.players[p].at = to;
        ev.push(Event::Moved { player: p, from, to });
        self.spend(ev);
    }

    /// Goes through a door to `dest` (6.2-6.5). Does not spend the step.
    fn travel(
        &mut self,
        p: usize,
        from: Location,
        dest: Option<Destination>,
        thing: Option<u8>,
        ev: &mut Vec<Event>,
    ) {
        let to = match dest {
            Some(Destination::WorldMap { x, y }) if x < MAP_SIZE && y < MAP_SIZE => {
                Some(Location::World { x, y })
            }
            Some(Destination::Room { region, room, x, y }) => self
                .room(region, room)
                .filter(|r| x < r.width && y < r.height)
                .map(|_| Location::Room { region, room, x, y }),
            _ => None,
        };
        let Some(to) = to else {
            self.send_to_start(p, from, ev);
            return;
        };
        self.players[p].at = to;
        ev.push(Event::TookDoor {
            player: p,
            from,
            to,
            thing,
        });
        self.arrive(p, from, to, ev);
    }

    /// 6.4: a door into a deleted room puts players at the start.
    fn send_to_start(&mut self, p: usize, from: Location, ev: &mut Vec<Event>) {
        let s = self.adv.world.start;
        let to = Location::World { x: s.x, y: s.y };
        self.players[p].at = to;
        ev.push(Event::SentToStart { player: p });
        self.arrive(p, from, to, ev);
    }

    /// Region and room changes after a jump, and the room's encounter roll.
    fn arrive(&mut self, p: usize, from: Location, to: Location, ev: &mut Vec<Event>) {
        let (old, new) = (from.region(), to.region());
        if old != new {
            if old != 0 {
                ev.push(Event::LeftRegion {
                    player: p,
                    region: old,
                });
            }
            if new != 0 {
                ev.push(Event::EnteredRegion {
                    player: p,
                    region: new,
                });
            } else {
                ev.push(Event::EnteredWorldMap { player: p });
            }
        }
        if let Some((region, room)) = to.room() {
            if from.room() != Some((region, room)) {
                ev.push(Event::EnteredRoom {
                    player: p,
                    region,
                    room,
                });
                self.roll_room_encounter(p, region, room, ev);
            }
        }
    }

    /// 7.2: one roll on entering a room.
    fn roll_room_encounter(&mut self, p: usize, region: u8, room: u8, ev: &mut Vec<Event>) {
        let Some(reg) = self.region(region) else {
            return;
        };
        let Some(r) = reg.rooms.get(room as usize) else {
            return;
        };
        let slot = r.random_creature;
        let chance = r.random_chance;
        let usable = reg.random_creatures.get(slot as usize).is_some_and(|c| c.in_use);
        let present = reg.residents.iter().filter(|c| c.in_use).count();
        if chance == 0 || !usable || present >= REGION_CREATURE_LIMIT {
            return;
        }
        if self.rng.percent(chance) {
            ev.push(Event::RoomEncounter {
                player: p,
                region,
                room,
                creature: slot,
            });
        }
    }
}
