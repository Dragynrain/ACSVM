// SPDX-License-Identifier: GPL-3.0-or-later

//! What a player asks for: [`Command`]s, the replay log.

use crate::{Dir, Player, PlayerSetup, Who};
use acs_core::model::{PictureId, ThingId};
use serde::{Deserialize, Serialize};

/// What a player asks for. Commands are the replay log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum Command {
    /// One step for the player whose turn it is.
    Move { dir: Dir },
    /// The menu's REST (magic 5.6): the rest of the bar goes at once as
    /// idle moves, each with its healing roll, and the turn ends.
    Wait,
    /// The real-time drain (movement 2.4): one step of the bar went while
    /// the player did nothing. One idle move; the turn ends with the last.
    Idle,
    /// "Add Someone" with a character whose ratings are given (a saved
    /// one, characters 3): it joins at the adventure's start. In play the
    /// request is handled at the round's end (characters 6.2).
    AddPlayer { setup: PlayerSetup },
    /// "Add Someone", "Create New Player": a new character is rolled
    /// (characters 1) and joins at the adventure's start; in play, rolled
    /// at the round's end (characters 6.2).
    /// `picture`: the one chosen (characters 2.4), none for the first
    /// offered; written only when chosen (saves 2.1).
    CreatePlayer {
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        picture: Option<PictureId>,
    },
    /// "Add Someone" with a character retired with Save, chosen from the
    /// adventure's own list (characters 3.1): `player` is the record its
    /// `Retired` carried. It comes back as saved, gear readied, where it
    /// was saved; in play, at the round's end (characters 6.2).
    Rejoin { player: Box<Player> },
    /// "Drop Object": one of `thing` onto the adjacent square in `dir`, or
    /// the player's own square when `dir` is `None` (inventory 5).
    Drop {
        thing: ThingId,
        #[serde(default)]
        dir: Option<Dir>,
    },
    /// Drops one pile of `amount` (1..=`inventory::PILE_LIMIT`, 25,000) gold
    /// pieces (inventory 5.8).
    DropGold {
        amount: u16,
        #[serde(default)]
        dir: Option<Dir>,
    },
    /// "New Weapon" / "New Armor": readies a carried weapon or armor.
    Ready { thing: ThingId },
    /// "Use Power": casts a carried magic spell (magic 4.1). Ends the turn.
    /// The target (magic 2.4) is the square `at` of the caster's room when
    /// given; else the square `target` stands on now; else the caster's
    /// own.
    Cast {
        thing: ThingId,
        #[serde(default)]
        target: Option<Who>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        at: Option<(u8, u8)>,
    },
    /// "Use Object": uses a carried magic item (magic 4.2), aimed as
    /// `Cast` is.
    Use {
        thing: ThingId,
        #[serde(default)]
        target: Option<Who>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        at: Option<(u8, u8)>,
    },
    /// The menu's FIRE (combat 8.8), in a room. `target` is the cursor's
    /// square (`None`: left on the player's own square), `weapon` the owned
    /// missile weapon chosen from the menu when none is held (`None` for
    /// NONE). Refused with no move used beside a foe or on the world map.
    Fire {
        #[serde(default)]
        target: Option<(u8, u8)>,
        #[serde(default)]
        weapon: Option<ThingId>,
    },
    /// RETIRE (characters 7): the current player retires at the end of the
    /// round, saved for later adventures (`save`) or destroyed. The offer's
    /// answer is given with the request. No move used.
    Retire { save: bool },
    /// The menu's SAVE GAME (creatures 2.2.7): the game is saved at the end
    /// of the round, with the round's other requests. No move used and the
    /// turn goes on.
    SaveGame,
    /// In an open store (stores 3): buys one of `thing` (stores 4).
    Buy { thing: ThingId },
    /// In an open store: sells one of `thing` (stores 5).
    Sell { thing: ThingId },
    /// Leaves the open store (stores 3.8).
    LeaveStore,
    /// The death message's button (creatures 2.2.6), played on the game
    /// rebuilt at [`Game::restore_point`](crate::Game::restore_point): the saved state comes back as it
    /// was when saved, the visit ends and a new cycle begins from the
    /// restored places. While nobody is in play it is accepted only on the
    /// game at an all-dead save (creatures 2.2.7), and ignored otherwise.
    Restore,
}

/// Why a store refused a purchase or a sale (stores 4.1, 4.3, 5.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StoreRefusal {
    /// No store is open.
    NoStore,
    /// The store has none of it.
    NotStocked,
    NotEnoughGold,
    /// Spells and gold can't be sold.
    NotSellable,
    /// The player has none to sell.
    NotOwned,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Location, Newcomer};

    /// Saves 2.1, version 5: `rejoin` carries the saved record and reads
    /// back as it was; as a waiting newcomer (in the game's fingerprint)
    /// its own key keeps it apart from a `Join` setup.
    #[test]
    fn a_rejoin_reads_back_as_it_was_written() {
        let p = Player {
            name: "T".into(),
            speed: 6,
            strength: 11,
            constitution: 11,
            life_force: 9,
            moves_left: 6,
            turn_moves: 6,
            at: Location::Room {
                region: 1,
                room: 2,
                x: 3,
                y: 4,
            },
            possessions: vec![3, 5],
            gold: 40,
            weapon: Some(3),
            armor: None,
            dexterity: 11,
            wisdom: 11,
            power: 12,
            size: 11,
            dodge: 22,
            parry: 22,
            armor_skill: 0,
            melee: 33,
            missile: 22,
            magic_defense: acs_core::model::MagicDefense::None,
            alive: true,
            picture: None,
        };
        let cmd = Command::Rejoin {
            player: Box::new(p.clone()),
        };
        let text = serde_json::to_string(&cmd).unwrap();
        assert!(text.starts_with(r#"{"command":"rejoin","player":"#), "{text}");
        assert_eq!(serde_json::from_str::<Command>(&text).unwrap(), cmd);
        let n = Newcomer::Rejoin { rejoin: Box::new(p) };
        let text = serde_json::to_string(&n).unwrap();
        assert_eq!(serde_json::from_str::<Newcomer>(&text).unwrap(), n);
    }
}
