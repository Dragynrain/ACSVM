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
2026-11-14: lists of ours written into a scenario disk) and 10.34 (runs
rr30, 2026-11-14: door items moved and doubled, the door table's bytes
changed, an area reopened, the lists the decoder warns on, a store in an
area). Play with several players, deaths, saves and full areas or slots
(2.4, 2.5, 2.6, 4.4, 5.2, 6.1, 6.4, 7.3, 7.4, 7.6) are **[capture]** of
capture.md 10.36 (runs rr32, 2026-11-15; our own, no research pass).

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
   use a step takes no rolls at all. [research, traced] [capture:
   capture.md 10.36 item 5, all four areas held open: 16 of 16 runs drew
   nothing on any grass step, the player walked on and no area or slot
   changed]
5. **Which records roll.** Records 0 to 7 in order; one whose terrain is
   not the entered square's is skipped without a roll; each that matches
   takes one percentile roll (0..99) and **hits below its chance**.
   **Every record that hits appears**: there is no stopping at the first.
   [research, run: one roll on the grass, 8 on the field (WOLF and the 7
   empty records on terrain 0, rolled against 0), WOLF and HARE both
   hitting] So empty records on terrain 0 roll and never hit. [capture:
   capture.md 10.20, every step of 824 runs: one roll per grass step,
   8 per field step with WOLF's first, 7 with only empty records there;
   WOLF and HARE both appearing in 87 of 256 runs] With two records on
   the grass (60 % each, one step a run, 256 runs) the outcomes were
   neither 45, WOLF alone 65, HARE alone 59, both 87; each draw of every
   run follows from the generator alone, and the two hits are
   independent (152 and 146 hits of 256 predict 86.7 runs with both,
   against 87). [capture: capture.md 10.36 item 8, which corrects 10.20's
   WOLF-alone count] So there is no rule linking the records' rolls.
   **Engine:** so, empty records included (their roll uses the generator
   as the original's does).
6. **A hit places a copy** of its record (4.3) in the first free creature
   slot of the map, 8 to 15, shared by all 4 areas; with none free that
   hit places nothing. The area opens only if at least one copy was
   placed. [research, traced] [capture: capture.md 10.36 item 5, slots 8
   to 15 all alive (in area 4): every grass step still took its roll,
   hits included, yet no area opened and the player walked on with its
   moves, 14 of 14 runs; item 2, a second area's copy took slot 9 beside
   area 1's in slot 8] **Engine:** so; a record that is not in use
   places nothing (it cannot hit in practice, its chance being 0).
7. Setting the area up takes no rolls: its place, the squares and the
   creatures' values are all fixed. [research, run; capture: no draw
   between the rolls and the turn's end]

## 3. The encounter area

1. An area is one of **four spare rooms of the world map's region** (rooms
   1 to 4 of region 0; room 0 is the open map). A hit takes **the first
   free one** and notes **the map square stepped onto** as its entrance.
   [research, run: room 1 in every run; capture: area 1 in every one of
   our encounters; area 2 for a second encounter while area 1 was in use,
   capture.md 10.36 item 2] **Engine:** the map region is
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
   the doors, 7.2; an area reopened later has only what closing left,
   capture.md 10.34). Room 0's list (empty in the corpus) is kept for
   round-tripping and not placed on the open map; how the original treats
   items in it was not tried [unknown].

   **Odd lists** (capture.md 10.34): the game read every one of these
   without refusing the area or crashing. [capture]
   - An item outside the area's 15 x 10 squares is not drawn and can't be
     reached. At closing, one such item (on 7,12) was marked deleted like
     any other, another (x 15, row 3) was left in place. [capture]
   - A list whose stored length runs past its own items takes the bytes
     after them (the following lists, then the zeros after the lists) as
     more items of that area. Items so read that lie on wall squares show
     there as gaps in the wall (three gaps in the run, one of them the
     real door on 0,5, which worked as usual). At that area's closing no
     entry was marked deleted. [capture] Whether the other gaps can be
     walked through was not tried. [unknown]

   **Engine:** the lists are read one after another by their stored
   lengths, as the original reads them, so an over-long length gives that
   area the items of the bytes that follow, as on the C64. Only a length
   that is not 2 + 3k, or that runs past the end of the block, is a
   `BadRoomItems` error; that list and every list after it are left
   empty. An item outside 15 x 10 stays in its list but is never in play
   (not drawn, not reached, not picked up); closing deletes it with the
   other items, which can't be told apart in play from the original
   keeping one. Only a thing-82 item makes a wall square a way through
   (3.5); front ends may draw any item on a wall square as a gap. Closing
   an area read from an over-long list deletes its non-door items as
   usual, unlike the one run above (left open, 8). The decoder warns (it
   does not refuse) on an area with no door or several, a door parameter
   above 4 (3.5), an item outside 15 x 10, a non-empty room 0 list, and
   bytes at world map + 0x806 or + 0x80E other than the corpus's ([format
   notes](../format-notes.md), the world map's tail).
5. **The way out is a door item**: thing 82 with a parameter n. As
   stored it lies on the left wall, column 0, row 5, with n = 0..3 for
   areas 1..4. [data] It is an ordinary item, not a fixed square:
   [capture: capture.md 10.26, 10.34; the gap at row 5 and the door taken
   from 1,5 as stored, 10.20] [research, run: the gap in
   `patch-step-1.png`]
   - **On a wall square** it makes a gap there, and stepping onto it walks
     out (7.1). A wall square with no door item is wall: with doors on
     0,3 and 0,7 both rows had gaps, each a way out, and 0,5 was wall (a
     push west from 1,5 refused, no move used); with the list empty there
     was no way out at all. [capture: 10.26, 10.34]
   - **On an inside square** it is a way out too, and is drawn as plain
     floor: with one on 7,5, stepping onto it from 8,5 took the player
     out. [capture: 10.34]
   - **Several doors on one square**: the first thing-82 item in the list
     gives the parameter. Two on 0,5 listed with parameters 0 then 1 led
     to area 1's entrance; listed the other way round, to area 2's.
     [capture: 10.34]

   **Where it leads.** The parameter n picks entry n of the **area door
   table** at world map + 0x806 ([format notes](../format-notes.md), the
   world map's tail), entries 0 to 3, two bytes each in the room door
   format ([movement](movement.md) 6.1): byte 0 is region x 16 + room,
   byte 1 is x x 16 + y. The entry is read as a room door's destination
   is, with codes of its own for the areas: [capture: 10.34, each entry
   changed on its own with area 1's door at parameter 0]
   - **Region 0, point code 0x41..0x44** (`00 41`..`00 44`, the corpus's
     four entries in order): the map entrance of area 1..4, whichever
     area the door stands in. When that area is free (entrance 41,41) the
     player goes to the adventure's start point on the map. [capture:
     entry 0 set to `00 42` led to area 2's entrance; 10.26: a door with
     n = 1 led to the start while area 2 was free, to its entrance once
     set]
   - **Region 0, a point 0..63**: that world map point. `00 00` put the
     player on map point 0, the square of map door 0, standing there: it
     was not sent through that door. [capture]
   - **Region 1 or above**: that region, room and square, as a room door
     to another region goes ("The action moves to" the region): `10 55`
     took the player to region 1, room 0, square 5,5. [capture]
   - **Parameter 4** reads the word at + 0x80E as a fifth entry: with
     `00 42` there the player came out at area 2's entrance; with the
     editor's `32 08` the game moved to region 3, room 2, square 0,8 (a
     region the scenario doesn't use; the template disk still held data
     for it). Changing + 0x80E made no visible difference to doors with
     parameters 0..3. [capture] What parameters above 4 read is
     [unknown] (by the pattern, the bytes after + 0x80E, where the lists
     begin).

   **Engine:** an item of thing 82 in an area is a door wherever it lies;
   any number may share a square, the first in the room's item list
   giving the parameter; stepping onto one is walking out (7.1). The
   parameter picks the entry: n = 0..3 the bytes 2n and 2n + 1 of
   `WorldMap::unknown_806`, n = 4 the word `WorldMap::unknown_80e` (its
   low byte as byte 0, its high byte as byte 1). These are raw fields the
   model already keeps, so `MODEL_VERSION` does not change. The entry is
   resolved as above. The start point is used for an area code whose area
   is free, a region 0 code that is neither 0..63 nor 0x41..0x44, an
   unused map point (0xFF), a region or room the adventure doesn't have
   (as a door into a deleted room, movement 6.4, 6.5), and a parameter
   above 4. Arriving is not a step: there is no map roll (2.2), and a map
   door or room door on the arrival square is not taken (movement 6.3).
   Whether a door leading into a room of region 1 or above rolls that
   room's random creature ([movement](movement.md) 7.2) was not looked at
   [unknown]. **Engine:** it does, reading "as a room door to another
   region goes" to include the room's roll on entering. A destination
   whose region or room is missing sends the player to the start point
   with `LeftArea` only; whether the original shows anything more there
   is [unknown], and the engine emits no `SentToStart`. Front ends
   (walk mode included) draw a door item with the floor picture, on a
   wall square and an inside square alike, so a wall door shows as an
   opening; whether the original's wall gap is the floor picture or a
   picture of its own was not checked [unknown]. An area with no door item has no way out.

   **A door leading to another open area's entrance**: the player comes
   out on that map square, on the open map (place 0) with its moves left,
   and does not join the area, which stays open. [capture: 10.34, area 2
   held open by pokes (its room record and entrance) with no creature in
   it; an area opened by a real encounter was not tried] **Engine:** so;
   the arrival is not a step, so it neither joins the area (5.2) nor
   takes a roll (2.2).
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
4. **A copy killed in the area** drops what it carries and its gold on
   its square, as a creature dying anywhere does ([combat](combat.md)):
   a WOLF copy holding a blade and 50 gold left two new entries at the
   front of the area's item list, on its square, the blade and a gold
   item of 50, the door after them; the killer's gold did not change.
   Both were deleted when the area closed (7.2), the door kept.
   [capture: capture.md 10.36 item 7] **Engine:** so.

## 5. Entering

1. **Entering ends the player's turn**: its moves left go to 0, and the
   area's creatures take part from the next round (6.2). [research, run; capture: the moves 0 in
   every encounter] Events: `Encounter` for each copy placed, then `EnteredArea`.
2. **A second player** who steps onto the entrance square of an open area
   joins it at once, with no roll, on 12,4 like the first, and its turn
   ends. [research, traced] [capture: capture.md 10.36 item 1: the player
   joined area 1 on 12,4, the square the first player stood on, with no
   other area opened, no copy placed and its moves 0; no draw between the
   step and the join in 8 of 8 runs] The two share 12,4. Any player may
   join any open area, whoever opened it: one that walked out of its own
   area (which closed) and onto another area's entrance joined that area.
   [capture: 10.36 item 2, `o3`] **Engine:** so, sharing 12,4 as on a
   door's destination (movement 6.3).

## 6. Play inside

1. An area is a room of the map's region, so its turns run in the map's
   **region visit** ([creatures](creatures.md) 2.2): the open map and all
   four areas share **one round counter and one turn list a round**.
   [research: run for one player and one creature (T, then WOLF); traced
   for the rest] [capture: capture.md 10.36 item 2, five cases] The
   round's list, built at its start:
   1. the living players on the open map;
   2. then each area that holds a living player, **ordered by the lowest
      player number in it** (not by area number, nor by which opened
      first);
   3. within an area, its living players **in player-number order** (not
      the order they came in), then its creature copies in slot order.

   Seen, players T (1) and U (2): U on the map, T in area 1 with copy 8:
   U, T, 8. Both in area 1: T, U, 8. T in area 1 with copy 8, U in area 2
   with copy 9: T, 8, U, 9. U into area 1 first (copy 8), T into area 2
   later (copy 9): T, 9, U, 8. U in area 2 with copy 9, T joining it
   afterwards: T, U, 9. [capture: 10.36 item 2]

   Not seen: the order of **several players on the open map** at once
   while an area is in use (every case had at most one there), and
   whether the map's visit **stops after 10 rounds** as other regions' do
   (creatures 2.2.2): nothing outside region 0 was in play, so a new
   visit could not be told from the old one. [unknown] **Engine:** the
   open map's players in player-number order, like an area's (the same
   as joining order unless a retired player's number is taken again);
   the 10-round visit limit applies to region 0 as to any region.
2. A creature there takes a room creature's turn (creatures 5 to 8).
   [research, run: the draws match]
3. **No end-of-round rolls** in the map's region: no slot roll, no
   left-behind check, no random room creature. [research, run]
4. **The area's door is closed to creatures.** A creature never steps
   onto it and does not count it as an exit (creatures 5.3, 6.4), so a
   copy stays in its area until it dies or the area closes (7.2).
   [capture: capture.md 10.36 item 6: a peaceful neutral copy, whose only
   goal is an exit, put on 1,5 beside the door on 0,5 never stepped onto
   it in 8 of 8 runs, its first step going north or south; in the turn
   order runs, 150M instructions each, the copies wandered rows 1 to 8
   and none left] The researcher's reading, that a creature walking
   through the door is removed, [research, traced] was not borne out:
   nothing of ours ever reached the door. Only a door on a wall square
   (0,5) was tried; a door item on an inside square (3.5) was not tried
   with a creature beside it. [unknown] **Engine:** a square holding a
   thing-82 item in an area is closed to creatures wherever it lies and
   is never a goal, so an area offers no exit at all to a creature (one
   whose only goal is an exit has no goal there, creatures 6.2). No creature
   is ever removed for leaving an area.

## 7. Leaving and closing

1. **Walking out.** The player leaves by stepping onto the door (3.5, at
   0,5 as stored), which uses one move as usual; it comes back on the map
   **on the area's entrance square** (the square it stepped onto when the
   roll hit, not the one it came from), keeping the moves it has left.
   This is the stored door, whose parameter picks its own area's entry in
   the area door table; another parameter or entry leads where that entry
   says (3.5). The return takes no roll; the next step does. [research, run; capture: from 1,5 west, T
   on the entrance 13,12 with 5 of 6 moves] Event: `LeftArea`. A door
   whose region or room is missing sends the player to the start with
   `LeftArea` alone, no `SentToStart` (engine's choice, 3.5 [unknown]).
2. **Closing.** When no living player is left in the area it closes: it is
   marked free, its entrance forgotten, every creature copy in it removed
   for good (`CreatureRemoved`, reason `area_closed`), and every item
   lying in it deleted (the door stays). [research: run for the freeing,
   the entrance and the copy's removal; traced for the items] [capture:
   the size byte 0, the entrance 41,41, slot 8 free; capture.md 10.26: a
   treasure's entry in area 1's list marked deleted in place, the door's
   entry kept] **Every door entry stays**, wherever it lies and however
   many there are: two doors on 0,5, doors on 0,3 and 0,7, and one on the
   floor at 7,5 were all kept, the treasure beside them marked deleted.
   [capture: capture.md 10.34] Items written into the file's lists go
   too: **a treasure an area starts with is there only until the area
   first closes**. Opened again, the area had its doors and no treasure;
   the deleted entry (square byte 255) stayed in the list unchanged, the
   list not compacted. [capture: 10.34, three runs] (Items outside 15 x
   10 and over-long lists: 3.4.) **Engine:** every item of thing 82 stays
   (as 3.5, any of them is a door), every other item is deleted;
   `AreaClosed`. The engine drops deleted items from the room rather than
   keeping their entries; nothing in play can tell the difference.
3. **The creatures dying does not end the area**; only its last player
   leaving, dying or retiring does (7.4, 7.6). [research, traced]
   [capture: capture.md 10.36 item 7: with the only copy killed the area
   stayed open until the player walked out] (Creatures cannot leave, 6.4.)
4. **A player dying in the area** closes it if no player is left, as 7.2,
   and the usual death follows. [research, traced] [capture: capture.md
   10.36 item 3, four runs, the player alone and with a second player on
   the map: between the killing blow and the death message the area
   closed (free, entrance forgotten, the copy's slot freed); answered with
   F1, the other player played on alone on the map; with the button, the
   game came back at the last save, made before the area opened] The
   dead player's place byte was left naming the area, which nothing
   showed. **Engine:** so; the death closes the area before the death's
   choice ([creatures](creatures.md) 2.2.5, 2.2.6).
5. **A player standing in a freed area** (only in odd cases) is sent to
   the adventure's start; a creature there is removed. [research, traced]
   **Engine:** the player's next step does it, as for any room that is
   gone (movement 6.4).
6. **Save and retire in an area.** [capture: capture.md 10.36 item 4]
   - **SAVE GAME** inside keeps the area: restored, the game came back in
     the round after the save with the player in the area on its square,
     its gold as saved, the area open with its entrance, and the copy
     alive on the square it held at the save.
   - **Retiring** inside: the offer to save comes at the round's end with
     the area still open; once the player is saved its record is retired,
     and if no living player is left in the area it closes as in 7.2 (the
     copy gone), the next player's turn following. The retired record
     keeps the area's place and 12,4, but a retired player taken back
     into the game starts **at the adventure's start point on the open
     map**, not in the area. Retiring one of two players in an area was
     not tried. [unknown]

   **Engine:** saves are replays of the command log ([saves](../saves.md)),
   so a game saved in an area comes back in it, as above. Retiring takes
   the player out of the area like a death (7.4): the area closes when
   no living player is left in it; with another player still there it
   stays open. A retired player added again starts at the start point.
7. Nothing on the map itself changes: its squares and its records are
   untouched. [research, traced]

## 8. Left for later

The encounters research is closed (no further passes); what is left is
capture work. Still open: the meaning of the high bit cleared with a
copy's Power (4.2), and a player standing in a freed area (7.5,
[research, traced] only).

**Captures needed** for the engine's choices above:
- the turn order of several players on the open map while an area is in
  use, and whether region 0's visit ends after 10 rounds (6.1; the
  engine uses player-number order and the 10-round limit);
- a creature beside a door item on an inside square (6.4; the engine
  closes every door item's square to creatures);
- retiring one of two players in an area (7.6; the engine keeps the area
  open for the other);
- a door parameter above 4 (3.5: what it reads past + 0x80E);
- a door to another area's entrance where that area was opened by a
  real encounter, not by pokes (3.5);
- items in room 0's list (3.4);
- the extra wall gaps an over-long list makes: can they be walked
  through (3.4); and why closing such an area marked no entry deleted,
  where the engine deletes as usual (3.4, 7.2);
- which off-room items closing keeps (one on 7,12 deleted, one at x 15
  kept, 3.4): no effect on play;
- a door entry leading into a room of region 1 or above that has a
  random creature chance: does arriving roll it (3.5; the engine rolls,
  movement 7.2);
- a door entry naming a region or room the adventure doesn't have: what
  the game shows on the way to the start (3.5, 7.1; the engine sends
  `LeftArea` only, no `SentToStart`);
- the picture of a door's gap on a wall square, at 0,5 and elsewhere
  (3.5; the engine draws the floor picture on wall and inside squares).

Settled by capture.md 10.34: several doors on one square, doors moved
off 0,5 and on the floor, the area door table at + 0x806 and the fifth
entry at + 0x80E, a door to an open area's entrance, reopening after
closing, items outside 15 x 10 and over-long lists (3.4, 3.5, 7.2).
Settled by capture.md 10.36: a second player joining, players in
several areas and their turn order, a death inside, SAVE GAME and Retire
inside, all areas or all slots in use, a creature and the door, a killed
copy's drop, and the independence of two records' rolls (2.4 to 2.6,
4.4, 5.2, 6.1, 6.4, 7.3, 7.4, 7.6).
