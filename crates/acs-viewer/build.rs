// SPDX-License-Identifier: GPL-3.0-or-later

//! Builds the sample adventures (docs/content.md 7) from their source
//! folders and leaves the bundles in OUT_DIR, where `samples.rs` embeds
//! them. A sample that doesn't build fails this build, with its problems.
//! Also stamps the build with its commit (content.md 8): `ACSVM_COMMIT`.

use std::path::Path;

/// The samples' folders under `adventures/`, as `samples.rs` lists them.
const SAMPLES: &[&str] = &["first-voyage"];

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    stamp(&root);
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

/// The commit the viewer is built from, short: `GITHUB_SHA` in CI, else
/// `git rev-parse`, else "unknown" (a source tarball).
fn stamp(root: &Path) {
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");
    let git = root.join(".git");
    for f in ["HEAD", "refs/heads", "packed-refs"] {
        if git.join(f).exists() {
            println!("cargo:rerun-if-changed={}", git.join(f).display());
        }
    }
    let commit = std::env::var("GITHUB_SHA")
        .ok()
        .or_else(|| {
            let out = std::process::Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(root)
                .output()
                .ok()?;
            out.status
                .success()
                .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
        })
        .filter(|c| c.len() >= 7)
        .map_or_else(|| "unknown".to_string(), |c| c[..7].to_string());
    println!("cargo:rustc-env=ACSVM_COMMIT={commit}");
}
