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
fn a_picture_whose_pixels_dont_fill_it_is_refused() {
    // The viewer indexes a picture's pixels by its width and height.
    let mut b = bundle(false);
    b.adventure.pictures.push(acs_core::graphics::Picture {
        width: 16,
        height: 16,
        pixel_aspect: 1,
        pixels: vec![],
    });
    let e = read(&write(&b).unwrap()).unwrap_err().to_string();
    assert!(e.contains("malformed") && e.contains("picture"), "{e}");
}

#[test]
fn a_world_map_with_uneven_rows_is_refused() {
    // The viewer draws the map as wide as its first row.
    let mut b = bundle(false);
    b.adventure.world.tiles[1].pop();
    let e = read(&write(&b).unwrap()).unwrap_err().to_string();
    assert!(e.contains("malformed") && e.contains("world map row 1"), "{e}");
}

#[test]
fn a_room_numbered_out_of_place_is_refused() {
    // The engine and the viewer find a room by its place in the list.
    let mut b = bundle(false);
    let r = &mut b.adventure.regions[0];
    r.rooms[0].number = 1;
    let e = read(&write(&b).unwrap()).unwrap_err().to_string();
    assert!(e.contains("malformed") && e.contains("numbered 1"), "{e}");
}

#[test]
fn a_region_numbered_out_of_place_is_refused() {
    // The engine finds a region by its number, and 0 is the world map's.
    let mut b = bundle(false);
    b.adventure.regions[0].number = 0;
    let e = read(&write(&b).unwrap()).unwrap_err().to_string();
    assert!(
        e.contains("malformed") && e.contains("region 1 numbered 0"),
        "{e}"
    );
}

#[test]
fn an_entry_past_the_size_cap_is_refused() {
    // A zip bomb: a few KB packed, past ENTRY_MAX unpacked.
    let (m, _) = parts(&bundle(false));
    let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
    z.start_file(MANIFEST, zip::write::SimpleFileOptions::default())
        .unwrap();
    z.write_all(&m).unwrap();
    let deflated = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .large_file(true);
    z.start_file(ADVENTURE, deflated).unwrap();
    let chunk = vec![b' '; 1 << 20];
    for _ in 0..=(acs_bundle::ENTRY_MAX >> 20) {
        z.write_all(&chunk).unwrap();
    }
    let bytes = z.finish().unwrap().into_inner();
    let e = read(&bytes).unwrap_err().to_string();
    assert!(e.contains("larger than"), "{e}");
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
