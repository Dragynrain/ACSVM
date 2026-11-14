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
const NARROW: f32 = 900.0;

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
}

impl Viewer {
    pub fn new(cc: &eframe::CreationContext<'_>, start: Startup) -> Self {
        cc.egui_ctx.set_theme(egui::Theme::Dark);
        let mut v = Viewer {
            inbox: Arc::default(),
            doc: None,
            error: None,
            show_nav: None,
            show_info: None,
            capture: start.screenshot.map(|p| (p, 10)),
        };
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
            doc.walk = Some(walk);
            doc.view = View::Walk;
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

    fn open(&mut self, ctx: &egui::Context, file: Incoming) {
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
            Ok(decoded) => {
                for d in &decoded.diagnostics {
                    log::warn!("{}: {d}", file.name);
                }
                self.doc = Some(Doc::new(ctx, file.name, decoded));
                self.error = None;
            }
            Err(e) => self.error = Some(format!("{}: {e}", file.name)),
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
                    "Adventures (.d64, .fpy, .hrd)",
                    &["d64", "D64", "fpy", "FPY", "hrd", "HRD"],
                )
                .add_filter("Saves (.json)", &["json", "JSON"])
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
        let narrow = ctx.content_rect().width() < NARROW;
        egui::MenuBar::new().ui(ui, |ui| {
            if self.doc.is_some() {
                let nav = self.show_nav.unwrap_or(!narrow);
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
                self.pick_file(&ctx);
            }
            if let Some(doc) = &mut self.doc {
                if ui.selectable_label(doc.view == View::Walk, "Walk").clicked() {
                    doc.go(View::Walk);
                }
                ui.add_enabled_ui(!doc.history.is_empty(), |ui| {
                    if ui.button("Back").clicked() {
                        doc.back();
                    }
                });
                ui.separator();
                let a = &doc.decoded.adventure;
                ui.label(RichText::new(&a.name).strong());
                ui.label(format!("{} · {}", a.platform, doc.file_name));
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
            }
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
            });
        });
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

    fn empty_state(&self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.3);
            ui.heading(format!("{} viewer", project_name()));
            ui.add_space(8.0);
            ui.label("Drag a C64 .d64 disk image or a PC ADVEN.FPY / ADVEN.HRD here, or use Open….");
            ui.label(RichText::new("The file is read on this machine and never uploaded.").weak());
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
            if self.show_nav.unwrap_or(!narrow) {
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
