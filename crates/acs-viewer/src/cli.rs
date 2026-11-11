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
}

pub const USAGE: &str = "usage: acs-viewer [FILE] [--view walk|world|region:R|room:R:N|things|creatures|messages|diagnostics] \
[--size WxH] [--screenshot OUT.png] [--seed N] [--walk n,ne,e,...,wait]\n  R is the region number as listed (1-based), N the room slot (0-based).\n  --seed and --walk open walk mode and play the steps first.";

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
            "-h" | "--help" => return Err(USAGE.into()),
            f if f.starts_with("--") => return Err(format!("unknown option {f}\n{USAGE}")),
            _ if out.file.is_none() => out.file = Some(PathBuf::from(a)),
            _ => return Err(format!("only one file at a time\n{USAGE}")),
        }
    }
    Ok(out)
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
