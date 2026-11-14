// SPDX-License-Identifier: GPL-3.0-or-later

//! The adventure viewer: browse an adventure you own, natively or in a
//! browser. The file is read locally and never sent anywhere.
//!
//! Desktop: `acs-viewer [FILE]` (see `cli.rs` for options). Web: `trunk serve` in this crate (see
//! `scripts/web.sh`).

mod app;
#[cfg(not(target_arch = "wasm32"))]
mod cli;
mod load;
mod savefile;
mod tiles;
mod views;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    env_logger::init();
    let args = match cli::parse(std::env::args_os().skip(1)) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    let title = format!("{} viewer", app::project_name());
    let walking = args.seed.is_some()
        || args.walk.is_some()
        || args.place.is_some()
        || args.save.is_some()
        || args.view == Some(app::View::Walk);
    let start = app::Startup {
        path: args.file,
        view: args.view,
        screenshot: args.screenshot,
        place: args.place,
        load: args.load,
        save: args.save,
        walk: walking.then(|| {
            (
                args.seed.unwrap_or_else(views::walk::fresh_seed),
                args.walk.unwrap_or_default(),
            )
        }),
    };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(&title)
            .with_inner_size(args.size.unwrap_or([1280.0, 800.0]))
            .with_min_inner_size([480.0, 320.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        &title,
        options,
        Box::new(move |cc| Ok(Box::new(app::Viewer::new(cc, start)))),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use wasm_bindgen::JsCast as _;
    console_error_panic_hook::set_once();
    eframe::WebLogger::init(log::LevelFilter::Info).ok();
    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window().expect("window").document().expect("document");
        document.set_title(&format!("{} viewer", app::project_name()));
        let canvas = document
            .get_element_by_id("viewer")
            .expect("a canvas with id 'viewer'")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("#viewer is a canvas");
        let result = eframe::WebRunner::new()
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(|cc| Ok(Box::new(app::Viewer::new(cc, app::Startup::default())))),
            )
            .await;
        if let Some(loading) = document.get_element_by_id("loading") {
            match result {
                Ok(()) => loading.remove(),
                Err(e) => loading.set_inner_html(&format!("The viewer failed to start: {e:?}")),
            }
        }
    });
}
