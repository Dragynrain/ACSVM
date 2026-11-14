// SPDX-License-Identifier: GPL-3.0-or-later

//! Desktop command line: `acs-viewer [FILE] [--view V] [--size WxH] [--screenshot OUT.png]`.
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
    pub screenshot: Option<PathBuf>,
    /// Walk mode with this seed (`--seed`) and these steps (`--walk`).
    pub seed: Option<u64>,
    pub walk: Option<Vec<acs_engine::Command>>,
    /// Walk mode starting in a room square (`--place R:N:X:Y`), a tool
    /// shortcut that is not part of the replayable command log.
    pub place: Option<acs_engine::Location>,
}

pub const USAGE: &str = "usage: acs-viewer [FILE] [--view walk|world|region:R|room:R:N|things|creatures|messages|diagnostics] \
[--size WxH] [--screenshot OUT.png] [--seed N] [--place R:N:X:Y] [--walk n,ne,e,...,wait,cast:T[:me|pN|cS],use:T,add[:NAME]]\n  R is the region number as listed (1-based), N the room slot (0-based).\n  --seed, --place and --walk open walk mode; --place puts the player on that square first\n  (a tool shortcut, not a replayable command), then the steps are played.";

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
            "--screenshot" => out.screenshot = Some(PathBuf::from(value("--screenshot")?)),
            "--seed" => {
                let v = value("--seed")?;
                out.seed = Some(v.parse().map_err(|_| format!("bad seed {v:?}"))?);
            }
            "--walk" => out.walk = Some(crate::views::walk::parse_commands(&value("--walk")?)?),
            "--place" => out.place = Some(parse_place(&value("--place")?)?),
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
        ["walk"] => View::Walk,
        ["world"] => View::World,
        ["region", r] => View::Region(num(r)?.checked_sub(1).ok_or("regions start at 1")?),
        ["room", r, n] => View::Room(num(r)?.checked_sub(1).ok_or("regions start at 1")?, num(n)?),
        ["things"] => View::Things,
        ["creatures"] => View::Creatures,
        ["messages"] => View::Messages,
        ["diagnostics"] => View::Diagnostics,
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
                acs_engine::Command::CreatePlayer { name } => name.as_str(),
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
        assert!(p(&["a.d64", "b.d64"]).is_err());
        assert!(p(&["--frobnicate"]).is_err());
    }
}
