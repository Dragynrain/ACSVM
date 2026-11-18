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
   (custom spaces and custom obstacles take it from the placement when
   bit 7 of the first property byte says so, else from the master,
   movement 5.5 and 5.8):
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
   [manual] On the C64 a player's cast asks for a target, Kill All, a
   summons and Add to Room included (Activate All does not, 2.5): a cursor
   starts on the caster's square, the stick moves it a square at a time,
   and the button picks the square under it. [capture: rr36, the casts of
   capture.md 10.40; ours, no research pass, 2026-11-16]
   - **Reach**: the cursor goes to any inside square of the caster's room,
     however far (9 squares away in a 15 x 10 room), and stops at the
     inside edge: pushed past it, it stays on the last inside square, so
     it never reaches a wall square. [capture: rr38 `tc-e9`, `tc-e12`,
     `tc-s4`, `tc-s6`, `tc-n4`, `tc-n6`, `tc-w4`, `tc-w6`, capture.md 10.42
     item 1; ours, no research pass, 2026-11-16]
   - **Use Object** asks for its target the same way ("move cursor to
     target"), the cursor starting on the user: a magic item doing Add to
     Room put its thing on the user's square with no tap, two squares
     east with two. [capture: rr38 `ta-use`, `ta-use2`, capture.md 10.42
     item 1]

   **Engine:** the target is a square of the caster's room (on the world
   map only the caster's own), for a cast and a use alike; a spell that
   acts on an individual acts on the one standing there, the caster itself
   when it is the caster's square. A wall square, or a square with no
   individual for a spell that needs one, refuses the command at no cost
   (`CastRefused`); the C64's cursor never offers a wall square, so that
   refusal only guards commands a front end should not send. A creature's
   target is the square of the goal it casts at (6.3).
5. Spells that act on the room (Rid Room, Activate All, banishing) ignore
   the target. [manual] Three of the manual's room spells do not: on the
   C64 Kill All acts on the target alone, and a summons and Add to Room
   take effect on the target square (3.1, 3.2, 3.13). [capture: rr36,
   capture.md 10.40 items 1 to 3; ours, no research pass, 2026-11-16]
   Activate All cast by a player asks for no target at all: choosing the
   spell and pressing runs it at once on the **caster's own square**, the
   stack there run as in 3.14 even when the caster had arrived on it
   without setting it off (a space giving a thing and one changing Power
   by +5: the thing given, Power 13 to 17, the turn over); on an empty
   square nothing happens. [capture: rr38 `ta-sel`, `ta-actc`, `tb-c`,
   capture.md 10.42 item 1; ours, no research pass, 2026-11-16]
   **Engine:** so: a cast Activate All ignores the target and runs the
   caster's square. Whether Rid Room and banishing ask for a target is
   [unknown]: the front end's only, since they ignore it either way, and
   not yet run (capture.md 10.43).

## 3. The 15 actions

The parameter layout is the format draft's; numbers below are corpus counts.

1. **Kill all but an owner of** (thing). Everyone who does not own the thing
   dies. [manual] On the C64 the spell kills only the individual it acts
   on (the target of a cast, 2.4), and only when that individual does not
   own the thing; nobody else dies, in the room, another room or the
   region. Being the caster spares no one: a player not owning the thing
   who cast it at itself died, its Life Force 0; cast at a creature not
   owning it, the creature died and everyone else lived; cast at an owner,
   or by an owner at itself, nobody died. [capture: rr36 `ka-c`, `ka-bb`,
   `ka-aa`, `kb-c`, capture.md 10.40 item 1; ours, no research pass,
   2026-11-16] **Engine:** so: the individual the spell acts on dies
   (section 7) unless it owns the thing. The same on the world map and for a space, obstacle or door
   (2.1: the one who set it off). (Superseded: the engine's earlier choice
   of everyone in the room but owners.) 6 uses.
2. **Summon or banish** (creature number, 0 = all). [data: draft] 0 banishes
   every creature in the room at once, and banished creatures take what they
   carry with them. [manual] A creature that casts it banishes everyone but
   itself. [manual] **Engine:** players are never banished. A number summons
   that master creature to the room; summons are answered on the following
   turn, never on the world map, and not while the region holds 16 creatures.
   [manual]
   - When: on the C64 the creature is made **at the cast**, not later: by
     the time the target prompt shows (or at the first press) it already
     holds its slot, and the press puts it in the room in the same round.
     Its first turn comes in the next round; that is the manual's
     "following turn". There is no wait for a later turn and no reserved
     square. [capture: rr36 `sa`, `sb15`, capture.md 10.40 item 2; ours,
     no research pass, 2026-11-16] **Engine:** so: the creature is made
     and placed as the spell takes effect, and takes no turn in that
     round. (Superseded: the engine's earlier choice of an arrival at the
     next player turn.)
   - Where: a cast puts it **on the target square** (2.4), whatever is
     there: an empty square, a square a creature stands on (the two share
     it), or the caster's own square. [capture: rr36 `sa-p66`, `sb15-c`,
     `sb15-27`, `sa-p2`, `sa-p3`, capture.md 10.40 item 2] Baber's note
     that creatures step onto a space where a summons is due to arrive
     [designer] does not fit this: the C64 showed no such wait.
     **Engine:** the target square, shared with whoever stands there.
     With no target press the creature lands on **the square of the thing
     that set it off**, not the setter's: a space stepped onto, on the
     space (the stepper's square too); an obstacle bumped, on the
     obstacle's own square, the bumper staying beside it; a portal passed,
     on the portal's own square in the room left, not with the traveller
     at the destination; a magic item picked up, on the square where the
     item lay, the picker staying beside it (inventory 2.1); a magic item
     dropped, on the drop square (the dropper's own or one beside it).
     Each time a copy in the lowest free slot, made at once. [capture: rr38
     `sn-sp`, `sn-ob`, `sn-po`, `sn-pk2`, `sn-dro`, `sn-drs`, capture.md
     10.42 item 1; ours, no research pass, 2026-11-16] A creature passing
     that portal set it off too (6.6): a second copy appeared in the first
     room. [capture: rr38 `sn-dro`] **Engine:** so: the space's square, the
     obstacle's square, the portal's square in the room the traveller
     left, the item's square for a pick-up, the drop square for a drop. An
     obstacle or portal on the wall line was not tried with a summons
     [unknown]; **Engine:** as Add to Room's (3.13), the bumper's own
     square, which the C64 showed for Add to Room from a wall obstacle and
     from a wall portal.
     (Superseded: the engine's earlier choices of the bumper's square for
     an obstacle inside the room, the door's destination and the picker's
     square.)
   - A note on the prompt: once, a first press with the cursor on the
     caster's own square left the prompt up, and a second press there
     placed the creature; in another run the cursor moved after such a
     press and the creature went where it was pressed next. [capture:
     rr36 `sa-p2`, `sa-p3`] Not explained. **Engine:** one press, one
     square.
   - What arrives: a copy of the master creature's record, alignment,
     Speed, Constitution and all, as a resident in the **lowest free
     slot** (not a random creature, so it is never removed for being left
     behind). With slots 0 to 14 in use it took slot 15. [capture: rr36
     `sa`, `sb15`, capture.md 10.40 item 2] Baber reports that summoning a
     creature from outside the region's own list can corrupt names or
     crash the original; that is a bug the engine does not copy.
     [designer]
   - **A full region**: with all 16 slots in use the spell fails as a
     failed cast does ("a few sparks fizzle", 4.1), even at cost 0, with
     no target prompt and no slot changed. [capture: rr36 `sb16-c`,
     capture.md 10.40 item 2] At a cost above 0 the cast goes through 4.1
     first: the Power is spent and the roll drawn, and only then does the
     full region turn the cast into the same fizzle, a successful roll
     included. With all 16 slots in use and Power 31 against cost 20,
     three random states gave the fizzle and Power 11 each time; the same
     states with 15 in use gave two successes (the creature in slot 15)
     and one failed roll, and the random state moved alike in both
     regions. [capture: rr38 `sf-r0`, `sf-r85`, `sf-r170`, `sg-r0`,
     `sg-r85`, `sg-r170`, capture.md 10.42 item 1; ours, no research pass,
     2026-11-16] **Engine:** a cast summons in a full region spends and
     rolls as any cast (4.1; at cost 0 nothing is spent and nothing
     drawn), then fails whatever the roll (`SummonUnanswered`), with no
     target chosen and no slot changed, and ends the turn as any cast does
     (4.3). From a space, an obstacle, a door or an item, nothing happens
     [unknown: not tried full]. (Superseded: the engine's earlier choice
     of checking the slots first, spending nothing and drawing no roll at
     any cost.)
   - 30 named summons and 17 banish-alls among master things; 82 of 83
     custom placements name a creature. [data]
3. **Increase magic defense, but not above** (0..3: none, non-magic weapons
   do half damage, only magic weapons hurt, magic weapons do half damage).
   [manual] One spell raises it **one level**: a player at 0 cast one
   with parameter 3 at itself and went to 1; at 1 the same cast gave 2.
   [capture: rr39 `md-c`, `md-c1`, capture.md 10.43 item 3; ours, no
   research pass, 2026-11-16] **Engine:** one level, never past the
   parameter; nothing if already there (the cap not run).
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
   weapon given is not readied: a melee weapon given to an empty-handed
   player was owned and not held, and one given to a player holding a
   missile weapon left the missile weapon in hand. [capture: rr39 `gv-c`,
   `gv-cb`, capture.md 10.43 item 3; ours, no research pass, 2026-11-16]
   **Parameter 0 gives thing 80**: a Give with parameter 0 (from a custom
   obstacle's master byte, bumped or under Activate All) gave the player
   thing 80, in every case tried. [capture: rr41 `v2b-ee`, `v2b-ww`,
   `v2c`, `v3`, capture.md 10.45 item 1; ours, no research pass,
   2026-11-17] **Engine:** so: parameter 0 gives thing 80, any other
   parameter that thing. 94 uses.
10. **Display long message** (message 1..255). [manual] A long message is
    stored as 8 lines of 32 characters (format notes, Text). On the C64 it
    is shown as **8 screen lines, one per stored line, exactly as
    stored**: the program does not rewrap, so a word that runs across a
    line end in the data shows split across two screen lines, and two
    words either side of a line end show on separate lines with no space
    added. [capture: rr41 `lm`, screenshot `a00e`, capture.md 10.45 item
    4; ours, no research pass, 2026-11-17] **Engine:** a `LongMessage`
    event whose text keeps the 8 stored lines, separated by line breaks
    (trailing blanks of a line may be trimmed, nothing else changed); 0
    shows nothing. A front end may show the lines as stored or join
    them into paragraphs (the viewer does, client 3.1).
    (Superseded: the engine's earlier choice of joining the lines with
    nothing between them and squeezing the spaces.)
11. **Play music** (one of 30). Choices 1-12 are effects, 13-30 themes, and
    14-17 loop until other music plays. [manual] Placed parameters run 0..29,
    so the byte is the choice minus one. [data] **Engine:** a `Music` event
    with the 1-based choice; the front end plays it. 151 master uses and
    the most common custom placement.
12. **Rid room of every uncarried** (thing). Every copy lying in the room
    goes, in stacks too. [manual] On the world map: nothing. [unknown;
    not to be run: nothing lies on the map to rid, so play cannot tell,
    capture.md 10.43]
13. **Add to room one** (thing). Takes effect at once. [manual] On the C64
    one of the thing lands on top of the square where the spell takes
    effect: for a cast the target square (2.4), the caster's own or an
    empty one, on top of whatever lies there (a space under the caster
    included); for a space, on top of the space itself; for an obstacle
    bumped, on top of the obstacle's square, the bumper staying where it
    was. [capture: rr36 `aa-c`, `ab-c`, `aa-66`, `ac-s2`, `ad-s`,
    capture.md 10.40 item 3; ours, no research pass, 2026-11-16] So a
    space that adds a key puts the key on top of itself, and stepping off
    and on again picks it up without setting the space off again
    (movement 5.3). Set off by a thing on the wall line or with no target
    press: an obstacle on the wall line, bumped from the inside square
    beside it, put the thing on **the bumper's own square**, under the
    player standing there; a portal inside the room put it on **the
    portal's own square** in the room the player left, not at the
    destination; a magic item picked up put it on **the square where the
    item lay**, the picker staying beside it. [capture: rr38 `ar-wall`,
    `ar-po`, `ar-pk2`, capture.md 10.42 item 4; ours, no research pass,
    2026-11-16] **Engine:** so: a cast's target square, the space's or
    inside obstacle's own square, the bumper's own square for an obstacle
    on the wall line, the portal's own square in the room left, the
    item's square for a pick-up and the drop square for a drop. **A
    portal on the wall line** is as a wall obstacle: a player walking
    through one out to the side room left the thing on the inside square
    it stood on before the door, in the room it left. [capture: rr39
    `wp-po`, capture.md 10.43 item 6; ours, no research pass, 2026-11-16]
    **Engine:** so, the traveller's own square before the door. Nothing
    on the world map.
    (Superseded: the engine's earlier choices of the caster's square for
    every cast, a door's destination, and the nearest free inside square
    for a thing on the wall.)
14. **Activate all things at this place** runs, top to bottom, every spell in
    the stack below the thing that holds it, and only those. [manual]
    **Engine:**
    - a stack's spells are those of spaces that invoke a spell, custom spaces
      set to "when moved onto", obstacles that invoke a spell when bumped,
      custom obstacles (whether or not their spell bit is set, movement
      5.5 [capture: rr41 `v2c-e`, `v2c-s`, capture.md 10.45 item 1]), portals that invoke a spell, and magic items and magic spells
      lying there;
    - custom spaces open to owners (or non-owners) of a thing run only when
      the individual meets the condition, which is how "Let There be Light"
      works [manual]; "when dropped here" spaces do not run;
    - an "owners of" custom space set to destroy its needed thing takes the
      thing here as on a direct step (movement 5.7), so a second such space
      lower in the same stack, needing the same thing, then fails its
      condition if that was the last one. On the C64 a player owning the
      blade stepped onto an Activate All space over two such spaces (giving
      21 and 22) and a spell space (giving 23): 21 and 23 were given, the
      blade was gone and 22 was not given; one such space alone took the
      blade and gave its thing the same way. [capture: rr41 `n1-ew`,
      `n1-s`, capture.md 10.45 item 2; ours, no research pass, 2026-11-17;
      one run each] (Superseded: the engine's earlier choice of never
      using the needed thing up under Activate All.);
    - every thing in the stack with the disappearing bit whose spell runs
      goes as the stack runs (movement 5.9) [capture: rr41 `v3-ewe`,
      capture.md 10.45 item 1];
    - another Activate All lower down is skipped, so a stack runs once;
    - cast, used or picked up away from a stack, it runs the whole stack on
      the square where it takes effect: for a cast, the caster's own square
      (2.5) [capture: rr38 `tb-c`, `ta-sel`, capture.md 10.42 item 1]; for
      a use or a pick-up [unknown], the user's own square and the item's
      square (reachable, not yet run: capture.md 10.43).

    On the C64, a player stepping onto a stack of (top first) an Activate
    All space, a "when moved onto" custom space, one open to owners of a
    thing, a "when dropped here" space, a second Activate All and a plain
    spell space (each but the Activate Alls giving a thing) was given the
    "when moved onto", the owners' and the plain space's things when owning
    the thing, only the "when moved onto" and the plain space's when not,
    and never the "when dropped here" one's; the top space let it pass either way. With
    the last space a Power +5, Power rose by 5 once, not twice: the
    nested Activate All did not run the rest again. [capture: rr36 `acto`,
    `actn`, `actp`, capture.md 10.40 item 4; ours, no research pass,
    2026-11-16] The choices above on conditions, "when dropped here" and a nested
    Activate All agree. Cast away from a stack:
    above, and 2.5.
15. **Do nothing**: only the message. [manual] A spell with an action code
    outside 0..14 (one corpus key spell holds 15) does nothing too. [data]

Spells "may be made to disappear after first use" (magic spells and magic
items). [manual] On the C64 a magic spell set so goes only when a cast
works: at a 19% chance, of eight casts from eight random states one
worked (its Life Force +1 given) and the spell was gone; the seven that
failed spent the Power and kept it. [capture: rr39 `dsp-r0` to
`dsp-r224`, capture.md 10.43 item 4; ours, no research pass, 2026-11-16]
**Engine:** a magic spell goes after a successful cast, a magic item
after it is used (items always work, 4.2).

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
   comes just after the bar is refilled. [research: random-rolls second pass, 2026-11-12, `docs/research/random-rolls.md`] The moves themselves
   are set from the Life Force after the healing die for a player, before
   it for a creature (5.3.2, [inventory](inventory.md) 3.4). [capture:
   capture.md 10.39 item 4, 10.43 item 9] A creature drifts before it
   chooses what to do: a smart creature's Power after drift is what its
   casting and using test (6.3, 6.5). [capture: rr39 `cc-*`, capture.md
   10.43 item 7]
   **Engine:** the same, at the top of each turn: for a player before the
   die and the moves, for a creature (after its catch-up) before the
   moves and the die. A player joining an adventure keeps its
   saved Power and Life Force; drift and healing then move them as for
   anyone. [research; capture: rr13, [characters](characters.md) 3.1]
   (capture.md 8.4 read the profile after several rounds had passed, and
   took the drift for a rule of joining.) **Engine:** the same.
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
   [capture: drift and drift --walk] A player destroyed on Retire with a
   reversal waiting is never stepped back: its entry stays first in the
   queue, unchanged, and is never taken, though its round passes
   ([characters](characters.md) 7.5). [capture: rr38 `rv-a`, `rv-b`,
   capture.md 10.42 item 5; ours, no research pass, 2026-11-16] Not
   settled: whether such a left entry holds back reversals behind it, an
   individual out of play in other ways when its reversal falls due (the
   researcher read a 10-round delay), a temporary fall of Life Force (by the rule it ends 1 above
   where it started), other stats. **Engine:** the rule above: a queue of
   up to 30 reversals per game, each with its individual, its step (1
   toward undoing the change) and its count; at the end of each round
   every due reversal takes its step, then, with count left, counts down
   and is due again a die roll of rounds later. A dead individual's
   reversals are dropped. `temporary_life` reports what is still to be
   taken back (11 just after a temporary +10).
3. **Natural healing.** Life Force below Constitution heals on its own, about
   6 times slower than resting. [manual] On the C64 there are two chances,
   each healing 1 point only while Life Force (temporary part and all)
   is below Constitution, never above it; the rolls are drawn either way:
   1. each **idle move**, a move of the bar used up by waiting, takes a
      roll of 0 to (the turn's full moves - 1) and heals on a 0: 1 in the
      turn's full moves. The **turn's full moves** are the moves the bar
      was last refilled with at a turn's start: Speed, halved when the
      individual was hindered then ([creatures](creatures.md) 2.6), then
      the weight slowdown ([inventory](inventory.md) 3.3). The roll is drawn at full Life
      Force too, healing nothing. [capture: mem `--draws`, rr24
      `c1-button`, capture.md 10.28: 4 idle moves at Life Force 40 of 40,
      one draw each (one roll; `--draws` logs each roll as two stores,
      capture.md 10.29); ours, no research pass, 2026-11-14] A player's
      turn beside an enemy, its bar halved from 12 to 6, rolls 0 to 5:
      that range alone fit all 384 idle moves of 64 runs, and all 192 of
      32 runs on a turn that began with no moves carried. [capture: mem
      `--draws`, rr25, cases adj s8, adj s0, capture.md 10.29; ours, no
      research pass, 2026-11-14] Walking, using an item or casting takes no such
      roll. So a turn waited out in full heals 1 on average whatever the
      Speed (a Speed 1 turn always 1). With full moves 0 (a creature of
      Speed 0, whose one action may be an idle move, creatures 2.7) the
      roll is not drawn and counts as 0: 1 point every such turn while
      below Constitution. [research: combat fourth pass, 2026-11-14,
      `docs/research/combat.md`; capture: still-hurt, still-enemy,
      still-whole, 576 runs, capture.md 10.24] A turn that starts with
      moves carried through a door ([creatures](creatures.md) 2.2.4) is
      no refill, so its roll keeps the door turn's full moves, never the
      number carried: 12 for a Speed 12 player whose door turn was
      unhindered (it fit all 512 idle moves of 64 runs with 8 carried,
      where 8 fit 486), 6 when the door turn was itself beside an enemy
      (all 192 of 64 runs with 3 carried, where 3 fit 161). [capture: mem
      `--draws`, rr25, cases adj s8, adjside s3, capture.md 10.29; ours,
      no research pass, 2026-11-14] **Engine:** the full moves are set
      at each refilled turn start (halved when hindered) and left as they
      are on a turn that starts with carried moves;
   2. the **top of each turn**, just after drift, rolls a die and heals on
      a 2: 1 in 6, needed or not. A Speed 0 creature's die is the same
      one, and it is what gives such a creature its occasional second
      point on a turn (5.5). [capture: rr29, 768 runs, capture.md 10.33]
      A player's moves for the turn are set after this die, from the Life
      Force it leaves (the capacity and slowdown of
      [inventory](inventory.md) 3.2-3.4): a heal here can lengthen the very
      turn it starts. [capture: rr35 `c-s12c40`, `c-s12c41`, capture.md
      10.39 item 4; ours, no research pass, 2026-11-16] (5.1's "just after
      the bar is refilled" is the researcher's reading; the moves byte as
      captured comes from the Life Force after the die.) **A creature's
      moves come before this die**, from the Life Force it had: a heal
      here lengthens only its next turn. [capture: mem, rr39 `cw-m`,
      `cw-m2`, 24 healed turn starts of 112, capture.md 10.43 item 9; ours,
      no research pass, 2026-11-16] **Engine:** for a player, drift and
      this die, then the moves; for a creature, catch-up, drift, the moves,
      then this die (inventory 3.4).
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
   one healing roll each, and so does each `Idle` (the real-time drain,
   movement 2.4); a turn ended by walking its last move, casting
   or readying armor takes none. SAVE GAME ends no turn
   ([creatures](creatures.md) 2.2.7): the moves it leaves are walked or
   run out as idle moves with their rolls, as in the original (one draw
   per idle move, Life Force +1 at 4 of 32 such moves against 7 of 40
   without the save; [capture: rr24 `c4-save`, `c4-idle`, capture.md
   10.28]); creatures roll for the moves their turn
   leaves unused ([research, unconfirmed]: the researcher read that
   creatures share the players' turn routine). REST is 5.6.
4. These run for players and creatures alike, on their own turns, so
   creatures in rooms no player visits stand still in time, as they do in
   everything else (creatures 2.3), until their next turn, when they catch
   up (5.5); the reversals of 5.2 go by rounds of the region being
   visited. [unknown: reachable with a temporary change left running
   across a region change, not yet run, capture.md 10.43] They emit no events; the front end reads the
   numbers.
5. **Creatures catch up.** A creature's record keeps the round of its last
   turn, as the round counter's low byte (0 to 255; 0 means it has had no
   turn yet). At the top of each of its turns, when that byte is not zero,
   with N the rounds since it (the current round's low byte less the byte,
   counted round 256, so an absence of more than 255 rounds wraps), the
   creature gains up to N - 1 Power, never past its Wisdom, and up to N / 2
   Life Force, rounded down, never past its Constitution. When the byte
   equals the current round's low byte (N = 0, no round missed) the
   creature gains nothing: N - 1 does not wrap to 255. [capture: rr23,
   capture.md 10.27: the byte set to the round of the creature's turn, 20
   of 20 runs over 20 random states gained nothing; the controls as the
   rule gives, N = 255 Power and Life Force to their caps in 4 of 4, N = 1
   and byte 0 the same as N = 0] Then the byte is
   set to the current round's low byte (1 if that is 0). A creature with 0
   there, as every corpus and editor record has (format-notes, Creatures:
   byte 0x0C), skips the catch-up on its first turn and only notes the
   round; that is why a first entry into a room shows none. Drift and
   healing (5.1, 5.3) follow as usual; their order with the catch-up does
   not change the result. Players never catch up. [research: characters
   second pass, 2026-11-14, `docs/research/characters.md`; capture: rr19
   `catch-B`, capture.md 10.23: the byte set to 7 values (39, 38, 30, 24,
   10, 200 for the wrap, and 0) before a Speed 0 creature's first turn,
   its Life Force and Power afterwards as the rule gives in all 7, and the
   byte the current round after each; players never catching up:
   research, unconfirmed] The byte travels with the record, so a region
   left and visited again catches up the same way. [research; capture:
   ours, 2026-11-14, rr26 `ab-4`, `ab-10`, `ab-30`, capture.md 10.30: a
   Speed 0 creature (Life Force 8 of 20, Power 5 under Wisdom 20, the byte
   17) left while T went out to the map for 4, 10 and 30 rounds; at its
   first turn back, 6, 12 and 32 rounds after its last, Power as the rule
   gives in 3 of 3 and Life Force in 2 of 3 (the third one point more, the
   Speed 0 extra point below), the caps at 32, and the byte the current
   round after each turn] On the C64 the room's region is unloaded while
   no player is in it (the creature's slot in memory read all zeros while
   T was on the map) and loaded back from the disk with the byte kept, so
   the catch-up is what carries the creature's time across that reload.
   **The Speed 0 creature's extra point.** In these captures a Speed 0
   creature sometimes healed 1 point more on its turn than its idle move
   gives (5.3.1). That point is the top-of-turn die of 5.3.2, a 2 healing
   1, as for any individual; it is not an effect of the byte (capture.md
   10.23 had read it as one of a byte of 0). With the creature's byte set
   to 40, 39 and 0 and the random state set to each of its 256 values,
   the die drawn just before its turn predicted every extra point, 256 of
   256 runs for each byte. The byte seemed to change which states gave
   the point only because each roll mixes the raster line into the
   random state (as with casting, 4.1; capture.md 10.29):
   the catch-up's work shifts when the die is drawn by a raster line or
   more, and so its face. [capture: mem `--draws`, rr29 `catch-40`,
   `catch-39`, `catch-0`, 768 runs, capture.md 10.33; ours, no research
   pass, 2026-11-14]
   **Engine:** every creature keeps its last turn's round (the record's
   byte 0x0C, 0 as decoded) and catches up by this rule at the top of its
   turn, before drift, gaining nothing at N = 0. The engine keeps every
   region in memory rather than unloading it and loading it back from the
   disk: a different mechanism with the same outcome, as the record, its
   byte included, is the same either way, so leaving and coming back
   needs nothing more. The byte does not touch the turn's other rolls: the
   engine's rolls have no timing in them, and the extra point is 5.3.2's
   die, which it already rolls.
6. **REST.** The menu's REST (the first of its second row) is the way the
   manual gives to restore Life Force up to Constitution; without it the
   same healing takes 6 times as long. [manual] On the C64 REST plays out
   the moves left in the turn as idle moves, at once, each with 5.3.1's
   roll of 0 to (the turn's full moves - 1), healing 1 on a 0 below
   Constitution, and then the turn ends; no message, no move of its own.
   The rolls are drawn at full Life Force too. It is offered in rooms and
   on the world map, and a foe beside does not stop it (unlike FIRE,
   [combat](combat.md) 8.8.2). [capture: rr42, 56 runs, capture.md 10.46;
   ours, no research pass, 2026-11-18] So a rest heals no more than a
   turn left to drain: what is 6 times faster is the clock, the drain
   taking a move about every 7 seconds ([movement](movement.md) 2.4), and
   a walked move taking no roll at all. **Engine:** `Command::Wait` is
   REST: the moves left as idle moves with their rolls, then the turn's
   end (5.3). A front end names it REST.

## 6. Creatures and magic

1. Smart creatures (Wisdom above 14) cast spells and use magic items; dumb
   ones (below 6) never do; in between they act smart some of the time, more
   often the higher their Wisdom. [manual] **Engine:** a creature with
   Wisdom 6..14 counts as smart with chance (Wisdom - 5) x 10% (on the
   C64 a roll of 0 to 9 below Wisdom - 5, once a turn at its first
   decision, for fighters, friends and enemies; [research: random-rolls
   third pass]), rolled when
   it would otherwise cast. [unknown] When that roll is drawn against the
   turn's other first-decision draws needs a research pass on the draws'
   order inside a creature's turn (capture.md 10.43 (b)).
2. Only fighters cast; slinkers never attack or cast. [manual] On the C64
   a **peaceful friend** (a slinker by [creatures](creatures.md) 4.2) of
   Wisdom 20 cast: a Life Force +5 at the player on each of its turns,
   and a summons (6.4). [capture: rr39 `cc-fr`, `cc-fsum`, capture.md
   10.43 item 7; ours, no research pass, 2026-11-16] So the manual's rule
   does not hold for casting, at least for friends; slinker enemies,
   thieves and neutrals, and a slinker's magic item, were not tried
   [unknown]. **Engine:** a friend casts and uses magic items (6.3 to
   6.5) whether a fighter or a slinker; any other creature only as a
   fighter, as the manual says. (Superseded: the engine's earlier choice
   of fighters only.)
3. A smart creature casts only when its Power is at least its Wisdom, on the
   goal it has chosen if that goal is an individual, with its lowest-numbered
   appropriate spell. [manual] **Engine:** at the start of its turn, after
   the overburdened drop and readying, the creature chooses its goal as
   usual (creatures 6); if the goal is an individual anywhere in the room and
   the creature has a fitting spell, it casts it (section 4.1 chance and
   cost) and its turn ends. On the C64 an aggressive enemy of Wisdom 20
   and Speed 3, owning a Life Force -5 spell of cost 0, six squares from
   the player, cast it at the player on each of its turns without
   moving, its moves going from 3 to 0 at the cast: the turn over. The
   Power tested is the one after the turn's drift (5.1): Power 30 and 19
   (drifted to 20) cast; 10 and 15 (drifted to 11 and 16) did not, and
   the creature walked toward the player instead. [capture: mem, rr39
   `cc-s5`, `cc-p19`, `cc-p10`, `cc-p15`, capture.md 10.43 item 7; ours,
   no research pass, 2026-11-16] **Engine:** so, Power after drift at
   least the Wisdom. **Lowest-numbered** holds whatever the spells cost
   or do: an enemy of Wisdom 20 and Power 40 owning two fitting Life
   Force spells, in five pairs that set the cheaper against the dearer
   (cost 0 against 3) and the weaker against the stronger (-2 against
   -6), among things 1 to 32 and above, cast the lower-numbered one on
   every turn, 120 turns of 120 (two of them a failed cast of it, the
   Power spent and no harm). [capture: mem, rr41 `cs-p1` to `cs-p5`,
   capture.md 10.45 item 3; ours, no research pass, 2026-11-17]
   **Engine:** the lowest-numbered fitting spell: confirmed. Where the cast falls against the overburdened drop
   and readying was not seen (the caster carried neither) [unknown].
4. **Fitting.** Hurtful spells only on foes, helpful ones only on friends.
   [manual] A peaceful friend of Wisdom 20 owning only a Life Force +5
   spell cast it at the player on each of its turns (40, 45, 50, 55).
   [capture: rr39 `cc-fr`, capture.md 10.43 item 7] **Engine:** a goal is
   a foe as in creatures 7.4, else a friend.
   - hurtful: Kill All, Banish (summon 0), Decrease Magic Defense, Decrease
     trait or skill, a negative Power or Life Force change;
   - helpful: Increase Magic Defense, Increase trait or skill, a positive
     Power or Life Force change, Give;
   - **a summons (a creature number) fits any goal**, foe or friend: on
     the C64 the aggressive enemy owning only a summons cast it at the
     player on its turn, and so did the peaceful friend owning only that
     summons, at the player as its friend goal; each time the creature
     was made (3.2) and the caster did not move. [capture: rr39 `cc-sum`,
     `cc-fsum`, capture.md 10.43 item 7; ours, no research pass,
     2026-11-16] Other kinds of caster (a neutral, a thief) were not
     tried; **Engine:** a summons fits whatever individual the goal is.
     (Superseded: the engine's earlier choice of a summons as hurtful,
     cast only at a foe.) Where a creature's summons lands is not
     settled: the enemy's put the new creature on a square beside the
     player, not on the player's own square (one run; the friend's square
     was not read) [unknown]. **Engine:** the goal's square (2.4), shared
     with the goal, as for a player's cast.
   - never cast by creatures: Do Nothing, Rid Room, Add to Room, Activate
     All, Display Message, Play Music [manual], and invalid codes.
5. **Magic items.** Smart creatures at half power use magic items in the same
   way, if the item acts when used. [manual] On the C64 "half power" is
   **Power above half the Wisdom**: at Wisdom 20, a creature with Power
   11 used its item and one with Power 10 never did, walking instead
   (Power after drift, 5.1, swinging between 10 and 11 from turn to turn
   in two runs, a use at every 11 and none at any 10). A use works as a
   player's (4.2): it costs the creature 2 Power and **two moves, and
   does not end its turn**: with 3 moves it had 1 left after the use and
   took a step with it, at Power 28 as at 11. The item, a Life Force -3,
   acted on the player, the creature's foe goal. [capture: mem, rr39
   `cc-i30`, `cc-i10`, `cc-i9`, capture.md 10.43 item 7, 4 runs at the
   boundary; ours, no research pass, 2026-11-16] Only an even Wisdom was
   run: "above half rounded down" (Wisdom 15: Power 8 or more) and
   "above half rounded up" (15: 9 or more) both fit it and part only at
   an odd Wisdom, untested [unknown].
   **Engine:** when it casts nothing, a smart creature that may cast
   (6.2) whose Power after drift is more than half its Wisdom (2 x Power > Wisdom; for an odd
   Wisdom W, Power of (W + 1) / 2 or more) uses its lowest-numbered
   fitting magic item that acts when used, on its goal, at the point
   where 6.3 would cast; the use spends 2 Power (not below 0) and two moves
   (all it has if fewer), and with moves left after that the turn goes on
   as usual (its steps); with none left the turn ends, as for a player's
   use (4.3). At most one use a
   turn: a creature with two or more moves left after a use was not seen
   [unknown]. (Superseded: the engine's earlier choice of Power at least
   half the Wisdom rounded down, the turn ending at the use.)
6. **Spells creatures set off.** A creature that takes a door with a spell
   (other than the forbidden Activate All, Rid Room and Add to Room,
   creatures 5.3) sets it off, acting on itself; a creature that drops a
   magic item with a drop spell (the overburdened drop, a friend's drop)
   sets that off under 2.3. [manual] On the C64 a summoned creature that
   walked through a portal summoning it brought a second copy into the
   room. [capture: rr38 `sn-dro`, capture.md 10.42 item 1] Creatures never
   step on spaces or pick up magic items, so those never fire for them.
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
   armor can't be dropped. [manual] **The order on the square**: the gold
   goes down first, as one pile (inventory 2.5), then the possessions one
   by one from the lowest thing number up, each on top of what is there;
   so the highest-numbered possession ends on top and the gold lies under
   all of them, above whatever the square held before. A creature holding
   our blade (thing 59) and owning treasures 50 and 51, with 50 gold,
   killed on an empty room square, left (top first) the blade, treasure
   51, treasure 50, the gold; a copy in an encounter area holding the
   blade with 50 gold left the blade, the gold, then the door already
   there ([encounters](encounters.md) 4.4). Both times the player's push
   onto the square met the blade first ("cannot carry more blades", the
   player owning one). [capture: rr33 `dk` and rr32's `kill/k2`, capture.md
   10.37 item 2; ours, no research pass, 2026-11-15] A magic item goes
   down by its thing number like the rest, a magic spell is lost and the
   gold lies under all: a creature with the blade, treasures 50 and 51, a
   magic item 70, a spell and 50 gold left (top first) 70, the blade, 51,
   50, the gold; a player killed by its own cast, with the blade,
   treasures 13 and 14, item 70, a spell and 50 gold, left 70, the blade,
   14, 13, the gold. [capture: rr38 `dd-aa`, `dd-t`, capture.md 10.42 item
   2; ours, no research pass, 2026-11-16] **Which weapons are lost**: the
   mark that decides is "only the owner may use it", not the weight. A
   friend holding our blade died to Kill All: a blade of weight 0 that
   anyone may use went down on the square; one of weight 5 that only its
   owner may use was lost, and so was one with both marks. [capture: rr39
   `bl0-aa`, `bln-aa`, `blb-aa`, capture.md 10.43 item 10; ours, no
   research pass, 2026-11-16] **Drop spells do not go off** at a death: a
   magic item set to act when dropped (Add to Room) went down with the
   rest and added nothing, at a creature's death and at a player's own.
   [capture: rr39 `bl0-aa`, `bln-aa`, `blb-aa`, `bl0-t`, capture.md 10.43
   item 10] **Engine:** so, on the death square; treasures that vanish
   when dropped vanish; magic spells and weapons and armor only their
   owner may use are lost, whatever they weigh; a drop spell does not go
   off. (Superseded: the engine's earlier choices of the gold on top, and
   of losing a weapon only when it both weighed 0 and was owner-only.)
   Owner-only armor was not run; the engine treats it as the weapon. On
   the world map everything is lost. One `Died` event lists what was
   left.
3. A dead creature frees its slot. [data: creatures 1.1]
4. A dead player stays in the player list, marked dead, and gets no more
   turns. After a death, play resumes from the last save (the region change
   before it). [manual] On the C64 the offer comes after every round in
   which a player died, and play may also go on from there; with nobody
   left alive the game goes to the add-a-player screen rather than ending
   ([creatures](creatures.md) 2.2.5, 2.2.6). [research, capture] The
   save restored is the latest of the start of the last visit that began
   in a different region from the visit before it, the last SAVE GAME,
   the last Retire with Save and the last add of a player from a roster
   (each written at the end of the round it was asked in; an add of a
   created player saves nothing, creatures 2.2.7.4); the round counter comes back as that visit's
   first round (or the round after the save), not the round of the door
   step (creatures 2.2.7). [capture: rr21, rr24, capture.md 10.25, 10.28]
   The button at a death message restores the save from before that
   round, even when a SAVE GAME was asked in it; answered with F1, that
   SAVE GAME is written, with nobody alive if nobody is, and a newcomer's
   first visit then saves over it (creatures 2.2.7). By the add-a-player screen the player records
   are cleared; declining to add a player shows the same screen again
   and no turn is taken (creatures 2.2.6). [capture: rr21 `decline`] The
   same screen comes back when the last player retires
   ([characters](characters.md) 7.6). When the last player died on the
   world map and a created newcomer starts there, no region change
   follows and the all-dead save comes back at the newcomer's death:
   nobody alive, the add-a-player screen again (creatures 2.2.7).
   [capture: mem, rr34 `i3e-button`, capture.md 10.38 item 3; ours, no
   research pass, 2026-11-15]
   **Engine:** when no player is
   left alive, or the last one has retired, the game emits
   `AllPlayersDead` ("nobody is left in play") and ignores every command
   but adding a player; declining is the front end showing its screen
   again. The save point is creatures 2.2.7's, kept in the save file
   (`docs/saves.md` 1.4); the button resumes from it, and a save with
   nobody alive brings `AllPlayersDead` back.
5. A player who dies during their own turn ends it at once.

## 8. Characters

Players have every trait. A new character is rolled, a saved one joins
with its record: [characters](characters.md) 1 to 3. **Engine:**
`PlayerSetup` carries every rating; its defaults (traits 11, Dodge, Parry,
Melee and Armor skill 30, Missile 15) are only for tests and tools.

## 9. Left for later

- **Combat:** done for melee in [combat.md](combat.md) (blows lowering
  Life Force and Power, magic defense in fights, creatures turning hostile
  when hurt, death in battle by section 7).
- **Stores** (specced in [stores.md](stores.md)) and the world map
  encounter areas ([encounters](encounters.md)): what a summons does
  there is not known. **Engine:** as in any room of a region.

## Captures needed

1. Cast success, a failed cast's cost and a magic item's cost: captured
   (4.1, 4.2), and a use's cost in steps (two).
2. Power drift, temporary decay and healing: captured, researched and
   confirmed (5.1-5.3); creatures catching up by the record's byte, also
   across a real absence from the region (5.5, capture.md 10.30); a Speed
   0 creature's extra point of healing is the top-of-turn die (5.3.2,
   5.5, capture.md 10.33); a destroyed player's reversal never comes due
   (5.2, capture.md 10.42). Still open: creatures' healing, a temporary
   fall, whether a left entry holds back later reversals, an individual
   otherwise out of play when its reversal is due. The idle roll's
   range on a turn that starts with carried moves: captured (5.3.1,
   capture.md 10.29).
3. Kill All: captured (3.1: the target only, unless it owns the thing;
   the caster not spared; capture.md 10.40).
4. Summons: captured for a cast (3.2: made at the cast in the lowest free
   slot, on the target square; a full region fizzles, after the cost is
   spent and the roll drawn) and with no target press (the setting-off
   thing's square: space, obstacle, portal in the room left, item's
   square, drop square; capture.md 10.42). Still open: the first press on
   the caster's square that left the prompt up; a summons from a thing on
   the wall line; a summons from a space or item with the region full.
5. Add to Room: captured (3.13: on top of the square where it takes
   effect, a cast's target square; a wall obstacle the bumper's square, a
   portal its own square, a pick-up the item's square, capture.md 10.42).
   A portal on the wall line: the traveller's square before the door
   (capture.md 10.43).
6. Activate All with conditional spaces, "when dropped here" spaces and a
   nested Activate All: captured (3.14); cast away from a stack (no
   target, the caster's square), the cursor's reach and a magic item's
   use (2.4, 2.5, capture.md 10.42); a needed thing used up under it and
   disappearing things going with it (3.14, capture.md 10.45). Still open, reachable but not run
   (capture.md 10.43): Activate All from a used or picked-up item away
   from a stack; whether Rid Room and banishing ask for a target; the
   reversals going by the visited region's rounds (5.4).
7. Creatures casting: captured (6.3: Power after drift at least the
   Wisdom, anywhere in the room, the turn ending; 6.4: a summons at a foe
   and at a friend; 6.5: "half power" is Power above half the Wisdom, a
   use costing two moves, the turn going on; 6.2: a peaceful friend
   casts; capture.md 10.43); the lowest-numbered spell whatever its cost
   or strength (6.3, capture.md 10.45). Still open: an odd Wisdom at the "half
   power" boundary, a creature's summons' square, slinkers other than
   friends casting, a use with two or more moves left. When the 0 to 9
   smartness roll is drawn (6.1) needs a research pass (capture.md 10.43
   (b)).
8. What casting costs in moves: captured (4.3: the rest of the turn).
9. Increase Magic Defense (3.3: one level), Give (3.9: not readied),
   "disappears after first use" (3: a working cast only), Add to Room
   from a wall portal (3.13) and a death's drops (7.2: owner-only lost,
   drop spells not set off): captured (capture.md 10.43). A creature's
   moves set before its top-of-turn die (5.3.2): captured (capture.md
   10.43). Rid Room on the world map (3.12) is not worth a run.
10. A long message's screen lines (3.10: as stored, no rewrap) and Give
    with parameter 0 (3.9: thing 80): captured (capture.md 10.45).
