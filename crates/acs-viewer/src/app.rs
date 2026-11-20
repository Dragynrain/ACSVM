// SPDX-License-Identifier: GPL-3.0-or-later

//! The viewer application: loading files, navigation and the panel layout.

use crate::load::{self, Incoming};
use crate::tiles::Tiles;
use crate::views;
use acs_core::model::Destination;
use acs_core::{Decoded, Severity};
use egui::{Color32, RichText};
use std::sync::{Arc, Mutex};

/// The project's working name, read from the workspace description
/// ("NAME: ...") so that it lives only in Cargo.toml and README.md.
pub fn project_name() -> &'static str {
    env!("CARGO_PKG_DESCRIPTION")
        .split(':')
        .next()
        .unwrap_or("viewer")
}

/// The build: version and commit (content 8), as `0.1.0 (abc1234)`.
pub fn build_stamp() -> String {
    format!("{} ({})", env!("CARGO_PKG_VERSION"), env!("ACSVM_COMMIT"))
}

/// The playtest's issue forms and tester guide (content 8).
const REPORT_URL: &str = "https://github.com/Dragynrain/ACSVM/issues/new/choose";
const GUIDE_URL: &str = "https://github.com/Dragynrain/ACSVM/blob/main/docs/playtest.md";

/// Below this window width (in points) the side panels start hidden.
pub(crate) const NARROW: f32 = 900.0;
/// In walk mode the menu column shows by default only from this width,
/// where the board still gets room beside it and the info column (client 2).
const WALK_NAV: f32 = 1500.0;

/// Whether the menu column shows when no button has fixed it.
fn nav_default(doc: Option<&Doc>, width: f32) -> bool {
    match doc {
        Some(d) if d.view == View::Walk => width >= WALK_NAV,
        _ => width >= NARROW,
    }
}

/// The top bar's divider: one row high, where `ui.separator()` in a
/// wrapping row would take all the height left.
fn bar_separator(ui: &mut egui::Ui) {
    let h = ui.spacing().interact_size.y;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(6.0, h), egui::Sense::hover());
    let stroke = ui.visuals().widgets.noninteractive.bg_stroke;
    ui.painter().vline(rect.center().x, rect.y_range(), stroke);
}

/// What the central area shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    /// Walk mode: play movement through the rules engine.
    Walk,
    World,
    /// 0-based region index.
    Region(usize),
    /// 0-based region index and room slot.
    Room(usize, usize),
    Things,
    Creatures,
    Messages,
    Diagnostics,
    /// The loaded HD pack and its coverage (client 5).
    Pack,
}

/// A loaded adventure and everything the views remember about it.
pub struct Doc {
    pub file_name: String,
    pub decoded: Decoded,
    pub tiles: Tiles,
    pub view: View,
    history: Vec<View>,
    /// A square to point out after following a door: (view, x, y).
    pub highlight: Option<(View, u8, u8)>,
    /// Square selected by a click in the current map or room view.
    pub selected: Option<(u8, u8)>,
    /// Square under the pointer in the last frame.
    pub hover: Option<(u8, u8)>,
    pub thing: usize,
    pub creature: usize,
    pub message_tab: views::messages::Tab,
    /// A note for the status line (e.g. a door into a deleted room).
    pub note: Option<String>,
    /// Whether the info panel of map and room views is shown.
    pub show_info: bool,
    /// The walk-mode game, begun from the New character screen.
    pub walk: Option<views::walk::Walk>,
    /// That screen while a new game's character is made (client 4.2).
    pub creating: Option<views::walk::NewCharacter>,
    /// A view asked for the file picker (Load from file…).
    pub pick: bool,
    /// Walk mode's bar never drains (design.md, Play options).
    pub turn_based: bool,
    /// Walk mode's on-screen pad fixed on or off; `None` follows the last
    /// input (client 4).
    pub pad: Option<bool>,
    /// Play Music's output and controls (client 6).
    pub audio: crate::audio::Audio,
    /// The session's HD pack (client 5).
    pub pack: Option<std::rc::Rc<crate::hdpack::Pack>>,
    /// A bundle's credits, shown on the end screen (winning 4.6).
    pub credits: Vec<String>,
    /// The title screen or intro page still to show before play (client
    /// 3.3): set when the adventure opens and on a new game.
    pub opening: Option<views::opening::Opening>,
}

impl Doc {
    fn new(ctx: &egui::Context, file_name: String, decoded: Decoded) -> Self {
        let tiles = Tiles::new(ctx, &decoded.adventure);
        let view = if decoded.diagnostics.iter().any(|d| d.severity == Severity::Error) {
            View::Diagnostics
        } else {
            View::World
        };
        Doc {
            file_name,
            decoded,
            tiles,
            view,
            history: Vec::new(),
            highlight: None,
            selected: None,
            hover: None,
            thing: 0,
            creature: 0,
            message_tab: views::messages::Tab::Long,
            note: None,
            show_info: true,
            walk: None,
            creating: None,
            pick: false,
            turn_based: false,
            pad: None,
            audio: Default::default(),
            pack: None,
            credits: Vec::new(),
            opening: Some(views::opening::Opening::Title),
        }
    }

    /// Switches view, remembering the old one for Back.
    pub fn go(&mut self, view: View) {
        if view != self.view {
            self.history.push(self.view);
            if self.history.len() > 200 {
                self.history.remove(0);
            }
            self.view = view;
            self.selected = None;
            self.hover = None;
            self.note = None;
        }
    }

    pub fn back(&mut self) {
        if let Some(v) = self.history.pop() {
            self.view = v;
            self.selected = None;
            self.hover = None;
            self.highlight = None;
            self.note = None;
        }
    }

    /// Follows a door: opens the destination and points at its square.
    pub fn follow(&mut self, dest: Destination) {
        match dest {
            Destination::WorldMap { x, y } => {
                self.go(View::World);
                self.highlight = Some((View::World, x, y));
            }
            Destination::Room { region, room, x, y } => {
                let r = (region as usize).wrapping_sub(1);
                let ok = self
                    .decoded
                    .adventure
                    .regions
                    .get(r)
                    .and_then(|reg| reg.rooms.get(room as usize))
                    .map(|rm| !rm.deleted);
                match ok {
                    Some(true) => {
                        let v = View::Room(r, room as usize);
                        self.go(v);
                        self.highlight = Some((v, x, y));
                    }
                    Some(false) => {
                        self.note = Some(format!(
                            "That door leads to deleted room {room} of region {region}; \
                             the original sends players back to the start."
                        ))
                    }
                    None => {
                        self.note = Some(format!(
                            "That door leads to a missing region or room ({region}, {room})."
                        ))
                    }
                }
            }
        }
    }
}

/// What to do at start-up (desktop command line; empty on the web).
#[derive(Default)]
pub struct Startup {
    pub path: Option<std::path::PathBuf>,
    pub view: Option<View>,
    /// Render at this many pixels per point, the window this many points.
    pub ppp: Option<(f32, [f32; 2])>,
    /// Walk mode with the on-screen pad on (`--pad`).
    pub pad: bool,
    /// An HD pack to load first (`--pack`).
    pub pack: Option<std::path::PathBuf>,
    /// Options to start with (`--text-size`, `--colour-blind`).
    pub text_size: Option<f32>,
    pub colour_blind: bool,
    /// The render mode to start in (`--render`).
    pub render: Option<crate::tiles::RenderMode>,
    /// Write a screenshot here after a few frames, then quit.
    pub screenshot: Option<std::path::PathBuf>,
    /// Walk mode: seed and commands to play before showing it.
    pub walk: Option<(u64, Vec<acs_engine::Command>)>,
    /// Walk mode: put the player here before the commands (a tool shortcut).
    pub place: Option<acs_engine::Location>,
    /// Walk mode from this save file (docs/saves.md), after the commands.
    pub load: Option<std::path::PathBuf>,
    /// Write the walk's save file here once started (desktop only).
    pub save: Option<std::path::PathBuf>,
}

/// What the viewer keeps between sessions (eframe's storage: a file on
/// the desktop, the browser's local storage on the web): the options, the
/// sound settings and the last game played (client 7.2).
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct Kept {
    access: Option<crate::access::Access>,
    /// Sound on, and the volume.
    sound: Option<(bool, f32)>,
    game: Option<KeptGame>,
    /// The welcome has been seen once.
    welcomed: bool,
    /// "Don't show this again" on How to play.
    no_play_help: bool,
    /// Characters retired with Save, from every adventure played: the
    /// original's other adventure disk (characters 3.2).
    characters: Vec<views::walk::SavedCharacter>,
}

/// The last game: its save file (docs/saves.md), resumed when the same
/// adventure (by fingerprint) is opened again.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct KeptGame {
    adventure: String,
    save: String,
}

/// The storage key.
const KEPT: &str = "kept";

pub struct Viewer {
    inbox: Arc<Mutex<Vec<Incoming>>>,
    doc: Option<Doc>,
    error: Option<String>,
    /// Navigation and info panels: `None` follows the window width, a
    /// button click fixes it.
    show_nav: Option<bool>,
    show_info: Option<bool>,
    /// Pending screenshot: path and frames left to render first.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    capture: Option<(std::path::PathBuf, u32)>,
    /// `--ppp`: the window size in points still to be reached. The first
    /// frames run at the screen's scale, so panels remember sizes from a
    /// different window; they are forgotten once this size is reached.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    resize_to: Option<egui::Vec2>,
    /// The HD pack loaded this session, for every adventure opened.
    pack: Option<std::rc::Rc<crate::hdpack::Pack>>,
    /// The Options menu (client 7).
    access: crate::access::Access,
    /// Gamepads (client 4).
    pads: crate::gamepad::Gamepads,
    /// The welcome (before an adventure) and How to play (after one
    /// opens) windows are up (client 7.3).
    welcome: bool,
    play_help: bool,
    /// Kept: the welcome seen, and How to play not wanted on opening.
    welcomed: bool,
    no_play_help: bool,
    /// The text size while its slider is held: applied on letting go, as
    /// a new size resizes the menu, slider and all, under the pointer.
    text_drag: Option<f32>,
    /// A sample chosen on the title screen: open it in walk mode
    /// (content 7).
    play_next: bool,
    /// The title art, made when the title screen first shows.
    title: Option<egui::TextureHandle>,
    /// Whether this session reads and writes the kept state: not for a
    /// screenshot or a scripted walk, which must come out the same.
    keep: bool,
    kept_sound: Option<(bool, f32)>,
    kept_game: Option<KeptGame>,
    /// The kept characters (`Kept::characters`), this session's included.
    characters: Vec<views::walk::SavedCharacter>,
    /// The open file and the game's seed and command count when last
    /// kept, so an unchanged game isn't written again.
    kept_mark: Option<(String, (u64, usize))>,
}

impl Viewer {
    pub fn new(cc: &eframe::CreationContext<'_>, start: Startup) -> Self {
        cc.egui_ctx.set_theme(egui::Theme::Dark);
        crate::fonts::install(&cc.egui_ctx);
        if let Some((ppp, size)) = start.ppp {
            let native = cc.egui_ctx.native_pixels_per_point().unwrap_or(1.0);
            cc.egui_ctx.set_zoom_factor(ppp / native);
            cc.egui_ctx
                .send_viewport_cmd(egui::ViewportCommand::InnerSize(size.into()));
        }
        let keep = start.screenshot.is_none() && start.walk.is_none() && start.load.is_none();
        let mut v = Viewer {
            inbox: Arc::default(),
            pads: crate::gamepad::Gamepads::default(),
            doc: None,
            error: None,
            show_nav: None,
            show_info: None,
            capture: start.screenshot.map(|p| (p, 10)),
            resize_to: start.ppp.map(|(_, size)| size.into()),
            pack: None,
            access: crate::access::Access {
                text: start.text_size.unwrap_or(1.0),
                colour_blind: start.colour_blind,
                reduce_motion: crate::live::system_reduced_motion(),
                simple_shops: false,
            },
            text_drag: None,
            welcome: false,
            play_help: false,
            welcomed: false,
            no_play_help: false,
            play_next: false,
            title: None,
            keep,
            kept_sound: None,
            kept_game: None,
            characters: Vec::new(),
            kept_mark: None,
        };
        if v.keep {
            let kept: Kept = cc
                .storage
                .and_then(|s| eframe::get_value(s, KEPT))
                .unwrap_or_default();
            // The command line's options win over the kept ones.
            if let (Some(a), None, false) = (kept.access, start.text_size, start.colour_blind) {
                v.access = a;
            }
            v.kept_sound = kept.sound;
            v.kept_game = kept.game;
            v.characters = kept.characters;
            v.welcomed = kept.welcomed;
            v.no_play_help = kept.no_play_help;
            // The first visit opens on the welcome (client 7.3).
            v.welcome = !kept.welcomed && start.path.is_none();
        }
        v.access.set(&cc.egui_ctx);
        // The pack first, so the adventure opens with it.
        if let Some(p) = start.pack {
            let bytes = std::fs::read(&p).map_err(|e| format!("cannot read {}: {e}", p.display()));
            v.open(
                &cc.egui_ctx,
                Incoming {
                    name: load::display_name(&p),
                    bytes,
                },
            );
        }
        if let Some(p) = start.path {
            let bytes = std::fs::read(&p).map_err(|e| format!("cannot read {}: {e}", p.display()));
            v.open(
                &cc.egui_ctx,
                Incoming {
                    name: load::display_name(&p),
                    bytes,
                },
            );
        }
        if let (Some(doc), Some(view)) = (&mut v.doc, start.view) {
            doc.view = view;
        }
        if let (Some(doc), Some((seed, commands))) = (&mut v.doc, start.walk) {
            let mut walk = views::walk::Walk::new(&doc.decoded.adventure, seed, Default::default());
            if let Some(at) = start.place {
                walk.place(at);
            }
            for c in commands {
                walk.apply(c);
            }
            // The start-up commands' cues are not played (client 6), nor
            // their lines read (client 7).
            walk.take_cues();
            walk.take_unannounced();
            doc.walk = Some(walk);
            doc.view = View::Walk;
            doc.opening = None;
        }
        if let (true, Some(doc)) = (start.pad, &mut v.doc) {
            doc.pad = Some(true);
        }
        if let (Some(m), Some(doc)) = (start.render, &mut v.doc) {
            doc.tiles.set_mode(&cc.egui_ctx, m);
        }
        if let Some(p) = start.load {
            let bytes = std::fs::read(&p).map_err(|e| format!("cannot read {}: {e}", p.display()));
            v.open(
                &cc.egui_ctx,
                Incoming {
                    name: load::display_name(&p),
                    bytes,
                },
            );
        }
        if let (Some(p), Some(doc)) = (start.save, &v.doc) {
            if let Some(walk) = &doc.walk {
                let (_, text) = walk.save_file(&doc.decoded.adventure);
                match std::fs::write(&p, text) {
                    Ok(()) => eprintln!("wrote {}", p.display()),
                    Err(e) => eprintln!("cannot write {}: {e}", p.display()),
                }
            }
        }
        if let Some(e) = &v.error {
            log::error!("{e}");
        }
        v
    }

    /// Counts down the frames before a requested screenshot, then takes it
    /// and closes the window once it is written.
    #[cfg(not(target_arch = "wasm32"))]
    fn capture_tick(&mut self, ctx: &egui::Context) {
        if let Some(size) = self.resize_to {
            ctx.request_repaint();
            if (ctx.content_rect().size() - size).length() > 1.0 {
                return;
            }
            self.resize_to = None;
            ctx.memory_mut(|m| m.data.clear());
            if let Some((_, frames)) = &mut self.capture {
                *frames = 10;
            }
        }
        let Some((path, frames)) = &mut self.capture else {
            return;
        };
        ctx.request_repaint();
        if *frames > 0 {
            *frames -= 1;
            return;
        }
        let path = path.clone();
        self.capture = None;
        let ctx2 = ctx.clone();
        ctx.request_screenshot(move |img| {
            let rgba: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
            match crate::cli::write_png(&path, img.size, &rgba) {
                Ok(()) => eprintln!("wrote {} ({}x{})", path.display(), img.size[0], img.size[1]),
                Err(e) => eprintln!("screenshot failed: {e}"),
            }
            ctx2.send_viewport_cmd(egui::ViewportCommand::Close);
        });
    }

    /// Puts the session's pack on `doc` and switches it to HD pack mode.
    fn apply_pack(ctx: &egui::Context, pack: &std::rc::Rc<crate::hdpack::Pack>, doc: &mut Doc) {
        doc.tiles.set_pack(ctx, pack);
        doc.tiles.set_mode(ctx, crate::tiles::RenderMode::HdPack);
        let c = crate::hdpack::coverage(pack, &doc.decoded.adventure);
        doc.note = Some(format!(
            "HD pack {}: {} of {} pictures ({:.0}%); Lists > HD pack shows what it misses.",
            pack.name,
            c.matched,
            c.counted,
            c.percent()
        ));
        doc.pack = Some(pack.clone());
    }

    /// The kept sound settings, and the kept game if it was played on this
    /// very adventure (client 7.2).
    fn resume_kept(&mut self, doc: &mut Doc) {
        if !self.keep {
            return;
        }
        if let Some((on, volume)) = self.kept_sound {
            doc.audio.on = on;
            doc.audio.volume = volume.clamp(0.0, 1.0);
            doc.audio.apply_controls();
        }
        let print = acs_engine::save::fingerprint(&doc.decoded.adventure);
        let Some(kept) = self.kept_game.as_ref().filter(|k| k.adventure == print) else {
            return;
        };
        match views::walk::Walk::from_save(&doc.decoded.adventure, &kept.save) {
            Ok(mut walk) => {
                walk.note_kept();
                self.kept_mark = Some((doc.file_name.clone(), walk.mark()));
                doc.walk = Some(walk);
            }
            Err(e) => log::warn!("the kept game didn't load: {e}"),
        }
    }

    /// Characters 3.2: the open game's characters retired with Save join
    /// the kept list (one entry per adventure and name, the latest), a
    /// kept character that joined leaves it (3.4), and the walk is offered
    /// the rest but this game's own.
    fn sync_characters(&mut self) {
        let Some(walk) = self.doc.as_mut().and_then(|d| d.walk.as_mut()) else {
            return;
        };
        if let Some(c) = walk.take_joined_other() {
            self.characters.retain(|k| k != &c);
        }
        let print = walk.fingerprint().to_string();
        for name in walk.take_rejoined() {
            self.characters
                .retain(|k| k.adventure != print || k.player.name != name);
        }
        let adv = walk.game.adventure();
        for p in walk.saved_characters() {
            let c = views::walk::SavedCharacter {
                adventure: print.clone(),
                adventure_name: adv.name.trim().to_string(),
                player: p.clone(),
            };
            match self
                .characters
                .iter_mut()
                .find(|k| k.adventure == c.adventure && k.player.name == c.player.name)
            {
                Some(k) => *k = c,
                None => self.characters.push(c),
            }
        }
        // This game's own list has its own; earlier games' of this
        // adventure come back by Rejoin, other adventures' by 3.3.
        let own: Vec<String> = walk.saved_characters().iter().map(|p| p.name.clone()).collect();
        walk.others = self
            .characters
            .iter()
            .filter(|k| k.adventure != print || !own.contains(&k.player.name))
            .cloned()
            .collect();
    }

    /// What to keep now: the options, the sound and the open game.
    fn kept(&mut self) -> Kept {
        if let Some(doc) = &self.doc {
            self.kept_sound = Some((doc.audio.on, doc.audio.volume));
            if let Some(walk) = &doc.walk {
                let mark = (doc.file_name.clone(), walk.mark());
                if self.kept_mark.as_ref() != Some(&mark) {
                    let adv = &doc.decoded.adventure;
                    self.kept_game = Some(KeptGame {
                        adventure: acs_engine::save::fingerprint(adv),
                        save: walk.save_file(adv).1,
                    });
                    self.kept_mark = Some(mark);
                }
            }
        }
        Kept {
            access: Some(self.access),
            sound: self.kept_sound,
            game: self.kept_game.clone(),
            welcomed: self.welcomed,
            no_play_help: self.no_play_help,
            characters: self.characters.clone(),
        }
    }

    fn open(&mut self, ctx: &egui::Context, file: Incoming) {
        // An HD pack stays for every adventure opened after it (client 5).
        if let Ok(bytes) = &file.bytes {
            match crate::hdpack::Pack::read(bytes) {
                Ok(pack) => {
                    let pack = std::rc::Rc::new(pack);
                    if let Some(doc) = &mut self.doc {
                        Self::apply_pack(ctx, &pack, doc);
                    }
                    log::info!("{}: HD pack {}", file.name, pack.name);
                    self.pack = Some(pack);
                    self.error = None;
                    return;
                }
                Err(crate::hdpack::PackError::Broken(why)) => {
                    self.error = Some(format!("{}: not a usable HD pack: {why}", file.name));
                    return;
                }
                Err(crate::hdpack::PackError::NotAPack) => {}
            }
        }
        // A save file goes to the open adventure's walk (docs/saves.md 4.1).
        if let Some(text) = file.bytes.as_deref().ok().and_then(load::save_text) {
            self.error = match &mut self.doc {
                None => Some(format!("{}: open the adventure first, then its save", file.name)),
                Some(doc) => match views::walk::Walk::from_save(&doc.decoded.adventure, text) {
                    Ok(walk) => {
                        doc.walk = Some(walk);
                        doc.go(View::Walk);
                        // A loaded save goes straight to the board (client 3.3).
                        doc.opening = None;
                        None
                    }
                    Err(e) => Some(format!("{}: {e}", file.name)),
                },
            };
            return;
        }
        match file.bytes.and_then(|b| load::load(&file.name, b)) {
            Ok(load::Loaded {
                decoded,
                art,
                credits,
            }) => {
                for d in &decoded.diagnostics {
                    log::warn!("{}: {d}", file.name);
                }
                let mut doc = Doc::new(ctx, file.name, decoded);
                doc.credits = credits;
                if let Some(pack) = &self.pack {
                    Self::apply_pack(ctx, pack, &mut doc);
                }
                if std::mem::take(&mut self.play_next) {
                    doc.go(View::Walk);
                }
                // A bundle's own art is its default look (content 2).
                if let Some(art) = &art {
                    doc.tiles.set_native(ctx, art);
                    doc.tiles.set_mode(ctx, crate::tiles::RenderMode::NativeHd);
                }
                self.resume_kept(&mut doc);
                self.doc = Some(doc);
                self.error = None;
                // What to do now that an adventure is open (client 7.3).
                self.welcome = false;
                self.play_help = self.keep && !self.no_play_help;
            }
            Err(e) => {
                self.play_next = false;
                self.error = Some(format!("{}: {e}", file.name));
            }
        }
    }

    /// Takes files from drops (native: read now; web: read asynchronously).
    fn take_dropped(&self, ctx: &egui::Context) {
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        // Only the first file: one adventure at a time.
        let Some(file) = dropped.into_iter().next() else {
            return;
        };
        let name = load::display_name(file.path());
        #[cfg(not(target_arch = "wasm32"))]
        {
            let bytes = file.bytes();
            self.inbox.lock().unwrap().push(Incoming { name, bytes });
        }
        #[cfg(target_arch = "wasm32")]
        {
            let inbox = self.inbox.clone();
            let ctx = ctx.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let bytes = file.bytes_async().await;
                inbox.lock().unwrap().push(Incoming { name, bytes });
                ctx.request_repaint();
            });
        }
    }

    fn pick_file(&self, ctx: &egui::Context) {
        let inbox = self.inbox.clone();
        let ctx = ctx.clone();
        let task = async move {
            let file = rfd::AsyncFileDialog::new()
                .set_title("Open an adventure or a save")
                .add_filter(
                    "Adventures (.d64, .fpy, .hrd, .adf, adventure, .advx)",
                    &[
                        "d64", "D64", "fpy", "FPY", "hrd", "HRD", "adf", "ADF", "advx", "ADVX",
                    ],
                )
                .add_filter("Saves (.json)", &["json", "JSON"])
                .add_filter("HD packs (.png)", &["png", "PNG"])
                .add_filter("All files", &["*"])
                .pick_file()
                .await;
            if let Some(f) = file {
                let name = f.file_name();
                let bytes = f.read().await;
                inbox.lock().unwrap().push(Incoming {
                    name,
                    bytes: Ok(bytes),
                });
                ctx.request_repaint();
            }
        };
        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn(move || pollster::block_on(task));
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(task);
    }

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        // Narrow (client 2): the bar wraps onto more rows rather than run
        // off the window, which would also widen every panel below it.
        let wrapping = egui::Layout::left_to_right(egui::Align::Center).with_main_wrap(true);
        egui::MenuBar::new().ui(ui, |ui| {
            // Fullscreen takes its place on the right first, so the rest
            // fits in what is left and never runs under it (client 2).
            // Browsers have their own fullscreen key (F11).
            #[cfg(not(target_arch = "wasm32"))]
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let full = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
                if ui
                    .button(if full { "Exit fullscreen" } else { "Fullscreen" })
                    .clicked()
                {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!full));
                }
                ui.with_layout(wrapping, |ui| {
                    self.top_bar_left(ui, &ctx);
                });
            });
            #[cfg(target_arch = "wasm32")]
            ui.with_layout(wrapping, |ui| {
                self.top_bar_left(ui, &ctx);
            });
        });
    }

    /// The top bar's toggles, menus and the open adventure.
    fn top_bar_left(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        // A wrapping row would also wrap a button's own words.
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
        let narrow = ctx.content_rect().width() < NARROW;
        if self.doc.is_some() {
            let nav = self
                .show_nav
                .unwrap_or_else(|| nav_default(self.doc.as_ref(), ctx.content_rect().width()));
            let info = self.show_info.unwrap_or(!narrow);
            // On a narrow window the two columns don't fit beside each
            // other and the board: opening one closes the other (client 2).
            if ui.selectable_label(nav, "Menu").clicked() {
                self.show_nav = Some(!nav);
                if narrow && !nav {
                    self.show_info = Some(false);
                }
            }
            if ui.selectable_label(info, "Info").clicked() {
                self.show_info = Some(!info);
                if narrow && !info {
                    self.show_nav = Some(false);
                }
            }
            bar_separator(ui);
        }
        if ui.button("Open…").clicked() {
            self.pick_file(ctx);
        }
        // Accessibility options for every view (client 7).
        ui.menu_button("Options", |ui| {
            let a = &mut self.access;
            let mut pct = self.text_drag.unwrap_or((a.text * 100.0).round());
            let slider = ui.add(
                egui::Slider::new(&mut pct, 75.0..=200.0)
                    .step_by(5.0)
                    .suffix("%")
                    .text("Text size"),
            );
            if slider.dragged() {
                self.text_drag = Some(pct);
            } else {
                if slider.changed() || self.text_drag.is_some() {
                    a.text = pct / 100.0;
                }
                self.text_drag = None;
            }
            ui.checkbox(&mut a.colour_blind, "Colour-blind palette")
                .on_hover_text("Okabe-Ito colours for the log, the players, the creatures and the map.");
            ui.checkbox(&mut a.reduce_motion, "Reduce motion")
                .on_hover_text("No pulsing outlines.");
            ui.checkbox(&mut a.simple_shops, "Simplify shops").on_hover_text(
                "A store as a plain list of what it sells and buys, instead of the original's store screen.",
            );
        });
        // The playtest's ways to report, and the build (content 8).
        ui.menu_button("Help", |ui| {
            if self.doc.is_some() && ui.button("How to play").clicked() {
                self.play_help = true;
                ui.close();
            }
            if ui.button("Welcome").clicked() {
                self.welcome = true;
                ui.close();
            }
            bar_separator(ui);
            if ui.button("Report a problem…").clicked() {
                ctx.open_url(egui::OpenUrl::new_tab(REPORT_URL));
                ui.close();
            }
            if ui.button("Playtest guide").clicked() {
                ctx.open_url(egui::OpenUrl::new_tab(GUIDE_URL));
                ui.close();
            }
            bar_separator(ui);
            ui.label(RichText::new(format!("Build {}", build_stamp())).weak());
        });
        if let Some(doc) = &mut self.doc {
            if ui.selectable_label(doc.view == View::Walk, "Walk").clicked() {
                doc.go(View::Walk);
            }
            ui.add_enabled_ui(!doc.history.is_empty(), |ui| {
                if ui.button("Back").clicked() {
                    doc.back();
                }
            });
            // Render mode for every view (client 5).
            let mode = doc.tiles.mode();
            ui.menu_button(format!("Look: {}", mode.label()), |ui| {
                for m in crate::tiles::RenderMode::ALL {
                    if !doc.tiles.offers(m) {
                        continue;
                    }
                    if ui.selectable_label(mode == m, m.label()).clicked() {
                        doc.tiles.set_mode(ctx, m);
                        ui.close();
                    }
                }
            });
            bar_separator(ui);
            // Narrow, the bar keeps to what matters: a phone at large
            // text would otherwise give it a quarter of the screen (client 2).
            let name = doc.decoded.adventure.name.clone();
            let place = format!("{} · {}", doc.decoded.adventure.platform, doc.file_name);
            let errors = doc.decoded.errors().count();
            let warnings = doc.decoded.diagnostics.len() - errors;
            if errors + warnings > 0 {
                let text = match (errors, warnings) {
                    (0, w) => RichText::new(format!("{w} warning(s)")).color(Color32::YELLOW),
                    (e, w) => {
                        RichText::new(format!("{e} error(s), {w} warning(s)")).color(Color32::LIGHT_RED)
                    }
                };
                if ui.button(text).clicked() {
                    doc.go(View::Diagnostics);
                }
            } else if !narrow {
                ui.label(RichText::new("no diagnostics").weak());
            }
            // Last, so on a narrow bar they give way, elided (client 2).
            ui.add(egui::Label::new(RichText::new(name).strong()).truncate());
            if !narrow {
                ui.add(egui::Label::new(place).truncate());
            }
        }
    }

    fn nav(doc: &mut Doc, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            let mut go = None;
            let a = &doc.decoded.adventure;
            if ui
                .selectable_label(doc.view == View::Walk, RichText::new("Walk (play)").strong())
                .on_hover_text("Walk a player from the start with the keyboard")
                .clicked()
            {
                go = Some(View::Walk);
            }
            ui.add_space(4.0);
            if ui
                .selectable_label(
                    doc.view == View::World,
                    format!("World map: {}", a.world.name.trim()),
                )
                .clicked()
            {
                go = Some(View::World);
            }
            ui.add_space(4.0);
            ui.label(RichText::new("Regions").weak());
            for (r, region) in a.regions.iter().enumerate() {
                let in_region = matches!(doc.view, View::Region(x) | View::Room(x, _) if x == r);
                let id = ui.make_persistent_id(("nav-region", r));
                egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, in_region)
                    .show_header(ui, |ui| {
                        if ui
                            .selectable_label(
                                doc.view == View::Region(r),
                                format!("{} {}", r + 1, region.name.trim()),
                            )
                            .clicked()
                        {
                            go = Some(View::Region(r));
                        }
                    })
                    .body(|ui| {
                        for (i, room) in region.rooms.iter().enumerate().filter(|(_, rm)| !rm.deleted) {
                            if ui
                                .selectable_label(
                                    doc.view == View::Room(r, i),
                                    format!("{i:>2} {}", room.name.trim()),
                                )
                                .clicked()
                            {
                                go = Some(View::Room(r, i));
                            }
                        }
                    });
            }
            ui.add_space(4.0);
            ui.label(RichText::new("Lists").weak());
            for (v, name) in [
                (View::Things, "Things"),
                (View::Creatures, "Creatures"),
                (View::Messages, "Messages"),
                (View::Diagnostics, "Diagnostics"),
                (View::Pack, "HD pack"),
            ] {
                if ui.selectable_label(doc.view == v, name).clicked() {
                    go = Some(v);
                }
            }
            if let Some(v) = go {
                doc.go(v);
                doc.highlight = None;
            }
        });
    }

    /// The title screen (content 7): the kit's title art lettered with
    /// the project's name, the samples to play, then the player's own
    /// files. Text in the kit font (client 7.1).
    fn empty_state(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let tex = self.title.get_or_insert_with(|| title_texture(&ctx)).clone();
        let kit = |pt: f32| crate::fonts::kit(&ctx, pt, false);
        let ink = Color32::from_rgb(251, 241, 207);
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(16.0);
                // The art at the largest whole multiple of its pixels that
                // fits (whole physical pixels, like the tiles).
                let ppp = ctx.pixels_per_point();
                let [w, h] = tex.size().map(|v| v as f32);
                let room = (ui.available_width() * 0.9).min(ui.available_height() * 0.55 * w / h);
                let scale = ((room * ppp / w).floor().max(1.0)) / ppp;
                let size = egui::vec2(w * scale, h * scale);
                let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                ui.painter().image(
                    tex.id(),
                    rect,
                    egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
                // The name in the open sky: bold, gold, outlined (pirates-style 12).
                let font = crate::fonts::kit(&ctx, size.y * 0.14, true);
                let at = egui::pos2(rect.center().x, rect.top() + size.y * 0.2);
                let px = font.size / 11.0;
                for (dx, dy) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
                    ui.painter().text(
                        at + egui::vec2(dx * px, dy * px),
                        egui::Align2::CENTER_CENTER,
                        project_name(),
                        font.clone(),
                        Color32::from_rgb(28, 20, 25),
                    );
                }
                ui.painter().text(
                    at,
                    egui::Align2::CENTER_CENTER,
                    project_name(),
                    font,
                    Color32::from_rgb(246, 222, 74),
                );

                ui.add_space(12.0);
                if ui
                    .button(RichText::new("New here? What this is and how it works").font(kit(22.0)))
                    .clicked()
                {
                    self.welcome = true;
                }
                ui.add_space(12.0);
                ui.label(RichText::new("Play a sample adventure").font(kit(22.0)).color(ink));
                ui.add_space(6.0);
                for s in crate::samples::SAMPLES {
                    let button = egui::Button::new(RichText::new(format!("Play {}", s.name)).font(kit(22.0)));
                    if ui.add(button).clicked() {
                        self.play_next = true;
                        self.inbox.lock().unwrap().push(Incoming {
                            name: s.file.to_string(),
                            bytes: Ok(s.bytes.to_vec()),
                        });
                        ctx.request_repaint();
                    }
                    ui.label(RichText::new(s.about).font(kit(22.0)).weak());
                    ui.add_space(6.0);
                }
                ui.add_space(14.0);
                ui.label(RichText::new("Or open your own").font(kit(22.0)).color(ink));
                ui.label(
                    RichText::new("Drag a C64 .d64, an Amiga .adf, a PC ADVEN.FPY or ADVEN.HRD, or an .advx bundle here, or use Open...")
                        .font(kit(22.0)),
                );
                ui.label(
                    RichText::new("The file is read on this machine and never uploaded.")
                        .font(kit(22.0))
                        .weak(),
                );
                if let Some(p) = &self.pack {
                    ui.label(format!("HD pack loaded: {}. Open an adventure to use it.", p.name));
                }
                if let Some(e) = &self.error {
                    ui.add_space(12.0);
                    ui.colored_label(Color32::LIGHT_RED, e);
                }
                // The build, and where to report (content 8).
                ui.add_space(18.0);
                ui.hyperlink_to("Report a problem", REPORT_URL);
                ui.hyperlink_to("Playtest guide", GUIDE_URL);
                ui.label(RichText::new(format!("Build {}", build_stamp())).weak());
                ui.add_space(12.0);
            });
        });
    }
}

/// The welcome's parts (client 7.3): a heading (none for the first) and
/// its paragraph; `{name}` is the project's name.
const WELCOME: &[(&str, &str)] = &[
    ("", "{name} plays adventures made with Adventure Construction Set, the 1984 game kit, on today's computers: the original's rules, rebuilt and checked against the real thing."),
    ("Start with First Voyage", "The short tutorial on this screen: about fifteen minutes, with notes that explain each step as you reach it. It opens ready to play."),
    ("Or bring your own", "A C64 disk image (.d64), an Amiga disk image (.adf) or the adventure file from one, a PC adventure (ADVEN.FPY or ADVEN.HRD) or an .advx bundle: drag it onto the window or use Open… at the top. Apple II disks don't open yet. The file stays on your machine."),
    ("Explore, then Walk", "An adventure you open shows first in Explore: the world map, its regions and rooms, and lists of its things, creatures and messages, the way its designer built it. Walk, in the top bar, is where you play."),
    ("Help is in the Help menu", "How to play (keys, actions, stores and saving), this page, and the way to report a problem."),
];

/// How to play (client 7.3): first, for an adventure still in Explore.
const EXPLORE: (&str, &str) = ("You are in Explore", "Look over the adventure as its designer built it: the Menu on the left lists the world map, regions, rooms, things and creatures. To play, press Start playing below, or Walk in the top bar.");
const PLAY: &[(&str, &str)] = &[
    ("Moving", "Arrow keys, WASD or the number pad move you one square, and so does a click on a square beside you. Walk into a door to go through it, into a creature to fight it, onto a thing to pick it up. Space or Enter rests: the rest of your turn passes at once, each move left a chance to heal (the menu's REST, also a button). On a touch screen a pad appears."),
    ("Everything else: the Info column", "The column on the right (Info in the top bar shows or hides it) holds your actions: Fire for a bow or thrown weapon, Save Game and Retire at the top; then your stats; then what you carry, each with Ready, Drop, Cast or Use; then the creatures near you."),
    ("Time runs as in the original", "Your move bar drains while you do nothing, about a step every seven seconds. Tick Turn-based in the Info column to make it wait for you."),
    ("The log", "The strip at the bottom tells what happens: fights, finds, doors, and the adventure's own messages."),
    ("Stores", "Walk into a store to go in. Walk along the aisles and into a shelf to see what is on it and its price; Space buys. From the first prompt, Space (or Sell your goods) offers what you carry, one thing at a time. Walk into an exit to leave. Options > Simplify shops shows a plain list instead."),
    ("Friends and saving", "Add Someone brings in up to four characters, who take turns. Your game is kept in this browser or on this computer; Save to file… writes a save you can load later, or attach to a problem report."),
];

/// A help window's parts: each heading and its paragraph.
fn help_parts(ui: &mut egui::Ui, parts: &[(&str, &str)]) {
    for (title, body) in parts {
        ui.add_space(6.0);
        if !title.is_empty() {
            ui.label(RichText::new(*title).strong());
        }
        ui.label(body.replace("{name}", project_name()));
    }
}

/// A help window: centred, at most 560 points wide, scrolling when the
/// window is short.
fn help_window<'a>(title: String, open: &'a mut bool, ctx: &egui::Context) -> egui::Window<'a> {
    let width = (ctx.content_rect().width() - 32.0).clamp(240.0, 560.0);
    egui::Window::new(title)
        .open(open)
        .collapsible(false)
        .resizable(false)
        .default_width(width)
        .max_width(width)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
}

impl Viewer {
    /// The welcome, before an adventure is chosen, and How to play, after
    /// (client 7.3).
    fn help_windows(&mut self, ctx: &egui::Context) {
        let height = (ctx.content_rect().height() - 140.0).max(120.0);
        if self.welcome {
            let mut open = true;
            let mut close = false;
            help_window(format!("Welcome to {}", project_name()), &mut open, ctx).show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .min_scrolled_height(height.min(520.0))
                    .max_height(height)
                    .show(ui, |ui| {
                        help_parts(ui, WELCOME);
                        ui.add_space(10.0);
                        close = ui.button("Got it").clicked();
                    });
            });
            if !open || close {
                self.welcome = false;
                self.welcomed = true;
            }
        }
        if self.play_help {
            let Some(doc) = &mut self.doc else {
                self.play_help = false;
                return;
            };
            let name = doc.decoded.adventure.name.trim().to_string();
            let exploring = doc.view != View::Walk;
            let mut open = true;
            let mut close = false;
            let mut play = false;
            let mut hide = self.no_play_help;
            help_window(format!("How to play {name}"), &mut open, ctx).show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .min_scrolled_height(height.min(520.0))
                    .max_height(height)
                    .show(ui, |ui| {
                        if exploring {
                            help_parts(ui, &[EXPLORE]);
                        }
                        help_parts(ui, PLAY);
                        ui.add_space(10.0);
                        ui.checkbox(
                            &mut hide,
                            "Don't show this when an adventure opens (it stays in Help)",
                        );
                        ui.horizontal_wrapped(|ui| {
                            if exploring {
                                play = ui.button("Start playing").clicked();
                                close = ui.button("Look around first").clicked();
                            } else {
                                close = ui.button("Got it").clicked();
                            }
                        });
                    });
            });
            self.no_play_help = hide;
            if play {
                doc.go(View::Walk);
            }
            if !open || close || play {
                self.play_help = false;
            }
        }
    }
}

impl eframe::App for Viewer {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        if self.keep {
            let kept = self.kept();
            eframe::set_value(storage, KEPT, &kept);
        }
    }

    /// Often, so closing a browser tab loses little.
    fn auto_save_interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs(5)
    }

    /// Panel sizes and the like start fresh: the layout follows the window.
    fn persist_egui_memory(&self) -> bool {
        false
    }

    /// Gamepads become the keyboard's keys before the frame (client 4);
    /// on the board, whose turn it is decides which pad plays.
    fn raw_input_hook(&mut self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        let player = self
            .doc
            .as_ref()
            .filter(|d| d.opening.is_none() && d.creating.is_none())
            .and_then(|d| d.walk.as_ref())
            .map(|w| w.game.current());
        self.pads.feed(ctx, raw_input, player);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.take_dropped(&ctx);
        let incoming: Vec<Incoming> = std::mem::take(&mut *self.inbox.lock().unwrap());
        for f in incoming {
            self.open(&ctx, f);
        }
        // Not while typing: there Backspace deletes a letter (the focus is
        // last frame's, which is the one the key was typed into).
        let typing = ctx.memory(|m| m.focused().is_some());
        if ctx.input(|i| {
            (i.key_pressed(egui::Key::Backspace) && !typing)
                || i.pointer.button_pressed(egui::PointerButton::Extra1)
        }) {
            if let Some(d) = &mut self.doc {
                d.back();
            }
        }

        self.access.set(&ctx);
        self.sync_characters();
        egui::Panel::top("top").show(ui, |ui| self.top_bar(ui));

        if let Some(doc) = &mut self.doc {
            if let Some(e) = &self.error {
                egui::Panel::bottom("error").show(ui, |ui| {
                    ui.colored_label(Color32::LIGHT_RED, e);
                });
            }
            if let Some(n) = doc.note.clone() {
                egui::Panel::bottom("note").show(ui, |ui| {
                    ui.colored_label(Color32::YELLOW, n);
                });
            }
            let narrow = ctx.content_rect().width() < NARROW;
            doc.show_info = self.show_info.unwrap_or(!narrow);
            if self
                .show_nav
                .unwrap_or(nav_default(Some(doc), ctx.content_rect().width()))
            {
                egui::Panel::left("nav")
                    .resizable(true)
                    .default_size(220.0)
                    .show(ui, |ui| Self::nav(doc, ui));
            }
            views::show(doc, ui);
            if std::mem::take(&mut doc.pick) {
                self.pick_file(&ctx);
            }
        } else {
            egui::CentralPanel::default_margins().show(ui, |ui| self.empty_state(ui));
        }

        self.help_windows(&ctx);

        #[cfg(not(target_arch = "wasm32"))]
        self.capture_tick(&ctx);

        // A hint while a file hovers over the window.
        if ctx.input(|i| !i.raw.hovered_files.is_empty()) {
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::unique("drop"),
            ));
            let rect = ctx.content_rect();
            painter.rect_filled(rect, 0.0, Color32::from_black_alpha(180));
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Drop to open",
                egui::FontId::proportional(28.0),
                Color32::WHITE,
            );
        }
    }
}

/// The kit's title art (pirates-style 8: the *Magpie* off the island, no
/// lettering), nearest-filtered so its flat bands stay sharp.
fn title_texture(ctx: &egui::Context) -> egui::TextureHandle {
    let bytes: &[u8] = include_bytes!("../assets/title.png");
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8() | png::Transformations::ALPHA);
    let mut reader = decoder.read_info().expect("the title art is a PNG");
    let mut buf = vec![0; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut buf).expect("the title art decodes");
    let image = egui::ColorImage::from_rgba_unmultiplied(
        [frame.width as usize, frame.height as usize],
        &buf[..frame.buffer_size()],
    );
    ctx.load_texture("title", image, egui::TextureOptions::NEAREST)
}
