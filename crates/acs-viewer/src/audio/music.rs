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

/// Cue `n` (1..=30).
pub fn cue(n: u8) -> Cue {
    match n {
        // --- Effects -----------------------------------------------------
        1 => score(240.0, &[(Bell, "c6:2 g6:6")]),
        2 => score(200.0, &[(Pulse, "g5:1 c6:5"), (Bell, "g5:1 c6:5")]),
        3 => score(280.0, &[(Square, "b5:1 e6:5")]),
        4 => Cue::Sweep {
            inst: Pulse,
            from: 110.0,
            to: 70.0,
            secs: 0.6,
        },
        5 => Cue::Sweep {
            inst: Sine,
            from: 120.0,
            to: 40.0,
            secs: 0.25,
        },
        6 => Cue::Noise {
            secs: 0.5,
            boom: false,
        },
        7 => Cue::Sweep {
            inst: Square,
            from: 300.0,
            to: 1800.0,
            secs: 0.3,
        },
        8 => Cue::Sweep {
            inst: Pulse,
            from: 600.0,
            to: 150.0,
            secs: 0.6,
        },
        9 => score(240.0, &[(Square, "a5:2 e5 a5 e5")]),
        10 => score(240.0, &[(Bell, "c4:14")]),
        11 => Cue::Noise {
            secs: 0.8,
            boom: true,
        },
        12 => score(300.0, &[(Drums, "s:1 s s s s s s s s s k:4")]),

        // --- Themes ------------------------------------------------------
        // 13 Victory fanfare.
        13 => score(
            120.0,
            &[
                (Pulse, "c5:2 c5 c5 e5:6 r:2 g5 g5 g5 c6:12 r:4"),
                (Triangle, "c3:6 r:2 e3:4 r:2 g3:6 c3:12 r:4"),
                (Drums, "s:2 s s k:6 r:2 s s s k:12 r:4"),
            ],
        ),
        // 14 Endless voyage (loops): D dorian, a rolling sea tune.
        14 => score(
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
        // 15 Endless intrigue (loops): A minor, a creeping bass.
        15 => score(
            96.0,
            &[
                (
                    Square,
                    "r:8 e5:2 r c5 r | d5:6 c5:2 b4:8 | r:8 e5:2 r g5 r | f5:6 e5:2 d5:8 | \
                     r:4 c5:2 d5 e5:4 f5 | e5:8 c5 | b4:4 d5 g#5:8 | a5:4 e5 a4 r",
                ),
                (
                    Sine,
                    "a2:2 r a2 c3 e3 r d3 c3 | a2 r a2 c3 e3 r d3 c3 | a2 r a2 c3 e3 r d3 c3 | a2 r a2 c3 e3 r d3 c3 | \
                     f2 r f2 a2 c3 r b2 a2 | f2 r f2 a2 c3 r b2 a2 | e2 r e2 g#2 b2 r d3 b2 | a2 r a2 c3 e3 r:6",
                ),
                (
                    Drums,
                    "h:2 r h r h r h r | h r h r h r h r | h r h r h r h r | h r h r h r h r | \
                     h r h r h r h r | h r h r h r h r | h r h r h r h r | h r h r h r:6",
                ),
            ],
        ),
        // 16 Endless stars (loops): arpeggios over slow chords.
        16 => score(
            120.0,
            &[
                (
                    Pulse,
                    "c5:1 g5 d6 g5 c5 g5 d6 g5 c5 g5 d6 g5 c5 g5 d6 g5 | \
                     c5 g5 d6 g5 c5 g5 d6 g5 c5 g5 d6 g5 c5 g5 d6 g5 | \
                     ab4 eb5 bb5 eb5 ab4 eb5 bb5 eb5 ab4 eb5 bb5 eb5 ab4 eb5 bb5 eb5 | \
                     ab4 eb5 bb5 eb5 ab4 eb5 bb5 eb5 ab4 eb5 bb5 eb5 ab4 eb5 bb5 eb5 | \
                     bb4 f5 c6 f5 bb4 f5 c6 f5 bb4 f5 c6 f5 bb4 f5 c6 f5 | \
                     bb4 f5 c6 f5 bb4 f5 c6 f5 bb4 f5 c6 f5 bb4 f5 c6 f5 | \
                     c5 g5 d6 g5 c5 g5 d6 g5 c5 g5 d6 g5 c5 g5 d6 g5 | \
                     c5 g5 d6 g5 c5 g5 d6 g5 c5 g5 d6 g5 r:4",
                ),
                (Sine, "c4:16 | e4 | ab3 | c4 | bb3 | d4 | g3 | c4:12 r:4"),
                (
                    Bell,
                    "r:16 | g5:4 e5 d5:8 | r:16 | eb5:4 c5 bb4:8 | r:16 | f5:4 d5 c5:8 | r:8 g5 | c6:12 r:4",
                ),
            ],
        ),
        // 17 Endless danger (loops): E minor, driving eighths.
        17 => score(
            140.0,
            &[
                (
                    Pulse,
                    "e5:4 g5 b5 a5:2 g5 | f#5:4 e5 b4:8 | c5:4 e5 g5 f#5:2 e5 | d5:4 e5 c5:8 | \
                     d5:4 f#5 a5 g5:2 f#5 | e5:4 d5 a4:8 | b4:2 c5 d#5:4 f#5 a5 | e5:4 b4:2 e5 e5:4 r",
                ),
                (
                    Square,
                    "e2:2 e2 e3 e2 e2 e3 e2 d3 | e2 e2 e3 e2 e2 e3 e2 d3 | c2 c2 c3 c2 c2 c3 c2 b2 | c2 c2 c3 c2 c2 c3 c2 b2 | \
                     d2 d2 d3 d2 d2 d3 d2 a2 | d2 d2 d3 d2 d2 d3 d2 a2 | b1 b1 b2 b1 b1 b2 b1 d#3 | e2 e2 e3 e2 e2:4 r",
                ),
                (
                    Drums,
                    "k:2 h s h k k s h | k h s h k k s h | k h s h k k s h | k h s h k k s h | \
                     k h s h k k s h | k h s h k k s h | k h s h k k s h | k h s h k:4 r",
                ),
            ],
        ),
        // 18 Sea shanty: G major.
        18 => score(
            112.0,
            &[
                (Pulse, "d5:4 g5:2 g5 g5:4 b5 | a5:2 g5 f#5 e5 d5:8 | e5:4 c5 d5 b4 | a4:4 d5 g4:8"),
                (Triangle, "g3:8 d3 | d3 a2 | c3 g2 | d3 g2"),
                (Drums, "k:4 s k s | k s k s | k s k s | k s k s"),
            ],
        ),
        // 19 Lament: A minor, slow.
        19 => score(
            80.0,
            &[
                (Triangle, "e5:6 d5:2 c5:8 | b4:4 c5 a4 g#4 | a4:12 r:4"),
                (Sine, "a3:16 | e3 | a2:12 r:4"),
            ],
        ),
        // 20 Triumph march: C major.
        20 => score(
            116.0,
            &[
                (Pulse, "g4:3 g4:1 c5:4 e5 g5 | f5:3 e5:1 d5:4 g4:8 | a4:3 b4:1 c5:4 d5 e5 | c5:12 r:4"),
                (Triangle, "c3:4 g3 c3 g3 | g2 d3 g2 d3 | f2 c3 g2 b2 | c3:12 r:4"),
                (Drums, "k:4 s:2 s k:4 s | k:4 s:2 s k:4 s | k:4 s:2 s k:4 s | k:4 s k r"),
            ],
        ),
        // 21 Mystery sting.
        21 => score(
            90.0,
            &[(Bell, "e5:2 r f5 r b5:8 | a#4:12 r:4"), (Sine, "e2:16 | f2:12 r:4")],
        ),
        // 22 Lullaby: F major.
        22 => score(
            84.0,
            &[
                (Triangle, "c5:4 a4 f4:8 | g4:4 a4 c5 a4 | f4:12 r:4"),
                (Sine, "f3:16 | c3 | f2:12 r:4"),
            ],
        ),
        // 23 Tavern jig: D major.
        23 => score(
            140.0,
            &[
                (
                    Pulse,
                    "a4:2 d5 d5 f#5 e5 d5 c#5 e5 | a5 f#5 e5 d5 c#5 a4 b4 c#5 | \
                     d5 f#5 a5 f#5 g5 e5 c#5 e5 | d5:4 a4 d5 r",
                ),
                (Triangle, "d3:4 a3 d3 a3 | a2 e3 a2 e3 | d3 a3 a2 e3 | d3 a2 d3 r"),
                (Drums, "k:2 h s h k h s h | k h s h k h s h | k h s h k h s h | k:4 s k r"),
            ],
        ),
        // 24 Dread.
        24 => score(
            60.0,
            &[(Sine, "c2:16 | c#2:12 r:4"), (Bell, "r:4 g4 r f#4 | c4:12 r:4")],
        ),
        // 25 Discovery: a climbing arpeggio.
        25 => score(
            100.0,
            &[
                (Pulse, "c5:1 e5 g5 c6 d5 f#5 a5 d6 e5 g#5 b5 e6 f#5 a#5 c#6 f#6 | g6:12 r:4"),
                (Bell, "r:16 | g5:12 r:4"),
                (Sine, "c3:4 d3 e3 f#3 | g3:12 r:4"),
            ],
        ),
        // 26 Royal court: D major, stately.
        26 => score(
            88.0,
            &[
                (Square, "d5:4 d5:2 e5 f#5:4 d5 | a5:6 g5:2 f#5:4 e5 | d5:12 r:4"),
                (Triangle, "d3:8 a2 | d3:4 c#3 b2 a2 | d2:12 r:4"),
                (Drums, "k:8 s | k s | k:12 r:4"),
            ],
        ),
        // 27 Chase: E minor, fast.
        27 => score(
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
        // 28 Farewell: C major, gentle.
        28 => score(
            76.0,
            &[
                (Triangle, "g5:4 e5 d5 c5 | a4 c5 d5:8 | c5:12 r:4"),
                (Sine, "c3:8 a2 | f2 g2 | c2:12 r:4"),
            ],
        ),
        // 29 Ghostly waltz: A minor, three beats to the bar.
        29 => score(
            132.0,
            &[
                (Bell, "a4:4 c5 e5 | a5:8 g#5:4 | f5:4 d5 b4 | e5:8 r:4"),
                (Sine, "a2:4 e3 e3 | a2 e3 e3 | d2 a2 a2 | e2:8 r:4"),
            ],
        ),
        // 30 Homecoming: C major, the finale.
        30 => score(
            100.0,
            &[
                (Pulse, "c5:4 e5 g5 e5 | f5 a5 g5:8 | e5:4 g5 c6 b5 | c6:12 r:4"),
                (Triangle, "c3:8 g2 | f2 c3 | a2 g2 | c3:12 r:4"),
                (Bell, "r:16 | r:8 e6 | r:16 | g6:12 r:4"),
                (Drums, "k:4 h s h | k h s h | k h s h | k s k r"),
            ],
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
