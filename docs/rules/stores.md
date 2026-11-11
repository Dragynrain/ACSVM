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
   [manual] **Engine:** the buying price is the thing's full value (with its
   multiplier), paid in gold.
2. Whether Wisdom also lowers the buying price. [unknown] The manual ties
   Wisdom only to selling. **Engine:** no.
3. A purchase is refused if the player has too little gold or already owns
   as many of that thing as allowed (inventory 2.2). Weight never refuses a
   purchase; it only slows the player (inventory 3). [manual, by analogy
   with picking up]
4. A thing bought leaves the stock (one at a time) and joins the player's
   possessions as if picked up: a weapon is readied; a magic item set to act
   when picked up does not act. [unknown] **Engine:** as stated.

## 5. Selling back

1. Stores buy items back at a reduced rate, and a wiser player gets a better
   deal. [manual] Wisdom ranges 0..31. [manual] The rate is [unknown].
   **Engine:** the store pays value × (32 + Wisdom) / 96, rounded down: a
   third of value at Wisdom 0, about two thirds at 31.
2. **What a store will buy.** [unknown] **Engine:** anything it could have
   stocked (2.1): treasures, magic items, weapons and armor, whether or not
   it stocks that thing. Gold can't be sold.
3. **Where a sold thing goes.** Whether it joins the store's stock (and can be
   bought back) is [unknown]. **Engine:** it joins the stock while the stock
   is under that thing's limit and the thing is numbered 1..80 (2.3);
   otherwise it is gone from the adventure.
4. A treasure set to disappear when dropped still sells. [unknown]
   **Engine:** yes, and it is then gone, never stocked.
5. Gold paid out is capped at 25,000 in the player's purse (inventory 1.3).
   **Engine:** a sale that would pass the cap is refused.

## 6. Extended adventures

The bitmap limit (2.3) and one store per region are Classic limits. An
Extended adventure stores stock as a list of thing ids and counts, still one
store per region, so any carriable thing can be stocked. More stores per
region, or per-store stock, would be a rule feature with its own spec.

## Captures needed

1. What opening a store looks like: whether it is a separate area, how the
   stock is laid out, how bumping an item offers it, how selling and leaving
   work, and what it costs in moves.
2. The sell-back rate at several Wisdom values, and whether Wisdom changes
   the buying price.
3. Whether a sold thing appears in the stock, and what a store does with a
   thing it doesn't stock.
4. Whether a store buys magic spells or gold.
5. Whether a store's stock is kept per region across visits and saves.
