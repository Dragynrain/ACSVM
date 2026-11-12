// SPDX-License-Identifier: GPL-3.0-or-later

//! `acs-capture`: the behavior capture harness's command line.

use acs_capture::script::Session;
use acs_capture::timing::Stats;
use acs_capture::vice::{Options, Vice};
use acs_capture::{experiment, scenario};
use clap::{Parser, Subcommand};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Distils `OUT/runs.jsonl` into `OUT/summary.json` and prints it.
fn write_summary(out: &Path, speed: u8) {
    let summary = experiment::summarize(&out.join("runs.jsonl"), speed).expect("summary");
    let text = serde_json::to_string_pretty(&summary).unwrap_or_default();
    std::fs::write(out.join("summary.json"), &text).expect("summary.json");
    println!("{text}");
}

/// Per-run timing (docs/capture.md 2.7): one line per run in `timing.jsonl`
/// next to `runs.jsonl`, and a short line on stderr.
struct RunTimer {
    file: std::fs::File,
    started: Instant,
    at_start: Stats,
    run_started: Instant,
    before: Stats,
}

impl RunTimer {
    fn new(out: &Path, stats: &Stats) -> RunTimer {
        RunTimer {
            file: std::fs::File::create(out.join("timing.jsonl")).expect("timing.jsonl"),
            started: Instant::now(),
            at_start: stats.clone(),
            run_started: Instant::now(),
            before: stats.clone(),
        }
    }

    fn start(&mut self, stats: &Stats) {
        self.run_started = Instant::now();
        self.before = stats.clone();
    }

    fn done(&mut self, run: usize, stats: &Stats) {
        let wall = self.run_started.elapsed();
        let d = stats.since(&self.before);
        let mut j = d.to_json(wall);
        j["run"] = run.into();
        let _ = writeln!(self.file, "{j}");
        eprintln!(
            "run {run} done ({:.0}s): {}",
            self.started.elapsed().as_secs_f64(),
            d.line(wall)
        );
    }

    fn total(&mut self, stats: &Stats) {
        let wall = self.started.elapsed();
        let d = stats.since(&self.at_start);
        let mut j = d.to_json(wall);
        j["run"] = "total".into();
        let _ = writeln!(self.file, "{j}");
        eprintln!("all runs: {}", d.line(wall));
    }
}

#[derive(Parser)]
#[command(about = "Drive VICE over its binary monitor to measure the original program")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Write a scenario adventure disk (docs/capture.md section 7).
    Scenario {
        /// Scenario name: move-basic.
        name: String,
        /// Output .d64 (outside the repository).
        out: PathBuf,
        /// Blank adventure disk made by the original program, to write over
        /// (default: ACSVM_CAPTURE_TEMPLATE).
        #[arg(long)]
        template: Option<PathBuf>,
        /// Also write a retired player of ours with these stats (`key=value`
        /// pairs over move-basic's rolled player: con, lf, str, dex, spd,
        /// wis, pow, size, dodge, parry, melee, missile, name).
        #[arg(long)]
        player: Option<String>,
        /// Also put an obstacle on room square 5,4 (east of the entry).
        #[arg(long)]
        obstacle: bool,
        /// Treasures of these weights (comma-separated) in a row east of
        /// the room's entry square.
        #[arg(long, value_delimiter = ',')]
        weights: Vec<u16>,
        /// Also give the player a spell of ours: COST (shows a long
        /// message) or COST:ACTION:PARAM (numbers, 0x.. for hex). Repeat
        /// for more spells (things 80, 79, ..).
        #[arg(long)]
        spell: Vec<String>,
        /// Also give the player a magic item of ours that acts when used:
        /// ACTION:PARAM (10:2 shows a long message).
        #[arg(long)]
        item: Option<String>,
    },
    /// Run an experiment from a scenario snapshot (docs/capture.md 4).
    Run {
        /// Experiment: move-basic (world map) or move-room (the same pushes
        /// in the room behind the map door; the snapshot is taken there);
        /// refuse (docs/capture.md 10.2: from the first turn of a disk made
        /// with --obstacle and a fast player).
        name: String,
        /// Snapshot taken at the scenario's first player turn.
        #[arg(long)]
        snapshot: PathBuf,
        /// The scenario disk (attached to drive 8).
        #[arg(long)]
        disk: PathBuf,
        /// Output folder (outside the repository): runs.jsonl, summary.json.
        #[arg(long)]
        out: PathBuf,
        /// Repetitions, each with a different idle start (jitter).
        #[arg(long, default_value_t = 5)]
        runs: usize,
        /// Index of the first run (sets its jitter), to split a batch over
        /// several VICE sessions.
        #[arg(long, default_value_t = 0)]
        first: usize,
        /// Split the runs over this many VICE processes side by side, in
        /// OUT/part0.. on copies of the disk, and join their output.
        #[arg(long, default_value_t = 1)]
        jobs: usize,
        /// Jitter of run n: n x step mod range idle instructions before the
        /// run's input (docs/capture.md 4.3, 10.5).
        #[arg(long, default_value_t = 7_919)]
        jitter_step: u64,
        #[arg(long, default_value_t = 1_000_000)]
        jitter_range: u64,
        /// weight: how many things the row holds.
        #[arg(long, default_value_t = 9)]
        weight_items: usize,
        /// The player's Speed, for our engine's prediction in the summary.
        #[arg(long, default_value_t = 6)]
        speed: u8,
        /// drift: what to cast first (none, spell1, spell1-spell2).
        #[arg(long, default_value = "none")]
        plan: String,
        /// drift: turns to read after the casts.
        #[arg(long, default_value_t = 40)]
        turns: usize,
        /// The glyph table for reading the profile (default:
        /// ACSVM_CAPTURE_GLYPHS).
        #[arg(long)]
        glyphs: Option<PathBuf>,
    },
    /// Distil a runs.jsonl into summary.json.
    Summarize {
        runs: PathBuf,
        /// The player's Speed, for our engine's prediction.
        #[arg(long, default_value_t = 6)]
        speed: u8,
    },
    /// Learn or read screen text (docs/capture.md 5.3).
    Glyphs {
        /// learn (add new cells of the shots to the table, unlabelled) or
        /// read (print the shots as text; with --profile, the stats too).
        action: String,
        /// Screenshots of the inner screen (any integer scale).
        shots: Vec<PathBuf>,
        /// The glyph table (default: ACSVM_CAPTURE_GLYPHS); learn creates it.
        #[arg(long)]
        glyphs: Option<PathBuf>,
        /// read: also parse the profile screen's stats.
        #[arg(long)]
        profile: bool,
    },
    /// Run a line script (see `script.rs`) against a fresh VICE.
    Script {
        /// The script file, or `-` for stdin.
        script: PathBuf,
        /// Folder for screenshots and snapshots (outside the repository).
        #[arg(long)]
        out: PathBuf,
        /// Autostart this disk image.
        #[arg(long)]
        boot: Option<PathBuf>,
        /// Attach this disk image to drive 8 without booting it.
        #[arg(long)]
        disk: Option<PathBuf>,
        /// Restore this snapshot first.
        #[arg(long)]
        snapshot: Option<PathBuf>,
        /// Show VICE's window.
        #[arg(long)]
        visible: bool,
        /// NAME=VALUE: replaces `${NAME}` in the script.
        #[arg(long = "var")]
        vars: Vec<String>,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Scenario {
            name,
            out,
            template,
            player,
            obstacle,
            weights,
            spell,
            item,
        } => {
            let spells: Vec<scenario::Spell> = spell
                .iter()
                .map(|t| scenario::Spell::parse(t).unwrap_or_else(|e| fail(&e)))
                .collect();
            let item = item.map(|t| {
                let s = scenario::Spell::parse(&format!("0:{t}")).unwrap_or_else(|e| fail(&e));
                (s.action, s.param)
            });
            let template = template
                .or_else(|| std::env::var_os("ACSVM_CAPTURE_TEMPLATE").map(PathBuf::from))
                .map(|p| std::fs::read(&p).unwrap_or_else(|e| fail(&format!("{}: {e}", p.display()))));
            if template.is_none() {
                eprintln!("note: no template; the original program will refuse this disk");
            }
            let img = match name.as_str() {
                "move-basic" => {
                    let player =
                        player.map(|p| scenario::Player::default().with(&p).unwrap_or_else(|e| fail(&e)));
                    let s = scenario::MoveBasic {
                        player,
                        obstacle: obstacle.then_some((5, 4)),
                        weights: weights.clone(),
                        spells: spells.clone(),
                        item,
                        ..scenario::MoveBasic::default()
                    };
                    scenario::move_basic(&s, template.as_deref())
                }
                other => fail(&format!("unknown scenario {other}")),
            };
            std::fs::write(&out, &img.data).unwrap_or_else(|e| fail(&e.to_string()));
            println!("wrote {} ({} bytes)", out.display(), img.data.len());
        }
        Cmd::Run {
            name,
            snapshot,
            disk,
            out,
            runs,
            first,
            jobs,
            weight_items,
            jitter_step,
            jitter_range,
            speed,
            plan,
            turns,
            glyphs,
        } => {
            let plan = acs_capture::power::plan(&plan).unwrap_or_else(|e| fail(&e));
            let glyphs = glyphs
                .or_else(|| std::env::var_os("ACSVM_CAPTURE_GLYPHS").map(PathBuf::from))
                .map(|p| acs_capture::glyphs::Table::load(&p).unwrap_or_else(|e| fail(&e)));
            let place = match name.as_str() {
                "move-basic" | "refuse" | "weight" | "cast" | "profile-check" | "item" | "drift" => {
                    experiment::Place::Map
                }
                "move-room" => experiment::Place::Room,
                other => fail(&format!("unknown experiment {other}")),
            };
            std::fs::create_dir_all(&out).expect("out folder");
            if jobs > 1 {
                let args: Vec<String> = std::env::args().skip(1).collect();
                acs_capture::jobs::run(&args, &disk, &out, first, runs, jobs).unwrap_or_else(|e| fail(&e));
                write_summary(&out, speed);
                return;
            }
            let opts = Options {
                drive8: Some(disk),
                ..Options::default()
            };
            let vice = Vice::start(&opts).unwrap_or_else(|e| fail(&e.to_string()));
            let mut s = Session::new(vice, out.clone());
            let file = std::fs::File::create(out.join("runs.jsonl")).expect("runs.jsonl");
            let mut r = experiment::Runner {
                s: &mut s,
                snapshot: snapshot.display().to_string(),
                scen: scenario::MoveBasic::default(),
                place,
                out: file,
                glyphs,
            };
            let mut timer = RunTimer::new(&out, &r.s.vice.mon.stats);
            let mut result = Ok(());
            'outer: for run in first..first + runs {
                timer.start(&r.s.vice.mon.stats);
                // Jitter: a different number of idle instructions before each
                // run moves the machine's noise sources (docs/capture.md 4.3).
                let jitter = (run as u64 * jitter_step) % jitter_range;
                if matches!(name.as_str(), "profile-check" | "item" | "drift") {
                    result = match name.as_str() {
                        "item" => r.item_run(run, jitter),
                        "drift" => r.drift_run(run, jitter, &plan, turns),
                        _ => r.profile_check(run, jitter),
                    };
                    if result.is_err() {
                        break 'outer;
                    }
                    timer.done(run, &r.s.vice.mon.stats);
                    continue;
                }
                if name == "cast" {
                    result = r.cast_run(run, jitter);
                    if result.is_err() {
                        break 'outer;
                    }
                    timer.done(run, &r.s.vice.mon.stats);
                    continue;
                }
                if name == "weight" {
                    result = r.weight_run(run, jitter, weight_items);
                    if result.is_err() {
                        break 'outer;
                    }
                    timer.done(run, &r.s.vice.mon.stats);
                    continue;
                }
                if name == "refuse" {
                    let cases = &experiment::REFUSE;
                    for (case, acts) in cases.iter() {
                        result = r.refuse_case(run, jitter, case, acts, place);
                        if result.is_err() {
                            break 'outer;
                        }
                    }
                    timer.done(run, &r.s.vice.mon.stats);
                    continue;
                }
                for dir in experiment::DIRS {
                    result = r.direction(run, jitter, dir);
                    if result.is_err() {
                        break 'outer;
                    }
                }
                result = r
                    .steps_per_turn(run, jitter)
                    .and_then(|_| r.idle_drain(run, jitter));
                if result.is_err() {
                    break;
                }
                timer.done(run, &r.s.vice.mon.stats);
            }
            timer.total(&r.s.vice.mon.stats);
            drop(r);
            drop(s);
            if let Err(e) = result {
                fail(&e);
            }
            write_summary(&out, speed);
        }
        Cmd::Summarize { runs, speed } => {
            let summary = experiment::summarize(&runs, speed).unwrap_or_else(|e| fail(&e.to_string()));
            println!("{}", serde_json::to_string_pretty(&summary).unwrap_or_default());
        }
        Cmd::Glyphs {
            action,
            shots,
            glyphs,
            profile,
        } => {
            use acs_capture::glyphs as g;
            let path = glyphs
                .or_else(|| std::env::var_os("ACSVM_CAPTURE_GLYPHS").map(PathBuf::from))
                .unwrap_or_else(|| fail("no glyph table: --glyphs or ACSVM_CAPTURE_GLYPHS"));
            let mut table = if action == "learn" && !path.exists() {
                g::Table::default()
            } else {
                g::Table::load(&path).unwrap_or_else(|e| fail(&e))
            };
            let screens: Vec<(PathBuf, g::Screen)> = shots
                .into_iter()
                .map(|p| {
                    let s = g::Screen::from_png(&p, &[g::INK], &g::MASK).unwrap_or_else(|e| fail(&e));
                    (p, s)
                })
                .collect();
            match action.as_str() {
                "learn" => {
                    for (p, s) in &screens {
                        let added = g::learn(s, &mut table);
                        println!("{}: {} new", p.display(), added.len());
                    }
                    std::fs::write(&path, table.to_text()).unwrap_or_else(|e| fail(&e.to_string()));
                    // Text art of every glyph, for labelling by eye.
                    let mut art = String::new();
                    for (i, (bits, label)) in table.glyphs.iter().enumerate() {
                        art.push_str(&format!(
                            "{i} {bits:016X} {}
",
                            label.as_deref().unwrap_or("?")
                        ));
                        art.push_str(&g::art(*bits));
                    }
                    let art_path = path.with_extension("art.txt");
                    std::fs::write(&art_path, art).unwrap_or_else(|e| fail(&e.to_string()));
                    println!(
                        "{} glyphs in {}; art in {}",
                        table.glyphs.len(),
                        path.display(),
                        art_path.display()
                    );
                }
                "read" => {
                    for (p, s) in &screens {
                        let text = g::read(s, &table);
                        println!(
                            "== {} (grid {:?}, doubtful {:?})",
                            p.display(),
                            text.grid,
                            text.doubtful
                        );
                        for l in &text.lines {
                            println!("|{l}");
                        }
                        if profile {
                            let stats = g::profile(&text);
                            println!("{}", serde_json::to_string(&stats).unwrap_or_default());
                        }
                    }
                }
                other => fail(&format!("glyphs {other}: learn or read")),
            }
        }
        Cmd::Script {
            script,
            out,
            boot,
            disk,
            snapshot,
            visible,
            vars,
        } => {
            let mut text = if script.as_os_str() == "-" {
                std::io::read_to_string(std::io::stdin()).expect("stdin")
            } else {
                std::fs::read_to_string(&script).expect("script file")
            };
            for v in &vars {
                let (k, val) = v.split_once('=').unwrap_or_else(|| fail("--var NAME=VALUE"));
                text = text.replace(&format!("${{{k}}}"), &val.replace('\\', "/"));
            }
            std::fs::create_dir_all(&out).expect("out folder");
            let opts = Options {
                drive8: boot.clone().or(disk),
                autostart: boot.is_some(),
                visible,
                ..Options::default()
            };
            let vice = Vice::start(&opts).unwrap_or_else(|e| fail(&e.to_string()));
            let mut s = Session::new(vice, out);
            match s.vice.mon.vice_version() {
                Ok(v) => println!("VICE {v}"),
                Err(e) => {
                    drop(s);
                    fail(&e.to_string())
                }
            }
            if let Some(snap) = snapshot {
                let line = format!("undump {}", snap.display());
                report(&line, s.line(&line));
            }
            let started = std::time::Instant::now();
            let mut failed = false;
            for line in text.lines() {
                let r = s.line(line);
                failed = r.is_err();
                report(line, r);
                if failed {
                    break;
                }
            }
            // Shut VICE down before exiting: process::exit skips destructors.
            drop(s);
            if failed {
                std::process::exit(1);
            }
            println!("done in {:.1}s", started.elapsed().as_secs_f64());
        }
    }
}

fn report(line: &str, r: Result<String, String>) {
    match r {
        Ok(note) if note.is_empty() => {}
        Ok(note) => println!("{note}"),
        Err(e) => eprintln!("{}: {e}", line.trim()),
    }
}

fn fail(msg: &str) -> ! {
    eprintln!("acs-capture: {msg}");
    std::process::exit(1)
}
