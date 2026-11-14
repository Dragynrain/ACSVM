# Research brief: stores

A request to the project's clean-room researcher (docs/research/README.md).
Licensed CC BY 4.0. Status: briefed 2026-11-13, inputs ready; not started.

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

## Findings

None yet.
