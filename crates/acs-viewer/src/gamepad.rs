// SPDX-License-Identifier: GPL-3.0-or-later

//! Gamepads (client 4): up to four pads, their buttons turned into the
//! keyboard's keys in the frame's raw input, so a pad works everywhere
//! the keyboard does. `Pads` holds the rules (slots, owners, the
//! direction's hold and repeat) and is tested without hardware;
//! `Gamepads` feeds it from gilrs.

use egui::{Event, Key, Modifiers, RawInput};

/// Pads kept, one per hot-seat player.
pub const SLOTS: usize = 4;
/// A cardinal direction goes once it has held this long (s).
const SETTLE: f64 = 0.05;
/// A held direction repeats after this (s)...
const DELAY: f64 = 0.35;
/// ...then this often (s).
const REPEAT: f64 = 0.18;
/// The stick past this is pushed.
const DEADZONE: f32 = 0.5;

/// The temp-data key that says a pad pressed Fire this frame.
pub fn fire_id() -> egui::Id {
    egui::Id::unique("gamepad-fire")
}

/// What a pad's button does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    South,
    East,
    West,
    LeftShoulder,
    RightShoulder,
}

/// A pad's directional state this frame: D-pad arrows and the left stick.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stick {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    /// Left stick, x right and y up, -1..1.
    pub x: f32,
    pub y: f32,
}

impl Stick {
    /// The key for the direction held, if any: arrows, and Home, PageUp,
    /// End and PageDown for the diagonals (play mode's keys).
    fn key(&self) -> Option<Key> {
        let mut dx = self.right as i8 - self.left as i8;
        let mut dy = self.down as i8 - self.up as i8;
        if dx == 0 && dy == 0 && self.x.hypot(self.y) > DEADZONE {
            // Eight sectors of 45 degrees.
            let a = self.y.atan2(self.x).to_degrees();
            let sector = ((a + 360.0 + 22.5) / 45.0) as i32 % 8;
            (dx, dy) = [
                (1, 0),
                (1, -1),
                (0, -1),
                (-1, -1),
                (-1, 0),
                (-1, 1),
                (0, 1),
                (1, 1),
            ][sector as usize];
        }
        Some(match (dx, dy) {
            (0, -1) => Key::ArrowUp,
            (0, 1) => Key::ArrowDown,
            (-1, 0) => Key::ArrowLeft,
            (1, 0) => Key::ArrowRight,
            (-1, -1) => Key::Home,
            (1, -1) => Key::PageUp,
            (-1, 1) => Key::End,
            (1, 1) => Key::PageDown,
            _ => return None,
        })
    }
}

fn diagonal(k: Key) -> bool {
    matches!(k, Key::Home | Key::PageUp | Key::End | Key::PageDown)
}

/// One slot's held direction.
#[derive(Clone, Copy, Debug, Default)]
struct Held {
    key: Option<Key>,
    since: f64,
    /// When it next goes; None until the first time.
    next: Option<f64>,
}

/// The pads' slots and held directions, whatever drives them.
pub struct Pads<Id> {
    slots: [Option<Id>; SLOTS],
    held: [Held; SLOTS],
}

impl<Id> Default for Pads<Id> {
    fn default() -> Self {
        Pads {
            slots: std::array::from_fn(|_| None),
            held: [Held::default(); SLOTS],
        }
    }
}

/// What the pads did this frame.
#[derive(Default, Debug, PartialEq)]
pub struct Out {
    pub keys: Vec<(Key, Modifiers)>,
    pub fire: bool,
}

impl<Id: PartialEq + Copy> Pads<Id> {
    pub fn any(&self) -> bool {
        self.slots.iter().any(Option::is_some)
    }

    /// A pad connected: the first free slot, if any.
    pub fn connect(&mut self, id: Id) {
        if self.slot(id).is_some() {
            return;
        }
        if let Some(i) = self.slots.iter().position(Option::is_none) {
            self.slots[i] = Some(id);
            self.held[i] = Held::default();
        }
    }

    pub fn disconnect(&mut self, id: Id) {
        if let Some(i) = self.slot(id) {
            self.slots[i] = None;
        }
    }

    fn slot(&self, id: Id) -> Option<usize> {
        self.slots.iter().position(|s| *s == Some(id))
    }

    /// Whether the pad in `slot` plays now: `player` is the current
    /// player's index on the board (None elsewhere). The player's own pad
    /// plays alone; a player with no pad is played by any.
    fn plays(&self, slot: usize, player: Option<usize>) -> bool {
        match player {
            Some(p) if self.slots.get(p).is_some_and(Option::is_some) => slot == p,
            _ => true,
        }
    }

    /// A button pressed on pad `id`.
    pub fn press(&self, id: Id, b: Button, player: Option<usize>, out: &mut Out) {
        let Some(slot) = self.slot(id) else { return };
        if !self.plays(slot, player) {
            return;
        }
        let none = Modifiers::NONE;
        match b {
            Button::South => out.keys.push((Key::Space, none)),
            Button::East => out.keys.push((Key::Escape, none)),
            Button::West => out.fire = true,
            Button::RightShoulder => out.keys.push((Key::Tab, none)),
            Button::LeftShoulder => out.keys.push((Key::Tab, Modifiers::SHIFT)),
        }
    }

    /// Pad `id`'s direction at time `now` (s): a key when it goes.
    pub fn stick(&mut self, id: Id, s: Stick, now: f64, player: Option<usize>, out: &mut Out) {
        let Some(slot) = self.slot(id) else { return };
        let key = s.key();
        let plays = self.plays(slot, player);
        let h = &mut self.held[slot];
        if key != h.key {
            *h = Held {
                key,
                since: now,
                next: None,
            };
        }
        let Some(k) = h.key else { return };
        let due = match h.next {
            None => diagonal(k) || now - h.since >= SETTLE,
            Some(t) => now >= t,
        };
        if due {
            h.next = Some(match h.next {
                None => now + DELAY,
                Some(t) => t.max(now - REPEAT) + REPEAT,
            });
            if plays {
                out.keys.push((k, Modifiers::NONE));
            }
        }
    }
}

impl Out {
    /// Into the frame's input: each key pressed and released.
    pub fn feed(&self, raw: &mut RawInput) {
        for &(key, modifiers) in &self.keys {
            for pressed in [true, false] {
                raw.events.push(Event::Key {
                    key,
                    physical_key: None,
                    pressed,
                    repeat: false,
                    modifiers,
                });
            }
        }
    }
}

/// The pads gilrs sees.
pub struct Gamepads {
    gilrs: Option<gilrs::Gilrs>,
    pads: Pads<gilrs::GamepadId>,
}

impl Default for Gamepads {
    fn default() -> Self {
        let gilrs = gilrs::Gilrs::new()
            .map_err(|e| log::warn!("no gamepads: {e}"))
            .ok();
        Gamepads {
            gilrs,
            pads: Pads::default(),
        }
    }
}

impl Gamepads {
    /// Reads the pads into `raw` before the frame; `player` as for
    /// `Pads::plays`. A pad takes its slot on its first press, not when it
    /// connects, so a device that never presses (a receiver or wheel
    /// gilrs lists as a pad) takes none. Keeps frames coming while a pad
    /// is in: gilrs does not wake the window.
    pub fn feed(&mut self, ctx: &egui::Context, raw: &mut RawInput, player: Option<usize>) {
        let Some(g) = &mut self.gilrs else { return };
        let mut out = Out::default();
        while let Some(gilrs::Event { id, event, .. }) = g.next_event() {
            use gilrs::{Button as B, EventType as E};
            match event {
                E::Disconnected => self.pads.disconnect(id),
                E::ButtonPressed(b, _) => {
                    self.pads.connect(id);
                    let b = match b {
                        B::South => Button::South,
                        B::East => Button::East,
                        B::West => Button::West,
                        B::LeftTrigger => Button::LeftShoulder,
                        B::RightTrigger => Button::RightShoulder,
                        _ => continue,
                    };
                    self.pads.press(id, b, player, &mut out);
                }
                _ => {}
            }
        }
        let now = raw.time.unwrap_or(0.0);
        for (id, pad) in g.gamepads() {
            use gilrs::{Axis, Button as B};
            let s = Stick {
                up: pad.is_pressed(B::DPadUp),
                down: pad.is_pressed(B::DPadDown),
                left: pad.is_pressed(B::DPadLeft),
                right: pad.is_pressed(B::DPadRight),
                x: pad.value(Axis::LeftStickX),
                y: pad.value(Axis::LeftStickY),
            };
            if s.key().is_some() {
                self.pads.connect(id);
            }
            self.pads.stick(id, s, now, player, &mut out);
        }
        out.feed(raw);
        if out.fire {
            ctx.data_mut(|d| d.insert_temp(fire_id(), true));
        }
        // Smoothly once a pad plays; slowly while one only waits for its
        // first press.
        let ms = if self.pads.any() {
            16
        } else if g.gamepads().next().is_some() {
            100
        } else {
            return;
        };
        ctx.request_repaint_after(std::time::Duration::from_millis(ms));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn up() -> Stick {
        Stick {
            up: true,
            ..Stick::default()
        }
    }

    fn keys(out: &Out) -> Vec<Key> {
        out.keys.iter().map(|k| k.0).collect()
    }

    #[test]
    fn the_stick_reads_eight_ways_past_the_deadzone() {
        let at = |x, y| {
            Stick {
                x,
                y,
                ..Stick::default()
            }
            .key()
        };
        assert_eq!(at(0.2, 0.3), None);
        assert_eq!(at(0.0, 0.9), Some(Key::ArrowUp));
        assert_eq!(at(0.0, -0.9), Some(Key::ArrowDown));
        assert_eq!(at(0.9, 0.0), Some(Key::ArrowRight));
        assert_eq!(at(-0.9, 0.0), Some(Key::ArrowLeft));
        assert_eq!(at(0.7, 0.7), Some(Key::PageUp));
        assert_eq!(at(-0.7, -0.7), Some(Key::End));
        let both = Stick {
            up: true,
            left: true,
            ..Stick::default()
        };
        assert_eq!(both.key(), Some(Key::Home));
    }

    #[test]
    fn a_direction_settles_then_repeats() {
        let mut p = Pads::<u8>::default();
        p.connect(7);
        let mut out = Out::default();
        let mut at = |t: f64, s: Stick, out: &mut Out| p.stick(7, s, t, None, out);
        at(0.0, up(), &mut out);
        at(0.03, up(), &mut out);
        assert!(out.keys.is_empty(), "not before it settles");
        at(0.06, up(), &mut out);
        assert_eq!(keys(&out), [Key::ArrowUp]);
        at(0.3, up(), &mut out);
        assert_eq!(out.keys.len(), 1, "no repeat before the delay");
        at(0.42, up(), &mut out);
        at(0.6, up(), &mut out);
        assert_eq!(out.keys.len(), 3);
        // Let go and press again: a fresh press.
        at(0.7, Stick::default(), &mut out);
        at(0.71, up(), &mut out);
        at(0.77, up(), &mut out);
        assert_eq!(out.keys.len(), 4);
    }

    #[test]
    fn two_arrows_make_one_diagonal_not_a_stray_step() {
        let mut p = Pads::<u8>::default();
        p.connect(1);
        let mut out = Out::default();
        p.stick(1, up(), 0.0, None, &mut out);
        let ne = Stick {
            up: true,
            right: true,
            ..Stick::default()
        };
        p.stick(1, ne, 0.02, None, &mut out);
        assert_eq!(keys(&out), [Key::PageUp]);
    }

    #[test]
    fn the_current_players_pad_plays_alone_else_any_pad() {
        let mut p = Pads::<u8>::default();
        p.connect(10);
        p.connect(20);
        let press = |p: &Pads<u8>, id, player| {
            let mut out = Out::default();
            p.press(id, Button::South, player, &mut out);
            !out.keys.is_empty()
        };
        // Player 1 (slot 1) has pad 20.
        assert!(press(&p, 20, Some(1)));
        assert!(!press(&p, 10, Some(1)));
        // Player 3 has no pad: either plays.
        assert!(press(&p, 10, Some(3)));
        assert!(press(&p, 20, Some(3)));
        // Off the board, any pad.
        assert!(press(&p, 10, None));
        // Unplugging frees the slot for the next pad.
        p.disconnect(10);
        p.connect(30);
        assert!(press(&p, 30, Some(0)));
        assert!(!press(&p, 20, Some(0)));
    }

    #[test]
    fn buttons_are_the_keyboards_keys() {
        let mut p = Pads::<u8>::default();
        p.connect(1);
        let mut out = Out::default();
        for b in [
            Button::South,
            Button::East,
            Button::LeftShoulder,
            Button::RightShoulder,
            Button::West,
        ] {
            p.press(1, b, None, &mut out);
        }
        assert_eq!(
            out.keys,
            [
                (Key::Space, Modifiers::NONE),
                (Key::Escape, Modifiers::NONE),
                (Key::Tab, Modifiers::SHIFT),
                (Key::Tab, Modifiers::NONE),
            ]
        );
        assert!(out.fire);
        let mut raw = RawInput::default();
        out.feed(&mut raw);
        assert_eq!(raw.events.len(), 8, "each key pressed and released");
    }
}
