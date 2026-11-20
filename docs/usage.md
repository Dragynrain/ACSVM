# Building and using ACSVM

The reference for building the viewer and the `acs` inspector, their
command-line options, and the tests. The [README](../README.md) is the
short introduction. Licensed CC BY 4.0.

## Building

Requires Rust (stable; 1.85 or newer for the core and inspector, 1.95 or newer
for the viewer).

```bash
cargo build --release
```

The binaries are `target/release/acs` (inspector) and
`target/release/acs-viewer` (viewer).

Packaged desktop builds of the viewer for Windows, macOS and Linux come
from the Release workflow (`.github/workflows/release.yml`) when a `v*`
tag is pushed; `scripts/package.py` makes them
([client.md](client.md) 8).

## Using the viewer

```bash
acs-viewer                 # then drag a file onto the window, or use Open…
acs-viewer GAME.D64        # open a file straight away
```

Drop a C64 `.d64` disk image, a PC `ADVEN.FPY` / `ADVEN.HRD`, an Amiga `.adf`
disk image or an `.advx` bundle. The left panel
lists the world map, each region and its rooms, and the thing, creature,
message and diagnostics lists. Click a region to see all its rooms in place,
a room to open it, and a door (in a room or on the world map) to follow it;
Back (or Backspace) returns. Pictures are drawn at whole-number scales that fit
the window, with C64 pixels twice as wide as tall.

**Walk mode** (Walk in the top bar) puts a player at the adventure's start.
Move with the arrow keys or WASD, diagonally with Q E Z C (or Home, PgUp,
End, PgDn, or the numpad), and press Space to end a turn. The log on the right
lists what the rules engine reports: steps, blocked moves with the designer's
messages, doors, pick-ups, spells and what they did, random encounters, and
each creature's turn. The panel shows Life Force, Power and the other
ratings, the load against the carrying capacity and what the player
carries; Ready readies a weapon or puts on armor, Cast (spells, with their
cost and chance) and Use (magic items) open a menu of targets, and Drop asks
for a direction (Space drops on the player's own square, Escape cancels).
Below it, the creatures in the room are listed with their personality
(fighter or slinker; enemy, friend, neutral or thief), speed, Life Force,
Power against Wisdom, spells and readied weapon; a coloured dot on each
one's square shows its alignment (red enemy, green friend, yellow neutral,
purple thief). The viewer keeps the save the original makes when a visit
to another region begins, or at the end of a round in which SAVE GAME was
chosen; after a death, Resume from last save goes back to it (after a death
in a round that also saved, to the save before that round).
As in the original, the movement bar drains while you do nothing (a step
about every 7 seconds); tick Turn-based to make it wait for you instead.
Save to file… writes a small save file (the seed and the moves played,
[saves.md](saves.md)); Load from file…, or dropping the file on the
window, replays it over the same adventure. The adventure is not in the
save, so a save can go with a bug report.

`acs-viewer FILE --view room:1:0 --size 1280x800 --screenshot out.png` renders
one view and saves it as PNG (views: `world`, `region:R`, `room:R:N`, `things`,
`creatures`, `messages`, `diagnostics`, `walk`; R counts from 1, N from 0).
`--seed N --walk n,ne,e,...,wait` opens walk mode and plays those steps first;
steps can also be `idle` (one step of the bar drained), `ready:T`, `drop:T[:dir]`, `gold:N[:dir]`, `cast:T[:target]`,
`use:T[:target]`, `buy:T`, `sell:T`, `leave` (a store), `save` (SAVE GAME),
`fire[:X:Y[:T]]` (at a square, with weapon T) and `retire[:save|destroy]` (T is a thing number; a target is `me`, `pN` for player
N or `cS` for creature slot S in the player's region). `--place R:N:X:Y`
starts the walk on a room square instead of the start (a tool shortcut that
is not part of the replayable command log). `--save OUT.json` writes the
walk's save file after the steps, and `--load SAVE.json` opens one.
Keep such
screenshots outside the repository: they show the adventure's art.

### In a browser

The same viewer builds for the web with [trunk](https://trunk-rs.github.io):

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
scripts/web.sh serve       # http://127.0.0.1:8080, rebuilds on change
scripts/web.sh build       # static site in target/web/
```

The page reads the file you drop or pick inside the browser tab. Nothing is
uploaded: the page's content security policy only allows connections back to
the site it came from, and the viewer never makes any.

## Using the inspector

```bash
acs dump GAME.D64                    # summary on stdout, JSON next to the input
acs dump GAME.D64 -v -o game.json    # list every region, room, thing and creature
acs pictures GAME.D64 out/ --scale 4 --sheet
```

`dump` writes the canonical adventure model as JSON (`<name>.acs.json` by
default; that pattern is git-ignored because it is derived from your files).
`pictures` writes one PNG per picture, with C64 pixels doubled in width, and
optionally a contact sheet. Warnings about odd data go to stderr; the exit code
is 2 if the decoder hit data it could not interpret.

## Testing

```bash
cargo test
```

Unit and synthetic tests build their fixtures byte by byte and need no
adventure files. Tests against real adventures read a corpus folder outside
the repository and skip unless `ACSVM_CORPUS` points to it:

```bash
ACSVM_CORPUS=/path/to/corpus cargo test --test corpus -- --nocapture
```

The corpus folder holds `c64/` (`.d64` disks, searched recursively) and
optionally `pc/` (`.fpy`/`.hrd`).

## Repository layout

- `crates/acs-core`: library with no GUI: disk readers (C64, PC, Amiga),
  platform layouts, text codecs, picture decoders and the canonical model
  (serde).
- `crates/acs-engine`: the rules engine, also GUI-free: commands in, events
  out, one seeded random generator, so a game replays from its seed and
  command log.
- `crates/acs-bundle`: `.advx` bundles, the format our own adventures ship in
  ([content.md](content.md) 2).
- `crates/acs-build`: `acs build`, an adventure's source folder of TOML to a
  bundle ([content.md](content.md) 3).
- `crates/acs-cli`: the `acs` binary (`dump`, `pictures`, `extract`, `build`,
  `bundle`, `play`).
- `crates/acs-viewer`: the viewer (eframe + egui; desktop and web). Rendering
  lives here; what goes on each room square is worked out in `acs-core`
  (`scene.rs`).
- `crates/acs-capture`: the capture harness that measures the original in
  the VICE emulator ([capture.md](capture.md)).
- `adventures/`: our own adventures' source folders; `kits/`: starter kits
  (art, things and creatures) they are built from.
- `docs/`: [design](design.md), [rules specs](rules/movement.md),
  [format notes](format-notes.md) and the
  [provenance record](provenance.md).
