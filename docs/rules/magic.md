# Rules: magic

Behaviour spec for spells: what each of the 15 spell actions does, who it
acts on, casting spells and using magic items, Power drifting toward Wisdom,
temporary Life Force, natural healing, creatures casting, and death (which
spells can now cause). Licensed CC BY 4.0. The engine implements it in
`crates/acs-engine` (`magic.rs`, with the creature side in `ai.rs`).

Tags as in [movement](movement.md): **[manual]** paraphrased from the
manual, **[data]** from the stored format or corpus counts, **[designer]**
from Baber's Jadeworld notes, **[research]** from the clean-room
researcher's plain-language findings (CONTRIBUTING.md 3;
`docs/research/`), **[unknown]** with the engine's current choice as
**Engine:**. Counts cover all 15 corpus adventures.

## 1. Where spells come from

1. A spell is one of 15 actions with one parameter byte. It can be a thing of
   its own (a *magic spell*, cast by its owner) or hang on a magic item, a
   portal, a space, a custom space, an obstacle, a custom obstacle or a
   terrain type. [manual] Every spell can show a short message, which
   appears before the spell takes effect. [manual]
2. Spell actions in the corpus, by what carries them: 364 magic spells
   (312 change Life Force, 26 change Power), 134 magic items (57 Life Force,
   39 give, 13 summon), and 535 on doors, spaces and obstacles, mostly music
   (143), give (47), Activate All (46) and messages (36). [data]
3. Movement and inventory already find each spell and resolve its parameter
   (custom spaces and custom obstacles take it from the placement):
   [movement](movement.md) 5.5, 9; [inventory](inventory.md) 2.6, 5.6, 5.7.
   This spec says what happens next.

## 2. Who a spell acts on

1. A spell set off by a space, obstacle, door or terrain takes effect at that
   place; a spell that applies to an individual acts on the one who set it
   off (who stepped on it, bumped it or passed through). [manual]
2. A magic item's pick-up spell acts on the one who picked it up. [manual]
3. A magic item dropped next to its owner acts on the owner, unless it is
   dropped on a square with a creature, which it then acts on. [manual]
   **Engine:** a drop on the owner's own square also acts on the owner.
4. When a spell is cast, or a magic item used, the caster picks the target.
   [manual] **Engine:** the caster itself or any individual in the same room
   (players and creatures); on the world map only the caster. A target that
   is not there refuses the command at no cost (`CastRefused`).
5. Spells that act on the room (Kill All, Summon or Banish, Rid Room, Add to
   Room, Activate All) ignore the target. [manual]

## 3. The 15 actions

The parameter layout is the format draft's; numbers below are corpus counts.

1. **Kill all but an owner of** (thing). Everyone who does not own the thing
   dies. [manual] Who "everyone" is: [unknown] **Engine:** every individual
   in the room where the spell takes effect, the caster included unless it
   owns the thing; on the world map only the individual it acts on. 6 uses.
2. **Summon or banish** (creature number, 0 = all). [data: draft] 0 banishes
   every creature in the room at once, and banished creatures take what they
   carry with them. [manual] A creature that casts it banishes everyone but
   itself. [manual] **Engine:** players are never banished. A number summons
   that master creature to the room; summons are answered on the following
   turn, never on the world map, and not while the region holds 16 creatures.
   [manual]
   - When "the following turn" is: [unknown] **Engine:** at the start of the
     next player turn after the spell, whoever's turn it is.
   - Where it appears: Baber notes that creatures will step onto a space where
     a summoned creature is due to arrive, so arrival has a square.
     [designer] **Engine:** the square where the spell took effect (for a
     cast or a used item, the caster's square), or if someone stands there,
     the nearest inside square with no individual and no thing (creatures
     5.4). With none free, or no free slot, the summons goes unanswered
     (`SummonUnanswered`).
   - What arrives: a copy of the master creature, as a resident in the
     lowest free slot (not a random creature, so it is never removed for
     being left behind). [unknown] Baber reports that summoning a creature
     from outside the region's own list can corrupt names or crash the
     original; that is a bug the engine does not copy. [designer]
   - 30 named summons and 17 banish-alls among master things; 82 of 83
     custom placements name a creature. [data]
3. **Increase magic defense, but not above** (0..3: none, non-magic weapons
   do half damage, only magic weapons hurt, magic weapons do half damage).
   [manual] How far one spell raises it: [unknown] **Engine:** one level,
   never past the parameter; nothing if already there.
4. **Decrease magic defense, but not below**: the mirror image. [manual]
5. **Increase the victim's** trait or skill (parameter 0..9: Constitution,
   Strength, Dexterity, Speed, Wisdom, Dodge, Parry, Armor, Melee, Missile).
   Traits rise by 1, skills by 5, permanently. [manual] **Engine:** traits
   stop at 31 (Speed at 15), skills at 127 (the manual's maxima); a
   parameter above 9 does nothing (3 corpus magic items hold 32). [data]
6. **Decrease the victim's** trait or skill: as above, down to 0. [manual]
7. **Change Power by** -63..+63 (bits 0-5 the amount, bit 7 negative).
   Power always drifts back toward Wisdom, so the temporary/permanent bit
   (bit 6) makes no difference. [manual] **Engine:** Power stays within
   0..127. 52 uses.
8. **Change Life Force by** -63..+63, the same layout. Life Force raised
   above Constitution by a permanent change does not decay; a temporary
   change wears off. [manual] **Engine:** see section 5. Life Force stays
   within 0..63, and at 0 the individual dies (section 7). 432 uses, 367 of
   them negative. [data]
9. **Give the victim one** (thing). [manual] **Engine:** as a pick-up
   within the carry limits ([inventory](inventory.md) 1.2); at the limit
   nothing is given (`Given` with `kept: false`). Gold gives one piece. A
   weapon given is not readied. [unknown] 94 uses.
10. **Display long message** (message 1..255). [manual] **Engine:** a
    `LongMessage` event with the text; 0 shows nothing.
11. **Play music** (one of 30). Choices 1-12 are effects, 13-30 themes, and
    14-17 loop until other music plays. [manual] Placed parameters run 0..29,
    so the byte is the choice minus one. [data] **Engine:** a `Music` event
    with the 1-based choice; the front end plays it. 151 master uses and
    the most common custom placement.
12. **Rid room of every uncarried** (thing). Every copy lying in the room
    goes, in stacks too. [manual] On the world map: nothing. [unknown]
13. **Add to room one** (thing). Takes effect at once. [manual] Where it
    lands: [unknown] **Engine:** on top of the stack where the spell took
    effect if that is an inside square, else on the nearest inside square
    with no thing and no individual. So a space that adds a key puts the key
    on top of itself, and stepping off and on again picks it up without
    setting the space off again (movement 5.3). Nothing on the world map.
14. **Activate all things at this place** runs, top to bottom, every spell in
    the stack below the thing that holds it, and only those. [manual]
    **Engine:**
    - a stack's spells are those of spaces that invoke a spell, custom spaces
      set to "when moved onto", obstacles that invoke a spell when bumped,
      custom obstacles, portals that invoke a spell, and magic items and
      magic spells lying there;
    - custom spaces open to owners (or non-owners) of a thing run only when
      the individual meets the condition, which is how "Let There be Light"
      works [manual]; their needed thing is not used up here; "when dropped
      here" spaces do not run [unknown];
    - another Activate All lower down is skipped, so a stack runs once;
    - cast, used or picked up away from a stack, it runs the whole stack on
      the square where it takes effect.
15. **Do nothing**: only the message. [manual] A spell with an action code
    outside 0..14 (one corpus key spell holds 15) does nothing too. [data]

Spells "may be made to disappear after first use" (magic spells and magic
items). [manual] **Engine:** a magic spell goes after a successful cast, a
magic item after it is used. [unknown]

## 4. Casting and using

1. *Use Power* casts a magic spell the character owns; success depends on
   the spell's cost (0..127) against the caster's Power, and casting uses up
   Power. [manual] The formula is not given. On the C64: a cost of 0
   takes no roll, spends nothing and always works. Otherwise Power is
   spent first, the cost or all of the caster's Power if that is less;
   the chance in percent is 90 x the Power spent / the cost, rounded
   down, then raised to 14 if lower and cut to 90 if higher; and the
   spell works when a roll of 0..99 is below the chance. So a cost at or
   below Power works 90% of the time, never 100%, and no cost falls below
   14%. [research: findings of 2026-11-11, `docs/research/random-rolls.md`]
   Confirmed exactly: with the random state set to each of its 256 values
   just before the cast, the researcher's generator (the brief's Findings)
   with this chance predicted every one of 200 first-draw casts
   at each of Power 20 against cost 10 (90%), 60 (30%) and 90 (20%) and
   Power 10 against cost 20 (45%); worked over all 256 states: 231, 74,
   50 and 117. [capture: cast --set-random-state, capture.md 10.9] The
   earlier statistical fit, a roll of 0..cost below Power, was within 10
   points wherever the grid looked but is wrong: it makes Power 20 against
   cost 10 certain, and 25 of 256 such casts failed. [capture: cast, 60
   runs at each of ten Power/cost pairs, jitter over about 16 s; the
   90 x Power / cost chance in brackets] Power 10: cost 10 85% (90%),
   cost 20 62% (45%). Power 20: cost 10 100% (90%), 20 92% (90%), 30 67%
   (60%), 40 58% (45%), 60 32% (30%), 90 10% (20%). Power 27: cost 31 83%
   (78%), cost 62 42% (39%). The roll mixes in the raster line at the
   moment it is taken, and from a given screen only a few lines are
   reachable, so these grids sampled few distinct rolls (outcomes came in
   streaks along the jitter). A cost 0 spell always worked; a failure shows
   "A FEW SPARKS FIZZLE BUT ... FAILS." A failure costs as much as a
   success: at Power 20 against cost 30, all 60 casts (40 worked, 20
   failed) left Power at 0, read as 1 after the turn's drift; half the
   cost would have left 6. [capture: cast with the profile read back,
   60 wide-jitter runs] **Engine:** the rule above (`cast_chance`,
   `cast_spend`, `cast_roll`): a cost of 0 always works and spends
   nothing; otherwise the cost or all of Power comes off first, whether
   or not the spell works, and a roll of 0..99 below the chance works.
   `SpellCast` reports that chance (100 for a cost of 0). The engine's
   roll is its own seeded generator, not the original's (which mixes in
   the raster line), so only the chance matches, not the sequence.
   Corpus costs run 0..60, median 13. [data]
2. *Use Object* uses a magic item that is set to act when used; magic items
   always work. [manual] Some must be dropped instead. [manual] The manual
   says using magic objects lowers Power but gives no amount. On the C64 a
   use costs 2 Power, at Wisdom 10 and 20 alike, and again on a second use;
   using does not end the turn: the bar goes on draining afterwards.
   [capture: item, 3 runs at each Wisdom, two uses a run, the profile
   screen] A use costs exactly two steps: after one use a Speed 15 player
   walked 13 squares before its turn ended, after three uses 9, against 15
   after reading the profile instead; a Speed 6 player walked 4 against 6,
   and three uses took its whole turn. [capture: acts, table use, 10 and 5
   runs] **Engine:** an item without the "used" trigger refuses
   (`CastRefused`, `not_usable`); a use costs 2 Power (not below 0) and
   two steps. 127 of 134 corpus magic items act when used. [data]
3. **Cost in moves.** Casting ends the caster's turn, whether the spell
   works or not, however much of the movement bar is left. [capture: cast,
   600 casts with 5 to 28 pixels of bar left; the next turn came at the
   same moment after each] Using a magic item does not end the turn
   (4.2). **Engine:** casting ends the caster's turn, like readying armor
   (inventory 4.3); using spends two steps, and ends the turn when
   fewer are left. A refused command costs nothing.
4. Magic spells can't be dropped (inventory 5.2), so a spell is only lost by
   disappearing after use, by banishment or by death.

## 5. Power, Life Force and healing over time

1. **Power drifts.** Each turn an individual's Power moves a little toward
   its Wisdom. [manual] On the C64: 1 point at the top of each of its turns,
   up or down, stopping at Wisdom, every time. [capture: drift, Power 10
   and 30 against Wisdom 20, 3 runs each, the profile read every turn: both
   reached 20 after 10 turn ends in every run] It takes no roll, and
   comes just after the bar is refilled. [research: random-rolls second pass, 2026-11-12, `docs/research/random-rolls.md`] **Engine:** the
   same, at the top of each turn. A
   player joining an adventure starts with Power equal to its Wisdom, at
   most 27, whatever its saved Power; above 27 it then drifts up to its
   Wisdom like any other Power. [capture: retired
   players of ours with Wisdom 13, 20, 25 and 26 to 31, profile screen:
   Wisdom 28 to 31 showed 27 on the first turn, then 28, 29 on the next]
   **Engine:** a joining player's Power is its Wisdom, at most 27; a
   `PlayerSetup` has no Power of its own.
   A joining player's Life Force is raised to its Constitution if below it,
   and kept if above. [capture: Life Force 5 under Constitution 20 showed
   20; 22 over 21 showed 22; one player each] **Engine:** the same.
2. **Temporary Life Force.** A temporary change wears off; the manual's
   "disease" stack (a temporary +10 then a permanent -10) leaves Life Force
   unchanged at first, after which the 10 points tick away about twice as
   fast as natural healing. [manual] Bit 6 of the parameter set means
   temporary: +10 with it wore off, +10 without it stayed for 60 turns.
   [capture: drift] On the C64 a temporary change of N moves Life Force by
   N at once and queues a reversal: after a die roll of 1 to 6 **rounds**
   (a round is everyone in play taking a turn,
   [creatures](creatures.md) 2.2), Life Force moves 1 back toward where it
   was, and again after a new die each time, for every count from N down
   to 0. So a temporary +10 is taken back 11 times, once more than it was
   given: that is the "eleventh tick", which leaves Life Force 1 below
   where it started until healing (5.3) puts it back. Nothing else changes
   the delay: not Speed, time, walking or the size of the change. The
   queue holds 30 reversals; a temporary change made while it is full is
   never taken back. [research: random-rolls second pass, 2026-11-12, `docs/research/random-rolls.md`] Confirmed exactly: with the random
   state set to each of its 256 values at the end of a round with a
   reversal due, the next one fell due 1 to 6 rounds later as that roll
   predicted, in all 252 runs that needed no redraw. [capture: mem,
   capture.md 10.12] Every gap between ticks in the 19 earlier temporary
   +10 runs was 1 to 6 turns of a lone player (one turn a round), and an
   eleventh fall of 1 showed in every run long enough; it was hidden in
   3 of 13 when the next turn's healing (5.3) put it back first.
   [capture: drift and drift --walk] Not settled: an individual out of
   play when its reversal falls due (the researcher read a 10-round
   delay), a temporary fall of Life Force (by the rule it ends 1 above
   where it started), other stats. **Engine:** the rule above: a queue of
   up to 30 reversals per game, each with its individual, its step (1
   toward undoing the change) and its count; at the end of each round
   every due reversal takes its step, then, with count left, counts down
   and is due again a die roll of rounds later. A dead individual's
   reversals are dropped. `temporary_life` reports what is still to be
   taken back (11 just after a temporary +10).
3. **Natural healing.** Life Force below Constitution heals on its own, about
   6 times slower than resting. [manual] On the C64 there are two chances,
   both only while Life Force (temporary part and all) is below
   Constitution, 1 point each, never above it:
   1. each **idle move**, a move of the bar used up by waiting, takes a
      roll of 0 to (the turn's full moves - 1) and heals on a 0: 1 in the
      turn's full moves. Walking, using an item or casting takes no such
      roll. So a turn waited out in full heals 1 on average whatever the
      Speed (a Speed 1 turn always 1);
   2. the **top of each turn**, just after drift, rolls a die and heals on
      a 2: 1 in 6, needed or not.
   [research: random-rolls second pass, 2026-11-12, `docs/research/random-rolls.md`] Confirmed exactly: with the random state set to each of
   its 256 values one idle move before a Speed 15 turn's end (Life Force
   6, Constitution 20), the idle move's roll and the next turn's die
   predicted every gain of 0, 1 or 2 in all 253 runs that needed no
   redraw. [capture: mem, capture.md 10.12] It explains the earlier
   captures together: a Speed 1 player healed 1 a turn and now and then
   2; a Speed 15 player healed from 6 to 20 in about 12 turns waiting, 0 to
   5 a turn, but only 4 to 7 points in 25 turns when walking every square.
   [capture: drift and drift --walk, 3 runs each] The manual's "twice as
   fast" does not hold: on average healing gives 7 points in 6 waited
   turns, the reversal of a temporary +10 1 in 3.5 rounds. **Engine:** the
   rule above. A `Wait` uses up the turn's remaining moves as idle moves,
   one healing roll each; a turn ended by walking its last move, casting
   or readying armor takes none; creatures roll for the moves their turn
   leaves unused ([research, unconfirmed]: the researcher read that
   creatures share the players' turn routine). Rest comes with a later
   rule set.
4. These run for players and creatures alike, on their own turns, so
   creatures in rooms no player visits stand still in time, as they do in
   everything else (creatures 2.3); the reversals of 5.2 go by rounds of
   the region being visited. [unknown] They emit no events; the front end
   reads the numbers.

## 6. Creatures and magic

1. Smart creatures (Wisdom above 14) cast spells and use magic items; dumb
   ones (below 6) never do; in between they act smart some of the time, more
   often the higher their Wisdom. [manual] **Engine:** a creature with
   Wisdom 6..14 counts as smart with chance (Wisdom - 5) x 10% (on the
   C64 a roll of 0 to 9 below Wisdom - 5, once a turn at its first
   decision, for fighters, friends and enemies; [research: random-rolls
   third pass]), rolled when
   it would otherwise cast. [unknown]
2. Only fighters cast; slinkers never attack or cast. [manual]
3. A smart creature casts only when its Power is at least its Wisdom, on the
   goal it has chosen if that goal is an individual, with its lowest-numbered
   appropriate spell. [manual] **Engine:** at the start of its turn, after
   the overburdened drop and readying, the creature chooses its goal as
   usual (creatures 6); if the goal is an individual anywhere in the room and
   the creature has a fitting spell, it casts it (section 4.1 chance and
   cost) and its turn ends. [unknown]
4. **Fitting.** Hurtful spells only on foes, helpful ones only on friends.
   [manual] **Engine:** a goal is a foe as in creatures 7.4, else a friend.
   - hurtful: Kill All, Banish (summon 0), Decrease Magic Defense, Decrease
     trait or skill, a negative Power or Life Force change;
   - helpful: Increase Magic Defense, Increase trait or skill, a positive
     Power or Life Force change, Give;
   - a summons counts as hurtful (help against a foe) [unknown];
   - never cast by creatures: Do Nothing, Rid Room, Add to Room, Activate
     All, Display Message, Play Music [manual], and invalid codes.
5. **Magic items.** Smart creatures at half power use magic items in the same
   way, if the item acts when used. [manual] "Half power" is not defined.
   [unknown] **Engine:** when it casts nothing, a smart fighter whose Power
   is at least half its Wisdom (rounded down) uses its lowest-numbered
   fitting magic item that acts when used, on its goal; its turn ends.
6. **Spells creatures set off.** A creature that takes a door with a spell
   (other than the forbidden Activate All, Rid Room and Add to Room,
   creatures 5.3) sets it off, acting on itself; a creature that drops a
   magic item with a drop spell (the overburdened drop, a friend's drop)
   sets that off under 2.3. [manual] Creatures never step on spaces or pick
   up magic items, so those never fire for them.
7. Corpus: 1,762 placed residents carry a magic spell, nearly all Life
   Force changes; 1,163 of them are smart and 1,617 start with Power at
   least Wisdom. [data] So in most adventures smart enemies will cast at
   the player as soon as they see them.

## 7. Death

Combat will reuse this; spells are the first thing that can kill.

1. An individual whose Life Force reaches 0 dies. [manual] **Engine:** checked
   whenever Life Force changes (spells, decay).
2. The dead drop everything they carried where they died, except on the
   world map, and except things set to disappear. Owner-only weapons and
   armor can't be dropped. [manual] **Engine:** possessions land on the
   death square, top of the stack, gold as one pile (inventory 2.5); treasures
   that vanish when dropped vanish; magic spells and owner-only weapons and
   armor are lost. Drop spells do not go off. [unknown] On the world map
   everything is lost. One `Died` event lists what was left.
3. A dead creature frees its slot. [data: creatures 1.1]
4. A dead player stays in the player list, marked dead, and gets no more
   turns. When no player is left alive the game emits `AllPlayersDead` and
   ignores further commands. After a death, play resumes from the last save
   (the region change before it). [manual] **Engine:** saves are the front
   end's snapshots at `EnteredRegion` / `LeftRegion` (movement 6.6).
5. A player who dies during their own turn ends it at once.

## 8. Characters

Players now need every trait. **Engine:** `PlayerSetup` adds Dexterity,
Wisdom, Power and Size (11 each by default, the top of the manual's normal
10..11), Dodge, Parry and Melee (30, the middle of 25..35), Missile (15, the
middle of 10..20), Armor skill (30, lost in the OCR [unknown]) and magic
defense (none). Trait generation is a later rule set.

## 9. Left for later

- **Combat:** done for melee in [combat.md](combat.md) (blows lowering
  Life Force and Power, magic defense in fights, creatures turning hostile
  when hurt, death in battle by section 7).
- **Rest:** healing 6 times faster than section 5.3.
- **Stores** (specced in [stores.md](stores.md)) and the world map
  encounter areas.

## Captures needed

1. Cast success, a failed cast's cost and a magic item's cost: captured
   (4.1, 4.2), and a use's cost in steps (two).
2. Power drift, temporary decay and healing: captured, researched and
   confirmed (5.1-5.3). Still open: creatures' healing, a temporary fall,
   an individual out of play when its reversal is due.
3. Kill All: room or everyone; is the caster spared.
4. Summons: when and where it arrives, and as what.
5. Add to Room: where the thing lands.
6. Activate All with conditional spaces, "when dropped here" spaces and a
   nested Activate All.
7. How often a mid-Wisdom creature acts smart; what "half power" is.
8. What casting costs in moves: captured (4.3: the rest of the turn).
