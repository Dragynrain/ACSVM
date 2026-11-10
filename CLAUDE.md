# Project instructions

Open-source engine for Adventure Construction Set adventures (working name in
`Cargo.toml` `description` and `README.md` only; never hard-code it elsewhere).
Plan: `docs/design.md`. Current phase: 0 (inspector).

## Hard rules

- **No EA material in the repo, ever**: no disk images, adventures, bytes
  copied from them, manual text, builder-set art, exported pictures or audio.
  The pre-commit hook (`git config core.hooksPath .githooks`) and CI enforce
  the extensions; the rest is on you.
- Research material lives outside the repo in `D:\projects\ACSVM-research`
  (manual OCR: paraphrase only; Boris draft; C64 corpus). `advcnst*.d64` are
  EA's program disks (advcnst2 holds EA's *Rivers of Light*): never decode them
  for analysis.
- ACS Viewer (`danlb2000/ACSViewer`, GPL-3.0) may be ported with credit
  headers. Its `RiversOfLight.hrd` test file is EA material: do not use it.
- Record every outside source in `docs/provenance.md`, format findings in
  `docs/format-notes.md`.
- Tests: synthetic fixtures only (`crates/acs-core/tests/common`); corpus
  tests read `ACSVM_CORPUS` and skip when unset.
- Ask before creating a GitHub remote or pushing.

## Commands

```bash
cargo test                                              # no corpus needed
ACSVM_CORPUS='D:\projects\ACSVM-research\corpus' cargo test --test corpus -- --nocapture
cargo clippy --all-targets -- -D warnings
cargo fmt --all
cargo build -p acs-core --target wasm32-unknown-unknown
cargo run -p acs-cli -- dump FILE.D64 -v -o out.json
cargo run -p acs-cli -- pictures FILE.D64 outdir --scale 4 --sheet
```

Write CLI output for real adventures outside the repo (scratch folders):
it is derived from EA or third-party material.

## Layout

- `crates/acs-core/src/layout.rs`: every offset (block bases per platform,
  offsets inside blocks shared).
- `decode.rs`: image to model; non-fatal oddities become `Diagnostic`s.
- `model.rs`: canonical serde model (the future editor format; bump
  `MODEL_VERSION` when meaning changes).
- `graphics.rs`, `text.rs`, `disk.rs`, `bits.rs`: codecs and helpers.
- `crates/acs-cli`: the `acs` binary (`dump`, `pictures`).
