// SPDX-License-Identifier: GPL-3.0-or-later

//! A Classic long message's 8 stored lines of 32 characters as paragraphs
//! a client wraps to its own width (client 3, magic 3.10). The original
//! shows the stored lines as they are; a modern screen is wider, so the
//! lines are joined back into running text.
//!
//! Where two lines meet, a space goes between them unless the break cuts
//! a word in two. Designers did both: most broke lines between words, a
//! few typed straight on and let words run across the line end. A full
//! line (32 characters, no space at its end) followed by a line that
//! starts with a letter is the ambiguous case, settled with the
//! adventure's own words (`Reflow::new`): the two pieces are one word when
//! together they make a word the adventure uses elsewhere and apart they
//! don't both, when a piece is a stray letter or two that is no word, or,
//! in an adventure whose designer habitually ran words across line ends,
//! whenever the pieces aren't both words. A message laid out as a picture
//! or a table (runs of spaces inside a line, rule lines) keeps its lines.

use crate::model::Adventure;
use std::collections::HashSet;

/// The stored width of a long message's line.
const WIDTH: usize = 32;
/// Above this share of full-line breaks that cut a word for certain, the
/// adventure is taken to run words across line ends as a habit.
const SPLITTER_SHARE: f64 = 0.15;

/// A long message as shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Shown {
    /// Paragraphs (parted by a blank line) to wrap to the client's width.
    Paragraphs(String),
    /// The stored lines, one per screen line, never rewrapped: a layout.
    Lines(String),
}

/// What an adventure's messages tell about its words.
#[derive(Debug, Clone, Default)]
pub struct Reflow {
    words: HashSet<String>,
    /// The designer runs words across line ends.
    splits: bool,
}

/// The letters of a token, for looking words up.
fn key(token: &str) -> String {
    token
        .chars()
        .filter(|c| c.is_ascii_alphabetic() || *c == '\'')
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

fn is_layout(lines: &[&str]) -> bool {
    lines.iter().any(|l| {
        let body = l.trim();
        // A rule line, or a line drawn with symbols.
        let drawn = body.len() >= 4 && !body.chars().any(|c| c.is_ascii_alphanumeric());
        // Columns: a run of 3 spaces inside the line.
        let columns = body.contains("   ");
        // Not text at all (control or graphics bytes).
        let odd = body.chars().any(|c| !(' '..='~').contains(&c));
        drawn || columns || odd
    })
}

/// How a full line's last word meets the next line's first.
enum Meet {
    Space,
    Join,
    /// Pieces that aren't both words, with no other evidence.
    Unsure,
}

impl Reflow {
    /// Learns `adv`'s words: every whole word in its messages (not the
    /// first or last of a long message's line, which may be a piece), its
    /// short messages and its names.
    pub fn new(adv: &Adventure) -> Reflow {
        let mut words = HashSet::new();
        let mut add = |t: &str| {
            let k = key(t);
            // One letter is a word only as A or I.
            if k.len() > 1 || k == "A" || k == "I" {
                words.insert(k);
            }
        };
        for m in adv.long_messages.iter().chain(std::iter::once(&adv.intro)) {
            for line in m.lines() {
                let toks: Vec<&str> = line.split_whitespace().collect();
                for t in toks.iter().skip(1).take(toks.len().saturating_sub(2)) {
                    add(t);
                }
            }
        }
        for m in &adv.short_messages {
            m.split_whitespace().for_each(&mut add);
        }
        for n in adv
            .things
            .iter()
            .map(|t| &t.name)
            .chain(adv.creatures.iter().map(|c| &c.name))
        {
            n.split_whitespace().for_each(&mut add);
        }
        let mut r = Reflow { words, splits: false };
        let (mut full, mut cut) = (0usize, 0usize);
        for m in adv.long_messages.iter().chain(std::iter::once(&adv.intro)) {
            let lines: Vec<&str> = m.lines().collect();
            if is_layout(&lines) {
                continue;
            }
            for w in lines.windows(2) {
                if let Some((x, y)) = Self::full_break(w[0], w[1]) {
                    full += 1;
                    if matches!(r.meet(x, y), Meet::Join) {
                        cut += 1;
                    }
                }
            }
        }
        r.splits = full >= 8 && cut as f64 > full as f64 * SPLITTER_SHARE;
        r
    }

    fn known(&self, k: &str) -> bool {
        self.words.contains(k)
    }

    /// The last word of `a` and the first of `b`, where `a` is a full line
    /// running into a letter or digit at the start of `b`.
    fn full_break<'a>(a: &'a str, b: &'a str) -> Option<(&'a str, &'a str)> {
        let full = a.chars().count() >= WIDTH && !a.ends_with(' ');
        let starts = b.chars().next().is_some_and(|c| c.is_ascii_alphanumeric());
        if !full || !starts {
            return None;
        }
        let x = a.rsplit(' ').next()?;
        let y = b.split(' ').next()?;
        // A whole line of one letter (a fill) says nothing.
        let x_letters: HashSet<char> = x.chars().collect();
        (x_letters.len() > 1).then_some((x, y))
    }

    fn meet(&self, x: &str, y: &str) -> Meet {
        // A word that ends in punctuation ends there.
        if x.ends_with(|c: char| !c.is_ascii_alphanumeric()) {
            return Meet::Space;
        }
        let (kx, ky) = (key(x), key(y));
        let (wx, wy) = (self.known(&kx), self.known(&ky));
        if wx && wy {
            return Meet::Space;
        }
        let joined = format!("{kx}{ky}");
        let stray = |w: bool, k: &str| !w && k.len() <= 2;
        if self.known(&joined) || stray(wx, &kx) || stray(wy, &ky) {
            Meet::Join
        } else {
            Meet::Unsure
        }
    }

    /// `text` (a long message's stored lines, separated by line breaks)
    /// as it is shown.
    pub fn show(&self, text: &str) -> Shown {
        let lines: Vec<&str> = text.lines().collect();
        if is_layout(&lines) {
            return Shown::Lines(text.to_string());
        }
        let mut paras: Vec<String> = vec![String::new()];
        let mut prev: Option<&str> = None;
        for &line in &lines {
            let body = line.trim();
            if body.is_empty() {
                if !paras.last().expect("never empty").is_empty() {
                    paras.push(String::new());
                }
                prev = None;
                continue;
            }
            // A line indented by two or more starts a paragraph.
            if line.starts_with("  ") && !paras.last().expect("never empty").is_empty() {
                paras.push(String::new());
                prev = None;
            }
            let p = paras.last_mut().expect("never empty");
            if let Some(a) = prev.filter(|_| !p.is_empty()) {
                let glue = if body.starts_with(|c: char| ",.;:!?)\"'".contains(c))
                    && !a.ends_with(' ')
                    && a.chars().count() >= WIDTH
                {
                    // The end of a word's punctuation pushed onto the next line.
                    ""
                } else if a.ends_with('-') && !a.ends_with("--") {
                    // A hyphenated word broken at its hyphen.
                    ""
                } else {
                    match Self::full_break(a, line).map(|(x, y)| self.meet(x, y)) {
                        Some(Meet::Join) => "",
                        Some(Meet::Unsure) if self.splits => "",
                        _ => " ",
                    }
                };
                p.push_str(glue);
            }
            p.push_str(body);
            prev = Some(line);
        }
        paras.retain(|p| !p.is_empty());
        // Squeeze the designer's spacing inside a paragraph to one space,
        // two after a full stop kept as one.
        let paras: Vec<String> = paras
            .iter()
            .map(|p| {
                p.split(' ')
                    .filter(|w| !w.is_empty())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect();
        Shown::Paragraphs(paras.join("\n\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reflow(words: &[&str], splits: bool) -> Reflow {
        Reflow {
            words: words.iter().map(|w| w.to_string()).collect(),
            splits,
        }
    }

    fn para(s: &Shown) -> &str {
        match s {
            Shown::Paragraphs(p) => p,
            Shown::Lines(l) => panic!("kept as lines: {l}"),
        }
    }

    #[test]
    fn short_lines_join_with_a_space() {
        let r = reflow(&[], false);
        let s = r.show("A HEALER SELLS SALVES\nFROM HER HUT.");
        assert_eq!(para(&s), "A HEALER SELLS SALVES FROM HER HUT.");
    }

    #[test]
    fn a_word_cut_at_the_line_end_is_mended() {
        let r = reflow(&["ESCAPE", "THE", "YOU", "MUST"], false);
        // 32 characters, the word ESCAPE running across the end.
        let a = "YOU MUST FIND THE WAY TO ESCA";
        let a = format!("{a:>32}");
        let s = r.show(&format!("{a}\nPE THE ISLAND."));
        assert!(para(&s).ends_with("WAY TO ESCAPE THE ISLAND."), "{s:?}");
    }

    #[test]
    fn two_words_at_a_full_line_end_keep_their_space() {
        let r = reflow(&["SAIL", "SAFELY"], true);
        let a = format!("{:>32}", "BEFORE YOU SAIL");
        let s = r.show(&format!("{a}\nSAFELY HOME."));
        assert!(para(&s).ends_with("YOU SAIL SAFELY HOME."), "{s:?}");
    }

    #[test]
    fn unknown_pieces_join_only_for_a_designer_who_splits() {
        let a = format!("{:>32}", "THE ROAD TO HEADQ");
        let text = format!("{a}\nUARTERS IS LONG.");
        assert!(para(&reflow(&[], true).show(&text)).contains("HEADQUARTERS"));
        assert!(para(&reflow(&[], false).show(&text)).contains("HEADQ UARTERS"));
    }

    #[test]
    fn blank_lines_and_indents_part_paragraphs() {
        let r = reflow(&[], false);
        let s = r.show("   ONE LINE\nGOES ON.\n\nTWO.\n  THREE.");
        assert_eq!(para(&s), "ONE LINE GOES ON.\n\nTWO.\n\nTHREE.");
    }

    #[test]
    fn a_layout_keeps_its_lines() {
        let r = reflow(&[], false);
        let text = "  NAME      SCORE\n--------------\n  ANNA      12";
        assert_eq!(r.show(text), Shown::Lines(text.to_string()));
    }

    #[test]
    fn a_hyphen_at_the_end_joins() {
        let r = reflow(&[], false);
        assert_eq!(para(&r.show("A BIG-\nTOED FARMER")), "A BIG-TOED FARMER");
    }
}
