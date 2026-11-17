// SPDX-License-Identifier: GPL-3.0-or-later

//! The rules engine: plays a decoded [`acs_core::Adventure`].
//!
//! No GUI, no I/O, no clocks. A [`Game`] is built from an adventure, a seed
//! and [`Rules`]; front ends send it [`Command`]s and get back [`Event`]s
//! that say what happened. All randomness comes from one seeded [`Rng`], so a
//! game replays exactly from its seed, its first player and its command log
//! ([`Game::replay`]).
//!
//! Each rule is specified in `docs/rules/` before it is implemented here;
//! the spec tags every rule as coming from the manual, from the data, or as
//! still unknown, and says what the engine does for the unknown ones.
//! Implemented so far: movement (`docs/rules/movement.md`), inventory
//! (`docs/rules/inventory.md`), creatures (`docs/rules/creatures.md`), magic
//! (`docs/rules/magic.md`), combat (`docs/rules/combat.md`), stores
//! (`docs/rules/stores.md`), characters (`docs/rules/characters.md`) and
//! world map encounters (`docs/rules/encounters.md`).
//! Saves are replays of the command log (`docs/saves.md`, [`SaveFile`]).
//! A front end asks [`view`]'s queries for what the rules would answer
//! (prices, chances, marks) instead of working it out (`docs/client.md`).

mod ai;
pub mod characters;
pub mod combat;
mod command;
pub mod creatures;
mod describe;
pub mod encounters;
mod event;
mod game;
pub mod inventory;
pub mod magic;
pub mod rng;
pub mod save;
mod step;
pub mod stores;
pub mod view;

// acs-core's fixture builder for unit tests. It sits here, not inside a test
// module, so its `#[path]` goes through `src/` only: a path through a
// directory that doesn't exist (`src/stores/tests/..`) fails on Linux.
#[cfg(test)]
#[path = "../../acs-core/tests/common/mod.rs"]
mod test_fixture;

pub use combat::BlowOutcome;
pub use command::{Command, StoreRefusal};
pub use describe::describe;
pub use event::{
    BlockReason, CastRefusal, DropRefusal, Event, FireRefusal, ReadyRefusal, RemovalReason, Slot,
    SpellSource, Who,
};
pub use game::{Game, Newcomer, Player, PlayerSetup, MAX_PLAYERS, NAME_MAX};
pub use rng::Rng;
pub use save::{Restored, SaveError, SaveFile};
pub use view::{BuyOffer, SellOffer, StoreOffers};

use serde::{Deserialize, Serialize};

/// Where a character stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "on", rename_all = "snake_case")]
pub enum Location {
    /// A world map cell, 0..40 each way.
    World { x: u8, y: u8 },
    /// A square in a room: `region` 1-based, `room` 0-based slot, `x`/`y`
    /// counted from the room's top-left wall square.
    Room { region: u8, room: u8, x: u8, y: u8 },
}

impl Location {
    /// The region number, 0 on the world map.
    pub fn region(&self) -> u8 {
        match *self {
            Location::World { .. } => 0,
            Location::Room { region, .. } => region,
        }
    }

    /// `(region, room)` when in a room.
    pub fn room(&self) -> Option<(u8, u8)> {
        match *self {
            Location::World { .. } => None,
            Location::Room { region, room, .. } => Some((region, room)),
        }
    }

    pub fn xy(&self) -> (u8, u8) {
        match *self {
            Location::World { x, y } | Location::Room { x, y, .. } => (x, y),
        }
    }
}

/// One of the eight joystick directions. `y` grows downwards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Dir {
    N,
    NE,
    E,
    SE,
    S,
    SW,
    W,
    NW,
}

impl Dir {
    pub const ALL: [Dir; 8] = [Dir::N, Dir::NE, Dir::E, Dir::SE, Dir::S, Dir::SW, Dir::W, Dir::NW];

    pub fn delta(self) -> (i8, i8) {
        match self {
            Dir::N => (0, -1),
            Dir::NE => (1, -1),
            Dir::E => (1, 0),
            Dir::SE => (1, 1),
            Dir::S => (0, 1),
            Dir::SW => (-1, 1),
            Dir::W => (-1, 0),
            Dir::NW => (-1, -1),
        }
    }

    pub fn is_diagonal(self) -> bool {
        let (dx, dy) = self.delta();
        dx != 0 && dy != 0
    }

    pub fn from_delta(dx: i8, dy: i8) -> Option<Dir> {
        Dir::ALL.into_iter().find(|d| d.delta() == (dx, dy))
    }
}

/// Choices the original leaves open or that are still unmeasured
/// (see the "Engine:" notes in `docs/rules/movement.md`).
/// The defaults follow the original as measured.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rules {
    /// Allow diagonal steps. Off by default: the C64 original ignores a
    /// diagonal push on the world map and in rooms (movement 3.2, captured).
    /// Creatures follow the same rule; whether the original's creatures step
    /// diagonally is still unmeasured.
    pub diagonals: bool,
}

/// A trait, skill or other rating a spell can change (magic 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stat {
    Constitution,
    Strength,
    Dexterity,
    Speed,
    Wisdom,
    Dodge,
    Parry,
    ArmorSkill,
    Melee,
    Missile,
    Power,
    LifeForce,
    /// Magic defense as its level 0..=3.
    MagicDefense,
}

impl Stat {
    /// The Increase / Decrease spell parameter 0..=9 (magic 3.5).
    pub fn from_spell_parameter(p: u16) -> Option<Stat> {
        use Stat::*;
        Some(match p {
            0 => Constitution,
            1 => Strength,
            2 => Dexterity,
            3 => Speed,
            4 => Wisdom,
            5 => Dodge,
            6 => Parry,
            7 => ArmorSkill,
            8 => Melee,
            9 => Missile,
            _ => return None,
        })
    }

    /// Skills move by 5 per spell, traits by 1 (magic 3.5).
    pub fn is_skill(self) -> bool {
        matches!(
            self,
            Stat::Dodge | Stat::Parry | Stat::ArmorSkill | Stat::Melee | Stat::Missile
        )
    }

    /// The highest value the manual allows.
    pub fn max(self) -> u8 {
        match self {
            Stat::Speed => 15,
            Stat::LifeForce => 63,
            Stat::Power => 127,
            Stat::MagicDefense => 3,
            s if s.is_skill() => 127,
            _ => 31,
        }
    }
}
