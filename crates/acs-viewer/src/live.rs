// SPDX-License-Identifier: GPL-3.0-or-later

//! The browser's side of accessibility (client 7): a hidden live region a
//! screen reader reads the log from, and the system's reduced-motion
//! setting. On the desktop both do nothing yet.

/// Lines kept in the live region.
#[cfg(target_arch = "wasm32")]
const KEEP: u32 = 50;

/// Adds `lines` to the page's live region, made on first use.
#[cfg(target_arch = "wasm32")]
pub fn announce(lines: &[String]) {
    if lines.is_empty() {
        return;
    }
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    let region = match doc.get_element_by_id("acsvm-live") {
        Some(r) => r,
        None => {
            let Ok(r) = doc.create_element("div") else { return };
            r.set_id("acsvm-live");
            let _ = r.set_attribute("role", "log");
            let _ = r.set_attribute("aria-live", "polite");
            // Visually hidden, still read.
            let _ = r.set_attribute(
                "style",
                "position:absolute;left:-10000px;width:1px;height:1px;overflow:hidden",
            );
            let Some(body) = doc.body() else { return };
            if body.append_child(&r).is_err() {
                return;
            }
            r
        }
    };
    for line in lines {
        if let Ok(p) = doc.create_element("p") {
            p.set_text_content(Some(line));
            let _ = region.append_child(&p);
        }
    }
    while region.child_element_count() > KEEP {
        match region.first_element_child() {
            Some(old) => old.remove(),
            None => break,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn announce(_lines: &[String]) {}

/// The system asks for reduced motion (`prefers-reduced-motion`).
#[cfg(target_arch = "wasm32")]
pub fn system_reduced_motion() -> bool {
    web_sys::window()
        .and_then(|w| w.match_media("(prefers-reduced-motion: reduce)").ok().flatten())
        .is_some_and(|m| m.matches())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn system_reduced_motion() -> bool {
    false
}
