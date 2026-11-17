// SPDX-License-Identifier: GPL-3.0-or-later

//! Placeholder cues made in code (client 6): ours, nothing copied, until
//! Phase 6's finished set. Effects are short sounds, themes short tunes on
//! a pentatonic scale; each choice is its own, the same every time.

use super::cues::{is_theme, CUES};
use std::f32::consts::TAU;

/// Every cue at `rate` samples a second, cue `n` at index `n - 1`.
pub fn bank(rate: u32) -> Vec<Vec<f32>> {
    (1..=CUES).map(|n| cue(n, rate)).collect()
}

pub fn cue(n: u8, rate: u32) -> Vec<f32> {
    let mut out = if is_theme(n) {
        theme(n, rate as f32)
    } else {
        effect(n, rate as f32)
    };
    normalise(&mut out, 0.5);
    out
}

/// A small generator so each cue has its own shape.
struct Seed(u32);

impl Seed {
    fn new(n: u8) -> Self {
        Seed(0x9E37_79B9 ^ (n as u32).wrapping_mul(0x85EB_CA6B))
    }
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
    /// 0..1.
    fn unit(&mut self) -> f32 {
        (self.next() >> 8) as f32 / (1u32 << 24) as f32
    }
}

/// Rises in `attack` seconds, falls away by the end, and is 0 at both ends.
fn envelope(t: f32, len: f32, attack: f32) -> f32 {
    let rise = (t / attack).min(1.0);
    let fall = (1.0 - t / len).max(0.0);
    rise * fall * fall
}

/// Effects 1-12: a sweep up, a sweep down, a run of blips or a noise
/// burst, by `n`'s place in fours; pitch and length vary with `n`.
fn effect(n: u8, rate: f32) -> Vec<f32> {
    let mut seed = Seed::new(n);
    let len = 0.18 + 0.06 * (n % 6) as f32;
    let base = 220.0 * 2f32.powf((n as f32 * 5.0 % 24.0) / 12.0);
    let count = (len * rate) as usize;
    let mut phase = 0.0f32;
    let mut noise = 0.0f32;
    (0..count)
        .map(|i| {
            let t = i as f32 / rate;
            let x = t / len;
            let env = envelope(t, len, 0.004);
            match n % 4 {
                0 => {
                    phase += TAU * base * (1.0 + 2.0 * x) / rate;
                    phase.sin() * env
                }
                1 => {
                    phase += TAU * base * 2.0 * (1.0 - 0.7 * x) / rate;
                    phase.sin() * env
                }
                2 => {
                    // Three blips up a fifth each.
                    let step = (x * 3.0).floor().min(2.0);
                    phase += TAU * base * 1.5f32.powf(step) / rate;
                    square(phase) * 0.6 * env
                }
                _ => {
                    // Noise, smoothed less as it goes on.
                    let k = 0.05 + 0.4 * x;
                    noise += k * (seed.unit() * 2.0 - 1.0 - noise);
                    noise * 1.5 * env
                }
            }
        })
        .collect()
}

/// Themes 13-30: two bars of eighths on a pentatonic scale, a walking
/// tune over a root-and-fifth bass, tempo and key by `n`. Every note
/// fades to 0 before the next, so the end is silent and a loop is clean.
fn theme(n: u8, rate: f32) -> Vec<f32> {
    const SCALE: [i32; 5] = [0, 2, 4, 7, 9];
    let mut seed = Seed::new(n);
    let bpm = 112.0 + 4.0 * (n % 8) as f32;
    let eighth = 30.0 / bpm;
    let key = 196.0 * 2f32.powf((n % 7) as f32 / 12.0);
    let notes = 16;
    let note_len = (eighth * rate) as usize;
    let mut out = vec![0.0f32; note_len * notes];
    let mut degree = (seed.next() % 5) as i32;
    for k in 0..notes {
        // A step or two up or down; now and then a rest; home at the end.
        let r = seed.next() % 8;
        degree = (degree + [-2, -1, -1, 0, 1, 1, 2, 0][r as usize]).clamp(-3, 9);
        if k == notes - 1 {
            degree = 0;
        }
        let rest = k != notes - 1 && seed.next().is_multiple_of(7);
        let semis = SCALE[degree.rem_euclid(5) as usize] + 12 * degree.div_euclid(5);
        let melody = key * 2f32.powf(semis as f32 / 12.0);
        let bass = key / 2.0 * if (k / 4) % 2 == 0 { 1.0 } else { 1.5 };
        let len = eighth * 0.92;
        for i in 0..note_len {
            let t = i as f32 / rate;
            let env = envelope(t, len, 0.006);
            let mut v = 0.0;
            if !rest {
                v += triangle(TAU * melody * t) * env;
            }
            if k % 2 == 0 {
                // The bass lasts two eighths, held in its own envelope.
                let tb = t;
                v += 0.45 * (TAU * bass * tb).sin() * envelope(tb, len, 0.01);
            }
            out[k * note_len + i] += v;
        }
    }
    out
}

fn square(phase: f32) -> f32 {
    if phase.rem_euclid(TAU) < TAU / 2.0 {
        1.0
    } else {
        -1.0
    }
}

fn triangle(phase: f32) -> f32 {
    let x = phase.rem_euclid(TAU) / TAU;
    4.0 * (x - (x + 0.5).floor()).abs() - 1.0
}

/// Scales so the loudest sample is `peak`.
fn normalise(buf: &mut [f32], peak: f32) {
    let max = buf.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    if max > 0.0 {
        buf.iter_mut().for_each(|v| *v *= peak / max);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_cue_is_its_own_finite_and_within_range() {
        let b = bank(22_050);
        assert_eq!(b.len(), CUES as usize);
        for (i, c) in b.iter().enumerate() {
            let n = i + 1;
            assert!(!c.is_empty(), "cue {n}");
            assert!(
                c.iter().all(|v| v.is_finite() && v.abs() <= 0.5 + 1e-6),
                "cue {n}"
            );
            // Effects are under a second, themes a few seconds.
            let secs = c.len() as f32 / 22_050.0;
            if is_theme(n as u8) {
                assert!((3.0..10.0).contains(&secs), "cue {n}: {secs}");
            } else {
                assert!(secs < 1.0, "cue {n}: {secs}");
            }
        }
        for i in 0..b.len() {
            for j in i + 1..b.len() {
                assert_ne!(b[i], b[j], "cues {} and {}", i + 1, j + 1);
            }
        }
        assert_eq!(cue(20, 22_050), b[19], "the same every time");
    }

    #[test]
    fn cues_start_and_end_silent_so_loops_dont_click() {
        for c in bank(22_050) {
            assert!(c[0].abs() < 0.01);
            assert!(c[c.len() - 1].abs() < 0.01);
        }
    }
}
