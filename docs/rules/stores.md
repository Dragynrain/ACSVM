# Rules: stores

Behaviour spec for stores: where they stand, what they stock, buying,
selling back, and how Wisdom changes the deal. Licensed CC BY 4.0. The
engine implements it in `crates/acs-engine/src/stores.rs` (opened from
`step.rs`, where bumping the store steps onto it, [movement](movement.md)
5.5).

Tags as in [movement](movement.md): **[manual]** paraphrased from the
manual, **[data]** from the stored format or corpus counts, **[unknown]**
with the engine's choice as **Engine:**, **[recollection]** for the
project owner's memory of play, a lead for captures rather than a source,
**[research]** for the clean-room researcher's stores passes
(`docs/research/stores.md`: the first 2026-11-13, the second "pass 2",
2026-11-14) and **[capture]** where our own runs confirm it; "[research,
unconfirmed]" means no capture of ours has checked it yet.
Counts cover all 15 corpus adventures (222 regions).

## 1. The store thing

1. Store is one of the thing types. Only its picture can be set, and a
   master list holds only one. [manual] It is thing 83 in all 15 adventures,
   and no other thing has the store type. [data]
2. A designer places a store while editing a room's contents, at most one per
   region. [manual] 73 regions have one placed store and 149 have none; no
   region has two. [data]
3. ACS never adds a store when it finishes an adventure for the designer.
   [manual]

## 2. Stock

1. **What a store can stock.** When the store is placed, the designer picks
   its stock from the treasures, magic items, weapons and armor in the
   master list. [manual] Magic spells, gold and the non-carriable types can't
   be stocked. The 499 stock lines in the corpus are 256 magic items, 92
   melee weapons, 62 missile weapons, 60 armor and 29 treasures. [data]
2. **How many of each.** Up to the thing's own limit: one, or up to three
   (inventory 1.2). [manual] Counts in the corpus: 431 of 1, 17 of 2, 51 of
   3. [data]
3. **Stock belongs to the region, not the store square.** It is stored once
   per region, in the same bitmap as creature possessions, so only things
   1..80 can be stocked (format notes). [data]
4. **Stock with no store.** 5 regions have stock but no placed store
   (leftovers from a deleted store). [data] **Engine:** that stock can't be
   reached and is kept unchanged for round-tripping.
5. **Restocking.** When a weapon or armor breaks, a missile weapon breaks,
   or a weapon set to disappear after use goes, one of it leaves its owner
   (player or creature) and one joins the stock of the region where that
   happened, within the limits of 2.6 and for things 1..80 only; never on
   the world map (region 0). A magic item used up does not join it. That is
   the only restocking. [research; capture: rr12, a store in the room and
   the stock watched: the disappearing blade went from its creature in 13
   of 16 runs and joined the stock at once each time, never in the other
   3; broken mail (3 runs) and a broken bow (3 runs) joined it the same
   way] A missile weapon set to disappear joins it the same way: the bow
   left its creature and joined the stock in the same moment or the next
   in every run. [research, pass 2, 2026-11-14; capture: rr17 `rounds`,
   32 of 32 runs, capture.md 10.21] A magic item set to disappear after
   use, used once in a room with a store, leaves its owner and does not
   join the stock: neither its stock count nor the shelves changed.
   [capture: rr27 `item`, capture.md 10.31] **Engine:** as stated
   (`Restocked`).

   Harness note, not a rule: a region with a store and a disappearing bow
   once seemed never to finish loading ("the action moves to" stayed up).
   That was the emulated disk drive, dead in the snapshots saved for
   those two disks; after a drive reset, or played from a fresh boot,
   the same disk loads. No game rule makes a region hang, and the engine
   copies nothing from it. [research, pass 2; capture: rr17 `scan`,
   `reset`, `fresh`, capture.md 10.21]
6. **The limits.** A stock holds at most 3 of things 1..32 and 1 of things
   33..80, as possessions do (inventory 1.2); an addition past the limit is
   lost and a removal below none does nothing. [research; capture 5.3]
7. **Stores on the world map.** A player can't meet a store while the
   world map (region 0) is the visited region. A step on the open map
   checks only for a map door, never for a thing on the square, so a
   store can't open there. [research, pass 2, 2026-11-14, unconfirmed]
   **A store in an encounter area** ([encounters](encounters.md) 3.4) can
   come from the file: the world map block stores each area's items as a
   room item list ([format notes](../format-notes.md), the world map's
   tail), and a store written into one is drawn in the area and opens
   the ordinary store screen when stepped onto. [data; capture: a store
   on 7,3 of area 1, stepped onto from 8,3, the store screen with the
   player on 7,5, capture.md 10.26] No corpus adventure has one (every
   area's list holds only its door), and nothing in play can make one:
   closing an area deletes everything but the door, and nothing dropped
   there can be a store (the store is thing 83, possessions only 1..80).
   [data; research, pass 2] Leaving works as in a room (section 3): the
   player is back on the store's square in the area with **exactly one
   move** (3.7; five inside, one just after, a step then taken).
   [research, pass 2; capture: rr30 `store`, capture.md 10.34]

   **The shelves offer every thing a store can stock.** On the C64 every
   shelf place of a stockable thing (2.1) with a value above 0 offered
   it: 9 things of the template (things 1, 2, 4, 33, 35, 40, 59, 68 and
   72), at their usual prices. Out of stock were one thing of value 0 (the
   template's HAND) and the three spells. Every stocked thing is drawn on
   its place; in the run only two showed, the others' pictures being
   blank in our scenario. [capture: rr30 `store`, capture.md 10.34]
   **The value-0 thing was out for what it is, not its value**: in a room
   store whose stock we set to things 1, 12 (HAND, a melee weapon of value
   0 and weight 0 that nobody may use), 22 (a treasure of value 0) and a
   magic item, bumping HAND's place said out of stock though it was
   stocked, while the value-0 treasure was offered ("purchase ... for
   0") and the button bought it: the stock went to 0, the player owned
   it, the gold stayed 500. [capture: rr38 `st-buy2`, capture.md 10.42
   item 3; ours, no research pass, 2026-11-16] Which of HAND's marks
   keeps it off the shelves (weight 0, nobody may use, or both, the
   creatures' own weapons) is [unknown]; every such thing tried had all
   of them, and capture.md 10.43 did not run the shelf side (the blade's
   shelf place is not known). At a death the owner-only mark alone
   decides ([magic](magic.md) 7.2); whether the shelves use the same test
   is not known.
   **Where that stock comes from**: the file holds no stock record for
   region 0 anywhere [data], and the 14 bytes where a visited region's
   stock sits read 255 throughout, while the map was the visited region,
   inside an area, with the store open and after buying and selling.
   [research, pass 2; capture: rr17 `mapstock`, `mappeek`, capture.md
   10.21; rr22, 10.26; rr30, 10.34] On the map those bytes are part of the
   map's point table, unused points (0xFF) in our template ([format
   notes](../format-notes.md), the world map's tail); read as a stock
   bitmap they are everything in stock, which matches the shelves. So the
   shelves most likely read those bytes, but no capture has changed them:
   a map whose points there are in use might offer less.
   **Buying and selling** there work as in a room, and **never change
   that stock**: a horse bought for 2,000; the blade the player already
   owned bought twice for 10 each, the gold 5,000 to 4,990 to 4,980, the
   possession unchanged (its limit of 1 reached, 4.3) and the blade still
   on its shelf; the blade sold at the opening prompt for its usual offer
   (4) and taken from the player. [research, pass 2; capture: rr30, the
   stock bytes unchanged throughout] So a thing can be bought again and
   again. (A player made to carry 5,000 gold came out unable to move, the
   gold's weight, inventory 3.)

   **Engine:** no store on the open map. A store in an area opens as in a
   room. Its shelves hold every thing 1..80 of a type a store can stock
   (2.1: treasures, magic items, weapons and armor; never spells), value
   0 included, each as many as the stock limit allows (2.6): the engine
   builds the map region's stock that way rather than modelling the map
   bytes the original reads. **In every store, room or area**, a weapon
   or armor of weight 0 that nobody may use (both marks, as before; no
   longer the test of a death or a destroy, [magic](magic.md) 7.2,
   [characters](characters.md) 7.5.1, which lose an owner-only thing of
   any weight) is never offered: its place says out of stock, whatever
   the stock holds, and the stock count is kept as stored. [unknown: the
   engine keeps both marks here until a shelf capture splits them] A thing of value 0 otherwise
   sells for 0 like any price (4.1). (Superseded: the engine's earlier
   choice of leaving value-0 things off the map's shelves.) Buying and selling pay and take as
   in a room (4.3, 4.4, 5.1, 5.2), but in region 0 neither changes the
   stock, so every shelf stays as it was.
8. **The stock is part of the region's saved state**, like its residents
   ([saves](../saves.md)):
   - Leaving the region writes the stock to the disk with the rest of the
     region, and coming back reads it: after a sale and a purchase, out to
     the map and in again, every stock count and every shelf was exactly
     as left.
   - A death with no SAVE GAME since the visit began restores the stock
     the visit started with, as it restores everything else (a thing
     bought back on its shelf, a thing sold gone from it).
   - A SAVE GAME after buying and selling saves the stock as it then
     stands, and the restore after a later death brings back that stock.

   [capture: rr27 `out`, `back`, `nosave`, `save`, capture.md 10.31]
   **Engine:** as stated: the stock is kept with the region across
   visits, saved with the game and restored with it.

## 3. The store screen

1. How a player gets into a store isn't described in the manual. [manual]
   On the C64, bumping the store opens a **separate screen** with no
   walls: the player among the stock drawn as things, its gold shown ("T
   has 500 to spend"), the button to sell (section 5) and the stick to
   browse. [capture: look, capture.md 10.17] As the project owner
   remembered. [recollection]
2. **The bump is an ordinary step.** The player steps onto the store's
   square, using one move, and the store opens there. [research; capture:
   rr11, 5 moves became 4] The store stays in the room under the player.
3. **The layout is one fixed map**, the same for every store, whatever
   the store's square, the room or the approach: 15 squares across, 10
   down, each square floor, the shelf place of one thing 1..80, or an
   exit. [research] The player always appears on 7,5. [research; capture]
   A store in a 10 x 8 room gives the same screen: the player on 7,5,
   thing 1 drawn on its place 6,6. [capture: rr38 `ss-in`, capture.md
   10.42 item 3; ours, 2026-11-16]
   The map (x across, y down; a number is that thing's place, `.` floor,
   `X` an exit):

   ```
        x  0  1  2  3  4  5  6  7  8  9 10 11 12 13 14
   y 0     .  32 .  .  30 .  .  76 .  .  78 .  .  80 .
     1     74 .  26 21 .  16 11 .  45 50 .  55 60 .  67
     2     73 .  25 20 .  15 10 .  44 49 .  54 59 .  66
     3     72 .  24 19 .  14 9  .  43 48 .  53 58 .  65
     4     X  .  8  6  .  4  2  .  34 36 .  38 40 .  X
     5     71 .  .  .  .  .  .  .  .  .  .  .  .  .  64
     6     68 .  7  5  .  3  1  .  33 35 .  37 39 .  61
     7     69 .  27 22 .  17 12 .  41 46 .  51 56 .  62
     8     70 .  28 23 .  18 13 .  42 47 .  52 57 .  63
     9     .  31 .  .  29 .  .  75 .  .  77 .  .  79 .
   ```

   Aisles run down columns 1, 4, 7, 10 and 13 with a cross aisle along
   row 5; the gaps in the outer ring are floor too. [research; capture:
   things 1, 2, 3, 13, 14 on their places; rr11 `ring`, 7,1 to 6,0 to 5,0]
4. **What is drawn.** Every stocked thing on its place with its own
   picture, one square however many; empty places, floor and exits look
   like floor (the aisles in the room's floor picture). A thing sold is
   drawn at once; one bought until none are left is erased. [research;
   capture]
5. **Moving on it.** One push moves one square in any of eight
   directions. Pushing toward floor steps there; toward a stocked thing's
   place offers it (4.1); toward another place says it is out of stock
   (the button continues); toward an exit leaves (3.6). [research;
   capture] A push off the map reads past its edge (thing 32 above the
   top row and left of 0,0; off the sides it wraps to the row above or
   below). [research] **Engine:** a push off the map is out of stock.
6. **Leaving.** The exits are 0,4 and 14,4, at the ends of row 4: pushing
   into one says "Thank you, T. Please come back soon", and the button
   returns the player to the room on the store's square. [research;
   capture: rr11, both exits] No key, the button or waiting does anything
   else. [capture]
7. **What the visit costs.** No game time passes inside the store: the
   round, the movement bar and everyone else stand still. [research;
   capture] On leaving, the player has **exactly one move left** in that
   turn, set to one, not reduced: a player that bumped the store with its
   last move comes out with one move it would not otherwise have had.
   [research; capture: rr11 `lastmove`, 1, then 0 inside, then 1]
8. **Engine:** the bump steps onto the store square (one move, ending
   no turn) and opens the region's store (`StoreOpened`). While it is
   open the player buys (`Command::Buy`) and sells (`Command::Sell`)
   with no moves used. `Command::LeaveStore`, or any command other than
   buying and selling, closes it (`StoreClosed`), the player on the
   store's square with one move left; the command then runs. Walking
   about the store screen is the front end's, from the layout of 3.3
   (`stores::square`, `stores::shelf_place`): it changes nothing in play.
   The viewer's store screen is client.md 4.1.
9. Only players use stores. A creature that walks into a store spends one
   move and stays where it was. [research, unconfirmed] **Engine:**
   creatures never choose a store square ([creatures](creatures.md) 5.1),
   so the case doesn't arise.

## 4. Buying

1. Stores price everything from the value the designer gave the thing.
   [manual] **The buying price is the thing's full value** (with its x20
   multiplier), paid in gold. Bumping a stocked thing on the store screen
   offers it ("purchase X for N", the button buys, the stick moves on);
   bumping an empty shelf place says the item is out of stock. [capture:
   look, buyb; a treasure of value 50 and one of 5,100, capture.md 10.17]
2. **Wisdom doesn't change the buying price**: 50 at Wisdom 0, 1, 5, 13,
   20, 25 and 31. [capture] The manual ties Wisdom only to selling.
3. A purchase is refused with too little gold ("does not have enough gold
   for the X"): 5,100 with 500 or 5,000 gold. [capture] It goes through
   with exactly enough. [research] Gold is the only check: **owning as
   many as allowed doesn't refuse a purchase**. The gold is paid and the
   stock goes down by one, but the thing is lost, the player's count
   staying at its limit. [research; capture: rr11 `buy-limit`, 3 owned,
   gold 500 to 490, stock 1 to 0, still 3] Weight never refuses a
   purchase; it only slows the player (inventory 3). [research]
4. A thing bought leaves the stock, one at a time: a line of 2 sold twice,
   the gold going down by the price each time. [capture] A purchase only
   adds one to the player's count: **a weapon bought is not readied**,
   not even on leaving. [research; capture: rr12 `buy-blade`, the blade
   bought for 10, owned, the hand still empty in the store and back in the
   room] A magic item set to act when picked up does not act: one showing
   a long message when picked up, bought for 1, left the gold 500 to 499,
   the stock 1 to 0 and the player owning it, with no message, back at
   the store screen. [research; capture: rr38 `st-buy3`, capture.md 10.42
   item 3; ours, 2026-11-16] **Engine:** as stated (`Bought`, with `kept` false when
   the thing was lost to the limit); refusals give `StoreRefused`.

## 5. Selling back

1. Stores buy items back at a reduced rate, and a wiser player gets a better
   deal. [manual] Wisdom ranges 0..31. [manual] **The offer** for a thing of
   value V (with its multiplier), by a player of Wisdom W:
   - the base: V minus the store's cut, the cut being the smaller of
     V - V / 3 (rounded down) and 1,334; so V / 3 up to a value of 2,000,
     and V - 1,334 above;
   - plus the Wisdom bonus: the smaller of V / 3 (rounded down) and 666,
     times W, divided by 25, rounded down.

   So up to 2,000 the offer is V / 3 x (25 + W) / 25: a third at Wisdom 0,
   1.52 thirds at 13, 2.24 thirds at 31 (100 brings 33, 50 and 73).
   [capture: sell, sell2, sell3, 36 values from 1 to 5,100 at every Wisdom
   0..31, 1,152 offers, every one; no random roll drawn, capture.md 10.17]
   Values above 255 are multiples of 20, so nothing between 2,000 and 2,020
   can be tested.
2. **Selling**: the button on the store screen offers the player's goods
   one at a time ("X has 1 Y", "sell it for N"); the button sells at once
   (no confirmation) and offers the next, the stick skips to the next;
   after the last it is back at the store's first prompt. [capture]
   **What a store will buy**: treasures, weapons, armor and magic items,
   stocked or not, each at the same rule (a blade, bow and mail of value 10
   offered 4, a magic item of value 1 offered 0); never spells (a spell
   owned was not offered). The goods come in thing order. [capture:
   sellkit, capture.md 10.17] A held weapon or worn armor is offered like
   anything else owned, and selling the last one empties the hand or the
   body. [research; capture: rr12 `sell-held`, the held blade offered for
   4, sold, the hand empty, the blade in stock] A thing owned 2 or 3 times
   is offered once per copy, each offer at the full rule (5.1). While
   more than one is owned the offer names the count with the plural and
   sells "one" ("T has 2 GOODAS", "sell one for 59"); with one left it
   reads as for a single thing ("T has 1 GOODA", "sell it for 59"). Each
   sale pays at once and adds one to the stock. [research; capture: rr27
   `twice`, two copies sold for 59 each, gold 500 to 559 to 618, the
   stock count 0 to 2, capture.md 10.31] **Engine:** as stated; the
   offer's wording, plural and "one" included, is the front end's.
   Gold can't be sold. The button sells
   only at the store's first prompt; once the player has moved it only
   answers offers and messages. [research]
3. **A sold thing joins the store's stock** and shows on its shelf place
   at once. [capture: look, two goods sold] Past the thing's limit the
   player is still paid and the thing is gone from the adventure.
   [research; capture: rr11 `sell-full`, stock of 3, sold for 4, gold 500
   to 504, stock still 3] A thing numbered above 80 is paid for and never
   joins the stock. [research, unconfirmed] In region 0 nothing joins it
   (2.7).
4. A treasure set to disappear when dropped still sells. [unknown:
   reachable through 10.17's sell flow, not yet run, capture.md 10.43]
   **Engine:** yes, and it is then gone, never stocked.
5. **The purse cap**: a sale is never refused for it. At 24,998 gold a
   sale of 50 left 25,000, and a further sale of 19 still went through,
   the gold staying at 25,000: what passes the cap is lost. [capture]

## 6. Extended adventures

The bitmap limit (2.3) and one store per region are Classic limits. An
Extended adventure stores stock as a list of thing ids and counts, still one
store per region, so any carriable thing can be stocked. More stores per
region, or per-store stock, would be a rule feature with its own spec.

## Captures needed

1. A store in an encounter area on a map whose point table uses the
   points at the stock bytes (2.7): do the shelves follow those bytes?
   Value 0: settled (2.7, not the value; capture.md 10.42). Still open
   (reachable, not run in capture.md 10.43, which split the marks at a
   death only): which of HAND's marks (weight 0, nobody may use) keeps it
   off the shelves; a stocked weapon of weight 0 usable by anyone, and
   one of weight above 0 nobody may use, each with its shelf place found
   first, would split it.
2. A treasure that disappears when dropped, sold (5.4): reachable, not
   run (capture.md 10.43).

Settled: a magic item that acts when picked up, bought (4.4), and a
store in a 10 x 8 room (3.3), capture.md 10.42; restocking from a used-up magic item (2.5), the stock across
visits, saves and restores (2.8) and a thing owned twice (5.2), all
capture.md 10.31; a store in an area, its shelves, buying, selling and
leaving (2.7), capture.md 10.34.
