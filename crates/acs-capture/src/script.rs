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
//! ```

use crate::monitor::Result;
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
    lin_id: Option<u8>,
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
            lin_id: None,
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

    /// Steps `n` instructions in small chunks and counts the frames that
    /// pass (raster wraparounds), so instruction counts can be turned into
    /// emulated time (PAL: 50.125 frames per second).
    pub fn run_counting_frames(&mut self, n: u64) -> Result<u64> {
        const CHUNK: u64 = 1_000;
        let mut frames = 0;
        let mut last = self.raster()?;
        let mut done = 0;
        while done < n {
            let c = CHUNK.min(n - done);
            self.run(c)?;
            done += c;
            let now = self.raster()?;
            if now < last {
                frames += 1;
            }
            last = now;
        }
        Ok(frames)
    }

    /// The VIC-II raster line, from the monitor's register list.
    pub fn raster(&mut self) -> Result<u16> {
        if self.lin_id.is_none() {
            let names = self.vice.mon.registers_available()?;
            self.lin_id = names.iter().find(|(_, n)| n == "LIN").map(|(i, _)| *i);
        }
        let id = self.lin_id.unwrap_or(0xFF);
        let regs = self.vice.mon.registers()?;
        Ok(regs.iter().find(|(i, _)| *i == id).map(|(_, v)| *v).unwrap_or(0))
    }

    pub fn shot(&mut self, name: &str) -> Result<PathBuf> {
        let f = self.frame()?;
        let p = self.out.join(format!("{name}.png"));
        f.save_png(&p, false, 2)?;
        Ok(p)
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
                // Documented hardware registers only (docs/capture.md section 8).
                let a = u16::from_str_radix(arg.trim_start_matches('$'), 16).map_err(|_| "peek HEXADDR")?;
                let b = self.vice.mon.mem_read(a, a, true).map_err(e)?;
                Ok(format!("${a:04x} = {:08b}", b.first().copied().unwrap_or(0)))
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
    fn counts_and_inputs() {
        assert_eq!(parse_count("2m"), Some(2_000_000));
        assert_eq!(parse_count("40k"), Some(40_000));
        assert_eq!(parse_count("17"), Some(17));
        assert_eq!(joy_bits("ne"), Some(9));
        assert_eq!(joy_bits("fire"), Some(16));
        assert_eq!(joy_bits("w+fire"), Some(20));
        assert_eq!(petscii("t{RETURN}"), vec![b'T', 13]);
        assert_eq!(petscii("{F7}"), vec![136]);
    }
}
