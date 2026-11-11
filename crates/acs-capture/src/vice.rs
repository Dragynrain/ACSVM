// SPDX-License-Identifier: GPL-3.0-or-later

//! Starts VICE's `x64sc` with both remote monitors and connects to them.

use crate::monitor::{Monitor, Result};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// Finds `x64sc` from `ACSVM_VICE` (the executable or its folder), else PATH.
pub fn find_x64sc() -> PathBuf {
    let exe = if cfg!(windows) { "x64sc.exe" } else { "x64sc" };
    if let Ok(v) = std::env::var("ACSVM_VICE") {
        let p = PathBuf::from(v);
        if p.is_dir() {
            for cand in [p.join(exe), p.join("bin").join(exe)] {
                if cand.is_file() {
                    return cand;
                }
            }
        }
        return p;
    }
    PathBuf::from(exe)
}

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .and_then(|l| l.local_addr())
        .map(|a| a.port())
        .expect("no free local port")
}

pub struct Options {
    /// Disk image for drive 8 at start-up, if any.
    pub drive8: Option<PathBuf>,
    /// Autostart that image (boot it), or just attach it.
    pub autostart: bool,
    pub warp: bool,
    /// Show VICE's window (for watching); otherwise it starts minimized.
    pub visible: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            drive8: None,
            autostart: false,
            warp: true,
            visible: false,
        }
    }
}

pub struct Vice {
    child: Child,
    pub mon: Monitor,
    text_port: u16,
}

impl Vice {
    pub fn start(opts: &Options) -> Result<Vice> {
        let bin_port = free_port();
        let text_port = free_port();
        let mut cmd = Command::new(find_x64sc());
        cmd.args(["-default", "-silent", "+confirmonexit", "+saveres"])
            .args(["-sounddev", "dummy"])
            // VICE 3.10's "joyport set" drives the I/O simulation device
            // (device 37). Only control port 2 gets one: on port 1 its
            // unused lines read low on $DC01 and look like held keys.
            .args(["-controlport2device", "37"])
            .arg("-binarymonitor")
            .arg("-binarymonitoraddress")
            .arg(format!("ip4://127.0.0.1:{bin_port}"))
            .arg("-remotemonitor")
            .arg("-remotemonitoraddress")
            .arg(format!("ip4://127.0.0.1:{text_port}"));
        if opts.warp {
            cmd.arg("-warp");
        }
        if !opts.visible {
            cmd.arg("-minimized");
        }
        if let Some(d) = &opts.drive8 {
            if opts.autostart {
                cmd.arg("-autostart").arg(d);
            } else {
                cmd.arg("-8").arg(d);
            }
        }
        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let mut child = cmd.spawn()?;
        let deadline = Instant::now() + Duration::from_secs(20);
        let addr = format!("127.0.0.1:{bin_port}");
        let mon = loop {
            match Monitor::connect(&addr) {
                Ok(m) => break m,
                Err(e) if Instant::now() > deadline => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(e);
                }
                Err(_) => std::thread::sleep(Duration::from_millis(100)),
            }
        };
        let mut v = Vice {
            child,
            mon,
            text_port,
        };
        v.mon.ping()?;
        v.release_joysticks()?;
        Ok(v)
    }

    /// Both joysticks centred, fire up (the I/O simulation lines start low,
    /// which would read as every direction pressed).
    pub fn release_joysticks(&mut self) -> Result<()> {
        self.mon.joystick(1, 0)
    }

    /// Sends one line to the text monitor and returns what it printed.
    /// The text monitor stops the machine while connected; this leaves it
    /// stopped (the binary monitor's next command keeps it so).
    pub fn text_command(&mut self, line: &str) -> Result<String> {
        let mut s = TcpStream::connect(("127.0.0.1", self.text_port))?;
        s.set_read_timeout(Some(Duration::from_millis(1500)))?;
        s.write_all(line.as_bytes())?;
        s.write_all(b"\n")?;
        let mut out = Vec::new();
        let mut r = BufReader::new(s.try_clone()?);
        let mut buf = [0u8; 4096];
        loop {
            match r.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => out.extend_from_slice(&buf[..n]),
                Err(_) => break,
            }
        }
        let _ = r.fill_buf();
        Ok(String::from_utf8_lossy(&out).into_owned())
    }

    /// Attaches a disk image to drive 8 (through the text monitor).
    pub fn attach8(&mut self, image: &Path) -> Result<String> {
        let p = image.display().to_string().replace('\\', "/");
        self.text_command(&format!("attach \"{p}\" 8"))
    }
}

impl Drop for Vice {
    fn drop(&mut self) {
        let _ = self.mon.quit();
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline {
            if let Ok(Some(_)) = self.child.try_wait() {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
