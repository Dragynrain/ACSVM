// SPDX-License-Identifier: GPL-3.0-or-later

//! Model to bytes, where the model keeps bytes the engine reads
//! (`Thing::raw`). The inverse of the decoder's reading, for adventures
//! we build ourselves (docs/content.md 3.4).

use crate::model::{Access, Carry, CustomTrigger, PlacementChoice, SpellAction, ThingKind, WeaponStats};

/// What a thing's property bytes hold that its kind doesn't.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PropFlags {
    /// Bit 4 of byte 0 for a space, custom space, obstacle or custom
    /// obstacle: it goes once set off (movement 5.9). Carriable kinds keep
    /// the bit in their own field.
    pub once: bool,
    /// A custom obstacle's bump runs its spell (bit 6 of byte 1, movement 5.5).
    pub bump: bool,
    /// A custom obstacle takes its placement's parameter (bit 7 of byte 0,
    /// movement 5.8).
    pub placed_parameter: bool,
    /// A weapon's, armor's or magic item's byte 3: its index in the
    /// extended table, which the model doesn't keep.
    pub table_index: u8,
}

impl PropFlags {
    /// The flags `raw` holds, as the engine reads them.
    pub fn from_raw(raw: &[u8; 4]) -> PropFlags {
        PropFlags {
            once: raw[0] & 0x10 != 0,
            bump: raw[1] & 0x40 != 0,
            placed_parameter: raw[0] & 0x80 != 0,
            table_index: raw[3],
        }
    }
}

/// Why a kind can't be written as property bytes.
pub type EncodeError = String;

fn pic(picture: u16) -> u8 {
    (picture & 0x3F) as u8
}

fn action_code(a: &SpellAction) -> Result<u8, EncodeError> {
    use crate::model::SpellActionKind::*;
    Ok(match a.kind {
        Nothing => 0,
        KillAllButOwnerOf => 1,
        SummonOrBanish => 2,
        IncreaseMagicDefense => 3,
        DecreaseMagicDefense => 4,
        IncreaseStat => 5,
        DecreaseStat => 6,
        ChangePower => 7,
        ChangeLifeForce => 8,
        Give => 9,
        LongMessage => 10,
        PlayMusic => 11,
        RidRoomOf => 12,
        AddToRoom => 13,
        ActivateAll => 14,
        Invalid(c) => c & 0x0F,
    })
}

/// A number's low byte: a spell parameter or thing number past 255 (an
/// Extended adventure's) keeps only that in the property bytes, as a
/// picture past 63 keeps its low six bits; the model's field is the number,
/// and it is what the engine reads (content.md 3.4).
fn low(v: u16) -> u8 {
    (v & 0xFF) as u8
}

/// Weight multiplier index (bits 5-6: x1, x-1, x20, x-20), the weight
/// byte, the value byte and the value's x20 bit.
fn carry(c: &Carry) -> Result<(u8, u8, u8, bool), EncodeError> {
    let w = c.weight;
    let (mult, wb) = if (0..=255).contains(&w) {
        (0, w)
    } else if (-255..0).contains(&w) {
        (1, -w)
    } else if w > 0 && w % 20 == 0 && w / 20 <= 255 {
        (2, w / 20)
    } else if w < 0 && w % 20 == 0 && -w / 20 <= 255 {
        (3, -w / 20)
    } else {
        return Err(format!(
            "weight {w}: not -255..255 or a multiple of 20 up to 5,100 either way"
        ));
    };
    let v = c.value;
    let (vb, x20) = if (0..=255).contains(&v) {
        (v, false)
    } else if v > 0 && v % 20 == 0 && v / 20 <= 255 {
        (v / 20, true)
    } else {
        return Err(format!("value {v}: not 0..255 or a multiple of 20 up to 5,100"));
    };
    Ok((mult, wb as u8, vb as u8, x20))
}

fn carriable(code: u8, once: bool, c: &Carry, b3: u8) -> Result<[u8; 4], EncodeError> {
    let (mult, w, v, x20) = carry(c)?;
    Ok([code | (once as u8) << 4 | mult << 5 | (x20 as u8) << 7, w, v, b3])
}

/// Access rule (bits 6-7 of byte 1), the spell code for byte 2 and byte 3.
fn access(a: &Access) -> Result<(u8, u8, u8), EncodeError> {
    Ok(match a {
        Access::Anyone => (0, 0, 0),
        Access::InvokeSpell { action } => (1, action_code(action)?, low(action.parameter)),
        Access::OwnersOf { thing } => (2, 0, low(*thing)),
        Access::NonOwnersOf { thing } => (3, 0, low(*thing)),
    })
}

fn weapon_check(s: &WeaponStats, missile: bool, armor: bool) -> Result<(), EncodeError> {
    let max = if missile { 15 } else { 31 };
    if s.power > max {
        return Err(format!("power {} above {max}", s.power));
    }
    if missile && !matches!(s.range, Some(2..=9)) {
        return Err(format!("range {:?}: a missile weapon's is 2..9", s.range));
    }
    if s.break_chance > 15 {
        return Err(format!("break chance {} above 15", s.break_chance));
    }
    let base = if armor { -65 } else { -35 };
    let step = s.attack_adjustment as i16 - base;
    if step % 5 != 0 || !(0..=75).contains(&step) {
        return Err(format!(
            "attack adjustment {}: a multiple of 5 from {base} to {}",
            s.attack_adjustment,
            base + 75
        ));
    }
    Ok(())
}

/// A thing's four property bytes, as the decoder reads them back into
/// `kind` and `flags` (docs/format-notes.md, Things).
pub fn thing_props(kind: &ThingKind, flags: PropFlags) -> Result<[u8; 4], EncodeError> {
    let once = flags.once as u8;
    Ok(match kind {
        ThingKind::RoomFloor { picture } => [0, pic(*picture), 0, 0],
        ThingKind::Treasure {
            picture,
            disappears_when_dropped,
            carry,
        } => carriable(1, *disappears_when_dropped, carry, pic(*picture))?,
        ThingKind::MagicItem {
            disappears_after_use,
            carry,
            ..
        } => carriable(2, *disappears_after_use, carry, flags.table_index)?,
        ThingKind::MissileWeapon {
            disappears_after_use,
            carry,
            stats,
            ..
        } => {
            weapon_check(stats, true, false)?;
            carriable(3, *disappears_after_use, carry, flags.table_index)?
        }
        ThingKind::MeleeWeapon {
            disappears_after_use,
            carry,
            stats,
            ..
        } => {
            weapon_check(stats, false, false)?;
            carriable(4, *disappears_after_use, carry, flags.table_index)?
        }
        ThingKind::Armor {
            disappears_after_use,
            carry,
            stats,
            ..
        } => {
            weapon_check(stats, false, true)?;
            carriable(5, *disappears_after_use, carry, flags.table_index)?
        }
        ThingKind::MagicSpell {
            disappears_after_use,
            power,
            action,
        } => {
            if *power > 127 {
                return Err(format!("power {power} above 127"));
            }
            [
                6 | (*disappears_after_use as u8) << 4,
                *power,
                action_code(action)?,
                low(action.parameter),
            ]
        }
        ThingKind::Portal {
            picture,
            two_way,
            access: a,
            destroy_needed_thing,
        } => {
            let (rule, code, b3) = access(a)?;
            [
                7 | (*two_way as u8) << 5,
                pic(*picture) | rule << 6,
                code | (*destroy_needed_thing as u8) << 7,
                b3,
            ]
        }
        ThingKind::Space { picture, access: a } => {
            let (rule, code, b3) = access(a)?;
            [8 | once << 4, pic(*picture) | rule << 6, code, b3]
        }
        ThingKind::CustomSpace {
            picture,
            placement_choice,
            trigger,
            action,
            destroy_needed_thing,
        } => {
            let choice = matches!(placement_choice, PlacementChoice::SpellParameter) as u8;
            let trig = match trigger {
                CustomTrigger::WhenDroppedHere => 0,
                CustomTrigger::WhenMovedOnto => 1,
                CustomTrigger::OpenToOwnersOf => 2,
                CustomTrigger::OpenToNonOwnersOf => 3,
            };
            // Seven bits here, as the decoder reads them.
            let p = (action.parameter & 0x7F) as u8;
            [
                9 | once << 4 | choice << 7,
                pic(*picture) | trig << 6,
                action_code(action)? | (*destroy_needed_thing as u8) << 7,
                p,
            ]
        }
        ThingKind::Obstacle {
            picture,
            bump_invokes_spell,
            action,
        } => [
            10 | once << 4,
            pic(*picture) | (*bump_invokes_spell as u8) << 6,
            action_code(action)?,
            low(action.parameter),
        ],
        ThingKind::CustomObstacle { picture, action } => [
            11 | once << 4 | (flags.placed_parameter as u8) << 7,
            pic(*picture) | (flags.bump as u8) << 6,
            action_code(action)?,
            low(action.parameter),
        ],
        ThingKind::Store { picture } => [12, pic(*picture), 0, 0],
        ThingKind::Undefined => [15, 0, 0, 0],
        ThingKind::Unknown { code } => [code & 0x0F, 0, 0, 0],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SpellActionKind;

    #[test]
    fn weights_and_values_take_their_multipliers() {
        let c = |weight, value| Carry { weight, value };
        assert_eq!(carry(&c(12, 30)).unwrap(), (0, 12, 30, false));
        assert_eq!(carry(&c(-5, 0)).unwrap(), (1, 5, 0, false));
        assert_eq!(carry(&c(400, 1000)).unwrap(), (2, 20, 50, true));
        assert_eq!(carry(&c(-400, 0)).unwrap(), (3, 20, 0, false));
        assert!(carry(&c(301, 0)).is_err());
        assert!(carry(&c(0, 5120)).is_err());
    }

    #[test]
    fn custom_obstacle_flags_land_where_the_engine_reads_them() {
        let kind = ThingKind::CustomObstacle {
            picture: 70,
            action: SpellAction {
                kind: SpellActionKind::Give,
                parameter: 3,
            },
        };
        let flags = PropFlags {
            once: true,
            bump: true,
            placed_parameter: true,
            table_index: 0,
        };
        let raw = thing_props(&kind, flags).unwrap();
        assert_eq!(raw, [11 | 0x10 | 0x80, (70 & 0x3F) | 0x40, 9, 3]);
        let back = PropFlags::from_raw(&raw);
        assert_eq!((back.once, back.bump, back.placed_parameter), (true, true, true));
    }

    #[test]
    fn parameters_past_a_byte_keep_their_low_bits_on_every_kind() {
        // content.md 3.4: an Extended long message 300 builds on a magic
        // spell as on a magic item; the model keeps the number.
        let action = SpellAction {
            kind: SpellActionKind::LongMessage,
            parameter: 300,
        };
        let spell = ThingKind::MagicSpell {
            disappears_after_use: false,
            power: 5,
            action,
        };
        assert_eq!(
            thing_props(&spell, PropFlags::default()).unwrap()[3],
            300u16 as u8
        );
        let space = ThingKind::CustomSpace {
            picture: 1,
            placement_choice: PlacementChoice::Thing,
            trigger: CustomTrigger::WhenMovedOnto,
            action,
            destroy_needed_thing: false,
        };
        assert_eq!(
            thing_props(&space, PropFlags::default()).unwrap()[3],
            (300 & 0x7F) as u8
        );
    }
}
