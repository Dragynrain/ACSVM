# Project instructions

Open-source engine for Adventure Construction Set adventures (working name in
`Cargo.toml` `description` and `README.md` only; never hard-code it elsewhere).
Plan: `docs/design.md`. Current phase: 3, the behavior capture harness
(`docs/capture.md`; first capture done), before any combat formulas. Phase 2
(rules engine from the manual: movement, inventory, creatures outside combat
and magic) is done.
Rules specs: `docs/rules/` (CC BY 4.0), each rule tagged [manual], [data],
[capture], [research] or [unknown] with the engine's choice; write the spec
before the code. Rules capture can't settle (random formulas, edge cases,
hidden state) go through clean-room research: `docs/research/README.md`.

## Hard rules

- **No EA material in the repo, ever**: no disk images, adventures, bytes
  copied from them, manual text, builder-set art, exported pictures or audio.
  The pre-commit hook (`git config core.hooksPath .githooks`) and CI enforce
  the extensions; the rest is on you.
- Research material lives outside the repo in `D:\projects\ACSVM-research`
  (manual OCR: paraphrase only; Boris draft; C64 corpus). `advcnst*.d64` are
  EA's program disks (advcnst2 holds EA's *Rivers of Light*): never decode them
  for analysis. Only a researcher session (a separate one, started in the
  research folder by the maintainer) may; sessions here are implementers.
- **Clean room** (`docs/research/README.md`): never open, list or search
  `ACSVM-research/research/*/notes/`; read only a pass's `findings.md`,
  confirm it by capture, then spec it, then code it. Never implement from
  anything but the spec.
- `corpus/pc/ACS.FPY` is EA's *Rivers of Light* from the user's own MS-DOS
  copy: fine to decode locally, never commit anything derived from it.
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
cargo build -p acs-core -p acs-engine --target wasm32-unknown-unknown
cargo clippy -p acs-viewer --target wasm32-unknown-unknown -- -D warnings
cargo run -p acs-viewer --release -- FILE.D64 [--view room:1:0|walk] [--seed N --walk n,e,wait] [--screenshot out.png]
scripts/web.sh serve                                    # trunk dev server, :8080
scripts/web.sh build                                    # static site in target/web
cargo run -p acs-cli -- dump FILE.D64 -v -o out.json
cargo run -p acs-cli -- pictures FILE.D64 outdir --scale 4 --sheet
cargo run -p acs-capture --release -- run move-basic --snapshot RUN/start.vsf --disk RUN/disk.d64 --out RUN [--jobs 4]  # docs/capture.md 4
python scripts/sprite64.py OUTDIR IMG.png... --palette 32 --sheet s.png  # our generated art only
python scripts/terrain64.py OUTDIR TEX.png... [--kind place] [--candidates]   # docs/art/pirates-style.md 11
python scripts/quilt.py OUTDIR TEX.png...   # seamless texture; flatten.py: flat sprite backgrounds
python scripts/palsnap.py docs/art/pirates-palette.json OUTDIR IMG.png... --sheet s.png  # snap to kit ramps
python scripts/tidy.py docs/art/pirates-palette.json OUTDIR TILE.png... --sheet s.png  # specks, strays, outline
```

Write CLI output and viewer screenshots for real adventures outside the repo
(scratch folders): they are derived from EA or third-party material.

## Layout

- `crates/acs-core/src/layout.rs`: every offset (block bases per platform,
  offsets inside blocks shared).
- `decode.rs`: image to model; non-fatal oddities become `Diagnostic`s.
- `model.rs`: canonical serde model (the future editor format; bump
  `MODEL_VERSION` when meaning changes).
- `graphics.rs`, `text.rs`, `disk.rs`, `bits.rs`: codecs and helpers.
- `scene.rs`: what each room square shows (walls, floor = thing 127, item
  stacks top first, residents) and the region grid. Layout only, no drawing.
- `crates/acs-engine`: rules, no GUI (Rust 1.85). `Game::new(adv, seed, rules,
  player)`, `apply(Command) -> Vec<Event>`, `Game::replay`; one seeded `Rng`
  (SplitMix64) for every roll; `game.rs` holds the turn scheduler (region
  visits of up to 10 rounds, a turn list per round). `step.rs` is movement (and pick-ups),
  `inventory.rs` carry limits, weight, Ready and Drop, `creatures.rs` turn
  order, random creatures and personality, `ai.rs` one creature's turn (goals,
  paths, doors, pick-ups, drops, readying, casting), `magic.rs` the 15 spell
  actions, Cast/Use, Power drift, temporary Life Force, healing and death
  (`run_spell` is the one place a spell takes effect); `describe` turns
  events into log lines. Live creatures are the region's 16 resident slots, edited in place. Tests reuse acs-core's fixture builder via `#[path]`.
- `crates/acs-cli`: the `acs` binary (`dump`, `pictures`).
- `crates/acs-capture`: the capture harness (tool crate, not wasm): drives
  VICE 3.10 (`ACSVM_VICE`: its folder or `x64sc`) over its binary
  monitor; scenario disks are our blocks over the user's blank adventure
  disk (`ACSVM_CAPTURE_TEMPLATE`); scripts in `scripts/`. All its output goes
  to `ACSVM-research/capture/`. See `docs/capture.md`. `run --jobs N`
  (`jobs.rs`) splits runs over N VICE processes; every run writes
  `timing.jsonl` (`timing.rs`). Menu inputs wait for the screen to settle
  (`Runner::settle`), never a fixed count; check `timing.jsonl` before
  adding a long fixed wait. `snap.rs` saves a snapshot just before a roll
  (researcher inputs, state sweeps); `turns.rs` logs who moves when from
  the name strip (`--creature` puts residents of ours in the room).
- `crates/acs-viewer`: eframe/egui viewer, native and web (trunk;
  `index.html`, `Trunk.toml`). Needs Rust 1.95 (eframe 0.37); the core stays
  at 1.85. `tiles.rs` computes the integer tile scale from the window in
  physical pixels each frame; `views/` holds one file per view; `views/walk.rs` is walk mode
  (keys to engine commands, events to the log; no rules in the viewer).
- Repo: public at github.com/Dragynrain/ACSVM; Pages deploys `target/web` on
  every push to main. Commits use the GitHub noreply address.
