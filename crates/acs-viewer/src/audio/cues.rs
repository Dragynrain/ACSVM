// SPDX-License-Identifier: GPL-3.0-or-later

//! Which cue does what, and the one-cue-at-a-time player that the output
//! pulls samples from (client 6). No device here, so it is testable.

/// The Play Music choices (magic 3, action 11).
pub const CUES: u8 = 30;

/// Choices 13-30 are themes, 1-12 effects (magic 3.11).
pub fn is_theme(n: u8) -> bool {
    (13..=CUES).contains(&n)
}

/// Themes 14-17 loop until other music plays (magic 3.11).
pub fn loops(n: u8) -> bool {
    (14..=17).contains(&n)
}

/// One cue at a time: a new one replaces whatever plays (client 6).
pub struct Player {
    /// Cue `n`'s samples at `bank[n - 1]`.
    bank: Vec<Vec<f32>>,
    /// The cue playing and the next sample.
    now: Option<(u8, usize)>,
    /// 0..=1.
    pub volume: f32,
}

impl Player {
    pub fn new(bank: Vec<Vec<f32>>) -> Self {
        Player {
            bank,
            now: None,
            volume: 1.0,
        }
    }

    /// Starts cue `n`, replacing what plays; other numbers are ignored.
    pub fn start(&mut self, n: u8) {
        if (1..=CUES).contains(&n) && self.bank.len() >= n as usize {
            self.now = Some((n, 0));
        }
    }

    pub fn stop(&mut self) {
        self.now = None;
    }

    /// The cue playing.
    #[cfg(test)]
    pub fn playing(&self) -> Option<u8> {
        self.now.map(|(n, _)| n)
    }

    /// The next sample: a looping theme starts over at its end, any other
    /// cue stops there.
    pub fn next_sample(&mut self) -> f32 {
        let Some((n, pos)) = self.now.as_mut() else {
            return 0.0;
        };
        let buf = &self.bank[*n as usize - 1];
        if *pos >= buf.len() {
            if loops(*n) && !buf.is_empty() {
                *pos = 0;
            } else {
                self.now = None;
                return 0.0;
            }
        }
        let v = buf[*pos];
        *pos += 1;
        v * self.volume
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn player() -> Player {
        // Cue n: n samples, each worth n.
        Player::new((1..=CUES).map(|n| vec![n as f32; n as usize]).collect())
    }

    fn take(p: &mut Player, k: usize) -> Vec<f32> {
        (0..k).map(|_| p.next_sample()).collect()
    }

    #[test]
    fn effects_and_themes_split_at_13_and_four_themes_loop() {
        assert!(!is_theme(12) && is_theme(13) && is_theme(30) && !is_theme(31));
        assert_eq!(
            (1..=CUES).filter(|&n| loops(n)).collect::<Vec<_>>(),
            [14, 15, 16, 17]
        );
    }

    #[test]
    fn a_cue_plays_once_and_a_looping_theme_starts_over() {
        let mut p = player();
        p.start(2);
        assert_eq!(take(&mut p, 4), [2.0, 2.0, 0.0, 0.0]);
        assert_eq!(p.playing(), None);
        p.start(14);
        assert_eq!(take(&mut p, 30).iter().filter(|&&v| v == 14.0).count(), 30);
        assert_eq!(p.playing(), Some(14));
    }

    #[test]
    fn a_new_cue_replaces_a_looping_theme_and_volume_scales() {
        let mut p = player();
        p.start(15);
        take(&mut p, 20);
        p.volume = 0.5;
        p.start(1);
        assert_eq!(take(&mut p, 2), [0.5, 0.0]);
        p.start(0);
        p.start(31);
        assert_eq!(p.playing(), None);
        p.start(16);
        p.stop();
        assert_eq!(p.next_sample(), 0.0);
    }
}
