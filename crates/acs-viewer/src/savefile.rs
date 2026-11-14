// SPDX-License-Identifier: GPL-3.0-or-later

//! Handing a save file to the user (docs/saves.md 4.1): a save dialog on the
//! desktop, a download in the browser. Nothing leaves the machine.

/// Offers `text` as a file named `name`.
#[cfg(not(target_arch = "wasm32"))]
pub fn offer(name: String, text: String) {
    std::thread::spawn(move || {
        let file = pollster::block_on(
            rfd::AsyncFileDialog::new()
                .set_title("Save the game")
                .set_file_name(&name)
                .add_filter("Saves (.json)", &["json"])
                .save_file(),
        );
        if let Some(f) = file {
            if let Err(e) = std::fs::write(f.path(), text) {
                log::error!("cannot write {}: {e}", f.path().display());
            }
        }
    });
}

/// Offers `text` as a file named `name`.
#[cfg(target_arch = "wasm32")]
pub fn offer(name: String, text: String) {
    if let Err(e) = download(&name, &text) {
        log::error!("download of {name} failed: {e:?}");
    }
}

#[cfg(target_arch = "wasm32")]
fn download(name: &str, text: &str) -> Result<(), wasm_bindgen::JsValue> {
    use wasm_bindgen::JsCast;
    let parts = js_sys::Array::of1(&wasm_bindgen::JsValue::from_str(text));
    let opts = web_sys::BlobPropertyBag::new();
    opts.set_type("application/json");
    let blob = web_sys::Blob::new_with_str_sequence_and_options(&parts, &opts)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob)?;
    let doc = web_sys::window()
        .and_then(|w| w.document())
        .ok_or("no document")?;
    let a: web_sys::HtmlAnchorElement = doc.create_element("a")?.dyn_into()?;
    a.set_href(&url);
    a.set_download(name);
    a.click();
    web_sys::Url::revoke_object_url(&url)
}
