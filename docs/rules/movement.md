# Rules: movement

Behaviour spec for moving characters around the world map and rooms. Licensed
CC BY 4.0. The engine implements it in `crates/acs-engine` (`step.rs`).

Every rule is tagged with where it comes from:

- **[manual]**: paraphrased from the *Adventure Construction Set* manual
  (EA, 1984; private OCR copy, never quoted).
- **[data]**: follows from the stored format or from corpus counts (the 14
  Jadeworld C64 adventures plus EA's PC *Rivers of Light*, see
  `docs/format-notes.md` and `docs/provenance.md`).
- **[capture]**: measured by playing the original in an emulator with the
  capture harness ([capture.md](../capture.md)); the scenario and run count
  are given.
- **[unknown]**: not settled by either. The engine's current choice is given
  as **Engine:**, and the rule needs a capture from the original program
  (VICE or DOSBox-X) before it can be called faithful.

Counts below come from all 15 corpus adventures unless they say otherwise.

## 1. Places and positions

1. A character is either on the **world map** (a cell 0..39, 0..39) or in a
   **room** (region 1..15, room slot 0..15, square x, y inside the room's
   size, walls included). [data]
2. A new character starts on the world map at the adventure's start point.
   Characters added later start there too. [manual]
3. All 15 corpus start points lie on open terrain that is not a door. [data]
4. **Up to 4 characters play at once** (hot-seat), taking turns, all able
   to share one joystick; "Add Someone" brings in another. [manual] A
   character added mid-game joins the next round (creatures 2.2). The dead
   don't count toward the 4; a fifth while 4 live is refused at the end of
   the round it was asked in, before anything is rolled. [research;
   capture: [characters](characters.md) 6] A newcomer takes the lowest
   free slot, a dead or retired player's included (characters 6.1).
   **Engine:** characters 6.4, with no move used and nothing logged for replay besides the command.
5. A character's name is up to 15 characters. [manual] The C64 keeps all
   15 in a 40-symbol alphabet, and typing past 15 wraps to the start.
   [research; capture: [characters](characters.md) 5] **Engine:**
   characters 5.7 and 5.8.
6. "Retire" takes a character out of the adventure (or saves it for
   another) at the end of the round it was asked in. [manual; research;
   capture: [characters](characters.md) 7] **Engine:** characters 7.

## 2. Turns and the movement bar

1. On its turn a character has a **movement bar** that starts at its Speed
   (0..15) and drops by one for every square moved. The turn ends when it
   reaches zero. [manual] On the C64 the full bar is 8 pixels per point of
   Speed and each step takes 8 pixels: a Speed 6 player's bar is 48 pixels
   and holding the stick in one direction gives exactly 6 steps before the
   turn passes; the same holds for Speed 1, 3, 9 and 15 (8 to 120 pixels,
   1 to 15 steps). [capture: move-basic and the Speed survey, world map,
   20 runs each]
2. A move that is refused (impassable terrain, an obstacle, a wall) leaves
   the bar unchanged. [manual] On the C64, holding the stick into the world
   map's wall ring, a room wall or an obstacle thing leaves the bar as it
   was and the player in place: no step is spent. [capture: refuse, 20 runs
   each] A room door open only to owners of a thing refuses a non-owner
   the same way: no step, and nothing drains while the stick is held into
   it. [capture: acts, table lock, 5 runs] A space open only to owners of
   a thing refuses a non-owner the same way: the player stays where it
   was, no step is spent, and the bar does not drain while the push is
   answered. [capture: rr36 `bar-p` against the idle control `bar-c`,
   capture.md 10.40 item 5; ours, no research pass, 2026-11-16]
   **Engine:** no cost, as for obstacles.
3. Taking a door costs the one step onto the door square, and the turn goes
   on in the new place. [capture: refuse, 20 runs: a Speed 15 player that
   walks 3 squares and through the map door arrives on the room's entry
   square with 4 steps spent and the rest of the same turn] Room doors do
   the same: a one-way door out to a world map point, and a locked door
   (owners of a key) to another square of the same room, each took one
   step, and the turn went on in the new place; the same room is drawn
   again with the bar hidden, as after a region load. [capture: acts,
   tables exit and lock, 10 and 5 runs] A room door into **another room**
   is different: it costs the step and ends the turn, and the player shows
   in the new room only when its next turn starts. A Speed 12 player that
   walked 3 squares, the map door and 3 squares east onto a one-way portal
   into a second room of the region had 5 steps left; the room's creatures
   moved next, and the player was next seen in the new room at the top of
   its following turn. [capture: turns, case follow, 3 runs, capture.md
   10.10.3] Its next turn there starts with the moves it had left after
   the door's step (5 in that case), or its full Speed if none, exactly
   as after a map door; the round counter goes up between. One-way and
   two-way portals into another room of the region, and a one-way portal
   into a room of another region, all behave so. [capture: mem, rr25,
   capture.md 10.29; ours, no research pass, 2026-11-14] A **map door
   into a room** also ends the turn, as far as the scheduler goes: the
   region visit changes, and the player's first turn in the room starts
   with the moves it had left at the door (its full Speed if none), so a
   lone player seems to go on with the same turn, as `refuse` saw
   ([creatures](creatures.md) 2.2.4). [research: turn-order second pass,
   2026-11-14; capture: mem, capture.md 10.22] **Engine:** one step; the
   character then stands on the destination square and keeps its
   remaining moves, except through a door into another room (a room door
   to another room of the region or of another region, or a map door
   into a room), which ends the turn and carries the moves left into the
   next turn's start (creatures 2.2.4). A creature taking a door carries
   its moves left into its next turn the same way. [capture: mem, rr37,
   capture.md 10.41 item 4; [creatures](creatures.md) 5.4]
4. The bar also drains slowly in real time while the player does nothing.
   [manual] On the C64, a full Speed 6 bar left alone empties in 2,136 to
   2,142 frames (about 42.7 s on PAL), losing a pixel every 43 to 48 frames,
   so about 7.1 s per unused step. [capture: move-basic, 20 runs] The rate
   per pixel doesn't depend on Speed: an idle turn lasts about 354 frames
   per point of Speed plus about 17 (369 frames at Speed 1, 5,331 at
   Speed 15). [capture: Speed survey, 20 runs each] The drain runs only
   while the stick is let go: walking spends exactly a step's 8 pixels per
   square, and holding the stick into a wall or an obstacle drains nothing
   (the turn grows by the time held). [capture: refuse, 20 runs; acts,
   table obstacle, 10 runs: pushes of 0.5, 2 and 8 million instructions and
   five short bumps, the obstacle against the room wall] The one pixel the
   first `refuse` run saw against the obstacle went before the push: that
   case pushed at once on entering the room, while the bar was still
   hidden. Loading a region through a door doesn't drain the bar either.
   **Engine:** the rules have no clock. A front end keeps it and sends
   `Idle` each time a step's worth of the bar has drained: one idle move,
   with its healing roll ([magic](magic.md) 5.3), the turn ending when it
   was the last (a turn with no moves left ends at once). So the drain is
   in the command log and replays exactly, and a turn idled out in full
   draws the same rolls as one `Wait`. The bar drains in whole steps, one
   every 354 frames of idle time, the first after 371 (the turn's extra
   17 frames). **A part-drained step can be walked**: only a step's whole
   drain takes it. A Speed 6 player left idle until its bar first fell to
   5, then a further 0.1M to 1.75M instructions (its second drain coming
   between 1.75M and 1.85M after the first), then holding the stick,
   walked 5 squares each time; at 1.85M its moves read 4 and it walked
   4. [capture: rr40 `pd`, `pd.txt`, 5 of 5 runs, capture.md 10.44 item
   7; ours, no research pass, 2026-11-16] **Engine:** so: whole steps,
   the moves left at the push. `Idle`
   during a store visit does nothing, as no time passes inside
   ([stores](stores.md) 3.7), and the front end stops the clock there.
   The viewer's walk mode drains in real time by default (50.125 frames a
   second, the PAL C64's rate) and stops the clock while a store or a
   text field has the keys, or the window is out of focus; its
   **Turn-based** option stops the drain altogether (design.md, Play
   options).
5. With one character and no creatures in the room, the same character moves
   again when the bar refills. With several players, or creatures in the room,
   each gets a turn first. [manual] **Engine:** players and creatures take
   turns in the order of [creatures](creatures.md) 2.2.
6. Players exploring different regions get several turns in a row. [manual]
   How many is not given. On the C64 each region gets visits of up to 10
   rounds in turn ([creatures](creatures.md) 2.2). [research, capture] A
   player that walks into another region waits for the next cycle if
   that region's visit has already gone by in the running one, and keeps
   the moves it carried through the door meanwhile (creatures 2.2.4).
   [capture: mem, rr25, case reg2, capture.md 10.29] **Engine:** so.
7. Effective Speed is lowered by carrying too much and by standing next to an
   aggressive, unfriendly individual. [manual] By how much the manual
   does not give; capture has settled both (below; capture.md 10.39 item
   4, 10.41 item 2). **Engine:** the bar starts at Speed, halved first
   when hindered (below), then lowered by the weight slowdown
   ([inventory](inventory.md) 3.3, 3.4; creatures 2.6), set after the top-of-turn healing die
   from the Life Force it leaves (inventory 3.4; [capture: capture.md
   10.39 item 4]); adjacency halves the moves, rounding up, at the
   turn's start only, as in [creatures](creatures.md) 2.6 ([capture:
   capture.md 10.41 item 2]), except moves carried through a door
   into another room (2.3, creatures 2.2.4), which are not halved, as on
   the C64. [capture: mem, rr25, capture.md 10.29]
8. Speed 0: the manual only says such a creature does not show its name on its
   turn. A character with Speed 0 cannot move. On the C64 a player set to
   Speed 0 began its next turn with 0 moves and the menu opened by
   itself; the stick moved it nowhere and the turn did not end on its
   own (20M instructions); MOVE chosen from that menu showed a message
   that it can't move, perhaps carrying too much, and the turn still went
   on. [capture: rr40 `sp0.txt`, `sp0b.txt`, from rr37's `hp-room.vsf`,
   one run each, capture.md 10.44 item 8; ours, no research pass,
   2026-11-16] **Engine:** its moves are refused (`NoMovesLeft`) and only
   `Wait` ends its turn. Opening the menu at once and the "can't move"
   message are the front end's to show.

## 3. Directions

1. Players move with the joystick, one square per step. [manual]
2. Whether diagonal steps are allowed is not stated. [manual] On the C64
   they are not, on the world map or in rooms: a diagonal push does nothing
   at all (no move, no bar cost) in 80 of 80 tries in each place, while
   straight pushes always move one square. [capture: move-basic and
   move-room, 20 runs each] **Engine:** diagonal steps are refused
   (`BlockReason::Diagonal`, no cost); `Rules::diagonals` turns them on.

## 4. The world map

1. The map is 40 x 40 cells; 10 x 15 are visible at once. [manual]
   **Engine:** the size comes from the map's own tiles, so Extended
   adventures can be larger (`acs_core::limits`).
2. **Wraparound.** If the adventure allows it, stepping off one edge comes in
   at the opposite edge (a torus). Otherwise the edge stops the move like
   impassable terrain. [manual] 1 of 15 corpus adventures wraps. [data]
3. **Terrain passability** (one of four per terrain type): [manual]
   - *impassable*: the move is refused;
   - *open*: anyone may enter;
   - *open only to owners of* a thing: entered only by a character who owns it.
     The needed thing is a treasure or a magic spell in all 19 corpus uses.
     [data] Whether non-owners get a message: terrain has no message slot, so
     no. [data] **Engine:** blocked silently;
   - *triggers a spell*: anyone may enter, and the thing's spell runs. The one
     corpus use names a magic spell. [data] Whether the spell runs before or
     after the move: [unknown] **Engine:** after the character arrives.
     Capture can't settle it: the scenario builder writes no such
     terrain; it needs the editor (capture.md 10.44).
4. **Map doors.** Terrain types 13-16 (stored 12-15) are one-way doors,
   whatever their name or picture. Up to 32 doors, each at a map point with a
   destination: a room square or another map point. [manual] In the corpus
   every door-terrain cell has a door at it, and all 60 door terrains are
   *open*. One door record sits on a cell painted over with ordinary terrain.
   [data] **Engine:** stepping onto a door-terrain cell that has a door takes
   the door; a door record on ordinary terrain is ignored; door terrain with
   no door record behaves as its passability says.
5. **Exit points.** 32 more map points mark where one-way room doors come out
   onto the map. The editor lets designers paint anything over them,
   impassable terrain included. [manual] 4 of 51 used exit points are on
   impassable terrain. [data] **Engine:** arriving ignores the terrain, so a
   character can be stranded, as the manual warns.

## 5. Rooms

1. A room is up to 15 x 10 squares [manual]; its stored size includes the
   wall line. Edge squares are walls, the inside is floor. [data]
2. A plain wall square refuses the move. [data] Things placed on edge squares
   replace the wall there; only portals, obstacles and custom obstacles are
   ever placed on walls. [data]
3. **Stacks.** Several things may share a square; the one placed last is on
   top and acts first. [manual] The decoded list stores the top first.
   [data] **Engine:** only the top thing decides whether the move succeeds
   and what it triggers. *Activate All Things* on the top runs the rest; that
   belongs to the magic rules (section 9).
4. A **resident creature** on the square refuses the move. With a readied
   melee weapon the move starts a fight instead. [manual] On the C64 any
   individual on the square, another player too, makes the move an attack
   when the player holds or owns a melee weapon, and owning none nothing
   happens and no move is used ([combat](combat.md) 2.7). [research,
   capture: combat 2.7, capture.md 10.14.10, 10.24] **Engine:** so: an attack (`Attacked`), or `BumpedCreature`
   / `BumpedPlayer` at no cost.
5. What the top thing does when moved onto:

   | Top thing | Move | Triggers | Source |
   | --- | --- | --- | --- |
   | treasure, magic item, weapon, armor, magic spell | stays; the thing is picked up (one step), or refused at the carry limit ([inventory](inventory.md) 2) | a magic item's pick-up spell | [manual]; spells are placed 31 times [data] |
   | portal | takes the door if access allows, else refused | its spell if access is "invoke spell" | [manual] |
   | space | moves if access allows, else refused | its spell if access is "invoke spell" | [manual] |
   | custom space, "when moved onto" | moves | its spell | [manual] |
   | custom space, "open to (non-)owners of" | moves if the condition holds, else refused | its spell when the character passes | [manual] ("Let There be Light") |
   | custom space, "when dropped here" | moves | nothing on entry | [manual] |
   | obstacle | refused | its spell if "bump invokes spell" | [manual] |
   | custom obstacle | refused | its spell only if bit 6 of its second property byte is set; the parameter from the placement or the master (below) | [manual]; [capture] |
   | store | moves (one step); the store opens with the player on its square ([stores](stores.md) 3.2) | the store screen | [research; capture] |
   | floor / undefined / unknown type | moves | nothing | [data] (never placed) |

   On the C64 these trigger only for a step or a bump: a space that gives
   a thing gave it when the player stepped off it and back on, and an
   obstacle that gives a thing when bumped gave it at a push into it, the
   player staying put; a space barring a non-owner refused it at no cost
   (2.2). Arriving on them by a door triggers nothing (6.3). [capture:
   rr36 `arsp-ew`, `arob-ew`, `bar`, capture.md 10.40 item 5; ours, no
   research pass, 2026-11-16] Creatures never step onto a space, a plain
   one open to anyone and a custom one whose condition they meet included
   ([creatures](creatures.md) 5.1). [capture: mem, rr37 `cs-m`,
   capture.md 10.41 item 3; ours, no research pass, 2026-11-16] A
   creature that followed its goal through a door onto an obstacle's
   square can bump that obstacle later in the same turn, after stepping
   off; from beside the square on any other turn it never does
   (creatures 5.1). [capture: mem, rr40 `bump-m`, 8 of 231 such turns,
   none of 586 others, capture.md 10.44 item 1; rr37 `ad-ob-m`, 10.41
   item 3]

   **A custom obstacle's spell and parameter.** The manual has a custom
   obstacle run its spell on every bump with the parameter chosen at
   placement. On the C64 two bits decide instead (format notes, Things):
   - a bump runs the spell only when **bit 6 of the second property
     byte** is set, the same bit that makes a plain obstacle's bump invoke
     its spell; with it clear a bump is refused and nothing else happens.
     Under *Activate All* the spell runs either way ([magic](magic.md)
     3.14);
   - the parameter is the **placement's** when bit 7 of the first property
     byte is set, and otherwise the **master's fourth property byte**,
     whatever the placement holds: the choice a custom space makes (5.8).
     A placement of 21 with bit 7 set gave thing 21; with bit 7 clear,
     masters holding 23 and 25 gave 23 and 25 whatever was placed.

   [capture: rr41 `v2`, `v2b`, `v2c`, `v2d`, capture.md 10.45 item 1;
   ours, no research pass, 2026-11-17] **Engine:** so, for a bump by a
   player or a creature; under Activate All the parameter is chosen the
   same way. (Superseded: the engine's earlier choice of a spell on every
   bump with the placement's parameter.)

6. **Access rules** for portals and spaces: anyone; invoke a spell (anyone
   passes and the spell runs); open only to owners of a thing; open only to
   non-owners of a thing. When passage is refused, the thing's blocked
   message (if set and shown) appears. [manual]
7. **Destroy needed thing.** A portal or custom space set this way removes one
   of the needed thing from the character who passes. [manual] On the C64
   a player who passed such a locked door was refused by it the next
   time; without the setting it passed again. [capture: acts, table lock,
   5 runs each] Only "owners of" makes sense here. [unknown] **Engine:**
   applied only to "owners of" access. (Reachable, not yet run: a portal
   of ours set so with non-owners access; capture.md 10.44.) An "owners
   of" custom space set so takes the thing when an *Activate All* above
   it runs it too, exactly as on a direct step onto it ([magic](magic.md)
   3.14). [capture: rr41 `n1`, capture.md 10.45 item 2; ours, no research
   pass, 2026-11-17]
8. **Custom spaces** choose one of two values at placement: either the needed
   thing (then the spell's parameter is fixed in the master list) or the
   spell's parameter (then the needed thing is fixed). Reading the placement
   parameter that way, every needed thing in the corpus (120 placements with
   an owner or drop trigger) is a carriable thing. [data] Bit 7 of the
   first property byte makes the choice: set, the placement holds the
   parameter. Custom obstacles read the same bit the same way (5.5).
   [capture: rr41 `v1b-ewe`, `v2d`, capture.md 10.45 item 1]
9. **Things that disappear.** An obstacle can be set to vanish the first
   time it is bumped. [manual] Bit 4 of the first property byte is the usual
   "disappears" flag; 85 of 171 corpus obstacles set it, 84 of those also
   invoke a spell on bump. [data] On the C64 the bit works on spaces,
   custom spaces and custom obstacles too. [capture: rr41 `v1`, `v1b`,
   `v2`, `v2b`, `v2d`, `v3`, `v4`, capture.md 10.45 item 1; ours, no
   research pass, 2026-11-17]
   - A **space** with the bit (a spell space, or a plain one open to
     anyone with no spell) goes at the step onto it, after its spell; the
     player stands on its square.
   - A **custom space** with the bit goes at the step onto it, after its
     spell: a "when moved onto" one, and an "open to owners of" one when
     the stepper owns the thing (with either placement choice, 5.8). One
     whose condition fails refuses the step (5.6) and **stays**, however
     often it is tried.
   - A plain **obstacle** with the bit goes at the first bump, after its
     spell if it has one.
   - A **custom obstacle** with the bit goes at the first bump **whether
     or not its spell ran** (5.5). Without the bit it stays, bump after
     bump.
   - Once it is gone the square shows the thing below, and that thing acts
     at the next step or bump in the usual way (the player stepping off
     and back on, or stepping onto a square that is now open).
   - Under *Activate All* every thing with the bit whose spell the stack
     runs goes in the same step: a spell space, a "when moved onto" and an
     "owners of" custom space, an obstacle and a custom obstacle all went
     at once, and the next step ran only what was left ([magic](magic.md)
     3.14).
   - A thing that went and is put back by *Add to Room* ([magic](magic.md)
     3.13) works again and goes again: a spell space with the bit over a
     space adding it gave its thing, went, came back on top at the next
     step, gave its thing again at the third and went again; the same
     with an *Activate All* between them.

   A thing that goes leaves the room's list as a take does. **Engine:**
   so: after its spell, a space or custom space with the bit is removed
   from the square at the step onto it, a custom space only when its
   condition held; an obstacle or custom obstacle with the bit at the
   bump, the spell run or not; under Activate All each thing with the bit
   whose spell runs is removed (`ObstacleVanished`, or an event of the
   same kind for spaces). Not tried [unknown]: a portal with the bit, an
   *Activate All* space with the bit itself, and an "owners of" space in
   an Activate All stack whose condition fails; **Engine:** a portal
   ignores the bit, an Activate All thing with the bit goes as a spell
   space does, and a space whose condition failed stays. (Superseded: the
   engine's earlier choice of plain obstacles only.)
10. **Picking up** happens by moving onto the thing. A character who cannot
    carry more of a thing gets a message instead, and the move is refused.
    [manual] On the C64 the character takes the thing but stays on its own
    square. [capture: weight] Limits, gold piles and readying are
    [inventory](inventory.md) rules (section 2).

## 6. Room doors

1. A portal's placement parameter picks an entry in the region's door table,
   which names the destination region and room and the square inside it, or a
   world map point. [data]
2. **Two-way doors** can only sit on a wall shared with a neighbouring room;
   **one-way doors** can lead anywhere in the region, to another region, or to
   the world map. [manual] Both are taken the same way: step on the door,
   arrive at the destination square. [manual] 2,210 two-way and 824 one-way
   portals are placed. [data]
3. Every one of the 2,981 corpus door destinations inside a live room is an
   inside (non-wall) square. 2,600 are empty; the rest hold a space (255),
   an obstacle (54), a custom space (49), a portal (14, all from one-way
   doors) or another thing. [data] Whether arriving on such a square triggers
   it is not stated. On the C64 it does not: a player arriving by the map
   door onto a space that gives a thing, an obstacle that gives one when
   bumped, a portal, or a custom space open only to owners of a thing it
   did not own stood on the square each time, was given nothing and took
   no door; stepping off and back, or bumping from beside, then set them
   off as usual (5.5). [capture: rr36 `arsp`, `arob`, `ardo`, `arcs`,
   capture.md 10.40 item 5; ours, no research pass, 2026-11-16] Arriving
   by a room door is the same, for a player and a creature: through a
   portal onto a square holding a space that invokes a spell (Life Force
   -5), an obstacle that bumps for -5, or a custom space open only to
   owners of a thing (giving +5) that it did not own, a Speed 12 player
   stood on the square with its Life Force unchanged each time, and a
   friend of Speed 6 that followed it through landed there with its Life
   Force unchanged in 6 of 6 runs, then stepped off later. [capture: mem,
   rr37 `ad-sp`, `ad-ob`, `ad-cs`, `arrive.txt`, capture.md 10.41 item 4;
   ours, no research pass, 2026-11-16] **Engine:** arriving triggers nothing
   and never chains to another door, by any door; the character just
   stands there, on an obstacle's square too. If an
   individual already stands on the destination square, the character
   arrives there all the same, sharing it, as creatures do: on the C64 a
   player came through a portal onto a creature of ours standing on its
   destination, in 6 of 6 runs. [capture: capture.md 10.10.5,
   [creatures](creatures.md) 5.4]
   The corpus smoke test (`crates/acs-engine/tests/corpus.rs`) steps onto
   every door that is on top of its stack: 3,025 lead to an inside square of
   a live room or a map cell, 1 sends the player to the start (the deleted
   room above), 4 have a resident standing on them; 47 doors are covered by
   another thing.
4. **Door into a deleted room.** Creatures passing are banished; players are
   placed on the world map at the adventure's start. [manual] One corpus
   door does this. [data]
5. A door to a room slot outside the region's 16 rooms, or to a missing
   region, cannot be built by the editor. [data: no corpus case]
   **Engine:** treated like a deleted room, with a diagnostic event.
6. Entering or leaving a region saves the game automatically; after a death
   play resumes from the last save. [manual] On the C64 the save is not
   made at the door step but when the scheduler next begins a visit in a
   different region from the visit before it, with that visit's first
   round on the counter; visits of one region one after another save
   nothing, and a SAVE GAME from the menu or a Retire with Save saves at
   its round's end. The latest of these is restored
   ([creatures](creatures.md) 2.2.7). [capture: rr21, rr24, capture.md
   10.25, 10.28] **Engine:** the same; the save point
   is kept in the save file (`docs/saves.md` 1.4). `EnteredRegion` /
   `LeftRegion` still report the door step but no longer mark the save.

## 7. Random encounters

1. **World map.** Before every move on the map there is a chance (0..60% per
   creature) that one of the map's random creatures appears, if the
   character is in that creature's terrain. The character is then pulled into
   a special encounter area whose border is that terrain. [manual] On the
   C64 the roll follows each completed step on the open map, by the terrain
   entered, one roll for each of the map's records on it, every hit
   appearing; refused moves and map doors take none. The area is one of
   four 15 x 10 rooms of the map's region with one door out. All of it,
   with the engine's choices: [encounters](encounters.md). [research;
   capture: capture.md 10.19]
2. **Rooms.** On entering a room there is one roll against the room's chance
   (0..75%) for its random creature. It is placed only on a square with no
   thing on it. [manual]
   - Which empty square: a random one of the room's inside, never a
     player's square ([creatures](creatures.md) 3.2). [research; capture:
     rr36 `rc-sw`, capture.md 10.40] **Engine:** creatures 3.2 (the
     `RoomEncounter` event gives the slot and square).
   - No more than 16 creatures in a region at once; then no random creatures
     appear. [manual] **Engine:** counts in-use residents; when every slot
     is in use (16 in Classic) the roll is skipped. Extended adventures
     also skip it when the room already holds `creatures_per_room`.
   - Random creatures in rooms no player occupies are removed now and then.
     [manual] How often: a roll at each round's end, one percentile per
     random-marked slot ([creatures](creatures.md) 2.4). [research;
     capture]
3. All rolls use the game's one seeded generator, so a game replays exactly
   from its seed and command log. [design]

## 8. Creatures and movement

These need the AI and are listed so the movement code leaves room for them.

1. Residents get no turn until a player enters their room; the player goes
   first, and a creature still gets its turn after the player leaves, so it
   can follow through a door. [manual]
2. Creatures never step onto spaces or obstacles, custom ones included. They
   take doors only within their own region, and only doors without a do-all,
   rid-room or add-to-room spell. [manual] Spaces: [capture, capture.md
   10.41 item 3] ([creatures](creatures.md) 5.1, with its one exception, a
   bump at the goal's square on the turn after a shared door, capture.md
   10.44 item 1).
3. A creature sent through a door into a deleted room is banished. [manual]

## 9. What later rules need from movement

- **Combat:** done in [combat.md](combat.md): melee (moving into an
  individual attacks it, 5.4 here), firing (combat 8) and death
  ([magic](magic.md) 7).
- **Magic:** done in [magic.md](magic.md): every spell found here (terrain,
  portals, spaces, custom spaces, obstacles, picked-up items) now takes
  effect, including *Activate All Things* running the rest of the stack top
  to bottom, messages before effects, and summons made at once on the
  square where they take effect.
- **Inventory:** done in [inventory.md](inventory.md): pick-up limits,
  weight and capacity slowing the bar, dropping (also onto "when dropped
  here" custom spaces and on the map).
- **AI:** done in [creatures.md](creatures.md): turn order with
  creatures, creatures following through doors, creature door rules,
  random creature placement and periodic removal, the 16-creature region
  limit.
- **Stores:** done in [stores.md](stores.md) (an ordinary step onto the
  store opens its screen, stores 3.2).
- **Saves:** the save point of 6.6 (creatures 2.2.7), resume after death;
  save files in [saves.md](../saves.md).

## Captures needed

In priority order, for the behaviour-capture harness:

1. Diagonal moves allowed? No, on the map and in rooms (captured).
2. Bar cost of a refused door/space (captured: walls, obstacles and a
   locked door cost nothing; taking a door, map or room, costs one step;
   a space that bars passage costs nothing, capture.md 10.40).
3. Arriving on a space, obstacle or door: captured by the map door (6.3:
   nothing triggers; capture.md 10.40) and by a room door, for a player
   and a creature, and creatures never stepping onto a space (5.5,
   capture.md 10.41). Still open: arriving by a room door on a portal.
4. Map encounter roll and area: researched and captured
   ([encounters](encounters.md), capture.md 10.20, 10.26, 10.34, 10.36);
   its open points are listed there (section 8).
5. Room random creature: square choice researched and swept (creatures
   3.2, capture.md 10.40: random over the inside, never the player's).
   Still open: the square of each random state, draw by draw.
6. Real-time drain rate of the bar (captured: about 354 frames per point of
   Speed); extra turns for players in other regions (captured: 2.6,
   creatures 2.2, capture.md 10.29).
7. Stores: bump or step on (captured: a step on, stores 3.2).
8. Moves carried through a door into another room (2.3): captured (not
   halved beside an enemy, [creatures](creatures.md) 2.6; the idle healing
   roll keeps the door turn's range, [magic](magic.md) 5.3.1; room doors
   and region to region carry them too, creatures 2.2.4; capture.md
   10.29).
9. A part-drained step (2.4: walkable) and a player of Speed 0 (2.8):
   captured (capture.md 10.44). Still open: destroy needed thing with
   non-owners access (5.7; reachable, not yet run); terrain that
   triggers a spell, before or after the move (4.3; needs the editor).
10. Things that disappear (5.9), a custom obstacle's spell bit and
    parameter (5.5) and a needed thing used up under Activate All (5.7):
    captured (capture.md 10.45). Still open: a portal or an Activate All
    thing with bit 4, and a failing conditional space with it under
    Activate All.
