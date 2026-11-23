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

/// Picks the platform from the extension (or the Amiga's `adventure` file
/// name), falling back to the size: every C64 adventure is a whole `.d64`
/// disk, so a disk-sized file is a C64 one, and the Amiga's file and disks
/// have sizes of their own.
pub fn guess_platform(name: &str, len: usize) -> Platform {
    Platform::from_path(Path::new(name)).unwrap_or(if d64::VALID_SIZES.contains(&len) {
        Platform::C64
    } else if len == acs_core::disk::AMIGA_FILE_SIZE || acs_core::adf::SIZES.contains(&len) {
        Platform::Amiga
    } else {
        Platform::Pc
    })
}

/// An opened adventure, and its Native HD art if it is a bundle with some.
pub struct Loaded {
    pub decoded: Decoded,
    pub art: Option<acs_bundle::Art>,
    /// A bundle's credits (content 2), for the end screen (winning 4.6).
    pub credits: Vec<String>,
}

/// Decodes `bytes`, an original file or an Extended bundle (content 2);
/// the error is a message for the user.
pub fn load(name: &str, bytes: Vec<u8>) -> Result<Loaded, String> {
    if acs_bundle::is_bundle(name, &bytes) {
        let b = acs_bundle::read(&bytes).map_err(|e| e.to_string())?;
        return Ok(Loaded {
            decoded: Decoded {
                adventure: b.adventure,
                diagnostics: Vec::new(),
            },
            art: b.art,
            credits: b.manifest.credits,
        });
    }
    let platform = guess_platform(name, bytes.len());
    let image = Image::from_bytes(platform, bytes).map_err(|e| e.to_string())?;
    let decoded = decode(&image).map_err(|e| e.to_string())?;
    Ok(Loaded {
        decoded,
        art: None,
        credits: Vec::new(),
    })
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
        assert_eq!(guess_platform("adventure", 0), Platform::Amiga);
        assert_eq!(guess_platform("ga.bin", 0x28300), Platform::Amiga);
        assert_eq!(guess_platform("GAME.ADF", 0), Platform::Amiga);
        assert_eq!(guess_platform("disk", 901_120), Platform::Amiga);
    }

    #[test]
    fn bad_files_give_messages_not_panics() {
        assert!(load("x.d64", vec![0; 100]).err().unwrap().contains("bytes"));
        assert!(load("x.fpy", Vec::new()).is_err());
        assert!(load("x.advx", b"PK not really".to_vec())
            .err()
            .unwrap()
            .contains("not a bundle"));
    }

    /// A bundle opens with its art, which Native HD mode draws (content 2).
    #[test]
    fn a_bundle_opens_in_native_hd() {
        use crate::tiles::{RenderMode, Tiles};
        let mut f = crate::test_fixture::Fixture::new(Platform::Pc);
        f.header("COVE", "", "");
        let mut adventure = decode(&f.build()).expect("decodes").adventure;
        adventure.profile = acs_core::Profile::Extended;
        adventure.limits = acs_core::Limits::EXTENDED;
        let art = acs_bundle::Art {
            tile: 16,
            width: 16,
            height: 16,
            rgba: [9, 8, 7, 255].repeat(256),
            places: [(1, 0)].into(),
        };
        let bundle = acs_bundle::Bundle {
            manifest: acs_bundle::Manifest::new(&adventure, "CC0"),
            adventure,
            art: Some(art),
        };
        let bytes = acs_bundle::write(&bundle).unwrap();
        // By its signature, whatever the name.
        let loaded = load("upload", bytes).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(loaded.decoded.adventure.name, "COVE");
        let art = loaded.art.expect("art");

        let ctx = egui::Context::default();
        let mut tiles = Tiles::new(&ctx, &loaded.decoded.adventure);
        assert!(!tiles.offers(RenderMode::NativeHd));
        tiles.set_mode(&ctx, RenderMode::NativeHd);
        assert_eq!(tiles.mode(), RenderMode::Original, "refused without art");
        tiles.set_native(&ctx, &art);
        tiles.set_mode(&ctx, RenderMode::NativeHd);
        assert_eq!(tiles.mode(), RenderMode::NativeHd);
        assert_eq!(tiles.get(1).unwrap().size(), [16, 16]);
        // Picture 0 has no HD art: drawn Smoothed (4x).
        assert_eq!(tiles.get(0).unwrap().size(), [64, 64]);
    }

    /// A bundle's pictures need not all be the first one's size: each is
    /// smoothed at its own (content 2).
    #[test]
    fn pictures_of_other_sizes_smooth_at_their_own() {
        use crate::tiles::{RenderMode, Tiles};
        let mut adventure = decode(&crate::test_fixture::Fixture::new(Platform::Pc).build())
            .expect("decodes")
            .adventure;
        adventure.pictures.push(acs_core::graphics::Picture {
            width: 8,
            height: 16,
            pixel_aspect: 1,
            pixels: vec![1; 8 * 16],
        });
        let last = adventure.pictures.len() as u16 - 1;
        let ctx = egui::Context::default();
        let mut tiles = Tiles::new(&ctx, &adventure);
        assert_eq!(tiles.size, [16, 16]);
        tiles.set_mode(&ctx, RenderMode::Smoothed);
        assert_eq!(tiles.get(last).unwrap().size(), [32, 64]);
        assert_eq!(tiles.get(0).unwrap().size(), [64, 64]);
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
