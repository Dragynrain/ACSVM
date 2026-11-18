# The modern client (Phase 5)

What the player sees and touches: layout, the message log, input, render
modes, audio, accessibility and builds. Phase 5's plan and the client's
spec; each section is written before its code, like `docs/rules/`.
Licensed CC BY 4.0. Code: `crates/acs-viewer` (walk mode is
`src/views/walk.rs`).

The client grows out of the viewer: eframe/egui, one codebase for the
desktop and the browser (trunk). The design's first plan drew tiles with
macroquad (design.md, "Modern client in Rust"). The viewer already draws
them in egui with whole-number scaling from one tile-size calculation
(`tiles.rs`), so it stays on egui and nothing is drawn twice.

## 1. The engine boundary

The client holds no rules. It sends `Command`s, draws from the
`Event`s they return, and reads state through `Game`'s accessors. When it
needs an answer the rules give (a price, a chance, a mark), it asks a
query in `acs-engine`'s `view.rs` and does not call a formula itself.
A missing answer means a new query there, with a test, and not
arithmetic in the client. The queries so far:

| Query | Answers |
| --- | --- |
| `Game::store_offers` | The open store's shelves (offered and in stock), prices, whether Buy would be refused or the thing lost; what the visitor could sell and for how much (stores 2.7, 4, 5) |
| `Game::cast_chance` | A spell's chance for a player's Power now (magic 4.1) |
| `Game::residents_in` | The live creatures in a room, with their slots |
| `Player::rating`, `Player::mark` | A profile rating and its mark (characters 4) |

## 2. Layout

Sizes are logical points; egui multiplies by the screen's pixels per
point, so a 4K screen at 200% lays out like 1920 x 1080. A window
narrower than 900 points is **narrow** (`NARROW`, `app.rs`).

| Area | Wide | Narrow |
| --- | --- | --- |
| Top bar | Menu and Info toggles, Open, Walk, Back, the adventure | The same; it wraps |
| Menu (left) | Shown; in walk mode only from 1500 points wide, so the board keeps its room | Hidden until toggled |
| Board (centre) | The map or room, the tile scale fitted to what is left | The same |
| Info column (right) | Shown, 200 to 480 points and at most about a third of the window; long lines wrap; scrolls when taller than the window | Hidden until toggled |
| Message log (bottom, walk mode) | Shown, resizable, about a fifth of the height, or all the height the 3:2 board can't use if that is more; at most 60% | Shown, a sixth, or the board's spare height |

- Nothing clips: a column taller than its space scrolls, and a row wider
  than its space wraps.
- The message log is always visible in walk mode, whatever else is
  hidden: it replaces the original's press-to-acknowledge messages (3).
- The board keeps at least the room's 15 x 10 squares at scale 1; below
  that the window's minimum size (480 x 320) stops shrinking.

**Checked sizes** (the gate, 10), in points: laptop 1280 x 800 and
1366 x 768; 4K at 200% (1920 x 1080) and 150% (2560 x 1440); tablet
1024 x 768 and 768 x 1024 (a browser's CSS pixels).
`acs-viewer FILE --size WxH --ppp 1 [--view V | --seed N --walk ...] --screenshot OUT.png`
renders one. `--ppp 1` draws one pixel per point whatever the screen's
own scale, so even 2560 x 1440 points fit on a 4K screen at 175%; the
layout depends only on points. Screenshots of real adventures stay
outside the repo.

## 3. The message log

Every event the engine returns becomes a line (`describe`), coloured by
kind: turns and steps dim, blocks orange, doors blue, spells violet,
items green, creatures yellow, foes red. A long message keeps its stored
lines and is never rewrapped (magic 3.10). The log keeps the last 500
lines and sticks to the bottom unless scrolled up.

Later in the phase: a filter by kind, and the font size option (7).

## 4. Input

- **Keyboard** (now): the original's eight directions and its commands
  (`key_command`, walk.rs).
- **Mouse and taps** on the board (a tap is a click). What a click on a
  square does depends on what is waiting:
  - Nothing waiting: a square beside the player (any of the eight,
    across the map's wraparound) is a step that way. The player's own
    square and farther squares do nothing: walking a route there would
    be a rule, and the client holds none.
  - A drop waiting (Drop, then a direction): the player's own square
    drops it there, a square beside it drops it that way; other squares
    do nothing.
  - A target cursor up (FIRE, or Cast or Use at "a square…"): the
    cursor follows the mouse over the room's inside squares, and a click
    on one sends the command at it. Wall squares do nothing.
  Cast and Use offer "a square…" beside the people in the room when
  the player is in a room; it raises the same cursor as FIRE, moved by
  the direction keys, sent with Space or Enter, cancelled with Escape.
- **Touch**: an on-screen pad in its own strip along the bottom of the
  window, under the thumbs and never over the board: eight arrows round
  a Wait key on the left, each 52 points; on the right, while a drop or
  a cursor waits, a Cancel key, and on the map or holding a missile
  weapon, a Fire key. It shows when the last input was a touch, and
  hides on the next key press; the info column's "On-screen pad" option
  fixes it on or off. The pad's keys are the keyboard's: they go
  through the same handling, cursor and drop included.
- **Gamepads**: up to four, one per hot-seat player, each with its own
  bindings (design.md component 11).

Every input becomes the same `Command`; nothing reaches the engine any
other way.

## 5. Render modes

| Mode | What it draws |
| --- | --- |
| Original | The adventure's pixels, whole-number scaling (now) |
| Smoothed | The original tiles upscaled 4x once at load (Scale2x twice), drawn with linear filtering and mipmaps at any scale |
| HD pack | Replacement art for tiles whose hash a pack lists; the rest Smoothed |
| Native HD | An Extended bundle's own art (content.md 2), keyed by picture number; the rest Smoothed. The default for a bundle with art. A room square is drawn in layers (`scene::layers`): wall or floor, the nearest space under the top thing (the ground it stands on), the top thing, then a resident, so art with transparent parts stands on its ground; anything else in the stack stays hidden, as in the original |

**Smoothed.** Every picture, at its display size (16 x 16; a C64
picture's pixels already doubled in width), goes through Scale2x twice
(Andrea Mazzoleni's AdvMAME2x rules, written here from the published
description): each pixel becomes four, and a corner takes its two
neighbours' colour where they agree and the other two differ, so
staircases become diagonals and nothing new is invented. The 64 x 64
result is drawn with linear filtering and mipmaps, and the board's tile
size is the largest that fits, whole number or not; the grid's origin
stays on a physical pixel. Upscaling a whole adventure takes
milliseconds, so it happens when the mode is first chosen and nothing is
cached between sessions (the design's IndexedDB cache isn't needed).
The top bar's Look menu switches between Original and Smoothed for every
view; `--render smoothed` starts in it.

**HD packs.** A pack is replacement art for original pictures, keyed by
a hash of each picture's pixels, so it names a tile without containing
it (design.md, "Render modes"). Each replacement is drawn fresh from what
the tile shows, never traced.

*File.* One PNG, so a pack is one file to drop or open:

- The image is an atlas of square tiles, all `tile` pixels a side (16
  to 512), placed left to right and then top to bottom; place `at` is
  column `at % (width / tile)`, row `at / (width / tile)`. The width is
  a whole number of tiles; spare places may stay empty.
- An iTXt chunk with the keyword `acsvm-hd-pack` holds the manifest, a
  JSON object:

  ```json
  {
    "format": "acsvm-hd-pack",
    "version": 1,
    "name": "Stone and water",
    "license": "CC BY-SA 4.0",
    "tile": 128,
    "tiles": [{ "hash": "5d0f9e1c2b3a4f60", "at": 0, "note": "stone wall" }]
  }
  ```

  `name`, `license` and each `note` are free text (`note` optional).
  Several hashes may share one place. A later entry for a hash already
  listed is ignored. A file without the chunk is not a pack.

*Hash.* FNV-1a, 64-bit (offset basis `cbf29ce484222325`, prime
`100000001b3`), over the picture's RGBA bytes at its display size, row
by row (16 x 16 x 4 = 1,024 bytes; a C64 picture's pixels doubled in
width as shown). Written as 16 lowercase hexadecimal digits. The bytes
are after the adventure's palette, so a tile an adventure recolours is
another tile: the replacement's colours are fixed. [choice]

*Use.* Dropping or opening a pack (or `--pack FILE`) loads it for every
adventure opened afterwards in the session, and the Look menu gains
**HD pack**, which it switches to. In that mode a picture the pack
lists is drawn from the atlas with linear filtering and mipmaps; any
other is drawn Smoothed. Scale as in Smoothed: the largest that fits.
A broken pack (no chunk, bad JSON, a tile size or place outside the
image) is refused with a message and changes nothing.

*Coverage.* The Lists menu's **HD pack** view shows the pack (name,
license, tile size, entries) and, for the open adventure, its
**coverage**: the pictures the pack replaces out of those that aren't
blank (a picture of one colour is blank and not counted), as a count
and a percentage; then every picture it misses, with the picture, its
number and its hash. "Copy missing" copies the missing pictures as
manifest entries (hash, `at` left 0, the picture number in `note`), the
start of a pack for an artist.

`scripts/hdpack.py ATLAS.png MANIFEST.json OUT.png` writes a pack from
an atlas and a manifest; `--check` reads one back.

## 6. Audio

Play Music (magic 3, action 11) reaches the client as `Event::Music`
with the 1-based choice: 1-12 are effects, 13-30 themes, and the themes
14-17 loop until other music plays. [manual] Whether a cue starting
while another plays mixes with it or replaces it is [unknown]; **the
client's choice:** one cue at a time, and a new one replaces whatever
plays, a looping theme included. Every other cue plays once and stops.

**Cues.** 30 cues of our own, composed for the project (content.md 5):
scores in `audio/music.rs`, rendered by `audio/synth.rs` when the
output opens, so nothing is shipped or copied. Effects 1-12 are under a
second; themes are short pieces, and the looping ones (14-17) end on
silence, so they repeat without a click. The original's own audio,
read from the user's disk, can come later through research.

**Output.** `cpal`, on the desktop and in the browser (its Web Audio
host). The output opens at the first cue after an input, since a
browser plays nothing before one; a cue arriving before then is
dropped. Cues from the commands a session starts with (`--walk`, a
loaded save's replay) are not played.

**Controls.** In the info column: a volume slider (default 70%) and a
Sound checkbox to mute. Muting stops what plays.

## 7. Accessibility

The top bar's **Options** menu holds three settings, for every view and
every adventure in the session (not yet kept between sessions).

- **Text size**, 75% to 200% (default 100%): every font in the panels,
  lists, menus and the log. The board's tiles keep their own size.
  `--text-size PCT` starts with it.
- **Colour-blind palette** (off by default): the Okabe-Ito colours,
  told apart under the common colour-vision deficiencies, replace the
  log's kind colours, the player markers', the creatures' alignment
  marks' and the world map's door, exit and start outlines.
  `--colour-blind` starts with it.
- **Reduce motion**: the pulsing outline on a square a door leads to
  stays still. In the browser it starts on when the system asks for
  reduced motion (`prefers-reduced-motion`).

**Nothing told apart by colour alone**, whichever palette:

| What | Besides its colour |
| --- | --- |
| Log kinds | A symbol before the line: `×` blocked, `»` doors and places, `*` spells, `+` things, `~` creatures, `!` foes and harm, `¶` a long message; turns, steps and notes have none |
| Player markers | The player's number in the ring |
| Creature alignment | A shape: enemy a triangle, friend a circle, neutral a square, thief a diamond |
| World map squares | Door a plain outline, exit point corner brackets, start a double outline |

**Screen readers.** In the browser, every new log line is also put in a
hidden live region (`role="log"`, `aria-live="polite"`) that keeps the
last 50 lines, so a screen reader reads play as it happens. The desktop
build has no screen reader support yet.

### 7.1 The UI font

The kit's pixel font (`docs/art/pirates-style.md` 12) is the game's
voice: the message log, the title screen and its lettering. Menus,
lists and the stats keep egui's font, where density matters more.
`scripts/font.py --ttf-only` writes it as outline fonts
(`crates/acs-viewer/assets/font.ttf`, `font-bold.ttf`), each glyph
pixel a square and the em the 11-row cell, and `fonts.rs` loads them
with hinting and sub-pixel placement off. A size is always a whole
multiple of the font's pixel on the screen (`fonts::kit`: the wanted
size times the text-size option, rounded to a multiple of 11 physical
pixels, at least 1x), so the text stays crisp at any scale; the log is
at 2x. Long messages keep their stored lines (3), and are told apart by
colour, not italics, which a pixel font can't slant cleanly. Characters
outside Latin-1 fall back to egui's font.

## 8. Builds

The web build stays on Pages (`scripts/web.sh build`). Desktop builds come
from the **Release** workflow (`.github/workflows/release.yml`): pushing a
`v*` tag builds all three and opens a *draft* GitHub release with them,
for the maintainer to read over and publish; running the workflow by hand
builds them as workflow artifacts only.

| Platform | Runner | Build | Package |
| --- | --- | --- | --- |
| Windows | `windows-latest` | `x86_64-pc-windows-msvc`, with the `windows-gui` feature: no console window | `acs-viewer-VERSION-windows-x64.zip` |
| macOS | `macos-15` | `aarch64-apple-darwin` and `x86_64-apple-darwin`, joined with `lipo` | `acs-viewer-VERSION-macos-universal.zip`, holding `acs-viewer.app` |
| Linux | `ubuntu-22.04` (an older glibc, so it runs on more systems) | `x86_64-unknown-linux-gnu` | `acs-viewer-VERSION-linux-x64.tar.gz` |

- They build with the `dist` profile: the release profile plus full LTO,
  one codegen unit and stripped symbols. A local `cargo build --release` stays as it is,
  console and all, for the screenshot and test tools.
- `scripts/package.py` makes each package: the program, `LICENSE`,
  `LICENSES/`, and a `README.txt` naming the project (from the
  workspace's description), how to start it, that no adventure comes
  with it, and the platform's notes: macOS builds are unsigned (open the
  app with right-click, Open, the first time); Linux needs X11 or
  Wayland, OpenGL and ALSA.
- None bundle any EA material; the player's files are opened or
  dropped on the window. Our own sample adventures are built in
  (content.md 7). The forbidden-file check runs on the tree the
  release is built from.

## 9. Order of work

Phase 5's gate is met (10); next is Phase 6, original content and 1.0.

1. Layout: the message log in its own panel, the info column scrolling
   (2, 3). Done November 2026.
2. A screenshot sweep at the checked sizes, fixing what breaks. Done
   November 2026, every view at the six sizes (`--ppp 1`). Fixed: the
   log panel kept a size from before the window's last resize (a
   `--ppp` artifact, cleared once the size is reached); the menu
   column squeezed the walk board below 1500 points; the info column
   took up to 43% of a 1024-point window; the log left tall windows'
   spare height empty. Left as they are: Original mode's whole-number
   scale leaves the 40 x 40 world map at scale 1 when 2 is just out of
   reach (Smoothed, step 5, scales freely, and does); the creature list's
   stats table is a fixed width, so long values wrap; the message list
   is the original's 32-character column.
3. Mouse targeting and the touch pad (4). Done November 2026
   (`--pad` shows the pad for screenshots).
4. Audio: the cue player with placeholder cues (done November 2026;
   `crates/acs-viewer/src/audio`), then our 30 (6). Two ignored tests
   help by ear: `plays_a_cue_on_the_default_output`, and
   `write_cues_as_wav` with `ACSVM_CUES_DIR` set.
5. Smoothed mode (5). Done November 2026 (`smooth.rs`, `tiles.rs`);
   the sweep's world map at 1920 x 1080 now fills its height.
6. The HD pack format, loading and the coverage report (5). Done
   November 2026 (`hdpack.rs`, `views/pack.rs`, `scripts/hdpack.py`);
   the test `a_pack_loads_and_reports_its_coverage` is the gate's pack
   check on a test adventure of our own.
7. Accessibility options (7). Done November 2026 (`access.rs`,
   `live.rs`). Known: at large text sizes a bar's caption (Life Force,
   Power) can run past a narrow info column, since captions don't wrap.
8. Desktop builds (8). The workflow and packaging are in place
   (November 2026); the first tagged run is the maintainer's.

## 10. Gate

No layout breaks at the checked sizes (2) in walk mode, the map and
every list view; an HD pack loads and its coverage report is right for
a test adventure of our own.

**Met, November 2026.** Every view (walk mode on the map and in a room
with creatures, the world map, a region, a room, things, creatures,
messages, diagnostics and the HD pack view) at the six checked sizes,
with an HD pack loaded, the touch pad on and two players; and walk mode
and the pack view again at 150% text with the colour-blind palette, at
the two tablet sizes. Two breaks turned up and were fixed: the top bar
ran under Fullscreen at large text (Fullscreen now takes its place
first, and the adventure's name and file elide), and the Things list cut
entries off on a 768-point window (list views' list column may now take
about half the window, and entries that still don't fit end in "…").
Left as they are: the menu column scrolls on hover only; at large text
a bar's caption can run past a narrow info column. The pack half is the
test `a_pack_loads_and_reports_its_coverage`, and a demo pack built
with `scripts/hdpack.py` covered exactly what it was built to on
*Rivers of Light* (not committed: its hashes come from EA's pictures).
