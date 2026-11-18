// SPDX-License-Identifier: GPL-3.0-or-later

//! Builds the sample adventures (docs/content.md 7) from their source
//! folders and leaves the bundles in OUT_DIR, where `samples.rs` embeds
//! them. A sample that doesn't build fails this build, with its problems.

use std::path::Path;

/// The samples' folders under `adventures/`, as `samples.rs` lists them.
const SAMPLES: &[&str] = &["first-voyage"];

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    println!("cargo:rerun-if-changed={}", root.join("adventures").display());
    println!("cargo:rerun-if-changed={}", root.join("kits").display());
    let out = std::env::var("OUT_DIR").expect("OUT_DIR");
    for name in SAMPLES {
        let dir = root.join("adventures").join(name);
        let folder = acs_build::Folder::read(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
        let bundle = acs_build::build(&folder).unwrap_or_else(|p| {
            let list: Vec<String> = p.iter().map(ToString::to_string).collect();
            panic!("sample {name} doesn't build:\n{}", list.join("\n"))
        });
        let bytes = acs_bundle::write(&bundle).unwrap_or_else(|e| panic!("sample {name}: {e}"));
        std::fs::write(Path::new(&out).join(format!("{name}.advx")), bytes).expect("write the bundle");
    }
}
