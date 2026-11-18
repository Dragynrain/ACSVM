// SPDX-License-Identifier: GPL-3.0-or-later

//! Bundles (docs/content.md 2) round-trip, and each refusal names its entry.

#[path = "../../acs-core/tests/common/mod.rs"]
mod common;

use acs_bundle::{read, write, Art, Bundle, BundleError, Manifest, ADVENTURE, ART, MANIFEST};
use acs_core::limits::{Limits, Profile};
use acs_core::{decode, Adventure, Platform};
use common::{Fixture, RegionSpec, RoomSpec};
use std::collections::BTreeMap;
use std::io::{Cursor, Write};

fn extended() -> Adventure {
    let mut f = Fixture::new(Platform::Pc);
    f.header("COVE", "BY US", "");
    f.thing(127, "EMPTY FLOOR", [0x00, 1, 0, 0]);
    f.region(RegionSpec {
        name: "COVE",
        rooms: vec![RoomSpec {
            name: "BEACH",
            w: 6,
            h: 5,
            ..Default::default()
        }],
        ..Default::default()
    });
    let mut a = decode(&f.build()).expect("decodes").adventure;
    a.profile = Profile::Extended;
    a.limits = Limits::EXTENDED;
    a
}

/// A 2 x 1 atlas of 16 px tiles: place 0 red, place 1 blue.
fn art() -> Art {
    let (tile, width, height) = (16u32, 32u32, 16u32);
    let mut rgba = Vec::new();
    for _y in 0..height {
        for x in 0..width {
            rgba.extend_from_slice(if x < tile {
                &[255, 0, 0, 255]
            } else {
                &[0, 0, 255, 255]
            });
        }
    }
    Art {
        tile,
        width,
        height,
        rgba,
        places: BTreeMap::from([(0, 1), (3, 0)]),
    }
}

fn bundle(with_art: bool) -> Bundle {
    let adventure = extended();
    Bundle {
        manifest: Manifest::new(&adventure, "CC BY-SA 4.0"),
        adventure,
        art: with_art.then(art),
    }
}

/// A zip of the given entries, for refusals `write` won't make.
fn zip_of(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in entries {
        z.start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        z.write_all(bytes).unwrap();
    }
    z.finish().unwrap().into_inner()
}

fn parts(b: &Bundle) -> (Vec<u8>, Vec<u8>) {
    (
        serde_json::to_vec(&b.manifest).unwrap(),
        serde_json::to_vec(&b.adventure).unwrap(),
    )
}

#[test]
fn a_bundle_round_trips_with_and_without_art() {
    for with_art in [false, true] {
        let b = bundle(with_art);
        let bytes = write(&b).unwrap();
        let back = read(&bytes).unwrap();
        assert_eq!(back.adventure, b.adventure);
        assert_eq!(back.art, b.art);
        assert_eq!(back.manifest.art.is_some(), with_art);
        assert_eq!(back.manifest.name, "COVE");
    }
}

#[test]
fn writing_is_deterministic() {
    let b = bundle(true);
    assert_eq!(write(&b).unwrap(), write(&b).unwrap());
}

#[test]
fn art_tiles_come_from_their_places() {
    let a = art();
    let blue = a.picture(0).unwrap();
    assert_eq!(blue.len(), 16 * 16 * 4);
    assert!(blue.chunks(4).all(|p| p == [0, 0, 255, 255]));
    assert!(a.picture(3).unwrap().chunks(4).all(|p| p == [255, 0, 0, 255]));
    assert!(a.picture(1).is_none());
}

#[test]
fn not_a_zip_is_refused() {
    assert!(matches!(read(b"hello"), Err(BundleError::NotAZip(_))));
}

#[test]
fn missing_entries_are_named() {
    let (m, a) = parts(&bundle(false));
    assert_eq!(
        read(&zip_of(&[(ADVENTURE, &a)])).unwrap_err(),
        BundleError::Missing { entry: MANIFEST }
    );
    assert_eq!(
        read(&zip_of(&[(MANIFEST, &m)])).unwrap_err(),
        BundleError::Missing { entry: ADVENTURE }
    );
    let mut with_art = bundle(true).manifest;
    with_art.art = Some(acs_bundle::ArtManifest {
        tile: 16,
        pictures: vec![],
    });
    let m = serde_json::to_vec(&with_art).unwrap();
    assert_eq!(
        read(&zip_of(&[(MANIFEST, &m), (ADVENTURE, &a)])).unwrap_err(),
        BundleError::Missing { entry: ART }
    );
}

#[test]
fn unknown_features_are_refused_by_name() {
    let mut b = bundle(false);
    b.manifest.features = vec!["multi-square-creatures".into()];
    assert_eq!(
        read(&write(&b).unwrap()).unwrap_err(),
        BundleError::Features(vec!["multi-square-creatures".into()])
    );
}

#[test]
fn wrong_format_or_version_is_refused() {
    let mut b = bundle(false);
    b.manifest.version = 2;
    assert!(matches!(
        read(&write(&b).unwrap()),
        Err(BundleError::Bad { entry: MANIFEST, .. })
    ));
}

#[test]
fn newer_models_and_classic_profiles_are_refused() {
    let mut b = bundle(false);
    b.adventure.model_version = acs_core::model::MODEL_VERSION + 1;
    let e = read(&write(&b).unwrap()).unwrap_err().to_string();
    assert!(e.contains("newer version"), "{e}");

    let mut b = bundle(false);
    b.adventure.profile = Profile::Classic;
    assert!(matches!(
        read(&write(&b).unwrap()),
        Err(BundleError::Bad { entry: ADVENTURE, .. })
    ));
}

#[test]
fn limits_are_checked() {
    let mut b = bundle(false);
    b.adventure.limits.world_size = 4096;
    let e = read(&write(&b).unwrap()).unwrap_err().to_string();
    assert!(e.contains("world_size"), "{e}");

    let mut b = bundle(false);
    b.adventure.limits.regions = 0;
    let e = read(&write(&b).unwrap()).unwrap_err().to_string();
    assert!(e.contains("over its limits"), "{e}");
}

#[test]
fn art_out_of_range_is_refused() {
    let mut b = bundle(true);
    b.art.as_mut().unwrap().places.insert(1, 9);
    assert!(matches!(
        read(&write(&b).unwrap()),
        Err(BundleError::Bad { entry: MANIFEST, .. })
    ));

    let mut b = bundle(true);
    let n = b.adventure.pictures.len() as u16;
    b.art.as_mut().unwrap().places.insert(n, 0);
    assert!(matches!(
        read(&write(&b).unwrap()),
        Err(BundleError::Bad { entry: MANIFEST, .. })
    ));
}

#[test]
fn unsafe_entry_paths_are_refused() {
    let (m, a) = parts(&bundle(false));
    let bytes = zip_of(&[(MANIFEST, &m), (ADVENTURE, &a), ("../evil", b"x")]);
    assert!(matches!(read(&bytes), Err(BundleError::NotAZip(_))));
}

#[test]
fn other_entries_are_ignored() {
    let (m, a) = parts(&bundle(false));
    let bytes = zip_of(&[(MANIFEST, &m), (ADVENTURE, &a), ("audio/readme.txt", b"later")]);
    assert!(read(&bytes).is_ok());
}

/// Winning 4.5: the endings feature is listed exactly when there are
/// endings.
#[test]
fn the_endings_feature_goes_with_endings() {
    let ending = acs_core::model::Ending {
        region: 1,
        room: 0,
        thing: 1,
        message: 1,
    };
    let mut b = bundle(false);
    b.adventure.endings = vec![ending];
    b.manifest = Manifest::new(&b.adventure, "CC BY-SA 4.0");
    assert_eq!(b.manifest.features, ["endings"]);
    assert_eq!(read(&write(&b).unwrap()).unwrap().adventure.endings, [ending]);
    b.manifest.features.clear();
    let e = read(&write(&b).unwrap()).unwrap_err().to_string();
    assert!(e.contains("isn't listed"), "{e}");
    let mut b = bundle(false);
    b.manifest.features = vec!["endings".into()];
    let e = read(&write(&b).unwrap()).unwrap_err().to_string();
    assert!(e.contains("no endings"), "{e}");
}
