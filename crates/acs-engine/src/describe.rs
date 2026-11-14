// SPDX-License-Identifier: GPL-3.0-or-later

//! Plain-text lines for events, for message logs and traces.

use crate::game::Game;
use crate::magic::amount;
use crate::{
    BlockReason, BlowOutcome, CastRefusal, DropRefusal, Event, Location, ReadyRefusal, RemovalReason, Slot,
    SpellSource, Stat, Who,
};
use acs_core::model::{Alignment, SpellAction, SpellActionKind, ThingId};

/// One line describing `ev`, using the names in the game's adventure.
pub fn describe(game: &Game, ev: &Event) -> String {
    let who = |p: &usize| {
        game.players()
            .get(*p)
            .map(|pl| pl.name.clone())
            .unwrap_or_else(|| format!("player {}", p + 1))
    };
    let thing = |t: ThingId| {
        game.thing(t)
            .map(|t| t.name.trim().to_string())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| format!("thing {t}"))
    };
    let name = |w: &Who| individual(game, *w);
    let with_message = |line: String, m: &Option<String>| match m {
        Some(m) => format!("{line}: \"{m}\""),
        None => line,
    };
    match ev {
        Event::TurnStarted {
            player,
            moves,
            hindered,
        } => {
            let h = if *hindered {
                ", hindered by a creature next to them"
            } else {
                ""
            };
            format!("{}'s turn ({moves} moves{h}).", who(player))
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
                BlockReason::Store { thing: t } => format!("{}: the store opens", thing(t)),
                BlockReason::Diagonal => "diagonal moves are off".into(),
                BlockReason::NoMovesLeft => "no moves left".into(),
                BlockReason::CarryLimit { thing: t } => {
                    format!("can't carry any more {}", thing(t))
                }
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
            format!("{} bumps into {name} but has no melee weapon.", who(player))
        }
        Event::BumpedPlayer { player, other } => {
            format!(
                "{} bumps into {} but has no melee weapon.",
                who(player),
                who(other)
            )
        }
        Event::PickedUp { player, thing: t } => format!("{} picks up {}.", who(player), thing(*t)),
        Event::GoldPickedUp { player, amount } => {
            format!(
                "{} picks up {amount} {}.",
                who(player),
                thing(crate::inventory::GOLD)
            )
        }
        Event::Readied {
            player,
            thing: t,
            slot,
        } => match slot {
            Slot::Weapon => format!("{} readies {}.", who(player), thing(*t)),
            Slot::Armor => format!(
                "{} puts on {} (that takes the rest of the turn).",
                who(player),
                thing(*t)
            ),
        },
        Event::ReadyRefused {
            player,
            thing: t,
            reason,
        } => {
            let why = match reason {
                ReadyRefusal::NotCarried => "not carried",
                ReadyRefusal::NotWeaponOrArmor => "not a weapon or armor",
            };
            format!("{} can't ready {}: {why}.", who(player), thing(*t))
        }
        Event::Dropped { player, thing: t, at } => {
            format!("{} drops {} at {}.", who(player), thing(*t), place(game, at))
        }
        Event::GoldDropped { player, amount, at } => match at {
            Some(at) => format!(
                "{} drops {amount} {} at {}.",
                who(player),
                thing(crate::inventory::GOLD),
                place(game, at)
            ),
            None => format!(
                "{} drops {amount} {}; it is lost on the map.",
                who(player),
                thing(crate::inventory::GOLD)
            ),
        },
        Event::LostOnMap { player, thing: t } => {
            format!("{} drops {}; it is lost on the map.", who(player), thing(*t))
        }
        Event::VanishedWhenDropped { player, thing: t } => {
            format!("{} drops {} and it vanishes.", who(player), thing(*t))
        }
        Event::DropRefused {
            player,
            thing: t,
            reason,
        } => {
            let why = match *reason {
                DropRefusal::NotCarried => "not carried".to_string(),
                DropRefusal::MagicSpell => "spells can't be dropped".to_string(),
                DropRefusal::OwnerOnly => "only its owner can use it".to_string(),
                DropRefusal::Wall => "there is a wall there".to_string(),
                DropRefusal::Thing { thing: o } => format!("{} is in the way", thing(o)),
                DropRefusal::BadAmount => "one pile holds 1 to 255".to_string(),
            };
            format!("{} can't drop {}: {why}.", who(player), thing(*t))
        }
        Event::ThingUsedUp { player, thing: t } => {
            format!("{}'s {} is used up.", who(player), thing(*t))
        }
        Event::ObstacleVanished { thing: t, .. } => format!("{} vanishes.", thing(*t)),
        Event::SpellTriggered {
            by,
            target,
            source,
            thing: t,
            action,
            message,
        } => {
            let src = match source {
                SpellSource::Terrain => "terrain",
                SpellSource::Portal => "door",
                SpellSource::Space => "space",
                SpellSource::CustomSpace => "space",
                SpellSource::Obstacle => "obstacle",
                SpellSource::CustomObstacle => "obstacle",
                SpellSource::PickedUp => "picked-up",
                SpellSource::Dropped => "dropped",
                SpellSource::Cast => "spell",
                SpellSource::Used => "item",
                SpellSource::Stack => "stacked",
            };
            let on = match target {
                Some(w) if w != by => format!(" on {}", name(w)),
                _ => String::new(),
            };
            let line = format!(
                "{} sets off {src} {}: {}{on}",
                name(by),
                thing(*t),
                action_text(game, *action)
            );
            match message {
                Some(_) => with_message(line, message),
                None => format!("{line}."),
            }
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
        Event::RoomEncounter {
            region, creature, at, ..
        } => {
            let name = game
                .region(*region)
                .and_then(|r| r.random_creatures.get(*creature as usize))
                .map(|c| c.name.trim().to_string())
                .unwrap_or_default();
            match at {
                Some(at) => format!("{name} appears at {}.", place(game, at)),
                None => format!("{name} would appear, but no square is free."),
            }
        }
        Event::CreatureTurn { region, slot, moves } => {
            format!("{}'s turn ({moves} moves).", creature(game, *region, *slot))
        }
        Event::CreatureMoved { region, slot, to, .. } => {
            format!("{} moves to {}.", creature(game, *region, *slot), place(game, to))
        }
        Event::CreatureTookDoor { region, slot, to, .. } => {
            format!(
                "{} goes through to {}.",
                creature(game, *region, *slot),
                place(game, to)
            )
        }
        Event::CreaturePickedUp {
            region,
            slot,
            thing: t,
        } => {
            format!("{} picks up {}.", creature(game, *region, *slot), thing(*t))
        }
        Event::CreatureGoldPickedUp { region, slot, amount } => format!(
            "{} picks up {amount} {}.",
            creature(game, *region, *slot),
            thing(crate::inventory::GOLD)
        ),
        Event::CreatureDropped {
            region,
            slot,
            thing: t,
            amount,
            at,
        } => {
            let what = if *t == crate::inventory::GOLD {
                format!("{amount} {}", thing(*t))
            } else {
                thing(*t)
            };
            match at {
                Some(at) => format!(
                    "{} drops {what} at {}.",
                    creature(game, *region, *slot),
                    place(game, at)
                ),
                None => format!("{} drops {what} and it vanishes.", creature(game, *region, *slot)),
            }
        }
        Event::CreatureReadied {
            region,
            slot,
            thing: t,
        } => {
            format!("{} readies {}.", creature(game, *region, *slot), thing(*t))
        }
        Event::Attacked {
            by,
            target,
            weapon,
            blows,
        } => {
            let n = if *blows == 1 { "blow" } else { "blows" };
            format!(
                "{} attacks {} with {} ({blows} {n}).",
                name(by),
                name(target),
                thing(*weapon)
            )
        }
        Event::Blow {
            by,
            target,
            counter,
            chance,
            outcome,
        } => {
            let back = if *counter { " strikes back and" } else { "" };
            let what = match outcome {
                BlowOutcome::Miss => format!("misses {}", name(target)),
                BlowOutcome::Parried => format!("is parried by {}", name(target)),
                BlowOutcome::Hit { damage: 0, .. } => format!("hits {} but does no harm", name(target)),
                BlowOutcome::Hit { damage, heavy } => {
                    let h = if *heavy { " heavily" } else { "" };
                    format!("hits {}{h} for {damage}", name(target))
                }
                BlowOutcome::Stopped => format!("can't harm {}", name(target)),
            };
            format!("{}{back} {what} ({chance}%).", name(by))
        }
        Event::Fired {
            by,
            target,
            weapon,
            shots,
        } => {
            let n = if *shots == 1 { "shot" } else { "shots" };
            format!(
                "{} fires {} at {} ({shots} {n}).",
                name(by),
                thing(*weapon),
                name(target)
            )
        }
        Event::Shot {
            by,
            spread,
            deviation,
            side,
        } => {
            let way = if *side { "right" } else { "left" };
            format!(
                "{}'s shot goes {deviation} of {spread} off to the {way}.",
                name(by)
            )
        }
        Event::Dodged { by, who: w, chance } => {
            format!("{} dodges {}'s missile ({chance}%).", name(w), name(by))
        }
        Event::MissileHit {
            by,
            target,
            dodge,
            outcome,
        } => {
            let what = match outcome {
                BlowOutcome::Hit { damage: 0, .. } => format!("hits {} but does no harm", name(target)),
                BlowOutcome::Hit { damage, heavy } => {
                    let h = if *heavy { " heavily" } else { "" };
                    format!("hits {}{h} for {damage}", name(target))
                }
                _ => format!("can't harm {}", name(target)),
            };
            format!("{}'s missile {what} (dodge {dodge}%).", name(by))
        }
        Event::MissileStopped { by, x, y } => {
            format!("{}'s missile stops at ({x},{y}).", name(by))
        }
        Event::CannotFire { player } => {
            format!(
                "{} can't fire: only creatures shoot missiles.",
                name(&Who::Player { player: *player })
            )
        }
        Event::StoreOpened { player, gold, .. } => {
            format!("{} enters the store with {gold} gold to spend.", who(player))
        }
        Event::StoreClosed { player } => format!("{} leaves the store.", who(player)),
        Event::Bought {
            player,
            thing: t,
            price,
        } => {
            format!(
                "The storekeeper sells {} to {} for {price} gold.",
                thing(*t),
                who(player)
            )
        }
        Event::Sold {
            player,
            thing: t,
            offer,
            paid,
            ..
        } => {
            let capped = if paid < offer {
                format!(" ({paid} fit in the purse)")
            } else {
                String::new()
            };
            format!("{} sells {} for {offer} gold{capped}.", who(player), thing(*t))
        }
        Event::StoreRefused {
            player,
            thing: t,
            reason,
        } => {
            use crate::StoreRefusal as R;
            let why = match reason {
                R::NoStore => "no store is open".to_string(),
                R::NotStocked => format!("{} is out of stock", thing(*t)),
                R::NotEnoughGold => format!("not enough gold for {}", thing(*t)),
                R::CarryLimit => format!("can't carry another {}", thing(*t)),
                R::NotSellable => format!("the store won't buy {}", thing(*t)),
                R::NotOwned => format!("has no {} to sell", thing(*t)),
            };
            format!("{}: {why}.", who(player))
        }
        Event::Broke {
            who: w,
            thing: t,
            used_up,
        } => {
            let how = if *used_up { "is used up" } else { "breaks" };
            format!("{}'s {} {how}.", name(w), thing(*t))
        }
        Event::CreatureTurned {
            region,
            slot,
            alignment,
            aggressive,
        } => {
            let setting = if *aggressive { "aggressive" } else { "peaceful" };
            let side = match alignment {
                Alignment::Friend => "friend",
                Alignment::Enemy => "enemy",
                Alignment::Neutral => "neutral",
                Alignment::Thief => "thief",
            };
            format!("{} is now a {setting} {side}.", creature(game, *region, *slot))
        }
        Event::CreatureUsedUp {
            region,
            slot,
            thing: t,
        } => {
            format!("{}'s {} is used up.", creature(game, *region, *slot), thing(*t))
        }
        Event::CreatureRemoved { region, slot, reason } => {
            let why = match reason {
                RemovalReason::LeftBehind => "wanders off",
                RemovalReason::Banished => "is banished",
            };
            format!("{} {why}.", creature(game, *region, *slot))
        }
        Event::SpellCast {
            by,
            thing: t,
            target,
            chance,
            cost,
            success,
        } => {
            let on = if target == by {
                String::new()
            } else {
                format!(" at {}", name(target))
            };
            let outcome = if *success { "it works" } else { "it fails" };
            format!(
                "{} casts {}{on} ({chance}% chance, {cost} Power): {outcome}.",
                name(by),
                thing(*t)
            )
        }
        Event::ItemUsed { by, thing: t, target } => {
            let on = if target == by {
                String::new()
            } else {
                format!(" on {}", name(target))
            };
            format!("{} uses {}{on}.", name(by), thing(*t))
        }
        Event::CastRefused {
            player,
            thing: t,
            reason,
        } => {
            let why = match reason {
                CastRefusal::NotCarried => "not carried",
                CastRefusal::NotASpell => "not a magic spell",
                CastRefusal::NotUsable => "it does nothing when used",
                CastRefusal::BadTarget => "that target is not here",
            };
            format!("{} can't use {}: {why}.", who(player), thing(*t))
        }
        Event::SpellGone { who: w, thing: t } => format!("{}'s {} is gone.", name(w), thing(*t)),
        Event::StatChanged {
            who: w,
            stat,
            from,
            to,
        } => {
            format!("{}'s {} goes from {from} to {to}.", name(w), stat_name(*stat))
        }
        Event::Given {
            who: w,
            thing: t,
            kept,
        } => {
            if *kept {
                format!("{} receives {}.", name(w), thing(*t))
            } else {
                format!("{} can't carry another {}.", name(w), thing(*t))
            }
        }
        Event::LongMessage { text, .. } => format!("\"{text}\""),
        Event::Music { number } => format!("Music {number} plays."),
        Event::RoomRidOf {
            region,
            room,
            thing: t,
            count,
        } => format!(
            "{count} {} vanish from {}.",
            thing(*t),
            room_name(game, *region, *room)
        ),
        Event::ThingAdded { thing: t, at } => format!("{} appears at {}.", thing(*t), place(game, at)),
        Event::CreatureSummoned { region, slot, at, .. } => format!(
            "{} answers the summons at {}.",
            creature(game, *region, *slot),
            place(game, at)
        ),
        Event::SummonUnanswered { creature: c } => {
            format!("No one answers the summons for {}.", master_creature(game, *c))
        }
        Event::Died { who: w, left, .. } => {
            if left.is_empty() {
                format!("{} dies.", name(w))
            } else {
                let things: Vec<String> = left
                    .iter()
                    .map(|&(t, n)| {
                        if t == crate::inventory::GOLD {
                            format!("{n} {}", thing(t))
                        } else {
                            thing(t)
                        }
                    })
                    .collect();
                format!("{} dies, leaving {}.", name(w), things.join(", "))
            }
        }
        Event::AllPlayersDead => "Everyone is dead. Resume from the last save.".into(),
    }
}

/// A player's or creature's name.
fn individual(game: &Game, who: Who) -> String {
    match who {
        Who::Player { player } => game
            .players()
            .get(player)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| format!("player {}", player + 1)),
        Who::Creature { region, slot } => creature(game, region, slot),
    }
}

fn master_creature(game: &Game, number: u16) -> String {
    game.adventure()
        .creatures
        .get((number as usize).wrapping_sub(1))
        .map(|c| c.name.trim().to_string())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| format!("creature {number}"))
}

fn stat_name(s: Stat) -> &'static str {
    match s {
        Stat::Constitution => "Constitution",
        Stat::Strength => "Strength",
        Stat::Dexterity => "Dexterity",
        Stat::Speed => "Speed",
        Stat::Wisdom => "Wisdom",
        Stat::Dodge => "Dodge",
        Stat::Parry => "Parry",
        Stat::ArmorSkill => "Armor skill",
        Stat::Melee => "Melee",
        Stat::Missile => "Missile",
        Stat::Power => "Power",
        Stat::LifeForce => "Life Force",
        Stat::MagicDefense => "magic defense",
    }
}

/// What a spell action does, in a few words.
fn action_text(game: &Game, a: SpellAction) -> String {
    use SpellActionKind::*;
    let thing = |t: ThingId| {
        game.thing(t)
            .map(|t| t.name.trim().to_string())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| format!("thing {t}"))
    };
    let p = a.parameter;
    let signed = |p: u16| {
        let (n, temporary) = amount(p);
        let t = if temporary { " for a while" } else { "" };
        format!("{n:+}{t}")
    };
    let stat = |p: u16| Stat::from_spell_parameter(p).map_or("nothing", stat_name);
    match a.kind {
        Nothing => "nothing happens".into(),
        KillAllButOwnerOf => format!("kill all but owners of {}", thing(p)),
        SummonOrBanish if p == 0 => "banish every creature".into(),
        SummonOrBanish => format!("summon {}", master_creature(game, p)),
        IncreaseMagicDefense => "raise magic defense".into(),
        DecreaseMagicDefense => "lower magic defense".into(),
        IncreaseStat => format!("raise {}", stat(p)),
        DecreaseStat => format!("lower {}", stat(p)),
        ChangePower => format!("Power {}", signed(p)),
        ChangeLifeForce => format!("Life Force {}", signed(p)),
        Give => format!("give {}", thing(p)),
        LongMessage => format!("message {p}"),
        PlayMusic => format!("music {}", p + 1),
        RidRoomOf => format!("rid the room of {}", thing(p)),
        AddToRoom => format!("add {}", thing(p)),
        ActivateAll => "activate everything here".into(),
        Invalid(c) => format!("unknown action {c}"),
    }
}

/// A region slot's creature name (removed ones keep theirs in the record).
fn creature(game: &Game, region: u8, slot: usize) -> String {
    game.region(region)
        .and_then(|r| r.residents.get(slot))
        .map(|c| c.name.trim().to_string())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| format!("creature {}", slot + 1))
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
