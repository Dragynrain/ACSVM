// SPDX-License-Identifier: GPL-3.0-or-later

//! Plain-text lines for events, for message logs and traces.

use crate::game::Game;
use crate::{BlockReason, Event, Location, SpellSource};

/// One line describing `ev`, using the names in the game's adventure.
pub fn describe(game: &Game, ev: &Event) -> String {
    let who = |p: &usize| {
        game.players()
            .get(*p)
            .map(|pl| pl.name.clone())
            .unwrap_or_else(|| format!("player {}", p + 1))
    };
    let thing = |t: u8| {
        game.thing(t)
            .map(|t| t.name.trim().to_string())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| format!("thing {t}"))
    };
    let with_message = |line: String, m: &Option<String>| match m {
        Some(m) => format!("{line}: \"{m}\""),
        None => line,
    };
    match ev {
        Event::TurnStarted { player, moves } => {
            format!("{}'s turn ({moves} moves).", who(player))
        }
        Event::TurnEnded { player } => format!("{}'s turn ends.", who(player)),
        Event::PlayerAdded { player, at } => {
            format!("{} joins at {}.", who(player), place(game, at))
        }
        Event::Moved { player, to, .. } => format!("{} moves to {}.", who(player), place(game, to)),
        Event::Blocked {
            player,
            reason,
            message,
            ..
        } => {
            let why = match *reason {
                BlockReason::MapEdge => "the edge of the map".to_string(),
                BlockReason::Terrain { terrain } => game
                    .adventure()
                    .world
                    .terrain
                    .get(terrain as usize)
                    .map(|t| t.name.clone())
                    .filter(|n| !n.is_empty())
                    .unwrap_or_else(|| format!("terrain {}", terrain + 1)),
                BlockReason::NeedsThing { thing: t } => format!("only owners of {} may pass", thing(t)),
                BlockReason::ClosedToOwnersOf { thing: t } => {
                    format!("closed to owners of {}", thing(t))
                }
                BlockReason::Wall => "a wall".into(),
                BlockReason::Obstacle { thing: t } => thing(t),
                BlockReason::Store { thing: t } => format!("{} (stores come later)", thing(t)),
                BlockReason::Diagonal => "diagonal moves are off".into(),
                BlockReason::NoMovesLeft => "no moves left".into(),
            };
            with_message(format!("{} is blocked: {why}", who(player)), message)
        }
        Event::BumpedCreature {
            player,
            region,
            resident,
            ..
        } => {
            let name = game
                .region(*region)
                .and_then(|r| r.residents.get(*resident))
                .map(|c| c.name.trim().to_string())
                .unwrap_or_else(|| "someone".into());
            format!("{} bumps into {name} (combat comes later).", who(player))
        }
        Event::PickedUp { player, thing: t } => format!("{} picks up {}.", who(player), thing(*t)),
        Event::ThingUsedUp { player, thing: t } => {
            format!("{}'s {} is used up.", who(player), thing(*t))
        }
        Event::ObstacleVanished { thing: t, .. } => format!("{} vanishes.", thing(*t)),
        Event::SpellTriggered {
            source,
            thing: t,
            action,
            message,
            ..
        } => {
            let src = match source {
                SpellSource::Terrain => "terrain",
                SpellSource::Portal => "door",
                SpellSource::Space => "space",
                SpellSource::CustomSpace => "space",
                SpellSource::Obstacle => "obstacle",
                SpellSource::CustomObstacle => "obstacle",
                SpellSource::PickedUp => "pick-up",
            };
            with_message(
                format!(
                    "Spell from {src} {}: {:?} {}",
                    thing(*t),
                    action.kind,
                    action.parameter
                ),
                message,
            )
        }
        Event::TookDoor { player, to, .. } => {
            format!("{} goes through to {}.", who(player), place(game, to))
        }
        Event::EnteredRoom { region, room, .. } => {
            format!("Entered {}.", room_name(game, *region, *room))
        }
        Event::EnteredRegion { region, .. } => {
            format!("Entered region {} (autosave point).", region_name(game, *region))
        }
        Event::LeftRegion { region, .. } => {
            format!("Left region {} (autosave point).", region_name(game, *region))
        }
        Event::EnteredWorldMap { .. } => {
            format!("Out on {}.", map_name(game))
        }
        Event::SentToStart { player } => {
            format!("The door leads nowhere: {} is back at the start.", who(player))
        }
        Event::Encounter {
            creature, terrain, ..
        } => {
            let w = &game.adventure().world;
            let name = w
                .random_creatures
                .get(*creature)
                .map(|c| c.creature.name.trim().to_string())
                .unwrap_or_default();
            let tname = w
                .terrain
                .get(*terrain as usize)
                .map(|t| t.name.clone())
                .unwrap_or_default();
            format!("Encounter: {name} appears in the {tname} (encounters come later).")
        }
        Event::RoomEncounter { region, creature, .. } => {
            let name = game
                .region(*region)
                .and_then(|r| r.random_creatures.get(*creature as usize))
                .map(|c| c.name.trim().to_string())
                .unwrap_or_default();
            format!("{name} would appear here (random creatures come later).")
        }
    }
}

fn map_name(game: &Game) -> String {
    let n = game.adventure().world.name.trim();
    if n.is_empty() {
        "the world map".into()
    } else {
        n.to_string()
    }
}

fn region_name(game: &Game, region: u8) -> String {
    game.region(region)
        .map(|r| r.name.trim().to_string())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| format!("{region}"))
}

fn room_name(game: &Game, region: u8, room: u8) -> String {
    game.region(region)
        .and_then(|r| r.rooms.get(room as usize))
        .map(|r| r.name.trim().to_string())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| format!("room {room}"))
}

fn place(game: &Game, at: &Location) -> String {
    match *at {
        Location::World { x, y } => format!("{} {x},{y}", map_name(game)),
        Location::Room { region, room, x, y } => {
            format!("{} {x},{y}", room_name(game, region, room))
        }
    }
}
