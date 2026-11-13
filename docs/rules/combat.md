# Rules: combat

Behaviour spec for melee: who attacks whom, a blow from start to end,
parrying, damage, armor, magic defense, learning, readying and breaking,
and creatures turning hostile when a player hurts them. Licensed CC BY
4.0. The engine implements it in `crates/acs-engine` (`combat.rs`, with
the attack decision in `ai.rs` and `step.rs`).

Tags as in [movement](movement.md): **[manual]**, **[data]**,
**[designer]**, **[unknown]** with the engine's choice as **Engine:**,
plus **[research]** for the clean-room researcher's first combat pass
(2026-11-13, `docs/research/combat.md`) and **[capture]** where our own
captured runs confirm it. The researcher marked each finding as run in
the original (its experiments) or only read in the program ("traced");
"[research, unconfirmed]" means no capture of ours has checked it yet.

Firing missile weapons is not in this slice (section 8).

## 1. Terms

1. **A roll of 0 to 99** is the percentile roll; **below X** means
   smaller than X. **Clamped** means held between 14 and 90, the clamp of
   the cast chance ([magic](magic.md) 4.1). [research]
2. **Wound penalty**: 30 x (Constitution - Life Force) / Constitution,
   rounded down; 0 when Life Force is at or above Constitution, at most
   30. [research] **Engine:** 0 for a Constitution of 0.
3. **Halving** rounds down, negative numbers too (-5 halves to -3).
   [research]
4. **Holding** a weapon: having it readied. **Owning**: having it among
   one's things. **Wearing** armor: having it readied. [research]
5. **The armor term** of an individual: 0 without armor. With armor of
   attack adjustment A (-65..10): the smaller of A + Armor skill and the
   larger of A and 0. So skill makes up a negative adjustment up to 0, and
   a positive one never grows. [research]
6. **Class** of an individual: a player, or a creature's alignment with
   Aggressive or Peaceful. A creature is a **fighter** or a **slinker** as
   in [creatures](creatures.md) 4.2.

## 2. Who attacks whom

1. **A creature attacks by stepping into the square of the individual it
   attacks**; it does not move. Its step choice ([creatures](creatures.md)
   6.5) looks at squares holding an individual too, with the cost below as
   the square's cost, and an attack whenever the square it picks holds
   one. [research] Every attack in the researcher's runs came from a step
   choice, and every attack of 32 captured runs of ours fits it, steps and
   all. [capture: armed, capture.md 10.14.9]
2. **Who a fighter will strike, and how eagerly**: the cost of a square
   holding an individual of each class. "-" means closed: never stepped
   into, never attacked. 1 is cheaper than plain floor (2), so a fighter
   prefers striking to walking; 3 is dearer. [research]

   | Fighter | P thief | P neutral | P enemy | P friend | A thief | A neutral | A enemy | A friend | Player |
   | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
   | Thief | 1 | 3 | 3 | 3 | 1 | 1 | 1 | 1 | 1 |
   | Neutral | 1 | 3 | 3 | 3 | 3 | 3 | 1 | 1 | 1 |
   | Enemy | 3 | 3 | - | 1 | 3 | 3 | - | 1 | 1 |
   | Friend | 3 | 3 | 1 | - | 1 | 1 | 1 | - | - |

3. **Only a fighter that owns a melee weapon** uses the table, held or
   not. For a slinker, or a fighter owning none, every square with an
   individual on it is closed. [research] An unarmed fighter enemy of ours
   kept stepping round the player in 32 of 32 runs; armed, it attacked in
   all of them. [capture: fight, armed, capture.md 10.14.6, 10.14.7]
4. **The goal's own square** costs 1 when it is open; other individuals'
   squares keep their table cost. So attacking "others it meets on the
   way" ([manual]) is no separate decision: any individual beside a fighter
   whose square isn't closed can win the step score; a neutral or a thief
   (cost 3) loses to a floor square unless the distance or the roll favours
   it. [research] Only attacks on the goal have been seen run.
5. **Attacking ends the turn.** The blows (3.1) cost no moves; at the
   first, the attacker's moves left go to 0. [research; capture]
6. **Readying on contact.** A creature that owns a melee weapon but holds
   none readies it (its lowest-numbered) as it steps into the foe: one move
   if the weapon in hand changes, then the attack. [research: 3 of 3 runs]
   **Engine:** so; with no move left for it the attack goes ahead anyway,
   with 0 moves left (blows from 3.1).
7. **A player** attacks by moving into a square with any individual on
   it, whatever its alignment: a friend, a neutral, another player. Holding
   a melee weapon, the attack starts at once. Owning one but not holding
   it, the player readies one (4.1) and then attacks. Owning none (or only
   missile weapons), nothing happens: no blow, no move used. The attack
   ends the turn as 2.5. [research, unconfirmed] **Engine:** so; the
   readied weapon is the lowest-numbered melee weapon owned. On the world
   map there are no creatures to attack yet (movement 7.1).

## 3. A melee attack

1. **Blows**: 4 x the moves left / the full moves (rounded down), plus
   Dexterity / 8 (rounded down), minus 1; at least 1. The full moves are
   the turn's bar after any slowdown (and halving, creatures 2.6); if they
   are 0 the first term counts as 1. Counted once, as the attack starts,
   after any readying. [research] A fighter with Dexterity 13 and a bar of
   3 struck 4 blows with 3 moves left, 2 with 2. [capture: armed]
2. **Using them up.** Each blow uses one; **a blow that does damage uses
   two**. The attack ends when none are left, the defender dies, a weapon
   breaks (6.3) or magic defense calls it off (5.4). There is no separate
   extra blow at a turn's end; Speed counts only through the moves.
   [research] A hit on the first of 4 blows was followed by two more, not
   three; on the first of 2, by none. [capture: armed]
3. **One blow, in order:** [research]
   1. **To-hit**: a roll of 0 to 99; it hits below the hit chance (3.4).
   2. On a hit: the attacker's **Melee check** (5.5), then, if its armor
      term is below 0, its **Armor check** (5.5).
   3. **Parry**: a roll of 0 to 99, always drawn, even when the defender
      holds no weapon (its parry chance is then 0). It parries below its
      parry chance (3.5).
   4. Parried: the defender's **Parry check**; then a roll of 0 to 99, and
      below 30 the defender's Armor check (only with its armor term below
      0); then the striking weapon's **break roll** (6.3). No damage.
   5. Not parried: **damage** (section 4).
   6. **A miss draws a counter-blow** when the defender holds a melee
      weapon: the defender strikes back once at once, a whole blow (1 to
      5) with the roles swapped, its hit chance using the attacker's count
      of blows left. A counter that does damage uses one more of the
      attacker's blows; a parried or missed one uses none, and a missed
      counter draws nothing more (no counter to a counter).
   [capture: armed, every to-hit, Melee check, parry roll and heavy-blow
   roll of 122 blows in 32 runs, the defender holding no weapon]
   Parries and counters were seen only in the researcher's runs (the
   player holding the blade, 3 runs). [research, unconfirmed]
4. **The hit chance**, clamped: the weapon's attack adjustment (-35..40)
   + the attacker's armor term + its Melee skill + 5 x the blows left
   (counting this one) - 20 + its Power / 4 - its wound penalty. Only the
   attacker's numbers; the defender's come in through the parry.
   [research] With Melee 31 and Power 13, unhurt and unarmored: 34, 29,
   24, 19 down four blows. [capture: armed, 122 to-hit rolls]
5. **The parry chance**: 0 unless the defender holds a melee weapon.
   Otherwise, clamped: its Parry skill + half its weapon's attack
   adjustment + half of minus its wound penalty + its armor term + half
   its Power + 15 - its Size. [research, unconfirmed: the researcher's
   runs, Parry 26, Power 13, Size 9: 38]

## 4. Damage

1. With the striking weapon's Power P: [research]
   1. **Build**: Dexterity + Size - 6, held to 0..30, then - 15 (so
      -15..15).
   2. **Damage** = Build x P / 30 + P. [unknown] how a negative Build
      rounds. **Engine:** down (toward minus infinity).
   3. **Heavy blow**: a roll of 0 to 99; below the attacker's Dexterity,
      add half of P.
   4. At most 3 x P / 2.
   5. **Armor** (5.1), then **magic defense** (5.4).
2. **What damage D takes from the defender**: the Power share is the
   smaller of D x Power / (Power + Life Force) and (Power + 1) / 2. Power
   falls by twice the share, Life Force by D minus the share, neither below
   0. Life Force 0 is death ([magic](magic.md) 7) and ends the attack.
   [research] Our blade of Power 10 with Dexterity 13 and Size 9 did 10, or
   15 as a heavy blow; the player at Life Force 40, Power 13 lost 8 Life
   Force and 4 Power for a 10, Life Force read after every hit and
   healing. [capture: armed, 25 hits in 32 runs]
3. So Life Force above Constitution plays no part in a blow, nor does
   Strength. A wounded individual fights worse through the wound penalty
   (3.4, 3.5); Power counts in to-hit, parry and the share. [research]
4. **Engine:** a blow whose damage comes to 0 after armor and magic
   defense is a hit that does no damage: it uses one blow, as a miss, and
   is not a "damaging blow" (6.4, 7). [unknown]

## 5. Armor, magic defense and learning

1. **Armor against the weapon**, when the defender wears armor and the
   damage is above 0: [research, unconfirmed]

   | Weapon | Armor | Result |
   | --- | --- | --- |
   | not magic | not magic | the armor's Power is taken off |
   | not magic | magic | damage halved (D + 1, halved), then the armor's Power taken off |
   | magic | not magic | the armor is ignored |
   | magic | magic | the armor's Power is taken off |

   Not below 0. When the armor took part and the damage left is 0 or more,
   the armor rolls to break (6.3).
2. **Armor that disappears after use** goes after any blow it meets (the
   rows that take its Power off). [research, unconfirmed]
3. **Armor in the hit chance**: the attacker's armor term (1.5), so heavy
   armor makes its wearer miss until its Armor skill grows. [research]
4. **Magic defense** (levels 0..3, [magic](magic.md) 3.3), only when the
   damage is still above 0: level 1 halves a non-magic weapon's damage
   (D + 1, halved); levels 2 and 3 stop a non-magic weapon: the blow does
   nothing and the rest of the attack is called off; a magic weapon is
   halved at level 3 only. [manual: the levels; research, unconfirmed]
   **Engine:** the call-off ends the whole attack, also when it stops a
   counter-blow.
5. **Learning**: a skill goes up by 1 (at most 127) when a roll of 0 to 99
   is above it: Melee after each hit, Parry after each parry. So the
   higher the skill, the rarer the gain, and from 99 never. The Armor
   check is different: a gain when a roll of 0 to 99 is below 65 minus
   Armor skill, clamped (so still 14 in 100 when high), and only while the
   armor term is below 0. [research; capture: armed, 25 Melee checks]

## 6. Readying and breaking

1. **Readying a weapon** costs one move when the weapon in hand changes;
   **readying armor** ends the turn when the armor worn changes (as
   [inventory](inventory.md) 4.3). [research, unconfirmed] **Engine:** so,
   for players' `Ready` and creatures alike; with no move left it still
   readies.
2. **A creature's readying** at the top of its turn (after drift and the
   healing die): with an aggressive individual or a player that is its
   foe ([creatures](creatures.md) 7.4) beside it, it readies an owned melee
   weapon it doesn't hold; with none beside it, an owned missile weapon.
   Otherwise it keeps what it holds. One move each change. [research,
   unconfirmed] **Engine:** beside means the 4 squares (as 2.1); the
   lowest-numbered of the kind ([manual]).
3. **Breaking** is a roll of 0 to 99 below the thing's break chance
   (0..15): a melee weapon when its blow is parried (the striking weapon,
   not the parrying one); armor on a blow that reaches it (5.1). The thing
   is taken from its owner (not dropped) and leaves the hand or body. A
   weapon breaking ends the attack. [research, unconfirmed] **Engine:** a
   counter-blow's weapon breaking takes it from the defender and the
   attack goes on.
4. **Disappearing after use**: a melee weapon after its first damaging
   blow, armor as 5.2, taken from the owner as on breaking. [research,
   unconfirmed]

## 7. After a player's damaging blow

Right after the damage, by the defender's class: [research, unconfirmed]

1. a friend (aggressive or peaceful) becomes **neutral**, keeping its
   setting;
2. else a peaceful thief, neutral or enemy becomes **aggressive**;
3. else an aggressive neutral becomes an **enemy** if it passes the
   smartness check (Wisdom below 6 never, above 14 always, otherwise a
   roll of 0 to 9 below Wisdom minus 5);
4. an aggressive thief or enemy, or a player: no change.

One change a blow. A friend hit once is a neutral; hit again, it turns
aggressive. Only a player's own blows do this, never a creature's or a
counter-blow. [manual: a friend hurt turns neutral, a peaceful creature
aggressive, a smart aggressive neutral enemy]

## 8. Left for later

- **Firing** missile weapons: when a creature fires, range, the line, the
  aim's spread, dodging, the missile flying on. The researcher traced it
  but did not run it, and the aim's geometry needs a run. **Engine:**
  missile weapons are never fired; a fighter owning only missile weapons
  never attacks (2.3).
- **Mimics** copying their opponent.
- Map encounters fought on the world map.
- The square a dropped treasure lands on (creatures 8.1).

## Captures needed

1. The attack decision, blows, to-hit, damage, the share and Melee
   learning: captured (armed, 32 runs, every roll). Still open: attacks on
   individuals other than the goal (2.4), creature classes besides a
   fighter enemy.
2. Parry, counter-blows, Parry learning: a defender holding a melee
   weapon (the researcher's runs only).
3. Armor, its break roll, magic defense, a breaking weapon, things that
   disappear after use.
4. The player's attack, readying on contact, the alignment changes (7).
5. Firing (section 8).
