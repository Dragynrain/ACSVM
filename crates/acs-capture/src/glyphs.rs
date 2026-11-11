// SPDX-License-Identifier: GPL-3.0-or-later

//! Reading text off the rendered screen (docs/capture.md 5.3): 8 x 8 cells
//! matched against a glyph table taken from the program's own screens. The
//! table is the program's font, so it stays in the research folder and is
//! loaded at run time; nothing here knows what any glyph looks like.

use crate::screen::Frame;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

/// The text colour as VICE 3.10 renders C64 colour 1 (white).
pub const INK: [u8; 3] = [255, 255, 255];
/// Sprite colours over text: the cyan mark beside the profile's Wisdom and
/// Power (VICE 3.10's rendering of C64 colour 3).
pub const MASK: [[u8; 3]; 1] = [[126, 243, 214]];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Px {
    Paper,
    Ink,
    Unknown,
}

/// A screen as ink, paper and unknown pixels (inner screen, 1:1).
pub struct Screen {
    pub w: usize,
    pub h: usize,
    px: Vec<Px>,
}

fn classify(c: [u8; 3], ink: &[[u8; 3]], mask: &[[u8; 3]]) -> Px {
    if ink.contains(&c) {
        Px::Ink
    } else if mask.contains(&c) {
        Px::Unknown
    } else {
        Px::Paper
    }
}

impl Screen {
    pub fn new(w: usize, h: usize, px: Vec<Px>) -> Self {
        assert_eq!(px.len(), w * h);
        Screen { w, h, px }
    }

    /// A live frame's inner area, classified by the colours it shows as.
    pub fn from_frame(f: &Frame, ink: &[[u8; 3]], mask: &[[u8; 3]]) -> Self {
        let (_, _, w, h) = f.inner();
        let mut px = Vec::with_capacity(w * h);
        for y in 0..h {
            for x in 0..w {
                let c = f
                    .palette
                    .get(f.at(x, y) as usize)
                    .copied()
                    .unwrap_or([255, 0, 255]);
                px.push(classify(c, ink, mask));
            }
        }
        Screen { w, h, px }
    }

    /// A screenshot of the inner screen (as `Frame::save_png` writes it, at
    /// any integer scale of 320 pixels across).
    pub fn from_png(path: &Path, ink: &[[u8; 3]], mask: &[[u8; 3]]) -> Result<Self, String> {
        let err = |e: &dyn std::fmt::Display| format!("{}: {e}", path.display());
        let file = std::fs::File::open(path).map_err(|e| err(&e))?;
        let mut dec = png::Decoder::new(std::io::BufReader::new(file));
        dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut rd = dec.read_info().map_err(|e| err(&e))?;
        let mut buf = vec![0; rd.output_buffer_size()];
        let info = rd.next_frame(&mut buf).map_err(|e| err(&e))?;
        let ch = info.color_type.samples();
        if ch < 3 {
            return Err(err(&"want an RGB screenshot"));
        }
        let (fw, fh) = (info.width as usize, info.height as usize);
        let scale = (fw / 320).max(1);
        let (w, h) = (fw / scale, fh / scale);
        let mut px = Vec::with_capacity(w * h);
        for y in 0..h {
            for x in 0..w {
                let i = (y * scale * fw + x * scale) * ch;
                px.push(classify([buf[i], buf[i + 1], buf[i + 2]], ink, mask));
            }
        }
        Ok(Screen { w, h, px })
    }

    fn at(&self, x: usize, y: usize) -> Px {
        self.px[y * self.w + x]
    }

    /// The cell at column `cx`, row `cy` of the grid offset by (dx, dy).
    pub fn cell(&self, (dx, dy): (usize, usize), cx: usize, cy: usize) -> Cell {
        let mut c = Cell { bits: 0, known: 0 };
        for y in 0..8 {
            for x in 0..8 {
                let bit = 1u64 << (63 - (y * 8 + x));
                match self.at(dx + cx * 8 + x, dy + cy * 8 + y) {
                    Px::Ink => {
                        c.bits |= bit;
                        c.known |= bit;
                    }
                    Px::Paper => c.known |= bit,
                    Px::Unknown => {}
                }
            }
        }
        c
    }

    /// Columns and rows of whole cells on the grid offset by (dx, dy).
    pub fn cells(&self, (dx, dy): (usize, usize)) -> (usize, usize) {
        ((self.w - dx) / 8, (self.h - dy) / 8)
    }
}

/// One 8 x 8 cell: ink bits (row 0 in the top byte, bit 7 the left pixel)
/// and which pixels are known (not under a sprite).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub bits: u64,
    pub known: u64,
}

impl Cell {
    pub fn blank(&self) -> bool {
        self.bits == 0
    }
    pub fn whole(&self) -> bool {
        self.known == u64::MAX
    }
    /// Whether a glyph of these bits agrees with every known pixel.
    pub fn fits(&self, glyph: u64) -> bool {
        (glyph ^ self.bits) & self.known == 0
    }
}

/// The glyph table: bitmaps with labels (`None` until labelled by eye).
#[derive(Debug, Clone, Default)]
pub struct Table {
    pub glyphs: Vec<(u64, Option<String>)>,
}

impl Table {
    /// Lines of `HEX16 LABEL`; a missing label is an unlabelled glyph; `#`
    /// starts a comment line. A label is one character or a word.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut t = Table::default();
        for (n, line) in text.lines().enumerate() {
            let line = line.trim_end();
            if line.trim().is_empty() || line.starts_with('#') {
                continue;
            }
            let (hex, label) = line.split_once(' ').unwrap_or((line, ""));
            let bits = u64::from_str_radix(hex, 16).map_err(|_| format!("line {}: bad bitmap", n + 1))?;
            let label = label.trim();
            t.glyphs
                .push((bits, (!label.is_empty()).then(|| label.to_string())));
        }
        Ok(t)
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Self::parse(&text)
    }

    pub fn to_text(&self) -> String {
        let mut s = String::from("# glyph table: 8 x 8 bitmap rows top to bottom, then the label\n");
        for (bits, label) in &self.glyphs {
            s.push_str(&format!("{bits:016X} {}\n", label.as_deref().unwrap_or("")));
        }
        s
    }

    fn index(&self, bits: u64) -> Option<usize> {
        self.glyphs.iter().position(|g| g.0 == bits)
    }

    /// Glyphs that fit a cell.
    pub fn fitting(&self, c: &Cell) -> Vec<usize> {
        (0..self.glyphs.len())
            .filter(|&i| c.fits(self.glyphs[i].0))
            .collect()
    }
}

/// The grid offset when no table is known: the fewest distinct non-blank
/// cells, then the fewest non-blank cells (5.3.2). Only whole cells count.
pub fn find_grid(s: &Screen) -> (usize, usize) {
    let mut best = (usize::MAX, usize::MAX, (0, 0));
    for dy in 0..8 {
        for dx in 0..8 {
            let (cols, rows) = s.cells((dx, dy));
            let mut seen = std::collections::HashSet::new();
            let mut n = 0;
            for cy in 0..rows {
                for cx in 0..cols {
                    let c = s.cell((dx, dy), cx, cy);
                    if c.whole() && !c.blank() {
                        seen.insert(c.bits);
                        n += 1;
                    }
                }
            }
            let key = (seen.len(), n, (dx, dy));
            if (key.0, key.1) < (best.0, best.1) {
                best = key;
            }
        }
    }
    best.2
}

/// The grid offset that matches the most cells against the table, then the
/// fewest non-blank cells it can't match.
pub fn match_grid(s: &Screen, t: &Table) -> (usize, usize) {
    let mut best = (0usize, usize::MAX, (0, 0));
    for dy in 0..8 {
        for dx in 0..8 {
            let (cols, rows) = s.cells((dx, dy));
            let (mut hit, mut miss) = (0, 0);
            for cy in 0..rows {
                for cx in 0..cols {
                    let c = s.cell((dx, dy), cx, cy);
                    if c.blank() {
                        continue;
                    }
                    if c.whole() && t.index(c.bits).is_some() {
                        hit += 1;
                    } else {
                        miss += 1;
                    }
                }
            }
            if hit > best.0 || (hit == best.0 && miss < best.1) {
                best = (hit, miss, (dx, dy));
            }
        }
    }
    best.2
}

/// A screen read as text.
#[derive(Debug, Clone, Serialize)]
pub struct Text {
    pub grid: (usize, usize),
    /// One string per cell row, right-trimmed: a character per cell (`?`
    /// for a cell no labelled glyph fits, `{n}` for unlabelled glyph n).
    pub lines: Vec<String>,
    /// Cells with unknown pixels that a letter or digit also fits, or that
    /// more than one label fits, as (column, row): their reading is a guess.
    pub doubtful: Vec<(usize, usize)>,
}

impl Text {
    fn is_doubtful(&self, cx: usize, cy: usize) -> bool {
        self.doubtful.contains(&(cx, cy))
    }
}

/// Reads every cell of the screen on the given grid.
pub fn read_at(s: &Screen, t: &Table, grid: (usize, usize)) -> Text {
    let (cols, rows) = s.cells(grid);
    let mut lines = Vec::with_capacity(rows);
    let mut doubtful = Vec::new();
    for cy in 0..rows {
        let mut line = String::new();
        for cx in 0..cols {
            let c = s.cell(grid, cx, cy);
            let fit = t.fitting(&c);
            let labels: std::collections::BTreeSet<String> = fit
                .iter()
                .map(|&i| t.glyphs[i].1.clone().unwrap_or_else(|| format!("{{{i}}}")))
                .collect();
            if c.blank() {
                line.push(' ');
                // Doubtful when a letter or digit could hide under the
                // sprite (an unlabelled glyph counts; punctuation doesn't).
                let hides = |g: &(u64, Option<String>)| {
                    g.0 != 0
                        && c.fits(g.0)
                        && g.1
                            .as_deref()
                            .is_none_or(|l| l.chars().all(|ch| ch.is_ascii_alphanumeric()))
                };
                if !c.whole() && t.glyphs.iter().any(hides) {
                    doubtful.push((cx, cy));
                }
            } else if labels.len() == 1 {
                line.push_str(labels.iter().next().unwrap());
                if !c.whole() && fit.len() > 1 {
                    doubtful.push((cx, cy));
                }
            } else {
                line.push('?');
                if labels.len() > 1 {
                    doubtful.push((cx, cy));
                }
            }
        }
        lines.push(line.trim_end().to_string());
    }
    Text {
        grid,
        lines,
        doubtful,
    }
}

pub fn read(s: &Screen, t: &Table) -> Text {
    read_at(s, t, match_grid(s, t))
}

/// Adds every whole, non-blank cell not yet in the table, unlabelled, on the
/// table's grid if the table has any, else on the self-found one. Returns
/// the new glyphs' indices.
pub fn learn(s: &Screen, t: &mut Table) -> Vec<usize> {
    let grid = if !t.glyphs.is_empty() {
        match_grid(s, t)
    } else {
        find_grid(s)
    };
    let (cols, rows) = s.cells(grid);
    let mut added = Vec::new();
    for cy in 0..rows {
        for cx in 0..cols {
            let c = s.cell(grid, cx, cy);
            if c.whole() && !c.blank() && t.index(c.bits).is_none() {
                t.glyphs.push((c.bits, None));
                added.push(t.glyphs.len() - 1);
            }
        }
    }
    added
}

/// A glyph as eight lines of `#` and `.`.
pub fn art(bits: u64) -> String {
    let mut s = String::new();
    for y in 0..8 {
        for x in 0..8 {
            s.push(if bits >> (63 - (y * 8 + x)) & 1 == 1 {
                '#'
            } else {
                '.'
            });
        }
        s.push('\n');
    }
    s
}

/// The profile screen's stats, by the names it shows (docs/capture.md 5.3.5).
pub const PROFILE_STATS: [(&str, &str); 14] = [
    ("con", "CONSTITUTION"),
    ("lf", "LIFE FORCE"),
    ("wis", "WISDOM"),
    ("pow", "POWER"),
    ("str", "STRENGTH"),
    ("size", "SIZE"),
    ("dex", "DEXTERITY"),
    ("armor", "ARMOR SKILL"),
    ("missile", "MISSILE SKILL"),
    ("dodge", "DODGE SKILL"),
    ("melee", "MELEE SKILL"),
    ("parry", "PARRY SKILL"),
    ("wealth", "WEALTH"),
    ("spd", "SPEED"),
];

/// One stat read off the profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Stat {
    pub value: Option<u16>,
    /// What was read after the name, up to the next space.
    pub raw: String,
    /// The cell after the number is doubtful (it may hide another digit).
    pub doubtful: bool,
}

/// Finds each stat name on a line and reads the number after it.
pub fn profile(text: &Text) -> BTreeMap<String, Stat> {
    let mut out = BTreeMap::new();
    for (key, name) in PROFILE_STATS {
        for (cy, line) in text.lines.iter().enumerate() {
            // Cells, not bytes: unlabelled `{n}` glyphs would shift columns,
            // so look only at lines made of single characters.
            let chars: Vec<char> = line.chars().collect();
            let Some(at) = find_word(&chars, name) else {
                continue;
            };
            let mut i = at + name.chars().count();
            while i < chars.len() && chars[i] == ' ' {
                i += 1;
            }
            let start = i;
            while i < chars.len() && chars[i] != ' ' {
                i += 1;
            }
            let raw: String = chars[start..i].iter().collect();
            // The font draws zero and the letter O alike.
            let digits: String = raw.chars().map(|c| if c == 'O' { '0' } else { c }).collect();
            let value = (!digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()))
                .then(|| digits.parse().ok())
                .flatten();
            let doubtful = (start..=i).any(|cx| text.is_doubtful(cx, cy));
            out.insert(key.to_string(), Stat { value, raw, doubtful });
            break;
        }
    }
    out
}

/// Where `word` starts in `chars` as a whole word (not inside a longer one).
fn find_word(chars: &[char], word: &str) -> Option<usize> {
    let w: Vec<char> = word.chars().collect();
    (0..chars.len().saturating_sub(w.len() - 1)).find(|&i| {
        chars[i..i + w.len()] == w[..]
            && (i == 0 || !chars[i - 1].is_ascii_alphabetic())
            && chars.get(i + w.len()).is_none_or(|c| !c.is_ascii_alphabetic())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A made-up 3-glyph font of our own: a bar, a box and a cross, drawn
    /// in the top-left 6 x 7 of the cell.
    const BAR: u64 = 0x1010_1010_1010_1000;
    const BOX: u64 = 0x7C44_4444_4444_7C00;
    const CROSS: u64 = 0x1010_107C_1010_1000;

    fn screen(cells: &[&[u64]], off: (usize, usize), sprite: Option<(usize, usize, usize, usize)>) -> Screen {
        let (w, h) = (64, 32);
        let mut px = vec![Px::Paper; w * h];
        for (cy, row) in cells.iter().enumerate() {
            for (cx, &g) in row.iter().enumerate() {
                for y in 0..8 {
                    for x in 0..8 {
                        if g >> (63 - (y * 8 + x)) & 1 == 1 {
                            px[(off.1 + cy * 8 + y) * w + off.0 + cx * 8 + x] = Px::Ink;
                        }
                    }
                }
            }
        }
        if let Some((sx, sy, sw, sh)) = sprite {
            for y in sy..sy + sh {
                for x in sx..sx + sw {
                    px[y * w + x] = Px::Unknown;
                }
            }
        }
        Screen::new(w, h, px)
    }

    fn table() -> Table {
        Table::parse(&format!("# test\n{BAR:016X} 1\n{BOX:016X} 0\n{CROSS:016X} +\n")).unwrap()
    }

    #[test]
    fn table_round_trips() {
        let mut t = table();
        t.glyphs.push((0xFF, None));
        assert_eq!(Table::parse(&t.to_text()).unwrap().glyphs, t.glyphs);
    }

    #[test]
    fn finds_an_offset_grid_and_reads_it() {
        let s = screen(&[&[BAR, BOX, 0, CROSS], &[BOX, BOX, BAR]], (3, 5), None);
        let t = table();
        assert_eq!(match_grid(&s, &t), (3, 5));
        let text = read(&s, &t);
        assert_eq!(text.lines[0], "10 +");
        assert_eq!(text.lines[1], "001");
        assert!(text.doubtful.is_empty());
    }

    #[test]
    fn learns_unknown_cells_on_the_self_found_grid() {
        let s = screen(&[&[BAR, BOX, BAR, BOX], &[CROSS, BOX, BAR, BAR]], (3, 5), None);
        let mut t = Table::default();
        let added = learn(&s, &mut t);
        // Blank rows and columns make neighbouring grids tie; any of them
        // gives one glyph per shape, and the table then fixes the grid.
        assert_eq!(added.len(), 3);
        // Unlabelled glyphs read as their index.
        let text = read(&s, &t);
        assert!(text.lines[0].starts_with('{'));
    }

    #[test]
    fn a_sprite_over_a_cell_makes_it_doubtful_not_wrong() {
        // "10" then a blank cell under a sprite: the blank could hide a bar.
        let s = screen(&[&[BAR, BOX, 0]], (0, 0), Some((18, 0, 4, 8)));
        let t = table();
        let text = read_at(&s, &t, (0, 0));
        assert_eq!(text.lines[0], "10");
        assert_eq!(text.doubtful, vec![(2, 0)]);
        // A sprite over the box's middle rows: only the box still fits.
        let s = screen(&[&[BOX]], (0, 0), Some((0, 2, 8, 3)));
        let text = read_at(&s, &table(), (0, 0));
        assert_eq!(text.lines[0], "0");
    }

    #[test]
    fn profile_reads_numbers_after_names() {
        let text = Text {
            grid: (0, 0),
            lines: vec![
                "CONSTITUTION  11   LIFE FORCE   7".into(),
                "WISDOM        20   POWER        2".into(),
                "         SPEED 6   WEALTH 1O".into(),
            ],
            doubtful: vec![(33, 1)],
        };
        let p = profile(&text);
        assert_eq!(p["con"].value, Some(11));
        assert_eq!(p["lf"].value, Some(7));
        assert_eq!(p["wis"].value, Some(20));
        assert!(!p["wis"].doubtful);
        assert_eq!(p["pow"].value, Some(2));
        assert!(p["pow"].doubtful);
        assert_eq!(p["spd"].value, Some(6));
        assert_eq!(p["wealth"].value, Some(10));
        assert!(!p.contains_key("str"));
    }
}
