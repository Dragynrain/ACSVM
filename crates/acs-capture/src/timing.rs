// SPDX-License-Identifier: GPL-3.0-or-later

//! Where a run's wall time goes: calls and time per binary monitor command,
//! and the instructions advanced (docs/capture.md 2.7). Time outside the
//! monitor is the harness's own work (frame analysis, glyph reading, files).

use crate::monitor::cmd;
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::time::Duration;

#[derive(Debug, Clone, Default)]
pub struct Stats {
    /// Per command byte: calls and time waiting on VICE (request sent to
    /// reply read; for an advance, until the machine stops again).
    calls: BTreeMap<u8, (u64, Duration)>,
    /// Instructions stepped with "advance".
    pub advanced: u64,
}

pub fn name(command: u8) -> String {
    match command {
        cmd::MEM_GET => "mem_get".into(),
        cmd::MEM_SET => "mem_set".into(),
        cmd::DUMP => "dump".into(),
        cmd::UNDUMP => "undump".into(),
        cmd::ADVANCE => "advance".into(),
        cmd::KEYBOARD_FEED => "keyboard".into(),
        cmd::PING => "ping".into(),
        cmd::REGISTERS_GET => "registers".into(),
        cmd::REGISTERS_AVAILABLE => "registers_available".into(),
        cmd::DISPLAY_GET => "display".into(),
        cmd::VICE_INFO => "vice_info".into(),
        cmd::PALETTE_GET => "palette".into(),
        cmd::JOYPORT_SET => "joystick".into(),
        cmd::EXIT => "exit".into(),
        cmd::QUIT => "quit".into(),
        cmd::RESET => "reset".into(),
        cmd::AUTOSTART => "autostart".into(),
        other => format!("{other:#04x}"),
    }
}

impl Stats {
    /// One call of `command` that took `t`.
    pub fn call(&mut self, command: u8, t: Duration) {
        let e = self.calls.entry(command).or_default();
        e.0 += 1;
        e.1 += t;
    }

    /// More time for the last call of `command` (an advance's wait for the
    /// machine to stop after the reply).
    pub fn extend(&mut self, command: u8, t: Duration) {
        self.calls.entry(command).or_default().1 += t;
    }

    /// Everything that happened after `earlier` (a copy of these stats).
    pub fn since(&self, earlier: &Stats) -> Stats {
        let mut out = Stats {
            calls: BTreeMap::new(),
            advanced: self.advanced - earlier.advanced,
        };
        for (&c, &(n, t)) in &self.calls {
            let (n0, t0) = earlier.calls.get(&c).copied().unwrap_or_default();
            if n > n0 || t > t0 {
                out.calls.insert(c, (n - n0, t - t0));
            }
        }
        out
    }

    pub fn monitor_time(&self) -> Duration {
        self.calls.values().map(|(_, t)| *t).sum()
    }

    fn time_of(&self, command: u8) -> Duration {
        self.calls.get(&command).map(|(_, t)| *t).unwrap_or_default()
    }

    /// The stats over `wall` seconds of host time, as one JSON object.
    pub fn to_json(&self, wall: Duration) -> Value {
        let mut commands = Map::new();
        for (&c, &(n, t)) in &self.calls {
            commands.insert(name(c), json!({ "calls": n, "s": round3(t.as_secs_f64()) }));
        }
        let wall_s = wall.as_secs_f64();
        let adv_s = self.time_of(cmd::ADVANCE).as_secs_f64();
        json!({
            "wall_s": round3(wall_s),
            "instructions": self.advanced,
            // Instructions per host second, over the whole run and over the
            // time spent inside advances alone.
            "mips": round3(per_second(self.advanced, wall_s) / 1e6),
            "advance_mips": round3(per_second(self.advanced, adv_s) / 1e6),
            "monitor_s": round3(self.monitor_time().as_secs_f64()),
            "harness_s": round3(wall.saturating_sub(self.monitor_time()).as_secs_f64()),
            "commands": commands,
        })
    }

    /// One line for the terminal: the biggest costs first.
    pub fn line(&self, wall: Duration) -> String {
        let wall_s = wall.as_secs_f64();
        let mut parts: Vec<(Duration, String)> = self
            .calls
            .iter()
            .map(|(&c, &(n, t))| (t, format!("{} {:.1}s/{n}", name(c), t.as_secs_f64())))
            .collect();
        parts.sort_by_key(|a| std::cmp::Reverse(a.0));
        let top: Vec<String> = parts.into_iter().take(4).map(|(_, s)| s).collect();
        format!(
            "{:.1}s, {:.1}M instr ({:.2}M/s); {}; harness {:.1}s",
            wall_s,
            self.advanced as f64 / 1e6,
            per_second(self.advanced, wall_s) / 1e6,
            top.join(", "),
            wall.saturating_sub(self.monitor_time()).as_secs_f64(),
        )
    }
}

fn per_second(n: u64, s: f64) -> f64 {
    if s > 0.0 {
        n as f64 / s
    } else {
        0.0
    }
}

fn round3(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn since_subtracts_and_drops_unchanged_commands() {
        let mut a = Stats::default();
        a.call(cmd::ADVANCE, Duration::from_millis(10));
        a.call(cmd::PING, Duration::from_millis(1));
        a.advanced = 1_000;
        let before = a.clone();
        a.call(cmd::ADVANCE, Duration::from_millis(5));
        a.extend(cmd::ADVANCE, Duration::from_millis(5));
        a.advanced += 500;
        let d = a.since(&before);
        assert_eq!(d.advanced, 500);
        assert_eq!(d.calls.len(), 1);
        assert_eq!(d.calls[&cmd::ADVANCE], (1, Duration::from_millis(10)));
        let j = d.to_json(Duration::from_millis(20));
        assert_eq!(j["commands"]["advance"]["calls"], 1);
        assert_eq!(j["harness_s"], 0.01);
    }
}
