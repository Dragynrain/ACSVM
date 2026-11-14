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
//! (`docs/rules/magic.md`) and melee combat (`docs/rules/combat.md`).

mod ai;
pub mod combat;
pub mod creatures;
mod describe;
mod game;
pub mod inventory;
pub mod magic;
pub mod rng;
mod step;
pub mod stores;

pub use combat::BlowOutcome;
pub use describe::describe;
pub use game::{Game, Player, PlayerSetup};
pub use rng::Rng;

use acs_core::model::ThingId;
use acs_core::model::{Alignment, SpellAction};
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
        thing: ThingId,
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
    Ready { thing: ThingId },
    /// "Use Power": casts a carried magic spell at `target` (the caster
    /// when `None`) (magic 4.1). Ends the turn.
    Cast {
        thing: ThingId,
        #[serde(default)]
        target: Option<Who>,
    },
    /// "Use Object": uses a carried magic item (magic 4.2). Ends the turn.
    Use {
        thing: ThingId,
        #[serde(default)]
        target: Option<Who>,
    },
    /// Firing a missile weapon: players can't (combat 8.1); a message, no
    /// shot, no move used.
    Fire,
    /// In an open store (stores 3): buys one of `thing` (stores 4).
    Buy { thing: ThingId },
    /// In an open store: sells one of `thing` (stores 5).
    Sell { thing: ThingId },
    /// Leaves the open store (stores 3.4).
    LeaveStore,
}

/// Why a store refused a purchase or a sale (stores 4.3, 5.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StoreRefusal {
    /// No store is open.
    NoStore,
    /// The store has none of it.
    NotStocked,
    NotEnoughGold,
    /// The player owns as many as allowed (inventory 2.2).
    CarryLimit,
    /// Spells and gold can't be sold.
    NotSellable,
    /// The player has none to sell.
    NotOwned,
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

/// Why a move was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum BlockReason {
    /// The edge of a world map that does not wrap.
    MapEdge,
    /// Impassable terrain (0..16).
    Terrain { terrain: u8 },
    /// Terrain, a door or a space open only to owners of `thing`.
    NeedsThing { thing: ThingId },
    /// A door or space closed to owners of `thing`.
    ClosedToOwnersOf { thing: ThingId },
    /// A plain wall square.
    Wall,
    /// An obstacle or custom obstacle (master thing number).
    Obstacle { thing: ThingId },
    /// A store: bumping it opens it (stores 3).
    Store { thing: ThingId },
    /// A diagonal step while [`Rules::diagonals`] is off.
    Diagonal,
    /// The movement bar is empty (Speed 0); only `Wait` helps.
    NoMovesLeft,
    /// The top thing is carriable but the player already owns as many of it
    /// as anyone may (inventory 2.2).
    CarryLimit { thing: ThingId },
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
    Thing { thing: ThingId },
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
    /// Walked into another player holding and owning no melee weapon:
    /// nothing happens (combat 2.7).
    BumpedPlayer {
        player: usize,
        other: usize,
    },
    /// Walked into a resident creature holding and owning no melee weapon:
    /// nothing happens (combat 2.7).
    BumpedCreature {
        player: usize,
        region: u8,
        room: u8,
        /// Index into the region's resident list.
        resident: usize,
    },
    PickedUp {
        player: usize,
        thing: ThingId,
    },
    /// Picked up a pile of gold (thing 81).
    GoldPickedUp {
        player: usize,
        amount: u16,
    },
    /// A weapon or armor is now readied (`Ready`, or a weapon picked up).
    Readied {
        player: usize,
        thing: ThingId,
        slot: Slot,
    },
    ReadyRefused {
        player: usize,
        thing: ThingId,
        #[serde(flatten)]
        reason: ReadyRefusal,
    },
    /// The thing now lies on top of the stack at `at`.
    Dropped {
        player: usize,
        thing: ThingId,
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
        thing: ThingId,
    },
    /// A treasure that disappears when dropped.
    VanishedWhenDropped {
        player: usize,
        thing: ThingId,
    },
    DropRefused {
        player: usize,
        /// Thing 81 for gold.
        thing: ThingId,
        #[serde(flatten)]
        reason: DropRefusal,
    },
    /// A door, space or "when dropped here" space destroyed the needed thing.
    ThingUsedUp {
        player: usize,
        thing: ThingId,
    },
    /// A disappearing obstacle was bumped and is gone.
    ObstacleVanished {
        at: Location,
        thing: ThingId,
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
        thing: ThingId,
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
        thing: Option<ThingId>,
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
        thing: ThingId,
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
        thing: ThingId,
        amount: u16,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        at: Option<Location>,
    },
    CreatureReadied {
        region: u8,
        slot: usize,
        thing: ThingId,
    },
    /// A melee attack starts (combat 3): `weapon` in hand, `blows` to
    /// strike. Its blows follow; it ends the attacker's turn.
    Attacked {
        by: Who,
        target: Who,
        weapon: ThingId,
        blows: u8,
    },
    /// One blow of an attack (combat 3.3), `counter` for a counter-blow.
    /// `chance` is the hit chance rolled against. Lost Life Force and
    /// Power follow as `StatChanged`, a death as `Died`.
    Blow {
        by: Who,
        target: Who,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        counter: bool,
        chance: u8,
        #[serde(flatten)]
        outcome: BlowOutcome,
    },
    /// A creature fires `weapon` at `target` (combat 8.1): `shots` shots
    /// (8.2) follow as `Shot`s; it ends the shooter's turn.
    Fired {
        by: Who,
        target: Who,
        weapon: ThingId,
        shots: u8,
    },
    /// One shot leaves (combat 8.3): its `spread` (8.4), the `deviation`
    /// rolled below it (hundredths of the distance to the target) and the
    /// side it goes to. What it reaches follows as `Dodged` and
    /// `MissileHit`, or `MissileStopped`.
    Shot {
        by: Who,
        spread: u8,
        deviation: u8,
        side: bool,
    },
    /// An individual the missile reached dodged it (8.6), rolled below
    /// `chance`; the missile flies on.
    Dodged {
        by: Who,
        who: Who,
        chance: u8,
    },
    /// A missile hit (8.6): `outcome` is `Hit` (its damage after armor and
    /// magic defense, 0 when they took it all) or `Stopped` (magic
    /// defense); `dodge` the dodge chance it beat. Lost Life Force and
    /// Power follow as `StatChanged`. The missile stops.
    MissileHit {
        by: Who,
        target: Who,
        dodge: u8,
        #[serde(flatten)]
        outcome: BlowOutcome,
    },
    /// A wall or obstacle on (`x`, `y`) stopped a missile (8.5).
    MissileStopped {
        by: Who,
        x: u8,
        y: u8,
    },
    /// A player tried to fire (combat 8.1): players can't; nothing else
    /// happens.
    CannotFire {
        player: usize,
    },
    /// Bumping a store opened it (stores 3): `region`'s stock, `gold` to
    /// spend.
    StoreOpened {
        player: usize,
        region: u8,
        gold: u16,
    },
    /// The store visit ended (stores 3.4).
    StoreClosed {
        player: usize,
    },
    /// One of `thing` bought for `price` gold (stores 4).
    Bought {
        player: usize,
        thing: ThingId,
        price: i32,
    },
    /// One of `thing` sold: the store's `offer`, the gold `paid` once the
    /// purse cap took its share, and whether it joined the stock (stores 5).
    Sold {
        player: usize,
        thing: ThingId,
        offer: i32,
        paid: i32,
        stocked: bool,
    },
    /// A purchase or sale refused (stores 4.3, 5.2).
    StoreRefused {
        player: usize,
        thing: ThingId,
        reason: StoreRefusal,
    },
    /// A weapon or armor broke (combat 6.3), or was used up (`used_up`,
    /// 6.4), and is gone from its owner.
    Broke {
        who: Who,
        thing: ThingId,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        used_up: bool,
    },
    /// A creature hurt by a player changed its strategy (combat 7).
    CreatureTurned {
        region: u8,
        slot: usize,
        alignment: Alignment,
        aggressive: bool,
    },
    /// A door used up a creature's needed thing.
    CreatureUsedUp {
        region: u8,
        slot: usize,
        thing: ThingId,
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
        thing: ThingId,
        target: Who,
        /// Percent chance of success that was rolled against.
        chance: u8,
        cost: u8,
        success: bool,
    },
    /// `Use` or a creature using a magic item (magic 4.2).
    ItemUsed {
        by: Who,
        thing: ThingId,
        target: Who,
    },
    CastRefused {
        player: usize,
        thing: ThingId,
        #[serde(flatten)]
        reason: CastRefusal,
    },
    /// A spell that disappears after use is gone from its owner.
    SpellGone {
        who: Who,
        thing: ThingId,
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
        thing: ThingId,
        kept: bool,
    },
    LongMessage {
        number: u16,
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
        thing: ThingId,
        count: u16,
    },
    /// Add to Room: one `thing` now lies on top at `at`.
    ThingAdded {
        thing: ThingId,
        at: Location,
    },
    /// A summoned master creature arrived (magic 3.2).
    CreatureSummoned {
        region: u8,
        slot: usize,
        creature: u16,
        at: Location,
    },
    /// A summons found no free slot or square, or was made on the map.
    SummonUnanswered {
        creature: u16,
    },
    /// Life Force reached 0 (magic 7). `left` lists what now lies at `at`
    /// (thing, amount; gold in pieces).
    Died {
        who: Who,
        at: Location,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        left: Vec<(ThingId, u16)>,
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
