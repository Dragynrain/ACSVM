// SPDX-License-Identifier: GPL-3.0-or-later

//! The cues made in code (client 6, content 5): `music.rs` says what
//! each plays, this renders it. Each cue is the same every time.

use super::cues::CUES;
use super::music::{self, Cue, Inst, Pitch, Score};
use std::f32::consts::TAU;

/// Every cue at `rate` samples a second, cue `n` at index `n - 1`.
pub fn bank(rate: u32) -> Vec<Vec<f32>> {
    (1..=CUES).map(|n| cue(n, rate)).collect()
}

pub fn cue(n: u8, rate: u32) -> Vec<f32> {
    let rate = rate as f32;
    let mut out = match music::cue(n) {
        Cue::Score(s) => render(&s, rate, n),
        Cue::Sweep { inst, from, to, secs } => sweep(inst, from, to, secs, rate),
        Cue::Noise { secs, boom } => noise(secs, boom, rate, n),
    };
    normalise(&mut out, 0.5);
    out
}

/// A small generator for noise, seeded by the cue.
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
    /// -1..1.
    fn signed(&mut self) -> f32 {
        (self.next() >> 8) as f32 / (1u32 << 23) as f32 - 1.0
    }
}

/// A note's loudness at `t` of `len` seconds: a quick rise, a fall to a
/// held level, and a release that reaches 0 inside the note, so every
/// note (and so every cue) ends in silence.
fn envelope(t: f32, len: f32, inst: Inst) -> f32 {
    let (attack, hold, decay) = match inst {
        Inst::Bell => (0.003, 0.0, 0.35),
        Inst::Pulse | Inst::Square => (0.005, 0.6, 0.08),
        _ => (0.008, 0.75, 0.12),
    };
    let rise = (t / attack).min(1.0);
    let body = hold + (1.0 - hold) * (-t / decay).exp();
    let release = (len * 0.25).clamp(0.004, 0.06);
    let tail = ((len - t) / release).clamp(0.0, 1.0);
    rise * body * tail
}

/// One cycle's shape at `phase` (radians) for a pitched instrument.
fn wave(inst: Inst, phase: f32) -> f32 {
    let x = phase.rem_euclid(TAU) / TAU;
    match inst {
        Inst::Pulse => {
            if x < 0.25 {
                1.0
            } else {
                -0.33
            }
        }
        Inst::Square => {
            if x < 0.5 {
                1.0
            } else {
                -1.0
            }
        }
        Inst::Triangle => 4.0 * (x - (x + 0.5).floor()).abs() - 1.0,
        Inst::Sine | Inst::Drums => phase.sin(),
        // Two partials, the upper an inharmonic 2.76 times.
        Inst::Bell => phase.sin() + 0.4 * (phase * 2.76).sin(),
    }
}

/// How loud each instrument sits in the mix.
fn level(inst: Inst) -> f32 {
    match inst {
        Inst::Pulse | Inst::Square => 0.45,
        Inst::Triangle => 0.6,
        Inst::Sine => 0.55,
        Inst::Bell => 0.4,
        Inst::Drums => 0.5,
    }
}

/// A score: every voice from the start, summed.
fn render(s: &Score, rate: f32, n: u8) -> Vec<f32> {
    let sixteenth = 15.0 / s.bpm;
    let mut seed = Seed::new(n);
    let mut out: Vec<f32> = Vec::new();
    for (inst, line) in s.voices {
        // The scores are checked by music's tests; a bad line plays nothing.
        let Ok(notes) = music::parse(line) else { continue };
        let mut start = 0usize;
        for (pitch, len) in notes {
            let secs = len as f32 * sixteenth;
            let count = (secs * rate) as usize;
            if out.len() < start + count {
                out.resize(start + count, 0.0);
            }
            let mut phase = 0.0f32;
            let mut lp = 0.0f32;
            for i in 0..count {
                let t = i as f32 / rate;
                let v = match pitch {
                    Pitch::Rest => 0.0,
                    Pitch::Freq(f) => {
                        phase += TAU * f / rate;
                        wave(*inst, phase) * envelope(t, secs, *inst)
                    }
                    Pitch::Kick => {
                        // A sine falling from 150 to 45 Hz, gone in a tenth.
                        let f = 45.0 + 105.0 * (-t / 0.03).exp();
                        phase += TAU * f / rate;
                        phase.sin() * (-t / 0.1).exp() * envelope(t, secs, Inst::Sine)
                    }
                    Pitch::Snare => {
                        let w = seed.signed();
                        let hp = w - lp;
                        lp += 0.3 * (w - lp);
                        hp * 0.7 * (-t / 0.07).exp() * envelope(t, secs, Inst::Sine)
                    }
                    Pitch::Hat => {
                        let w = seed.signed();
                        let hp = w - lp;
                        lp += 0.6 * (w - lp);
                        hp * 0.35 * (-t / 0.025).exp() * envelope(t, secs, Inst::Sine)
                    }
                };
                out[start + i] += v * level(*inst);
            }
            start += count;
        }
    }
    out
}

/// A pitch gliding from `from` to `to` Hz (exponentially) over `secs`.
fn sweep(inst: Inst, from: f32, to: f32, secs: f32, rate: f32) -> Vec<f32> {
    let count = (secs * rate) as usize;
    let mut phase = 0.0f32;
    (0..count)
        .map(|i| {
            let t = i as f32 / rate;
            let f = from * (to / from).powf(t / secs);
            phase += TAU * f / rate;
            wave(inst, phase) * envelope(t, secs, Inst::Sine)
        })
        .collect()
}

/// Noise, bright to dull over `secs`; `boom` adds a falling thump.
fn noise(secs: f32, boom: bool, rate: f32, n: u8) -> Vec<f32> {
    let mut seed = Seed::new(n);
    let count = (secs * rate) as usize;
    let (mut lp, mut phase) = (0.0f32, 0.0f32);
    (0..count)
        .map(|i| {
            let t = i as f32 / rate;
            let x = t / secs;
            lp += (0.5 - 0.45 * x) * (seed.signed() - lp);
            let mut v = lp * (-t / (secs * 0.4)).exp();
            if boom {
                let f = 30.0 + 90.0 * (-t / 0.08).exp();
                phase += TAU * f / rate;
                v = 0.6 * v + phase.sin() * (-t / (secs * 0.35)).exp();
            }
            v * envelope(t, secs, Inst::Sine)
        })
        .collect()
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
    use super::super::cues::{is_theme, loops};
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
            // Effects under a second, themes 3 to 12 seconds, the
            // looping themes 13 to 21 before they repeat (content 5.1).
            let secs = c.len() as f32 / 22_050.0;
            let n = n as u8;
            if loops(n) {
                assert!((13.0..21.5).contains(&secs), "cue {n}: {secs}");
            } else if is_theme(n) {
                assert!((3.0..12.0).contains(&secs), "cue {n}: {secs}");
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
