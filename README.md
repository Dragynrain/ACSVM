# ACSVM

ACSVM (working name) is an open-source engine that plays adventures made with
*Adventure Construction Set* (Electronic Arts, 1984), in the spirit of ScummVM:
the engine is free software, and you bring the adventure files you own.

**Status: Phase 5 done (modern client and audio); next, Phase 6 (original
content and 1.0).** The rules engine plays whole adventures: movement,
inventory, creatures and their AI, the 15 spell actions, combat, stores,
new characters, world map encounters and saves, each rule specified in
`docs/rules/` and checked against the original C64 program, measured in
the VICE emulator by a capture harness ([docs/capture.md](docs/capture.md)).
Two players have finished *Rivers of Light*, the adventure that comes with
ACS, in hot-seat. The viewer plays adventures on the desktop and in a
browser ([try it](https://dragynrain.github.io/ACSVM/) with your own file):
keyboard, mouse and an on-screen touch pad, a message log, the original's
music cues (placeholders for now), Original and Smoothed looks, HD packs of
replacement art with a coverage report, and accessibility options. An
inspector decodes C64 and PC adventures into JSON and exports their
pictures. See
[docs/design.md](docs/design.md) for the full plan and
[docs/rules/](docs/rules/movement.md) for the rules as specified.

This project is not affiliated with or endorsed by Electronic Arts. It ships no
EA program code, art, music, manual text or adventures.

## Supported files

| Platform | File | Status |
| --- | --- | --- |
| C64 | whole `.d64` disk image | Decodes all 14 adventures in the private test corpus with no errors |
| PC (MS-DOS) | `ADVEN.FPY`, `ADVEN.HRD`, `ACS.FPY` | Decodes the one real PC adventure tested so far (EA's built-in one), plus synthetic tests |
| Apple II, Amiga | `.dsk`, `.adf` | Not yet |

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
([docs/client.md](docs/client.md) 8).

## Using the viewer

```bash
acs-viewer                 # then drag a file onto the window, or use Open…
acs-viewer GAME.D64        # open a file straight away
```

Drop a C64 `.d64` disk image or a PC `ADVEN.FPY` / `ADVEN.HRD`. The left panel
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
Save game writes a small save file (the seed and the moves played,
[docs/saves.md](docs/saves.md)); Load game, or dropping the file on the
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

- `crates/acs-core`: library with no GUI: disk readers, platform layouts,
  text codecs, picture decoders and the canonical model (serde).
- `crates/acs-engine`: the rules engine, also GUI-free: commands in, events
  out, one seeded random generator, so a game replays from its seed and
  command log.
- `crates/acs-cli`: the `acs` inspector binary.
- `crates/acs-viewer`: the viewer (eframe + egui; desktop and web). Rendering
  lives here; what goes on each room square is worked out in `acs-core`
  (`scene.rs`).
- `docs/`: design snapshot, [rules specs](docs/rules/movement.md),
  [format notes](docs/format-notes.md) and the
  [provenance record](docs/provenance.md).

## Licenses

- Code: [GPL-3.0-or-later](LICENSE). Parts of the decoder are ported from Dan
  Boris's [ACS Viewer](https://github.com/danlb2000/ACSViewer) (GPL-3.0); see
  [docs/provenance.md](docs/provenance.md).
- Art, audio and sample adventures (none yet): [CC BY-SA 4.0](LICENSES/CC-BY-SA-4.0.txt).
- Documentation: [CC BY 4.0](LICENSES/CC-BY-4.0.txt).

Contributions follow [CONTRIBUTING.md](CONTRIBUTING.md), including the
clean-room policy and the rule that no EA material ever enters the repository.
