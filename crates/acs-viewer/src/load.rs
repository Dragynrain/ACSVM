// SPDX-License-Identifier: GPL-3.0-or-later

//! Turning a user's file (bytes plus a file name) into a decoded adventure.
//! Nothing here touches the network: the bytes come from a drop, a file
//! picker or a path on the command line.

use acs_core::disk::d64;
use acs_core::{decode, Decoded, Image, Platform};
use std::path::Path;

/// A file the user handed to the viewer, read or failed.
pub struct Incoming {
    pub name: String,
    pub bytes: Result<Vec<u8>, String>,
}

/// Picks the platform from the extension, falling back to the size: every
/// C64 adventure is a whole `.d64` disk, so a disk-sized file is a C64 one.
pub fn guess_platform(name: &str, len: usize) -> Platform {
    Platform::from_path(Path::new(name)).unwrap_or(if d64::VALID_SIZES.contains(&len) {
        Platform::C64
    } else {
        Platform::Pc
    })
}

/// Decodes `bytes`; the error is a message for the user.
pub fn load(name: &str, bytes: Vec<u8>) -> Result<Decoded, String> {
    let platform = guess_platform(name, bytes.len());
    let image = Image::from_bytes(platform, bytes).map_err(|e| e.to_string())?;
    decode(&image).map_err(|e| e.to_string())
}

/// The text of a save file (docs/saves.md 2), or `None` for anything else:
/// it is JSON naming the save format, which no adventure file is.
pub fn save_text(bytes: &[u8]) -> Option<&str> {
    let text = std::str::from_utf8(bytes).ok()?;
    (text.trim_start().starts_with('{') && text.contains(acs_engine::save::SAVE_FORMAT)).then_some(text)
}

/// The file name part of a path or browser file name.
pub fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_from_extension_then_size() {
        assert_eq!(guess_platform("GAME.D64", 10), Platform::C64);
        assert_eq!(guess_platform("adven.fpy", 174_848), Platform::Pc);
        assert_eq!(guess_platform("ADVEN.HRD", 0), Platform::Pc);
        assert_eq!(guess_platform("disk", 174_848), Platform::C64);
        assert_eq!(guess_platform("disk.img", 196_608), Platform::C64);
        assert_eq!(guess_platform("ACS", 229_376), Platform::Pc);
    }

    #[test]
    fn bad_files_give_messages_not_panics() {
        assert!(load("x.d64", vec![0; 100]).unwrap_err().contains("bytes"));
        assert!(load("x.fpy", Vec::new()).is_err());
    }

    #[test]
    fn saves_are_told_from_adventures() {
        assert!(save_text(br#" {"format": "acsvm-save", "version": 1}"#).is_some());
        assert!(save_text(br#"{"format": "other"}"#).is_none());
        assert!(save_text(&[0x7b, 0xff, 0x00]).is_none());
        assert!(save_text(&[0; 64]).is_none());
    }

    #[test]
    fn display_name_drops_directories() {
        assert_eq!(display_name(Path::new("a/b/GAME.D64")), "GAME.D64");
        assert_eq!(display_name(Path::new("GAME.D64")), "GAME.D64");
    }
}
