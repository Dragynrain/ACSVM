// SPDX-License-Identifier: GPL-3.0-or-later

//! Disk images and adventure files.
//!
//! An ACS adventure is read as one flat byte image:
//!
//! - **PC** (`ADVEN.FPY`, `ADVEN.HRD`): the adventure file itself.
//! - **C64** (`.d64`): the whole disk, read raw. An adventure disk has no
//!   usable directory; the data fills every sector in order and simply skips
//!   the BAM and directory sectors on track 18 (see `docs/format-notes.md`).

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;

/// The machine an adventure was made on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    Pc,
    C64,
}

impl Platform {
    pub fn name(self) -> &'static str {
        match self {
            Platform::Pc => "PC",
            Platform::C64 => "C64",
        }
    }

    /// Guesses the platform from a file extension.
    pub fn from_path(path: &Path) -> Option<Platform> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        match ext.as_str() {
            "d64" => Some(Platform::C64),
            "fpy" | "hrd" => Some(Platform::Pc),
            _ => None,
        }
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// An adventure's bytes plus the platform they are laid out for.
#[derive(Clone)]
pub struct Image {
    pub platform: Platform,
    pub data: Vec<u8>,
}

impl fmt::Debug for Image {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Image")
            .field("platform", &self.platform)
            .field("len", &self.data.len())
            .finish()
    }
}

/// Smallest PC adventure file that holds every structure the decoder reads
/// (the last picture bank ends at 0x2B920).
pub const PC_MIN_SIZE: usize = 0x2B920;

impl Image {
    /// Wraps raw bytes, checking the size fits the platform.
    pub fn from_bytes(platform: Platform, data: Vec<u8>) -> Result<Self> {
        match platform {
            Platform::C64 => {
                if !d64::VALID_SIZES.contains(&data.len()) {
                    return Err(Error::BadSize {
                        platform: "C64 .d64",
                        actual: data.len(),
                        expected: d64::VALID_SIZES
                            .iter()
                            .map(|s| s.to_string())
                            .collect::<Vec<_>>()
                            .join(", "),
                    });
                }
            }
            Platform::Pc => {
                if data.len() < PC_MIN_SIZE {
                    return Err(Error::BadSize {
                        platform: "PC adventure",
                        actual: data.len(),
                        expected: format!("at least {PC_MIN_SIZE}"),
                    });
                }
            }
        }
        Ok(Self { platform, data })
    }

    /// Reads a file, taking the platform from `platform` or else the extension.
    pub fn open(path: &Path, platform: Option<Platform>) -> Result<Self> {
        let platform = platform
            .or_else(|| Platform::from_path(path))
            .ok_or_else(|| Error::UnknownPlatform(path.display().to_string()))?;
        let data = std::fs::read(path).map_err(|source| Error::Io {
            path: path.display().to_string(),
            source,
        })?;
        Self::from_bytes(platform, data)
    }
}

/// Commodore 1541 `.d64` geometry.
pub mod d64 {
    /// 35 tracks; 35 tracks with error bytes; 40 tracks; 40 tracks with error bytes.
    pub const VALID_SIZES: [usize; 4] = [174_848, 175_531, 196_608, 197_376];
    pub const SECTOR_SIZE: usize = 256;

    /// Sectors on a track (1-based), or `None` past track 40.
    pub const fn sectors_per_track(track: u8) -> Option<u8> {
        match track {
            1..=17 => Some(21),
            18..=24 => Some(19),
            25..=30 => Some(18),
            31..=40 => Some(17),
            _ => None,
        }
    }

    /// Byte offset of a track/sector in the image.
    pub fn sector_offset(track: u8, sector: u8) -> Option<usize> {
        if sector >= sectors_per_track(track)? {
            return None;
        }
        let before: usize = (1..track)
            .map(|t| sectors_per_track(t).unwrap_or(0) as usize)
            .sum();
        Some((before + sector as usize) * SECTOR_SIZE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn d64_geometry() {
        assert_eq!(d64::sector_offset(1, 0), Some(0));
        // Track 18 (BAM, directory) starts at 0x16500: the 0x200-byte gap
        // between the PC and C64 layouts is its first two sectors.
        assert_eq!(d64::sector_offset(18, 0), Some(0x16500));
        assert_eq!(d64::sector_offset(18, 2), Some(0x16700));
        assert_eq!(d64::sector_offset(35, 16), Some(174_848 - 256));
        assert_eq!(d64::sector_offset(1, 21), None);
        assert_eq!(d64::sector_offset(41, 0), None);
    }

    #[test]
    fn platform_from_extension() {
        assert_eq!(Platform::from_path(Path::new("x/GAME.D64")), Some(Platform::C64));
        assert_eq!(Platform::from_path(Path::new("ADVEN.HRD")), Some(Platform::Pc));
        assert_eq!(Platform::from_path(Path::new("adven.fpy")), Some(Platform::Pc));
        assert_eq!(Platform::from_path(Path::new("readme.txt")), None);
    }

    #[test]
    fn size_checks() {
        assert!(Image::from_bytes(Platform::C64, vec![0; 174_848]).is_ok());
        assert!(Image::from_bytes(Platform::C64, vec![0; 1000]).is_err());
        assert!(Image::from_bytes(Platform::Pc, vec![0; PC_MIN_SIZE]).is_ok());
        assert!(Image::from_bytes(Platform::Pc, vec![0; PC_MIN_SIZE - 1]).is_err());
    }
}
