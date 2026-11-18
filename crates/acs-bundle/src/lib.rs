// SPDX-License-Identifier: GPL-3.0-or-later

//! Extended adventure bundles (`docs/content.md` 2): one zip file holding
//! the manifest (`bundle.json`), the canonical model (`adventure.json`)
//! and optional Native HD art (`art/pictures.png`).
//!
//! The engine never sees a bundle, only the [`Adventure`] in it.

use acs_core::limits::{self, Limits, Profile};
use acs_core::model::{PictureId, MODEL_VERSION};
use acs_core::Adventure;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, ZipArchive, ZipWriter};

/// A bundle's file extension, without the dot. [choice] in content.md 2.
pub const EXTENSION: &str = "advx";
/// The manifest's `format`.
pub const FORMAT: &str = "acsvm-adventure";
/// The manifest version this reader reads and writes.
pub const VERSION: u32 = 1;

pub const MANIFEST: &str = "bundle.json";
pub const ADVENTURE: &str = "adventure.json";
pub const ART: &str = "art/pictures.png";

/// Rule features this engine knows (design.md, "What stays fixed in v1").
/// None yet.
pub const KNOWN_FEATURES: &[&str] = &[ENDINGS];

/// The end state (winning 4): listed exactly when the model has endings.
pub const ENDINGS: &str = "endings";

/// `bundle.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub format: String,
    pub version: u32,
    pub name: String,
    #[serde(default)]
    pub byline: String,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub credits: Vec<String>,
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub art: Option<ArtManifest>,
}

impl Manifest {
    /// A manifest for `adv` with no art or credits; its features are the
    /// ones its model needs (`endings` when it has endings).
    pub fn new(adv: &Adventure, license: &str) -> Manifest {
        Manifest {
            format: FORMAT.into(),
            version: VERSION,
            name: adv.name.clone(),
            byline: adv.byline.clone(),
            license: license.into(),
            credits: Vec::new(),
            features: if adv.endings.is_empty() {
                Vec::new()
            } else {
                vec![ENDINGS.into()]
            },
            art: None,
        }
    }
}

/// The manifest's `art` object: the atlas's tile size and where each
/// picture sits in it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtManifest {
    pub tile: u32,
    pub pictures: Vec<ArtEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtEntry {
    pub picture: PictureId,
    pub at: u32,
}

/// Native HD art: the decoded atlas and each picture's place in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Art {
    /// Tile side in pixels.
    pub tile: u32,
    pub width: u32,
    pub height: u32,
    /// The atlas, RGBA, row by row.
    pub rgba: Vec<u8>,
    /// Picture number to atlas place; first entry wins.
    pub places: BTreeMap<PictureId, u32>,
}

impl Art {
    /// One picture's HD tile as RGBA (`tile * tile * 4` bytes), if it has one.
    pub fn picture(&self, picture: PictureId) -> Option<Vec<u8>> {
        let at = *self.places.get(&picture)?;
        let per_row = self.width / self.tile;
        let (x0, y0) = ((at % per_row) * self.tile, (at / per_row) * self.tile);
        let t = self.tile as usize;
        let mut out = Vec::with_capacity(t * t * 4);
        for y in 0..t {
            let start = ((y0 as usize + y) * self.width as usize + x0 as usize) * 4;
            out.extend_from_slice(&self.rgba[start..start + t * 4]);
        }
        Some(out)
    }
}

/// A loaded bundle.
#[derive(Debug, Clone, PartialEq)]
pub struct Bundle {
    pub manifest: Manifest,
    pub adventure: Adventure,
    pub art: Option<Art>,
}

/// Why a bundle was refused (content.md 2, "Loading"). Each names the entry.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum BundleError {
    #[error("not a bundle: {0}")]
    NotAZip(String),
    #[error("{entry}: missing")]
    Missing { entry: &'static str },
    #[error("{entry}: {message}")]
    Bad { entry: &'static str, message: String },
    #[error("{MANIFEST}: needs rule features this engine lacks: {}", .0.join(", "))]
    Features(Vec<String>),
}

fn bad(entry: &'static str, message: impl ToString) -> BundleError {
    BundleError::Bad {
        entry,
        message: message.to_string(),
    }
}

/// The four bytes every zip file starts with.
const ZIP_MAGIC: [u8; 4] = *b"PK";

/// Whether a file a user handed over is a bundle: its extension, or for a
/// name without it (a browser drop, a renamed file) the zip signature. No
/// original adventure file starts with it.
pub fn is_bundle(name: &str, bytes: &[u8]) -> bool {
    let ext = std::path::Path::new(name).extension().and_then(|e| e.to_str());
    ext.is_some_and(|e| e.eq_ignore_ascii_case(EXTENSION)) || bytes.starts_with(&ZIP_MAGIC)
}

/// True if an entry's path is one a bundle may hold (no `..`, leading `/`
/// or backslash).
fn safe_path(name: &str) -> bool {
    !name.starts_with('/') && !name.contains('\\') && !name.split('/').any(|p| p == "..")
}

/// Reads and checks a bundle; nothing loads in part.
pub fn read(bytes: &[u8]) -> Result<Bundle, BundleError> {
    let mut zip = ZipArchive::new(Cursor::new(bytes)).map_err(|e| BundleError::NotAZip(e.to_string()))?;
    for i in 0..zip.len() {
        let f = zip
            .by_index_raw(i)
            .map_err(|e| BundleError::NotAZip(e.to_string()))?;
        let name = f.name().map_err(|e| BundleError::NotAZip(e.to_string()))?;
        if !safe_path(&name) {
            return Err(BundleError::NotAZip(format!("unsafe entry path {name:?}")));
        }
        if !matches!(
            f.compression(),
            CompressionMethod::Stored | CompressionMethod::Deflated
        ) {
            return Err(BundleError::NotAZip(format!(
                "{name}: compression other than stored or deflate"
            )));
        }
    }
    let manifest_bytes = entry(&mut zip, MANIFEST)?.ok_or(BundleError::Missing { entry: MANIFEST })?;
    let adventure_bytes = entry(&mut zip, ADVENTURE)?.ok_or(BundleError::Missing { entry: ADVENTURE })?;
    let art_bytes = entry(&mut zip, ART)?;

    let manifest: Manifest = serde_json::from_slice(&manifest_bytes).map_err(|e| bad(MANIFEST, e))?;
    if manifest.format != FORMAT || manifest.version != VERSION {
        return Err(bad(
            MANIFEST,
            format!(
                "format {} version {}, not {FORMAT} {VERSION}",
                manifest.format, manifest.version
            ),
        ));
    }
    let lacking: Vec<String> = manifest
        .features
        .iter()
        .filter(|f| !KNOWN_FEATURES.contains(&f.as_str()))
        .cloned()
        .collect();
    if !lacking.is_empty() {
        return Err(BundleError::Features(lacking));
    }

    let adventure = adventure(&adventure_bytes)?;
    // Winning 4.5: the feature is listed exactly when there are endings.
    let listed = manifest.features.iter().any(|f| f == ENDINGS);
    if listed != !adventure.endings.is_empty() {
        return Err(bad(
            MANIFEST,
            if listed {
                "lists the endings feature, but the adventure has no endings"
            } else {
                "the adventure has endings, but the endings feature isn't listed"
            },
        ));
    }
    let art = match (&manifest.art, art_bytes) {
        (Some(m), Some(png)) => Some(art(m, &png, &adventure)?),
        (None, None) => None,
        (Some(_), None) => return Err(BundleError::Missing { entry: ART }),
        (None, Some(_)) => return Err(bad(MANIFEST, format!("no art object, but {ART} is present"))),
    };
    Ok(Bundle {
        manifest,
        adventure,
        art,
    })
}

fn entry(zip: &mut ZipArchive<Cursor<&[u8]>>, name: &'static str) -> Result<Option<Vec<u8>>, BundleError> {
    let mut f = match zip.by_name(name) {
        Ok(f) => f,
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(e) => return Err(bad(name, e)),
    };
    let mut out = Vec::new();
    f.read_to_end(&mut out).map_err(|e| bad(name, e))?;
    Ok(Some(out))
}

fn adventure(bytes: &[u8]) -> Result<Adventure, BundleError> {
    #[derive(Deserialize)]
    struct Version {
        model_version: u32,
    }
    let v: Version = serde_json::from_slice(bytes).map_err(|e| bad(ADVENTURE, e))?;
    if v.model_version > MODEL_VERSION {
        return Err(bad(
            ADVENTURE,
            format!(
                "made by a newer version (model {}, this reader {MODEL_VERSION})",
                v.model_version
            ),
        ));
    }
    let adv: Adventure = serde_json::from_slice(bytes).map_err(|e| bad(ADVENTURE, e))?;
    if adv.profile != Profile::Extended {
        return Err(bad(
            ADVENTURE,
            "profile is classic; a bundle holds Extended adventures only",
        ));
    }
    let over = over_extended(&adv.limits);
    if !over.is_empty() {
        return Err(bad(
            ADVENTURE,
            format!("limits past Extended's: {}", over.join(", ")),
        ));
    }
    let violations = limits::check(&adv);
    if !violations.is_empty() {
        let list: Vec<String> = violations.iter().map(ToString::to_string).collect();
        return Err(bad(ADVENTURE, format!("over its limits: {}", list.join("; "))));
    }
    Ok(adv)
}

/// The limits in `l` that are larger than Extended's.
fn over_extended(l: &Limits) -> Vec<&'static str> {
    let e = Limits::EXTENDED;
    let pairs = [
        ("world_size", l.world_size, e.world_size),
        ("terrain_types", l.terrain_types, e.terrain_types),
        ("map_doors", l.map_doors, e.map_doors),
        ("regions", l.regions, e.regions),
        ("rooms_per_region", l.rooms_per_region, e.rooms_per_region),
        ("room_size", l.room_size, e.room_size),
        (
            "residents_per_region",
            l.residents_per_region,
            e.residents_per_region,
        ),
        (
            "random_creatures_per_region",
            l.random_creatures_per_region,
            e.random_creatures_per_region,
        ),
        ("creatures_per_room", l.creatures_per_room, e.creatures_per_room),
        ("things", l.things, e.things),
        ("creatures", l.creatures, e.creatures),
        ("creature_classes", l.creature_classes, e.creature_classes),
        ("pictures", l.pictures, e.pictures),
        ("long_messages", l.long_messages, e.long_messages),
        ("short_messages", l.short_messages, e.short_messages),
        ("name_length", l.name_length, e.name_length),
    ];
    pairs
        .iter()
        .filter(|(_, a, b)| a > b)
        .map(|(n, _, _)| *n)
        .collect()
}

fn art(m: &ArtManifest, png_bytes: &[u8], adv: &Adventure) -> Result<Art, BundleError> {
    let mut decoder = png::Decoder::new(Cursor::new(png_bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(|e| bad(ART, e))?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut buf).map_err(|e| bad(ART, e))?;
    let px = &buf[..frame.buffer_size()];
    let rgba: Vec<u8> = match frame.color_type {
        png::ColorType::Rgba => px.to_vec(),
        png::ColorType::Rgb => px
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|c| [c[0], c[1], c[2], 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => px
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|c| [c[0], c[0], c[0], c[1]])
            .collect(),
        png::ColorType::Grayscale => px.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        other => return Err(bad(ART, format!("colour type {other:?}"))),
    };
    let (width, height) = (frame.width, frame.height);
    if !(16..=512).contains(&m.tile) || width % m.tile != 0 || height % m.tile != 0 {
        return Err(bad(
            ART,
            format!("tile {} doesn't divide the {width} x {height} atlas", m.tile),
        ));
    }
    let places_in_atlas = (width / m.tile) * (height / m.tile);
    let mut places = BTreeMap::new();
    for e in &m.pictures {
        if e.at >= places_in_atlas {
            return Err(bad(
                MANIFEST,
                format!(
                    "picture {} at place {}, past the atlas's {places_in_atlas}",
                    e.picture, e.at
                ),
            ));
        }
        if e.picture as usize >= adv.pictures.len() {
            return Err(bad(
                MANIFEST,
                format!(
                    "art for picture {}, but the adventure has {}",
                    e.picture,
                    adv.pictures.len()
                ),
            ));
        }
        places.entry(e.picture).or_insert(e.at);
    }
    Ok(Art {
        tile: m.tile,
        width,
        height,
        rgba,
        places,
    })
}

/// Writes a bundle: manifest, model, then art, deflated with fixed
/// timestamps, so the same bundle always makes the same bytes. The
/// manifest's `art` object is written from `bundle.art`.
pub fn write(bundle: &Bundle) -> Result<Vec<u8>, BundleError> {
    let mut manifest = bundle.manifest.clone();
    manifest.art = bundle.art.as_ref().map(|a| ArtManifest {
        tile: a.tile,
        pictures: a
            .places
            .iter()
            .map(|(&picture, &at)| ArtEntry { picture, at })
            .collect(),
    });
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .last_modified_time(DateTime::DEFAULT);
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let io = |e: std::io::Error| BundleError::NotAZip(e.to_string());
    let z = |e: zip::result::ZipError| BundleError::NotAZip(e.to_string());

    let json = serde_json::to_vec_pretty(&manifest).map_err(|e| bad(MANIFEST, e))?;
    zip.start_file(MANIFEST, options).map_err(z)?;
    zip.write_all(&json).map_err(io)?;

    let json = serde_json::to_vec_pretty(&bundle.adventure).map_err(|e| bad(ADVENTURE, e))?;
    zip.start_file(ADVENTURE, options).map_err(z)?;
    zip.write_all(&json).map_err(io)?;

    if let Some(a) = &bundle.art {
        let mut png_bytes = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut png_bytes, a.width, a.height);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            let mut w = enc.write_header().map_err(|e| bad(ART, e))?;
            w.write_image_data(&a.rgba).map_err(|e| bad(ART, e))?;
        }
        zip.start_file(ART, options).map_err(z)?;
        zip.write_all(&png_bytes).map_err(io)?;
    }
    Ok(zip.finish().map_err(z)?.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsafe_paths_are_refused() {
        assert!(safe_path("art/pictures.png"));
        assert!(!safe_path("../x"));
        assert!(!safe_path("art/../../x"));
        assert!(!safe_path("/etc/x"));
        assert!(!safe_path("art\\x"));
    }

    #[test]
    fn bundles_are_told_by_extension_or_signature() {
        assert!(is_bundle("x.ADVX", b""));
        assert!(is_bundle("upload", b"PKrest"));
        assert!(!is_bundle("GAME.D64", &[0; 8]));
    }

    #[test]
    fn nothing_is_past_extended_in_extended() {
        assert!(over_extended(&Limits::EXTENDED).is_empty());
        assert!(over_extended(&Limits::CLASSIC).is_empty());
    }
}
