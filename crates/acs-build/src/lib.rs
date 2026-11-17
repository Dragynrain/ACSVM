// SPDX-License-Identifier: GPL-3.0-or-later

//! `acs build`: a source folder of TOML (docs/content.md 3) to an
//! Extended bundle. Everything is named by key; the build gives out the
//! numbers the engine and the rules assume, and checks every reference,
//! range and limit, reporting each problem with its file.

mod obj;

pub use obj::Problem;
use obj::{is_key, point, Obj, Problems};

use acs_bundle::{Art, Bundle, Manifest};
use acs_core::encode::{thing_props, PropFlags};
use acs_core::graphics::{Palette, PaletteColor, Picture, PC_MASTER};
use acs_core::layout::world_map as wm;
use acs_core::limits::{self, Limits, Profile};
use acs_core::model::*;
use acs_core::Platform;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;
use toml::{Table, Value};

/// A source folder's files, by path relative to its root with `/`.
#[derive(Debug, Clone, Default)]
pub struct Folder {
    pub files: BTreeMap<String, Vec<u8>>,
    /// The starter kit `adventure.toml` names (content 4.2).
    pub kit: Option<Box<Folder>>,
}

impl Folder {
    /// Reads every file under `dir`, and the kit `adventure.toml` names.
    pub fn read(dir: &Path) -> std::io::Result<Folder> {
        let mut folder = Folder::read_one(dir)?;
        let kit = folder
            .files
            .get(ADVENTURE)
            .and_then(|b| std::str::from_utf8(b).ok())
            .and_then(|t| t.parse::<Table>().ok())
            .and_then(|t| t.get("kit").and_then(Value::as_str).map(String::from));
        if let Some(k) = kit {
            let path = dir.join(&k);
            let kit = Folder::read_one(&path)
                .map_err(|e| std::io::Error::new(e.kind(), format!("kit {}: {e}", path.display())))?;
            folder.kit = Some(Box::new(kit));
        }
        Ok(folder)
    }

    fn read_one(dir: &Path) -> std::io::Result<Folder> {
        fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) -> std::io::Result<()> {
            for e in std::fs::read_dir(dir)? {
                let path = e?.path();
                if path.is_dir() {
                    walk(root, &path, out)?;
                } else {
                    let rel = path.strip_prefix(root).unwrap_or(&path);
                    let key = rel
                        .components()
                        .map(|c| c.as_os_str().to_string_lossy())
                        .collect::<Vec<_>>()
                        .join("/");
                    out.insert(key, std::fs::read(&path)?);
                }
            }
            Ok(())
        }
        let mut files = BTreeMap::new();
        walk(dir, dir, &mut files)?;
        Ok(Folder { files, kit: None })
    }

    /// Sets the kit (tests).
    pub fn with_kit(mut self, kit: Folder) -> Folder {
        self.kit = Some(Box::new(kit));
        self
    }

    /// Adds a text file (tests).
    pub fn with(mut self, path: &str, text: &str) -> Folder {
        self.files.insert(path.into(), text.as_bytes().to_vec());
        self
    }

    /// Adds a binary file (tests).
    pub fn with_bytes(mut self, path: &str, bytes: Vec<u8>) -> Folder {
        self.files.insert(path.into(), bytes);
        self
    }
}

/// Builds a folder into a bundle, or every problem found.
pub fn build(folder: &Folder) -> Result<Bundle, Vec<Problem>> {
    let p = Problems::default();
    let bundle = Build::new(folder, &p).run();
    let mut problems = p.0.into_inner();
    match bundle {
        Some(b) if problems.is_empty() => Ok(b),
        _ => {
            if problems.is_empty() {
                problems.push(Problem {
                    file: String::new(),
                    at: String::new(),
                    message: "nothing built".into(),
                });
            }
            Err(problems)
        }
    }
}

const ADVENTURE: &str = "adventure.toml";
const PICTURES: &str = "pictures.toml";
const THINGS: &str = "things.toml";
const CREATURES: &str = "creatures.toml";
const MESSAGES: &str = "messages.toml";
const WORLD: &str = "world.toml";
/// A kit's files, as problems name them.
const KIT: &str = "kit/kit.toml";
const KIT_THINGS: &str = "kit/things.toml";
const KIT_CREATURES: &str = "kit/creatures.toml";

/// A `[[thing]]` or `[[creature]]` table and the file it is from.
type Entry = (String, Table);

/// An atlas's tile size and, in list order, each picture's key and pixels.
type Tiles = (u32, Vec<(String, Vec<u8>)>);

/// Fixed thing numbers (content 3.4).
const GOLD: u16 = 81;
const AREA_DOOR: u16 = 82;
const FIRST_FIXTURE: u16 = 83;
const FLOOR: u16 = 127;
const BORDER: u16 = 128;
const CARRIABLE_MAX: u16 = 80;
const STACKABLE_MAX: u16 = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Type {
    Treasure,
    MagicItem,
    Missile,
    Melee,
    Armor,
    Spell,
    Portal,
    Space,
    CustomSpace,
    Obstacle,
    CustomObstacle,
    Store,
}

impl Type {
    fn parse(s: &str) -> Option<Type> {
        Some(match s {
            "treasure" => Type::Treasure,
            "magic_item" => Type::MagicItem,
            "missile_weapon" => Type::Missile,
            "melee_weapon" => Type::Melee,
            "armor" => Type::Armor,
            "magic_spell" => Type::Spell,
            "portal" => Type::Portal,
            "space" => Type::Space,
            "custom_space" => Type::CustomSpace,
            "obstacle" => Type::Obstacle,
            "custom_obstacle" => Type::CustomObstacle,
            "store" => Type::Store,
            _ => return None,
        })
    }

    fn carriable(self) -> bool {
        matches!(
            self,
            Type::Treasure | Type::MagicItem | Type::Missile | Type::Melee | Type::Armor | Type::Spell
        )
    }

    /// May replace a wall square (movement 5.2).
    fn in_walls(self) -> bool {
        matches!(self, Type::Portal | Type::Obstacle | Type::CustomObstacle)
    }
}

/// A room's place, known after the first pass over the regions.
struct RoomInfo {
    region: u8,
    room: u8,
    width: u8,
    height: u8,
    x: u8,
    y: u8,
}

struct Build<'a> {
    folder: &'a Folder,
    p: &'a Problems,
    tables: HashMap<String, Table>,
    limits: Limits,
    palette: Palette,
    pictures: HashMap<String, u16>,
    long: HashMap<String, u16>,
    short: HashMap<String, u8>,
    things: HashMap<String, (u16, Type)>,
    creatures: HashMap<String, u16>,
    masters: Vec<Creature>,
    rooms: HashMap<String, RoomInfo>,
    world_size: (u8, u8),
    /// `[floor] wall`, every room's wall unless it names one.
    default_wall: u16,
    exit_points: Vec<Option<MapPoint>>,
}

impl<'a> Build<'a> {
    fn new(folder: &'a Folder, p: &'a Problems) -> Build<'a> {
        Build {
            folder,
            p,
            tables: HashMap::new(),
            limits: Limits::CLASSIC,
            palette: Palette { colors: Vec::new() },
            pictures: HashMap::new(),
            long: HashMap::new(),
            short: HashMap::new(),
            things: HashMap::new(),
            creatures: HashMap::new(),
            masters: Vec::new(),
            rooms: HashMap::new(),
            world_size: (0, 0),
            default_wall: 0,
            exit_points: vec![None; wm::DOOR_COUNT],
        }
    }

    /// Parses a TOML file into `tables`; false if it is missing or broken.
    fn load(&mut self, name: &str, required: bool) -> bool {
        let folder = self.folder;
        self.load_from(folder, name, name, required)
    }

    /// Parses `path` in `folder` into `tables` as `name`.
    fn load_from(&mut self, folder: &Folder, path: &str, name: &str, required: bool) -> bool {
        let Some(bytes) = folder.files.get(path) else {
            if required {
                self.p.push(name, "", "missing");
            }
            return false;
        };
        let Ok(text) = std::str::from_utf8(bytes) else {
            self.p.push(name, "", "not UTF-8 text");
            return false;
        };
        match text.parse::<Table>() {
            Ok(t) => {
                self.tables.insert(name.into(), t);
                true
            }
            Err(e) => {
                self.p.push(name, "", e.to_string().trim_end().to_string());
                false
            }
        }
    }

    fn run(mut self) -> Option<Bundle> {
        let mut ok = true;
        for f in [ADVENTURE, THINGS, CREATURES, WORLD] {
            ok &= self.load(f, true);
        }
        self.load(MESSAGES, false);
        let names_kit = self.tables.get(ADVENTURE).is_some_and(|t| t.contains_key("kit"));
        let has_pictures = self.load(PICTURES, false);
        if !names_kit && !has_pictures {
            self.p.push(PICTURES, "", "missing");
            ok = false;
        }
        if names_kit {
            match self.folder.kit.as_deref() {
                None => {
                    self.p.push(ADVENTURE, "", "kit: the kit's folder wasn't read");
                    ok = false;
                }
                Some(kit) => {
                    ok &= self.load_from(kit, "kit.toml", KIT, true);
                    self.load_from(kit, "things.toml", KIT_THINGS, false);
                    self.load_from(kit, "creatures.toml", KIT_CREATURES, false);
                }
            }
        }
        let region_files: Vec<String> = self
            .folder
            .files
            .keys()
            .filter(|k| k.starts_with("regions/") && k.ends_with(".toml") && k.matches('/').count() == 1)
            .cloned()
            .collect();
        for f in &region_files {
            ok &= self.load(f, true);
        }
        if region_files.is_empty() {
            self.p.push("regions/", "", "no regions (regions/*.toml)");
        }
        if !ok {
            return None;
        }
        let tables = std::mem::take(&mut self.tables);
        let adv_t = &tables[ADVENTURE];
        let a = Obj::new(adv_t, ADVENTURE, "", self.p);
        a.raw("kit");
        let header = self.header(&a);

        // The kit's entries this adventure takes (content 4.2).
        let own_things = self.entries(THINGS, tables.get(THINGS), "thing");
        let own_creatures = self.entries(CREATURES, tables.get(CREATURES), "creature");
        let kit_things = self.entries(KIT_THINGS, tables.get(KIT_THINGS), "thing");
        let kit_creatures = self.entries(KIT_CREATURES, tables.get(KIT_CREATURES), "creature");
        let mut words = HashSet::new();
        for (name, t) in &tables {
            if !name.starts_with("kit/") {
                strings(&Value::Table(t.clone()), &mut words);
            }
        }
        let own_keys: HashSet<String> = own_things
            .iter()
            .chain(&own_creatures)
            .map(|(_, t)| entry_key(t))
            .collect();
        loop {
            let before = words.len();
            for (_, t) in kit_things.iter().chain(&kit_creatures) {
                let k = entry_key(t);
                if words.contains(&k) && !own_keys.contains(&k) {
                    strings(&Value::Table(t.clone()), &mut words);
                }
            }
            if words.len() == before {
                break;
            }
        }
        let things_list = merge(kit_things, own_things, &words);
        let creatures_list = merge(kit_creatures, own_creatures, &words);
        let (pictures, art) = self.pictures(tables.get(PICTURES), tables.get(KIT), &words)?;
        let (long_messages, short_messages) = match tables.get(MESSAGES) {
            Some(t) => self.messages(t),
            None => (Vec::new(), Vec::new()),
        };
        self.number_things(&things_list);
        self.number_creatures(&creatures_list);
        let things = self.things(&things_list, &a);
        self.masters = self.creature_list(&creatures_list, &header.classes);
        let regions_first: Vec<(&str, &Table)> =
            region_files.iter().map(|f| (f.as_str(), &tables[f])).collect();
        self.room_places(&regions_first);
        let world = self.world(&tables[WORLD]);
        let regions: Vec<Region> = regions_first
            .iter()
            .enumerate()
            .map(|(i, (f, t))| self.region(f, t, i as u8 + 1))
            .collect();
        a.done();
        let world = WorldMap {
            exit_points: self.exit_points.clone(),
            ..world?
        };

        let adventure = Adventure {
            model_version: MODEL_VERSION,
            platform: Platform::Pc,
            profile: Profile::Extended,
            limits: self.limits,
            name: header.name,
            byline: header.byline.clone(),
            intro: header.intro,
            theme: header.theme,
            palette: self.palette.clone(),
            pictures,
            things,
            creature_classes: header.classes,
            creatures: self.masters.clone(),
            players: Vec::new(),
            retired_players: Vec::new(),
            world,
            regions,
            long_messages,
            short_messages,
        };
        for v in limits::check(&adventure) {
            self.p.push(ADVENTURE, "limits", v.to_string());
        }
        let mut manifest = Manifest::new(&adventure, &header.license);
        manifest.byline = header.byline;
        manifest.credits = header.credits;
        Some(Bundle {
            manifest,
            adventure,
            art: Some(art),
        })
    }

    // -------------------------------------------------------------------
    // adventure.toml

    fn header(&mut self, a: &Obj) -> Header {
        let name = a.need_str("name").unwrap_or_default().to_string();
        let byline = a.str("byline").unwrap_or_default().to_string();
        let license = a.str("license").unwrap_or("CC BY-SA 4.0").to_string();
        let intro = a.str("intro").unwrap_or_default().to_string();
        let credits = a
            .array("credits")
            .iter()
            .filter_map(|v| match v {
                Value::String(s) => Some(s.clone()),
                _ => {
                    a.err("credits: should be a list of text");
                    None
                }
            })
            .collect();
        let theme = match a.str("theme").unwrap_or("fantasy") {
            "fantasy" => Theme::Fantasy,
            "spy_mystery" => Theme::SpyMystery,
            "sci_fi" => Theme::SciFi,
            t => {
                a.err(format!("theme: {t:?} is not fantasy, spy_mystery or sci_fi"));
                Theme::Fantasy
            }
        };
        let names: Vec<String> = match a.raw("palette") {
            None => ["black", "cyan", "magenta", "white"].map(String::from).to_vec(),
            Some(Value::Array(v)) if v.len() == 4 => {
                v.iter().map(|c| c.as_str().unwrap_or("").to_string()).collect()
            }
            Some(_) => {
                a.err("palette: should be four PC colour names");
                ["black", "cyan", "magenta", "white"].map(String::from).to_vec()
            }
        };
        let colors = names
            .iter()
            .map(|n| {
                let code = PC_MASTER.names.iter().position(|m| m == n).unwrap_or_else(|| {
                    a.err(format!(
                        "palette: {n:?} is not a PC colour ({})",
                        PC_MASTER.names.join(", ")
                    ));
                    0
                });
                PC_MASTER.color(code as u8)
            })
            .collect::<Vec<PaletteColor>>();
        self.palette = Palette { colors };
        let classes: Vec<String> = a
            .array("classes")
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
        if let Some(t) = a.table("limits") {
            let o = a.child(t, "limits");
            self.limits = self.limits_over(&o);
            o.done();
        }
        // The built-ins are read with the things; mark them read here.
        for k in ["gold", "area_door", "floor"] {
            a.raw(k);
        }
        Header {
            name,
            byline,
            license,
            intro,
            credits,
            theme,
            classes,
        }
    }

    fn limits_over(&self, o: &Obj) -> Limits {
        let e = Limits::EXTENDED;
        let mut l = Limits::CLASSIC;
        macro_rules! over {
            ($($f:ident),*) => {$(
                if let Some(v) = o.int(stringify!($f), 0, e.$f as i64) {
                    l.$f = v as u16;
                }
            )*};
        }
        over!(
            world_size,
            terrain_types,
            map_doors,
            regions,
            rooms_per_region,
            room_size,
            residents_per_region,
            random_creatures_per_region,
            creatures_per_room,
            things,
            creatures,
            creature_classes,
            pictures,
            long_messages,
            short_messages,
            name_length
        );
        l
    }

    // -------------------------------------------------------------------
    // pictures.toml

    /// One atlas's listed tiles, cut out: its tile size and, in list
    /// order, each picture's key and pixels.
    fn atlas(&self, folder: &Folder, file: &str, t: &Table) -> Option<Tiles> {
        let o = Obj::new(t, file, "", self.p);
        if file == KIT {
            for k in ["name", "license", "credits"] {
                o.raw(k);
            }
        }
        let atlas = o.need_str("atlas");
        let tile = o.need_int("tile", 16, 512);
        let list = o.table("pictures");
        o.done();
        let (atlas, tile, list) = (atlas?, tile? as u32, list?);
        let Some(bytes) = folder.files.get(atlas) else {
            o.err(format!("atlas: {atlas} is missing"));
            return None;
        };
        let (width, height, rgba) = match decode_png(bytes) {
            Ok(x) => x,
            Err(e) => {
                o.err(format!("atlas {atlas}: {e}"));
                return None;
            }
        };
        if width % tile != 0 || height % tile != 0 {
            o.err(format!("tile {tile} doesn't divide the {width} x {height} atlas"));
            return None;
        }
        let places = (width / tile) * (height / tile);
        let mut art = Art {
            tile,
            width,
            height,
            rgba,
            places: BTreeMap::new(),
        };
        let mut out = Vec::new();
        for (key, v) in list {
            let at = format!("picture {key}");
            if !is_key(key) {
                self.p.push(file, &at, "not a key (lowercase letters, digits, _)");
            }
            match v {
                Value::Integer(n) if (0..places as i64).contains(n) => {
                    art.places.insert(0, *n as u32);
                    out.push((key.clone(), art.picture(0).expect("placed")));
                }
                _ => self
                    .p
                    .push(file, &at, format!("should be an atlas place, 0..{}", places - 1)),
            }
        }
        Some((tile, out))
    }

    /// The pictures (content 3.3, 4.2): the kit's taken ones (an own one
    /// of the same key in its place), then the adventure's own; one atlas
    /// of just those, place n - 1 for picture n.
    fn pictures(
        &mut self,
        own: Option<&Table>,
        kit: Option<&Table>,
        words: &HashSet<String>,
    ) -> Option<(Vec<Picture>, Art)> {
        let folder = self.folder;
        let own = own.and_then(|t| self.atlas(folder, PICTURES, t));
        let kit = match (kit, folder.kit.as_deref()) {
            (Some(t), Some(f)) => self.atlas(f, KIT, t),
            _ => None,
        };
        let tile = own.as_ref().or(kit.as_ref())?.0;
        if let (Some(o), Some(k)) = (&own, &kit) {
            if o.0 != k.0 {
                self.p
                    .push(PICTURES, "", format!("tile {} but the kit's is {}", o.0, k.0));
                return None;
            }
        }
        let own_list: &[(String, Vec<u8>)] = own.as_ref().map_or(&[], |o| &o.1);
        let mut chosen: Vec<(&str, &[u8])> = Vec::new();
        if let Some((_, list)) = &kit {
            for (key, px) in list {
                let mine = own_list.iter().find(|(k, _)| k == key);
                if mine.is_some() || words.contains(key) {
                    chosen.push((key, mine.map_or(px.as_slice(), |m| m.1.as_slice())));
                }
            }
        }
        for (key, px) in own_list {
            if !chosen.iter().any(|(k, _)| k == key) {
                chosen.push((key, px));
            }
        }
        const PER_ROW: usize = 16;
        let rows = chosen.len().div_ceil(PER_ROW).max(1);
        let (width, height) = (PER_ROW as u32 * tile, rows as u32 * tile);
        let row = (tile * 4) as usize;
        let mut rgba = vec![0u8; (width * height * 4) as usize];
        let mut places = BTreeMap::new();
        let mut pictures = vec![Picture {
            width: 16,
            height: 16,
            pixel_aspect: 1,
            pixels: vec![0; 256],
        }];
        for (i, (key, px)) in chosen.iter().enumerate() {
            let (cx, cy) = ((i % PER_ROW) as u32 * tile, (i / PER_ROW) as u32 * tile);
            for y in 0..tile {
                let dst = (((cy + y) * width + cx) * 4) as usize;
                let src = y as usize * row;
                rgba[dst..dst + row].copy_from_slice(&px[src..src + row]);
            }
            let id = pictures.len() as u16;
            self.pictures.insert(key.to_string(), id);
            places.insert(id, i as u32);
            pictures.push(classic_pixels(px, tile as usize, &self.palette));
        }
        Some((
            pictures,
            Art {
                tile,
                width,
                height,
                rgba,
                places,
            },
        ))
    }

    fn picture(&self, o: &Obj, key: &str) -> u16 {
        match o.str(key) {
            Some(k) => self.pictures.get(k).copied().unwrap_or_else(|| {
                o.err(format!("{key}: no picture {k:?}"));
                0
            }),
            None => {
                if !o.has(key) {
                    o.err(format!("{key}: missing"));
                }
                0
            }
        }
    }

    // -------------------------------------------------------------------
    // messages.toml

    fn messages(&mut self, t: &Table) -> (Vec<String>, Vec<String>) {
        let o = Obj::new(t, MESSAGES, "", self.p);
        let text = |section: &str| -> Vec<(String, String)> {
            let Some(t) = o.table(section) else {
                return Vec::new();
            };
            t.iter()
                .filter_map(|(k, v)| {
                    let at = format!("{section} {k}");
                    if !is_key(k) {
                        self.p
                            .push(MESSAGES, &at, "not a key (lowercase letters, digits, _)");
                    }
                    match v {
                        Value::String(s) if s.chars().count() <= 4000 => Some((k.clone(), s.clone())),
                        Value::String(_) => {
                            self.p.push(MESSAGES, &at, "longer than 4,000 characters");
                            None
                        }
                        _ => {
                            self.p.push(MESSAGES, &at, "should be text");
                            None
                        }
                    }
                })
                .collect()
        };
        let long = text("long");
        let short = text("short");
        o.done();
        if short.len() > u8::MAX as usize {
            o.err(format!("{} short messages; at most 255", short.len()));
        }
        for (i, (k, _)) in long.iter().enumerate() {
            self.long.insert(k.clone(), i as u16 + 1);
        }
        for (i, (k, _)) in short.iter().enumerate() {
            self.short.insert(k.clone(), (i + 1).min(255) as u8);
        }
        (
            long.into_iter().map(|(_, s)| s).collect(),
            short.into_iter().map(|(_, s)| s).collect(),
        )
    }

    fn message(&self, o: &Obj, key: &str) -> Option<MessageRef> {
        let (k, shown) = match o.raw(key)? {
            Value::String(s) => (s.as_str(), true),
            Value::Table(t) => {
                let m = o.child(t, format!("{} {key}", o.at));
                let k = m.need_str("key");
                let shown = !m.has("shown") || m.bool("shown");
                m.done();
                (k?, shown)
            }
            _ => {
                o.err(format!("{key}: should be a short message key"));
                return None;
            }
        };
        match self.short.get(k) {
            Some(&slot) => Some(MessageRef { slot, shown }),
            None => {
                o.err(format!("{key}: no short message {k:?}"));
                None
            }
        }
    }

    // -------------------------------------------------------------------
    // things.toml

    /// A file's `[[name]]` tables, each with the file.
    fn entries(&self, file: &str, t: Option<&Table>, name: &str) -> Vec<Entry> {
        match t.and_then(|t| t.get(name)) {
            None => Vec::new(),
            Some(Value::Array(a)) => a
                .iter()
                .filter_map(|v| {
                    let t = v.as_table();
                    if t.is_none() {
                        self.p
                            .push(file, "", format!("each [[{name}]] should be a table"));
                    }
                    t.map(|t| (file.to_string(), t.clone()))
                })
                .collect(),
            Some(_) => {
                self.p
                    .push(file, "", format!("{name}: should be [[{name}]] tables"));
                Vec::new()
            }
        }
    }

    fn number_things(&mut self, list: &[Entry]) {
        let (mut carriable, mut fixture) = (1u16, FIRST_FIXTURE);
        for (file, e) in list {
            let key = e.get("key").and_then(Value::as_str).unwrap_or("");
            let at = format!("thing {key}");
            if !is_key(key) {
                self.p.push(
                    file,
                    &at,
                    "key: missing or not a key (lowercase letters, digits, _)",
                );
                continue;
            }
            let Some(ty) = e.get("type").and_then(Value::as_str).and_then(Type::parse) else {
                self.p.push(
                    file,
                    &at,
                    "type: missing or not one of treasure, magic_item, missile_weapon, melee_weapon, armor, magic_spell, portal, space, custom_space, obstacle, custom_obstacle, store",
                );
                continue;
            };
            let number = if ty.carriable() {
                let n = carriable;
                carriable += 1;
                if n > CARRIABLE_MAX {
                    self.p.push(file, &at, "more than 80 carriable things");
                }
                n
            } else {
                let n = fixture;
                fixture += 1;
                if fixture == FLOOR {
                    fixture = BORDER + 1;
                }
                n
            };
            if self.things.insert(key.into(), (number, ty)).is_some() {
                self.p.push(file, &at, "key used twice");
            }
        }
    }

    fn thing_ref(&self, o: &Obj, field: &str, k: &str) -> Option<(u16, Type)> {
        let r = self.things.get(k).copied();
        if r.is_none() {
            o.err(format!("{field}: no thing {k:?}"));
        }
        r
    }

    fn things(&mut self, list: &[Entry], a: &Obj) -> Vec<Thing> {
        let mut out: BTreeMap<u16, Thing> = BTreeMap::new();
        let mut table_index = 0u8;
        for (file, e) in list {
            let key = e.get("key").and_then(Value::as_str).unwrap_or("");
            let Some(&(number, ty)) = self.things.get(key) else {
                continue;
            };
            let o = Obj::new(e, file, format!("thing {key}"), self.p);
            o.raw("key");
            o.raw("type");
            let name = o.need_str("name").unwrap_or_default().to_string();
            let may_add = o.int_or("may_add", 0, 15, 0) as u8;
            let flags_index = if matches!(ty, Type::MagicItem | Type::Missile | Type::Melee | Type::Armor) {
                table_index = table_index.wrapping_add(1);
                table_index - 1
            } else {
                0
            };
            let (kind, flags, blocked, spell_msg) = self.thing_kind(&o, ty, flags_index);
            o.done();
            let raw = match thing_props(&kind, flags) {
                Ok(r) => r,
                Err(e) => {
                    o.err(e);
                    [0; 4]
                }
            };
            out.insert(
                number,
                Thing {
                    number,
                    name,
                    acs_may_add: may_add,
                    kind,
                    blocked_message: blocked,
                    spell_message: spell_msg,
                    raw,
                },
            );
        }
        // The built-ins (content 3.2).
        let builtin = |key: &str| a.table(key).map(|t| a.child(t, key));
        let gold = builtin("gold");
        let (gname, gpic, gweight) = match &gold {
            Some(o) => (
                o.str("name").unwrap_or("GOLD").to_string(),
                self.picture(o, "picture"),
                o.int_or("weight", 0, 255, 1) as i32,
            ),
            None => {
                a.err("[gold]: missing (its picture)");
                ("GOLD".into(), 0, 1)
            }
        };
        let door = builtin("area_door");
        let (dname, dpic) = match &door {
            Some(o) => (
                o.str("name").unwrap_or("EXIT").to_string(),
                self.picture(o, "picture"),
            ),
            None => {
                a.err("[area_door]: missing (its picture)");
                ("EXIT".into(), 0)
            }
        };
        let floor = builtin("floor");
        let (fpic, wall) = match &floor {
            Some(o) => (
                if o.has("picture") {
                    self.picture(o, "picture")
                } else {
                    0
                },
                if o.has("wall") { self.picture(o, "wall") } else { 0 },
            ),
            None => (0, 0),
        };
        self.default_wall = wall;
        for o in [&gold, &door, &floor].into_iter().flatten() {
            o.done();
        }
        let mut put = |number: u16, name: String, kind: ThingKind| {
            let raw = thing_props(&kind, PropFlags::default()).unwrap_or([0; 4]);
            out.insert(
                number,
                Thing {
                    number,
                    name,
                    acs_may_add: 0,
                    kind,
                    blocked_message: None,
                    spell_message: None,
                    raw,
                },
            );
        };
        put(
            GOLD,
            gname,
            ThingKind::Treasure {
                picture: gpic,
                disappears_when_dropped: false,
                carry: Carry {
                    weight: gweight,
                    value: 1,
                },
            },
        );
        put(
            AREA_DOOR,
            dname,
            ThingKind::Portal {
                picture: dpic,
                two_way: true,
                access: Access::Anyone,
                destroy_needed_thing: false,
            },
        );
        put(
            FLOOR,
            "EMPTY FLOOR".into(),
            ThingKind::RoomFloor { picture: fpic },
        );
        put(
            BORDER,
            "ROOM BORDER".into(),
            ThingKind::RoomFloor { picture: wall },
        );
        let last = out.keys().next_back().copied().unwrap_or(BORDER).max(BORDER);
        (1..=last)
            .map(|n| {
                out.remove(&n).unwrap_or(Thing {
                    number: n,
                    name: String::new(),
                    acs_may_add: 0,
                    kind: ThingKind::Undefined,
                    blocked_message: None,
                    spell_message: None,
                    raw: [15, 0, 0, 0],
                })
            })
            .collect()
    }

    fn carry(&self, o: &Obj) -> Carry {
        Carry {
            weight: o.int_or("weight", -5100, 5100, 0) as i32,
            value: o.int_or("value", 0, 5100, 0) as i32,
        }
    }

    fn weapon(&self, o: &Obj, missile: bool, armor: bool) -> WeaponStats {
        let attack = o.int_or("attack", -65, 40, 0) as i8;
        let _ = armor;
        WeaponStats {
            power: o.int_or("power", 0, 31, 0) as u8,
            range: if missile {
                Some(o.int("range", 2, 9).unwrap_or_else(|| {
                    if !o.has("range") {
                        o.err("range: missing (2..9)");
                    }
                    2
                }) as u8)
            } else {
                None
            },
            break_chance: o.int_or("break", 0, 15, 0) as u8,
            attack_adjustment: attack,
            usable_by_anyone: o.bool("anyone"),
            magic: o.bool("magic"),
        }
    }

    fn need_spell(&self, o: &Obj) -> SpellAction {
        match o.raw("spell") {
            Some(v) => self.spell(o, "spell", v),
            None => {
                o.err("spell: missing");
                nothing()
            }
        }
    }

    fn opt_spell(&self, o: &Obj) -> SpellAction {
        match o.raw("spell") {
            Some(v) => self.spell(o, "spell", v),
            None => nothing(),
        }
    }

    fn thing_kind(
        &self,
        o: &Obj,
        ty: Type,
        table_index: u8,
    ) -> (ThingKind, PropFlags, Option<MessageRef>, Option<MessageRef>) {
        let mut flags = PropFlags {
            table_index,
            ..Default::default()
        };
        let (mut blocked, mut spell_msg) = (None, None);
        let kind = match ty {
            Type::Treasure => ThingKind::Treasure {
                picture: self.picture(o, "picture"),
                disappears_when_dropped: o.bool("once"),
                carry: self.carry(o),
            },
            Type::MagicItem => {
                spell_msg = self.message(o, "spell_message");
                ThingKind::MagicItem {
                    picture: self.picture(o, "picture"),
                    disappears_after_use: o.bool("once"),
                    carry: self.carry(o),
                    action: self.need_spell(o),
                    on_use: o.bool("on_use"),
                    on_drop: o.bool("on_drop"),
                    on_pickup: o.bool("on_pickup"),
                }
            }
            Type::Missile | Type::Melee | Type::Armor => {
                let picture = self.picture(o, "picture");
                let disappears_after_use = o.bool("once");
                let carry = self.carry(o);
                let stats = self.weapon(o, ty == Type::Missile, ty == Type::Armor);
                match ty {
                    Type::Missile => ThingKind::MissileWeapon {
                        picture,
                        disappears_after_use,
                        carry,
                        stats,
                    },
                    Type::Melee => ThingKind::MeleeWeapon {
                        picture,
                        disappears_after_use,
                        carry,
                        stats,
                    },
                    _ => ThingKind::Armor {
                        picture,
                        disappears_after_use,
                        carry,
                        stats,
                    },
                }
            }
            Type::Spell => {
                spell_msg = self.message(o, "spell_message");
                ThingKind::MagicSpell {
                    disappears_after_use: o.bool("once"),
                    power: o.int_or("power", 0, 127, 0) as u8,
                    action: self.need_spell(o),
                }
            }
            Type::Portal => {
                blocked = self.message(o, "blocked");
                ThingKind::Portal {
                    picture: self.picture(o, "picture"),
                    two_way: o.bool("two_way"),
                    access: self.access(o),
                    destroy_needed_thing: o.bool("destroy_needed"),
                }
            }
            Type::Space => {
                blocked = self.message(o, "blocked");
                spell_msg = self.message(o, "spell_message");
                flags.once = o.bool("once");
                ThingKind::Space {
                    picture: self.picture(o, "picture"),
                    access: self.access(o),
                }
            }
            Type::CustomSpace => {
                blocked = self.message(o, "blocked");
                spell_msg = self.message(o, "spell_message");
                flags.once = o.bool("once");
                let trigger = match o.need_str("trigger").unwrap_or("dropped_here") {
                    "dropped_here" => CustomTrigger::WhenDroppedHere,
                    "moved_onto" => CustomTrigger::WhenMovedOnto,
                    "owners_of" => CustomTrigger::OpenToOwnersOf,
                    "non_owners_of" => CustomTrigger::OpenToNonOwnersOf,
                    t => {
                        o.err(format!(
                            "trigger: {t:?} is not dropped_here, moved_onto, owners_of or non_owners_of"
                        ));
                        CustomTrigger::WhenDroppedHere
                    }
                };
                let choice = match o.str("choice").unwrap_or("thing") {
                    "thing" => PlacementChoice::Thing,
                    "spell_parameter" => PlacementChoice::SpellParameter,
                    c => {
                        o.err(format!("choice: {c:?} is not thing or spell_parameter"));
                        PlacementChoice::Thing
                    }
                };
                ThingKind::CustomSpace {
                    picture: self.picture(o, "picture"),
                    placement_choice: choice,
                    trigger,
                    action: self.need_spell(o),
                    destroy_needed_thing: o.bool("destroy_needed"),
                }
            }
            Type::Obstacle => {
                blocked = self.message(o, "blocked");
                spell_msg = self.message(o, "spell_message");
                flags.once = o.bool("once");
                ThingKind::Obstacle {
                    picture: self.picture(o, "picture"),
                    bump_invokes_spell: o.bool("bump"),
                    action: self.opt_spell(o),
                }
            }
            Type::CustomObstacle => {
                spell_msg = self.message(o, "spell_message");
                flags.once = o.bool("once");
                flags.bump = o.bool("bump");
                flags.placed_parameter = o.bool("placed_parameter");
                ThingKind::CustomObstacle {
                    picture: self.picture(o, "picture"),
                    action: self.opt_spell(o),
                }
            }
            Type::Store => ThingKind::Store {
                picture: self.picture(o, "picture"),
            },
        };
        (kind, flags, blocked, spell_msg)
    }

    fn access(&self, o: &Obj) -> Access {
        match o.raw("access") {
            None => Access::Anyone,
            Some(Value::String(s)) if s == "anyone" => Access::Anyone,
            Some(Value::Table(t)) if t.len() == 1 => {
                let a = o.child(t, format!("{} access", o.at));
                let r = if let Some(k) = a.str("owners_of") {
                    self.thing_ref(&a, "owners_of", k)
                        .map(|(n, _)| Access::OwnersOf { thing: n })
                } else if let Some(k) = a.str("non_owners_of") {
                    self.thing_ref(&a, "non_owners_of", k)
                        .map(|(n, _)| Access::NonOwnersOf { thing: n })
                } else {
                    a.raw("spell").map(|v| Access::InvokeSpell {
                        action: self.spell(&a, "spell", v),
                    })
                };
                a.done();
                r.unwrap_or(Access::Anyone)
            }
            Some(_) => {
                o.err("access: should be \"anyone\", { owners_of = KEY }, { non_owners_of = KEY } or { spell = SPELL }");
                Access::Anyone
            }
        }
    }

    // -------------------------------------------------------------------
    // Spells (content 3.5)

    fn spell(&self, o: &Obj, field: &str, v: &Value) -> SpellAction {
        let Value::Table(t) = v else {
            o.err(format!("{field}: should be a table, {{ action = ... }}"));
            return nothing();
        };
        let s = o.child(t, format!("{} {field}", o.at));
        use SpellActionKind as K;
        let action = s.need_str("action").unwrap_or("nothing");
        let (kind, parameter) = match action {
            "nothing" => (K::Nothing, 0),
            "activate_all" => (K::ActivateAll, 0),
            "kill_all_but_owner_of" | "give" | "rid_room_of" | "add_to_room" => {
                let kind = match action {
                    "kill_all_but_owner_of" => K::KillAllButOwnerOf,
                    "give" => K::Give,
                    "rid_room_of" => K::RidRoomOf,
                    _ => K::AddToRoom,
                };
                let n = s
                    .need_str("thing")
                    .and_then(|k| self.thing_ref(&s, "thing", k))
                    .map_or(0, |(n, _)| n);
                (kind, n)
            }
            "summon_or_banish" => {
                let n = match s.str("creature") {
                    None => 0,
                    Some(k) => self.creatures.get(k).copied().unwrap_or_else(|| {
                        s.err(format!("creature: no creature {k:?}"));
                        0
                    }),
                };
                (K::SummonOrBanish, n)
            }
            "increase_magic_defense" | "decrease_magic_defense" => {
                let kind = if action.starts_with("increase") {
                    K::IncreaseMagicDefense
                } else {
                    K::DecreaseMagicDefense
                };
                let level = s.need_str("level").map_or(0, |l| {
                    magic_level(l).unwrap_or_else(|| {
                        s.err(format!(
                            "level: {l:?} is not none, non_magic_half, only_magic or magic_half"
                        ));
                        0
                    })
                });
                (kind, level as u16)
            }
            "increase_stat" | "decrease_stat" => {
                let kind = if action.starts_with("increase") {
                    K::IncreaseStat
                } else {
                    K::DecreaseStat
                };
                const STATS: [&str; 10] = [
                    "constitution",
                    "strength",
                    "dexterity",
                    "speed",
                    "wisdom",
                    "dodge",
                    "parry",
                    "armor",
                    "melee",
                    "missile",
                ];
                let n = s.need_str("stat").map_or(0, |st| {
                    STATS.iter().position(|x| *x == st).unwrap_or_else(|| {
                        s.err(format!("stat: {st:?} is not one of {}", STATS.join(", ")));
                        0
                    })
                });
                (kind, n as u16)
            }
            "change_power" | "change_life_force" => {
                let kind = if action == "change_power" {
                    K::ChangePower
                } else {
                    K::ChangeLifeForce
                };
                let by = s.need_int("by", -63, 63).unwrap_or(0);
                let temporary = s.bool("temporary");
                let p = by.unsigned_abs() as u16 | ((by < 0) as u16) << 7 | (temporary as u16) << 6;
                (kind, p)
            }
            "long_message" => {
                let n = s.need_str("message").map_or(0, |k| {
                    self.long.get(k).copied().unwrap_or_else(|| {
                        s.err(format!("message: no long message {k:?}"));
                        0
                    })
                });
                (K::LongMessage, n)
            }
            "play_music" => (
                K::PlayMusic,
                s.need_int("music", 1, 30).map_or(0, |m| m as u16 - 1),
            ),
            a => {
                s.err(format!(
                    "action: {a:?} is not a spell action (docs/content.md 3.5)"
                ));
                (K::Nothing, 0)
            }
        };
        s.done();
        SpellAction { kind, parameter }
    }

    // -------------------------------------------------------------------
    // creatures.toml

    fn number_creatures(&mut self, list: &[Entry]) {
        for (i, (file, e)) in list.iter().enumerate() {
            let key = e.get("key").and_then(Value::as_str).unwrap_or("");
            if !is_key(key) {
                self.p
                    .push(file, &format!("creature {}", i + 1), "key: missing or not a key");
                continue;
            }
            if self.creatures.insert(key.into(), i as u16 + 1).is_some() {
                self.p.push(file, &format!("creature {key}"), "key used twice");
            }
        }
    }

    fn creature_list(&self, list: &[Entry], classes: &[String]) -> Vec<Creature> {
        list.iter()
            .enumerate()
            .map(|(i, (file, e))| {
                let key = e.get("key").and_then(Value::as_str).unwrap_or("");
                let o = Obj::new(e, file, format!("creature {key}"), self.p);
                o.raw("key");
                let c = self.creature(&o, i as u16 + 1, classes);
                o.done();
                c
            })
            .collect()
    }

    fn creature(&self, o: &Obj, number: u16, classes: &[String]) -> Creature {
        let name = o.need_str("name").unwrap_or_default().to_string();
        let class = match o.str("class") {
            None => 0,
            Some(c) => classes.iter().position(|x| x == c).unwrap_or_else(|| {
                o.err(format!("class: {c:?} is not in adventure.toml's classes"));
                0
            }) as u8,
        };
        let picture = self.picture(o, "picture");
        let constitution = o.int_or("constitution", 0, 63, 0) as u8;
        let wisdom = o.int_or("wisdom", 0, 31, 0) as u8;
        let alignment = match o.str("alignment").unwrap_or("neutral") {
            "thief" => Alignment::Thief,
            "neutral" => Alignment::Neutral,
            "enemy" => Alignment::Enemy,
            "friend" => Alignment::Friend,
            a => {
                o.err(format!("alignment: {a:?} is not thief, neutral, enemy or friend"));
                Alignment::Neutral
            }
        };
        let magic_defense = match o.str("magic_defense") {
            None => MagicDefense::None,
            Some(l) => MagicDefense::from_code(magic_level(l).unwrap_or_else(|| {
                o.err(format!(
                    "magic_defense: {l:?} is not none, non_magic_half, only_magic or magic_half"
                ));
                0
            })),
        };
        let mut possessions: Vec<Possession> = Vec::new();
        for (i, v) in o.array("carries").iter().enumerate() {
            let Some(t) = v.as_table() else {
                o.err("carries: should be a list of { thing = KEY, count = N }");
                continue;
            };
            let c = o.child(t, format!("{} carries {}", o.at, i + 1));
            if let Some(p) = self.possession(&c) {
                if possessions.iter().any(|q| q.thing == p.thing) {
                    c.err("thing listed twice");
                }
                possessions.push(p);
            }
            c.done();
        }
        possessions.sort_by_key(|p| p.thing);
        let readied = |field: &str, ok: &dyn Fn(Type) -> bool, what: &str| -> u16 {
            let Some(k) = o.str(field) else { return 0 };
            let Some((n, ty)) = self.thing_ref(o, field, k) else {
                return 0;
            };
            if !ok(ty) {
                o.err(format!("{field}: {k} is not {what}"));
            }
            if !possessions.iter().any(|p| p.thing == n) {
                o.err(format!("{field}: {k} must also be carried"));
            }
            n
        };
        let weapon = readied(
            "weapon",
            &|t| matches!(t, Type::Melee | Type::Missile),
            "a weapon",
        );
        let armor = readied("armor", &|t| t == Type::Armor, "armor");
        Creature {
            number,
            in_use: true,
            name,
            class: Some(class),
            picture: Some(picture),
            constitution,
            life_force: o.int_or("life_force", 0, 63, constitution as i64) as u8,
            strength: o.int_or("strength", 0, 31, 0) as u8,
            dexterity: o.int_or("dexterity", 0, 31, 0) as u8,
            speed: o.int_or("speed", 0, 15, 0) as u8,
            wisdom,
            power: o.int_or("power", 0, 127, wisdom as i64) as u8,
            size: o.int_or("size", 0, 31, 0) as u8,
            dodge: o.int_or("dodge", 0, 127, 0) as u8,
            parry: o.int_or("parry", 0, 127, 0) as u8,
            armor_skill: o.int_or("armor_skill", 0, 127, 0) as u8,
            melee: o.int_or("melee", 0, 127, 0) as u8,
            missile: o.int_or("missile", 0, 127, 0) as u8,
            strategy: Strategy {
                alignment,
                aggressive: o.bool("aggressive"),
                brave: o.bool("brave"),
            },
            magic_defense,
            mimic: o.bool("mimic"),
            wealth: o.int_or("wealth", 0, 3100, 0) as u16,
            weapon,
            armor,
            x: 0,
            y: 0,
            region: 0,
            room: 0,
            possessions,
            unknown_0b: 0,
            last_turn_round: 0,
        }
    }

    /// `{ thing = KEY, count = N }`: a carriable thing 1..80.
    fn possession(&self, c: &Obj) -> Option<Possession> {
        let k = c.need_str("thing")?;
        let (n, ty) = self.thing_ref(c, "thing", k)?;
        if !ty.carriable() {
            c.err(format!("thing: {k} can't be carried"));
            return None;
        }
        let max = if n <= STACKABLE_MAX { 3 } else { 1 };
        let count = c.int_or("count", 1, 3, 1) as u8;
        if count > max {
            c.err(format!(
                "count: {k} is thing {n}; only things 1..32 come up to 3 (inventory 1)"
            ));
        }
        Some(Possession {
            thing: n,
            count: count.min(max),
        })
    }

    // -------------------------------------------------------------------
    // regions/*.toml, first pass: every room's place

    fn room_places(&mut self, files: &[(&str, &Table)]) {
        for (i, (f, t)) in files.iter().enumerate() {
            let region = i as u8 + 1;
            let rkey = t.get("key").and_then(Value::as_str).unwrap_or("");
            if !is_key(rkey) {
                self.p.push(f, "", "key: missing or not a key");
            }
            let Some(Value::Array(rooms)) = t.get("room") else {
                self.p.push(f, "", "no [[room]]");
                continue;
            };
            for (n, r) in rooms.iter().enumerate() {
                let Some(r) = r.as_table() else { continue };
                let key = r.get("key").and_then(Value::as_str).unwrap_or("");
                let full = format!("{rkey}.{key}");
                if !is_key(key) {
                    self.p
                        .push(f, &format!("room {full}"), "key: missing or not a key");
                    continue;
                }
                let rows = r.get("map").and_then(Value::as_str).map(grid).unwrap_or_default();
                let (width, height) = (rows.first().map_or(0, Vec::len), rows.len());
                let (x, y) = r.get("at").and_then(|v| point(v, 255)).unwrap_or((0, 0));
                let info = RoomInfo {
                    region,
                    room: n as u8,
                    width: width.min(255) as u8,
                    height: height.min(255) as u8,
                    x,
                    y,
                };
                if self.rooms.insert(full.clone(), info).is_some() {
                    self.p.push(f, &format!("room {full}"), "room key used twice");
                }
            }
        }
    }

    // -------------------------------------------------------------------
    // world.toml

    fn world(&mut self, t: &Table) -> Option<WorldMap> {
        let o = Obj::new(t, WORLD, "", self.p);
        let name = o.need_str("name").unwrap_or_default().to_string();
        let wraps = o.bool("wraps");
        let mut terrain: Vec<Option<Terrain>> = Vec::new();
        let mut keys: HashMap<String, (u8, bool)> = HashMap::new();
        let (mut ordinary, mut doors) = (0u8, 12u8);
        for v in o.array("terrain") {
            let Some(tt) = v.as_table() else { continue };
            let key = tt.get("key").and_then(Value::as_str).unwrap_or("");
            let to = o.child(tt, format!("terrain {key}"));
            to.raw("key");
            if !is_key(key) {
                to.err("key: missing or not a key");
            }
            let door = to.bool("door");
            let number = if door {
                let n = doors;
                doors += 1;
                if n > 15 {
                    to.err("more than four door terrains (12..15, movement 4.4)");
                }
                n
            } else {
                let n = ordinary;
                ordinary += 1;
                if ordinary == 12 {
                    ordinary = 16;
                }
                n
            };
            let (passability, parameter) = match to.raw("pass") {
                Some(Value::String(s)) if s == "open" => (Passability::Open, 0),
                Some(Value::String(s)) if s == "impassable" => (Passability::Impassable, 0),
                Some(Value::Table(pt)) => {
                    let po = to.child(pt, format!("terrain {key} pass"));
                    let r = if let Some(k) = po.str("owners_of") {
                        (
                            Passability::OwnersOf,
                            self.thing_ref(&po, "owners_of", k).map_or(0, |x| x.0),
                        )
                    } else if let Some(k) = po.str("spell") {
                        (
                            Passability::TriggersSpell,
                            self.thing_ref(&po, "spell", k).map_or(0, |x| x.0),
                        )
                    } else {
                        po.err("should be { owners_of = KEY } or { spell = KEY }");
                        (Passability::Impassable, 0)
                    };
                    po.done();
                    r
                }
                _ => {
                    to.err("pass: should be impassable, open, { owners_of = KEY } or { spell = KEY }");
                    (Passability::Impassable, 0)
                }
            };
            let tr = Terrain {
                number,
                name: to.need_str("name").unwrap_or_default().to_string(),
                picture: self.picture(&to, "picture"),
                passability,
                parameter,
                unknown_bit5: false,
            };
            to.done();
            if keys.insert(key.into(), (number, door)).is_some() {
                to.err("key used twice");
            }
            let i = number as usize;
            if terrain.len() <= i {
                terrain.resize(i + 1, None);
            }
            terrain[i] = Some(tr);
        }
        let terrain: Vec<Terrain> = terrain
            .into_iter()
            .enumerate()
            .map(|(i, t)| {
                t.unwrap_or(Terrain {
                    number: i as u8,
                    name: String::new(),
                    picture: 0,
                    passability: Passability::Impassable,
                    parameter: 0,
                    unknown_bit5: false,
                })
            })
            .collect();

        let rows = o.need_str("map").map(grid).unwrap_or_default();
        let legend = o.table("legend");
        let mut tiles: Vec<Vec<u8>> = Vec::new();
        let width = rows.first().map_or(0, Vec::len);
        if rows.is_empty() || width == 0 {
            o.err("map: missing or empty");
        }
        let mut is_door = vec![vec![false; width]; rows.len()];
        for (y, row) in rows.iter().enumerate() {
            if row.len() != width {
                o.err(format!("map: row {} is {} long, not {width}", y + 1, row.len()));
            }
            let mut out = Vec::new();
            for (x, ch) in row.iter().enumerate() {
                let key = legend
                    .and_then(|l| l.get(&ch.to_string()))
                    .and_then(Value::as_str);
                match key.and_then(|k| keys.get(k).map(|v| (k, *v))) {
                    Some((_, (n, door))) => {
                        out.push(n);
                        if x < width {
                            is_door[y][x] = door;
                        }
                    }
                    None => {
                        o.err(format!(
                            "map: {ch:?} at [{x}, {y}] {}",
                            match key {
                                None => "isn't in the legend".to_string(),
                                Some(k) => format!("names no terrain ({k:?})"),
                            }
                        ));
                        out.push(0);
                    }
                }
            }
            tiles.push(out);
        }
        self.world_size = (width.min(255) as u8, rows.len().min(255) as u8);
        if let Some(l) = legend {
            let lo = o.child(l, "legend");
            for k in l.keys() {
                lo.raw(k);
            }
            lo.done();
        }
        let (mw, mh) = (width.max(1) as i64 - 1, rows.len().max(1) as i64 - 1);
        let start = o.point("start", mw.max(mh)).unwrap_or_else(|| {
            if !o.has("start") {
                o.err("start: missing");
            }
            (0, 0)
        });
        if start.0 as usize >= width || start.1 as usize >= rows.len() {
            o.err("start: off the map");
        }

        let mut doors_out = Vec::new();
        for (i, v) in o.array("door").iter().enumerate() {
            let Some(dt) = v.as_table() else { continue };
            let d = o.child(dt, format!("door {}", i + 1));
            let at = d.point("at", 255);
            let dest = self.destination(&d, None);
            d.done();
            let (Some((x, y)), Some(destination)) = (at, dest) else {
                if at.is_none() && !d.has("at") {
                    d.err("at: missing");
                }
                continue;
            };
            if (x as usize) >= width || (y as usize) >= rows.len() {
                d.err("at: off the map");
                continue;
            }
            if !is_door[y as usize][x as usize] {
                d.err("at: not on door terrain (movement 4.4)");
            }
            if doors_out.iter().any(|m: &MapDoor| m.at == MapPoint { x, y }) {
                d.err("at: two doors on one cell");
            }
            doors_out.push(MapDoor {
                index: i as u16,
                at: MapPoint { x, y },
                destination,
                unknown_y_bit7: false,
            });
        }
        for (y, row) in is_door.iter().enumerate() {
            for (x, &door) in row.iter().enumerate() {
                if door
                    && !doors_out.iter().any(|m| {
                        m.at == MapPoint {
                            x: x as u8,
                            y: y as u8,
                        }
                    })
                {
                    o.err(format!(
                        "map: door terrain at [{x}, {y}] has no [[door]] (movement 4.4)"
                    ));
                }
            }
        }

        let mut encounters = Vec::new();
        for (i, v) in o.array("encounter").iter().enumerate() {
            let Some(et) = v.as_table() else { continue };
            let e = o.child(et, format!("encounter {}", i + 1));
            let creature = e.need_str("creature").and_then(|k| self.master(&e, k));
            let terrain_n = e.need_str("terrain").and_then(|k| {
                let r = keys.get(k).map(|x| x.0);
                if r.is_none() {
                    e.err(format!("terrain: no terrain {k:?}"));
                }
                r
            });
            let chance = e.need_int("chance", 0, 60).unwrap_or(0) as u8;
            e.done();
            if let (Some(mut c), Some(terrain)) = (creature, terrain_n) {
                c.number = encounters.len() as u16 + 1;
                c.class = None;
                encounters.push(WorldCreature {
                    creature: c,
                    terrain,
                    chance,
                });
            }
        }
        o.done();
        Some(WorldMap {
            name,
            wraps,
            start: MapPoint {
                x: start.0,
                y: start.1,
            },
            terrain,
            tiles,
            doors: doors_out,
            exit_points: Vec::new(),
            random_creatures: encounters,
            area_items: stock_area_items(),
            open_map_items: Vec::new(),
            unknown_806: wm::STOCK_AREA_DOORS.to_vec(),
            unknown_80e: wm::STOCK_80E,
        })
    }

    /// A master creature's record by key.
    fn master(&self, o: &Obj, k: &str) -> Option<Creature> {
        let n = self.creatures.get(k).copied();
        if n.is_none() {
            o.err(format!("creature: no creature {k:?}"));
        }
        self.masters.get(n? as usize - 1).cloned()
    }

    /// `to` and `square`: a room square or a map cell. `from` is the
    /// door's room and square, for an arrival through a shared wall.
    fn destination(&self, o: &Obj, from: Option<(&RoomInfo, u8, u8)>) -> Option<Destination> {
        let to = o.need_str("to")?;
        let square = o.point("square", 255);
        if to == "map" {
            let Some((x, y)) = square else {
                o.err("square: missing (the map cell to arrive on)");
                return None;
            };
            if x >= self.world_size.0 || y >= self.world_size.1 {
                o.err("square: off the map");
            }
            return Some(Destination::WorldMap { x, y });
        }
        let Some(r) = self.rooms.get(to) else {
            o.err(format!("to: no room {to:?} (region.room, or \"map\")"));
            return None;
        };
        let (x, y) = match square {
            Some(s) => s,
            None => match from.and_then(|(a, dx, dy)| through_wall(a, dx, dy, r)) {
                Some(s) => s,
                None => {
                    o.err(format!(
                        "square: missing, and the door isn't in a wall shared with {to}"
                    ));
                    return None;
                }
            },
        };
        if x == 0 || y == 0 || x + 1 >= r.width || y + 1 >= r.height {
            o.err(format!("square: [{x}, {y}] is not inside {to}"));
        }
        Some(Destination::Room {
            region: r.region,
            room: r.room,
            x,
            y,
        })
    }

    // -------------------------------------------------------------------
    // regions/*.toml, second pass

    fn region(&mut self, f: &str, t: &Table, number: u8) -> Region {
        let o = Obj::new(t, f, "", self.p);
        let rkey = o.str("key").unwrap_or_default().to_string();
        let name = o.need_str("name").unwrap_or_default().to_string();
        let mut store = Vec::new();
        for (i, v) in o.array("store").iter().enumerate() {
            let Some(st) = v.as_table() else { continue };
            let c = o.child(st, format!("store {}", i + 1));
            if let Some(p) = self.possession(&c) {
                store.push(p);
            }
            c.done();
        }
        store.sort_by_key(|p| p.thing);
        let random_keys: Vec<String> = o
            .array("random")
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
        let random_creatures: Vec<Creature> = random_keys
            .iter()
            .enumerate()
            .filter_map(|(i, k)| {
                let mut c = self.master(&o, k)?;
                c.number = i as u16 + 1;
                c.class = None;
                Some(c)
            })
            .collect();
        let mut portal_table: Vec<[u8; 2]> = vec![[0, 0]];
        let mut residents: Vec<Creature> = Vec::new();
        let mut rooms = Vec::new();
        for (n, v) in o.array("room").iter().enumerate() {
            let Some(rt) = v.as_table() else { continue };
            let key = rt.get("key").and_then(Value::as_str).unwrap_or("");
            let full = format!("{rkey}.{key}");
            let r = o.child(rt, format!("room {full}"));
            r.raw("key");
            r.raw("at");
            let room = self.room(
                &r,
                &full,
                number,
                n as u8,
                &random_keys,
                &mut portal_table,
                &mut residents,
            );
            r.done();
            rooms.push(room);
        }
        o.done();
        let slots = self.limits.residents_per_region as usize;
        if residents.len() > slots {
            o.err(format!(
                "{} residents; limits.residents_per_region is {slots}",
                residents.len()
            ));
        }
        while residents.len() < slots {
            residents.push(blank_creature(residents.len() as u16 + 1));
        }
        Region {
            number,
            name,
            rooms,
            random_creatures,
            residents,
            store,
            portal_table,
            unknown_header_03: 0x00F0,
            unknown_59: vec![0; 20],
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn room(
        &mut self,
        r: &Obj,
        full: &str,
        region: u8,
        number: u8,
        random_keys: &[String],
        portal_table: &mut Vec<[u8; 2]>,
        residents: &mut Vec<Creature>,
    ) -> Room {
        let name = r.need_str("name").unwrap_or_default().to_string();
        let wall_picture = if r.has("wall") {
            self.picture(r, "wall")
        } else {
            self.default_wall
        };
        let (random_chance, random_creature) = match r.table("random") {
            None => (0, 0),
            Some(t) => {
                let ro = r.child(t, format!("room {full} random"));
                let chance = ro.need_int("chance", 0, 75).unwrap_or(0) as u8;
                let idx = ro.need_str("creature").and_then(|k| {
                    let i = random_keys.iter().position(|x| x == k);
                    if i.is_none() {
                        ro.err(format!("creature: {k:?} is not in the region's random list"));
                    }
                    i
                });
                ro.done();
                (chance, idx.unwrap_or(0) as u8)
            }
        };
        let rows = r.need_str("map").map(grid).unwrap_or_default();
        let legend = r.table("legend");
        let (w, h) = (rows.first().map_or(0, Vec::len), rows.len());
        if w < 3 || h < 3 {
            r.err("map: a room is at least 3 x 3, walls included");
        }
        let info = RoomInfo {
            region,
            room: number,
            width: w as u8,
            height: h as u8,
            x: self.rooms.get(full).map_or(0, |i| i.x),
            y: self.rooms.get(full).map_or(0, |i| i.y),
        };
        let mut items = Vec::new();
        let mut used_chars = Vec::new();
        for (y, row) in rows.iter().enumerate() {
            if row.len() != w {
                r.err(format!("map: row {} is {} long, not {w}", y + 1, row.len()));
            }
            for (x, ch) in row.iter().enumerate() {
                let edge = x == 0 || y == 0 || x + 1 == w || y + 1 == h;
                let c = ch.to_string();
                let Some(v) = legend.and_then(|l| l.get(&c)) else {
                    r.err(format!("map: {ch:?} at [{x}, {y}] isn't in the legend"));
                    continue;
                };
                if !used_chars.contains(ch) {
                    used_chars.push(*ch);
                }
                let at = format!("room {full} [{x}, {y}]");
                let lo = r.child(legend.unwrap(), at.clone());
                let (x8, y8) = (x as u8, y as u8);
                self.cell(
                    &lo,
                    v,
                    edge,
                    (x8, y8),
                    &info,
                    portal_table,
                    residents,
                    &mut items,
                    true,
                );
            }
        }
        if let Some(l) = legend {
            for k in l.keys() {
                let ch = k.chars().next().unwrap_or(' ');
                if k.chars().count() != 1 {
                    r.err(format!("legend: {k:?} should be one character"));
                } else if !used_chars.contains(&ch) {
                    // An unused legend entry is harmless; nothing to report.
                }
            }
        }
        Room {
            number,
            name,
            deleted: false,
            wall_picture,
            width: w as u8,
            height: h as u8,
            x: info.x,
            y: info.y,
            random_chance,
            random_creature,
            items,
        }
    }

    /// One legend value on one square: walls, floor, things, gold, residents.
    #[allow(clippy::too_many_arguments)]
    fn cell(
        &mut self,
        o: &Obj,
        v: &Value,
        edge: bool,
        (x, y): (u8, u8),
        room: &RoomInfo,
        portal_table: &mut Vec<[u8; 2]>,
        residents: &mut Vec<Creature>,
        items: &mut Vec<RoomItem>,
        top: bool,
    ) {
        match v {
            Value::String(s) if s == "wall" => {
                if !edge {
                    o.err("a wall inside the room; use an obstacle");
                }
            }
            Value::String(s) if s == "floor" => {
                if edge {
                    o.err("floor on the edge: the edge is the room's wall");
                }
            }
            Value::String(k) => {
                let mut t = Table::new();
                t.insert("thing".into(), Value::String(k.clone()));
                self.placement(o, &t, edge, (x, y), room, portal_table, items);
            }
            Value::Array(list) if top => {
                for v in list {
                    self.cell(o, v, edge, (x, y), room, portal_table, residents, items, false);
                }
            }
            Value::Table(t) if t.contains_key("creature") => {
                let c = o.child(t, o.at.clone());
                let k = c.need_str("creature").unwrap_or("");
                if edge {
                    c.err("a resident must stand inside the room");
                }
                if let Some(mut m) = self.master(&c, k) {
                    m.number = residents.len() as u16 + 1;
                    m.class = None;
                    m.x = x;
                    m.y = y;
                    m.region = room.region;
                    m.room = room.room;
                    residents.push(m);
                }
                for v in c.array("things") {
                    self.cell(&c, v, edge, (x, y), room, portal_table, residents, items, false);
                }
                c.done();
            }
            Value::Table(t) if t.contains_key("gold") => {
                let c = o.child(t, o.at.clone());
                if edge {
                    c.err("gold on the wall");
                }
                let n = c.need_int("gold", 1, 25_000).unwrap_or(1) as u16;
                c.done();
                items.push(RoomItem {
                    x,
                    y,
                    thing: GOLD,
                    parameter: 0,
                    flag: false,
                    destination: None,
                    gold: Some(n),
                });
            }
            Value::Table(t) => self.placement(o, t, edge, (x, y), room, portal_table, items),
            _ => o.err("should be \"wall\", \"floor\", a thing key, a table or a list"),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn placement(
        &mut self,
        o: &Obj,
        t: &Table,
        edge: bool,
        (x, y): (u8, u8),
        room: &RoomInfo,
        portal_table: &mut Vec<[u8; 2]>,
        items: &mut Vec<RoomItem>,
    ) {
        let c = o.child(t, o.at.clone());
        let Some(k) = c.need_str("thing") else {
            c.done();
            return;
        };
        let Some((n, ty)) = self.thing_ref(&c, "thing", k) else {
            c.done();
            return;
        };
        if edge && !ty.in_walls() {
            c.err(format!(
                "{k} can't stand in a wall; only portals, obstacles and custom obstacles (movement 5.2)"
            ));
        }
        let mut parameter = 0u8;
        let mut destination = None;
        match ty {
            Type::Portal => {
                destination = self.destination(&c, Some((room, x, y)));
                if let Some(d) = destination {
                    let index = portal_table.len();
                    if index > u8::MAX as usize {
                        c.err("more than 255 doors in one region");
                    }
                    parameter = index as u8;
                    portal_table.push(self.table_entry(d));
                }
            }
            Type::CustomSpace | Type::CustomObstacle => match c.raw("parameter") {
                None => {}
                Some(Value::Integer(p)) if (0..=255).contains(p) => parameter = *p as u8,
                Some(Value::Table(pt)) if ty == Type::CustomSpace => {
                    let po = c.child(pt, format!("{} parameter", c.at));
                    if let Some((pn, _)) = po
                        .need_str("thing")
                        .and_then(|pk| self.thing_ref(&po, "thing", pk))
                    {
                        parameter = u8::try_from(pn).unwrap_or_else(|_| {
                            po.err("thing number above 255");
                            0
                        });
                    }
                    po.done();
                }
                Some(_) => c.err("parameter: should be 0..255 (or { thing = KEY } for a custom space)"),
            },
            _ => {}
        }
        c.done();
        items.push(RoomItem {
            x,
            y,
            thing: n,
            parameter,
            flag: false,
            destination,
            gold: None,
        });
    }

    /// A door table entry (format-notes, Doors), where it fits two bytes.
    fn table_entry(&mut self, d: Destination) -> [u8; 2] {
        match d {
            Destination::Room { region, room, x, y } if region < 16 && room < 16 && x < 16 && y < 16 => {
                [region << 4 | room, x << 4 | y]
            }
            Destination::WorldMap { x, y } => {
                let p = Some(MapPoint { x, y });
                let k = match self.exit_points.iter().position(|e| *e == p) {
                    Some(k) => Some(k),
                    None => self.exit_points.iter().position(Option::is_none).inspect(|&k| {
                        self.exit_points[k] = p;
                    }),
                };
                k.map_or([0xFF, 0xFF], |k| [0, (wm::DOOR_COUNT + k) as u8])
            }
            _ => [0xFF, 0xFF],
        }
    }
}

struct Header {
    name: String,
    byline: String,
    license: String,
    intro: String,
    credits: Vec<String>,
    theme: Theme,
    classes: Vec<String>,
}

fn nothing() -> SpellAction {
    SpellAction {
        kind: SpellActionKind::Nothing,
        parameter: 0,
    }
}

fn magic_level(l: &str) -> Option<u8> {
    Some(match l {
        "none" => 0,
        "non_magic_half" => 1,
        "only_magic" => 2,
        "magic_half" => 3,
        _ => return None,
    })
}

/// A map's rows as characters, without the blank first and last lines a
/// TOML multi-line string leaves.
fn grid(s: &str) -> Vec<Vec<char>> {
    s.lines()
        .map(|l| l.trim_end_matches('\r'))
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.chars().collect())
        .collect()
}

/// The arrival square one step through the wall a door at (`dx`, `dy`) of
/// room `a` is in, in room `b`, if the two rooms share that wall on the
/// region grid (content 3.9).
fn through_wall(a: &RoomInfo, dx: u8, dy: u8, b: &RoomInfo) -> Option<(u8, u8)> {
    if a.region != b.region {
        return None;
    }
    let (w, h) = (a.width, a.height);
    let (sx, sy): (i32, i32) = match (dx, dy) {
        (0, y) if y > 0 && y + 1 < h => (-1, 0),
        (x, y) if x + 1 == w && y > 0 && y + 1 < h => (1, 0),
        (x, 0) if x > 0 && x + 1 < w => (0, -1),
        (x, y) if y + 1 == h && x > 0 && x + 1 < w => (0, 1),
        _ => return None,
    };
    let (gx, gy) = (a.x as i32 + dx as i32, a.y as i32 + dy as i32);
    // The door's square must be on b's edge too: a shared wall.
    let (bx, by) = (gx - b.x as i32, gy - b.y as i32);
    let on_b_edge = (0..b.width as i32).contains(&bx)
        && (0..b.height as i32).contains(&by)
        && (bx == 0 || by == 0 || bx + 1 == b.width as i32 || by + 1 == b.height as i32);
    if !on_b_edge {
        return None;
    }
    let (tx, ty) = (bx + sx, by + sy);
    let inside = tx > 0 && ty > 0 && tx + 1 < b.width as i32 && ty + 1 < b.height as i32;
    inside.then_some((tx as u8, ty as u8))
}

fn blank_creature(number: u16) -> Creature {
    Creature {
        number,
        in_use: false,
        name: String::new(),
        class: None,
        picture: None,
        constitution: 0,
        life_force: 0,
        strength: 0,
        dexterity: 0,
        speed: 0,
        wisdom: 0,
        power: 0,
        size: 0,
        dodge: 0,
        parry: 0,
        armor_skill: 0,
        melee: 0,
        missile: 0,
        strategy: Strategy {
            alignment: Alignment::Neutral,
            aggressive: false,
            brave: false,
        },
        magic_defense: MagicDefense::None,
        mimic: false,
        wealth: 0,
        weapon: 0,
        armor: 0,
        x: 0,
        y: 0,
        region: 0,
        room: 0,
        possessions: Vec::new(),
        unknown_0b: 0,
        last_turn_round: 0,
    }
}

/// An RGBA PNG's size and pixels.
fn decode_png(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8() | png::Transformations::ALPHA);
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    let px = &buf[..frame.buffer_size()];
    let rgba = match frame.color_type {
        png::ColorType::Rgba => px.to_vec(),
        png::ColorType::GrayscaleAlpha => px
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|c| [c[0], c[0], c[0], c[1]])
            .collect(),
        other => return Err(format!("colour type {other:?}")),
    };
    Ok((frame.width, frame.height, rgba))
}

/// Classic pixels from an HD tile (content 3.3): box-averaged to 16 x 16,
/// transparent pixels as colour 0, each pixel the nearest palette colour.
pub fn classic_pixels(rgba: &[u8], tile: usize, palette: &Palette) -> Picture {
    let cols: Vec<[i32; 3]> = palette
        .colors
        .iter()
        .map(|c| [c.rgb[0] as i32, c.rgb[1] as i32, c.rgb[2] as i32])
        .collect();
    let bg = cols.first().copied().unwrap_or([0; 3]);
    let mut pixels = Vec::with_capacity(256);
    for oy in 0..16 {
        for ox in 0..16 {
            let (x0, x1) = (ox * tile / 16, ((ox + 1) * tile / 16).max(ox * tile / 16 + 1));
            let (y0, y1) = (oy * tile / 16, ((oy + 1) * tile / 16).max(oy * tile / 16 + 1));
            let mut sum = [0i64; 3];
            let mut n = 0i64;
            for y in y0..y1 {
                for x in x0..x1 {
                    let i = (y * tile + x) * 4;
                    let c = if rgba[i + 3] < 128 {
                        bg
                    } else {
                        [rgba[i] as i32, rgba[i + 1] as i32, rgba[i + 2] as i32]
                    };
                    for k in 0..3 {
                        sum[k] += c[k] as i64;
                    }
                    n += 1;
                }
            }
            let avg = sum.map(|s| (s / n) as i32);
            let best = cols
                .iter()
                .enumerate()
                .min_by_key(|(_, c)| (0..3).map(|k| (c[k] - avg[k]).pow(2)).sum::<i32>())
                .map_or(0, |(i, _)| i);
            pixels.push(best as u8);
        }
    }
    Picture {
        width: 16,
        height: 16,
        pixel_aspect: 1,
        pixels,
    }
}

/// Every string value in `v`, at any depth (content 4.2's "taken").
fn strings(v: &Value, out: &mut HashSet<String>) {
    match v {
        Value::String(s) => {
            out.insert(s.clone());
        }
        Value::Array(a) => a.iter().for_each(|v| strings(v, out)),
        Value::Table(t) => t.values().for_each(|v| strings(v, out)),
        _ => {}
    }
}

fn entry_key(t: &Table) -> String {
    t.get("key").and_then(Value::as_str).unwrap_or("").to_string()
}

/// The kit's taken entries in kit order (an own entry of the same key in
/// its place), then the rest of the adventure's own (content 4.2).
fn merge(kit: Vec<Entry>, own: Vec<Entry>, words: &HashSet<String>) -> Vec<Entry> {
    let mut own: Vec<Option<Entry>> = own.into_iter().map(Some).collect();
    let mut out = Vec::new();
    for e in kit {
        let k = entry_key(&e.1);
        if let Some(slot) = own
            .iter_mut()
            .find(|o| o.as_ref().is_some_and(|o| entry_key(&o.1) == k))
        {
            out.push(slot.take().expect("found"));
        } else if words.contains(&k) {
            out.push(e);
        }
    }
    out.extend(own.into_iter().flatten());
    out
}
