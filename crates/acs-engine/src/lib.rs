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
//! (`docs/rules/inventory.md`), creatures outside combat
//! (`docs/rules/creatures.md`) and magic (`docs/rules/magic.md`).

mod ai;
pub mod creatures;
mod describe;
mod game;
pub mod inventory;
pub mod magic;
pub mod rng;
mod step;

pub use describe::describe;
pub use game::{Game, Player, PlayerSetup};
pub use rng::Rng;

use acs_core::model::SpellAction;
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

/// What a player asks for. Commands are the replay log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum Command {
    /// One step for the player whose turn it is.
    Move { dir: Dir },
    /// End the current turn now (also what a real-time drain would send).
    Wait,
    /// "Add Someone": a new character joins at the adventure's start.
    AddPlayer { setup: PlayerSetup },
    /// "Drop Object": one of `thing` onto the adjacent square in `dir`, or
    /// the player's own square when `dir` is `None` (inventory 5).
    Drop {
        thing: u8,
        #[serde(default)]
        dir: Option<Dir>,
    },
    /// Drops one pile of `amount` (1..=255) gold pieces (inventory 5.8).
    DropGold {
        amount: u16,
        #[serde(default)]
        dir: Option<Dir>,
    },
    /// "New Weapon" / "New Armor": readies a carried weapon or armor.
    Ready { thing: u8 },
    /// "Use Power": casts a carried magic spell at `target` (the caster
    /// when `None`) (magic 4.1). Ends the turn.
    Cast {
        thing: u8,
        #[serde(default)]
        target: Option<Who>,
    },
    /// "Use Object": uses a carried magic item (magic 4.2). Ends the turn.
    Use {
        thing: u8,
        #[serde(default)]
        target: Option<Who>,
    },
}

/// Choices the original leaves open or that are still unmeasured
/// (see the "Engine:" notes in `docs/rules/movement.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rules {
    /// Allow diagonal steps. Unknown in the original; on by default.
    pub diagonals: bool,
}

impl Default for Rules {
    fn default() -> Self {
        Rules { diagonals: true }
    }
}

/// Why a move was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum BlockReason {
    /// The edge of a world map that does not wrap.
    MapEdge,
    /// Impassable terrain (0..16).
    Terrain { terrain: u8 },
    /// Terrain, a door or a space open only to owners of `thing`.
    NeedsThing { thing: u8 },
    /// A door or space closed to owners of `thing`.
    ClosedToOwnersOf { thing: u8 },
    /// A plain wall square.
    Wall,
    /// An obstacle or custom obstacle (master thing number).
    Obstacle { thing: u8 },
    /// A store (stores come with a later rule set).
    Store { thing: u8 },
    /// A diagonal step while [`Rules::diagonals`] is off.
    Diagonal,
    /// The movement bar is empty (Speed 0); only `Wait` helps.
    NoMovesLeft,
    /// The top thing is carriable but the player already owns as many of it
    /// as anyone may (inventory 2.2).
    CarryLimit { thing: u8 },
}

/// Why a drop was refused (it costs nothing).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum DropRefusal {
    /// The player does not carry it (or not that much gold).
    NotCarried,
    /// Magic spells cannot be dropped.
    MagicSpell,
    /// A weapon or armor usable only by its owner.
    OwnerOnly,
    /// The target is a wall or outside the room.
    Wall,
    /// The target's top thing does not take drops (portal, obstacle, store).
    Thing { thing: u8 },
    /// A gold amount of 0 or more than one pile holds (255).
    BadAmount,
}

/// Why `Ready` was refused (it costs nothing).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum ReadyRefusal {
    NotCarried,
    /// Only weapons and armor can be readied.
    NotWeaponOrArmor,
}

/// The two things an individual readies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Slot {
    /// A melee or missile weapon.
    Weapon,
    Armor,
}

/// What set off a spell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpellSource {
    Terrain,
    Portal,
    Space,
    CustomSpace,
    Obstacle,
    CustomObstacle,
    PickedUp,
    /// A magic item's drop spell.
    Dropped,
    /// A magic spell cast with `Cast`.
    Cast,
    /// A magic item used with `Use`.
    Used,
    /// A magic item or spell lying in a stack, run by Activate All.
    Stack,
}

/// Why `Cast` or `Use` was refused (it costs nothing).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum CastRefusal {
    NotCarried,
    /// `Cast` needs a magic spell.
    NotASpell,
    /// `Use` needs a magic item that acts when used.
    NotUsable,
    /// The target is not an individual in the caster's room.
    BadTarget,
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
    pub fn from_spell_parameter(p: u8) -> Option<Stat> {
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

/// What happened. Front ends draw from these; tests compare them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum Event {
    TurnStarted {
        player: usize,
        moves: u8,
        /// The bar was halved for standing next to an aggressive, unfriendly
        /// creature (creatures 2.6).
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        hindered: bool,
    },
    TurnEnded {
        player: usize,
    },
    PlayerAdded {
        player: usize,
        at: Location,
    },
    Moved {
        player: usize,
        from: Location,
        to: Location,
    },
    /// The move was refused; the movement bar did not change.
    Blocked {
        player: usize,
        /// The square the player tried to enter.
        target: Location,
        #[serde(flatten)]
        reason: BlockReason,
        /// The thing's blocked message, when it has one that is shown.
        #[serde(skip_serializing_if = "Option::is_none", default)]
        message: Option<String>,
    },
    /// Walked into a resident creature (combat comes later).
    BumpedCreature {
        player: usize,
        region: u8,
        room: u8,
        /// Index into the region's resident list.
        resident: usize,
    },
    PickedUp {
        player: usize,
        thing: u8,
    },
    /// Picked up a pile of gold (thing 81).
    GoldPickedUp {
        player: usize,
        amount: u16,
    },
    /// A weapon or armor is now readied (`Ready`, or a weapon picked up).
    Readied {
        player: usize,
        thing: u8,
        slot: Slot,
    },
    ReadyRefused {
        player: usize,
        thing: u8,
        #[serde(flatten)]
        reason: ReadyRefusal,
    },
    /// The thing now lies on top of the stack at `at`.
    Dropped {
        player: usize,
        thing: u8,
        at: Location,
    },
    /// A pile of gold was put down at `at`, or lost on the world map (`None`).
    GoldDropped {
        player: usize,
        amount: u16,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        at: Option<Location>,
    },
    /// Dropped on the world map: gone from the adventure.
    LostOnMap {
        player: usize,
        thing: u8,
    },
    /// A treasure that disappears when dropped.
    VanishedWhenDropped {
        player: usize,
        thing: u8,
    },
    DropRefused {
        player: usize,
        /// Thing 81 for gold.
        thing: u8,
        #[serde(flatten)]
        reason: DropRefusal,
    },
    /// A door, space or "when dropped here" space destroyed the needed thing.
    ThingUsedUp {
        player: usize,
        thing: u8,
    },
    /// A disappearing obstacle was bumped and is gone.
    ObstacleVanished {
        at: Location,
        thing: u8,
    },
    /// A spell went off; its effects follow as further events (magic 3).
    SpellTriggered {
        /// Who set it off (stepped on, bumped, picked up, dropped, cast).
        by: Who,
        /// The individual it acts on, for spells that act on one (magic 2).
        #[serde(skip_serializing_if = "Option::is_none", default)]
        target: Option<Who>,
        source: SpellSource,
        /// The master thing that carries the spell.
        thing: u8,
        /// The action with its parameter resolved (placement parameters of
        /// custom spaces and custom obstacles already applied).
        action: SpellAction,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        message: Option<String>,
    },
    /// Took a room door (`thing` = its master thing) or a map door (`None`).
    TookDoor {
        player: usize,
        from: Location,
        to: Location,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        thing: Option<u8>,
    },
    EnteredRoom {
        player: usize,
        region: u8,
        room: u8,
    },
    /// Entering or leaving a region is when the original autosaves.
    EnteredRegion {
        player: usize,
        region: u8,
    },
    LeftRegion {
        player: usize,
        region: u8,
    },
    EnteredWorldMap {
        player: usize,
    },
    /// A door into a deleted (or impossible) room put the player at the
    /// adventure's start.
    SentToStart {
        player: usize,
    },
    /// A world map random creature appeared (roll only for now: no
    /// encounter area, the move goes ahead).
    Encounter {
        player: usize,
        /// Index into the world map's random creature list.
        creature: usize,
        terrain: u8,
    },
    /// A room's random creature appeared on entry (creatures 3). `slot` and
    /// `at` are `None` when the roll hit but no square was free.
    RoomEncounter {
        player: usize,
        region: u8,
        room: u8,
        /// Index into the region's random creature list.
        creature: u8,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        slot: Option<usize>,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        at: Option<Location>,
    },
    /// A creature's turn in a creature phase (creatures 2.2).
    CreatureTurn {
        region: u8,
        slot: usize,
        moves: u8,
    },
    CreatureMoved {
        region: u8,
        slot: usize,
        from: Location,
        to: Location,
    },
    /// Took a door inside its region; `from` is the door square.
    CreatureTookDoor {
        region: u8,
        slot: usize,
        from: Location,
        to: Location,
    },
    CreaturePickedUp {
        region: u8,
        slot: usize,
        thing: u8,
    },
    CreatureGoldPickedUp {
        region: u8,
        slot: usize,
        amount: u16,
    },
    /// Put down `amount` of `thing` (gold pieces for thing 81, else 1) at
    /// `at`, or `None` when a treasure vanished on being dropped.
    CreatureDropped {
        region: u8,
        slot: usize,
        thing: u8,
        amount: u16,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        at: Option<Location>,
    },
    CreatureReadied {
        region: u8,
        slot: usize,
        thing: u8,
    },
    /// A fighter stands next to a foe it sought; combat will make this an
    /// attack (creatures 7.1).
    CreatureConfronts {
        region: u8,
        slot: usize,
        target: Who,
    },
    /// A door used up a creature's needed thing.
    CreatureUsedUp {
        region: u8,
        slot: usize,
        thing: u8,
    },
    CreatureRemoved {
        region: u8,
        slot: usize,
        reason: RemovalReason,
    },
    /// `Cast` or a creature's cast: the roll against Power (magic 4.1). The
    /// cost is already off the caster's Power.
    SpellCast {
        by: Who,
        thing: u8,
        target: Who,
        /// Percent chance of success that was rolled against.
        chance: u8,
        cost: u8,
        success: bool,
    },
    /// `Use` or a creature using a magic item (magic 4.2).
    ItemUsed {
        by: Who,
        thing: u8,
        target: Who,
    },
    CastRefused {
        player: usize,
        thing: u8,
        #[serde(flatten)]
        reason: CastRefusal,
    },
    /// A spell that disappears after use is gone from its owner.
    SpellGone {
        who: Who,
        thing: u8,
    },
    /// A spell changed a rating.
    StatChanged {
        who: Who,
        stat: Stat,
        from: u8,
        to: u8,
    },
    /// Give: `kept` is false when the carry limit refused it (magic 3.9).
    Given {
        who: Who,
        thing: u8,
        kept: bool,
    },
    LongMessage {
        number: u8,
        text: String,
    },
    /// Play Music, 1-based choice 1..=30 (magic 3.11).
    Music {
        number: u8,
    },
    /// Rid Room: `count` copies of `thing` lying in the room are gone.
    RoomRidOf {
        region: u8,
        room: u8,
        thing: u8,
        count: u16,
    },
    /// Add to Room: one `thing` now lies on top at `at`.
    ThingAdded {
        thing: u8,
        at: Location,
    },
    /// A summoned master creature arrived (magic 3.2).
    CreatureSummoned {
        region: u8,
        slot: usize,
        creature: u8,
        at: Location,
    },
    /// A summons found no free slot or square, or was made on the map.
    SummonUnanswered {
        creature: u8,
    },
    /// Life Force reached 0 (magic 7). `left` lists what now lies at `at`
    /// (thing, amount; gold in pieces).
    Died {
        who: Who,
        at: Location,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        left: Vec<(u8, u16)>,
    },
    /// No player is left alive; the game takes no more commands.
    AllPlayersDead,
}

/// An individual: a player or a creature in a region slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "who", rename_all = "snake_case")]
pub enum Who {
    Player { player: usize },
    Creature { region: u8, slot: usize },
}

/// Why a creature left the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemovalReason {
    /// A random creature left in a room with no player (creatures 2.4).
    LeftBehind,
    /// Sent through a door into a deleted room (creatures 5.5), or by a
    /// Banish spell (magic 3.2).
    Banished,
}
