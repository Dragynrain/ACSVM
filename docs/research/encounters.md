# Research brief: world map encounters

A request to the project's clean-room researcher (docs/research/README.md).
Licensed CC BY 4.0. Status: briefed 2026-11-14, with its inputs
(`inputs/`); first pass done 2026-11-14; in the spec and the engine.

## Why

Every adventure's world map has up to 8 random creatures, each tied to a
terrain with a chance of 0 to 60% (the data: `docs/format-notes.md`, the
decoder's `WorldCreature`). The manual says only that before a move on
the map one of them may appear, and that the player is then pulled into
an encounter area bordered by the terrain being crossed. Everything else
is open in our specs (`docs/rules/movement.md` 7.1 and 9 item 4,
`docs/rules/creatures.md` 1.3): when the roll is made, which terrain
counts, what the area looks like, who is placed where, how the turns go
inside it and how the player gets back to the map. The engine rolls and
emits an `Encounter` event, then carries on as if nothing appeared.

The roll is random and the area's rules hang on several cases at once
(README section 2), so this goes to research with the captures below to
check against. The random-rolls third pass (item 17) already found that
each walked map square rolls for each of the map's 8 records on its
terrain; this pass is the rest.

## What we ask

For the C64 version, in plain language:

1. **The roll.** When it is made in a map move (before or after the step;
   on a refused move, a move onto a map door, the last move of a turn),
   which terrain it goes by, how the chance code becomes a percentage
   (the decoder reads codes 0 to 4 as 0 to 8% in steps of 2 and 5 to 15 as
   10 to 60% in steps of 5: confirm or correct), how several creatures for
   the same terrain are taken (the order, one roll each, the first that
   hits, or something else), and whether anything else comes in (the
   player's stats, other players, the region, a cool-down after an
   encounter). Please say where the rolls are drawn, so the harness can
   save a snapshot just before and sweep the random state as it did for
   casting (capture.md 9, 10.9). Captured: in `patch`, the first push east
   from the field (terrain 0, no creature of its own) onto the grass
   (terrain 5, WOLF at 60%) brought the encounter area at once, bordered
   with the grass: so the terrain entered counts, or the roll comes after
   the step. Our engine rolls on the terrain stood on before the move.
   In `field` (WOLF at 60% on the field itself) the first push east was an
   ordinary step and the area showed by the second.
2. **The encounter area.** Its size and layout (the border of the
   terrain, the openings in it: in `patch-step-1.png` the left side has a
   gap), what fills the inside, where the player is placed (both captures
   put it on the right, after a push east), where the creature is placed,
   how many creatures come (one, or a number from its record or a roll),
   and whether the creature is the record as stored or has values rolled
   or changed (its Life Force, Power, possessions, gold).
3. **Play inside the area.** Whose turns run there and in what order (is
   it a region of the turn scheduler, turn-order.md, with its own visit,
   or part of the map's?), whether the area's creature behaves as a room
   creature does (its goals and steps, creatures 6), and what the other
   players on the map do meanwhile. Can a second player be pulled into
   the same area, or another one?
4. **Leaving.** How the player gets out (walking off through an opening,
   the creature dying or leaving, anything else), where on the map it
   comes back (the square it left, the square it was stepping onto), with
   how many moves, and what becomes of the creature (gone, left behind,
   following). What happens when the player dies in the area, and when
   it asks to save or retire there.
5. **What the area changes.** Anything the area does to the map or the
   save (capture.md 4: entering or leaving a region saves the game), and
   whether a random creature of the map can drop things, keep gold or
   carry anything back.

Enough for a first pass: 1 and 2, and how the area ends in 4. Item 1 is
a random rule: name what decides each roll's range (a fixed 100, a count
of creatures) and its draw order. For the area, say which state the
harness should read to see it (that a player is in an area, the
creature's slot and square, the area's size), if capture.md 9 doesn't
already give it.

## How the answers come back

As docs/research/README.md 3 and 4: `findings.md` in plain language only.
Addresses only of state the harness must set or read to check the
answers: the moment before the roll, the in-an-area state, the area
creature's record and square.

## Inputs

In `ACSVM-research/research/encounters/inputs/` (copies). VICE 3.10
`x64sc` snapshots; start VICE with `-controlport2device 37` to drive the
joystick as `docs/capture.md` 2.2 describes. Each scenario is our
move-basic field (capture.md 7: the open map cells 6..34 by 6..18 walled
in, markers at 15,14, 27,14 and 9,14, the map door at 12,16) with our
retired player T (Speed 6, holding our blade) starting on 12,12, and map
random creatures of ours (capture.md 10.19); every other map random
creature slot is empty. WOLF is an enemy, Speed 4, holding our blade
(power 5), drawn as a 12 x 12 box; T is the 16 x 16 square.

| File | What it is |
| --- | --- |
| `field-disk.d64`, `field-start.vsf`, `field-start.png` | WOLF on terrain 0 (the field), chance code 15 (60%); at T's first turn |
| `field-walk-1.png` .. `-5.png` | From `field-start.vsf`, a push east (300k instructions) then 40M instructions, five times (`walk5.txt`): an ordinary step, then the area (no border: the field's picture is blank) |
| `patch-disk.d64`, `patch-start.vsf`, `patch-start.png` | A patch of GRASS (terrain 5, a dotted picture) on map cells 13..16 by 10..12, just north-east of T; WOLF on the grass at 60% |
| `patch-step-1.png`, `patch-step-4.png` | From `patch-start.vsf`, one push east onto 13,12 (`walk.txt`): 10M and 200M instructions after; the area bordered with grass, WOLF moving |
| `two-disk.d64`, `two-start.vsf`, `two-start.png` | The grass patch with two creatures on it, both at 60%: WOLF (slot 0) and HARE (slot 1, a peaceful friend, Speed 3, drawn as a solid square like T): for item 1's several creatures |
| `build.sh`, `walk.txt`, `walk5.txt` | How the disks, snapshots and walks were made (acs-capture `scenario` and `script`) |

## Findings

First pass, 2026-11-14 (`ACSVM-research/research/encounters/findings.md`),
items 1, 2 and 4, with parts of 3 and 5; about 15 runs in VICE from the
snapshots above, several with the random draw forced. Written into
`docs/rules/encounters.md` (movement 7.1 points there); the state to read
is in capture.md 9.

- **The roll** follows each completed step on the open map (none on a
  refused move, a map door, in an area or a region), by the terrain
  entered: records 0..7 in order, one percentile each for those on that
  terrain, empty records on terrain 0 included, every hit appearing. No
  cool-down; no rolls at all while the 4 areas are in use. The decoder's
  chance table is right. (Run.)
- **The area** is one of 4 spare rooms of the map's region: 15 x 10, walls
  of the entered terrain's picture, one door at 0,5; the player on 12,4,
  record r's copy on 2 + r mod 2, 3 + r div 2. Copies are the records
  unchanged, in the map's slots 8..15. Entering ends the turn; nothing is
  drawn to set it up. (Run, the squares of records 2..7 traced.)
- **Play** runs in the map region's visit, the area's individuals after
  the open map's players; no round-end rolls. (Run for one player.)
- **Leaving** through the door puts the player on the entrance square
  (the one stepped onto) with the moves it had; with no player left the
  area closes, its copies removed and its items deleted. Death closes it
  too (traced). A creature through the door is removed (traced).

Next pass (findings section 7): two players, a death in an area, save and
retire inside, all areas and slots in use, fighting there, the door's
table entry. Each needs a "Pass 2" section here with its inputs.
