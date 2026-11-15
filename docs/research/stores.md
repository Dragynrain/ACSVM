# Research brief: stores

A request to the project's clean-room researcher (docs/research/README.md).
Licensed CC BY 4.0. Status: first pass done 2026-11-13; in the spec and the engine.
Pass 2 briefed and done 2026-11-14: the hang was the emulated drive,
the disappearing bow joins the stock (confirmed by capture, `rr17`); no
store on the open map (unconfirmed). No pass 3 for now (end of
"Findings").

## Why

Stores are the last rule area of Phase 4 (design.md) before the engine
can play a whole adventure's economy. Capture has settled the prices
(`docs/rules/stores.md` 4 and 5, `docs/capture.md` 10.17): buying costs
the full value at any Wisdom, and every one of 1,152 sell-back offers fits
one rule. What capture can't settle cheaply is the store screen itself:
which squares hold which thing, which are floor, and how a player gets
out. Several walks on that screen fitted no simple pattern, and each new
case would need a new disk (docs/research/README.md 2: "when to stop
capturing").

## What we ask

For the C64 version, in plain language:

1. **The store screen's layout.** After bumping the store, the program
   shows a screen without walls where the stock is drawn as things, the
   player among them. In our case (the store on room square 5,4, bumped
   from 4,4 by walking east) the player stood on 7,5, counting the room's
   squares as for the room, with things 1 to 3 in stock drawn on 6,4,
   5,6 and 6,6, and things 13 and 14, once sold to the store, on 5,3 and
   6,8. Bumping some empty squares, the store's own square among them,
   and the edges says the item is out of stock. Where does each thing
   1..80 go, which squares are floor, and does it depend on the store's
   square, the room, or where the player came from?
2. **Leaving the store.** We found no way out: the stick moves the
   player, every key we tried and the button at the browse prompt do
   nothing, and 60 million instructions of waiting changed nothing (the
   movement bar didn't drain). How does a player leave, where does it
   come out, and what does the visit cost in moves or turns?
3. **Buying, the rest.** Is a purchase refused when the player already
   owns as many of the thing as allowed? Is a weapon bought readied, and
   does a magic item set to act when picked up act? Where does a bought
   thing go when the player's possessions can't hold it?
4. **Selling, the rest.** Our runs: the button offers the player's
   treasures, weapons, armor and magic items one at a time in thing order,
   never its spells. Is a held weapon or worn armor offered? What happens
   to a thing sold when the stock already has as many as its limit, or
   for things above 80 (the stock is a bitmap over things 1..80)?
5. **The sell-back rule above a value of 2,000** (stores.md 5.1): the
   store's cut stops at 1,334 and the Wisdom bonus at 666 x Wisdom / 25.
   Is that how the program computes it, and what does it give for values
   our disks can't hold (2,001 to 2,019)?
6. **Stock over time.** Is a region's stock kept as it is across visits,
   saves and restores? Is anything ever restocked?

Enough for a first pass: 1 and 2.

## How the answers come back

As docs/research/README.md 3 and 4: `findings.md` in plain language only.
Addresses only for state the harness must read to check a store visit (the
stock in memory, the store screen's state, the square the player stands
on there), if capture.md 9 doesn't list them.

## Inputs

In `ACSVM-research/research/stores/inputs/` (copies; made by
`capture/runs/rr10/`, capture.md 10.17). VICE 3.10 `x64sc` snapshots saved
with their disks; start VICE with `-controlport2device 37` (capture.md
2.2). The scenario is ours: the open field, the map door at 12,16 into a
15 x 10 room (entry 4,4). The things are treasures of ours (weight 1, the
door picture): in stock things 1 (WARE1, value 10), 2 (WARE2, value 50,
count 2) and 3 (WARE3, value 200); the player owns things 13 (GOOD1, value
100) and 14 (GOOD2, value 40). The store is thing 83 on room square 5,4.

| File | What |
| --- | --- |
| `look-disk.d64` | the scenario disk |
| `look-room.vsf`, `.png` | T (Speed 6, Wisdom 13, gold 500) on 4,4 in its first turn in the room, the store east of it |
| `look-store-open.vsf`, `.png` | after pushing east into the store: the store screen, "T has 500 to spend", the button to sell, the stick to continue |
| `look-browse.vsf`, `.png` | after a push north: the browse prompt, T two squares north of where it appeared |
| `look-sold.vsf`, `.png` | after selling both goods (gold 569): two more things on the shelves |
| `shots/` | walks to the store screen's four ends (each ends in "out of stock"), a purchase offer (a diagonal push south-west from `look-browse`), an out-of-stock bump |
| `runs/sell*-offers.jsonl` | the 1,152 sell-back offers (Wisdom 0..31 by 36 values from 1 to 5,100), one JSON line each, all fitting stores.md 5.1 |

The player's Wisdom is `$F5B0` and its gold `$F5B1`-`$F5B2` (capture.md 9).

## Pass 2

Briefed 2026-11-14. The first pass is in the spec (`docs/rules/stores.md`)
and the engine; capture confirmed restocking from a broken mail, a broken
bow and a disappearing blade (`rr12`, capture.md 10.17). Two things are
left that capture can't reach: a region that hangs while loading, and
stores while the world map is the visited region. Inputs in
`inputs/pass2/` (table at the end of this section). Read
`docs/rules/stores.md` 2 and 3 and capture.md 10.17 (the `rr12` part)
and 10.19; capture.md 9 has the addresses already in use.

Not asked, because capture settles it: whether a region's stock is kept
when the player leaves and comes back, and in the adventure file the
original saves on entering and leaving a region (movement 6.6). We can buy,
walk out to the map and back, and read the region's stock off the disk
image and the shelves.

### What we ask

1. **The region that never loads.** Our combat scenario with a creature
   (EE, on 9,4) holding our bow set to disappear after use, and a store
   on 2,2 with no stock, stops on "THE ACTION MOVES TO TEST REGION" when
   T steps onto the map door, and is still there 33 million cycles
   later. The same disk without the store loads, and so does the same
   disk with the bow breakable (break chance 15) instead of
   disappearing. The two store disks differ only in the bow's break
   chance and its disappear setting (`acs dump`); with the store on 2,7
   it hung as well. Already seen: in the hanging case the disk is never
   read ("ACCESSING DISK" never shows, `bow-once-entering-3m.png`), in
   both others it is within 3 million cycles. What is the program doing
   when it hangs, and what rule in plain terms makes a store and a
   disappearing missile weapon in one region hang it (the weapon's
   owner, its square, the store's stock, the thing's number 41)? Would it
   hang the same way for a designer's adventure, and is there a setup with
   both in one region that loads, so a capture can watch a disappearing
   bow join the stock (stores.md 2.5)?
2. **Stores on the world map.** The first pass traced that buying and
   selling never change the stock while the visited region is the world
   map (region 0; stores.md 2.7). Can a player meet a store then: a store
   thing on a map square, in one of the four encounter areas (rooms 1..4
   of map region 0, encounters.md), or anywhere else? If so, what stock
   does it show and offer (whose record holds it), does the store screen
   and leaving work as in a room (stores.md 3), and where does the player
   come out? If not, say what stops it (the editor, the map's format, the
   program).

Enough for this pass: 1. Item 2 may be "not reachable" with the reason.

### How the answers come back

As the first pass: `findings.md` in plain language only, appended as a
"Pass 2" part. Addresses only for state a capture must set or read to
check an answer (for item 1, anything that tells a hang from a slow load;
for item 2, where a map store's stock is kept, if it has one).

### Inputs (pass 2)

In `ACSVM-research/research/stores/inputs/pass2/` (copies; made by
`capture/runs/rr12/` and `rr16/`, capture.md 10.15 and 10.17). VICE 3.10
`x64sc` snapshots saved with their disks; start VICE with
`-controlport2device 37` (capture.md 2.2). Each case is the combat kit's
scenario (`cases.txt` has the exact scenario keys): the open field, T
(Speed 6) on the map, the map door on 12,16 into a 15 x 10 room, EE an
aggressive enemy on 9,4 holding our bow (thing 41, power 10, range 7).
`near.txt` is the script that made the snapshots.

| File | What |
| --- | --- |
| `bow-once-disk.d64` | the hanging case: the bow set to disappear after use, the store (thing 83) on room square 2,2, no stock |
| `bow-once-near.vsf`, `.png` | T on the map one square north of the door, its turn under way: one push south steps onto the door |
| `bow-once-entering.vsf`, `.png` | just after that push: "THE ACTION MOVES TO TEST REGION" |
| `bow-once-entering-3m.png` | 3 million cycles later, still that screen, no disk access |
| `bow-once-hung.vsf`, `.png` | the same case about 33 million cycles after the step (made by `rr12`, an earlier build of the same disk): still that screen |
| `bo-nostore-disk.d64`, `-near.vsf`, `-entering.vsf`, `.png`s | the same without the store: loads ("ACCESSING DISK" by 3 million cycles, then the room) |
| `bow-break-disk.d64`, `-near.vsf`, `-entering.vsf`, `.png`s | the store on 2,2 and the bow breakable (break chance 15), not disappearing: loads |
| `cases.txt`, `near.txt` | the cases' scenario keys and the snapshot script |

For item 2 there are no inputs of ours: the harness can't put a store on
the map. The first pass's `look` inputs show a store in a room, and the
encounters brief's inputs (`research/encounters/inputs/`, capture.md
10.19) a map region with its encounter areas.

## Findings

First pass, 2026-11-13 (`ACSVM-research/research/stores/findings.md`),
reviewed and written into `docs/rules/stores.md` (sections 2, 3, 4, 5).

- **The layout** is one fixed 15 x 10 map for every store, a shelf place
  for each thing 1..80, aisles in columns 1, 4, 7, 10, 13 and row 5; the
  player appears on 7,5. Confirmed by the five captured places and a
  walk onto the outer ring (capture.md 10.17, `rr11`).
- **Leaving**: the exits are 0,4 and 14,4, one row above the ends of the
  cross aisle (why our walks missed them). The bump is an ordinary step
  onto the store's square, no time passes inside, and the player comes
  out there with exactly one move left. Confirmed (`rr11`, both exits, and
  a bump with the last move).
- **Buying** at the limit takes the gold and the stock and loses the
  thing (confirmed). A bought weapon is not readied and nothing acts
  (traced, unconfirmed).
- **Selling** into a full stock pays and loses the thing (confirmed);
  held and worn things are offered, things above 80 never stock (traced).
- **The sell-back rule** is the captured one, for every value.
- **Stock over time**: broken and used-up weapons and armor go back into
  the region's stock; stock in region 0 never changes (traced,
  unconfirmed). Saving was not traced.

Next pass, when wanted: confirm restocking by capture (a weapon breaking
in a room with a store), a store on the world map, the stock across a
save and restore, a bought weapon, and a store in a room of another
size. Each needs a "Pass 2" section here with its inputs.

Pass 2, 2026-11-14 (`findings.md`, its "Pass 2" part), reviewed,
confirmed by capture where the harness reaches (`rr17`, capture.md
10.21) and written into `docs/rules/stores.md` 2.5 and 2.7.

- **The region that never loaded** (item 1) was not the game. The
  emulated disk drive was dead in the snapshots the harness had saved for
  the two "store and disappearing bow" disks: its timer interrupt was
  never served, so the first disk read of the region change never ran.
  The same disk loads after a drive reset or when played from a fresh
  boot, and no rule makes a region hang. Confirmed (`rr17` `scan`,
  `reset`, `fresh`). The harness now checks drive 8 after every restore
  (`drivecheck`, capture.md 10.21); stores.md 2.5 keeps it as a harness
  note, nothing for the engine.
- **The disappearing bow joins the stock** (item 1's last question):
  the bow left its creature and joined the region's stock in the same
  moment or the next in 32 of 32 runs (`rr17` `rounds`). Confirmed;
  stores.md 2.5.
- **Stores on the world map** (item 2): not reachable in play. A step
  on the open map checks only for a door, and an encounter area holds
  only its door; nothing dropped there can be a store. Traced, not
  captured: the harness can't put a store on a map square or in an
  area's item list. In stores.md 2.7 as [research, unconfirmed]. The
  researcher's poked store in an area opened the ordinary screen,
  showed whatever the map's data holds where a room region's stock is
  kept, and never changed it; capture agrees as far as it reached (those
  bytes all set while the map was visited, in 4 scenarios, `rr17`
  `mapstock`, `mappeek`).
- **Saving the stock** (2.8) was left to capture (pass 2's "not asked")
  and is still [unknown]; the engine keeps and saves it with the region.

Open, and why none of it is a pass 3:

- Whether any corpus adventure has a thing in an encounter area's item
  list is a data question for our own tools, not the researcher's. Our
  model reads no items for an area; reading them is a model change
  (`MODEL_VERSION`), the maintainer's call (stores.md, Captures needed 1).
- What the map's data at the stock's place holds in a real adventure
  matters only for such a store, and is in the adventure file: a decoder
  question once area items are read.
- The moves left after leaving an area store: the store can only be
  there by poking memory, so it waits for an adventure that has one.
- Saving the stock across save and restore: capture (buy, save, restore,
  read the shelves), not research.

Next pass: none. Reopen if the corpus turns up a store in an encounter
area, or if capture of saving the stock fits no simple rule.
