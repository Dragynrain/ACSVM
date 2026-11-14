// SPDX-License-Identifier: GPL-3.0-or-later

//! Save files (`docs/saves.md`): a game is kept as the replay that makes
//! it, its seed, rules, first player, tool placements and command log, plus
//! the point of the last autosave. The adventure is not in the file; it is
//! loaded over the same adventure, and fingerprints say whether the
//! adventure and the replayed game are the ones the save was made with.

use crate::{Command, Game, Location, Newcomer, Rules};
use acs_core::model::Adventure;
use serde::{Deserialize, Serialize};
use std::fmt;

/// The `format` field of every save (saves 2).
pub const SAVE_FORMAT: &str = "acsvm-save";
/// The save format version this engine writes and reads.
pub const SAVE_VERSION: u32 = 1;

/// A tool placement (`Game::place`): made after `after` commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Placement {
    pub after: usize,
    pub player: usize,
    pub at: Location,
}

/// Which adventure a save belongs to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdventureId {
    pub name: String,
    /// [`fingerprint`] of the decoded adventure as play began.
    pub fingerprint: String,
}

/// A save file (saves 2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaveFile {
    pub format: String,
    pub version: u32,
    pub engine: String,
    pub adventure: AdventureId,
    pub seed: u64,
    pub rules: Rules,
    pub first: Newcomer,
    #[serde(default)]
    pub placements: Vec<Placement>,
    pub commands: Vec<Command>,
    /// Commands played when the last autosave was made.
    pub autosave: usize,
    /// [`fingerprint`] of the game after every command.
    pub state: String,
}

/// Why a save can't be loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveError {
    /// Not JSON, or not a save.
    NotASave(String),
    /// A version this engine doesn't read.
    Version(u32),
    /// The save is for another adventure.
    WrongAdventure { save: String, loaded: String },
    /// Out-of-range autosave point or placement.
    Damaged(String),
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SaveError::NotASave(e) => write!(f, "not a save file: {e}"),
            SaveError::Version(v) => write!(
                f,
                "save format version {v} is newer than this engine reads ({SAVE_VERSION})"
            ),
            SaveError::WrongAdventure { save, loaded } => {
                write!(f, "this save is for \"{save}\", not \"{loaded}\"")
            }
            SaveError::Damaged(e) => write!(f, "damaged save: {e}"),
        }
    }
}

impl std::error::Error for SaveError {}

/// A loaded save: the game, the autosave to resume from after a death, and
/// the warnings of saves 3.2.
#[derive(Debug, Clone)]
pub struct Restored {
    pub game: Game,
    pub autosave: Game,
    /// The adventure's fingerprint differs (same name).
    pub adventure_changed: bool,
    /// The replayed game differs from the one saved.
    pub diverged: bool,
}

/// 64-bit FNV-1a over `value`'s JSON form, as 16 hex digits (saves 3.1).
pub fn fingerprint<T: Serialize + ?Sized>(value: &T) -> String {
    let bytes = serde_json::to_vec(value).expect("model values serialize");
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{h:016x}")
}

impl SaveFile {
    /// The save of `game`, played on `original` (the adventure as decoded,
    /// before play changed it), with its last autosave `autosave` (a game
    /// whose log is a prefix of `game`'s; saves 1.4).
    pub fn new(game: &Game, original: &Adventure, autosave: &Game) -> Self {
        SaveFile {
            format: SAVE_FORMAT.into(),
            version: SAVE_VERSION,
            engine: env!("CARGO_PKG_VERSION").into(),
            adventure: AdventureId {
                name: original.name.clone(),
                fingerprint: fingerprint(original),
            },
            seed: game.seed(),
            rules: game.rules(),
            first: game.first_player().clone(),
            placements: game.placements().to_vec(),
            commands: game.log().to_vec(),
            autosave: autosave.log().len().min(game.log().len()),
            state: fingerprint(game),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("saves serialize")
    }

    /// Reads a save, checking only that it is one (saves 3.2, first check).
    pub fn from_json(text: &str) -> Result<Self, SaveError> {
        #[derive(Deserialize)]
        struct Head {
            format: String,
            version: u32,
        }
        let head: Head = serde_json::from_str(text).map_err(|e| SaveError::NotASave(e.to_string()))?;
        if head.format != SAVE_FORMAT {
            return Err(SaveError::NotASave(format!("format is \"{}\"", head.format)));
        }
        if head.version > SAVE_VERSION {
            return Err(SaveError::Version(head.version));
        }
        serde_json::from_str(text).map_err(|e| SaveError::Damaged(e.to_string()))
    }

    /// Replays the save over `adv` (saves 3.2).
    pub fn restore(&self, adv: &Adventure) -> Result<Restored, SaveError> {
        if self.adventure.name != adv.name {
            return Err(SaveError::WrongAdventure {
                save: self.adventure.name.clone(),
                loaded: adv.name.clone(),
            });
        }
        let n = self.commands.len();
        if self.autosave > n {
            return Err(SaveError::Damaged(format!(
                "autosave after {} commands, only {n} saved",
                self.autosave
            )));
        }
        if let Some(p) = self.placements.iter().find(|p| p.after > n) {
            return Err(SaveError::Damaged(format!(
                "placement after {} commands, only {n} saved",
                p.after
            )));
        }
        let adventure_changed = fingerprint(adv) != self.adventure.fingerprint;
        let (mut game, _) = Game::new(adv.clone(), self.seed, self.rules, self.first.clone());
        let mut autosave = None;
        let mut places = self.placements.iter().peekable();
        for i in 0..=n {
            while let Some(p) = places.next_if(|p| p.after == i) {
                if p.player >= game.players().len() {
                    return Err(SaveError::Damaged(format!(
                        "placement of player {} after {i} commands, who isn't there",
                        p.player
                    )));
                }
                game.place(p.player, p.at);
            }
            if i == self.autosave {
                autosave = Some(game.clone());
            }
            if i < n {
                game.apply(self.commands[i].clone());
            }
        }
        if places.next().is_some() {
            return Err(SaveError::Damaged("placements out of order".into()));
        }
        let diverged = fingerprint(&game) != self.state;
        Ok(Restored {
            autosave: autosave.expect("autosave is within the commands"),
            game,
            adventure_changed,
            diverged,
        })
    }
}
