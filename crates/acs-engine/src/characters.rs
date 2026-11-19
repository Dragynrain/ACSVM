// SPDX-License-Identifier: GPL-3.0-or-later

//! New characters, names and the profile's marks
//! (`docs/rules/characters.md`).

use crate::game::{Game, PlayerSetup, NAME_MAX};
use crate::inventory::GOLD;
use crate::rng::Rng;
use crate::{Event, Location, Who};
use acs_core::model::{Adventure, MagicDefense, PictureId, ThingId, ThingKind};
use serde::{Deserialize, Serialize};

/// Wisdom from which a new character has magic defense level 1
/// (characters 2.1).
pub const DEFENSE_WISDOM: u8 = 16;

/// Three dice of 1 to 6 (characters 1.2).
fn three_dice(rng: &mut Rng) -> u8 {
    (0..3).map(|_| rng.below(6) as u8 + 1).sum()
}

impl PlayerSetup {
    /// "Create New Player" (characters 1, 2): Strength, Constitution,
    /// Dexterity, Wisdom and Size three dice each, in that order; then a
    /// roll of 0..=2, on 0 a second three dice for the first four, keeping
    /// the higher; everything else follows from those five.
    pub fn roll(name: &str, rng: &mut Rng) -> PlayerSetup {
        let mut t = [0u8; 5];
        for v in &mut t {
            *v = three_dice(rng);
        }
        if rng.below(3) == 0 {
            for v in &mut t[..4] {
                *v = (*v).max(three_dice(rng));
            }
        }
        let [strength, constitution, dexterity, wisdom, size] = t;
        PlayerSetup {
            name: fold_name(name),
            speed: 5 + dexterity / 7,
            strength,
            constitution,
            life_force: constitution,
            possessions: Vec::new(),
            gold: 0,
            dexterity,
            wisdom,
            power: Some(wisdom),
            size,
            // Characters 1.4: kept in a byte, so it wraps below 0.
            dodge: (3 * dexterity).wrapping_sub(size),
            parry: dexterity + wisdom,
            armor_skill: 0,
            melee: size + strength + dexterity,
            missile: dexterity + strength,
            magic_defense: if wisdom >= DEFENSE_WISDOM {
                MagicDefense::NonMagicHalfDamage
            } else {
                MagicDefense::None
            },
            picture: None,
        }
    }
}

/// The pictures a new character may take (characters 2.4), the first the
/// one it has when none is chosen: an Extended adventure's own list, else
/// every picture a creature of the master list uses, in picture order,
/// else every picture.
pub fn player_pictures(adv: &Adventure) -> Vec<PictureId> {
    let count = adv.pictures.len();
    let ok = |p: &PictureId| (*p as usize) < count;
    let own: Vec<PictureId> = adv.player_pictures.iter().copied().filter(ok).collect();
    if !own.is_empty() {
        return own;
    }
    let mut used: Vec<PictureId> = adv
        .creatures
        .iter()
        .filter(|c| c.in_use)
        .filter_map(|c| c.picture)
        .filter(ok)
        .collect();
    used.sort_unstable();
    used.dedup();
    if used.is_empty() {
        (0..count as PictureId).collect()
    } else {
        used
    }
}

/// The least gold a player from another adventure brings (3.3).
pub const STRANGER_GOLD: u16 = 1_000;

/// 3.3: a character saved in another adventure, as it joins this one:
/// every trait and skill, Life Force, Power and magic defense kept; no
/// possessions, nothing readied, the map's start cell; gold raised to at
/// least 1,000. `picture` is the one chosen here (3.2.4), none for the
/// first offered.
pub fn from_another_adventure(p: &crate::Player, picture: Option<PictureId>) -> PlayerSetup {
    PlayerSetup {
        name: p.name.clone(),
        speed: p.speed,
        strength: p.strength,
        constitution: p.constitution,
        life_force: p.life_force,
        possessions: Vec::new(),
        gold: p.gold.max(STRANGER_GOLD),
        dexterity: p.dexterity,
        wisdom: p.wisdom,
        power: Some(p.power),
        size: p.size,
        dodge: p.dodge,
        parry: p.parry,
        armor_skill: p.armor_skill,
        melee: p.melee,
        missile: p.missile,
        magic_defense: p.magic_defense,
        picture,
    }
}

/// 3.1: an entry of the adventure's own roster (`Adventure::retired_players`)
/// as the record `Rejoin` brings back: every rating, Life Force and Power
/// as saved, each possession once per count, its gold, the weapon held and
/// armor worn, its place (a region of 0 is the world map) and picture.
pub fn from_roster(c: &acs_core::model::Creature) -> crate::Player {
    let at = match c.region {
        0 => Location::World { x: c.x, y: c.y },
        region => Location::Room {
            region,
            room: c.room,
            x: c.x,
            y: c.y,
        },
    };
    let thing = |t: ThingId| (t != 0).then_some(t);
    crate::Player {
        name: c.name.clone(),
        speed: c.speed,
        strength: c.strength,
        constitution: c.constitution,
        life_force: c.life_force,
        moves_left: 0,
        turn_moves: 0,
        at,
        possessions: c
            .possessions
            .iter()
            .flat_map(|p| std::iter::repeat_n(p.thing, p.count as usize))
            .collect(),
        gold: c.wealth,
        weapon: thing(c.weapon),
        armor: thing(c.armor),
        dexterity: c.dexterity,
        wisdom: c.wisdom,
        power: c.power,
        size: c.size,
        dodge: c.dodge,
        parry: c.parry,
        armor_skill: c.armor_skill,
        melee: c.melee,
        missile: c.missile,
        magic_defense: c.magic_defense,
        alive: true,
        picture: c.picture,
    }
}

/// The picture `player` shows (characters 2.4): its own, else the first
/// offered.
pub fn picture_of(adv: &Adventure, picture: Option<PictureId>) -> Option<PictureId> {
    picture.or_else(|| player_pictures(adv).first().copied())
}

/// One typed key as name entry keeps it (characters 5.3): letters of either
/// case are letters, 1 to 9 digits, and every other key folds to one of
/// the 40 symbols; no key is refused. A character with no C64 key is a
/// space (5.7).
pub fn fold_key(c: char) -> char {
    match c.to_ascii_uppercase() {
        c @ ('A'..='Z' | '1'..='9' | '.' | ',' | '-' | '\'') => c,
        '0' => 'O',
        '!' | '?' => '.',
        '+' | '/' | ':' | ';' | '=' => '-',
        '"' => '\'',
        '*' => 'X',
        '&' => 'A',
        '\u{a3}' => 'L',   // pound
        '\u{2191}' => 'I', // up-arrow
        '\u{2190}' => 'K', // left-arrow
        _ => ' ',
    }
}

/// A name as the C64 keeps it (characters 5.3, 5.4, 5.7): each character a
/// typed key folded by [`fold_key`], the cursor wrapping past 15 so the
/// 16th replaces the 1st. Trailing spaces are the stored name's padding
/// (5.2) and are left off.
pub fn fold_name(name: &str) -> String {
    let mut buf = [' '; NAME_MAX];
    for (i, c) in name.chars().enumerate() {
        buf[i % NAME_MAX] = fold_key(c);
    }
    let s: String = buf.iter().collect();
    s.trim_end().to_string()
}

/// True when name entry asks again for `name` (characters 5.6, 5.7): its
/// first three characters, folded, are spaces.
pub fn name_refused(name: &str) -> bool {
    let folded = fold_name(name);
    let mut c = folded.chars().chain(std::iter::repeat(' '));
    (0..3).all(|_| c.next() == Some(' '))
}

/// A rating on the profile screen (characters 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Rating {
    Constitution,
    LifeForce,
    Wisdom,
    Power,
    Strength,
    Size,
    Dexterity,
    ArmorSkill,
    Missile,
    Dodge,
    Melee,
    Parry,
    Gold,
    Speed,
}

/// The mark beside a rating on the profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mark {
    /// Lower than average.
    Down,
    /// Higher than average.
    Up,
}

impl Rating {
    /// Below the first the down mark, at or above the second the up mark.
    pub fn bounds(self) -> (u16, u16) {
        use Rating::*;
        match self {
            Constitution | LifeForce | Wisdom | Power | Strength | Size | Dexterity => (6, 15),
            ArmorSkill => (0, 25),
            Missile => (10, 35),
            Dodge => (10, 60),
            Melee => (15, 45),
            Parry => (10, 40),
            Gold => (0, 250),
            Speed => (6, 8),
        }
    }
}

/// The profile's mark for a rating of `value` (characters 4.1).
pub fn mark(rating: Rating, value: u16) -> Option<Mark> {
    let (low, high) = rating.bounds();
    if value < low {
        Some(Mark::Down)
    } else if value >= high {
        Some(Mark::Up)
    } else {
        None
    }
}

/// A waiting RETIRE (characters 7.1): who, and the offer's answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Retirement {
    pub player: usize,
    /// Saved for later adventures (7.3), else destroyed (7.4, 7.5).
    pub save: bool,
}

impl Game {
    /// RETIRE (7.1): only recorded, no move used, nothing drawn; acted on
    /// at the end of the round. Only one can wait: another asker is told
    /// someone will retire first and nothing is recorded.
    pub(crate) fn request_retire(&mut self, save: bool, ev: &mut Vec<Event>) {
        let player = self.current;
        if self.retire.is_some() {
            ev.push(Event::RetireRefused { player });
            return;
        }
        self.retire = Some(Retirement { player, save });
        ev.push(Event::RetireRequested { player });
    }

    /// The round's retirement (7.1), before its add requests: a player
    /// dead by now is offered nothing. Saved (7.3): the whole record, Life
    /// Force raised to Constitution, goes to the front end (the engine has
    /// no roster yet). Destroyed: on the map nothing is put down (7.4); in a
    /// room the gold as one pile, then the possessions that go down (held
    /// weapon and worn armor among them, `goes_down_on_destroy`) in thing
    /// number order, each on top of the last (7.5). Either way only the slot is freed (7.6), and the
    /// reversals due for it are dropped (7.3). A Destroy, on the map or in
    /// a room, is no save point (7.4, 7.5).
    /// True when a Save was taken: the round's end then saves the game
    /// (7.3, creatures 2.2.7.3).
    pub(crate) fn take_retirement(&mut self, ev: &mut Vec<Event>) -> bool {
        let Some(Retirement { player, save }) = self.retire.take() else {
            return false;
        };
        if !self.players.get(player).is_some_and(|p| p.alive) {
            return false;
        }
        if self.store.as_ref().is_some_and(|s| s.player == player) {
            self.leave_store(ev);
        }
        let who = Who::Player { player };
        let at = self.players[player].at;
        let mut saved = None;
        let mut left = Vec::new();
        if save {
            let mut record = self.players[player].clone();
            record.life_force = record.life_force.max(record.constitution);
            saved = Some(Box::new(record));
        } else if let Location::Room { region, room, x, y } = at {
            let p = &mut self.players[player];
            let mut things = std::mem::take(&mut p.possessions);
            let gold = std::mem::take(&mut p.gold);
            p.weapon = None;
            p.armor = None;
            if gold > 0 {
                self.put_gold(region, room, x, y, gold);
                left.push((GOLD, gold));
            }
            things.retain(|&t| self.goes_down_on_destroy(t));
            things.sort_unstable();
            for t in things {
                self.put_on_top(region, room, x, y, t, 1);
                left.push((t, 1));
            }
        }
        self.players[player].alive = false;
        self.reversals.retain(|r| r.who != who);
        self.forget_vitals(who);
        self.carried.retain(|&(q, _)| q != player);
        ev.push(Event::Retired { player, saved, left });
        // Encounters 7.2: retiring may leave its area empty.
        if let Location::Room { region: 0, room, .. } = at {
            self.close_if_empty(room, ev);
        }
        save
    }

    /// Whether a possession goes down on a Destroy in a room (7.5): never
    /// a spell, nor a weapon or armor only its owner may use, whatever it
    /// weighs (1, a death's test, magic 7.2); every other thing does,
    /// whoever else is in play and whatever its "ACS may add" count (2).
    fn goes_down_on_destroy(&self, thing: ThingId) -> bool {
        let Some(t) = self.thing(thing) else {
            return true;
        };
        !matches!(t.kind, ThingKind::MagicSpell { .. }) && !crate::magic::owner_only(&t.kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 3.3: ratings kept, things and readied gear dropped, gold at least
    /// 1,000 (1,500 stays 1,500, 300 becomes 1,000).
    #[test]
    fn a_stranger_keeps_its_ratings_and_brings_at_least_1000_gold() {
        let mut p = crate::Player {
            name: "T".into(),
            ..crate::Player::cleared()
        };
        p.strength = 17;
        p.life_force = 5;
        p.power = 31;
        p.possessions = vec![3, 5];
        p.weapon = Some(3);
        p.gold = 300;
        let s = from_another_adventure(&p, Some(4));
        assert_eq!((s.strength, s.life_force, s.power), (17, 5, Some(31)));
        assert!(s.possessions.is_empty());
        assert_eq!((s.gold, s.picture), (1_000, Some(4)));
        p.gold = 1_500;
        assert_eq!(from_another_adventure(&p, None).gold, 1_500);
    }

    /// 3.1: a roster entry comes back with everything it kept: ratings,
    /// each thing once per count, gold, readied gear, place and picture.
    #[test]
    fn a_roster_entry_keeps_its_things_place_and_gear() {
        use acs_core::model::{Alignment, Creature, Possession, Strategy};
        let c = Creature {
            number: 2,
            in_use: true,
            name: "U".into(),
            class: None,
            picture: Some(7),
            constitution: 20,
            life_force: 5,
            strength: 14,
            dexterity: 12,
            speed: 6,
            wisdom: 13,
            power: 20,
            size: 7,
            dodge: 29,
            parry: 25,
            armor_skill: 3,
            melee: 33,
            missile: 26,
            strategy: Strategy {
                alignment: Alignment::Thief,
                aggressive: false,
                brave: false,
            },
            magic_defense: MagicDefense::None,
            mimic: false,
            wealth: 346,
            weapon: 4,
            armor: 0,
            x: 5,
            y: 2,
            region: 3,
            room: 14,
            possessions: vec![
                Possession { thing: 4, count: 1 },
                Possession { thing: 16, count: 2 },
            ],
            unknown_0b: 0,
            last_turn_round: 0,
        };
        let p = from_roster(&c);
        assert_eq!((p.constitution, p.life_force, p.power, p.wisdom), (20, 5, 20, 13));
        assert_eq!(p.possessions, [4, 16, 16]);
        assert_eq!((p.gold, p.weapon, p.armor), (346, Some(4), None));
        assert_eq!(
            p.at,
            Location::Room {
                region: 3,
                room: 14,
                x: 5,
                y: 2
            }
        );
        assert_eq!((p.picture, p.alive), (Some(7), true));
        let map = from_roster(&Creature { region: 0, ..c });
        assert_eq!(map.at, Location::World { x: 5, y: 2 });
    }

    #[test]
    fn derived_ratings_follow_the_traits() {
        let mut rng = Rng::new(3);
        for _ in 0..2000 {
            let p = PlayerSetup::roll("T", &mut rng);
            for t in [p.strength, p.constitution, p.dexterity, p.wisdom, p.size] {
                assert!((3..=18).contains(&t));
            }
            assert_eq!(p.speed, 5 + p.dexterity / 7);
            assert_eq!(p.dodge, (3 * p.dexterity).wrapping_sub(p.size));
            assert_eq!(p.parry, p.dexterity + p.wisdom);
            assert_eq!(p.melee, p.size + p.strength + p.dexterity);
            assert_eq!(p.missile, p.dexterity + p.strength);
            assert_eq!((p.armor_skill, p.gold, p.life_force), (0, 0, p.constitution));
            assert_eq!(p.power, Some(p.wisdom));
            assert_eq!(p.magic_defense != MagicDefense::None, p.wisdom >= 16);
            assert!(p.possessions.is_empty());
        }
    }

    #[test]
    fn draws_in_order_with_a_second_chance() {
        // Replays the draws by hand: 15 dice, the 0..=2 roll, and on a 0
        // twelve more; the second chance comes about 1 time in 3.
        let mut seconds = 0;
        for seed in 0..3000 {
            let mut a = Rng::new(seed);
            let p = PlayerSetup::roll("T", &mut a);
            let mut b = Rng::new(seed);
            let mut t: Vec<u8> = (0..5).map(|_| three_dice(&mut b)).collect();
            if b.below(3) == 0 {
                seconds += 1;
                for v in &mut t[..4] {
                    *v = (*v).max(three_dice(&mut b));
                }
            }
            assert_eq!(t, [p.strength, p.constitution, p.dexterity, p.wisdom, p.size]);
            assert_eq!(a, b, "seed {seed}: same number of draws");
        }
        assert!((850..1150).contains(&seconds), "{seconds}");
    }

    #[test]
    fn names_fold_to_the_c64_alphabet() {
        assert_eq!(fold_name("Sir Bob 2nd!"), "SIR BOB 2ND.");
        assert_eq!(fold_name("x0/y\"z@"), "XO-Y'Z");
        assert_eq!(fold_name("a*b[c]d"), "AXB C D");
        // 5.3: every row of the fold table, no key refused.
        for (key, kept) in [
            ('0', 'O'),
            ('!', '.'),
            ('?', '.'),
            ('.', '.'),
            ('+', '-'),
            ('-', '-'),
            ('/', '-'),
            (':', '-'),
            (';', '-'),
            ('=', '-'),
            (',', ','),
            ('\'', '\''),
            ('"', '\''),
            ('*', 'X'),
            ('&', 'A'),
            ('\u{a3}', 'L'),
            ('\u{2191}', 'I'),
            ('\u{2190}', 'K'),
            ('@', ' '),
            ('#', ' '),
            ('$', ' '),
            ('%', ' '),
            ('(', ' '),
            (')', ' '),
            ('<', ' '),
            ('>', ' '),
            ('[', ' '),
            (']', ' '),
            (' ', ' '),
            ('q', 'Q'),
            ('Q', 'Q'),
            ('7', '7'),
            // 5.7: no C64 key, a space.
            ('\u{e9}', ' '),
            ('~', ' '),
        ] {
            assert_eq!(fold_key(key), kept, "{key:?}");
        }
        // A folded key keeps its place; nothing after it shifts.
        assert_eq!(fold_name("A@B?C"), "A B.C");
        assert_eq!(fold_name("?:@\"X"), ".- 'X");
    }

    #[test]
    fn the_cursor_wraps_past_15() {
        // 5.4: A to Q gives PQCDEFGHIJKLMNO.
        assert_eq!(fold_name("abcdefghijklmnopq"), "PQCDEFGHIJKLMNO");
        assert_eq!(fold_name("ABCDEFGHIJKLMNO"), "ABCDEFGHIJKLMNO");
        // 21 keys: the 16th to 21st replace the 1st to 6th.
        assert_eq!(fold_name("ABCDEFGHIJKLMNOPQRSTU"), "PQRSTUGHIJKLMNO");
        // Past 30 it goes round again.
        assert_eq!(fold_name(&format!("{}AB", "X".repeat(30))), "ABXXXXXXXXXXXXX");
    }

    #[test]
    fn three_leading_spaces_are_asked_again() {
        // 5.6: as captured, 8 spaces then ABCDEFG.
        assert!(name_refused("        ABCDEFG"));
        assert!(name_refused(""));
        assert!(name_refused("  "));
        assert!(name_refused("   X"));
        // Folded first: @ and [ are spaces.
        assert!(name_refused("@[ X"));
        assert!(!name_refused("  X"));
        assert!(!name_refused("AB"));
        assert!(!name_refused("A   B"));
        // After the wrap the 16th to 18th keys are the first three.
        assert!(name_refused("ABCDEFGHIJKLMNO   "));
        assert!(!name_refused("               ABC"));
    }

    #[test]
    fn marks_as_captured() {
        // The retired player's profile and move-basic's T (characters 4.1).
        use Rating::*;
        for (r, v, m) in [
            (Constitution, 21, Some(Mark::Up)),
            (Missile, 31, None),
            (Melee, 30, None),
            (ArmorSkill, 0, None),
            (Dodge, 28, None),
            (Parry, 29, None),
            (Speed, 7, None),
            (Wisdom, 4, Some(Mark::Down)),
            (Power, 4, Some(Mark::Down)),
            (LifeForce, 17, Some(Mark::Up)),
            (Strength, 12, None),
            (Speed, 6, None),
            (Speed, 5, Some(Mark::Down)),
            (Gold, 0, None),
            (Gold, 250, Some(Mark::Up)),
        ] {
            assert_eq!(mark(r, v), m, "{r:?} {v}");
        }
    }
}
