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
    /// The walk-mode game, started on first use.
    pub walk: Option<views::walk::Walk>,
    /// A view asked for the file picker (Load game).
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
            pick: false,
            turn_based: false,
            pad: None,
            audio: Default::default(),
            pack: None,
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
    /// A sample chosen on the title screen: open it in walk mode
    /// (content 7).
    play_next: bool,
}

impl Viewer {
    pub fn new(cc: &eframe::CreationContext<'_>, start: Startup) -> Self {
        cc.egui_ctx.set_theme(egui::Theme::Dark);
        if let Some((ppp, size)) = start.ppp {
            let native = cc.egui_ctx.native_pixels_per_point().unwrap_or(1.0);
            cc.egui_ctx.set_zoom_factor(ppp / native);
            cc.egui_ctx
                .send_viewport_cmd(egui::ViewportCommand::InnerSize(size.into()));
        }
        let mut v = Viewer {
            inbox: Arc::default(),
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
            },
            play_next: false,
        };
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
                        None
                    }
                    Err(e) => Some(format!("{}: {e}", file.name)),
                },
            };
            return;
        }
        match file.bytes.and_then(|b| load::load(&file.name, b)) {
            Ok(load::Loaded { decoded, art }) => {
                for d in &decoded.diagnostics {
                    log::warn!("{}: {d}", file.name);
                }
                let mut doc = Doc::new(ctx, file.name, decoded);
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
                self.doc = Some(doc);
                self.error = None;
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
                    "Adventures (.d64, .fpy, .hrd, .advx)",
                    &["d64", "D64", "fpy", "FPY", "hrd", "HRD", "advx", "ADVX"],
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
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    self.top_bar_left(ui, &ctx);
                });
            });
            #[cfg(target_arch = "wasm32")]
            self.top_bar_left(ui, &ctx);
        });
    }

    /// The top bar's toggles, menus and the open adventure.
    fn top_bar_left(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let narrow = ctx.content_rect().width() < NARROW;
        if self.doc.is_some() {
            let nav = self
                .show_nav
                .unwrap_or_else(|| nav_default(self.doc.as_ref(), ctx.content_rect().width()));
            if ui.selectable_label(nav, "Menu").clicked() {
                self.show_nav = Some(!nav);
            }
            let info = self.show_info.unwrap_or(!narrow);
            if ui.selectable_label(info, "Info").clicked() {
                self.show_info = Some(!info);
            }
            ui.separator();
        }
        if ui.button("Open…").clicked() {
            self.pick_file(ctx);
        }
        // Accessibility options for every view (client 7).
        ui.menu_button("Options", |ui| {
            let a = &mut self.access;
            let mut pct = (a.text * 100.0).round();
            ui.add(
                egui::Slider::new(&mut pct, 75.0..=200.0)
                    .step_by(5.0)
                    .suffix("%")
                    .text("Text size"),
            );
            a.text = pct / 100.0;
            ui.checkbox(&mut a.colour_blind, "Colour-blind palette")
                .on_hover_text("Okabe-Ito colours for the log, the players, the creatures and the map.");
            ui.checkbox(&mut a.reduce_motion, "Reduce motion")
                .on_hover_text("No pulsing outlines.");
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
            ui.separator();
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
            } else {
                ui.label(RichText::new("no diagnostics").weak());
            }
            // Last, so on a narrow bar they give way, elided (client 2).
            ui.add(egui::Label::new(RichText::new(name).strong()).truncate());
            ui.add(egui::Label::new(place).truncate());
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

    /// The title screen (content 7): the samples to play, then the
    /// player's own files.
    fn empty_state(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.2);
            ui.heading(project_name());
            ui.add_space(16.0);
            ui.label(RichText::new("Play a sample adventure").strong());
            ui.add_space(4.0);
            for s in crate::samples::SAMPLES {
                ui.horizontal(|ui| {
                    // Centre the row: the button and its line of text.
                    let w = (ui.available_width() - 520.0).max(0.0) / 2.0;
                    ui.add_space(w);
                    if ui.button(RichText::new(format!("Play {}", s.name)).strong()).clicked() {
                        self.play_next = true;
                        self.inbox.lock().unwrap().push(Incoming {
                            name: s.file.to_string(),
                            bytes: Ok(s.bytes.to_vec()),
                        });
                        ui.ctx().request_repaint();
                    }
                    ui.label(RichText::new(s.about).weak());
                });
            }
            ui.add_space(20.0);
            ui.label(RichText::new("Or open your own").strong());
            ui.label("Drag a C64 .d64 disk image, a PC ADVEN.FPY / ADVEN.HRD or an .advx bundle here, or use Open….");
            ui.label(RichText::new("The file is read on this machine and never uploaded.").weak());
            if let Some(p) = &self.pack {
                ui.label(format!(
                    "HD pack loaded: {}. Open an adventure to use it.",
                    p.name
                ));
            }
            if let Some(e) = &self.error {
                ui.add_space(12.0);
                ui.colored_label(Color32::LIGHT_RED, e);
            }
        });
    }
}

impl eframe::App for Viewer {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.take_dropped(&ctx);
        let incoming: Vec<Incoming> = std::mem::take(&mut *self.inbox.lock().unwrap());
        for f in incoming {
            self.open(&ctx, f);
        }
        if ctx.input(|i| {
            i.key_pressed(egui::Key::Backspace) || i.pointer.button_pressed(egui::PointerButton::Extra1)
        }) {
            if let Some(d) = &mut self.doc {
                d.back();
            }
        }

        self.access.set(&ctx);
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
