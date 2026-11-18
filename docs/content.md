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

Until the editor (after 1.0), an adventure is a folder of TOML that
`acs build DIR -o OUT.advx` turns into a bundle. Everything is named by a
**key** (lowercase letters, digits and `_`), never by number: the build
gives out the numbers, keeping the conventions the engine and the rules
assume (below), and checks every reference, range and limit. A problem
is reported with its file and what it is in; the build writes nothing
if there is any. The folder is what's committed (art as PNG, under
CC BY-SA 4.0); the bundle is build output. Code: `crates/acs-build`.

### 3.1 Files

| File | Holds |
| --- | --- |
| `adventure.toml` | The adventure: name, byline, licence, credits, intro, theme, palette, creature classes, limits, and the built-in things' looks |
| `pictures.toml` | The art atlas and the picture list (optional with a kit, 4.2) |
| `things.toml` | The master thing list |
| `creatures.toml` | The master creature list |
| `messages.toml` | Long and short messages |
| `world.toml` | Terrain, the map, its doors and its encounters |
| `regions/*.toml` | One region each, in file-name order (`01-port.toml`, ...) |

A key is unique within its kind (two things can't share one; a thing
and a creature can). Rooms are named `region.room`. Unknown fields are
errors, so a misspelt field is caught.

### 3.2 `adventure.toml`

```toml
name = "THE KING'S PARDON"        # the model's name, shown in play
byline = "ACSVM contributors"
license = "CC BY-SA 4.0"
credits = ["Pirate kit art: ..."]
intro = """..."""                 # the intro text
theme = "fantasy"                 # fantasy | spy_mystery | sci_fi: the endless music
palette = ["black", "brown", "light cyan", "white"]   # PC colour names, Classic pixels only
classes = ["PIRATE", "NAVY", "ISLANDER"]              # creature classes, in order
kit = "../../kits/pirates"        # optional: a starter kit (4.2)

[limits]                          # caps above Classic's; the rest stay Classic's
pictures = 160

[gold]                            # thing 81
name = "GOLD"
picture = "gold_pile"
weight = 1

[area_door]                       # thing 82, the encounter areas' exit door
name = "EXIT"
picture = "cave_passage"

[floor]                           # thing 127, and the wall rooms default to
picture = "sand_floor"            # default: the blank picture
wall = "plank_wall"
```

The model's `limits` are `Limits::CLASSIC` with `[limits]` laid over it
(field names as in `Limits`), each value within `Limits::EXTENDED`; its
profile is Extended and its platform PC. [choice: Classic sizes unless
an adventure asks, so our adventures play inside the sizes the golden
tests cover.]

### 3.3 Pictures

`pictures.toml` is the adventure's own art; with a kit (4.2) it is
optional.

```toml
atlas = "art/kit.png"   # RGBA, square tiles left to right, then down
tile = 64

[pictures]              # in order; the value is the atlas place
sand_floor = 0
plank_wall = 1
cutlass = 14
```

Picture 0 is the build's own blank picture (no key; floors and slots
with nothing to show); the list follows from 1, in file order. Each
listed picture gets its atlas tile as Native HD art and **Classic
pixels** made from it: the tile is box-averaged to 16 x 16 (transparent
pixels count as colour 0, the background), and each pixel takes the
nearest palette colour (squared RGB distance, the lower index on a
tie). [choice]

### 3.4 Things

```toml
[[thing]]
key = "cutlass"
name = "CUTLASS"
type = "melee_weapon"
picture = "cutlass"
weight = 4
value = 30
power = 6
break = 2               # break chance, percent, 0..15
attack = 10             # attack adjustment, a multiple of 5
anyone = true           # usable by anyone
magic = false
once = false            # disappears after use
may_add = 0             # how many ACS may add (15 = many)
```

`type` is one of `treasure`, `magic_item`, `missile_weapon`,
`melee_weapon`, `armor`, `magic_spell`, `portal`, `space`,
`custom_space`, `obstacle`, `custom_obstacle` and `store` (room floors
are the built-in floor and border only). Each takes the model's fields
for its kind (`ThingKind`), by these names; a field its kind doesn't
take is an error:

| Field | Kinds | Values |
| --- | --- | --- |
| `picture` | all but magic spells | a picture key |
| `weight`, `value` | carriable | weight -255..255, or a multiple of 20 to ±5,100; value 0..255 or a multiple of 20 to 5,100 |
| `once` | carriable, space, custom space, obstacle, custom obstacle | disappears after use (when dropped, for a treasure); for the last four, once set off (movement 5.9) |
| `power`, `range`, `break`, `attack`, `anyone`, `magic` | weapons, armor | power 0..31 (missile 0..15); range 2..9, missile only; attack -35..+40 (armor -65..+10), a multiple of 5 |
| `power` | magic spell | 0..127 |
| `spell` | magic item, magic spell, obstacle, custom obstacle, custom space | a spell (3.5) |
| `on_use`, `on_drop`, `on_pickup` | magic item | when it runs |
| `two_way`, `access` | portal | `access`: `"anyone"`, `{ owners_of = KEY }`, `{ non_owners_of = KEY }` or `{ spell = SPELL }` |
| `access` | space | as for a portal |
| `destroy_needed` | portal, custom space | |
| `trigger`, `choice` | custom space | `dropped_here`, `moved_onto`, `owners_of`, `non_owners_of`; `thing` or `spell_parameter` |
| `bump` | obstacle, custom obstacle | the bump runs the spell (5.5) |
| `placed_parameter` | custom obstacle | the placement's parameter replaces the spell's (5.8) |
| `blocked`, `spell_message` | as the kind has the model's two message slots | a short message key, or `{ key = K, shown = false }` to keep it but not show it |

**Numbers** follow the original's lists: carriable things (treasure to
magic spell) take 1..80 in file order, and only 1..32 can be owned up
to three at a time (inventory 1), so list stackable things first; 81 is
gold, 82 the area door (an open two-way portal), the fixtures (portal to
store) 83..126 in file order, 127 the floor and 128 the room border
(both room floors, with the `[floor]` pictures); fixtures past 44 go on
from 129 if `limits.things` allows. Unused numbers are empty slots.

The four property bytes (`Thing::raw`) are written from the fields as
the decoder reads them (`acs_core::encode::thing_props`), so what the
engine reads from them (movement 5.5, 5.8, 5.9) matches the fields. A
picture number past 63 keeps only its low six bits there; the model's
field is the number. A weapon's or magic item's byte 3 (its index in a
table the model doesn't keep) is its order among such things. [choice]

### 3.5 Spells

A spell is a table, `{ action = NAME, ... }`, with the parameter named
for what it means (magic 3):

| `action` | Parameter |
| --- | --- |
| `nothing`, `activate_all` | none |
| `kill_all_but_owner_of`, `give`, `rid_room_of`, `add_to_room` | `thing = KEY` |
| `summon_or_banish` | `creature = KEY`, or none to banish all |
| `increase_magic_defense`, `decrease_magic_defense` | `level =` `none`, `non_magic_half`, `only_magic` or `magic_half` |
| `increase_stat`, `decrease_stat` | `stat =` `constitution`, `strength`, `dexterity`, `speed`, `wisdom`, `dodge`, `parry`, `armor`, `melee` or `missile` |
| `change_power`, `change_life_force` | `by = -63..63`, `temporary = true` or `false` (default) |
| `long_message` | `message = KEY` |
| `play_music` | `music = 1..30` |

`give` must name its thing (parameter 0 gives thing 80, magic 3.9:
name thing 80 instead).

### 3.6 Creatures

```toml
[[creature]]
key = "harrow"
name = "MEG HARROW"
class = "PIRATE"
picture = "meg_harrow"
constitution = 30      # 0..63
strength = 14          # 0..31
dexterity = 15         # 0..31
speed = 8              # 0..15
wisdom = 12            # 0..31
size = 10              # 0..31
power = 12             # 0..127; default: wisdom
life_force = 30        # 0..63; default: constitution
dodge = 20             # skills 0..127: dodge, parry, armor_skill, melee, missile
alignment = "enemy"    # thief | neutral (default) | enemy | friend
aggressive = true
brave = true
magic_defense = "none" # as a spell's level
mimic = false
wealth = 200           # 0..3,100
weapon = "cutlass"     # readied; it must also be carried
armor = "buff_coat"
carries = [{ thing = "cutlass" }, { thing = "pistol_ball", count = 3 }]
```

Fields left out are 0 (or as stated). Masters are numbered from 1 in
file order. A carried thing must be carriable; `count` is 1 to 3 for
things 1..32, 1 above (default 1).

### 3.7 Messages

```toml
[long]
welcome = """THE GOVERNOR'S LETTER..."""

[short]
locked = "THE GATE IS BARRED."
```

Long and short messages are numbered from 1 in file order. Text is
UTF-8 up to 4,000 characters (design.md, "Limits"); the client wraps it.

### 3.8 The world

```toml
name = "THE SPANISH MAIN"
wraps = false
start = [4, 6]
map = """
~~~~~~
~..P.~
~~~~~~
"""

[legend]
"~" = "deep_sea"
"." = "beach"
"P" = "port"

[[terrain]]
key = "deep_sea"
name = "DEEP SEA"
picture = "deep_sea"
pass = "impassable"    # impassable | open | { owners_of = KEY } | { spell = KEY }

[[terrain]]
key = "port"
name = "PORT"
picture = "port_town"
pass = "open"
door = true            # a door terrain (movement 4.4)

[[door]]
at = [3, 1]
to = "port.dock"       # a room, or "map"
square = [1, 2]        # the arrival square, in that room or on the map

[[encounter]]
creature = "reef_shark"
terrain = "deep_sea"
chance = 5             # percent per move, 0..60
```

The map's rows must all be one length; its size is the world's. Door
terrains take numbers 12..15 (at most four), the others 0..11 and then
16 on (movement 4.4 makes 12..15 the doors). Every cell of door terrain
needs a `[[door]]`, and a door must stand on door terrain. Doors are
numbered in file order. `{ spell = KEY }` names a thing whose spell
runs. The four encounter areas keep the model's stock lists and door
table.

### 3.9 Regions

```toml
key = "port"
name = "PORT ROYAL"
store = [{ thing = "cutlass", count = 2 }]
random = ["deckhand", "gull"]     # its random creatures, in order

[[room]]
key = "tavern"
name = "TAVERN"
at = [0, 0]                        # its corner on the region grid
wall = "plank_wall"                # default: [floor] wall
random = { creature = "deckhand", chance = 20 }   # one of the region's; chance 0..75
map = """
#####
#.t.D
#.H.#
#####
"""

[room.legend]
"#" = "wall"
"." = "floor"
"t" = "tavern_table"
"H" = { creature = "harrow" }
"D" = { thing = "wooden_door", to = "port.street" }
```

The map draws the room with its walls; its size is the room's (3..16
a side in Classic). A legend value is `"wall"`, `"floor"`, a thing key,
a table, or a list of thing keys and tables (top first) stacked on one
square. A table is one of:

- `{ thing = KEY, ... }` with what its kind needs: a portal's `to` (a
  room, `region.room`, or `"map"`) and `square` (the arrival square; for
  a room it may be left out when the two rooms share the wall the door
  is in, and then it is the square one step through that wall on the
  region grid); a custom space's or custom obstacle's `parameter`
  (`thing = KEY` for a custom space whose choice is `thing`, else a
  number 0..255).
- `{ thing = KEY, count = N }` for a carriable thing: a take gives
  the placed count (inventory 2.7), 1 by default, up to 3 for things
  1..32. A thing named by key alone is placed with 1, never 0, which
  would be taken off the floor and owned by no one.
- `{ gold = N }`, a gold pile of N (1..25,000).
- `{ creature = KEY }`, a resident with the master's record, standing on
  that square; `things = [...]` lists what lies under it.

Only portals, obstacles and custom obstacles may replace a wall square
(movement 5.2), and a resident must stand inside. Rooms are numbered
from 0 in file order. Portals' door table entries are numbered from 1
(the C64 editor's way), written where they fit the 2-byte format, and
each portal item carries its destination, which is what the engine
reads. Residents take slots from 0 in room order, then row by row; the
rest of `limits.residents_per_region` are free slots for arrivals.

## 4. Starter kits

Four kits (design.md, Kit 4): pirates first, then the three runners-up.
A kit is a library an adventure builds on: pictures, things and
creatures, all under CC BY-SA 4.0, in `kits/NAME/` in the repository.

### 4.1 The kit folder

| File | Holds |
| --- | --- |
| `kit.toml` | Name, licence, credits, the atlas and its picture list (as `pictures.toml`, 3.3) |
| `atlas.png` | The kit's tiles, packed |
| `things.toml`, `creatures.toml` | As an adventure's (3.4, 3.6) |
| `provenance.json` | Each tile's generation record, from the art pipeline's packing step (`docs/art/pirates-style.md` 18) |

`scripts/kitfolder.py PACK.json PACK.png OUTDIR` writes `atlas.png`,
`provenance.json` and the picture list from a packed kit, leaving out
the groups an adventure keeps for itself (`--skip kp`). The things and
creatures are written by hand.

### 4.2 Using a kit

`adventure.toml` names it: `kit = "../../kits/pirates"`, a path from the
adventure's folder. Then:

- A kit entry (picture, thing or creature) is **taken** when its key
  appears as a value anywhere in the adventure's own files, or in a kit
  entry already taken (a creature's weapon, a thing's spell, a picture).
  Nothing else of the kit goes in, so an adventure pays only for what
  it uses: the pirate kit's 202 pictures would pass the limit alone.
  [choice: any string value counts, so a name or text that happens to
  equal a kit key takes that entry too; that costs a slot, nothing else.]
- Taken kit entries come first, in the kit's order, then the
  adventure's own; numbering then runs as in 3.4 and 3.6. An adventure
  entry with a kit entry's key replaces it, in the kit entry's place.
- Pictures from the kit and the adventure's own atlas must share one
  tile size. The bundle's atlas holds only the pictures taken, at their
  picture numbers' places (place n - 1 for picture n), so `pictures.toml`
  is optional when the kit has every picture.

### 4.3 The pirate kit

`kits/pirates`, tiles at 64 px (the art pipeline's working size;
design.md's 128 px is the later target). Its art is made and checked
outside the repository (`ACSVM-research/art/pirates`,
`docs/art/pirates-style.md`); the folder holds the packed result.
Things and creatures are named and rated for this kit alone, never
from the original's builder sets (design.md, "Our own spy"):

- **Weapons and armour** span the ranges in 3.4 from a gutting knife to
  a blunderbuss, so a designer can pace a game by what's for sale.
- **People** come in three bands, deckhand to captain, by Constitution,
  skills and gear; **beasts** have no gear and rely on Size and
  Strength; **the dead** (skeletons, the drowned, ghosts) take
  `only_magic` or `non_magic_half` magic defence, so the kit's few
  magic weapons matter.
- **Doors** are one portal per door picture and wall (`wooden_door_hull`
  and so on), all open and two-way; locks are the adventure's to add
  (an adventure entry with the same key, 4.2).

## 5. Audio

The 30 Play Music cues (client.md 6), composed for this project and made
in code at start-up: no audio file ships, and nothing is taken from the
original's music, whose names and tunes stay unknown to us. The scores
are text in `crates/acs-viewer/src/audio/music.rs` (CC BY-SA 4.0 as
compositions, GPL as code); `synth.rs` renders them.

### 5.1 The set

Effects 1-12 last under a second; themes 13-30 are short pieces of 4 to
12 seconds, except 14-17, which loop until other music plays (magic
3.11) and run 13 to 21 seconds before they repeat. Each cue's feel is
the brief *The King's Pardon*'s design gives it (its "Music" table, the
first adventure to use them all); the cues stay general enough for any
adventure. 14, 15 and 16 are also the endless themes an adventure's
genre picks (`Theme::music_number`).

| # | Cue | | # | Cue |
| --- | --- | --- | --- | --- |
| 1 | A cannon boom | | 16 | Dripping, slow, uneasy (loops) |
| 2 | A ship's bell, rung hard | | 17 | Low and tense (loops) |
| 3 | A pop and a fizz | | 18 | Stately, a little too sweet |
| 4 | A shrill bone whistle | | 19 | Brassy and loud |
| 5 | A splash and a crack of timber | | 20 | Sly, plucked |
| 6 | A wooden crash | | 21 | Two tunes arguing |
| 7 | A heavy lock turning | | 22 | A navy march gone ragged |
| 8 | Coins counted onto a desk | | 23 | Drums and voices, calm |
| 9 | A squawk | | 24 | Underwater, hollow |
| 10 | A steam hiss | | 25 | A single held sting |
| 11 | A soft rising chime | | 26 | Fast, a chase |
| 12 | A low cold drone | | 27 | 18's tune in a minor key |
| 13 | A bright sea-shanty opening | | 28 | 13's tune, slower and free |
| 14 | An easy harbour tune (loops) | | 29 | 18's tune, played alone |
| 15 | Sailing, steady (loops) | | 30 | Short and sad |

A designer picks by feel; the adventure stores only the number.

### 5.2 Scores

A score is a tempo and voices, each an instrument and a line of notes.
A note is a pitch and a length in sixteenths, `c#5:4` (C sharp in
octave 5, a quarter note; a length left out repeats the last one); `r`
is a rest; `|` marks a bar and is ignored. Drum voices write `k` (kick),
`s` (snare) and `h` (hat) for pitches. Every voice of a score is the
same length (a test adds them up), and every note fades out inside its
own length, so a looping theme ends in silence and repeats without a
click.

Instruments: pulse (a quarter-width square, the lead), square, triangle
(bass and soft leads), sine (pads and low bass), bell (two partials,
decaying) and the three drums (a falling sine, two noises). Effects that
aren't tunes are sweeps (a pitch gliding between two frequencies) and
noise bursts (bright to dull), each with its own length.

## 6. Sample adventures

- **A tutorial**, *First Voyage* (`adventures/first-voyage`), small,
  on the pirate kit, teaching movement, rooms, things, a store, a fight
  and a spell in about fifteen minutes: notices on the dock and at the
  trader explain each step, a shore crab of Speed 0 guards the cave (it
  strikes back but never chases, so the fight is where the player
  chooses it), a logbook gives the sea chant, and a chest of coin on a
  success stack (winning 1.2) ends it.
- ***The King's Pardon***, mid-size, on the pirate kit. Its story,
  design, script and **buildable source** are their own project
  (`D:\Projects\KingsPardon`), kept private for now (the maintainer's
  call, November 2026): built with `acs build` against `kits/pirates`,
  its bundle and walkthrough stay out of this repository. Whether it
  ships with 1.0, and so the gate's "both sample adventures" (1), is
  open until it is published.

*First Voyage* is under CC BY-SA 4.0 and a test fixture; *The King's
Pardon* will be too if it is published.

## 7. The front door

The viewer opens on a **title screen** when no adventure is open: the
project's name, the sample adventures to play, and the way to open the
player's own files (a drop or Open..., original disks and bundles
alike, read on the machine and never uploaded).

- **Samples are built in.** The viewer's `build.rs` builds each sample's
  source folder (`adventures/NAME`, with its kit) with `acs-build` and
  embeds the bundle (*First Voyage*: about 60 KB, against a 5 MB wasm
  file), so the browser and the desktop builds carry the same samples
  with nothing to fetch or package. A sample that doesn't build fails
  the viewer's build. [choice]
- **Play** opens the sample in walk mode, ready for Add Someone; it is
  an ordinary bundle from there (saves, the Look menu, the lists).
- Only our own adventures are built in (CC BY-SA 4.0); nothing of EA's
  is (client.md 8).

## 8. Order of work

1. The bundle (2). Done November 2026: `crates/acs-bundle` reads,
   checks and writes bundles (`tests/bundle.rs`); every `acs` command
   and the viewer open them (a drop or the picker, by extension or zip
   signature); the viewer's Look menu gains **Native HD**, the default
   for a bundle with art (`--render native`). `acs bundle MODEL.json -o
   OUT.advx [--art ATLAS.png --tile N]` wraps an Extended model, place N
   of the atlas holding picture N: a stopgap until `acs build`.
2. The source format and `acs build` (3). Done November 2026:
   `crates/acs-build` (`tests/build.rs` builds a two-room adventure
   written in the test, loads its bundle and walks it in the engine),
   `acs build DIR -o OUT.advx`, and `acs_core::encode` for the property
   bytes, checked against every corpus thing (the corpus test
   `thing_props_encode_back`).
3. The pirate kit as data (4), and a test adventure built from it.
   Done November 2026: `kits/pirates` (202 pictures, 123 things, 53
   creatures; `scripts/kitfolder.py`, `scripts/kitcreatures.py`), kits
   in `acs build` (4.2), and `crates/acs-build/tests/kit.rs` (only what
   is used goes in; an entry replaced; the whole kit builds; a kit
   adventure plays). Seen in the viewer: transparent HD art needs what
   it stands on drawn under it, so room squares are drawn as layers
   (`acs_core::scene::layers`: wall or floor, top thing, resident). Left
   for the art side: the store tile's background is opaque white.
4. The tutorial (6), with its replay test. Done November 2026:
   `adventures/first-voyage` (two regions, four rooms, built on the
   pirate kit), its `walkthrough.txt`, and the test
   `first_voyage_plays_to_the_end` (crates/acs-cli), which builds it and
   plays it to the congratulations on eight seeds. Found on the way: a
   carriable placed with parameter 0 gives nothing when taken (inventory
   2.7), so the build places them with 1 (3.9).
5. The 30 cues (5). Done November 2026: `audio/music.rs` (scores,
   checked to parse and to agree bar for bar), `audio/synth.rs`
   (instruments, sweeps, noise), after *The King's Pardon*'s music
   brief (5.1; a first set ignored it and was redone). Not yet heard by
   a person:
   `ACSVM_CUES_DIR=DIR cargo test -p acs-viewer write_cues_as_wav --
   --ignored` writes them as WAV files.
6. *The King's Pardon* (6), with its replay test, in its own private
   project; its walkthrough test runs there, not in this repository's CI.
7. The front door (7). Done November 2026: `crates/acs-viewer/build.rs`
   and `samples.rs`, the title screen in `app.rs`.
8. The other three kits (4).
9. The playtest, then 1.0 (1).
