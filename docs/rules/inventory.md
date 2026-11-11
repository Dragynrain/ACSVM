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
   **Engine:** a character's gold is a count, 0..25,000; it "owns" thing 81
   when the count is above 0.
4. A carriable thing numbered above 81 has no place in the bitmap. [data: no
   corpus case] **Engine:** at most one, like 33..80.

## 2. Picking up

1. Moving onto a square whose top thing is carriable picks it up and moves
   there. [manual] (movement 5.5)
2. **At the limit.** If the character already owns as many of that thing as
   allowed, a message says they can't carry more of those, and they must drop
   one first if they need to pass. [manual] So the move is refused.
   **Engine:** `Blocked` with `CarryLimit`; the bar does not change, as for
   other refused moves (movement 2.2). [unknown]
3. Weight never stops a pick-up; it only slows the character (section 3).
   [manual]
4. **Picking up a weapon readies it.** [manual] Whether it replaces a weapon
   already readied: [unknown] **Engine:** yes, missile or melee alike.
   Picking up armor does not ready it. [unknown]
5. **Gold on the floor.** A gold pile's placement parameter is most likely its
   size: 117 placed piles hold 0..51, 37 of them 0. [data] Its meaning is
   [unknown]. **Engine:** the pile holds `parameter` pieces, and 0 counts as
   1. A pile that would take the character past 25,000 gives what fits and
   the rest stays on the square. The corpus smoke test steps onto every
   carriable thing on top of its stack: 791 things and 107 gold piles (485
   pieces) are picked up, 144 of the things being weapons that get readied.
6. A magic item set to act when picked up casts its spell (`SpellTriggered`,
   source `picked_up`); a spell that applies to individuals acts on the one
   who picked it up. [manual] 3 corpus items do this. [data] The effect is
   the magic rules' business.

## 3. Weight, capacity and the slowdown

1. **Load.** Everything an individual owns counts, readied things included:
   each item's weight, plus gold pieces times Gold's weight. Negative weights
   (a magic bag, a horse) lower the load. [manual] Weights are stored with a
   ×1, ×-1, ×20 or ×-20 multiplier (format notes); 74 carriable things weigh
   less than 0, 285 weigh 0 and 39 use ×20. [data]
2. **Capacity.** Take Life Force from Constitution, halve it, take the result
   from Strength and multiply by 100. [manual] So wounds lower capacity and it
   comes back with healing. [manual] Life Force is above Constitution in 1,333
   of 1,693 master creatures, where the formula *adds* capacity. [data]
   **Engine:** the formula as written, halving toward zero; whether the
   original clamps the wound term at 0 or rounds the other way is [unknown].
3. **Slowdown.** Carrying more than half the capacity slows the character
   down: the movement bar shrinks. [manual] By how much is [unknown].
   **Engine:** load above half the capacity halves Speed (rounding down, at
   least 1); load above the whole capacity leaves 1 move. Speed 0 stays 0.
   Needs a capture.
4. **When it applies.** [unknown] **Engine:** the bar fills from the slowed
   Speed at the start of each turn; a load change during a turn counts from
   the next turn.
5. For the AI later: at the start, 3 master creatures and 5 residents carry
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
3. Readying armor uses up the rest of the turn's moves. [designer]
   **Engine:** it ends the turn. Readying a weapon costs nothing. [unknown]
4. When the last copy of a readied thing leaves the character (dropped, or
   used up by a door), nothing is readied in that slot. [unknown] **Engine:**
   so.
5. What readied things do in a fight belongs to the combat rules.

## 5. Dropping

1. *Drop Object* puts a possession down on a square next to the character, or
   just gets rid of it; dropping is also how some magic items work. [manual]
   **Engine:** `Drop { thing, dir }`, where `dir` is the adjacent square or
   none for the character's own square. Whether the original lets you drop
   on your own square is [unknown].
2. **Cannot be dropped:** magic spells [manual], and weapons or armor usable
   only by their owner (claws, fists) [manual]; 279 corpus weapons and armors
   are owner-only. [data] **Engine:** refused (`DropRefused`) at no cost.
3. **On the world map** a dropped thing is lost from the adventure for good.
   [manual] **Engine:** `dir` is ignored; the thing is gone (`LostOnMap`).
   A drop spell still goes off. [unknown]
4. **In a room** the thing lands on top of the target square's stack (movement
   5.3). Which squares accept a drop is [unknown]. **Engine:** an inside
   square whose top is nothing, a carriable thing, a floor, a space or a
   custom space; walls, portals, obstacles, custom obstacles and stores
   refuse (`DropRefused`). A square with a creature on it accepts the thing,
   which lies under the creature.
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
   [data] **Engine:** with that flag the dropped thing is consumed
   (`ThingUsedUp`); without it, it lands on top of the space. Dropping
   anything else there just lands on top. [unknown]
8. **Dropping gold.** **Engine:** `DropGold { amount, dir }` puts down one
   pile of 1..255 pieces (a placement parameter is one byte). How the
   original asks for the amount is [unknown]. A pile dropped on a matching
   "when dropped here" space casts the spell once.
9. Dropping costs no moves. [unknown] **Engine:** free.

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

1. The slowdown: bar length against load, between half and full capacity and
   past it.
2. Capacity with Life Force above Constitution, and rounding of odd wounds.
3. Gold pile size against the placement parameter, and a 0 parameter.
4. Which squares accept a drop, dropping on one's own square, and the cost
   of dropping and of readying a weapon.
5. Whether a non-destroying "when dropped here" space leaves the thing.
