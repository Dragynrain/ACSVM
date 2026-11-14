// SPDX-License-Identifier: GPL-3.0-or-later

//! A small line script for driving a VICE session by hand: what a player
//! would do (joystick, keys, waiting) plus snapshots and screenshots.
//!
//! ```text
//! run 2m            # step 2,000,000 instructions (k and m suffixes)
//! joy 1 ne          # control port 2 (0 = port 1): n s e w ne nw se sw, fire, none
//! key Y{RETURN}     # PETSCII into the keyboard buffer
//! shot title        # OUT/title.png
//! settle 20m        # run until the screen stops changing (or the limit)
//! dump boot.vsf     # snapshot; undump boot.vsf to restore
//! attach scen.d64   # disk image into drive 8
//! poke F5B0 20      # set memory (hex address, decimal bytes)
//! peek F59F 37      # COUNT bytes in decimal (one byte alone: binary)
//! until 71FA !1 30m # run until a byte equals (=) or leaves (!) a value
//! typemem 4F4F AB   # feed each key until the byte at the address changes
//! ```

use crate::monitor::{Error, Result, MEMSPACE_DRIVE8};
use crate::screen::Frame;
use crate::vice::Vice;
use std::path::{Path, PathBuf};

pub struct Session {
    pub vice: Vice,
    pub out: PathBuf,
    palette: Option<Vec<[u8; 3]>>,
    /// Instructions stepped since the session (or last undump) began.
    pub clock: u64,
    pub log: Vec<String>,
    /// Register ids of LIN and CYC, once looked up.
    beam_ids: Option<(u8, u8)>,
}

/// Drive 8's VIA 2 interrupt flag register, in the drive's address space
/// (the stores researcher, pass 2, 2026-11-14; docs/capture.md section 9).
pub const DRIVE_VIA2_IFR: u16 = 0x1C0D;

/// PAL C64: 63 cycles a raster line, 312 lines a frame.
const PAL_LINE_CYCLES: u64 = 63;
const PAL_FRAME_CYCLES: u64 = PAL_LINE_CYCLES * 312;

/// Frames begun while `instructions` ran, the beam going from cycle `from`
/// to cycle `to` of the frame: the elapsed cycles are the shortest
/// distance of at least 2 per instruction (see `run_counting_frames`).
fn frames_passed(from: u64, to: u64, instructions: u64) -> u64 {
    let mut elapsed = (to + PAL_FRAME_CYCLES - from) % PAL_FRAME_CYCLES;
    if elapsed < 2 * instructions {
        elapsed += PAL_FRAME_CYCLES;
    }
    (from + elapsed) / PAL_FRAME_CYCLES
}

pub fn parse_count(s: &str) -> Option<u64> {
    let s = s.trim().to_ascii_lowercase();
    let (num, mul) = match s.chars().last()? {
        'k' => (&s[..s.len() - 1], 1_000),
        'm' => (&s[..s.len() - 1], 1_000_000),
        _ => (&s[..], 1),
    };
    num.parse::<f64>().ok().map(|n| (n * mul as f64) as u64)
}

/// A hex address, with or without a leading `$`.
fn parse_addr(s: &str) -> Option<u16> {
    u16::from_str_radix(s.trim_start_matches('$'), 16).ok()
}

/// Joystick bits for a direction word, or a number.
pub fn joy_bits(word: &str) -> Option<u16> {
    let mut bits = 0u16;
    for part in word.split('+') {
        bits |= match part {
            "none" | "0" => 0,
            "n" | "up" => 1,
            "s" | "down" => 2,
            "w" | "left" => 4,
            "e" | "right" => 8,
            "ne" => 1 | 8,
            "nw" => 1 | 4,
            "se" => 2 | 8,
            "sw" => 2 | 4,
            "fire" => 16,
            n => n.parse().ok()?,
        };
    }
    Some(bits)
}

/// ASCII to PETSCII for the keyboard buffer; `{RETURN}`, `{SPACE}`, `{F1}`..
pub fn petscii(text: &str) -> Vec<u8> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        if c == '{' {
            if let Some(end) = rest.find('}') {
                let name = &rest[1..end];
                let code = match name {
                    "RETURN" => 13,
                    "SPACE" => 32,
                    "F1" => 133,
                    "F3" => 134,
                    "F5" => 135,
                    "F7" => 136,
                    "F2" => 137,
                    "F4" => 138,
                    "F6" => 139,
                    "F8" => 140,
                    "UP" => 145,
                    "DOWN" => 17,
                    "LEFT" => 157,
                    "RIGHT" => 29,
                    "DEL" => 20,
                    "STOP" => 3,
                    n => n.parse().unwrap_or(32),
                };
                out.push(code);
                rest = &rest[end + 1..];
                continue;
            }
        }
        let b = c as u32;
        out.push(match c {
            'a'..='z' => (b - 32) as u8,
            _ => b as u8,
        });
        rest = &rest[c.len_utf8()..];
    }
    out
}

impl Session {
    pub fn new(vice: Vice, out: PathBuf) -> Self {
        Session {
            vice,
            out,
            palette: None,
            clock: 0,
            log: Vec::new(),
            beam_ids: None,
        }
    }

    pub fn frame(&mut self) -> Result<Frame> {
        if self.palette.is_none() {
            self.palette = Some(self.vice.mon.palette()?);
        }
        let display = self.vice.mon.display()?;
        Ok(Frame {
            display,
            palette: self.palette.clone().unwrap_or_default(),
        })
    }

    pub fn run(&mut self, n: u64) -> Result<()> {
        self.vice.mon.advance_long(n)?;
        self.clock += n;
        Ok(())
    }

    /// Steps `n` instructions in chunks and counts the frames that pass
    /// (beam wraparounds), so instruction counts can be turned into emulated
    /// time (PAL: 50.125 frames per second). Exact (docs/capture.md 2.6):
    /// the beam's cycle in the frame is read after each chunk, and a chunk
    /// of c instructions takes at least 2c cycles (no 6502 instruction is
    /// shorter), so a chunk that seems to take fewer went once more round
    /// the frame. That holds while a chunk takes under a frame plus 2c
    /// cycles, almost 12 cycles an instruction at 2,000 (the slowest
    /// instruction takes 8, plus the VIC-II's stolen cycles).
    pub fn run_counting_frames(&mut self, n: u64) -> Result<u64> {
        const CHUNK: u64 = 2_000;
        let mut frames = 0;
        let mut last = self.beam()?;
        let mut done = 0;
        while done < n {
            let c = CHUNK.min(n - done);
            self.run(c)?;
            done += c;
            let now = self.beam()?;
            frames += frames_passed(last, now, c);
            last = now;
        }
        Ok(frames)
    }

    /// The beam's cycle within the PAL frame: raster line x 63 + the cycle
    /// in the line, from the monitor's register list (LIN, CYC).
    pub fn beam(&mut self) -> Result<u64> {
        if self.beam_ids.is_none() {
            let names = self.vice.mon.registers_available()?;
            let id = |want: &str| names.iter().find(|(_, n)| n == want).map(|(i, _)| *i);
            match (id("LIN"), id("CYC")) {
                (Some(l), Some(c)) => self.beam_ids = Some((l, c)),
                _ => return Err(Error::Protocol("no LIN or CYC register".into())),
            }
        }
        let (lin, cyc) = self.beam_ids.unwrap();
        let regs = self.vice.mon.registers()?;
        let get = |id| {
            regs.iter()
                .find(|(i, _)| *i == id)
                .map(|(_, v)| *v as u64)
                .unwrap_or(0)
        };
        Ok((get(lin) * PAL_LINE_CYCLES + get(cyc)) % PAL_FRAME_CYCLES)
    }

    pub fn shot(&mut self, name: &str) -> Result<PathBuf> {
        let f = self.frame()?;
        let p = self.out.join(format!("{name}.png"));
        f.save_png(&p, false, 2)?;
        Ok(p)
    }

    /// Whether drive 8 serves its timer interrupt (docs/capture.md 10.21):
    /// its VIA 2 interrupt flags (`$1C0D` in the drive's own memory) read
    /// `samples` times, `gap` instructions apart. A working idle drive
    /// clears the timer flag (bit 6) now and then; a dead one (seen in some
    /// saved snapshots) never does, and its next disk read never starts.
    /// Returns whether it was ever clear, and the readings.
    pub fn drive_alive(&mut self, samples: usize, gap: u64) -> Result<(bool, Vec<u8>)> {
        let mut seen = Vec::with_capacity(samples);
        for i in 0..samples {
            if i > 0 {
                self.run(gap)?;
            }
            let b = self
                .vice
                .mon
                .mem_read_space(DRIVE_VIA2_IFR, DRIVE_VIA2_IFR, false, MEMSPACE_DRIVE8)?;
            seen.push(b.first().copied().unwrap_or(0));
        }
        let alive = seen.iter().any(|b| b & 0x40 == 0);
        Ok((alive, seen))
    }

    /// Runs in chunks until two frames `quiet` chunks apart are equal.
    pub fn settle(&mut self, limit: u64, chunk: u64) -> Result<u64> {
        let mut last = self.frame()?.hash();
        let mut stepped = 0;
        let mut same = 0;
        while stepped < limit {
            self.run(chunk)?;
            stepped += chunk;
            let h = self.frame()?.hash();
            if h == last {
                same += 1;
                if same >= 3 {
                    break;
                }
            } else {
                same = 0;
                last = h;
            }
        }
        Ok(stepped)
    }

    fn resolve(&self, p: &str) -> PathBuf {
        let path = Path::new(p);
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.out.join(path)
        }
    }

    /// Executes one script line; returns a note to print.
    pub fn line(&mut self, line: &str) -> std::result::Result<String, String> {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            return Ok(String::new());
        }
        let (word, arg) = line.split_once(' ').unwrap_or((line, ""));
        let arg = arg.trim();
        let e = |x: crate::monitor::Error| x.to_string();
        match word {
            "run" => {
                let n = parse_count(arg).ok_or("run: bad count")?;
                self.run(n).map_err(e)?;
                Ok(format!("ran {n} (clock {})", self.clock))
            }
            "settle" => {
                let n = parse_count(arg).unwrap_or(50_000_000);
                let s = self.settle(n, 200_000).map_err(e)?;
                Ok(format!("settled after {s} (clock {})", self.clock))
            }
            "shot" => {
                let p = self.shot(arg).map_err(e)?;
                Ok(format!("wrote {}", p.display()))
            }
            "joy" => {
                let (port, dir) = arg.split_once(' ').ok_or("joy PORT DIR")?;
                let port: u16 = port.parse().map_err(|_| "joy: bad port")?;
                let bits = joy_bits(dir.trim()).ok_or("joy: bad direction")?;
                self.vice.mon.joystick(port, bits).map_err(e)?;
                Ok(format!("joy {port} = {bits}"))
            }
            "key" => {
                self.vice.mon.keyboard_feed(&petscii(arg)).map_err(e)?;
                Ok(format!("fed {arg}"))
            }
            "type" => {
                // type TEXT: one key at a time, each fed again (up to 6
                // times) until the screen changes, since a fed key is
                // sometimes dropped; a key that never shows is reported.
                let mut notes = Vec::new();
                for code in petscii(arg) {
                    let before = self.frame().map_err(e)?.hash();
                    let mut shown = None;
                    'tries: for attempt in 1..=6 {
                        self.vice.mon.keyboard_feed(&[code]).map_err(e)?;
                        for _ in 0..20 {
                            self.run(100_000).map_err(e)?;
                            if self.frame().map_err(e)?.hash() != before {
                                shown = Some(attempt);
                                break 'tries;
                            }
                        }
                    }
                    self.settle(10_000_000, 200_000).map_err(e)?;
                    notes.push(match shown {
                        Some(n) => format!("{code}:{n}"),
                        None => format!("{code}:none"),
                    });
                }
                Ok(format!("typed {}", notes.join(" ")))
            }
            "dump" => {
                // dump FILE [nodisk]: nodisk leaves the disk images out, so a
                // restore keeps whatever disk VICE was started with.
                let (file, nodisk) = match arg.rsplit_once(' ') {
                    Some((f, "nodisk")) => (f, true),
                    _ => (arg, false),
                };
                let p = self.resolve(file);
                self.vice.mon.dump(&p.display().to_string(), !nodisk).map_err(e)?;
                Ok(format!("snapshot {}", p.display()))
            }
            "undump" => {
                let p = self.resolve(arg);
                let pc = self.vice.mon.undump(&p.display().to_string()).map_err(e)?;
                self.vice.release_joysticks().map_err(e)?;
                self.clock = 0;
                Ok(format!("restored {} (pc {pc:#06x})", p.display()))
            }
            "detach" => {
                // VICE keeps written sectors in the drive's track buffer and
                // writes the image file on a track change or a detach, so
                // detach before reading a disk the program has just written.
                let out = self.vice.text_command("detach 8").map_err(e)?;
                Ok(format!("detached 8: {}", out.trim()))
            }
            "attach" => {
                let p = self.resolve(arg);
                let out = self.vice.attach8(&p).map_err(e)?;
                Ok(format!("attached {}: {}", p.display(), out.trim()))
            }
            "reset" => {
                // reset [8..11]: the machine, or one drive.
                let what: u8 = if arg.is_empty() {
                    0
                } else {
                    arg.parse().map_err(|_| "reset [8..11]")?
                };
                self.vice.mon.reset(what).map_err(e)?;
                Ok(format!("reset {what}"))
            }
            "watch" => {
                // watch TOTAL STEP PREFIX: run, saving a shot whenever the screen changes.
                let mut parts = arg.split_whitespace();
                let total = parts
                    .next()
                    .and_then(parse_count)
                    .ok_or("watch TOTAL STEP PREFIX")?;
                let step = parts.next().and_then(parse_count).ok_or("watch: bad step")?;
                let prefix = parts.next().unwrap_or("w").to_string();
                let mut last = 0u64;
                let mut saved = Vec::new();
                let mut done = 0;
                while done < total {
                    self.run(step).map_err(e)?;
                    done += step;
                    let f = self.frame().map_err(e)?;
                    let h = f.hash();
                    if h != last {
                        last = h;
                        let name = format!("{prefix}{:04}", self.clock / 100_000);
                        f.save_png(&self.out.join(format!("{name}.png")), false, 2)
                            .map_err(|x| x.to_string())?;
                        saved.push(name);
                    }
                }
                Ok(format!("watch saved {}", saved.join(" ")))
            }
            "track" => {
                // track TOTAL STEP COLOUR: print the colour's blobs whenever they change.
                let mut parts = arg.split_whitespace();
                let total = parts
                    .next()
                    .and_then(parse_count)
                    .ok_or("track TOTAL STEP COLOUR")?;
                let step = parts.next().and_then(parse_count).ok_or("track: bad step")?;
                let colour: u8 = parts
                    .next()
                    .and_then(|c| c.parse().ok())
                    .ok_or("track: bad colour")?;
                let mut last = String::new();
                let mut lines = Vec::new();
                let mut done = 0;
                while done < total {
                    self.run(step).map_err(e)?;
                    done += step;
                    let f = self.frame().map_err(e)?;
                    let bl = crate::observe::blobs(&f, colour, (0, 0, 10_000, 10_000));
                    let desc: Vec<String> = bl
                        .iter()
                        .map(|b| format!("({},{})-({},{})", b.x0, b.y0, b.x1, b.y1))
                        .collect();
                    let d = desc.join(" ");
                    if d != last {
                        lines.push(format!("{:>10} {d}", self.clock));
                        last = d;
                    }
                }
                Ok(lines.join("\n"))
            }
            "regs" => {
                let names = self.vice.mon.registers_available().map_err(e)?;
                let vals = self.vice.mon.registers().map_err(e)?;
                let v: Vec<String> = vals
                    .iter()
                    .map(|(id, v)| {
                        let n = names
                            .iter()
                            .find(|(i, _)| i == id)
                            .map(|x| x.1.as_str())
                            .unwrap_or("?");
                        format!("{n}={v}")
                    })
                    .collect();
                Ok(v.join(" "))
            }
            "peek" => {
                // peek HEXADDR: one byte in binary; peek HEXADDR COUNT: COUNT
                // bytes in decimal (section 9's addresses).
                let mut parts = arg.split_whitespace();
                let a = parts.next().and_then(parse_addr).ok_or("peek HEXADDR [COUNT]")?;
                match parts.next() {
                    None => {
                        let b = self.vice.mon.mem_read(a, a, true).map_err(e)?;
                        Ok(format!("${a:04x} = {:08b}", b.first().copied().unwrap_or(0)))
                    }
                    Some(n) => {
                        let n: u16 = n.parse().map_err(|_| "peek: bad count")?;
                        let end = a.saturating_add(n.max(1) - 1);
                        let b = self.vice.mon.mem_read(a, end, true).map_err(e)?;
                        let v: Vec<String> = b.iter().map(|x| x.to_string()).collect();
                        Ok(format!("${a:04x} = {}", v.join(" ")))
                    }
                }
            }
            "until" => {
                // until HEXADDR =V|!V [LIMIT [STEP]]: run in STEP chunks
                // (100k) until the byte equals (=) or differs from (!) V, at
                // most LIMIT (50m); reports the instructions run and the byte.
                let mut parts = arg.split_whitespace();
                let a = parts.next().and_then(parse_addr).ok_or("until HEXADDR =V|!V")?;
                let cond = parts.next().ok_or("until HEXADDR =V|!V")?;
                let (want_eq, v) = match cond.split_at(1) {
                    ("=", v) => (true, v),
                    ("!", v) => (false, v),
                    _ => return Err("until: =V or !V".into()),
                };
                let v: u8 = v.parse().map_err(|_| "until: V 0..255")?;
                let limit = parts.next().and_then(parse_count).unwrap_or(50_000_000);
                let step = parts.next().and_then(parse_count).unwrap_or(100_000);
                let mut done = 0;
                loop {
                    let b = self.vice.mon.mem_read(a, a, true).map_err(e)?[0];
                    if (b == v) == want_eq {
                        return Ok(format!(
                            "until ${a:04x} {cond}: {b} after {done} (clock {})",
                            self.clock
                        ));
                    }
                    if done >= limit {
                        return Ok(format!(
                            "until ${a:04x} {cond}: TIMEOUT, {b} after {done} (clock {})",
                            self.clock
                        ));
                    }
                    self.run(step).map_err(e)?;
                    done += step;
                }
            }
            "typemem" => {
                // typemem HEXADDR TEXT: one key at a time, fed again (up to 8
                // times, 1M apart) until the byte at HEXADDR changes, as name
                // entry's cursor does when it takes a key (10.23).
                let (a, text) = arg.split_once(' ').ok_or("typemem HEXADDR TEXT")?;
                let a = parse_addr(a).ok_or("typemem: bad address")?;
                let mut notes = Vec::new();
                for code in petscii(text) {
                    let before = self.vice.mon.mem_read(a, a, true).map_err(e)?[0];
                    let mut taken = None;
                    'feeds: for attempt in 1..=8 {
                        self.vice.mon.keyboard_feed(&[code]).map_err(e)?;
                        for _ in 0..10 {
                            self.run(100_000).map_err(e)?;
                            let now = self.vice.mon.mem_read(a, a, true).map_err(e)?[0];
                            if now != before {
                                taken = Some((attempt, now));
                                break 'feeds;
                            }
                        }
                    }
                    notes.push(match taken {
                        Some((n, now)) => format!("{code}:{n}>{now}"),
                        None => format!("{code}:none"),
                    });
                }
                Ok(format!("typemem {}", notes.join(" ")))
            }
            "poke" => {
                // poke HEXADDR BYTE..: set memory (a researcher-given
                // address, docs/capture.md section 9), e.g. a player's stat.
                let mut parts = arg.split_whitespace();
                let a = parts
                    .next()
                    .and_then(|t| u16::from_str_radix(t.trim_start_matches('$'), 16).ok())
                    .ok_or("poke HEXADDR BYTE..")?;
                let bytes: Vec<u8> = parts
                    .map(|t| t.parse::<u8>().map_err(|_| "poke: bytes 0..255".to_string()))
                    .collect::<std::result::Result<_, String>>()?;
                if bytes.is_empty() {
                    return Err("poke HEXADDR BYTE..".into());
                }
                self.vice.mon.mem_set(a, &bytes).map_err(e)?;
                Ok(format!("${a:04x} <- {bytes:?}"))
            }
            "dpeek" => {
                // dpeek HEXADDR [COUNT]: drive 8's own memory (decimal bytes).
                let mut parts = arg.split_whitespace();
                let a = parts
                    .next()
                    .and_then(|t| u16::from_str_radix(t.trim_start_matches('$'), 16).ok())
                    .ok_or("dpeek HEXADDR [COUNT]")?;
                let n: u16 = parts
                    .next()
                    .map(|t| t.parse().map_err(|_| "dpeek: bad count"))
                    .transpose()?
                    .unwrap_or(1);
                let end = a.saturating_add(n.max(1) - 1);
                let b = self
                    .vice
                    .mon
                    .mem_read_space(a, end, false, MEMSPACE_DRIVE8)
                    .map_err(e)?;
                let v: Vec<String> = b.iter().map(|x| x.to_string()).collect();
                Ok(format!("drive ${a:04x} = {}", v.join(" ")))
            }
            "drivecheck" => {
                // drivecheck [SAMPLES [GAP]]: is drive 8 serving its timer
                // interrupt (Session::drive_alive)? 30 x 500 by default.
                let mut parts = arg.split_whitespace();
                let samples: usize = parts.next().and_then(|t| t.parse().ok()).unwrap_or(30);
                let gap = parts.next().and_then(parse_count).unwrap_or(500);
                let (alive, seen) = self.drive_alive(samples, gap).map_err(e)?;
                let clear = seen.iter().filter(|b| *b & 0x40 == 0).count();
                Ok(format!(
                    "drive 8 {}: timer flag clear in {clear} of {}",
                    if alive { "alive" } else { "DEAD" },
                    seen.len()
                ))
            }
            "print" => Ok(arg.to_string()),
            other => Err(format!("unknown command {other}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_passed_uses_the_two_cycle_floor() {
        const F: u64 = PAL_FRAME_CYCLES;
        // 2,000 instructions, 8,000 cycles, no wrap and one wrap.
        assert_eq!(frames_passed(100, 8_100, 2_000), 0);
        assert_eq!(frames_passed(F - 100, 7_900, 2_000), 1);
        // Seems to take 1,000 cycles, under 2 per instruction: once more
        // round the frame.
        assert_eq!(frames_passed(100, 1_100, 2_000), 1);
        assert_eq!(frames_passed(F - 500, 500, 2_000), 2);
    }

    #[test]
    fn counts_and_inputs() {
        assert_eq!(parse_count("2m"), Some(2_000_000));
        assert_eq!(parse_count("40k"), Some(40_000));
        assert_eq!(parse_count("17"), Some(17));
        assert_eq!(joy_bits("ne"), Some(9));
        assert_eq!(joy_bits("fire"), Some(16));
        assert_eq!(joy_bits("w+fire"), Some(20));
        assert_eq!(petscii("t{RETURN}"), vec![b'T', 13]);
        assert_eq!(petscii("{F7}"), vec![136]);
        assert_eq!(parse_addr("$4F4F"), Some(0x4F4F));
        assert_eq!(parse_addr("f59f"), Some(0xF59F));
        assert_eq!(parse_addr("x"), None);
    }
}
