# ACS format notes

Findings about the adventure file format beyond (or against) Dan Boris's draft
v0.5 and ACS Viewer's AcsLib. Licensed CC BY 4.0.

Evidence comes from decoding the 14 C64 Jadeworld adventures (private corpus,
see `provenance.md`): 210 regions, 1,670 master creatures, 3,224 placed
residents, 15,092 room items, 2,624 room doors. Counts below refer to that
corpus. **PC findings are inferred from the shared layout and have not been
checked against a real PC file.**

The code is the precise reference: `crates/acs-core/src/layout.rs` holds every
offset, `decode.rs` every field.

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

**The 0x200 "header" is track 18.** A C64 adventure is written raw across the
disk in sector order and simply skips track 18 sectors 0 and 1 (BAM and the
first directory sector), which start at image offset 0x16500. So data before
0x16500 is at the PC offset and data after it is shifted by 0x200. Long
messages 1-249 end exactly at 0x16500, so AcsLib's split at message 249 is
this gap. This is why disk tools see no files on an adventure disk.

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
order). Untested against real files.

## Regions

The region header is more structured than either source says:

| Offset | Meaning |
| --- | --- |
| 0x00 | Number of **live** rooms |
| 0x01-02 | End of the room item lists, relative to the region start |
| 0x03-04 | Heap offset of the door (portal) table |
| 0x05-06 | Heap offset of the 16 resident creature names |
| 0x07-08 | Heap offset of the room item lists |

"Heap" is region + 0x3F3: 16 room names (0xA0 bytes), then 8 random creature
names (0x50), then the door table (variable), resident names (0xA0) and the
room item lists. In all 210 regions the door table starts at heap + 0xF0, the
names end exactly where the items begin, and the end pointer matches.

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

### Doors (the portal table)

- **Room doors**: a portal item's parameter indexes the region's door table
  (2 bytes per entry; entry 0 unused). Byte 0 = `region << 4 | room`, byte 1 =
  `x << 4 | y` in the destination room. Region 0 means the world map: then
  `y | x << 4` indexes the 64-entry world map point table. Over 2,624 room
  doors, every destination region and room slot exists and the links come in
  matching pairs. One door (THE BLOOD SWORD) leads to a deleted room, which the
  manual describes as legal (players are sent back to the start).
- **World map points** (world map + 0x20, 64 x/y pairs, 0xFF = unused): points
  0-31 are door locations on the map; points 32-63 are the extra exit points.
  For map door *k*, `+0xAA + 2k` holds the same 2-byte destination format; if
  its region is 0, the destination is point 32 + *k*.
- Bit 7 of a door's stored y coordinate is set on 7 of 28 doors; meaning
  unknown (kept as `unknown_y_bit7`). AcsLib masks it off.

## Things

- **Weight multiplier: bits 5-6 = ×1, ×-1, ×20, ×-20** (bit 5 = negative,
  bit 6 = ×20) for every thing type, as in AcsLib. The draft's alternative
  order for treasures and magic items gives a BOULDER weight of -100 and an
  ICE BLOCK -125, against 2,000 and 2,500 with AcsLib's order, while things
  named for lightness (LIGHTFOOT, HOVERING PACE) come out negative only with
  AcsLib's order.
- Picture fields are 6 bits (0-63); things never use creature pictures.
- No thing in the corpus has type 13 or 14; 27 slots are 15 (empty).
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

## Open questions

- Door y bit 7 (7 of 28 doors) and room item thing-byte bit 7 (9 items, all
  at the stash square).
- Region bytes 0x59-0x6C: zero in all 210 regions.
- Creature bytes 0x0B, 0x0C: need a saved game or an emulator capture.
- Retired players: AcsLib reads 26 names and 7 records per 256-byte page, which
  overruns the page; the decoder reads 25 and 6. No corpus adventure has any.
- PC: everything is untested against real files, including the picture column
  order and the palette address.
- Music and sound data: still unlocated (expected in the program, not the
  adventure).
