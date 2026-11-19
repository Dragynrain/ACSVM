# Project instructions

Open-source engine for Adventure Construction Set adventures (working name in
`Cargo.toml` `description` and `README.md` only; never hard-code it elsewhere).
Plan: `docs/design.md`. Phases 2 (rules engine from the manual), 3 (the
capture harness, `docs/capture.md`) and 4 (formulas through research and
capture, hot-seat) are done: the Phase 4 gate is two players finishing
*Rivers of Light* by an automated replay (`acs play`, `docs/play.md`; the
corpus test `two_players_finish_rivers_of_light`), November 2026. Phase 5
(modern client and audio) is done too: its gate (no layout breaks at the
checked sizes, HD packs with a coverage report) was met November 2026;
the client's spec is `docs/client.md`. Phase 6 (original content and 1.0)
is under way; its plan and the bundle spec are `docs/content.md`. Rules [unknown]s left in `docs/rules/`
each carry the engine's choice; capture them as they matter.
Rules specs: `docs/rules/` (CC BY 4.0), each rule tagged [manual], [data],
[capture], [research] or [unknown] with the engine's choice; write the spec
before the code. Rules capture can't settle (random formulas, edge cases,
hidden state) go through clean-room research: `docs/research/README.md`.
Passes can run unattended (README 7): `/integrate` here (state in
`docs/research/ledger.md`), `/research-queue` in a fresh session in the
research folder. The clean-room hook (`scripts/hooks/cleanroom.py`, wired
in `.claude/settings.json`) blocks research notes, the program disks and
the Amiga program files and Kickstart ROM;
`python scripts/hooks/test_cleanroom.py` after changing it.

## Hard rules

- **No EA material in the repo, ever**: no disk images, adventures, bytes
  copied from them, manual text, builder-set art, exported pictures or audio.
  The pre-commit hook (`git config core.hooksPath .githooks`) and CI enforce
  the extensions; the rest is on you.
- Research material lives outside the repo in `D:\projects\ACSVM-research`
  (manual OCR: paraphrase only; Boris draft; C64 corpus). `advcnst*.d64` are
  EA's program disks (advcnst2 holds EA's *Rivers of Light*): never decode them
  for analysis. The same goes for EA's Amiga version in
  `ACSVM-research\corpus\amiga\`: the program files in
  `AdventureConstructionSet\data\` (`ACS.EXE`, `acs.1` to `acs.4`,
  `c\ACS.dat`, with `adventure`, `acs.master` and `acs.msgs` beside them)
  and the Kickstart ROM `kick13.rom` (copyrighted, never commit). Only a
  researcher session (a separate one, started in the research folder by
  the maintainer) may; sessions here are implementers.
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
ACSVM_PLAYTHROUGH='D:\projects\ACSVM-research\playthrough\runs\final.save.json' ACSVM_CORPUS=... cargo test --test corpus two_players   # the Phase 4 gate
cargo test -p acs-engine --test golden                  # golden tests: the engine vs capture.md's counts; ACSVM_BLESS=1 rewrites traces/
cargo clippy --all-targets -- -D warnings
cargo fmt --all
cargo build -p acs-core -p acs-engine -p acs-bundle --target wasm32-unknown-unknown
cargo clippy -p acs-viewer --target wasm32-unknown-unknown -- -D warnings
cargo run -p acs-viewer --release -- FILE.D64 [--view room:1:0|walk] [--seed N --walk n,e,wait] [--save s.json|--load s.json] [--size WxH --ppp 1] [--pad] [--render smoothed|hd|native] [--pack PACK.png] [--text-size PCT] [--colour-blind] [--screenshot out.png]   # docs/client.md 2
scripts/web.sh serve                                    # trunk dev server, :8080
scripts/web.sh build                                    # static site in target/web
cargo run -p acs-cli -- dump FILE.D64 -v -o out.json
cargo run -p acs-cli -- pictures FILE.D64 outdir --scale 4 --sheet
cargo run -p acs-cli -- build SRC_DIR -o OUT.advx          # an adventure source folder to a bundle (content.md 3)
cargo run -p acs-cli -- bundle MODEL.json -o OUT.advx [--art ATLAS.png --tile 64]   # Extended bundle from model JSON (content.md 2)
cargo run -p acs-cli --release -- play FILE --script S.txt --seed N [--players 2] --save s.json [--trace t.jsonl]   # scripted playthrough (docs/play.md)
cargo run -p acs-capture --release -- run move-basic --snapshot RUN/start.vsf --disk RUN/disk.d64 --out RUN [--jobs 4]  # docs/capture.md 4
python scripts/combatcheck.py RUNS.jsonl [--case NAME] [-v]   # combat rules vs a case's runs, roll by roll (capture.md 10.14.9-10, 10.16)
python scripts/storeread.py table|check ...           # store screens to text, sell offers vs stores 5.1 (capture.md 10.17)
python scripts/charcheck.py RUNS.jsonl [-v]          # new-character sweep vs characters 1-2, draw by draw (capture.md 10.18)
python scripts/encountercheck.py RUNS.jsonl --records 5:60 [--patch ..] [--empty 7]   # map-encounter sweep vs encounters 2-5, step by step (capture.md 10.20)
python scripts/hdpack.py build ATLAS.png MANIFEST.json OUT.png | check PACK.png | hashes DUMP.json   # HD packs (client.md 5)
python scripts/sprite64.py OUTDIR IMG.png... --palette 32 --sheet s.png  # our generated art only
python scripts/terrain64.py OUTDIR TEX.png... [--kind place] [--candidates]   # docs/art/pirates-style.md 11
python scripts/quilt.py OUTDIR TEX.png...   # seamless texture; flatten.py: flat sprite backgrounds
python scripts/palsnap.py docs/art/pirates-palette.json OUTDIR IMG.png... --sheet s.png  # snap to kit ramps
python scripts/tidy.py docs/art/pirates-palette.json OUTDIR TILE.png... --sheet s.png  # specks, strays, outline
python scripts/font.py docs/art/pirates-font.txt OUTDIR   # UI font atlas, BDF, preview (docs/art/pirates-style.md 12)
python scripts/font.py docs/art/pirates-font.txt crates/acs-viewer/assets --ttf-only   # the viewer's UI font TTFs (client.md 7.1)
python scripts/mood.py|emissive.py|normals.py|animate.py|variants.py ...   # moods, glow, normals, animation, recolours (pirates-style.md 13-17; --help)
python scripts/kitfolder.py PACK.json PACK.png kits/NAME --skip kp   # a kit folder's art from a packed kit (content.md 4.1)
python scripts/kitcreatures.py   # kits/pirates/creatures.toml from its table
python scripts/kitcheck.py KIT OUT; python scripts/pack.py KIT ARTROOT OUT; python scripts/frames64.py PAL IN OUT   # checks, atlas+manifest, frames (18-19)
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
  (`run_spell` is the one place a spell takes effect), `view.rs` the
  read-only queries a client asks instead of computing rules (client.md 1),
  `command.rs`/`event.rs` the `Command` and `Event` types, `combat.rs` melee
  and firing (`attack` and its blows, `fire` and its shots; who strikes
  whom is the step choice in `ai.rs`, firing `creature_fires` there, and
  `player_attack` in `step.rs`), `encounters.rs` world map encounters
  (the roll after a map step; the four areas are rooms 1..4 of a map
  region 0 the game keeps, `Game::region(0)`), `characters.rs` rolling a new character
  (`CreatePlayer`, `Newcomer::Create`), names and the profile's marks;
  `save.rs` save files, a replay of the command log
  (`docs/saves.md`); `describe` turns
  events into log lines. Live creatures are the region's 16 resident slots, edited in place. Tests reuse acs-core's fixture builder via `#[path]`.
- `crates/acs-bundle`: Extended bundles (`.advx`, content.md 2): read,
  check and write; Rust 1.88 for the zip library, so it stays out of
  acs-core. The engine sees only the `Adventure` in a bundle.
- `crates/acs-build`: `acs build`, a source folder of TOML to a bundle
  (content.md 3): keys to numbers, every reference and limit checked;
  `acs_core::encode` writes things' property bytes.
- `adventures/`: our own adventures' source folders (content.md 3, 6),
  CC BY-SA 4.0, each with a `walkthrough.txt` a test plays to the end.
- `kits/`: starter kits (content.md 4): atlas, picture list, things,
  creatures, provenance; CC BY-SA 4.0. An adventure takes only what it
  names.
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
  `memwatch.rs` is `run mem` (`--draws` logs every roll; `--answer T`
  presses through the messages creatures' pick-ups show; `--push e:300k`
  holds the stick from the run's start).
- `crates/acs-viewer`: eframe/egui viewer, native and web (trunk;
  `index.html`, `Trunk.toml`). Needs Rust 1.95 (eframe 0.37); the core stays
  at 1.85. `tiles.rs` computes the integer tile scale from the window in
  physical pixels each frame; `views/` holds one file per view; `views/walk.rs` is walk mode
  (keys, clicks and the touch pad to engine commands, events to the log;
  no rules in the viewer); `audio/` plays Play Music's cues through cpal
  (placeholders made in code, client.md 6).
- Repo: public at github.com/Dragynrain/ACSVM; Pages deploys `target/web` on
  every push to main. Commits use the GitHub noreply address.
