// SPDX-License-Identifier: GPL-3.0-or-later

//! Client for VICE's binary monitor protocol, API version 2 (VICE manual,
//! chapter 13). Written from the published protocol description.

use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

const STX: u8 = 0x02;
const API: u8 = 0x02;
/// Request id VICE uses for responses it sends on its own (events).
pub const EVENT_ID: u32 = 0xFFFF_FFFF;

pub mod cmd {
    pub const MEM_GET: u8 = 0x01;
    pub const MEM_SET: u8 = 0x02;
    pub const DUMP: u8 = 0x41;
    pub const UNDUMP: u8 = 0x42;
    pub const ADVANCE: u8 = 0x71;
    pub const KEYBOARD_FEED: u8 = 0x72;
    pub const PING: u8 = 0x81;
    pub const DISPLAY_GET: u8 = 0x84;
    pub const VICE_INFO: u8 = 0x85;
    pub const PALETTE_GET: u8 = 0x91;
    pub const JOYPORT_SET: u8 = 0xA2;
    pub const EXIT: u8 = 0xAA;
    pub const QUIT: u8 = 0xBB;
    pub const RESET: u8 = 0xCC;
    pub const AUTOSTART: u8 = 0xDD;
    pub const REGISTERS_GET: u8 = 0x31;
    pub const REGISTERS_AVAILABLE: u8 = 0x83;
    pub const STOPPED: u8 = 0x62;
    pub const RESUMED: u8 = 0x63;
}

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    /// The server answered with a non-zero error code.
    Vice {
        command: u8,
        code: u8,
    },
    Protocol(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "monitor i/o: {e}"),
            Error::Vice { command, code } => {
                write!(f, "VICE refused command {command:#04x} (error {code:#04x})")
            }
            Error::Protocol(s) => write!(f, "monitor protocol: {s}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// One response frame.
#[derive(Debug, Clone)]
pub struct Response {
    pub kind: u8,
    pub code: u8,
    pub id: u32,
    pub body: Vec<u8>,
}

/// The rendered screen, 8-bit indexed (one byte per pixel, a palette index).
#[derive(Debug, Clone)]
pub struct Display {
    pub width: usize,
    pub height: usize,
    /// The inner (non-border) area.
    pub inner: (usize, usize, usize, usize),
    pub pixels: Vec<u8>,
}

pub struct Monitor {
    stream: TcpStream,
    next_id: u32,
    /// Program counter from the last "stopped" event.
    pub last_stop_pc: Option<u16>,
    pub running: bool,
    /// Calls and time per command (docs/capture.md 2.8).
    pub stats: crate::timing::Stats,
}

impl Monitor {
    pub fn connect(addr: &str) -> Result<Self> {
        let stream = TcpStream::connect(addr)?;
        stream.set_nodelay(true)?;
        stream.set_read_timeout(Some(Duration::from_secs(60)))?;
        Ok(Monitor {
            stream,
            next_id: 1,
            last_stop_pc: None,
            running: true,
            stats: Default::default(),
        })
    }

    fn read_frame(&mut self) -> Result<Response> {
        let mut head = [0u8; 12];
        self.stream.read_exact(&mut head)?;
        if head[0] != STX {
            return Err(Error::Protocol(format!("bad start byte {:#04x}", head[0])));
        }
        let len = u32::from_le_bytes([head[2], head[3], head[4], head[5]]) as usize;
        let kind = head[6];
        let code = head[7];
        let id = u32::from_le_bytes([head[8], head[9], head[10], head[11]]);
        let mut body = vec![0u8; len];
        self.stream.read_exact(&mut body)?;
        let r = Response { kind, code, id, body };
        self.note_event(&r);
        Ok(r)
    }

    fn note_event(&mut self, r: &Response) {
        match r.kind {
            cmd::STOPPED => {
                self.running = false;
                if r.body.len() >= 2 {
                    self.last_stop_pc = Some(u16::from_le_bytes([r.body[0], r.body[1]]));
                }
            }
            cmd::RESUMED => self.running = true,
            _ => {}
        }
    }

    /// Sends a command and waits for its response (events in between are noted).
    pub fn request(&mut self, command: u8, body: &[u8]) -> Result<Vec<u8>> {
        let t0 = Instant::now();
        let r = self.request_untimed(command, body);
        self.stats.call(command, t0.elapsed());
        r
    }

    fn request_untimed(&mut self, command: u8, body: &[u8]) -> Result<Vec<u8>> {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);
        let mut frame = Vec::with_capacity(11 + body.len());
        frame.push(STX);
        frame.push(API);
        frame.extend_from_slice(&(body.len() as u32).to_le_bytes());
        frame.extend_from_slice(&id.to_le_bytes());
        frame.push(command);
        frame.extend_from_slice(body);
        self.stream.write_all(&frame)?;
        loop {
            let r = self.read_frame()?;
            if r.id == id {
                if r.code != 0 {
                    return Err(Error::Vice {
                        command,
                        code: r.code,
                    });
                }
                return Ok(r.body);
            }
        }
    }

    /// Reads frames until the machine reports it stopped (after an advance).
    fn wait_stopped(&mut self) -> Result<()> {
        while self.running {
            self.read_frame()?;
        }
        Ok(())
    }

    pub fn ping(&mut self) -> Result<()> {
        self.request(cmd::PING, &[]).map(|_| ())
    }

    /// VICE's version as "major.minor.build.revision".
    pub fn vice_version(&mut self) -> Result<String> {
        let b = self.request(cmd::VICE_INFO, &[])?;
        let n = *b.first().unwrap_or(&0) as usize;
        let v: Vec<String> = b
            .get(1..1 + n)
            .unwrap_or(&[])
            .iter()
            .map(|x| x.to_string())
            .collect();
        Ok(v.join("."))
    }

    /// Reads `start..=end` of main memory without side effects.
    pub fn mem_get(&mut self, start: u16, end: u16) -> Result<Vec<u8>> {
        self.mem_read(start, end, false)
    }

    /// Reads `start..=end` of main memory; `side_effects` reads I/O registers
    /// as the CPU would (live port values).
    pub fn mem_read(&mut self, start: u16, end: u16, side_effects: bool) -> Result<Vec<u8>> {
        let mut body = vec![side_effects as u8];
        body.extend_from_slice(&start.to_le_bytes());
        body.extend_from_slice(&end.to_le_bytes());
        body.push(0); // main memory
        body.extend_from_slice(&0u16.to_le_bytes()); // default bank
        let b = self.request(cmd::MEM_GET, &body)?;
        Ok(b.get(2..).unwrap_or(&[]).to_vec())
    }

    pub fn mem_set(&mut self, start: u16, bytes: &[u8]) -> Result<()> {
        let end = start.wrapping_add(bytes.len() as u16 - 1);
        let mut body = vec![0u8];
        body.extend_from_slice(&start.to_le_bytes());
        body.extend_from_slice(&end.to_le_bytes());
        body.push(0);
        body.extend_from_slice(&0u16.to_le_bytes());
        body.extend_from_slice(bytes);
        self.request(cmd::MEM_SET, &body).map(|_| ())
    }

    /// Steps `count` instructions (subroutines counted inside), then stays stopped.
    pub fn advance(&mut self, count: u16) -> Result<()> {
        let mut body = vec![0u8];
        body.extend_from_slice(&count.to_le_bytes());
        self.running = true;
        self.request(cmd::ADVANCE, &body)?;
        let t0 = Instant::now();
        let r = self.wait_stopped();
        self.stats.extend(cmd::ADVANCE, t0.elapsed());
        self.stats.advanced += count as u64;
        r
    }

    /// Steps `count` instructions in chunks.
    pub fn advance_long(&mut self, mut count: u64) -> Result<()> {
        while count > 0 {
            let n = count.min(u16::MAX as u64) as u16;
            self.advance(n)?;
            count -= n as u64;
        }
        Ok(())
    }

    /// Leaves the monitor; the machine runs freely.
    pub fn resume(&mut self) -> Result<()> {
        self.request(cmd::EXIT, &[])?;
        self.running = true;
        Ok(())
    }

    /// Saves a snapshot; `with_disks` embeds the attached disk images, so
    /// restoring it brings them back.
    pub fn dump(&mut self, path: &str, with_disks: bool) -> Result<()> {
        let mut body = vec![0u8, with_disks as u8, path.len() as u8];
        body.extend_from_slice(path.as_bytes());
        self.request(cmd::DUMP, &body).map(|_| ())
    }

    pub fn undump(&mut self, path: &str) -> Result<u16> {
        let mut body = vec![path.len() as u8];
        body.extend_from_slice(path.as_bytes());
        let b = self.request(cmd::UNDUMP, &body)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    pub fn reset(&mut self, what: u8) -> Result<()> {
        self.request(cmd::RESET, &[what]).map(|_| ())
    }

    /// Puts PETSCII text into the KERNAL keyboard buffer.
    pub fn keyboard_feed(&mut self, petscii: &[u8]) -> Result<()> {
        let mut body = vec![petscii.len() as u8];
        body.extend_from_slice(petscii);
        self.request(cmd::KEYBOARD_FEED, &body).map(|_| ())
    }

    /// Sets the joystick on `port` (0 = control port 1, 1 = control port 2)
    /// to the pressed directions `bits` (up 1, down 2, left 4, right 8,
    /// fire 16). VICE 3.10's command drives the I/O simulation device
    /// plugged into the port, whose lines are active low, so the value sent
    /// is the complement.
    pub fn joystick(&mut self, port: u16, bits: u16) -> Result<()> {
        let mut body = port.to_le_bytes().to_vec();
        body.extend_from_slice(&(!bits & 0xFF).to_le_bytes());
        self.request(cmd::JOYPORT_SET, &body).map(|_| ())
    }

    pub fn autostart(&mut self, path: &str, run: bool) -> Result<()> {
        let mut body = vec![run as u8];
        body.extend_from_slice(&0u16.to_le_bytes());
        body.push(path.len() as u8);
        body.extend_from_slice(path.as_bytes());
        self.request(cmd::AUTOSTART, &body).map(|_| ())
    }

    /// The palette as RGB triples.
    pub fn palette(&mut self) -> Result<Vec<[u8; 3]>> {
        let b = self.request(cmd::PALETTE_GET, &[1])?;
        let n = u16::from_le_bytes([b[0], b[1]]) as usize;
        let mut out = Vec::with_capacity(n);
        let mut i = 2;
        for _ in 0..n {
            let size = b[i] as usize;
            out.push([b[i + 1], b[i + 2], b[i + 3]]);
            i += 1 + size;
        }
        Ok(out)
    }

    pub fn display(&mut self) -> Result<Display> {
        let b = self.request(cmd::DISPLAY_GET, &[1, 0])?;
        let u16at = |i: usize| u16::from_le_bytes([b[i], b[i + 1]]) as usize;
        let width = u16at(4);
        let height = u16at(6);
        let inner = (u16at(8), u16at(10), u16at(12), u16at(14));
        // The buffer is the last width x height bytes of the reply.
        let n = width * height;
        if b.len() < n {
            return Err(Error::Protocol(format!(
                "display reply is {} bytes, expected {width} x {height}",
                b.len()
            )));
        }
        let pixels = b[b.len() - n..].to_vec();
        Ok(Display {
            width,
            height,
            inner,
            pixels,
        })
    }

    /// Register names and ids the CPU exposes (main memory space).
    pub fn registers_available(&mut self) -> Result<Vec<(u8, String)>> {
        let b = self.request(cmd::REGISTERS_AVAILABLE, &[0])?;
        let n = u16::from_le_bytes([b[0], b[1]]) as usize;
        let mut out = Vec::with_capacity(n);
        let mut i = 2;
        for _ in 0..n {
            let size = b[i] as usize;
            let id = b[i + 1];
            let name_len = b[i + 3] as usize;
            let name = String::from_utf8_lossy(&b[i + 4..i + 4 + name_len]).into_owned();
            out.push((id, name));
            i += 1 + size;
        }
        Ok(out)
    }

    /// Current register values as (id, value).
    pub fn registers(&mut self) -> Result<Vec<(u8, u16)>> {
        let b = self.request(cmd::REGISTERS_GET, &[0])?;
        let n = u16::from_le_bytes([b[0], b[1]]) as usize;
        let mut out = Vec::with_capacity(n);
        let mut i = 2;
        for _ in 0..n {
            let size = b[i] as usize;
            out.push((b[i + 1], u16::from_le_bytes([b[i + 2], b[i + 3]])));
            i += 1 + size;
        }
        Ok(out)
    }

    pub fn quit(&mut self) -> Result<()> {
        self.request(cmd::QUIT, &[]).map(|_| ())
    }
}
