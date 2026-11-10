// SPDX-License-Identifier: GPL-3.0-or-later

//! Decodes every adventure in the private test corpus.
//!
//! Set `ACSVM_CORPUS` to a folder containing `c64/` (and later `pc/`). The
//! corpus lives outside the repository; without the variable these tests
//! pass with a "skipped" note. Disks named `advcnst*` are EA's program disks
//! and are skipped.

use acs_core::model::{Adventure, Destination, ThingKind};
use acs_core::scene::{self, RoomScene};
use acs_core::{decode, DiagnosticKind, Image, Platform};
use std::path::{Path, PathBuf};

fn corpus() -> Option<PathBuf> {
    let dir = PathBuf::from(std::env::var_os("ACSVM_CORPUS")?);
    assert!(dir.is_dir(), "ACSVM_CORPUS={} is not a directory", dir.display());
    Some(dir)
}

fn find(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            find(&p, out);
        } else if Platform::from_path(&p).is_some() {
            let name = p.file_name().unwrap().to_string_lossy().to_ascii_lowercase();
            if !name.starts_with("advcnst") {
                out.push(p);
            }
        }
    }
}

fn adventures(sub: &str) -> Option<Vec<PathBuf>> {
    let mut files = Vec::new();
    find(&corpus()?.join(sub), &mut files);
    files.sort();
    Some(files)
}

/// Checks that hold for any well-formed adventure.
fn check_consistency(a: &Adventure, file: &str) {
    for r in &a.regions {
        for room in r.rooms.iter().filter(|r| !r.deleted) {
            for item in &room.items {
                let t = &a.things[item.thing as usize - 1];
                assert!(
                    !matches!(t.kind, ThingKind::Undefined | ThingKind::Unknown { .. }),
                    "{file}: region {} room {} places undefined thing {}",
                    r.number,
                    room.number,
                    item.thing
                );
                // A door may lead to a deleted room (the game then sends the
                // player to the start), but never to a region or slot that
                // does not exist.
                if let Some(Destination::Room { region, room: dr, .. }) = item.destination {
                    let target = a
                        .regions
                        .get(region as usize - 1)
                        .and_then(|reg| reg.rooms.get(dr as usize));
                    assert!(
                        target.is_some(),
                        "{file}: door in region {} room {} leads to missing room {region}/{dr}",
                        r.number,
                        room.number
                    );
                }
            }
        }
        for c in r.residents.iter().filter(|c| c.in_use) {
            assert_eq!(c.region, r.number, "{file}: resident {} region", c.name);
        }
    }
    for c in a.creatures.iter().filter(|c| c.in_use) {
        for (thing, ok) in [(c.weapon, "weapon"), (c.armor, "armor")] {
            if thing == 0 {
                continue;
            }
            let k = &a.things[thing as usize - 1].kind;
            let good = match ok {
                "weapon" => matches!(k, ThingKind::MeleeWeapon { .. } | ThingKind::MissileWeapon { .. }),
                _ => matches!(k, ThingKind::Armor { .. }),
            };
            assert!(
                good,
                "{file}: creature {} readies {ok} {thing}, a {}",
                c.name,
                k.type_name()
            );
        }
    }
}

/// The room-drawing rules `acs_core::scene` relies on (format-notes,
/// "Drawing rooms").
fn check_scene(a: &Adventure, file: &str) {
    for n in [scene::FLOOR_THING, scene::BORDER_THING] {
        assert!(
            matches!(a.things[n as usize - 1].kind, ThingKind::RoomFloor { .. }),
            "{file}: thing {n} is not a room floor"
        );
    }
    for r in &a.regions {
        let (layout, _) = scene::region_layout(r);
        // Rooms never overlap by more than a shared wall line.
        for (i, (_, g)) in layout.iter().enumerate() {
            for (_, h) in &layout[i + 1..] {
                let ix = (g.x + g.w).min(h.x + h.w) as i32 - g.x.max(h.x) as i32;
                let iy = (g.y + g.h).min(h.y + h.h) as i32 - g.y.max(h.y) as i32;
                assert!(
                    ix <= 0 || iy <= 0 || ix.min(iy) == 1,
                    "{file}: region {} rooms overlap by {ix}x{iy}",
                    r.number
                );
            }
        }
        for (k, _) in &layout {
            let room = &r.rooms[*k];
            let s = RoomScene::new(r, room);
            // Items fall outside their room only at the stash square.
            for &i in &s.off_room {
                let it = &room.items[i];
                assert_eq!((it.x, it.y), (15, 14), "{file}: region {} room {k}", r.number);
            }
        }
        // Every resident of a live room stands inside it.
        for c in r.residents.iter().filter(|c| c.in_use) {
            if let Some(room) = r.rooms.get(c.room as usize).filter(|rm| !rm.deleted) {
                assert!(
                    c.x < room.width && c.y < room.height,
                    "{file}: region {} resident {} outside room {}",
                    r.number,
                    c.name,
                    c.room
                );
            }
        }
    }
}

#[test]
fn c64_corpus_decodes_cleanly() {
    let Some(files) = adventures("c64") else {
        eprintln!("skipped: ACSVM_CORPUS not set");
        return;
    };
    assert!(!files.is_empty(), "no .d64 adventures under ACSVM_CORPUS/c64");
    for path in &files {
        let file = path.file_name().unwrap().to_string_lossy();
        let img = Image::open(path, None).unwrap_or_else(|e| panic!("{file}: {e}"));
        let d = decode(&img).unwrap_or_else(|e| panic!("{file}: {e}"));
        let unknown = d
            .diagnostics
            .iter()
            .filter(|x| x.kind == DiagnosticKind::UnknownThingType)
            .count();
        assert_eq!(unknown, 0, "{file}: unknown thing types");
        assert!(d.diagnostics.is_empty(), "{file}: {:#?}", d.diagnostics);
        assert_eq!(d.adventure.pictures.len(), 110);
        check_consistency(&d.adventure, &file);
        check_scene(&d.adventure, &file);
        // The model survives a JSON round trip.
        let json = serde_json::to_string(&d.adventure).unwrap();
        let back: Adventure = serde_json::from_str(&json).unwrap();
        assert_eq!(back, d.adventure, "{file}: JSON round trip");
        eprintln!("ok: {file}: {}", d.adventure.name);
    }
    eprintln!("{} C64 adventures decoded", files.len());
}

#[test]
fn c64_program_disks_are_rejected() {
    let Some(dir) = corpus() else { return };
    let mut stack = vec![dir.join("c64")];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.file_name().unwrap().to_string_lossy().to_ascii_lowercase() == "advcnst1.d64" {
                let img = Image::open(&p, None).unwrap();
                assert!(
                    decode(&img).is_err(),
                    "the ACS program disk must not decode as an adventure"
                );
            }
        }
    }
}

#[test]
fn pc_corpus_decodes_cleanly() {
    let Some(files) = adventures("pc") else {
        eprintln!("skipped: ACSVM_CORPUS not set");
        return;
    };
    if files.is_empty() {
        eprintln!(
            "skipped: no PC adventures under ACSVM_CORPUS/pc (PC decoding is untested against real files)"
        );
        return;
    }
    for path in &files {
        let file = path.file_name().unwrap().to_string_lossy();
        let img = Image::open(path, None).unwrap_or_else(|e| panic!("{file}: {e}"));
        let d = decode(&img).unwrap_or_else(|e| panic!("{file}: {e}"));
        assert_eq!(d.errors().count(), 0, "{file}: {:#?}", d.diagnostics);
        assert_eq!(d.adventure.pictures.len(), 96);
        check_consistency(&d.adventure, &file);
        check_scene(&d.adventure, &file);
        eprintln!(
            "ok: {file}: {} ({} warnings)",
            d.adventure.name,
            d.diagnostics.len()
        );
    }
    eprintln!("{} PC adventures decoded", files.len());
}
