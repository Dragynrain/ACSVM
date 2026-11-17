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
| Native HD | 128 px art stored in the extended format (Phase 6; a `MODEL_VERSION` bump) |

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

An HD pack is keyed by a hash of the original tile's decoded pixels, so
the pack identifies a tile without containing it (design.md, "Render
modes"). The client shows a pack's **coverage** for the open adventure:
how many of its pictures the pack replaces, and which it doesn't. The
pack's file format (manifest, hash, image layout) is written here before
its code.

## 6. Audio

Play Music (magic 3, action 11) reaches the client as `Event::Music`
with the 1-based choice: 1-12 are effects, 13-30 themes, and the themes
14-17 loop until other music plays. [manual] Whether a cue starting
while another plays mixes with it or replaces it is [unknown]; **the
client's choice:** one cue at a time, and a new one replaces whatever
plays, a looping theme included. Every other cue plays once and stops.

**Cues.** 30 cues of our own (Phase 6 makes the finished set). Until
then the client makes placeholders in code, so nothing is shipped or
copied: effects are short sounds under a second (sweeps, blips, noise
bursts), each choice its own; themes are short tunes of a few seconds
on a pentatonic scale, each choice its own seed; the looping themes end
on silence, so they repeat without a click. The original's own audio,
read from the user's disk, can come later through research.

**Output.** `cpal`, on the desktop and in the browser (its Web Audio
host). The output opens at the first cue after an input, since a
browser plays nothing before one; a cue arriving before then is
dropped. Cues from the commands a session starts with (`--walk`, a
loaded save's replay) are not played.

**Controls.** In the info column: a volume slider (default 70%) and a
Sound checkbox to mute. Muting stops what plays.

## 7. Accessibility

A font size option for the panels and the log; a colour-blind palette for
the log's kinds and the player markers (none of them told apart by
colour alone); in the browser, the log mirrored into a live region a
screen reader announces; reduced motion turns off pulses and smooth
movement.

## 8. Builds

The web build stays on Pages (`scripts/web.sh build`). Desktop builds for
Windows, macOS and Linux come from CI as release artifacts. None bundle
an adventure: files are opened or dropped on the window.

## 9. Order of work

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
6. The HD pack format, loading and the coverage report (5).
7. Accessibility options (7).
8. Desktop builds (8).

## 10. Gate

No layout breaks at the checked sizes (2) in walk mode, the map and
every list view; an HD pack loads and its coverage report is right for
a test adventure of our own.
