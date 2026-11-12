// SPDX-License-Identifier: GPL-3.0-or-later

//! `acs-capture run --jobs N` (docs/capture.md 4.3): splits the runs into N
//! blocks of consecutive run indices, runs each block in its own process
//! (and so its own VICE) in `OUT/partK` on its own copy of the disk, then
//! joins their `runs.jsonl` and `timing.jsonl` into `OUT`. A run's result
//! depends only on its index, so the joined file is what one process gives.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

/// `runs` runs from index `first`, split into at most `jobs` blocks of
/// consecutive indices, as (first, count); the larger blocks first.
pub fn split(first: usize, runs: usize, jobs: usize) -> Vec<(usize, usize)> {
    let jobs = jobs.clamp(1, runs.max(1));
    let (base, extra) = (runs / jobs, runs % jobs);
    let mut at = first;
    let mut out = Vec::new();
    for k in 0..jobs {
        let n = base + usize::from(k < extra);
        if n > 0 {
            out.push((at, n));
        }
        at += n;
    }
    out
}

/// The command line for one part: the parent's arguments without the ones
/// a part sets itself, then the part's own.
pub fn part_args(args: &[String], first: usize, runs: usize, out: &Path, disk: &Path) -> Vec<String> {
    const OWN: [&str; 5] = ["--jobs", "--first", "--runs", "--out", "--disk"];
    let mut kept = Vec::new();
    let mut skip_value = false;
    for a in args {
        if skip_value {
            skip_value = false;
        } else if OWN.contains(&a.as_str()) {
            skip_value = true;
        } else if !OWN.iter().any(|o| a.starts_with(&format!("{o}="))) {
            kept.push(a.clone());
        }
    }
    kept.extend([
        "--first".into(),
        first.to_string(),
        "--runs".into(),
        runs.to_string(),
        "--out".into(),
        out.display().to_string(),
        "--disk".into(),
        disk.display().to_string(),
    ]);
    kept
}

/// Runs the parts and joins their output. `args` is the parent's command
/// line without the program. Each part's stderr is shown with its name and
/// kept in `OUT/partK.log`. Fails, after joining what there is, if a part did.
pub fn run(
    args: &[String],
    disk: &Path,
    out: &Path,
    first: usize,
    runs: usize,
    jobs: usize,
) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let started = Instant::now();
    let parts = split(first, runs, jobs);
    let mut children = Vec::new();
    for (k, &(f, n)) in parts.iter().enumerate() {
        let dir = out.join(format!("part{k}"));
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        // Each VICE gets its own copy: the program may write to the disk.
        let part_disk = dir.join("disk.d64");
        std::fs::copy(disk, &part_disk).map_err(|e| format!("{}: {e}", disk.display()))?;
        let mut child = Command::new(&exe)
            .args(part_args(args, f, n, &dir, &part_disk))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?;
        let stderr = child.stderr.take().expect("piped stderr");
        let mut log = std::fs::File::create(out.join(format!("part{k}.log"))).map_err(|e| e.to_string())?;
        let echo = std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                eprintln!("[part{k}] {line}");
                let _ = writeln!(log, "{line}");
            }
        });
        eprintln!("[part{k}] runs {f}..{}", f + n - 1);
        children.push((k, child, echo));
    }
    let mut failed = Vec::new();
    for (k, mut child, echo) in children {
        let ok = child.wait().map(|s| s.success()).unwrap_or(false);
        let _ = echo.join();
        if !ok {
            failed.push(format!("part{k}"));
        }
    }
    join(out, parts.len(), started.elapsed().as_secs_f64())?;
    if failed.is_empty() {
        Ok(())
    } else {
        Err(format!("{} failed (see {}.log)", failed.join(", "), failed[0]))
    }
}

/// Joins the parts' `runs.jsonl` (in part order, so in run order) and
/// `timing.jsonl` (each line tagged with its part), and adds a total line
/// over all the parts with the batch's wall time.
fn join(out: &Path, parts: usize, wall_s: f64) -> Result<(), String> {
    let part = |k: usize, f: &str| -> PathBuf { out.join(format!("part{k}")).join(f) };
    let mut runs = String::new();
    let mut timing = String::new();
    let mut instructions = 0u64;
    for k in 0..parts {
        if let Ok(text) = std::fs::read_to_string(part(k, "runs.jsonl")) {
            runs.push_str(&text);
        }
        for line in std::fs::read_to_string(part(k, "timing.jsonl"))
            .unwrap_or_default()
            .lines()
        {
            let Ok(mut v) = serde_json::from_str::<Value>(line) else {
                continue;
            };
            if v["run"] == "total" {
                instructions += v["instructions"].as_u64().unwrap_or(0);
            }
            v["part"] = k.into();
            timing.push_str(&format!("{v}\n"));
        }
    }
    timing.push_str(&format!(
        "{}\n",
        json!({
            "run": "total", "parts": parts, "wall_s": (wall_s * 1000.0).round() / 1000.0,
            "instructions": instructions,
            "mips": (instructions as f64 / wall_s / 1e3).round() / 1e3,
        })
    ));
    eprintln!(
        "all parts: {wall_s:.1}s, {:.1}M instr ({:.2}M/s over {parts} parts)",
        instructions as f64 / 1e6,
        instructions as f64 / wall_s / 1e6
    );
    std::fs::write(out.join("runs.jsonl"), runs).map_err(|e| e.to_string())?;
    std::fs::write(out.join("timing.jsonl"), timing).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_covers_every_run_once() {
        assert_eq!(split(0, 60, 4), vec![(0, 15), (15, 15), (30, 15), (45, 15)]);
        assert_eq!(split(5, 10, 4), vec![(5, 3), (8, 3), (11, 2), (13, 2)]);
        assert_eq!(split(0, 2, 8), vec![(0, 1), (1, 1)]);
        assert_eq!(split(0, 0, 4), vec![]);
    }

    #[test]
    fn part_args_replaces_the_parts_own() {
        let args: Vec<String> = [
            "run",
            "cast",
            "--jobs",
            "4",
            "--snapshot",
            "s.vsf",
            "--disk=d.d64",
            "--out",
            "o",
            "--runs",
            "60",
            "--jitter-step",
            "7",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let got = part_args(&args, 15, 15, Path::new("o/part1"), Path::new("o/part1/disk.d64"));
        let want = [
            "run",
            "cast",
            "--snapshot",
            "s.vsf",
            "--jitter-step",
            "7",
            "--first",
            "15",
            "--runs",
            "15",
            "--out",
            "o/part1",
            "--disk",
            "o/part1/disk.d64",
        ];
        assert_eq!(got, want);
    }
}
