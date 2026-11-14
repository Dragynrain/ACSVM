# Rules: stores

Behaviour spec for stores: where they stand, what they stock, buying,
selling back, and how Wisdom changes the deal. Licensed CC BY 4.0. Not yet
implemented; [movement](movement.md) 5.5 refuses a step onto a store square
and leaves opening it to this spec.

Tags as in [movement](movement.md): **[manual]** paraphrased from the
manual, **[data]** from the stored format or corpus counts, **[unknown]**
with the engine's choice as **Engine:**, and **[recollection]** for the
project owner's memory of play, a lead for captures rather than a source.
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
5. Stock is not restocked over time. [unknown] **Engine:** stock changes only
   by buying and selling (sections 4 and 5).

## 3. Opening a store

1. How a player gets into a store isn't described in the manual. [unknown]
   **[recollection]** The project owner remembers, from playing it years
   ago, that a store opens a separate area the player walks into, with the
   stock laid out as items, and that bumping an item shows it for sale. This
   is the first thing to capture.
   On the C64, bumping the store opens a separate screen with the wares
   laid out round the player, its gold shown, the button to sell and the
   stick to browse; selling a treasure of value 100 at Wisdom 13 was
   offered 50, with no random roll drawn. [capture: look, capture.md
   10.17]
2. **Engine:** following that recollection. Bumping the store square uses a
   move, and the move is refused as before ([movement](movement.md) 5.5). The
   player is then moved into a **store room** the engine builds from the
   region's stock: one stocked thing per square, a count of 2 or 3 shown as
   one square, and an exit door. The engine emits `StoreOpened`.
3. **Engine:** in the store room, bumping a stocked thing shows its name and
   price (`StoreOffer`), and a separate Buy command takes it (section 4).
   Stocked things can't be picked up by walking onto them. Selling is a
   command while in the store room (section 5).
4. **Engine:** walking out through the exit puts the player back on the
   square they bumped the store from, with the moves they had left. Time
   passes as on any turn while they browse. [unknown]
5. Only players use stores. [unknown] **Engine:** creatures never open one,
   and still can't step onto a store square ([creatures](creatures.md) 5.1).
   Creatures don't follow a player into the store room.

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
   for the X"): 5,100 with 500 or 5,000 gold. [capture] Whether owning as
   many of that thing as allowed (inventory 2.2) refuses one is not
   captured. Weight never refuses a purchase; it only slows the player
   (inventory 3). [manual, by analogy with picking up]
4. A thing bought leaves the stock, one at a time: a line of 2 sold twice,
   the gold going down by the price each time. [capture] It joins the
   player's possessions as if picked up: a weapon is readied; a magic item
   set to act when picked up does not act. [unknown] **Engine:** as stated.

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
   **What a store will buy**: our treasures, stocked or not. Magic items,
   weapons, armor, spells and gold are not captured. [unknown]
   **Engine:** anything it could have stocked (2.1): treasures, magic
   items, weapons and armor. Gold can't be sold.
3. **A sold thing joins the store's stock** and shows on its shelf place
   at once. [capture: look, two goods sold] Whether that holds past the
   thing's limit, or for things numbered above 80, is [unknown].
   **Engine:** it joins the stock while the stock is under that thing's
   limit and the thing is numbered 1..80 (2.3); otherwise it is gone from
   the adventure.
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

1. Opening the store, browsing, buying and selling: captured (capture.md
   10.17). Still open: leaving the store and what the visit costs in
   moves; where each thing's shelf place is (a researcher question).
2. The sell-back rate and the buying price at every Wisdom: captured.
3. What a store does with magic items, weapons, armor, spells and gold
   offered for sale, and with a thing past its limit.
4. Whether a store's stock is kept per region across visits and saves.
5. What a thing bought does on arrival (a weapon readied, a magic item
   that acts when picked up).
