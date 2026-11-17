// SPDX-License-Identifier: GPL-3.0-or-later

//! What happened: the [`Event`]s a command returns, and the reasons a
//! command was refused. Front ends draw from these; tests compare them.

use crate::{BlowOutcome, Location, Player, Stat, StoreRefusal};
use acs_core::model::{Alignment, SpellAction, ThingId};
use serde::{Deserialize, Serialize};

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
    /// A diagonal step while [`Rules::diagonals`](crate::Rules::diagonals) is off.
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
    /// The target is a wall (a portal on the wall line included) or
    /// outside the room.
    Wall,
    /// The target's top thing does not take drops (an obstacle or a custom
    /// obstacle, inventory 5.4).
    Thing { thing: ThingId },
    /// A diagonal square: a drop goes on the own square or one of the four
    /// beside it (inventory 5.1).
    NotBeside,
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
    /// The target is not a square inside the caster's room (a wall, or
    /// another square on the world map), or the individual named is not
    /// there (magic 2.4).
    BadTarget,
    /// The target square has no individual, for a spell that acts on one
    /// (magic 2.4).
    NoOneThere,
}

/// Why FIRE did nothing (combat 8.8.2, 8.8.5); no move is used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum FireRefusal {
    /// A foe stands beside the player (8.8.2): the message names it.
    FoeBeside { foe: Who },
    /// On the world map FIRE does nothing (8.8.5).
    WorldMap,
    /// **Engine:** the cursor's square is outside the room's walls (the
    /// original's cursor can't go there).
    BadTarget,
    /// **Engine:** the chosen weapon is not a missile weapon the player
    /// owns (the original's menu lists only those).
    NotAMissileWeapon,
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
    /// A thing with the disappearing bit (an obstacle or custom obstacle
    /// at a bump, a space or custom space at a step, any of them under
    /// Activate All) was set off and is gone (movement 5.9).
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
    /// Went into another region; the save comes when its visit begins
    /// (`Saved`, creatures 2.2.7), not here.
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
    /// A world map random creature's roll hit after a map step
    /// (encounters 2.5, 2.6): its copy stands in `area` in `slot` of the
    /// map region (0), or `slot` is `None` when every slot 8..15 is taken.
    Encounter {
        player: usize,
        /// Index into the world map's random creature list.
        creature: usize,
        terrain: u8,
        /// The encounter area, 1..=4.
        area: u8,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        slot: Option<usize>,
    },
    /// Pulled into (or joining) encounter area `area`; the turn ends
    /// (encounters 5).
    EnteredArea {
        player: usize,
        area: u8,
        from: Location,
        to: Location,
    },
    /// Walked out of an encounter area onto its entrance (encounters 7.1).
    LeftArea {
        player: usize,
        area: u8,
        from: Location,
        to: Location,
    },
    /// The area has no player left and is cleared (encounters 7.2).
    AreaClosed {
        area: u8,
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
    /// Bumped the obstacle under its goal and stayed put (creatures 5.1);
    /// `at` is the obstacle's square.
    CreatureBumped {
        region: u8,
        slot: usize,
        at: Location,
        thing: ThingId,
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
    /// An individual fires `weapon` (combat 8.1, 8.8): a creature at
    /// `target`, a player at the square `at`; `shots` shots (8.2) follow as
    /// `Shot`s; it ends the shooter's turn.
    Fired {
        by: Who,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        target: Option<Who>,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        at: Option<(u8, u8)>,
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
    /// FIRE refused (combat 8.8.2, 8.8.5): no move used, nothing drawn.
    CannotFire {
        player: usize,
        #[serde(flatten)]
        reason: FireRefusal,
    },
    /// FIRE with no missile weapon held (8.8.4): its move is used and NONE
    /// was chosen; no shot, the turn goes on.
    FireCancelled {
        player: usize,
    },
    /// Bumping a store opened it (stores 3): `region`'s stock, `gold` to
    /// spend.
    StoreOpened {
        player: usize,
        region: u8,
        gold: u16,
    },
    /// The store visit ended (stores 3.8).
    StoreClosed {
        player: usize,
    },
    /// One of `thing` bought for `price` gold (stores 4); `kept` is false
    /// when the player already owned as many as allowed and the thing was
    /// lost (stores 4.3).
    Bought {
        player: usize,
        thing: ThingId,
        price: i32,
        kept: bool,
    },
    /// One of `thing`, broken or used up, joined `region`'s store stock
    /// (stores 2.5).
    Restocked {
        region: u8,
        thing: ThingId,
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
    /// Display long message (magic 3.10): `text` is its stored lines, one
    /// per line, separated by line breaks, not to be rewrapped.
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
    /// A summons found no free slot (a cast into a full region fails
    /// before anything is spent, magic 3.2), or was made on the map.
    SummonUnanswered {
        creature: u16,
    },
    /// Life Force reached 0 (magic 7). `left` lists what now lies at `at`
    /// (thing, amount; gold in pieces), in the order put down, the last
    /// on top (magic 7.2: the gold first).
    Died {
        who: Who,
        at: Location,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        left: Vec<(ThingId, u16)>,
    },
    /// No player is left in play (all dead or retired); the game takes no
    /// more commands but adding a player (magic 7.4, characters 7.6).
    AllPlayersDead,
    /// RETIRE taken: the player retires at the end of the round
    /// (characters 7.1).
    RetireRequested {
        player: usize,
    },
    /// Another player will retire first: nothing recorded (characters 7.1).
    RetireRefused {
        player: usize,
    },
    /// At the round's end the player retired and its slot is free
    /// (characters 7.3-7.6). Saved: `saved` holds the whole record, Life
    /// Force raised to Constitution, for the front end to keep. Destroyed in
    /// a room: `left` lists what now lies on its square (thing, amount;
    /// gold in pieces).
    Retired {
        player: usize,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        saved: Option<Box<Player>>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        left: Vec<(ThingId, u16)>,
    },
    /// SAVE GAME taken: the game will be saved at the end of the round
    /// (creatures 2.2.7).
    SaveRequested {
        player: usize,
    },
    /// The game was saved (creatures 2.2.7): at the start of a visit to
    /// region `visit`, another than the one visited before, or (`None`) at
    /// the end of a round with SAVE GAME, a Retire with Save or an add that
    /// brought a newcomer in (after `Retired` and `PlayerAdded`; one a
    /// round for these, 2.2.7.4). `round` is the counter it holds,
    /// the round play resumes in. Restoring goes back to the latest save
    /// ([`Game::restore_point`](crate::Game::restore_point)).
    Saved {
        round: u32,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        visit: Option<u8>,
    },
    /// "Add Someone" taken: the newcomer is handled at the end of the round
    /// (characters 6.2).
    AddRequested,
    /// At the end of the round an "Add Someone" found `MAX_PLAYERS`
    /// living: one must retire first (characters 6.2, movement 1.4).
    AddRefused,
    /// A name whose first three characters are spaces: name entry asks
    /// again and nothing is added (characters 5.6, 5.7).
    NameRefused,
    /// The number of living players differed at the round's end from its
    /// start: the round-end message (restore the last save, or go on;
    /// creatures 2.2.5, 2.2.6, characters 6.3).
    LivingChanged {
        before: usize,
        after: usize,
    },
    /// The last saved game came back (`Command::Restore`, creatures
    /// 2.2.6), its counter at `round`; a new cycle begins from the
    /// restored places.
    Restored {
        round: u32,
    },
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
    /// Its encounter area closed (encounters 7.2).
    AreaClosed,
}
