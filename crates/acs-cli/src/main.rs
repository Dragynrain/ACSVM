// SPDX-License-Identifier: GPL-3.0-or-later

//! `acs`: the adventure inspector.

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
}

#[derive(Subcommand)]
enum Command {
    /// Print a summary of an adventure and write it as JSON.
    Dump {
        /// Adventure file: .d64 (C64) or ADVEN.FPY / ADVEN.HRD (PC).
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
    let platform = platform.map(|p| match p {
        PlatformArg::Pc => Platform::Pc,
        PlatformArg::C64 => Platform::C64,
    });
    let image = Image::open(file, platform)?;
    let decoded = decode(&image)?;
    for d in &decoded.diagnostics {
        eprintln!("{d}");
    }
    Ok(decoded)
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
