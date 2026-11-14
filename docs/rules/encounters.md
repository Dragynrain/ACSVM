# Rules: world map encounters

Behaviour spec for the world map's random creatures: the roll on a map
step, the encounter area the player is pulled into, play there and how it
ends. Licensed CC BY 4.0. The engine implements it in
`crates/acs-engine/src/encounters.rs` (the roll called from `step.rs`).

Tags as in [movement](movement.md): **[manual]** paraphrased from the
manual, **[data]** from the stored format or corpus counts, **[unknown]**
with the engine's choice as **Engine:**, **[capture]** for our own runs.
**[research]** is the clean-room researcher's first encounters pass
(2026-11-14, `docs/research/encounters.md`): "run" where the researcher
confirmed it in VICE (about 15 runs from the brief's snapshots, some with
the random draw forced), "traced" where it was only read from the
program. No capture of ours has checked any of it yet beyond capture.md
10.19.

## 1. The records

1. The world map has 8 random creature records, each with a terrain and a
   chance code 0..15. [data] An unused record is stored as an empty
   creature on terrain 0 with code 0. [data: the capture template;
   research, run]
2. **The chance**: 2 x c percent for c 0..5, then 10 + 5 x (c - 5) for
   c 6..15: 0, 2, 4, 6, 8, 10, 15, 20, .., 60. [manual: 0..60%] This is
   the decoder's reading (`WorldCreature::chance`). [research: run for
   code 15, a forced 59 hit and 60 missed; traced for the rest]

## 2. The roll

1. **After the step.** The roll comes once the player stands on the new
   map square and the move is used, and goes by the **terrain of the
   square entered**. [research, run; capture: capture.md 10.19, the step
   from the field onto the grass]
2. **Every completed step on the open map rolls**, the last move of a turn
   included. [research, run] None is taken for:
   - a refused move (no move used either); [research, run]
   - a step onto a map door (the player goes through as usual); [research,
     run]
   - a player in a room, a region or an encounter area; [research, run]
   - a step onto the entrance of an open area (5.1). [research, traced]
3. **No cool-down**: the first step after coming back from an area rolls
   as usual. Nothing else changes the chance: not the player's ratings,
   other players or the turn. [research, run; traced]
4. **Only while an area is free.** There are 4 areas (3.1). With all 4 in
   use a step takes no rolls at all. [research, traced]
5. **Which records roll.** Records 0 to 7 in order; one whose terrain is
   not the entered square's is skipped without a roll; each that matches
   takes one percentile roll (0..99) and **hits below its chance**.
   **Every record that hits appears**: there is no stopping at the first.
   [research, run: one roll on the grass, 8 on the field (WOLF and the 7
   empty records on terrain 0, rolled against 0), WOLF and HARE both
   hitting] So empty records on terrain 0 roll and never hit.
   **Engine:** so, empty records included (their roll uses the generator
   as the original's does).
6. **A hit places a copy** of its record (4.3) in the first free creature
   slot of the map, 8 to 15, shared by all 4 areas; with none free that
   hit places nothing. The area opens only if at least one copy was
   placed. [research, traced] **Engine:** so; a record that is not in use
   places nothing (it cannot hit in practice, its chance being 0).
7. Setting the area up takes no rolls: its place, the squares and the
   creatures' values are all fixed. [research, run]

## 3. The encounter area

1. An area is one of **four spare rooms of the world map's region** (rooms
   1 to 4 of region 0; room 0 is the open map). A hit takes **the first
   free one** and notes **the map square stepped onto** as its entrance.
   [research, run: room 1 in every run] **Engine:** the map region is
   kept by the game (it is not in the adventure's region list), its rooms
   1..4 the areas; a player in an area stands at
   `Location::Room { region: 0, room: 1..=4, .. }`.
2. **Size**: 15 wide, 10 tall, walls included (the inside 13 x 8), every
   area the same. [research, run]
3. **Walls**: the picture of the terrain entered (the grass's dots; the
   field's blank picture, so no border). [research, run]
4. **Inside**: empty floor; no items, no random creature of its own
   (chance 0), not on any region grid. [research, run]
5. **One way out**: a door on the left wall at column 0, row 5 (the gap in
   `patch-step-1.png`), part of the stored map data, the same in every
   area and kept when the area is cleared. [research, run] **Engine:** the
   door is not an item: the square 0,5 of an area is its exit (6.1, 6.4)
   and front ends draw it as an opening.
6. **Squares**: the player on **12,4**, whichever way it came. [research,
   run: entering eastward and northward] Each creature by its record
   number r: **column 2 + (r mod 2), row 3 + (r div 2)**, so record 0 on
   2,3, record 1 on 3,3, .., record 7 on 3,6. [research: run for records 0
   and 1, traced for the rest]

## 4. The creatures

1. **One copy per record that hit**, up to the free slots: no count, no
   roll for a number. [research, run for 1 and 2]
2. **The copy is the record as stored**: Life Force, Power, Speed, skills,
   possessions, gold and name unchanged. [research, run: compared byte for
   byte] It differs in its place (the area and its square), in carrying
   the random-creature mark ([creatures](creatures.md) 2.4) and in a high
   bit kept with Power being cleared, whose meaning is unknown. [research,
   run] **Engine:** the copy is marked random; the cleared bit is not
   modelled (the model keeps Power without it).
3. The record itself never changes: what happens to the copy stays with
   the copy. [research, traced]

## 5. Entering

1. **Entering ends the player's turn**: its moves left go to 0, and the
   area's creatures take part from the next round (6.2). [research, run]
   Events: `Encounter` for each copy placed, then `EnteredArea`.
2. **A second player** who steps onto the entrance square of an open area
   joins it at once, with no roll, on 12,4 like the first, and its turn
   ends. [research, traced] What two players on 12,4 do is [unknown].
   **Engine:** it joins, sharing 12,4 (as on a door's destination,
   movement 6.3).

## 6. Play inside

1. An area is a room of the map's region, so its turns run in the map's
   **region visit** ([creatures](creatures.md) 2.2): the round's list
   holds the players on the open map, then the area's individuals, players
   before creatures in number order. [research: run for one player and one
   creature (T, then WOLF); traced for the rest] With players in several
   areas the order is [unknown]. **Engine:** the open map's players in
   joining order, then each area in use, 1 to 4, with its players in
   joining order and its creatures in slot order.
2. A creature there takes a room creature's turn (creatures 5 to 8).
   [research, run: the draws match]
3. **No end-of-round rolls** in the map's region: no slot roll, no
   left-behind check, no random room creature. [research, run]
4. **A creature that walks through the door is removed**: it is sent
   towards the open map, which creatures cannot enter. [research, traced]
   **Engine:** creatures see 0,5 as a door (an exit, creatures 6.4) and
   one that steps onto it is removed (`CreatureRemoved`, reason
   `left_area`).

## 7. Leaving and closing

1. **Walking out.** The player leaves by stepping onto the door at 0,5,
   which uses one move as usual; it comes back on the map **on the area's
   entrance square** (the square it stepped onto when the roll hit, not
   the one it came from), keeping the moves it has left. The return takes
   no roll; the next step does. [research, run] Event: `LeftArea`.
2. **Closing.** When no living player is left in the area it closes: it is
   marked free, its entrance forgotten, every creature copy in it removed
   for good (`CreatureRemoved`, reason `area_closed`), and every item
   lying in it deleted (the door stays). [research: run for the freeing,
   the entrance and the copy's removal; traced for the items] Event:
   `AreaClosed`.
3. **The creatures dying or leaving does not end the area**; only its last
   player leaving or dying does. [research, traced]
4. **A player dying in the area** closes it if no player is left, as 7.2,
   and the usual death follows. [research, traced]
5. **A player standing in a freed area** (only in odd cases) is sent to
   the adventure's start; a creature there is removed. [research, traced]
   **Engine:** the player's next step does it, as for any room that is
   gone (movement 6.4).
6. **Save and retire in an area**: not looked at. [unknown] **Engine:**
   saves are replays of the command log ([saves](../saves.md)), so a game
   saved in an area comes back in it.
7. Nothing on the map itself changes: its squares and its records are
   untouched. [research, traced]

## 8. Left for later

The researcher's next pass (`docs/research/encounters.md`): two players
in one area or in different areas, a death in an area run rather than
traced, save and retire inside, all four areas in use and all eight slots
taken, fighting there (the cleared bit, what a dead copy drops and its
gold), and which door table entry the area door reads. Our own captures
should check the roll's draw order, the squares and the walk-out
(capture.md 9 gives the state to read; findings section 6 the addresses).
