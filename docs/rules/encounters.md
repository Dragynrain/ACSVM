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
program. **[capture]** marks what our own runs confirm (capture.md 10.20:
every draw of 824 runs from the brief's scenarios, the random state
swept, and a walk out); "[research]" alone is not yet captured by us.
The areas' item lists (3.4, 3.5, 7.2) are our own format find, tagged
**[data]** for the stored lists ([format notes](../format-notes.md), the
world map's tail) and **[capture]** for capture.md 10.26 (runs rr22,
2026-11-14: lists of ours written into a scenario disk).

## 1. The records

1. The world map has 8 random creature records, each with a terrain and a
   chance code 0..15. [data] An unused record is stored as an empty
   creature on terrain 0 with code 0. [data: the capture template;
   research, run]
2. **The chance**: 2 x c percent for c 0..5, then 10 + 5 x (c - 5) for
   c 6..15: 0, 2, 4, 6, 8, 10, 15, 20, .., 60. [manual: 0..60%] This is
   the decoder's reading (`WorldCreature::chance`). [research: run for
   code 15, a forced 59 hit and 60 missed; traced for the rest] [capture:
   code 15 in 824 runs, a roll of 60 missing]

## 2. The roll

1. **After the step.** The roll comes once the player stands on the new
   map square and the move is used, and goes by the **terrain of the
   square entered**. [research, run; capture: capture.md 10.19, 10.20: the
   draws come right after the square changes]
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
   hitting] So empty records on terrain 0 roll and never hit. [capture:
   capture.md 10.20, every step of 824 runs: one roll per grass step,
   8 per field step with WOLF's first, 7 with only empty records there;
   WOLF and HARE both appearing in 87 of 256 runs]
   **Engine:** so, empty records included (their roll uses the generator
   as the original's does).
6. **A hit places a copy** of its record (4.3) in the first free creature
   slot of the map, 8 to 15, shared by all 4 areas; with none free that
   hit places nothing. The area opens only if at least one copy was
   placed. [research, traced] **Engine:** so; a record that is not in use
   places nothing (it cannot hit in practice, its chance being 0).
7. Setting the area up takes no rolls: its place, the squares and the
   creatures' values are all fixed. [research, run; capture: no draw
   between the rolls and the turn's end]

## 3. The encounter area

1. An area is one of **four spare rooms of the world map's region** (rooms
   1 to 4 of region 0; room 0 is the open map). A hit takes **the first
   free one** and notes **the map square stepped onto** as its entrance.
   [research, run: room 1 in every run; capture: area 1 in every one of
   our encounters] **Engine:** the map region is
   kept by the game (it is not in the adventure's region list), its rooms
   1..4 the areas; a player in an area stands at
   `Location::Room { region: 0, room: 1..=4, .. }`.
2. **Size**: 15 wide, 10 tall, walls included (the inside 13 x 8), every
   area the same. [research, run]
3. **Walls**: the picture of the terrain entered (the grass's dots; the
   field's blank picture, so no border). [research, run; capture: the
   grass's picture in area 1's wall byte]
4. **Inside**: no random creature of its own (chance 0), not on any
   region grid. [research, run] **Its things come from the file**: the
   world map block stores one item list per area, in the room item-list
   format, read as that area's room items. [data; capture: a treasure
   added to area 1's list was drawn there and could be taken, a second
   list showed only in area 2, capture.md 10.26] In every corpus
   adventure each list holds only the door (3.5), so in practice an area
   is empty floor. [data; research, run] Anything placed there acts as in
   any room: a store, for one, opens the store screen
   ([stores](stores.md) 2.7). [capture] **Engine:** rooms 1..4 of the map
   region are seeded from the four lists once, when the game starts, and
   keep their items from then on like any room (closing deletes all but
   the door, 7.2). Room 0's list (empty in the corpus) is kept for
   round-tripping and not placed on the open map. How the original treats
   a list it cannot read is [unknown]. **Engine:** a list the decoder
   cannot read is a `BadRoomItems` error, and that list and every list
   after it are left empty; the decoder warns (it does not refuse) on an
   area with no door or several, a door parameter above 3, an item outside
   15 x 10, a non-empty room 0 list, and bytes at world map + 0x806 or
   + 0x80E other than the corpus's ([format notes](../format-notes.md),
   the world map's tail).
5. **The way out is the list's door item**: thing 82 on the left wall,
   column 0, row 5, with a parameter n (0..3 for areas 1..4 in the
   corpus). [data] It is an ordinary item, not a fixed square: moved to
   0,3 the wall gap and the way out moved with it and 0,5 became wall;
   with the list empty there was no way out at all, a push west from 1,5
   refused with no move used. [capture: capture.md 10.26; the gap at row
   5 and the door taken from 1,5 as stored, 10.20] [research, run: the
   gap in `patch-step-1.png`] **Where it leads**: to the map entrance of
   **area n + 1** (7.1), whichever area the door stands in; if that area
   is free (its entrance 41,41) the player goes to the adventure's start
   point on the map instead. [capture: a door with n = 1 in area 1 sent
   the player to 12,12, the scenario's start, and to 15,11 once area 2's
   entrance was set there] The table the parameter reads is most likely
   the four entries before the lists (format notes); unconfirmed.
   **Engine:** the exit is the list item: an item of thing 82 in an area
   is a door whose parameter n names area n + 1's entrance (the start
   point when that area is free, or n is above 3); stepping onto it is
   walking out (7.1). Front ends draw its square as an opening, and an
   area with no such item has no way out.
   **Several doors on one square**: what the original does is [unknown].
   **Engine:** any item of thing 82 there makes the square a door; the
   first such item in the room's item list gives the parameter.
   **A door leading to another open area's entrance**: whether the player
   joins that area is [unknown]. **Engine:** it does not; the player is
   put on that map square as on any door's destination, which is not a
   step, so it neither joins the area (5.2) nor takes a roll (2.2).
6. **Squares**: the player on **12,4**, whichever way it came. [research,
   run: entering eastward and northward] Each creature by its record
   number r: **column 2 + (r mod 2), row 3 + (r div 2)**, so record 0 on
   2,3, record 1 on 3,3, .., record 7 on 3,6. [research: run for records 0
   and 1, traced for the rest] [capture: 12,4 entering east and north;
   record 0 on 2,3 and record 1 on 3,3, also alone in slot 8]

## 4. The creatures

1. **One copy per record that hit**, up to the free slots: no count, no
   roll for a number. [research, run for 1 and 2; capture: 1 and 2, in
   slots 8 and 9 in record order]
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
   area's creatures take part from the next round (6.2). [research, run; capture: the moves 0 in
   every encounter] Events: `Encounter` for each copy placed, then `EnteredArea`.
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
   **Engine:** creatures see the door item's square (3.5) as a door (an
   exit, creatures 6.4) and one that steps onto it is removed
   (`CreatureRemoved`, reason `left_area`).

## 7. Leaving and closing

1. **Walking out.** The player leaves by stepping onto the door (3.5, at
   0,5 as stored), which uses one move as usual; it comes back on the map
   **on the area's entrance square** (the square it stepped onto when the
   roll hit, not the one it came from), keeping the moves it has left.
   This is the stored door, whose parameter names its own area; a door
   naming another area leads to that one's entrance (3.5). The return takes
   no roll; the next step does. [research, run; capture: from 1,5 west, T
   on the entrance 13,12 with 5 of 6 moves] Event: `LeftArea`.
2. **Closing.** When no living player is left in the area it closes: it is
   marked free, its entrance forgotten, every creature copy in it removed
   for good (`CreatureRemoved`, reason `area_closed`), and every item
   lying in it deleted (the door stays). [research: run for the freeing,
   the entrance and the copy's removal; traced for the items] [capture:
   the size byte 0, the entrance 41,41, slot 8 free; capture.md 10.26: a
   treasure's entry in area 1's list marked deleted in place, the door's
   entry kept] Items written into the file's lists go too, so a treasure
   an area starts with is most likely there only until it first closes
   (reopening the area after that was not captured). Which entries count
   as "the door" with several door items or a door moved is [unknown].
   **Engine:** every item of thing 82 stays, wherever it lies and however
   many there are (as 3.5, any of them is a door), every other item is
   deleted; `AreaClosed`.
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
gold), and which door table entry the area door reads (capture.md 10.26
showed the parameter picking area n + 1's entrance; the four entries at
world map + 0x806 as its table are a guess). Not captured by us yet: a
second player joining, a death in an area, a creature through the door,
all areas or slots in use (capture.md 10.20); an area reopened after its
listed items were deleted, several doors in one area, and changing the
entries at + 0x806 (capture.md 10.26).

**Captures needed** for the engine's choices above: several door items
on one square (which parameter counts) and a door moved off 0,5 at
closing (3.5, 7.2); a door whose parameter names another open area's
entrance, and whether the player joins that area (3.5); how the game
loads a damaged area list and lists with no door, several doors, a
parameter above 3, an item outside 15 x 10 or items in room 0's list
(3.4); and other bytes at + 0x806 / + 0x80E.
