# Original content and 1.0 (Phase 6)

Our own adventures and everything they need: the Extended bundle they
ship in, a way to write them without an editor, the starter kits, the 30
cues, the two sample adventures, a front door for the browser build, the
playtest and the 1.0 release. Phase 6's plan and the spec of the bundle
and the source format; each section is written before its code, like
`docs/rules/` and `client.md`. Licensed CC BY 4.0.

The rules don't change in this phase. Our adventures play under the
Extended profile (design.md, "Extended adventures"): the same rules as
Classic with larger caps, and no rule features in 1.0.

## 1. The gate

1.0: the browser build at the project's Pages site plays both sample
adventures start to finish with no EA file anywhere in the site or the
repository, and a public playtest with long-time ACS players has run
and its findings are dealt with. Each sample adventure also finishes by
an automated replay (a corpus-free test, since the adventures are ours),
as *Rivers of Light* does for Phase 4.

## 2. The bundle

An Extended adventure travels as one file, a zip archive with the
extension **`.advx`** ("adventure, extended"). [choice] The name avoids
the ACS mark (design.md, open question); it is one constant
(`acs_bundle::EXTENSION`), so a rename is cheap.

Zip entries (stored or deflate; nothing else):

| Entry | Required | What it holds |
| --- | --- | --- |
| `bundle.json` | yes | The manifest below |
| `adventure.json` | yes | The canonical model (`acs_core::Adventure`) |
| `art/pictures.png` | no | Native HD art: an atlas of square tiles |

Any other entry is ignored, so later versions can add entries (audio,
scene pictures) that older readers skip. A path with `..`, a leading
`/` or a backslash is refused.

**`bundle.json`:**

```json
{
  "format": "acsvm-adventure",
  "version": 1,
  "name": "The King's Pardon",
  "byline": "ACSVM contributors",
  "license": "CC BY-SA 4.0",
  "credits": ["Pirate kit art: ..."],
  "features": [],
  "art": { "tile": 64, "pictures": [{ "picture": 0, "at": 0 }] }
}
```

- `format` must be `acsvm-adventure` and `version` 1, or the file is
  refused.
- `name`, `byline`, `license` and `credits` are free text for the open
  dialog and the title screen; the model's own `name` and `byline` are
  what the game shows in play. `credits` may be empty.
- `features` names rule extensions (design.md). The engine knows none
  yet, so a bundle listing any is refused with the names it lacks.
- `art` is present exactly when `art/pictures.png` is. `tile` is 16 to
  512 pixels; place `at` sits in the atlas as in an HD pack (client.md
  5): column `at % (width / tile)`, row `at / (width / tile)`.
  `picture` is a picture number in the model. A picture listed twice
  keeps its first place; a picture not listed has no HD art.

**`adventure.json`** is the model as `acs dump` writes it, with
`profile` `extended`. Its `model_version` may not be newer than the
reader's `MODEL_VERSION` (refused, "made by a newer version"); older
ones are read as they are while the model stays compatible, and get a
migration when it doesn't.

**Pictures.** Every picture keeps its Classic pixels in the model, so
Original and Smoothed modes and everything that hashes pictures work
unchanged. The build step (3) writes them from the HD art, by a
downscale snapped to the adventure's four colours. The HD art is drawn
in the client's **Native HD** mode, the default for a bundle with art:
the same drawing as an HD pack's replacements (linear filtering,
mipmaps, the largest scale that fits), keyed by picture number instead
of hash. A picture without HD art is drawn Smoothed.

**Loading** checks, in order: the zip, the two required entries, the
manifest's format, version and features, the model's version, the
profile (`extended`; a Classic model in a bundle is refused, since
Classic means decoded from an original file), the limits
(`limits::check` against the model's own `limits`, and those limits
within `Limits::EXTENDED`), and the art (atlas decodes, tile divides
it, every place and picture number in range). Each failure is one
message naming the entry; nothing loads in part.

**Writing** (`acs_bundle::write`) puts `bundle.json` first, then
`adventure.json`, then the art, deflated, with fixed timestamps, so the
same adventure always makes the same bytes. [choice]

The reader and writer are `crates/acs-bundle`, used by the CLI and the
viewer. It is its own crate because the zip library needs Rust 1.88
and acs-core stays at 1.85; the engine never sees a bundle, only the
`Adventure` in it. Bundles are build output: the repository holds their
sources (3), and `.zip`-type files stay out of it (design.md,
"Repository rules").

## 3. Writing adventures

To be specified before its code. Until the editor (after 1.0), an
adventure is a folder of text the CLI builds into a bundle
(`acs build DIR -o OUT.advx`): things, creatures, regions and rooms by
name rather than number, room maps drawn as text grids with a legend,
messages in their own file, and the art as the kit's packed atlas plus a
picture list. The build checks every reference and limit and reports
them with the file and line. The folder is what's committed (art as the
kit's PNGs, under CC BY-SA 4.0).

## 4. Starter kits

Four kits at 128 px (design.md, Kit 4): pirates first, then the three
runners-up. The pirate kit's art exists outside the repository
(`ACSVM-research/art/pirates`, `docs/art/pirates-style.md`); what it
lacks for play is its things and creatures as data (names, stats,
types) and its packed atlas as a kit folder the build reads.

## 5. Audio

The 30 original cues (client.md 6), replacing the placeholders made in
code: 12 effects and 18 themes, 4 of them looping. To be specified.

## 6. Sample adventures

- **A tutorial**, small, on the pirate kit, teaching movement, rooms,
  things, a store, a fight and a spell in about fifteen minutes.
- ***The King's Pardon***, mid-size, on the pirate kit; its story,
  design and script are their own project (`D:\Projects\KingsPardon`),
  and only the built adventure's source comes into this repository.

Both under CC BY-SA 4.0, both also test fixtures.

## 7. The front door

The browser build opens on a title screen of our own with the sample
adventures to pick, plus the existing open-a-file path for the
player's own disks. Nothing EA's is bundled (client.md 8).

## 8. Order of work

1. The bundle (2). Done November 2026: `crates/acs-bundle` reads,
   checks and writes bundles (`tests/bundle.rs`); every `acs` command
   and the viewer open them (a drop or the picker, by extension or zip
   signature); the viewer's Look menu gains **Native HD**, the default
   for a bundle with art (`--render native`). `acs bundle MODEL.json -o
   OUT.advx [--art ATLAS.png --tile N]` wraps an Extended model, place N
   of the atlas holding picture N: a stopgap until `acs build`.
2. The source format and `acs build` (3).
3. The pirate kit as data (4), and a test adventure built from it.
4. The tutorial (6), with its replay test.
5. The 30 cues (5).
6. *The King's Pardon* (6), with its replay test.
7. The front door (7).
8. The other three kits (4).
9. The playtest, then 1.0 (1).
