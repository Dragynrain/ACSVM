// SPDX-License-Identifier: GPL-3.0-or-later

//! Desktop command line: `acs-viewer [FILE] [--view V] [--size WxH] [--ppp F] [--pad] [--screenshot OUT.png]`.
//!
//! `--screenshot` renders a few frames, writes the window as a PNG and exits;
//! it is for documentation and checking the renderer at known window sizes.
//! Screenshots of real adventures are derived from them: keep them outside
//! the repository.

use crate::app::View;
use std::path::PathBuf;

#[derive(Debug, Default, PartialEq)]
pub struct Args {
    pub file: Option<PathBuf>,
    pub view: Option<View>,
    /// Window inner size in points.
    pub size: Option<[f32; 2]>,
    /// Pixels per point to render at (`--ppp`), instead of the screen's
    /// scale: a size too big for this screen at its scale still fits, and
    /// the layout, which depends only on points, is the same.
    pub ppp: Option<f32>,
    /// Play mode with the on-screen pad shown (`--pad`).
    pub pad: bool,
    /// Play mode with Show more details on (`--details`, client 3).
    pub details: bool,
    /// An HD pack to load (`--pack FILE.png`).
    pub pack: Option<PathBuf>,
    /// Text size in percent (`--text-size`) and the colour-blind palette
    /// (`--colour-blind`), client 7.
    pub text_size: Option<f32>,
    pub colour_blind: bool,
    /// The render mode to start in (`--render original|smoothed|hd|native`).
    pub render: Option<crate::tiles::RenderMode>,
    pub screenshot: Option<PathBuf>,
    /// Play mode with this seed (`--seed`) and these steps (`--walk`).
    pub seed: Option<u64>,
    pub walk: Option<Vec<acs_engine::Command>>,
    /// Play mode starting in a room square (`--place R:N:X:Y`), a tool
    /// shortcut that is not part of the replayable command log.
    pub place: Option<acs_engine::Location>,
    /// A save file to load into play mode (`--load`), and where to write
    /// the walk's save once its steps are played (`--save`).
    pub load: Option<PathBuf>,
    pub save: Option<PathBuf>,
}

pub const USAGE: &str = "usage: acs-viewer [FILE] [--view play|world|region:R|room:R:N|things|creatures|messages|diagnostics|pack] \
[--size WxH] [--ppp F] [--pad] [--details] [--render original|smoothed|hd|native] [--pack PACK.png] [--text-size PCT] [--colour-blind] [--screenshot OUT.png] [--seed N] [--place R:N:X:Y] [--walk n,ne,e,...,wait,cast:T[:me|pN|cS],use:T,add[:NAME]] [--save OUT.json] [--load SAVE.json]\n  R is the region number as listed (1-based), N the room slot (0-based).\n  --view play opens the New character screen; --seed, --place and --walk start a game; --place puts the player on that square first\n  (a tool shortcut, not a replayable command), then the steps are played.\n  --save writes the game's save file after the steps; --load opens a save over FILE (docs/saves.md).";

pub fn parse(args: impl IntoIterator<Item = std::ffi::OsString>) -> Result<Args, String> {
    let mut out = Args::default();
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        let s = a.to_string_lossy().into_owned();
        let mut value = |name: &str| -> Result<String, String> {
            it.next()
                .map(|v| v.to_string_lossy().into_owned())
                .ok_or_else(|| format!("{name} needs a value"))
        };
        match s.as_str() {
            "--view" => out.view = Some(parse_view(&value("--view")?)?),
            "--size" => out.size = Some(parse_size(&value("--size")?)?),
            "--ppp" => {
                let v = value("--ppp")?;
                out.ppp = Some(
                    v.parse()
                        .ok()
                        .filter(|p: &f32| *p >= 0.5 && *p <= 4.0)
                        .ok_or(format!("bad --ppp {v:?} (0.5..4)"))?,
                );
            }
            "--pad" => out.pad = true,
            "--details" => out.details = true,
            "--pack" => out.pack = Some(PathBuf::from(value("--pack")?)),
            "--colour-blind" => out.colour_blind = true,
            "--text-size" => {
                let v = value("--text-size")?;
                let pct: f32 = v.parse().map_err(|_| format!("bad --text-size {v:?}"))?;
                if !(75.0..=200.0).contains(&pct) {
                    return Err(format!("--text-size {v} is outside 75..200"));
                }
                out.text_size = Some(pct / 100.0);
            }
            "--render" => {
                let v = value("--render")?;
                out.render = Some(
                    crate::tiles::RenderMode::parse(&v)
                        .ok_or(format!("unknown render mode {v:?} (original, smoothed, hd)"))?,
                );
            }
            "--screenshot" => out.screenshot = Some(PathBuf::from(value("--screenshot")?)),
            "--seed" => {
                let v = value("--seed")?;
                out.seed = Some(v.parse().map_err(|_| format!("bad seed {v:?}"))?);
            }
            "--walk" => out.walk = Some(crate::views::walk::parse_commands(&value("--walk")?)?),
            "--place" => out.place = Some(parse_place(&value("--place")?)?),
            "--load" => out.load = Some(PathBuf::from(value("--load")?)),
            "--save" => out.save = Some(PathBuf::from(value("--save")?)),
            "-h" | "--help" => return Err(USAGE.into()),
            f if f.starts_with("--") => return Err(format!("unknown option {f}\n{USAGE}")),
            _ if out.file.is_none() => out.file = Some(PathBuf::from(a)),
            _ => return Err(format!("only one file at a time\n{USAGE}")),
        }
    }
    Ok(out)
}

/// `R:N:X:Y`: region (1-based), room slot, square.
pub fn parse_place(s: &str) -> Result<acs_engine::Location, String> {
    let n: Vec<u8> = s
        .split(':')
        .map(|p| {
            p.parse::<u8>()
                .map_err(|_| format!("bad number {p:?} in --place {s}"))
        })
        .collect::<Result<_, _>>()?;
    match n.as_slice() {
        [region, room, x, y] if *region > 0 => Ok(acs_engine::Location::Room {
            region: *region,
            room: *room,
            x: *x,
            y: *y,
        }),
        _ => Err(format!("--place wants R:N:X:Y with R from 1, got {s:?}")),
    }
}

pub fn parse_view(s: &str) -> Result<View, String> {
    let parts: Vec<&str> = s.split(':').collect();
    let num = |p: &str| {
        p.parse::<usize>()
            .map_err(|_| format!("bad number {p:?} in --view {s}"))
    };
    Ok(match parts.as_slice() {
        // "walk" is the old name, still taken.
        ["play" | "walk"] => View::Walk,
        ["world"] => View::World,
        ["region", r] => View::Region(num(r)?.checked_sub(1).ok_or("regions start at 1")?),
        ["room", r, n] => View::Room(num(r)?.checked_sub(1).ok_or("regions start at 1")?, num(n)?),
        ["things"] => View::Things,
        ["creatures"] => View::Creatures,
        ["messages"] => View::Messages,
        ["diagnostics"] => View::Diagnostics,
        ["pack"] => View::Pack,
        _ => return Err(format!("unknown view {s:?}\n{USAGE}")),
    })
}

fn parse_size(s: &str) -> Result<[f32; 2], String> {
    let (w, h) = s
        .split_once('x')
        .ok_or_else(|| format!("size {s:?} is not WxH"))?;
    let n = |v: &str| v.parse::<f32>().map_err(|_| format!("size {s:?} is not WxH"));
    Ok([n(w)?, n(h)?])
}

/// Writes an RGBA image as PNG.
pub fn write_png(path: &std::path::Path, size: [usize; 2], rgba: &[u8]) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), size[0] as u32, size[1] as u32);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut w = enc.write_header().map_err(|e| e.to_string())?;
    w.write_image_data(rgba).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(args: &[&str]) -> Result<Args, String> {
        parse(args.iter().map(std::ffi::OsString::from))
    }

    #[test]
    fn parses_views_and_options() {
        let a = p(&[
            "GAME.D64",
            "--view",
            "room:2:5",
            "--size",
            "1280x800",
            "--screenshot",
            "x.png",
        ])
        .unwrap();
        assert_eq!(a.file, Some(PathBuf::from("GAME.D64")));
        assert_eq!(a.view, Some(View::Room(1, 5)));
        assert_eq!(a.size, Some([1280.0, 800.0]));
        assert_eq!(a.screenshot, Some(PathBuf::from("x.png")));
        assert_eq!(p(&[]).unwrap(), Args::default());
        assert_eq!(parse_view("region:1").unwrap(), View::Region(0));
        assert_eq!(parse_view("world").unwrap(), View::World);
        let w = p(&["--seed", "7", "--walk", "n, ne,wait"]).unwrap();
        assert_eq!(w.seed, Some(7));
        assert_eq!(w.walk.map(|c| c.len()), Some(3));
        assert!(p(&["--walk", "up"]).is_err());
        let w = p(&[
            "--place",
            "4:7:2:1",
            "--walk",
            "cast:33:c2,use:40,use:41:me,cast:9:p2",
        ])
        .unwrap();
        assert_eq!(
            w.place,
            Some(acs_engine::Location::Room {
                region: 4,
                room: 7,
                x: 2,
                y: 1
            })
        );
        assert_eq!(w.walk.map(|c| c.len()), Some(4));
        assert!(p(&["--place", "0:1:2:3"]).is_err());
        assert!(p(&["--walk", "cast:33:x9"]).is_err());
        let w = p(&["--walk", "add:two,add"]).unwrap().walk.unwrap();
        let names: Vec<_> = w
            .iter()
            .map(|c| match c {
                acs_engine::Command::CreatePlayer { name, .. } => name.as_str(),
                _ => "",
            })
            .collect();
        assert_eq!(names, ["TWO", "SOMEONE"]);
    }

    #[test]
    fn rejects_bad_input() {
        assert!(p(&["--view"]).is_err());
        assert!(p(&["--view", "region:0"]).is_err());
        assert!(p(&["--view", "attic"]).is_err());
        assert!(p(&["--size", "big"]).is_err());
        assert_eq!(p(&["--ppp", "1.5"]).unwrap().ppp, Some(1.5));
        assert!(p(&["--ppp", "9"]).is_err());
        assert!(p(&["--ppp", "x"]).is_err());
        assert_eq!(
            p(&["--render", "Smoothed"]).unwrap().render,
            Some(crate::tiles::RenderMode::Smoothed)
        );
        assert!(p(&["--render", "blurry"]).is_err());
        assert_eq!(p(&["--text-size", "150"]).unwrap().text_size, Some(1.5));
        assert!(p(&["--text-size", "500"]).is_err());
        assert!(p(&["--colour-blind"]).unwrap().colour_blind);
        assert!(p(&["--details"]).unwrap().details);
        assert!(p(&["a.d64", "b.d64"]).is_err());
        assert!(p(&["--frobnicate"]).is_err());
    }
}
