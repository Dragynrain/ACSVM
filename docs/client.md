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
| Menu (left) | Shown | Hidden until toggled |
| Board (centre) | The map or room, the tile scale fitted to what is left | The same |
| Info column (right, walk mode) | Shown; scrolls when taller than the window | Hidden until toggled |
| Message log (bottom, walk mode) | Shown, resizable, about a fifth of the height | Shown, shorter |

- Nothing clips: a column taller than its space scrolls, and a row wider
  than its space wraps.
- The message log is always visible in walk mode, whatever else is
  hidden: it replaces the original's press-to-acknowledge messages (3).
- The board keeps at least the room's 15 x 10 squares at scale 1; below
  that the window's minimum size (480 x 320) stops shrinking.

**Checked sizes** (the gate, 10), in points: laptop 1280 x 800 and
1366 x 768; 4K at 200% (1920 x 1080) and 150% (2560 x 1440); tablet
1024 x 768 and 768 x 1024 (a browser's CSS pixels). `acs-viewer FILE --size WxH --view walk --screenshot OUT.png`
renders one; screenshots of real adventures stay outside the repo.

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
- **Mouse**: clicking a square beside the player moves there; while
  aiming FIRE or a cast, clicking a square picks it.
- **Touch**: an on-screen eight-way pad and the command buttons, shown
  when the last input was a touch.
- **Gamepads**: up to four, one per hot-seat player, each with its own
  bindings (design.md component 11).

Every input becomes the same `Command`; nothing reaches the engine any
other way.

## 5. Render modes

| Mode | What it draws |
| --- | --- |
| Original | The adventure's pixels, whole-number scaling (now) |
| Smoothed | The original tiles upscaled once at load by an xBRZ-style filter, then any scale; cached per adventure |
| HD pack | Replacement art for tiles whose hash a pack lists; the rest Smoothed |
| Native HD | 128 px art stored in the extended format (Phase 6; a `MODEL_VERSION` bump) |

An HD pack is keyed by a hash of the original tile's decoded pixels, so
the pack identifies a tile without containing it (design.md, "Render
modes"). The client shows a pack's **coverage** for the open adventure:
how many of its pictures the pack replaces, and which it doesn't. The
pack's file format (manifest, hash, image layout) is written here before
its code.

## 6. Audio

Play Music (magic 3, action 11) reaches the client as `Event::Music`
with the 1-based choice: 1-12 are effects, played once over anything
else; 13-30 are themes, and 14-17 loop until other music plays (another
theme or an effect starts). The client plays 30 cues of our own; the
original's audio from the user's own disk can come later, through
research. A volume control and mute; muted until the first input in the
browser, which can't play sound before one.

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
2. A screenshot sweep at the checked sizes, fixing what breaks.
3. Mouse targeting and the touch pad (4).
4. Audio: the cue player with placeholder cues, then our 30 (6).
5. Smoothed mode (5).
6. The HD pack format, loading and the coverage report (5).
7. Accessibility options (7).
8. Desktop builds (8).

## 10. Gate

No layout breaks at the checked sizes (2) in walk mode, the map and
every list view; an HD pack loads and its coverage report is right for
a test adventure of our own.
