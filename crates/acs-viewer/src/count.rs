// SPDX-License-Identifier: GPL-3.0-or-later

//! Anonymous counts for the web build (docs/client.md 11): a handful of
//! named events passed to the page's `acsCount`, which sends them to
//! GoatCounter. Only the event name goes: never a file name, an
//! adventure's text, a save or anything typed. The native build counts
//! nothing, and the page itself sends nothing on a local host, with Do
//! Not Track or Global Privacy Control set, or with no site code.

/// Counts `event`, a fixed name such as `open/c64` or `won`.
pub fn event(event: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast as _;
        let Some(window) = web_sys::window() else { return };
        let Ok(f) = js_sys::Reflect::get(&window, &"acsCount".into()) else {
            return;
        };
        if let Ok(f) = f.dyn_into::<js_sys::Function>() {
            let _ = f.call1(&wasm_bindgen::JsValue::NULL, &event.into());
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = event;
}

/// The event for an opened file: its format, from the name and size the
/// loader goes by, never the name itself.
pub fn open_event(name: &str, bytes: &[u8]) -> &'static str {
    if acs_bundle::is_bundle(name, bytes) {
        return "open/advx";
    }
    match crate::load::guess_platform(name, bytes.len()) {
        acs_core::Platform::C64 => "open/c64",
        acs_core::Platform::Amiga => "open/amiga",
        acs_core::Platform::Pc => "open/pc",
    }
}

/// After the console's panic hook, counts a panic by where it happened
/// (`panic/src/views/walk.rs:123`); the message is left out, since it can
/// hold a file's name or contents.
#[cfg(target_arch = "wasm32")]
pub fn panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        previous(info);
        let at = info
            .location()
            .map(|l| format!("panic/{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "panic/unknown".into());
        event(&at);
    }));
}
