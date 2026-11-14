# Rules: combat

Behaviour spec for combat: who attacks whom, a blow from start to end,
parrying, damage, armor, magic defense, learning, readying and breaking,
creatures turning hostile when a player hurts them, and firing missile
weapons. Licensed CC BY 4.0. The engine implements it in
`crates/acs-engine` (`combat.rs`: `attack` and `fire`, with the decision
to attack or fire in `ai.rs` and the player's attack in `step.rs`).

Tags as in [movement](movement.md): **[manual]**, **[data]**,
**[designer]**, **[unknown]** with the engine's choice as **Engine:**,
plus **[research]** for the clean-room researcher's first and second
combat passes (2026-11-13, `docs/research/combat.md`) and **[capture]**
where our own captured runs confirm it. The researcher marked each
finding as run in the original (its experiments) or only read in the
program ("traced"); "[research, unconfirmed]" means no capture of ours has
checked it yet. The second pass's cases (capture.md 10.15) were checked
roll by roll with `scripts/combatcheck.py --case NAME` (capture.md
10.14.10): 128 creature attacks, 24 player attacks and 32 firing runs,
every one right.

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
   a melee weapon, the attack starts at once, its blows from the moves
   left (3.1), and ends the turn as 2.5. Owning none (or only missile
   weapons), nothing happens: no message, no blow, no step, no roll. [research:
   second pass, run] A player holding the blade with 5 of 6 moves left
   struck 3 blows at a friend east of it, every roll as section 3 says;
   pushing the stick at a neutral with no weapon owned did nothing, no
   roll drawn and nobody moved. [capture: player-friend, player-none, 12
   runs each, capture.md 10.14.10] Whether that push used a move: the
   researcher saw none used (the bar); not measured by us.
8. **Owning a melee weapon but holding none**, the player gets a message
   ("select a melee weapon to use"), then a menu of NONE (first, chosen
   at first) and the melee weapons owned. Choosing one readies it at one
   move and the attack follows at once, its blows from the moves left
   after readying; choosing NONE uses no move, makes no attack and gives
   the turn back. [research: second pass, 1 run each way]
   **Engine:** the menu is the front end's; `Game` readies the
   lowest-numbered melee weapon owned, then attacks. On the world map
   there are no creatures to attack yet (movement 7.1).

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
   Parries and counters too: with both holding the blade, 140 to-hit
   rolls and 101 counter-blow to-hit rolls, 58 parry rolls (18 parries,
   each followed by the Parry check, the 30-in-100 roll and the break
   roll), and 40 damaging blows, every Life Force, Power and Parry value
   after them. A damaging counter used one more of the attacker's blows;
   a parried or missed one none. [capture: parry, break, 32 runs,
   capture.md 10.14.10]
4. **The hit chance**, clamped: the weapon's attack adjustment (-35..40)
   + the attacker's armor term + its Melee skill + 5 x the blows left
   (counting this one) - 20 + its Power / 4 - its wound penalty. Only the
   attacker's numbers; the defender's come in through the parry.
   [research] With Melee 31 and Power 13, unhurt and unarmored: 34, 29,
   24, 19 down four blows. [capture: armed, 122 to-hit rolls]
5. **The parry chance**: 0 unless the defender holds a melee weapon.
   Otherwise, clamped: its Parry skill + half its weapon's attack
   adjustment + half of minus its wound penalty + its armor term + half
   its Power + 15 - its Size. [research] Parry 26, Power 13, Size 9,
   unhurt and unarmored: 38. [capture: parry, break] The armor term here
   (and in the hit chance) has not been run: no case had a wearer that
   strikes or parries. [research, unconfirmed]

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
   damage is above 0: [research]

   | Weapon | Armor | Result |
   | --- | --- | --- |
   | not magic | not magic | the armor's Power is taken off |
   | not magic | magic | damage halved (D + 1, halved), then the armor's Power taken off |
   | magic | not magic | the armor is ignored |
   | magic | magic | the armor's Power is taken off |

   Not below 0. When the armor took part and the damage left is 0 or more,
   the armor rolls to break (6.3), right then, also at a break chance of 0.
   When it breaks the attack goes on. Mail of Power 2 against our blade of
   Power 10: plain, 10 did 8 and a heavy 15 did 13; magic, 10 did 3 and 15
   did 6 (halved, rounding up, then 2 off). A break roll after every one
   of the 41 damaging blows; the plain mail (break chance 15) broke once,
   left its wearer, and the next blow did the full 10. [capture: mail,
   mail-magic, 32 runs, capture.md 10.14.10] Not run: a magic weapon
   against either, a blow weaker than the armor (no break roll expected).
2. **Armor that disappears after use** goes after any blow it meets (the
   rows that take its Power off). [research, unconfirmed]
3. **Armor in the hit chance**: the attacker's armor term (1.5), so heavy
   armor makes its wearer miss until its Armor skill grows. [research]
4. **Magic defense** (levels 0..3, [magic](magic.md) 3.3), only when the
   damage is still above 0: level 1 halves a non-magic weapon's damage
   (D + 1, halved); levels 2 and 3 stop a non-magic weapon: the blow does
   nothing and the rest of the attack is called off; a magic weapon is
   halved at level 3 only. [manual: the levels; research] Level 1 with
   our plain blade: 10 did 5, 15 did 8 (13 blows); level 2: every hit
   (15) did nothing and no blow followed it, with blows still due; level
   3 with the blade made magic: halved (15 blows). [capture: md1, md2,
   md3-magic, 48 runs, capture.md 10.14.10] Not run: level 3 against a
   plain weapon, armor and magic defense together (the order above is
   traced). **Engine:** the call-off ends the whole attack, also when it
   stops a counter-blow.
5. **Learning**: a skill goes up by 1 (at most 127) when a roll of 0 to 99
   is above it: Melee after each hit, Parry after each parry. So the
   higher the skill, the rarer the gain, and from 99 never. The Armor
   check is different: a gain when a roll of 0 to 99 is below 65 minus
   Armor skill, clamped (so still 14 in 100 when high), and only while the
   armor term is below 0. [research; capture: armed, 25 Melee checks;
   all second-pass melee cases, 154 Melee and 18 Parry checks] The Armor
   check has not been run. [research, unconfirmed]

## 6. Readying and breaking

1. **Readying a weapon** costs one move when the weapon in hand changes;
   **readying armor** ends the turn when the armor worn changes (as
   [inventory](inventory.md) 4.3). [research, unconfirmed] **Engine:** so,
   for players' `Ready` and creatures alike; with no move left it still
   readies.
2. **A creature's readying**, after its smartness checks and before its
   first step: with an aggressive individual or a player that is its
   foe ([creatures](creatures.md) 7.4) beside it, it readies an owned melee
   weapon it doesn't hold; with none beside it, an owned missile weapon.
   Otherwise it keeps what it holds. One move each change. [research:
   traced; the melee case run by the researcher, 4 runs, a snapshot changed
   by hand: one move, then a step into the player and 2 blows]
   **Engine:** beside means the 4 squares (as 2.1); the lowest-numbered of
   the kind ([manual]). The engine readies after the smartness check for
   casting (creatures 6.6), so a creature that casts doesn't ready; on a
   turn with an overburdened drop (creatures 8.1) it readies before
   idling. [unknown] where readying falls on such a turn.
3. **Breaking** is a roll of 0 to 99 below the thing's break chance
   (0..15): a melee weapon when its blow is parried (the striking weapon,
   not the parrying one); armor on a blow that reaches it (5.1). The thing
   is taken from its owner (not dropped) and leaves the hand or body. A
   weapon breaking ends the attack. [research] With the blade's break
   chance 15 and both holding it, 9 parries brought 9 break rolls; one
   broke the player's blade on its parried counter-blow, its held weapon
   went to none, and it made no counter-blow after. [capture: break, 16
   runs] That the break ends the attack was not seen (it came on the last
   blow), nor the message. [research: traced] **Engine:** a counter-blow's
   weapon breaking takes it from the defender and the attack goes on.
4. **Disappearing after use**: a melee weapon after its first damaging
   blow, and **the attack ends there**, also with blows still due; armor
   as 5.2; taken from the owner as on breaking. [research] The blade set
   to disappear went at the enemy's first damaging blow in 11 of 16 runs
   (one more on the runs' last roll) and no blow followed it. [capture:
   once, 16 runs]
5. **After a weapon has disappeared**, the creature that held it (owning
   no melee weapon now, so the player's square is closed, 2.3) had one
   move fewer on each later turn that began beside the player: Speed 3,
   two steps round the player, then its turn ended. Every such turn of
   ours (10) began beside the player, so whether being beside matters is
   not known; it looks like readying (6.2) charging its move with nothing
   to ready. [capture: once, 16 runs; 5 of them need it] [unknown]
   **Engine:** not modelled: such a creature keeps all its moves.

## 7. After a player's damaging blow

Right after the damage, by the defender's class: [research]

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
aggressive, a smart aggressive neutral enemy] A peaceful friend hit by the
player turned a peaceful neutral (10 runs), and an aggressive neutral
when hit twice in one attack (2). An aggressive neutral of Wisdom 10 took
one roll of 0 to 9 right after each damaging blow (none after a miss)
and turned an aggressive enemy on a roll below 5 (4 of 11 blows).
Read from the creature's class byte, every blow. [capture: player-friend,
and player-friend with the creature made an aggressive neutral of Wisdom
10, 12 runs each, capture.md 10.14.10] An enemy's arrow hitting its own
friend changed nothing about it. [research: second pass, 4 runs]

## 8. Firing

The engine implements this in `combat.rs` (`fire`, `flight`) with the
decision in `ai.rs` (`creature_fires`); the events are `Fired`, `Shot`,
`Dodged`, `MissileHit` and `MissileStopped`.

1. **When a creature fires**: after its smartness checks, instead of
   stepping, when all of these hold: it holds a missile weapon; its goal
   is an individual; the goal is within the weapon's range (2..9),
   counted as the larger of the steps across and down; no aggressive
   individual or player not on its side stands beside it; and a straight
   line to the goal crosses no wall or obstacle. [research: traced] An
   enemy holding our bow (range 7) with the player 5 squares west fired
   on every turn and never stepped. [capture: bow, bow-line, 32 runs,
   capture.md 10.14.10] Behind an obstacle on the line it never fired and
   walked instead (16 runs). But beside the player, holding only the bow,
   it hurt the player in 13 of 16 runs without moving, against "no foe
   beside". [capture: bow-wall, bow-beside, capture.md 10.16; asked in
   the combat brief's pass 3] [unknown] A player can't fire: the program
   gives a message. [research: traced] **Engine:** "beside" is the 4 squares (as 6.2); "not
   on its side" is a foe ([creatures](creatures.md) 7.4); the straight line
   is the flight of a shot without deviation (8.5), the squares before the
   goal's. [unknown] whether the goal must be a foe; the engine fires only
   at one, so a slinker friend whose goal is the player doesn't shoot it.
   The player's `Fire` command gives `CannotFire` (our own text), no shot,
   no move used.
2. **Shots**: Dexterity / 8 (at least 1) + half the moves left. The first
   shot ends the turn (moves left go to 0). [research] Dexterity 13 and 3
   of 3 moves: 2 shots. [capture: bow, bow-line, every turn]
3. **Each shot, in order**: the weapon's **break roll** (a roll of 0 to
   99, drawn even at break chance 0; a break ends the firing, 6.3); the
   **deviation**, a roll of 0 to spread - 1 (rolls of any size work like
   the percentile: the raw number divided by 256 / size, rounded down,
   drawn again when too big); the **side**, a roll of 0 or 1. Then the
   flight (8.5). [research; capture: every shot's draws, in order, each
   shot starting as the attack's count byte changes] **Engine:** the
   deviation from the engine's one `Rng`; each shot aims at where the
   target stands then, and the firing stops early when the target is dead
   or out of the room (with one target, as captured, that changes
   nothing).
4. **The spread** = 100 - accuracy, clamped. **Accuracy** = Missile skill
   + the weapon's attack adjustment + the shooter's armor term + half of
   (half its Power - its wound penalty) - (5 x the shots still to come
   after this one - 20); so the first of 2 shots gets + 15, the last + 20.
   [research: second pass, a correction to the first] Missile 22, Power 13,
   unhurt: spreads 60 then 55, and 54 or 53 as Missile grew. [capture]
5. **The flight**: the aim is moved sideways, at right angles to the line
   from the shooter to its target, by deviation hundredths of the distance
   to the target, to the side the side roll picks. The missile flies
   straight from the middle of the shooter's square along that line until
   it has gone the weapon's range in squares along the longer axis, or is
   stopped. It reaches every individual whose square its path crosses, in
   order, friend or foe, never the shooter. Walls and obstacles stop it.
   [research: traced] At 5 squares the player was reached at deviations up
   to 8, missed from 11; a friend 3 squares out was reached at 17, so the
   offset is not compared exactly with half a square (17 hundredths of 5
   squares is 0.51 square at 3 squares out). [capture: bow, bow-line, 64
   shots] [unknown] the exact arithmetic. **Engine:** the offset at the
   target in eighths of a square, rounded down, scaled to each square on
   the way; a square is reached while that is below half a square (fits
   all 64 shots, as do quarters and sixteenths). The distance is the
   larger of the steps across and down; off the axes the offset is split
   across and down at right angles to the line, each part truncated toward
   it; a missile whose point passes half a square from the line lies on
   the next square over, and reaches whoever stands there. Walls are the
   room's edge squares, obstacles an obstacle or custom obstacle on top of
   a square; a stopped missile gives `MissileStopped`.
6. **Reaching an individual**: the shooter's Missile check (5.5), then the
   **dodge**: a roll of 0 to 99 below the dodge chance, clamped: its Dodge
   skill + 2 x its armor term + half its Power - its wound penalty + 3 x
   (10 - its Size). A dodge brings its Dodge check (5.5) and the 30-in-100
   roll (below 30 its Armor check, only with its armor term below 0), and
   the missile flies on. Otherwise the missile does damage as a blow
   (section 4: the heavy-blow roll on the shooter's Dexterity, armor, magic
   defense, the Power share), turns the target hostile if a player shot
   (7), and stops, also when the damage came to 0. [research] Dodge 30,
   Power 13, Size 9, unhurt: 39. 15 dodge rolls (5 dodges, 10 hits), a
   friend in the line hit by its own side's arrows, and an arrow dodged by
   the friend going on to the player behind. [capture: bow, bow-line]
   **Engine:** two on one square are reached in the engine's order of
   individuals (players, then creatures by slot).
7. **Disappearing**: a missile weapon set to disappear after use goes after
   each shot. [research: traced]

Not run: a wall or obstacle in the line, a diagonal shot, a foe beside the
shooter, a bow breaking, the player's attempt to fire, a creature readying
a missile weapon (6.2).

## 9. Left for later

- Firing's open points (section 8, "Not run"), and the engine's choices
  there.
- **Mimics** copying their opponent.
- Map encounters fought on the world map.
- The square a dropped treasure lands on (creatures 8.1).

## Captures needed

1. The attack decision, blows, to-hit, damage, the share and Melee
   learning: captured (armed, 32 runs, every roll). Still open: attacks on
   individuals other than the goal (2.4), creature classes besides a
   fighter enemy.
2. Parry, counter-blows, Parry learning, armor against damage and its
   break roll, magic defense 1 to 3, a breaking weapon, a weapon that
   disappears: captured (capture.md 10.14.10). Still open: the armor term
   in its wearer's own to-hit and parry and the Armor check, a blow weaker
   than the armor, armor that disappears, magic defense 3 against a plain
   weapon, armor with magic defense, a weapon breaking with blows still
   due, and why a creature whose weapon disappeared loses a move (6.5).
3. The player's attack, its blows and the alignment changes (7):
   captured. Still open: the menu when the player owns but doesn't hold a
   weapon (2.8), and whether a push with no weapon uses a move.
4. Readying before stepping (6.2): the researcher's experiment only.
5. Firing (section 8): the rest of its "Not run" list.
