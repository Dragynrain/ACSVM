// SPDX-License-Identifier: GPL-3.0-or-later

//! `acs`: the adventure inspector, and `acs play`, a scripted playthrough.

mod play;
mod script;

// acs-core's fixture builder for the play tests (as acs-engine's unit tests).
#[cfg(test)]
mod play_tests;
#[cfg(test)]
#[path = "../../acs-core/tests/common/mod.rs"]
mod test_fixture;

use acs_core::model::{Adventure, Destination, ThingKind};
use acs_core::{decode, Decoded, Image, Platform, Severity};
use clap::{Parser, Subcommand, ValueEnum};
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "acs", version, about = env!("CARGO_PKG_DESCRIPTION"))]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Copy, ValueEnum)]
enum PlatformArg {
    Pc,
    C64,
    Amiga,
}

#[derive(Subcommand)]
enum Command {
    /// Print a summary of an adventure and write it as JSON.
    Dump {
        /// Adventure file: .d64 (C64), ADVEN.FPY / ADVEN.HRD (PC), `adventure`
        /// or an .adf disk (Amiga), or an .advx bundle.
        file: PathBuf,
        /// JSON output path (default: <file stem>.acs.json next to the input; "-" for stdout).
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Override platform detection.
        #[arg(long, value_enum)]
        platform: Option<PlatformArg>,
        /// Print every region, room, thing and creature, not just counts.
        #[arg(short, long)]
        verbose: bool,
    },
    /// Write every picture as a PNG (C64 pixels shown double wide).
    Pictures {
        file: PathBuf,
        outdir: PathBuf,
        /// Integer upscale factor.
        #[arg(short, long, default_value_t = 1)]
        scale: u32,
        #[arg(long, value_enum)]
        platform: Option<PlatformArg>,
        /// Also write a contact sheet of all pictures (sheet.png).
        #[arg(long)]
        sheet: bool,
    },
    /// Write an Extended bundle (.advx) from model JSON (docs/content.md 2).
    /// A stopgap until `acs build`: the model must already be Extended.
    Bundle {
        /// The model, as `dump` writes it.
        model: PathBuf,
        /// The bundle to write.
        #[arg(short, long)]
        output: PathBuf,
        /// Licence named in the manifest.
        #[arg(long, default_value = "CC BY-SA 4.0")]
        license: String,
        /// Native HD art: an atlas whose place N is picture N.
        #[arg(long)]
        art: Option<PathBuf>,
        /// The atlas's tile size in pixels.
        #[arg(long, default_value_t = 64)]
        tile: u32,
    },
    /// Build an adventure's source folder into a bundle (docs/content.md 3).
    Build {
        /// The source folder (adventure.toml, things.toml, ...).
        dir: PathBuf,
        /// The bundle to write.
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Copy the adventure file off an Amiga .adf disk, checking it decodes.
    Extract {
        /// The .adf disk image.
        disk: PathBuf,
        /// Where to write it (default: `adventure` in a folder named after
        /// the disk, next to it, where the corpus tests find it).
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Play an adventure with a scripted walkthrough (docs/play.md).
    Play {
        /// Adventure file, as for dump.
        file: PathBuf,
        /// The walkthrough script.
        #[arg(long)]
        script: PathBuf,
        /// The game's seed.
        #[arg(long)]
        seed: u64,
        /// Players in the game, 1 to 4 (at least those the script's header names).
        #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u8).range(1..=4))]
        players: u8,
        /// Save file written at the end (also after a failure).
        #[arg(long)]
        save: PathBuf,
        /// Trace: each step, its commands and their events, one JSON a line.
        #[arg(long)]
        trace: Option<PathBuf>,
        #[arg(long, value_enum)]
        platform: Option<PlatformArg>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("acs: {e}");
            ExitCode::FAILURE
        }
    }
}

type AnyResult<T> = Result<T, Box<dyn std::error::Error>>;

fn load(file: &Path, platform: Option<PlatformArg>) -> AnyResult<Decoded> {
    let name = file.to_string_lossy();
    if name
        .to_ascii_lowercase()
        .ends_with(&format!(".{}", acs_bundle::EXTENSION))
    {
        let bundle = acs_bundle::read(&std::fs::read(file)?)?;
        return Ok(Decoded {
            adventure: bundle.adventure,
            diagnostics: Vec::new(),
        });
    }
    let platform = platform.map(|p| match p {
        PlatformArg::Pc => Platform::Pc,
        PlatformArg::C64 => Platform::C64,
        PlatformArg::Amiga => Platform::Amiga,
    });
    let image = Image::open(file, platform)?;
    let decoded = decode(&image)?;
    for d in &decoded.diagnostics {
        eprintln!("{d}");
    }
    Ok(decoded)
}

/// The adventure file on an Amiga disk image, and what it decodes to: a
/// file that doesn't decode is refused rather than written.
fn extract(disk: &[u8]) -> AnyResult<(Vec<u8>, Adventure)> {
    if !acs_core::adf::is_adf(disk) {
        return Err("not an Amiga .adf disk image".into());
    }
    let bytes = acs_core::adf::adventure(disk)?;
    let decoded = decode(&Image::from_bytes(Platform::Amiga, bytes.clone())?)?;
    if let Some(e) = decoded.errors().next() {
        return Err(format!("the adventure on this disk doesn't decode cleanly: {e}").into());
    }
    Ok((bytes, decoded.adventure))
}

fn run(cli: Cli) -> AnyResult<ExitCode> {
    match cli.command {
        Command::Dump {
            file,
            output,
            platform,
            verbose,
        } => {
            let decoded = load(&file, platform)?;
            print_summary(&decoded.adventure, verbose);
            let out = output.unwrap_or_else(|| {
                let stem = file.file_stem().unwrap_or_default().to_string_lossy();
                file.with_file_name(format!("{stem}.acs.json"))
            });
            if out.as_os_str() == "-" {
                serde_json::to_writer_pretty(std::io::stdout().lock(), &decoded.adventure)?;
                println!();
            } else {
                serde_json::to_writer_pretty(BufWriter::new(File::create(&out)?), &decoded.adventure)?;
                eprintln!("wrote {}", out.display());
            }
            Ok(exit_for(&decoded))
        }
        Command::Pictures {
            file,
            outdir,
            scale,
            platform,
            sheet,
        } => {
            let decoded = load(&file, platform)?;
            let adv = &decoded.adventure;
            std::fs::create_dir_all(&outdir)?;
            let scale = scale.max(1);
            for (n, pic) in adv.pictures.iter().enumerate() {
                let (w, h) = pic.display_size();
                let rgba = upscale(&pic.to_rgba(&adv.palette), w, h, scale);
                write_png(
                    &outdir.join(format!("picture-{n:03}.png")),
                    w * scale,
                    h * scale,
                    &rgba,
                )?;
            }
            if sheet {
                write_sheet(adv, &outdir.join("sheet.png"), scale)?;
            }
            println!("wrote {} pictures to {}", adv.pictures.len(), outdir.display());
            Ok(exit_for(&decoded))
        }
        Command::Bundle {
            model,
            output,
            license,
            art,
            tile,
        } => {
            let adventure: Adventure = serde_json::from_slice(&std::fs::read(&model)?)?;
            let art = match art {
                Some(path) => Some(read_atlas(&path, tile, adventure.pictures.len())?),
                None => None,
            };
            let bundle = acs_bundle::Bundle {
                manifest: acs_bundle::Manifest::new(&adventure, &license),
                adventure,
                art,
            };
            let bytes = acs_bundle::write(&bundle)?;
            // Read it back: the same checks any player's copy gets.
            acs_bundle::read(&bytes)?;
            std::fs::write(&output, bytes)?;
            eprintln!("wrote {}", output.display());
            Ok(ExitCode::SUCCESS)
        }
        Command::Extract { disk, output } => {
            let (bytes, adventure) = extract(&std::fs::read(&disk)?)?;
            let out = output.unwrap_or_else(|| {
                let stem = disk.file_stem().unwrap_or_default().to_string_lossy();
                disk.with_file_name(stem.as_ref()).join("adventure")
            });
            if let Some(dir) = out.parent().filter(|d| !d.as_os_str().is_empty()) {
                std::fs::create_dir_all(dir)?;
            }
            std::fs::write(&out, &bytes)?;
            eprintln!(
                "wrote {}: {} ({} bytes)",
                out.display(),
                adventure.name.trim(),
                bytes.len()
            );
            Ok(ExitCode::SUCCESS)
        }
        Command::Build { dir, output } => {
            let folder = acs_build::Folder::read(&dir)?;
            let bundle = match acs_build::build(&folder) {
                Ok(b) => b,
                Err(problems) => {
                    for p in &problems {
                        eprintln!("{p}");
                    }
                    return Err(format!("{} problems; nothing written", problems.len()).into());
                }
            };
            let bytes = acs_bundle::write(&bundle)?;
            acs_bundle::read(&bytes)?;
            std::fs::write(&output, bytes)?;
            let a = &bundle.adventure;
            eprintln!(
                "wrote {}: {} regions, {} things, {} creatures, {} pictures",
                output.display(),
                a.regions.len(),
                a.things
                    .iter()
                    .filter(|t| !matches!(t.kind, ThingKind::Undefined))
                    .count(),
                a.creatures.len(),
                a.pictures.len()
            );
            Ok(ExitCode::SUCCESS)
        }
        Command::Play {
            file,
            script,
            seed,
            players,
            save,
            trace,
            platform,
        } => {
            let decoded = load(&file, platform)?;
            let text = std::fs::read_to_string(&script)?;
            let script = script::parse(&text, &decoded.adventure)?;
            let out = play::play(&decoded.adventure, &script, seed, players.into());
            for line in &out.progress {
                println!("{line}");
            }
            std::fs::write(&save, out.save.to_json())?;
            eprintln!("wrote {}", save.display());
            if let Some(t) = trace {
                let mut body = out.trace.join("\n");
                body.push('\n');
                std::fs::write(&t, body)?;
                eprintln!("wrote {}", t.display());
            }
            let p = out.game.players();
            eprintln!(
                "{} commands played, round {}, {} of {} players alive",
                out.game.log().len(),
                out.game.round(),
                p.iter().filter(|p| p.alive).count(),
                p.len()
            );
            match out.failure {
                Some(f) => {
                    eprintln!("{f}");
                    Ok(ExitCode::FAILURE)
                }
                None => Ok(ExitCode::SUCCESS),
            }
        }
    }
}

fn exit_for(d: &Decoded) -> ExitCode {
    if d.diagnostics.iter().any(|x| x.severity == Severity::Error) {
        ExitCode::from(2)
    } else {
        ExitCode::SUCCESS
    }
}

fn upscale(rgba: &[u8], w: u32, h: u32, s: u32) -> Vec<u8> {
    if s == 1 {
        return rgba.to_vec();
    }
    let mut out = Vec::with_capacity((w * h * s * s * 4) as usize);
    for y in 0..h {
        let row: Vec<u8> = (0..w)
            .flat_map(|x| {
                let i = ((y * w + x) * 4) as usize;
                std::iter::repeat_n(&rgba[i..i + 4], s as usize)
                    .flatten()
                    .copied()
            })
            .collect();
        for _ in 0..s {
            out.extend_from_slice(&row);
        }
    }
    out
}

fn write_png(path: &Path, w: u32, h: u32, rgba: &[u8]) -> AnyResult<()> {
    let mut enc = png::Encoder::new(BufWriter::new(File::create(path)?), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()?.write_image_data(rgba)?;
    Ok(())
}

/// All pictures on one sheet, 16 per row, with a 1-pixel grey grid.
fn write_sheet(adv: &Adventure, path: &Path, scale: u32) -> AnyResult<()> {
    const COLS: u32 = 16;
    let (tw, th) = adv.pictures.first().map(|p| p.display_size()).unwrap_or((16, 16));
    let (tw, th) = (tw * scale, th * scale);
    let rows = (adv.pictures.len() as u32).div_ceil(COLS);
    let (w, h) = (COLS * (tw + 1) + 1, rows * (th + 1) + 1);
    let mut img = vec![0x80u8; (w * h * 4) as usize];
    for (n, pic) in adv.pictures.iter().enumerate() {
        let (pw, ph) = pic.display_size();
        let tile = upscale(&pic.to_rgba(&adv.palette), pw, ph, scale);
        let (ox, oy) = ((n as u32 % COLS) * (tw + 1) + 1, (n as u32 / COLS) * (th + 1) + 1);
        for y in 0..th {
            let src = (y * tw * 4) as usize;
            let dst = (((oy + y) * w + ox) * 4) as usize;
            img[dst..dst + (tw * 4) as usize].copy_from_slice(&tile[src..src + (tw * 4) as usize]);
        }
    }
    write_png(path, w, h, &img)
}

fn print_summary(a: &Adventure, verbose: bool) {
    println!("{} ({})", a.name, a.platform);
    println!("by {}", a.byline);
    println!();
    for line in a.intro.lines() {
        println!("  {line}");
    }
    println!();
    let palette: Vec<&str> = a.palette.colors.iter().map(|c| c.name.as_str()).collect();
    println!("palette: {}", palette.join(", "));
    println!("theme: {:?} (music {})", a.theme, a.theme.music_number());

    let things: Vec<_> = a
        .things
        .iter()
        .filter(|t| !matches!(t.kind, ThingKind::Undefined))
        .collect();
    let creatures: Vec<_> = a.creatures.iter().filter(|c| c.in_use).collect();
    let rooms: usize = a
        .regions
        .iter()
        .map(|r| r.rooms.iter().filter(|r| !r.deleted).count())
        .sum();
    let items: usize = a
        .regions
        .iter()
        .flat_map(|r| &r.rooms)
        .map(|r| r.items.len())
        .sum();
    println!(
        "{} regions, {rooms} rooms, {items} room items, {} things, {} creatures, {} pictures",
        a.regions.len(),
        things.len(),
        creatures.len(),
        a.pictures.len()
    );
    println!(
        "world map {:?}: start {},{}, {} doors, wrap {}",
        a.world.name,
        a.world.start.x,
        a.world.start.y,
        a.world.doors.len(),
        if a.world.wraps { "on" } else { "off" }
    );
    println!(
        "players: {}, retired: {}",
        a.players.len(),
        a.retired_players.len()
    );

    if !verbose {
        return;
    }
    println!("\nregions:");
    for r in &a.regions {
        println!("  {:2} {}", r.number, r.name);
        for room in r.rooms.iter().filter(|r| !r.deleted) {
            let doors = room
                .items
                .iter()
                .filter_map(|i| i.destination)
                .map(|d| match d {
                    Destination::WorldMap { x, y } => format!("map {x},{y}"),
                    Destination::Room { region, room, .. } => format!("r{region}/{room}"),
                })
                .collect::<Vec<_>>();
            println!(
                "      {:2} {:15} {:2}x{:<2} items {:2}{}",
                room.number,
                room.name,
                room.width,
                room.height,
                room.items.len(),
                if doors.is_empty() {
                    String::new()
                } else {
                    format!("  doors: {}", doors.join(" "))
                }
            );
        }
    }
    println!("\nthings:");
    for t in &things {
        println!("  {:3} {:15} {}", t.number, t.name, t.kind.type_name());
    }
    println!("\ncreatures:");
    for c in &creatures {
        let class = c
            .class
            .and_then(|k| a.creature_classes.get(k as usize))
            .map(String::as_str)
            .unwrap_or("");
        println!(
            "  {:3} {:15} {:15} {:?} {} {}  CON {:2} STR {:2} DEX {:2} SPD {:2} WIS {:2} POW {:3}",
            c.number,
            c.name,
            class,
            c.strategy.alignment,
            if c.strategy.aggressive {
                "aggressive"
            } else {
                "peaceful"
            },
            if c.strategy.brave { "brave" } else { "cautious" },
            c.constitution,
            c.strength,
            c.dexterity,
            c.speed,
            c.wisdom,
            c.power
        );
    }
}

/// An RGBA atlas for `acs bundle --art`: place N holds picture N, for every
/// place up to the adventure's picture count.
fn read_atlas(path: &Path, tile: u32, pictures: usize) -> AnyResult<acs_bundle::Art> {
    let mut decoder = png::Decoder::new(std::io::BufReader::new(File::open(path)?));
    decoder.set_transformations(png::Transformations::normalize_to_color8() | png::Transformations::ALPHA);
    let mut reader = decoder.read_info()?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut buf)?;
    if frame.color_type != png::ColorType::Rgba {
        return Err(format!(
            "{}: colour type {:?}, need RGBA",
            path.display(),
            frame.color_type
        )
        .into());
    }
    if tile == 0 || frame.width % tile != 0 || frame.height % tile != 0 {
        return Err(format!(
            "tile {tile} doesn't divide the {} x {} atlas",
            frame.width, frame.height
        )
        .into());
    }
    let places = (frame.width / tile) * (frame.height / tile);
    buf.truncate(frame.buffer_size());
    Ok(acs_bundle::Art {
        tile,
        width: frame.width,
        height: frame.height,
        rgba: buf,
        places: (0..places.min(pictures as u32)).map(|n| (n as u16, n)).collect(),
    })
}

#[cfg(test)]
mod extract_tests {
    use super::*;

    #[test]
    fn extract_takes_the_adventure_off_a_disk() {
        let mut f = test_fixture::Fixture::new(Platform::Amiga);
        f.header("DISK QUEST", "BY NOBODY", "HELLO");
        let file = f.build().data;
        for ffs in [false, true] {
            let disk = acs_core::adf::write_test_disk(&[("readme", b"x"), ("Adventure", &file)], ffs);
            let (bytes, adv) = extract(&disk).unwrap();
            assert_eq!(bytes, file);
            assert_eq!(adv.name.trim(), "DISK QUEST");
        }
        let empty = acs_core::adf::write_test_disk(&[("readme", b"x")], false);
        assert!(extract(&empty).is_err());
        assert!(extract(&file).is_err(), "a bare adventure file is not a disk");
    }
}
