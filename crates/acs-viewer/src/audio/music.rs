// SPDX-License-Identifier: GPL-3.0-or-later
//
// The scores below are compositions of the ACSVM project, also under
// CC BY-SA 4.0 (docs/content.md 5). Nothing is taken from the original's
// music.

//! The 30 Play Music cues as scores and recipes (content 5): what each
//! plays. `synth.rs` renders them.

/// An instrument (content 5.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Inst {
    /// A quarter-width square: the lead.
    Pulse,
    Square,
    Triangle,
    Sine,
    /// Two partials, decaying.
    Bell,
    /// `k`, `s` and `h`: kick, snare, hat.
    Drums,
}

/// A tune: sixteenths at `bpm`, voices of one length.
pub struct Score {
    pub bpm: f32,
    pub voices: &'static [(Inst, &'static str)],
}

/// What a cue is made of.
pub enum Cue {
    Score(Score),
    /// A pitch gliding from one frequency to another over `secs`.
    Sweep {
        inst: Inst,
        from: f32,
        to: f32,
        secs: f32,
    },
    /// A noise burst, bright to dull; `boom` adds a falling low thump.
    Noise {
        secs: f32,
        boom: bool,
    },
}

const fn score(bpm: f32, voices: &'static [(Inst, &'static str)]) -> Cue {
    Cue::Score(Score { bpm, voices })
}

use Inst::*;

/// Cue `n` (1..=30), after the brief in content 5.1.
pub fn cue(n: u8) -> Cue {
    match n {
        // --- Effects -----------------------------------------------------
        // 1 A cannon boom.
        1 => Cue::Noise {
            secs: 0.8,
            boom: true,
        },
        // 2 A ship's bell, rung hard.
        2 => score(300.0, &[(Bell, "c5:2 c5 c5 c5:6")]),
        // 3 A pop and a fizz.
        3 => score(300.0, &[(Drums, "k:1 h h h h h h h h h")]),
        // 4 A shrill bone whistle.
        4 => Cue::Sweep {
            inst: Sine,
            from: 1800.0,
            to: 2200.0,
            secs: 0.5,
        },
        // 5 A splash and a crack of timber.
        5 => Cue::Noise {
            secs: 0.6,
            boom: false,
        },
        // 6 A wooden crash.
        6 => score(260.0, &[(Drums, "k:1 s k s s k s:3")]),
        // 7 A heavy lock turning.
        7 => score(200.0, &[(Drums, "h:2 r:1 h:1 k:6")]),
        // 8 Coins counted onto a desk.
        8 => score(300.0, &[(Square, "e6:1 r b5 r e6 r b5 r g6:4")]),
        // 9 A squawk.
        9 => Cue::Sweep {
            inst: Pulse,
            from: 700.0,
            to: 1500.0,
            secs: 0.25,
        },
        // 10 A steam hiss.
        10 => Cue::Noise {
            secs: 0.9,
            boom: false,
        },
        // 11 A soft rising chime.
        11 => score(240.0, &[(Bell, "c6:2 e6 g6 c7:6")]),
        // 12 A low cold drone.
        12 => Cue::Sweep {
            inst: Sine,
            from: 70.0,
            to: 62.0,
            secs: 0.95,
        },

        // --- Themes ------------------------------------------------------
        // 13 A bright sea-shanty opening, once: G major.
        13 => score(
            112.0,
            &[
                (Pulse, "d5:4 g5:2 g5 g5:4 b5 | a5:2 g5 f#5 e5 d5:8 | e5:4 c5 d5 b4 | a4:4 d5 g4:8"),
                (Triangle, "g3:8 d3 | d3 a2 | c3 g2 | d3 g2"),
                (Drums, "k:4 s k s | k s k s | k s k s | k s k s"),
            ],
        ),
        // 14 An easy harbour tune (loops): G major, unhurried.
        14 => score(
            100.0,
            &[
                (
                    Triangle,
                    "g4:4 b4 d5 b4 | c5 e5 d5:8 | b4:4 d5 g5 f#5 | e5 d5 b4:8 | \
                     c5:4 e5 g5 e5 | d5 b4 a4:8 | b4:4 a4 g4 a4 | g4:12 r:4",
                ),
                (Sine, "g3:8 d3 | c3 g2 | g3 d3 | c3 g2 | c3 g2 | d3 a2 | d3 d3 | g2:12 r:4"),
                (Drums, "k:8 h | k h | k h | k h | k h | k h | k h | k:12 r:4"),
            ],
        ),
        // 15 Sailing, steady (loops): D dorian, a rolling sea tune.
        15 => score(
            104.0,
            &[
                (
                    Pulse,
                    "d5:4 a4:2 d5 e5:4 f5 | e5:2 d5 c5:4 a4:8 | c5:4 g4:2 c5 d5:4 e5 | d5:2 c5 a4:4 d5:8 | \
                     f5:4 e5:2 f5 g5:4 a5 | g5:2 f5 e5:4 c5:8 | d5:4 e5:2 f5 e5:4 c5 | d5:12 r:4",
                ),
                (
                    Triangle,
                    "d3:4 a3 d3 a3 | c3 g3 c3 g3 | c3 g3 c3 g3 | d3 a3 d3 a3 | \
                     f3 c4 f3 c4 | c3 g3 c3 g3 | g2 d3 g2 d3 | d3 a3 d3 r",
                ),
                (
                    Drums,
                    "k:4 h s h | k h s h | k h s h | k h s h | k h s h | k h s h | k h s h | k h s r",
                ),
            ],
        ),
        // 16 Dripping, slow, uneasy (loops): a drone and falling drops.
        16 => score(
            72.0,
            &[
                (Sine, "e2:16 | e2 | f2 | e2 | d2 | e2:12 r:4"),
                (Triangle, "b3:16 | c4 | a3 | b3 | a3 | g#3:12 r:4"),
                (
                    Bell,
                    "r:6 b5:2 r:8 | r:10 g5:2 r:4 | r:4 c6:2 r:10 | r:8 b5:2 r e5 r | r:12 f5:2 r | e5:4 r:12",
                ),
            ],
        ),
        // 17 Low and tense (loops), the finale's bed: A minor.
        17 => score(
            90.0,
            &[
                (
                    Sine,
                    "a1:2 a1 a1 a1 a1 a1 a1 a1 | a1 a1 a1 a1 a1 a1 a1 a1 | f1 f1 f1 f1 f1 f1 f1 f1 | f1 f1 f1 f1 f1 f1 f1 f1 | \
                     d2 d2 d2 d2 d2 d2 d2 d2 | d2 d2 d2 d2 d2 d2 d2 d2 | e1 e1 e1 e1 e1 e1 e1 e1 | e1 e1 e1 e1 e1:4 r",
                ),
                (
                    Square,
                    "r:12 e4:4 | r:12 e4:4 | r:12 f4:4 | r:12 c4:4 | r:12 d4:4 | r:12 f4:4 | r:8 e4:4 g#4 | a4:8 r",
                ),
                (Drums, "k:4 r k h | k r k h | k r k h | k r k h | k r k h | k r k h | k r k h | k r k r"),
            ],
        ),
        // 18 The mansion: stately, a little too sweet. D major.
        18 => score(
            88.0,
            &[
                (Square, "d5:4 d5:2 e5 f#5:4 d5 | a5:6 g5:2 f#5:4 e5 | d5:12 r:4"),
                (Bell, "d6:4 d6:2 e6 f#6:4 d6 | a6:6 g6:2 f#6:4 e6 | d6:12 r:4"),
                (Triangle, "d3:8 a2 | d3:4 c#3 b2 a2 | d2:12 r:4"),
                (Drums, "k:8 s | k s | k:12 r:4"),
            ],
        ),
        // 19 Gullwreck Cay: brassy and loud. C major.
        19 => score(
            116.0,
            &[
                (Pulse, "g4:3 g4:1 c5:4 e5 g5 | f5:3 e5:1 d5:4 g4:8 | a4:3 b4:1 c5:4 d5 e5 | c5:12 r:4"),
                (Square, "e4:3 e4:1 g4:4 c5 e5 | d5:3 c5:1 b4:4 d4:8 | f4:3 g4:1 a4:4 b4 c5 | g4:12 r:4"),
                (Triangle, "c3:4 g3 c3 g3 | g2 d3 g2 d3 | f2 c3 g2 b2 | c3:12 r:4"),
                (Drums, "k:4 s:2 s k:4 s | k:4 s:2 s k:4 s | k:4 s:2 s k:4 s | k:4 s k r"),
            ],
        ),
        // 20 The Saltmarsh: sly and plucked. A minor.
        20 => score(
            108.0,
            &[
                (
                    Square,
                    "a4:1 r c5 r e5:2 r d5:1 r c5 r b4:2 r | a4:1 r c5 r e5:2 r g5:1 r f5 r e5:2 r | \
                     f5:1 r e5 r d5:2 r e5:1 r c5 r b4:2 r | a4:2 r e4 r a4:4 r",
                ),
                (Triangle, "a2:2 r e2 r a2 r e2 r | a2 r e2 r a2 r e2 r | d2 r a2 r e2 r b2 r | a2:4 e2 a2 r"),
            ],
        ),
        // 21 Twin Hulls: two tunes arguing, then agreeing.
        21 => score(
            120.0,
            &[
                (Pulse, "c5:2 e5 g5 e5 c5:8 | r:16 | c5:2 e5 g5 c6 g5:8 | c5:12 r:4"),
                (Square, "r:16 | a4:2 c#5 e5 c#5 a4:8 | a4:2 c#5 e5 a5 e5:8 | c5:12 r:4"),
                (Triangle, "c3:8 g2 | a2 e2 | c3 a2 | c3:12 r:4"),
                (Drums, "k:4 s k s | k s k s | k:2 k s k k s k s | k:4 s k r"),
            ],
        ),
        // 22 Fort Teal: a navy march gone ragged. C minor, limping.
        22 => score(
            104.0,
            &[
                (
                    Square,
                    "c5:3 c5:1 eb5:4 g5:3 r:1 f5:4 | eb5:3 d5:1 c5:4 r:2 g4 g4:4 | \
                     ab4:3 bb4:1 c5:4 d5:2 r eb5:4 | c5:6 b4:2 c5:4 r",
                ),
                (Triangle, "c3:4 g2 c3 g2 | c3 g2 c3 r | f2 c3 g2 d3 | c3:12 r:4"),
                (Drums, "k:4 s:2 s k:4 r | k:4 s:2 r k:4 s | k:4 r:2 s k:4 s:2 r | k:4 s k r"),
            ],
        ),
        // 23 Ama Reef: drums and voices, calm. Pentatonic.
        23 => score(
            92.0,
            &[
                (Sine, "e4:4 g4 a4:8 | g4:4 e4 d4:8 | e4:4 g4 b4 a4 | g4:4 e4 e4:4 r"),
                (Triangle, "c4:8 d4 | b3 a3 | c4 d4 | e4:12 r:4"),
                (Drums, "k:4 h:2 h k:4 h | k:4 h:2 h k:4 h | k:4 h:2 h k:4 h | k:4 h:2 h k:4 r"),
            ],
        ),
        // 24 The Constance: underwater, hollow.
        24 => score(
            66.0,
            &[(Sine, "c3:16 | ab2 | c3:12 r:4"), (Bell, "r:8 eb5 | r:4 c5:8 r:4 | g4:12 r:4")],
        ),
        // 25 The vault reveal: a single held sting.
        25 => score(
            60.0,
            &[(Square, "r:2 c#5:14"), (Bell, "c#4:16"), (Sine, "c#2:16")],
        ),
        // 26 The alarm's chase: fast, E minor.
        26 => score(
            160.0,
            &[
                (
                    Pulse,
                    "e5:1 d#5 e5 b4 e5 d#5 e5 b4 g5:2 f#5 e5 b4 | a5:1 g#5 a5 e5 a5 g#5 a5 e5 c6:2 b5 a5 e5 | \
                     b5:1 a#5 b5 f#5 b5 a#5 b5 f#5 d6:2 c#6 b5 f#5 | e6:4 b5 e5 r",
                ),
                (
                    Triangle,
                    "e2:2 e3 e2 e3 e2 e3 e2 e3 | a2 a3 a2 a3 a2 a3 a2 a3 | b2 b3 b2 b3 b2 b3 b2 b3 | e2:4 b2 e2 r",
                ),
                (Drums, "k:2 s k s k s k s | k s k s k s k s | k s k s k s k s | k:4 s k r"),
            ],
        ),
        // 27 Vail on the quarterdeck: the mansion tune in D minor.
        27 => score(
            76.0,
            &[
                (Square, "d5:4 d5:2 e5 f5:4 d5 | a5:6 g5:2 f5:4 e5 | d5:12 r:4"),
                (Triangle, "d3:8 a2 | d3:4 c3 bb2 a2 | d2:12 r:4"),
                (Drums, "k:8 s | k s | k:12 r:4"),
            ],
        ),
        // 28 Ending A: the intro tune, slower and free.
        28 => score(
            84.0,
            &[
                (Triangle, "d5:4 g5:2 g5 g5:4 b5 | a5:2 g5 f#5 e5 d5:8 | e5:4 c5 d5 b4 | a4:4 d5 g4:8"),
                (Sine, "g3:16 | d3 | c3:8 g2 | d3 g2"),
            ],
        ),
        // 29 Ending B: the mansion tune, played by you, alone.
        29 => score(
            96.0,
            &[
                (Triangle, "d5:4 d5:2 e5 f#5:4 d5 | a5:6 g5:2 f#5:4 e5 | d5:12 r:4"),
                (Sine, "d3:16 | a2 | d3:12 r:4"),
            ],
        ),
        // 30 A player's death: short and sad. A minor.
        30 => score(
            80.0,
            &[(Triangle, "e5:6 d5:2 c5:8 | b4:4 g#4 a4 r"), (Sine, "a3:16 | e3:12 r:4")],
        ),
        _ => score(120.0, &[]),
    }
}

/// One note of a line: a pitch (`None` for a rest; a drum letter as a
/// negative code) and a length in sixteenths.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Pitch {
    Rest,
    Freq(f32),
    Kick,
    Snare,
    Hat,
}

/// A line's notes (content 5.2), or what in it doesn't parse.
pub fn parse(line: &str) -> Result<Vec<(Pitch, u32)>, String> {
    let mut out = Vec::new();
    let mut last = 4u32;
    for tok in line.split_whitespace().filter(|t| *t != "|") {
        let (p, len) = match tok.split_once(':') {
            Some((p, l)) => (p, l.parse::<u32>().map_err(|_| format!("length in {tok:?}"))?),
            None => (tok, last),
        };
        if len == 0 {
            return Err(format!("zero length in {tok:?}"));
        }
        last = len;
        let pitch = match p {
            "r" => Pitch::Rest,
            "k" => Pitch::Kick,
            "s" => Pitch::Snare,
            "h" => Pitch::Hat,
            _ => Pitch::Freq(freq(p).ok_or_else(|| format!("pitch {tok:?}"))?),
        };
        out.push((pitch, len));
    }
    Ok(out)
}

/// `c#5`, `eb4`, `a4` to hertz (A4 = 440).
fn freq(p: &str) -> Option<f32> {
    let mut chars = p.chars();
    let pc: i32 = match chars.next()? {
        'c' => 0,
        'd' => 2,
        'e' => 4,
        'f' => 5,
        'g' => 7,
        'a' => 9,
        'b' => 11,
        _ => return None,
    };
    let rest: String = chars.collect();
    let (acc, oct) = match rest.chars().next()? {
        '#' => (1, &rest[1..]),
        'b' if rest.len() > 1 => (-1, &rest[1..]),
        _ => (0, rest.as_str()),
    };
    let oct: i32 = oct.parse().ok()?;
    let midi = 12 * (oct + 1) + pc + acc;
    Some(440.0 * 2f32.powf((midi - 69) as f32 / 12.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pitches_and_lengths_parse() {
        assert_eq!(freq("a4"), Some(440.0));
        assert!((freq("c4").unwrap() - 261.63).abs() < 0.01);
        assert!((freq("c#4").unwrap() - freq("db4").unwrap()).abs() < 1e-3);
        assert!((freq("bb3").unwrap() - 233.08).abs() < 0.01);
        let n = parse("c5:2 e5 | r:4 k").unwrap();
        assert_eq!(n.iter().map(|x| x.1).collect::<Vec<_>>(), [2, 2, 4, 4]);
        assert_eq!(n[3].0, Pitch::Kick);
        assert!(parse("x5:2").is_err());
        assert!(parse("c5:0").is_err());
    }

    /// Every voice of a score parses and adds up to the score's length,
    /// so no bar is miscounted.
    #[test]
    fn every_score_parses_and_its_voices_agree() {
        for n in 1..=30u8 {
            if let Cue::Score(s) = cue(n) {
                assert!(!s.voices.is_empty(), "cue {n}");
                let lens: Vec<u32> = s
                    .voices
                    .iter()
                    .map(|(_, line)| {
                        parse(line)
                            .unwrap_or_else(|e| panic!("cue {n}: {e}"))
                            .iter()
                            .map(|x| x.1)
                            .sum()
                    })
                    .collect();
                assert!(
                    lens.iter().all(|&l| l == lens[0]),
                    "cue {n}: voices of {lens:?} sixteenths"
                );
            }
        }
    }
}
