# Rules: movement

Behaviour spec for moving characters around the world map and rooms. Licensed
CC BY 4.0. The engine implements it in `crates/acs-engine` (`step.rs`).

Every rule is tagged with where it comes from:

- **[manual]**: paraphrased from the *Adventure Construction Set* manual
  (EA, 1984; private OCR copy, never quoted).
- **[data]**: follows from the stored format or from corpus counts (the 14
  Jadeworld C64 adventures plus EA's PC *Rivers of Light*, see
  `docs/format-notes.md` and `docs/provenance.md`).
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

## 2. Turns and the movement bar

1. On its turn a character has a **movement bar** that starts at its Speed
   (0..15) and drops by one for every square moved. The turn ends when it
   reaches zero. [manual]
2. A move that is refused (impassable terrain, an obstacle, a wall) leaves
   the bar unchanged. [manual] Being refused by a door or space that bars
   passage is not described. [unknown] **Engine:** no cost, as for obstacles.
3. Taking a door costs the one step onto the door square. [unknown]
   **Engine:** one step; the character then stands on the destination square.
4. The bar also drains slowly in real time while the player does nothing.
   [manual] The rate is not given. [unknown] **Engine:** no real-time drain in
   the rules; a front end may send `Wait` to end the turn, and later a timer
   option can do so on a clock.
5. With one character and no creatures in the room, the same character moves
   again when the bar refills. With several players, or creatures in the room,
   each gets a turn first. [manual] **Engine:** players take turns in order;
   creatures get no turns until the AI exists.
6. Players exploring different regions get several turns in a row. [manual]
   How many is not given. [unknown] **Engine:** one turn each, for now.
7. Effective Speed is lowered by carrying too much and by standing next to an
   aggressive, unfriendly individual. [manual] By how much is not given.
   [unknown] **Engine:** the bar starts at Speed after the weight slowdown
   ([inventory](inventory.md) 3.3); adjacency arrives with combat.
8. Speed 0: the manual only says such a creature does not show its name on its
   turn. A character with Speed 0 cannot move. [unknown] **Engine:** its
   moves are refused (`NoMovesLeft`) and only `Wait` ends its turn.

## 3. Directions

1. Players move with the joystick, one square per step. [manual]
2. Whether diagonal steps are allowed is not stated. Missile ranges count
   diagonals, and an 8-way joystick makes diagonals easy to send. [unknown]
   **Engine:** 8 directions, switchable off with `Rules::diagonals`.
   Needs a capture.

## 4. The world map

1. The map is 40 x 40 cells; 10 x 15 are visible at once. [manual]
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
   melee weapon the move starts a fight instead. [manual] **Engine:** refused
   with a `BumpedCreature` event; combat comes later.
5. What the top thing does when moved onto:

   | Top thing | Move | Triggers | Source |
   | --- | --- | --- | --- |
   | treasure, magic item, weapon, armor, magic spell | moves; the thing is picked up, or refused at the carry limit ([inventory](inventory.md) 2) | a magic item's pick-up spell | [manual]; spells are placed 31 times [data] |
   | portal | takes the door if access allows, else refused | its spell if access is "invoke spell" | [manual] |
   | space | moves if access allows, else refused | its spell if access is "invoke spell" | [manual] |
   | custom space, "when moved onto" | moves | its spell | [manual] |
   | custom space, "open to (non-)owners of" | moves if the condition holds, else refused | its spell when the character passes | [manual] ("Let There be Light") |
   | custom space, "when dropped here" | moves | nothing on entry | [manual] |
   | obstacle | refused | its spell if "bump invokes spell" | [manual] |
   | custom obstacle | refused | its spell, parameter from the placement | [manual] |
   | store | refused | opens the store (later) | [unknown] |
   | floor / undefined / unknown type | moves | nothing | [data] (never placed) |

6. **Access rules** for portals and spaces: anyone; invoke a spell (anyone
   passes and the spell runs); open only to owners of a thing; open only to
   non-owners of a thing. When passage is refused, the thing's blocked
   message (if set and shown) appears. [manual]
7. **Destroy needed thing.** A portal or custom space set this way removes one
   of the needed thing from the character who passes. [manual] Only "owners
   of" makes sense here. [unknown] **Engine:** applied only to "owners of"
   access.
8. **Custom spaces** choose one of two values at placement: either the needed
   thing (then the spell's parameter is fixed in the master list) or the
   spell's parameter (then the needed thing is fixed). Reading the placement
   parameter that way, every needed thing in the corpus (120 placements with
   an owner or drop trigger) is a carriable thing. [data]
9. **Disappearing obstacles.** An obstacle can be set to vanish the first time
   it is bumped. [manual] Bit 4 of the first property byte is the usual
   "disappears" flag; 85 of 171 corpus obstacles set it, 84 of those also
   invoke a spell on bump. [data] **Engine:** a bumped obstacle with the bit
   set is removed from the square.
10. **Picking up** happens by moving onto the thing. A character who cannot
    carry more of a thing gets a message instead, and the move is refused.
    [manual] Limits, gold piles and readying are [inventory](inventory.md)
    rules (section 2).

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
   it is not stated. [unknown] **Engine:** arriving triggers nothing and
   never chains to another door; the character just stands there.
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
   play resumes from the last save. [manual] **Engine:** emits
   `EnteredRegion` / `LeftRegion` so the front end can snapshot the state.

## 7. Random encounters

1. **World map.** Before every move on the map there is a chance (0..60% per
   creature) that one of the map's random creatures appears, if the
   character is in that creature's terrain. The character is then pulled into
   a special encounter area whose border is that terrain. [manual]
   - Whether the roll happens before a refused move too: [unknown]
     **Engine:** every move command on the map rolls, refused or not.
   - Which terrain counts, the one left or the one entered: [unknown]
     **Engine:** the cell the character stands on before the move.
   - Several creatures for the same terrain: [unknown] **Engine:** rolled in
     list order; the first hit wins.
   - The encounter area itself (size, layout) is [unknown]. **Engine:** for
     now only the roll: an `Encounter` event, after which the move goes ahead
     as if nothing appeared, until encounter areas exist.
2. **Rooms.** On entering a room there is one roll against the room's chance
   (0..75%) for its random creature. It is placed only on a square with no
   thing on it. [manual]
   - Which empty square: [unknown] **Engine:** roll only (`RoomEncounter`
     event); placement waits for the AI session.
   - No more than 16 creatures in a region at once; then no random creatures
     appear. [manual] **Engine:** counts in-use residents; at 16 the roll is
     skipped.
   - Random creatures in rooms no player occupies are removed now and then.
     [manual] How often is [unknown].
3. All rolls use the game's one seeded generator, so a game replays exactly
   from its seed and command log. [design]

## 8. Creatures and movement

These need the AI and are listed so the movement code leaves room for them.

1. Residents get no turn until a player enters their room; the player goes
   first, and a creature still gets its turn after the player leaves, so it
   can follow through a door. [manual]
2. Creatures never step onto spaces or obstacles, custom ones included. They
   take doors only within their own region, and only doors without a do-all,
   rid-room or add-to-room spell. [manual]
3. A creature sent through a door into a deleted room is banished. [manual]

## 9. What later rules need from movement

- **Combat:** bumping a creature (event `BumpedCreature`) starts melee when a
  melee weapon is readied; adjacency to aggressive unfriendlies shortens the
  bar; missile range counts squares including diagonals and stops at
  obstacles. Death on the map loses carried things.
- **Magic:** every `SpellTriggered` event (from terrain, portals, spaces,
  custom spaces, obstacles, picked-up items) carries the thing and the
  resolved action and parameter. The spell rules decide the effect, including
  *Activate All Things* running the rest of the stack top to bottom, messages
  before effects, and summons arriving next turn.
- **Inventory:** done in [inventory.md](inventory.md): pick-up limits,
  weight and capacity slowing the bar, dropping (also onto "when dropped
  here" custom spaces and on the map).
- **AI:** turn order with creatures, creatures following through doors,
  creature door rules, random creature placement on an empty square and
  periodic removal, the 16-creature region limit.
- **Stores:** what bumping or entering a store square does.
- **Saves:** snapshots at `EnteredRegion` / `LeftRegion`, resume after death.

## Captures needed

In priority order, for the behaviour-capture harness:

1. Diagonal moves allowed?
2. Bar cost of a refused door/space, and of taking a door.
3. Arriving on a space, obstacle or door: does it trigger?
4. Map encounter roll: before refused moves? Which terrain? Order among
   creatures? Shape of the encounter area.
5. Room random creature: square choice.
6. Real-time drain rate of the bar; extra turns for players in other regions.
7. Stores: bump or step on.
