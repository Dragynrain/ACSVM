# ACS format notes

Findings about the adventure file format beyond (or against) Dan Boris's draft
v0.5 and ACS Viewer's AcsLib. Licensed CC BY 4.0.

Evidence comes from decoding the 14 C64 Jadeworld adventures (private corpus,
see `provenance.md`): 210 regions, 1,670 master creatures, 3,224 placed
residents, 15,092 room items, 2,624 room doors. Counts refer to that corpus
unless marked **PC**: those come from one real PC file, EA's *Rivers of
Light* (`ACS.FPY` from the MS-DOS release; 12 regions, 149 rooms, 3,083 room
items, 382 doors).

The code is the precise reference: `crates/acs-core/src/layout.rs` holds every
offset, `decode.rs` every field.

## Platforms at a glance

The record layouts (messages, creatures, things, header, world map, regions)
are the same on every platform; only block positions and pictures differ.
C64 and PC are confirmed against real files; Apple II and Amiga come from
AcsLib and are untested (see below).

| | C64 | Apple II | MS-DOS | Amiga |
| --- | --- | --- | --- | --- |
| Released | 1984 (one listing: Jan 1985) | 1985 | 1987 | 1986 |
| Port | Original (Stuart Smith) | ? | Jerry Boutelle (one listing) | Glenn Tenney, Fantasia Systems |
| Container | Whole `.d64` (174,848 bytes), raw sectors | Whole `.dsk` (143,360 bytes), no directory | `ADVEN.FPY` / `.HRD` file (`ACS.FPY` is 229,376 bytes) | `Adventure` file in an `.adf` |
| Pictures | 110 | 110 | **96** | 110 |
| Picture size | 8 x 16, double-wide pixels | **14 x 16** hi-res | 16 x 16 | 16 x 16 |
| Bits per pixel | 2 | 1 (artifact colour) | 2 | **5** (bitplanes) |
| Colours per adventure | 4 of 16; colour 3 only from the first 8 | 4 or 6 artifact colours | 4 of 16 (CGA/EGA values) | **32 of 4,096**, shared by all pictures |
| Creature-only pictures | 64-109 | probably the same | unknown (96 total) | probably the same |
| Extras | | | | *Galactic Agent* (Ken St. Andre) |
| Status here | Decoded, 14 adventures | AcsLib only | Decoded, 1 adventure | AcsLib only |

The PC display mode is unconfirmed: a MobyGames caption says CGA, but CGA's
320 x 200 mode can't show any 4 of 16 colours, so how the PC shows a palette
chosen from 16 is open.

Limits shared by every platform: world map 40 x 40 with 16 terrain types,
32 map portals, 15 regions of up to 16 rooms, rooms up to 15 x 10, 16
resident and 8 random creatures per region, 128 things, 128 creatures, 8
creature classes, 255 long messages (8 x 32 characters, 255 is the intro),
80 short-message slots (3 x 40), 20-character region and world names. Room
items are a per-region list with 4-bit coordinates.

Sources: dates, the Amiga's 32 colours and *Galactic Agent* from
[Wikipedia](https://en.wikipedia.org/wiki/Adventure_Construction_Set); port
credits from [Pixelated Arcade](https://pixelatedarcade.com/games/stuart-smith-s-adventure-construction-set/releases)
and the [Amiga reference card](https://www.mocagh.org/ea/acsuk-refcard.pdf).

## Platform layout

| Block | PC | C64 | Note |
| --- | --- | --- | --- |
| Long messages 1-249 | 0x06C00 | 0x06C00 | **No shift on C64** (the draft says +0x200 for everything) |
| Long messages 250-255 | 0x16500 | 0x16700 | |
| Intro text | 0x16A00 | 0x16C00 | Long message 255 *is* the intro text |
| Short messages | 0x16B00 | 0x16D00 | |
| Creatures | 0x19300 | 0x19500 | |
| Things | 0x1B800 | 0x1BA00 | |
| Adventure header | 0x1C200 | 0x1C400 | |
| World map | 0x1C700 | 0x1C900 | |
| Regions (0xC00 each) | 0x1D300 | 0x1D500 | |
| Pictures | 0x28720 | 0x28920 | |
| Palette (4 bytes) | 0x2A802 | 0x29880 | **Not** PC + 0x200 |

The PC adventure in the MS-DOS release is `ACS.FPY` (229,376 bytes), next to
the program files; it decodes with these PC offsets.

**The 0x200 "header" is track 18.** A C64 adventure is written raw across the
disk in sector order and simply skips track 18 sectors 0 and 1 (BAM and the
first directory sector), which start at image offset 0x16500. So data before
0x16500 is at the PC offset and data after it is shifted by 0x200. Long
messages 1-249 end exactly at 0x16500, so AcsLib's split at message 249 is
this gap. This is why disk tools see no files on an adventure disk.

**An adventure disk is more than the adventure blocks.** The C64 player
refuses a disk that holds only the blocks this project decodes ("if you have
not made an adventure disk yet..."). It accepts the same blocks written over
a blank adventure disk made by the program's own "make an adventure disk"
step. 64,753 bytes are identical on all 14 corpus adventure disks, most of
tracks 1-5 among them; what the check looks at is unknown, and the capture
harness doesn't need to know (docs/capture.md 1.4).

## Text

- Single-byte text stores `@`, `A`-`Z` and `[\]^_` as 0-31 (C64 screen-code
  order) and other characters as ASCII; decode bytes below 32 as `byte + 64`.
  Letters in the corpus are almost always stored as 1-26. No referenced
  message contains a byte of 0x80 or above; unused short-message slots do.
- Long messages and the intro are 8 lines of 32 characters; short messages are
  3 lines of 40. Words run straight across line ends, so lines must be split
  by position, not by spaces.
- Two short messages share each 256-byte page (at +0 and +0x78). 80 slots fit
  before the creature block. Things refer to slots 1-127 (0 = none); slots
  above 80 would run into the creature data and are flagged.
- **Message reference bit 7 means "hidden"**, as in AcsLib (the draft says the
  opposite). In the corpus 655 references have it clear and 58 set; designers
  rarely hide messages, so clear = shown fits.
- The world map name is **20 bytes** at header +0x22D. AcsLib reads 22, which
  takes 2 bytes of the first region name; that only goes unnoticed because
  most names are space-padded.

## Pictures

**C64 pictures are multicolor characters.** Each picture is 2 x 2 characters
(8 x 16 multicolor pixels, each pixel twice as wide as tall), stored as:

- top-left character: 8 bytes at `pictures + n*16`
- top-right character: 8 bytes at `+8`
- bottom-left and bottom-right: the same, `+0x800` further on

Each byte is one row of 4 pixels, most significant bit pair first. 110
pictures (0-109); pictures 64-109 are creatures, and 1,636 of the 1,670 master
creatures use one of them.

**C64 palette (0x29880).** The four bytes are the colours for bit pairs 00,
01, 10, 11 in that order: the three shared background registers, then colour
RAM. In multicolor character mode colour-RAM bit 3 is the "this character is
multicolor" flag and only bits 0-2 select the colour. Every corpus adventure
stores colour 3 as 0x0B (flag + cyan), which is why AcsLib subtracts 8. The
decoder masks with 7 and warns if the flag is ever clear. So colour 3 is
limited to the first 8 C64 colours, while colours 0-2 may be any of 16.

PC pictures: 16 x 16, four 4-pixel columns at +0, +0x2400, +8, +0x2408, bottom
half at +0x800, as in AcsLib (the draft lists the columns in the reverse
order). **PC**: confirmed; all 96 pictures of *Rivers of Light* come out clean
with the palette at 0x2A802.

## Apple II and Amiga (from AcsLib, unverified)

Neither platform is decoded yet and no Apple II or Amiga file has been tested.
Everything here is what AcsLib (`GameLoader.cs`) does, restated as facts for
the port; treat it as a starting point to confirm against real files.

**Block bases.** The records (messages, creatures, things, header, world map,
regions) have the same layout as on the PC; only the bases move. Offsets are
into the file AcsLib loads: the whole `.dsk` image for the Apple II (sector
order not stated), the `Adventure` file extracted from the `.adf` for the Amiga.

| Block | PC | Apple II (PC - 0x6700) | Amiga |
| --- | --- | --- | --- |
| Long messages | 0x06C00 | 0x00500 | 0x18100 (PC + 0x11500) |
| Intro text | 0x16A00 | 0x10300 | 0x27F00 (PC + 0x11500) |
| Short messages | 0x16B00 | 0x10400 | 0x15900 (PC - 0x1200) |
| Creatures | 0x19300 | 0x12C00 | 0x06600 (PC - 0x12D00) |
| Things | 0x1B800 | 0x15100 | 0x08B00 (PC - 0x12D00) |
| Adventure header | 0x1C200 | 0x15B00 | 0x09500 (PC - 0x12D00) |
| World map | 0x1C700 | 0x16000 | 0x09900 (PC - 0x12E00) |
| Regions | 0x1D300 | 0x16C00 | 0x0A500 (PC - 0x12E00) |
| Pictures | 0x28720 | 0x22040 | bitplanes, see below |
| Palette | 0x2A802 | 0x22FF8-9 | 0x00000 |

So the Amiga file is reordered, not just shifted: palette and pictures first,
then creatures, things, header, world map and regions, then short messages,
then long messages and the intro at the end. Whether long messages 250-255
split off as on the C64 is unknown for both platforms.

**Amiga pictures: 110 pictures, 16 x 16 square pixels, 32 colours.**

- Palette at file offset 0: 32 colours of 2 bytes each in the Amiga `0x0RGB`
  form (byte 0 low nibble = red, byte 1 high nibble = green, low nibble =
  blue). Each nibble scales to 8 bits as `n * 0x11`. So the palette is chosen
  per adventure from 4,096 colours, and all 110 pictures share it. Colour 0 is
  the background.
- Five bitplanes at 0x40, 0xE00, 0x1BC0, 0x2980, 0x3740 (0xDC0 apart). Plane
  `j` gives bit `j` of each pixel's colour index; within a byte bit 7 is the
  leftmost pixel.
- Each plane holds two strips of 55 pictures side by side: 16 rows of 110
  bytes (2 bytes per picture per row). Picture `n` row `y` is at
  `plane + n*2 + y*110` for n < 55, and `plane + 0x6E0 + (n-55)*2 + y*110`
  for n >= 55 (AcsLib writes this as `+ 1650 + n*2`).

**Apple II pictures: 110 pictures, 14 x 16, hi-res.** 32 bytes per picture
at `0x22040 + n*32`: rows 0-7 take one byte from +0..7 and one from +8..15,
rows 8-15 from +16..23 and +24..31 (7 pixels per byte, the Apple hi-res
layout). Colours are the hi-res artifact colours (black, green, violet, white,
orange, blue). When byte 0x22FF9 is 0x7F the adventure uses 4 colours and
bits 5-7 of 0x22FF8 pick the green/violet or orange/blue pair; otherwise it
uses all 6. The exact pixel-to-colour rule is in AcsLib's `RenderBytesApple`
and still needs writing up.

**Effect on the model.** `Picture` stores one hex digit per pixel and
`PaletteColor.code` indexes a 16-colour master palette. Amiga needs colour
indices up to 31 and palette entries that are plain RGB with no master code.
Apple II needs 14-pixel-wide pictures and its artifact-colour rule. Both
change what the model means, so they need a `MODEL_VERSION` bump.

## Regions

The region header is more structured than either source says:

| Offset | Meaning |
| --- | --- |
| 0x00 | Number of **live** rooms |
| 0x01-02 | End of the room item lists, relative to the region start |
| 0x03-04 | Usually 0x00F0 (the door table's heap offset); **not reliable**, see below |
| 0x05-06 | Heap offset of the 16 resident creature names |
| 0x07-08 | Heap offset of the room item lists |

"Heap" is region + 0x3F3: 16 room names (0xA0 bytes), then 8 random creature
names (0x50), then the door table (variable), resident names (0xA0) and the
room item lists. In all 210 C64 regions and 12 PC regions the names end
exactly where the items begin and the end pointer matches.

**The door table always starts at heap + 0xF0** and runs up to the resident
names. Header bytes 3-4 hold 0x00F0 in all 210 C64 regions and 9 of 12 PC
regions, but **PC** regions 1-3 of *Rivers of Light* hold 0x3B, 0xB8 and 0xE6
there. Reading the table from those values misaligns every door (the first
decoder did, sending doors to world map points that do not exist). Reading
from 0xF0 gives exactly one entry per door in all three regions, and 270 of
275 two-way doors then land next to a door leading back. What bytes 3-4 mean
when they are not 0xF0 is open; the model keeps them as `unknown_header_03`.

- AcsLib finds resident names at `region + 0x453 + (low byte of the item
  pointer)`. That equals `heap + item pointer - 0xA0` only while the pointer
  is between 0x100 and 0x1FF. The decoder follows the header pointers instead,
  and a synthetic test pushes them past 0x200.
- **Room count byte = live rooms, not a slot limit.** Deleted rooms keep
  their slot with a zero size, and later slots stay live. JADE-WORLD region 7
  has count 14 with slots 14 and 15 in use, and doors into them. The decoder
  keeps all 16 slots and marks deleted ones; in all 210 regions the live count
  equals the byte.
- There are always **16 room item lists** (2-byte length each, including
  itself), even for deleted or unused slots.
- Store stock (14 bytes at +0x6D) and creature possessions (14 bytes at record
  +0x17) share one bitmap: things 1-32 get 2-bit counts (four per byte, high
  bits first), things 33-80 one bit each.

### Room items

`x << 4 | y`, thing number (bits 0-6), parameter. 0xFF in the first byte marks
a deleted item. All 110 items that fall outside their room sit at exactly
square (15,14), off the edge of any room, so this looks like a stash square for
things kept out of reach. The 9 items with bit 7 of the thing byte set are all
at (15,14) as well. Meaning of the bit still unknown; it is kept as `flag`.
The stash stays in the room's item list for now: Rid Room currently
removes stash copies too, and moving them out would change Classic play
before anyone knows what the original does. Extended rooms larger than
15 x 14 will need the stash moved to its own list first.

### Doors (the portal table)

- **Room doors**: a portal item's parameter indexes the region's door table
  (2 bytes per entry). C64 adventures leave entry 0 unused and number doors
  from 1 (184 of 210 regions use exactly 1..n; the rest have a few free
  entries left by deleted doors). **PC** *Rivers of Light* numbers its doors
  from 0 and uses entry 0. Byte 0 = `region << 4 | room`, byte 1 =
  `x << 4 | y` in the destination room. Region 0 means the world map: then
  `y | x << 4` indexes the 64-entry world map point table. Over 2,624 C64 room
  doors, every destination region and room slot exists and the links come in
  matching pairs. One door leads to a deleted room, which the
  manual describes as legal (players are sent back to the start).
- **World map points** (world map + 0x20, 64 x/y pairs, 0xFF = unused): points
  0-31 are door locations on the map; points 32-63 are the extra exit points.
  For map door *k*, `+0xAA + 2k` holds the same 2-byte destination format; if
  its region is 0, the destination is point 32 + *k*.
- Bit 7 of a door's stored y coordinate is set on 7 of 28 doors; meaning
  unknown (kept as `unknown_y_bit7`). AcsLib masks it off.

## Drawing rooms

How a room looks on screen, worked out for the viewer from all 15 corpus
adventures (14 C64 plus **PC** *Rivers of Light*; 1,405 live rooms). The code
is `crates/acs-core/src/scene.rs`; `tests/scene.rs` covers it with synthetic
data and `tests/corpus.rs` re-checks the counted rules on the corpus.

- **Walls and floor.** A room's size includes its walls: the squares on its
  edge show the room's wall picture (room byte 0, a picture number) and the
  inside shows the floor.
- **The floor is master thing 127.** In every corpus adventure thing 127 is a
  room-floor thing named EMPTY FLOOR (picture 1) and thing 128 a room-floor
  thing usually named ROOM BORDER. Thing 128's picture is the most common wall
  picture in 11 of 15 adventures, so it looks like the editor's default wall
  that a room starts with. Room-floor things are never placed as room items
  (0 of 18,226 items in live rooms). Picture 1 is blank (all colour 0) in all
  15 adventures, so floors are plain background in practice; the viewer still
  draws thing 127's picture in case an adventure changed it.
- **Stacks: the first item is on top.** Items on the same square are listed
  top first. Of 22 items named as covers ("COVR", "HIDDEN", "SECRET") in a
  stack, 19 come first and the other 3 come straight after other covers.
  The original shows the top item's picture; a thing without a picture (a
  magic spell) lets the next one show. Residents stand above everything.
- **Things in walls.** Items on edge squares replace the wall there: 2,918
  portals, 520 obstacles and 467 custom obstacles; no other type ever sits on
  a wall. 116 portals are inside rooms.
- **Region grid.** Room bytes 2-3 place the room on a region grid measured in
  squares; the furthest corners reach x 80 and y 40. Neighbouring rooms share
  a wall line: 1,300 pairs of rooms overlap, every one by exactly one line.
  2,250 of 2,580 doors within a region lead to the square next to the door on
  that grid, i.e. straight through the shared wall; the other 330 jump
  elsewhere. When two rooms share a wall, a door drawn in one room's copy of
  the wall must win over the other room's plain wall.

## Things

- **Weight multiplier: bits 5-6 = ×1, ×-1, ×20, ×-20** (bit 5 = negative,
  bit 6 = ×20) for every thing type, as in AcsLib. The draft's alternative
  order for treasures and magic items gives a BOULDER weight of -100 and an
  ICE BLOCK -125, against 2,000 and 2,500 with AcsLib's order, while things
  named for lightness (LIGHTFOOT, HOVERING PACE) come out negative only with
  AcsLib's order.
- Picture fields are 6 bits (0-63); things never use creature pictures.
- **Change Power / Change Life Force parameter: bits 0-5 the amount, bit 7
  negative, bit 6 temporary**, as the draft says. Confirmed on the C64
  (docs/capture.md section 7, experiment `drift`): Life Force +10 with bit
  6 set wore off, without it stayed.
- **Thing 81 is Gold** in all 15 adventures: a treasure of value 1 (weight 1
  or 4), the only carriable thing above 80. Every other carriable thing is
  1-80, matching the possession bitmap (2-bit counts for 1-32, one bit for
  33-80); creatures keep gold as the wealth count, at most 3,100.
- **Gold pile size**: the room item parameter of a placed Gold is most
  likely the pile's size: 117 piles hold 0-51 (37 are 0). Other carriable
  placements have parameter 1 (1,228) or 0 (93), with a handful of stray
  values; their meaning is unknown and the engine ignores it.
  But on the C64 (capture.md 10.14.4) piles our disks placed with
  parameter 5 and 10 were picked up by creatures as gold worth 512 and
  1,361, every time: the parameter is not simply the amount, or our
  disks place piles differently from the editor. Open: a capture of
  piles with parameters 0 to 51, read off the take message.
- No thing in the corpus has type 13 or 14; 27 slots are 15 (empty).
- **PC**: thing 80 of *Rivers of Light*, SWIMMING SKILL, is a magic spell with
  action code 15 (outside 0-14) and power 0. It is never placed; three
  creatures carry it, and seven river terrains plus a "swimmable water" space
  are open only to its owners. So a spell slot with code 15 serves as an
  uncastable key item. The decoder keeps it as `invalid(15)` with a warning.
- Theme byte (header +0x36E): 0, 1 or 2 in the corpus. Following AcsLib's
  music list it selects Endless Fantasy, Spy/Mystery or Sci-Fi. Unverified.

## Creatures

- **Bytes 0x0B and 0x0C are zero in every record**: 1,670 master, 3,224
  residents, 394 random, 43 world map. They are most likely runtime state that
  the editor never sets (players were also all unused, so saved games could
  not be checked). Kept raw as `unknown_0b` / `unknown_0c`.
- **Byte 0x10 = region << 4 | room**, confirmed: it is 0 in the master, random
  and world map lists, and in all 3,224 residents the high nibble equals the
  region that holds the record.
- **Mimic is bit 7 of byte 0x09** (13 creatures), as in AcsLib. The draft's
  0x0A bit 7 is never set.
- Picture byte 0x16 = picture + 2 (0 = unused slot). Values run 3-111, so 111
  maps to picture 109, the last one; no other offset fits that bound.
- Readied weapon (0x14) and armor (0x15) are thing numbers. Every non-zero one
  in the corpus points at a weapon or armor of the right kind. All possessions
  point at carriable things.

- **Retired players, first two slots confirmed by capture** (C64, November 2026;
  docs/capture.md section 8). Retiring a player on a scenario disk wrote
  retired name 1 at creatures + 0x1700 and retired record 1 at
  creatures + 0x1900, each a copy of player slot 1 in the adventure block
  (record at + 0x2B, name at + 0xBF), the record keeping the player's
  region, room and square. A record we wrote there ourselves, with no other
  change to the disk, was offered at start-up and played with its stats.
  Choosing the player clears the name's second byte (the first is already
  0 for a one-letter name), so a non-zero name marks the slot as used.
  The second slot follows the decoder's layout too: a name at
  creatures + 0x170A and a record at + 0x1925, written by us next to a
  first player, were offered (listed right of the first, under "don't add
  a player") and played with every stat as written (docs/capture.md
  10.8.6). Slots past the second, and the page break the decoder assumes
  after 25 names and 6 records, are unchecked. Every stat byte reads as the
  decoder says (a record with a different value in each, checked on the
  profile screen), except that a player joining play gets Power = Wisdom,
  so byte 5 of a retired record has no visible effect.

## Written by the program in play

Seen by diffing scenario disks (C64), meaning unknown:

- Adventure block + 0x400..0x401: written when play starts.
- Adventure block + 0xE7: 0 to 7 when a player retired.
- Adventure block + 0x36F..0x3FF (145 bytes, 0x08 fill in a blank disk):
  rewritten with what looks like multicolour bitmap data when a player
  retired. Not needed for the retired player to load.

## Open questions

- Door y bit 7 (7 of 28 doors) and room item thing-byte bit 7 (9 items, all
  at the stash square).
- Region bytes 0x59-0x6C: zero in all 210 C64 regions and 11 of 12 PC
  regions. **PC** region 1 holds "EARTH VALLEY" in screen codes there, an
  older name of that region, so possibly an editor buffer.
- Creature bytes 0x0B, 0x0C: need a saved game or an emulator capture.
- Retired players: AcsLib reads 26 names and 7 records per 256-byte page, which
  overruns the page; the decoder reads 25 and 6. No corpus adventure has any;
  the first two slots are confirmed (see Creatures).
- PC: only one real file so far, made by EA and possibly with an earlier
  editor (it also contains a room named "I DONT KNOW YET"). Adventures saved
  by the shipped PC editor may differ, e.g. in door numbering.
- Music and sound data: still unlocated (expected in the program, not the
  adventure).
