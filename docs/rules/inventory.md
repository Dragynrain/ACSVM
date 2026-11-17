# Rules: inventory and encumbrance

Behaviour spec for what characters carry: limits, picking up, weight and
the slowdown it causes, readying weapons and armor, and dropping. Licensed
CC BY 4.0. The engine implements it in `crates/acs-engine` (`inventory.rs`,
with pick-ups in `step.rs`).

Tags as in [movement](movement.md): **[manual]** paraphrased from the
manual, **[data]** from the stored format or corpus counts, **[unknown]**
with the engine's current choice as **Engine:**. Baber's Jadeworld notes are
tagged **[designer]**. Counts cover all 15 corpus adventures.

## 1. What can be carried

1. Treasures, magic items, missile and melee weapons, armor and magic spells
   are carried; nothing else is. [manual] Every carriable thing in the corpus
   is numbered 1..81 (1,189 things). [data]
2. **How many of one thing.** When a thing is added to the master list, the
   designer fixes how many of it one individual may own: one, or up to three.
   It cannot be changed later. [manual] The stored possession bitmap gives
   things 1..32 a 2-bit count and things 33..80 one bit (format notes), and
   all 4,969 corpus counts above 1 are on things 1..32. [data]
   **Engine:** things 1..32: at most 3 each; 33..80: at most 1.
3. **Gold.** Every new master list has one special treasure, Gold, which may be
   renamed (dollars, crystite). Anyone may own as much of it as they can
   carry, up to 25,000 pieces, and its value is always 1. [manual] It is
   thing 81 in all 15 adventures (13 keep the default name, two rename it),
   a treasure of weight 1 or 4 and value 1. Creature records store it as the
   wealth count (at most 3,100 in the corpus), not in the bitmap. [data]
   Gold's weight counts once per piece, 0 included (3.1). [capture: rr38,
   capture.md 10.42 item 2]
   **Engine:** a character's gold is a count, 0..25,000; it "owns" thing 81
   when the count is above 0.
4. A carriable thing numbered above 81 has no place in the bitmap. [data: no
   corpus case] **Engine:** at most one, like 33..80.

## 2. Picking up

1. Moving onto a square whose top thing is carriable picks it up. [manual]
   On the C64 the character **stays where it was**: the thing goes, the
   program shows "NAME TAKES THING" and waits for the button (the bar does
   not drain meanwhile), and the character must step again to enter the
   square. Taking costs exactly one step: after a take the turn had one
   step fewer than the same turn without it, every time. [capture: weight,
   300 takes; acts, table take, 125 takes of things of weight 1]
   **Engine:** the thing is taken, the
   character stays put, and the take spends one step (a heavy one ends
   the turn, 3.5). Gold piles are taken
   the same way. [unknown for gold]
2. **At the limit.** If the character already owns as many of that thing as
   allowed, a message says they can't carry more of those, and they must drop
   one first if they need to pass. [manual] So the move is refused, and it
   costs nothing: on the C64 a player owning the one copy of a treasure
   allowed, or the three of a thing 1..32, pushed onto another: "cannot
   carry more", the button, the player still on its square and its moves
   the same before the push, after the button and long after, until the
   bar's next drain; the same push with nothing owned took the thing and
   one move. [capture: rr39 `pc-e`, `pc-s`, `pc-e0`, capture.md 10.43
   item 1; ours, no research pass, 2026-11-16]
   **Engine:** `Blocked` with `CarryLimit`; the bar does not change, as for
   other refused moves (movement 2.2).
3. Weight never stops a pick-up; it only slows the character (section 3).
   [manual]
4. **Picking up a weapon readies it.** [manual] It replaces a weapon
   already readied, whichever kind either is: a player holding a melee
   weapon who took a missile weapon held the missile weapon, and one
   holding a missile weapon who took a melee weapon held the melee one.
   Picking up armor does not ready it: the armor taken was owned, nothing
   worn, the weapon in hand unchanged. [capture: rr39 `rd-e`, `rdb-e`,
   `rd-s`, capture.md 10.43 item 2; ours, no research pass, 2026-11-16]
   **Engine:** so.
5. **Gold on the floor.** A placed pile of gold is two item records: the
   gold, then a record on square 15,14 whose other two bytes are the
   pile's size, low byte first. All 112 corpus piles (110 C64, 2 PC) are
   followed by one, and no 15,14 record follows anything else; sizes run
   from 1 to 7,500. [data] The C64 reads it so: a creature of ours took
   piles whose size records we wrote as 1 to 9, and 10, 16, 20, 32, 51,
   0, 100, 200 and 255: each message seen named that size ("finds gold
   worth 0" too), and its gold ended at the sums, 45 and 684 [capture:
   capture.md 10.14.8]; before our disks wrote the record, it read the
   two bytes after the pile, whatever they were (5 gave 512). The pile's
   own placement parameter (0, 1 or 35 in the corpus) changes nothing a
   player sees: a player took a pile with parameter 35 and size record 10
   and got exactly 10, and one whose size bytes were 81 and 10 got 2,641
   (81 + 10 x 256), the parameter 35 again. [capture: rr36 `gp2`, `gp`,
   capture.md 10.40; ours, no research pass, 2026-11-16] What else it may
   mean is [unknown]: nothing in play shows it, so only a research pass on
   the take can tell (capture.md 10.43 sorts it so). A taken pile and its size record stay in the room's
   list, moved to square 15,15, as other taken things are. [capture:
   rr36] **Engine:** the decoder folds the size record into the
   pile (`RoomItem::gold`); the pile holds that many pieces (a pile the
   data gives no size: its parameter, and 0 counts as 1); a size of 0 is
   worth nothing. A pile that
   would take the character past 25,000 gives what fits and the rest
   stays on the square. The corpus smoke test steps onto every carriable
   thing on top of its stack: 791 things and 107 gold piles (47,155
   pieces) are picked up, 144 of the things being weapons that get
   readied.
6. A magic item set to act when picked up casts its spell (`SpellTriggered`,
   source `picked_up`); a spell that applies to individuals acts on the one
   who picked it up. [manual] 3 corpus items do this. [data] The effect is
   the magic rules' business.
7. **What a take gives: the placed parameter.** A carriable thing placed
   in a room carries a parameter byte (format notes, room items), and
   that byte is what a take gives. On the C64 a take of a thing placed
   with parameter 0 looks like any other ("T takes ...", the button, the
   thing gone from the floor, its list entry moved to 15,15) but gives
   nothing: no possession is set, the taker does not own it afterwards,
   and the thing is simply gone from the room. The same treasures placed
   with 1 and with 2 each set their possession bit (things 33..80, one
   bit, 1.2). [capture: rr38 `pk-e2`, `pk-s2`, `pk-w2`, capture.md 10.42
   item 6; ours, no research pass, 2026-11-16] This explains a take seen
   to set no bit before (winning 1.4, a treasure we had placed with 0).
   A magic item placed with 0 still runs its pick-up spell (2.6) when
   taken: ours set off its summons and its Add to Room. [capture: rr38
   `sn-pk2`, `ar-pk2`, capture.md 10.42 items 1 and 4] **Creatures
   alike**: a thief creature took two treasures, one placed with 0 and
   one with 1; both showed the take and left the floor, and only the one
   placed with 1 became its possession, whichever it took first, in 8 of
   8 runs. [capture: rr39 `ct-m`, capture.md 10.43 item 8; ours, no
   research pass, 2026-11-16] Not tried: what a 2 or 3 gives for things
   1..32, whose count has two bits [unknown]. The gold pile is not counted
   this way: its size record decides (2.5).
   **Engine:** a take adds the placed parameter to the count, capped at
   the thing's limit (1.2), so for things 33..80 any parameter above 0
   makes it owned; a parameter of 0 is taken off the floor, spends its
   step, shows the take and runs a pick-up spell as usual, and adds
   nothing (a weapon so taken is not readied, 2.4). The limit check of
   2.2 is made on the count owned, whatever the parameter. Creatures'
   takes follow the same rule (captured, above). Things the engine itself puts down
   (drops, a death's or a destroy's things, Add to Room, whose count was
   1 on the C64, magic 3.13 and capture.md 10.40) carry 1. How many
   corpus carriables are placed with 0 is not yet counted [data: to
   count].

## 3. Weight, capacity and the slowdown

1. **Load.** Everything an individual owns counts, readied things included:
   each item's weight, plus gold pieces times Gold's weight. Negative weights
   (a magic bag, a horse) lower the load. [manual] On the C64 gold weighs
   per piece: a player carrying nothing but gold (capacity 1,000, Speed
   15) refilled to 15, 12 and 6 moves with 300, 600 and 800 pieces of
   weight 1 and to none with 1,000; to 15, 12 and 6 with 100, 150 and 200
   pieces of weight 4; to 15 with 900 and 3,000 pieces of weight 0, each
   the slowdown of 3.3 for a load of the pieces times Gold's weight.
   [capture: rr38 `gw1`, `gw4`, `gw0`, 18 refills, capture.md 10.42 item
   2; ours, no research pass, 2026-11-16] **Engine:** so. Weights are stored with a
   ×1, ×-1, ×20 or ×-20 multiplier (format notes); 74 carriable things weigh
   less than 0, 285 weigh 0 and 39 use ×20. [data]
2. **Capacity.** Take Life Force from Constitution, halve it, take the result
   from Strength and multiply by 100. [manual] So wounds lower capacity and it
   comes back with healing. [manual] Life Force is above Constitution in 1,333
   of 1,693 master creatures, where the formula *adds* capacity. [data]
   On the C64 the wound term is not held at 0 and its halving **rounds
   down**, toward minus infinity: capacity = (Strength - floor((Constitution
   - Life Force) / 2)) x 100. An odd wound counts as the even one below it
   (wounds 13 and 12 alike), and Life Force above Constitution adds to the
   capacity, 1 over adding 100 (-1 halves to -1). Strength 9, Constitution
   20: capacity 900 at Life Force 20, 1,000 at 21 and 22, 1,200 at 25 and
   1,400 at 30. Read from a player's moves at each turn's start (the moves
   refill there, with the Life Force the top-of-turn die left, 3.4) while it carried 500,
   above half of every capacity below 1,000, so the slowdown (3.3) told
   those capacities to the nearest 50 (900: 13 moves of 15); from 1,000 up
   the load is at most half and the moves are full (15), which the
   formula's 1,000, 1,200 and 1,400 all give: rounding down fitted 169 of 169 turn starts, halving toward
   zero 160 (it misses Life Force 21 of 20), rounding up or not rounding
   111, any reading holding the wound term at 0 at most 133. [capture: rr35
   `c-s9l20` to `c-s9l30`, `c-s12c40`, `c-s12c41`, `scripts/capcheck.py`,
   capture.md 10.39 item 4; ours, no research pass, 2026-11-16]
   **Engine:** so: the wound term halved rounding down, unclamped.
   (Superseded: the engine's earlier halving toward zero, which gave 900,
   not 1,000, for Life Force 21 of Constitution 20.)
3. **Slowdown.** Carrying more than half the capacity slows the character
   down: the movement bar shrinks. [manual] On the C64 the slowed Speed
   falls in a straight line from the full Speed at half the capacity to 0
   at the full capacity: Speed x 2 x (capacity - load) / capacity, rounded
   down, for loads above half. Up to half there is no slowdown. A character
   at or past its capacity has Speed 0 and no longer gets a turn it can
   move in. [capture: weight, two players (Speed 15 with capacity 1,000,
   Speed 10 with 1,500), 20 runs each: loads 400, 500, 501, 600, 700, 800,
   1,000 gave Speed 15, 15, 14, 12, 9, 6, 0; loads 700, 750, 751, 900,
   1,050, 1,200, 1,350, 1,500 gave 10, 10, 9, 8, 6, 4, 2, 0] **Engine:**
   that formula; a capacity of 0 or less with any positive load gives 0.
4. **When it applies.** The next turn's bar starts from the slowed Speed:
   a load change does not shorten the turn under way. [capture: weight]
   **Engine:** a load change counts from the next turn.
   **The Life Force a turn's bar uses** is the one after the top of the
   turn's healing die ([magic](magic.md) 5.3.2): at a player's turn start
   the die (heal 1 on a 2) comes first, and only then are the moves set:
   Speed, halved when hindered ([creatures](creatures.md) 2.6), then this
   slowdown against the capacity of 3.2 worked out from the Life Force as
   the die left it. So a player whose die heals it from an odd wound to an
   even one starts that very turn with the bar of the smaller wound.
   Read from the moves byte at each turn's start against the Life Force:
   taking the Life Force after the die fitted all 169 turn starts of the
   healing runs; taking it when the turn byte changes, before the die's
   heal, fitted 12 of 63. [capture: rr35 `c-s12c40`, `c-s12c41`,
   `scripts/capcheck.py`, capture.md 10.39 item 4; ours, no research pass,
   2026-11-16] The halving takes no Life Force, so its place against the
   die cannot be seen; nor can drift's (magic 5.1), which changes Power
   only. Nothing else captured disagrees: the healing sweep (capture.md
   10.12) carried nothing, the idle-roll range (magic 5.3.1, capture.md
   10.29) is the moves this refill gave and so already the after-die bar,
   and the creatures' die (capture.md 10.33, 10.39 item 1) is drawn before
   their turn byte, as here.
   **A creature's turn start is the other way round**: its moves are set
   from the Life Force **before** the die, so a heal by the die lengthens
   only its next turn. A creature of Speed 10, Strength 10 and
   Constitution 20 at Life Force 14, carrying 600 (capacity 700, so 2
   moves; at 15 the capacity is 800 and the moves 5), began every first
   turn with 2 moves, 112 of 112; in the 24 of them where the die healed
   it to 15, the moves byte was set to 2 before the Life Force rose, and
   the turn byte changed after both. [capture: mem, rr39 `cw-m`, `cw-m2`,
   capture.md 10.43 item 9; ours, no research pass, 2026-11-16] The
   creature's slowdown itself follows 3.3 (creatures 2.5; capture.md
   10.32, and again here).
   **Engine:** at a player's turn start: drift, the healing die, then the
   moves (Speed, the halving, this slowdown from the capacity at the Life
   Force the die left). At a creature's: catch-up (magic 5.5), drift, the
   moves (the same, from the Life Force before the die), then the die.
   Either way these are the turn's full moves of magic 5.3.1. Where a
   creature's catch-up, which can raise its Life Force, falls against its
   moves is [unknown] (the creature watched had no round to catch up);
   the engine keeps it first. (Superseded: the engine set the moves
   before the die for everyone, then after it for everyone.)
5. **A heavy take ends the turn.** When the thing taken weighs more than
   a quarter of the taker's capacity (3.2), the take ends the turn under
   way, whatever steps are left; a thing of exactly a quarter, or less,
   costs its one step (2.1) and the turn goes on. Only the thing's own
   weight decides it (its stored weight with the multiplier): not the
   load after the take, nor whether the take brings on the slowdown. So
   at capacity 900 a thing of 225 goes on and one of 239 ends the turn;
   at 600, 150 goes on and 170 ends it; at 300, 70 goes on and 100 ends
   it; at 2,000 and 3,100 nothing up to 240 ended it. A load of 240 at
   capacity 900 went on (the thing 130) where a load of 340 ended (the
   thing 240), and a thing of 130 at capacity 500, load 240, ended the
   turn with no slowdown. [capture: mem, rr34 `i4/`, capture.md 10.38
   item 4: 36 runs, 72 takes at capacities 300 to 3,100 (Strength x 100,
   no wounds), every one as this rule; ours, no research pass,
   2026-11-15] The earlier runs fit too: things of 240 and 400 ended the
   turn at capacities 900 to 1,500, things of 1 to 100 never did.
   [capture: weight; acts, table take] Not tried: a capacity lowered by
   wounds (Life Force below Constitution), a negative or zero capacity,
   a gold pile, and a creature's take.
   **Engine:** a player's take ends its turn when 4 x the thing's weight
   is more than its capacity of 3.2 at that moment, wounds included (so a
   capacity of 0 or less ends it for any positive weight); a gold pile's
   weight is its pieces times Gold's weight. A creature's take spends its
   one move as before (the AI rules).
6. For the AI later: at the start, 3 master creatures and 5 residents carry
   more than their capacity, and 5 master and 9 residents more than half.
   [data] An overburdened creature drops a magic item, treasure or gold.
   [manual]

## 4. Readying weapons and armor

1. An individual has one readied weapon (melee or missile) and one readied
   armor. [data] Creature records have one weapon and one armor byte, and
   every non-zero one in the corpus points at a thing of the right kind.
2. *New Weapon* and *New Armor* ready a weapon or armor the character carries.
   [manual] **Engine:** `Ready { thing }`; anything else, or a thing not
   carried, is refused (`ReadyRefused`) at no cost.
3. Readying armor uses up the rest of the turn's moves. [designer] On the
   C64 readying a weapon costs one move and readying armor ends the turn,
   each only when the thing in hand or worn changes ([combat](combat.md)
   6.1). [research] A player's NEW ARMOR took the moves from 5 to 0 as the
   armor was chosen, set the worn armor and passed the turn to the next
   mover at once; the weapon's one move was captured before (combat 6.1,
   capture.md 10.32). [capture: rr36 `dk2-armor`, capture.md 10.40 item
   6; ours, no research pass, 2026-11-16] **Engine:** so. A weapon readied
   by picking it up (2.4) costs only the pick-up's step: a missile weapon
   taken and readied left the same 4 of 5 moves just after the button as
   a treasure taken. [capture: rr39 `pr-e`, `pr-w`, capture.md 10.43 item
   2; ours, no research pass, 2026-11-16] **Engine:** so.
4. When the last copy of a readied thing leaves the character (dropped, or
   used up by a door), nothing is readied in that slot: a player dropping
   the melee weapon it held, its only copy, held nothing after. [capture:
   rr39 `rd-drop`, capture.md 10.43 item 2; ours, no research pass,
   2026-11-16; a door using one up not tried] **Engine:** so.
5. What readied things do in a fight: [combat](combat.md) (parrying needs a
   held melee weapon, armor counts only when worn).

## 5. Dropping

1. *Drop Object* puts a possession down on a square next to the character, or
   just gets rid of it; dropping is also how some magic items work. [manual]
   On the C64 a player's DROP OBJECT asks for the thing, then shows a
   cursor on the player's square ("move cursor to drop spot"); the stick
   moves it only to the four squares beside the player, not further and
   not onto an obstacle (a push that way leaves it where it is), and the
   button drops the thing on the square under it, the player's own square
   included. [capture: rr36 `dh-own`, `dh-e`, `dh-ee`, `dh-es`, `dk2-e`,
   capture.md 10.40 item 6; ours, no research pass, 2026-11-16]
   **Engine:** `Drop { thing, dir }`, where `dir` is one of the four
   squares beside the character or none for its own square.
2. **Cannot be dropped:** magic spells [manual], and weapons or armor usable
   only by their owner (claws, fists) [manual]; 279 corpus weapons and armors
   are owner-only. [data] **Engine:** refused (`DropRefused`) at no cost.
3. **On the world map** a dropped thing is lost from the adventure for good.
   [manual] On the C64 the player's DROP OBJECT on the map asks for no
   spot: choosing the thing shows "the thing dropped is lost forever!",
   and the thing is gone. A magic item set to act when dropped (Life
   Force +5) still cast its spell on the dropper: Life Force 40 to 45.
   [capture: rr39 `mp-d`, `mp-d2`, capture.md 10.43 item 5; ours, no
   research pass, 2026-11-16] Its cost in moves is 5.9's.
   **Engine:** `dir` is ignored; the thing is gone (`LostOnMap`), and a
   drop spell goes off on the dropper.
4. **In a room** the thing lands on top of the target square's stack (movement
   5.3). On the C64 the player's own square, a portal's square, a store's
   square and a square a creature stands on all took a drop, on top each
   time; walls and obstacles are out of the cursor's reach (5.1).
   [capture: rr36 `dk2`, capture.md 10.40 item 6; ours, no research pass,
   2026-11-16] A thing dropped on a portal or a store then lies on top
   and is what the next step there meets (movement 5.3). A custom
   obstacle beside the player and a portal on the wall line beside it are
   out of reach too: the cursor pushed toward either stayed on the
   player's square, and the button dropped the thing there. [capture: rr39
   `wp-dco`, `wp-dpo`, capture.md 10.43 item 6; ours, no research pass,
   2026-11-16] A creature's drop finds a custom obstacle refusing as well
   ([creatures](creatures.md) 8.1.1). **Engine:** an inside
   square whose top is anything but an obstacle or a custom obstacle; a
   wall square (a portal on the wall line included), an obstacle and a
   custom obstacle refuse (`DropRefused`) at no cost. A square with a
   creature on it accepts the thing, which lies under the creature. The
   C64's cursor never offers the refused squares, so a front end with a
   cursor gets the same outcome; the refusal only guards commands it
   should not send.
   (Superseded: the engine's earlier refusal of portals and stores.)
5. **Treasures that disappear when dropped** vanish instead of landing
   (`VanishedWhenDropped`). [manual] 19 corpus treasures do this. [data]
6. **Magic items with a drop spell** cast it (`SpellTriggered`, source
   `dropped`). A spell that applies to individuals, dropped next to the
   dropper, acts on the dropper unless the square holds a creature. [manual]
   4 corpus items do this. [data] The effect is the magic rules' business.
7. **"When dropped here" custom spaces.** Dropping the needed thing onto a
   square whose top is such a space casts its spell (`SpellTriggered`, source
   `custom_space`; the needed thing and parameter are resolved as in movement
   5.8). [manual] This makes vending machines: drop a coin, get a candy bar.
   20 corpus spaces use this trigger, 19 with "destroy needed thing" set.
   [data] On the C64, with that flag the space's spell ran and the dropped
   thing was gone; without it the spell ran and the thing lay on top of
   the space. [capture: rr36 `dhd-e`, `dh-e`, capture.md 10.40 item 6;
   ours, no research pass, 2026-11-16] **Engine:** so: with the flag the
   dropped thing is consumed (`ThingUsedUp`); without it, it lands on top
   of the space. Such a space does not run under Activate All
   ([magic](magic.md) 3.14). Dropping anything else there just lands on
   top, the space's spell not run: armor dropped on a space waiting for a
   weapon lay on top of it, and the space gave nothing. [capture: rr39
   `dh2-e`, capture.md 10.43 item 6; ours, no research pass, 2026-11-16]
8. **Dropping gold.** **Engine:** `DropGold { amount, dir }` puts down one
   pile of 1..25,000 pieces (a pile's size is two bytes, 5). How the
   original asks for the amount is [unknown]: a front end matter only, not
   yet run (capture.md 10.43). A pile dropped on a matching
   "when dropped here" space casts the spell once.
9. Dropping costs no moves. On the C64 a player's moves fell by one as
   the thing was chosen and were back once the drop spot was pressed: no
   net cost, on the own square or beside it. [capture: rr36 `dh-own`,
   `dh-e`, the moves read around every press, capture.md 10.40 item 6;
   ours, no research pass, 2026-11-16] **On the world map a drop costs one
   move**: there is no spot to press, and the move taken as the thing is
   chosen never comes back. A player with 4 moves had 3 once the thing was
   chosen, 3 still after the message's button and on until the bar's next
   drain. [capture: rr39 `mp-d`, `mp-d2`, `mp2.txt`, 2 runs, capture.md
   10.43 item 5; ours, no research pass, 2026-11-16] **Engine:** a drop in
   a room is free; a drop on the world map spends one move (and with none
   left the turn ends, as any move does). A refused drop (5.2, 5.4) costs
   nothing anywhere. (Superseded: the engine's earlier choice of free
   everywhere.)

## 6. Characters

1. New characters start with Life Force equal to Constitution. [manual]
   Normal starting values are 10..11 for every trait except Speed (6..7).
   [manual] **Engine:** `PlayerSetup` adds Strength, Constitution, Life Force
   (11 each by default, giving capacity 1,100) and starting gold (0). Trait
   generation is a later rule set.
2. The player's profile shows ratings and possessions. [manual] **Engine:**
   `Player::load`, `Player::capacity` and `Player::effective_speed` give the
   numbers a front end needs.

## 7. Left for later

- **Creatures:** creature loads, the overburdened drop, gold and treasure
  pick-up by creatures (the AI rules).
- **Death:** an individual leaves what it carried where it died, except on
  the world map or for things set to disappear when the owner dies; an
  owner-only weapon set to be usable after death appears then. [manual]
- **Stores:** buying and selling, and stock limits, specced in
  [stores.md](stores.md).
- **Use Object, Use Power:** the magic rules.

## Captures needed

1. The slowdown, a take's cost (one step) and where a take ends the turn
   (more than a quarter of the capacity): captured (3.3, 2.1, 3.5;
   capture.md 10.38). Still open: a heavy take with the capacity lowered
   by wounds, a gold pile, and a creature's take (3.5).
2. Capacity with Life Force above Constitution, and rounding of odd
   wounds: captured (3.2: rounded down, unclamped; capture.md 10.39). A
   player's bar comes from the Life Force after the top-of-turn die, a
   creature's from the Life Force before it (3.4, capture.md 10.43).
   Still open: a creature's catch-up against its moves.
3. Gold pile size: captured (2.5, the size record alone; the pile's own
   parameter changes nothing seen, capture.md 10.40). What that parameter
   is for needs a research pass on the take (capture.md 10.43 (b)).
4. Which squares accept a drop, dropping on one's own square, and the cost
   of dropping and of readying: captured (5.1, 5.4, 5.9, 4.3; capture.md
   10.32, 10.40), and a custom obstacle and a wall portal out of the
   cursor's reach, a drop on the map (its spell goes off, it costs one
   move) and readying by a pick-up (only the step): capture.md 10.43.
5. Whether a non-destroying "when dropped here" space leaves the thing:
   captured (5.7: it does, on top), and another thing dropped there lies
   on top, giving nothing (capture.md 10.43).
6. Gold's weight per piece (3.1) and what a take gives (2.7: the placed
   parameter, 0 nothing): captured (capture.md 10.42), creatures' takes
   too (capture.md 10.43). Still open: a parameter of 2 or 3 on things
   1..32, and the corpus count of such placements.
7. The carry limit's refusal (2.2: no cost), a picked-up weapon replacing
   the one readied, armor not readied (2.4) and the last copy dropped
   (4.4): captured (capture.md 10.43). Still open, reachable but not run:
   how a gold drop asks for its amount (5.8, the front end's), and a door
   using up a readied thing (4.4).
