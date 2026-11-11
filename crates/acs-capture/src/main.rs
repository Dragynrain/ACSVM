// SPDX-License-Identifier: GPL-3.0-or-later

//! `acs-capture`: the behavior capture harness's command line.

use acs_capture::script::Session;
use acs_capture::vice::{Options, Vice};
use acs_capture::{experiment, scenario};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

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
    },
    /// Run an experiment from a scenario snapshot (docs/capture.md 4).
    Run {
        /// Experiment: move-basic (world map) or move-room (the same pushes
        /// in the room behind the map door; the snapshot is taken there).
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
    },
    /// Distil a runs.jsonl into summary.json.
    Summarize { runs: PathBuf },
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
        Cmd::Scenario { name, out, template } => {
            let template = template
                .or_else(|| std::env::var_os("ACSVM_CAPTURE_TEMPLATE").map(PathBuf::from))
                .map(|p| std::fs::read(&p).unwrap_or_else(|e| fail(&format!("{}: {e}", p.display()))));
            if template.is_none() {
                eprintln!("note: no template; the original program will refuse this disk");
            }
            let img = match name.as_str() {
                "move-basic" => scenario::move_basic(&scenario::MoveBasic::default(), template.as_deref()),
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
        } => {
            let place = match name.as_str() {
                "move-basic" => experiment::Place::Map,
                "move-room" => experiment::Place::Room,
                other => fail(&format!("unknown experiment {other}")),
            };
            std::fs::create_dir_all(&out).expect("out folder");
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
            };
            let started = std::time::Instant::now();
            let mut result = Ok(());
            'outer: for run in 0..runs {
                // Jitter: a different number of idle instructions before each
                // run moves the machine's noise sources (docs/capture.md 4.3).
                let jitter = (run as u64 * 7_919) % 1_000_000;
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
                eprintln!("run {run} done ({:.0}s)", started.elapsed().as_secs_f64());
            }
            drop(r);
            drop(s);
            if let Err(e) = result {
                fail(&e);
            }
            let summary = experiment::summarize(&out.join("runs.jsonl")).expect("summary");
            let text = serde_json::to_string_pretty(&summary).unwrap_or_default();
            std::fs::write(out.join("summary.json"), &text).expect("summary.json");
            println!("{text}");
        }
        Cmd::Summarize { runs } => {
            let summary = experiment::summarize(&runs).unwrap_or_else(|e| fail(&e.to_string()));
            println!("{}", serde_json::to_string_pretty(&summary).unwrap_or_default());
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
