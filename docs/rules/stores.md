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
**[research]** for the clean-room researcher's first stores pass
(2026-11-13, `docs/research/stores.md`) and **[capture]** where our own
runs confirm it; "[research, unconfirmed]" means no capture of ours has
checked it yet.
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
   the only restocking. [research, unconfirmed] **Engine:** as stated
   (`Restocked`).
6. **The limits.** A stock holds at most 3 of things 1..32 and 1 of things
   33..80, as possessions do (inventory 1.2); an addition past the limit is
   lost and a removal below none does nothing. [research; capture 5.3]
7. **Stores on the world map.** In region 0 buying and selling never
   change the stock. [research, unconfirmed] Whether a store can stand on
   the map at all is not known. **Engine:** as stated.
8. Whether the stock is saved with the game is not known. [unknown]
   **Engine:** it is part of the region's state, kept across visits and
   saved with the game.

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
   adds one to the player's count: **a weapon bought is not readied**, and
   a magic item set to act when picked up does not act. [research,
   unconfirmed] **Engine:** as stated (`Bought`, with `kept` false when
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
   body; a thing owned 2 or 3 times is offered once per copy. [research,
   unconfirmed] **Engine:** as stated. Gold can't be sold. The button sells
   only at the store's first prompt; once the player has moved it only
   answers offers and messages. [research]
3. **A sold thing joins the store's stock** and shows on its shelf place
   at once. [capture: look, two goods sold] Past the thing's limit the
   player is still paid and the thing is gone from the adventure.
   [research; capture: rr11 `sell-full`, stock of 3, sold for 4, gold 500
   to 504, stock still 3] A thing numbered above 80 is paid for and never
   joins the stock. [research, unconfirmed] In region 0 nothing joins it
   (2.7).
4. A treasure set to disappear when dropped still sells. [unknown]
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

1. Restocking from broken weapons and armor (2.5): break a weapon in a
   room with a store, then look at the stock.
2. Whether a store can stand on the world map, and its stock there (2.7).
3. Whether the stock is saved and restored with the game (2.8).
4. A bought weapon not readied, a held weapon offered for sale (4.4, 5.2).
5. A store in a room of another size (the layout is fixed either way).
