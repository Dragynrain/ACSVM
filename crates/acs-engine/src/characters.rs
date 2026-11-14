// SPDX-License-Identifier: GPL-3.0-or-later

//! New characters, names and the profile's marks
//! (`docs/rules/characters.md`).

use crate::game::{PlayerSetup, NAME_MAX};
use crate::rng::Rng;
use acs_core::model::MagicDefense;
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
        }
    }
}

/// A name as the C64 keeps it (characters 5): uppercase, at most 15
/// characters of space, A to Z, 1 to 9 and `. , - '`; 0 becomes O, `! ?` a
/// period, `+ / : ; =` a hyphen, `"` an apostrophe; anything else is
/// dropped.
pub fn fold_name(name: &str) -> String {
    name.chars()
        .filter_map(|c| match c.to_ascii_uppercase() {
            c @ ('A'..='Z' | '1'..='9' | ' ' | '.' | ',' | '-' | '\'') => Some(c),
            '0' => Some('O'),
            '!' | '?' => Some('.'),
            '+' | '/' | ':' | ';' | '=' => Some('-'),
            '"' => Some('\''),
            _ => None,
        })
        .take(NAME_MAX)
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(fold_name("x0/y\"z*"), "XO-Y'Z");
        assert_eq!(fold_name("abcdefghijklmnopq"), "ABCDEFGHIJKLMNO");
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
