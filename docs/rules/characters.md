# Rules: characters

Behaviour spec for a character's life in an adventure from start to end:
how a new character is rolled, what it starts with, joining with a saved
character (from this adventure or another), the profile's up and down
marks, names, how many may play, and retiring. Licensed CC BY 4.0.
The engine implements it in `crates/acs-engine/src/characters.rs` (rolling
and the marks) and `game.rs` (joining). Skill growth is in
[combat](combat.md) 5.5 and 8.6; Power drift, healing and creatures
catching up in [magic](magic.md) 5.

Tags as in [movement](movement.md): **[manual]** paraphrased from the
manual, **[unknown]** with the engine's choice as **Engine:**,
**[research]** for the clean-room researcher's characters passes
(`docs/research/characters.md`: the first, 2026-11-14, for sections 1 to 5
unless marked; the second, also 2026-11-14, marked "pass 2") and
**[capture]** where our own runs confirm it; "[research, unconfirmed]"
means no capture of ours has checked it yet. The confirming runs are
`ACSVM-research/capture/runs/rr13/` ([capture.md](../capture.md) 10.18)
for the first pass and `rr19/` (capture.md 10.23) for the second.

Words used: a **round** is one pass through a region visit's turn list
([creatures](creatures.md) 2.2.3). The **roster** is the list of retired
players an adventure disk keeps (7.3; its layout in format-notes,
Creatures). A
player **slot** is one of the 4 player records in play; its first byte
holds Constitution with magic defense in the top two bits.

## 1. Rolling a new character

1. "Create New Player" rolls the character at once, before the joystick
   question, the picture and the name; none of those, the adventure or the
   number of players in play changes the rolls. [research]
2. **Five traits, three dice each**, a die being 1 to 6, drawn in this
   order: **Strength, Constitution, Dexterity, Wisdom, Size** (each 3 to
   18). [research]
3. **A second chance**: then a roll of 0 to 2; on a 0 (1 chance in 3)
   Strength, Constitution, Dexterity and Wisdom, in that order, each roll
   three new dice and keep the higher of the old and new totals. Size never
   gets a second chance. [research]
4. **Everything else follows** with no more rolls, dividing down:
   - Speed = 5 + Dexterity / 7 (5, 6 or 7);
   - Dodge = 3 x Dexterity - Size, **kept in a byte**: when Size is larger
     than three times Dexterity it wraps to 256 less the difference (a
     Dexterity of 3 against Size 11 gives 254);
   - Parry = Dexterity + Wisdom;
   - Melee = Size + Strength + Dexterity;
   - Missile = Dexterity + Strength;
   - Armor skill = 0. [research; the wrap: capture]

   The manual's starting ranges (Speed 6 to 7, traits 10 to 11, Melee,
   Dodge and Parry 25 to 35, Missile 10 to 20) are averages at best: the
   C64 averages Melee about 31, Missile, Dodge and Parry about 21, and
   rolls Speed 5 for a Dexterity under 7. [manual; research]

   **Confirmed:** the random state set to each of its 256 values on the
   start-up add-player screen and "create new player" chosen: every draw
   was the generator's, and the new record held exactly what the draws
   predict by 1 to 4 in **all 256 runs** (99 with the second chance, 72
   with a die drawn again), Dodge 254 included. move-basic's T (Speed 6,
   Dodge 21, Parry 14) fits too. [capture: rr13 `create`, capture.md
   10.18]
5. **Engine:** `PlayerSetup::roll` draws the same 16 (or 28) rolls in the
   same order from the game's one generator: dice of 1 to 6, then the
   second-chance roll of 0 to 2, then the second dice. `CreatePlayer`
   (and a game started with `Newcomer::Create`) rolls this way; the
   generator is ours, so the values differ from the C64's for a seed, the
   distribution does not.

## 2. What a new character starts with

1. **Life Force** = Constitution; **Power** = Wisdom; **magic defense**
   level 1 (non-magical damage halved) when Wisdom is 16 or more, after
   any second chance, otherwise none. [research; capture: rr13 `create`,
   all 256]
2. **Gold** 0, **no possessions**, **no weapon held, no armor worn**.
   [research; capture: rr13 `create`, all 256]
3. It starts on the world map's start cell ([movement](movement.md) 1.2)
   with the first picture unless another is chosen. [manual; research]
   **Engine:** the same; pictures are the viewer's.

## 3. Joining with a saved character

1. **A retired character joining keeps every saved rating**, Power and
   Life Force included; nothing sets Power to Wisdom or Life Force to
   Constitution on joining. [research] **Confirmed:** our retired players
   read in RAM after joining: Power 20 under Wisdom 13, 10 under 20, 31
   under 31 and Life Force 5 under Constitution 20 all held their saved
   values; then, as rounds passed before our first look at the map (the
   turn's moves draining), Power drifted 1 a round toward Wisdom
   ([magic](magic.md) 5.1) and Life Force healed (magic 5.3). That drift
   is what capture.md 8.4 took for "Power starts at Wisdom, at most 27".
   [capture: rr13 `join`, `jointime`, capture.md 10.18]
   A player from this adventure's own roster keeps everything else too:
   possessions, weapon held, armor worn, gold and its picture (it is not
   asked for one). Choosing it clears its name from the roster.
   [research; the picture: research pass 2, unconfirmed; the roster name:
   capture, format-notes, Creatures]
   **Engine:** `AddPlayer` keeps the setup's Power (its Wisdom when the
   setup gives none) and Life Force as given.
2. **A player from another adventure** is reached from the add-player
   screen's "add player from another adventure". The steps, each waiting
   for the button:
   1. a note that the back side of the ACS disk may serve if there is no
      other adventure disk;
   2. a prompt to put the disk to copy from in the drive;
   3. the program reads that disk's roster and lists its players beside
      the menu, with the cursor back on "add player from another
      adventure" (pressing there again only repeats step 1); the player
      is chosen by moving the cursor to it;
   4. the joystick question and the picture, as for a new character;
   5. a prompt to put the adventure's own disk back, then the game is
      saved.
   [research, pass 2; capture: rr19 `ota`, `otb`, capture.md 10.23; step 2
   was seen in capture only, the findings left it out]
3. **What it keeps and what is reset.** Kept as recorded: every trait and
   skill, Life Force, Power and magic defense. Reset: no possessions, no
   weapon held, no armor worn, and the square set to not placed, so it
   starts on the world map's start cell (12,12 in ours). **Gold is raised
   to at least 1,000**; more is kept (1,500 stayed 1,500, 300 became
   1,000). This replaces the first pass's "cut to at most 1,000", which
   had the comparison the wrong way round. [manual, in part; research,
   pass 2; capture: rr19 `otb-1500`, `otb-300`, 3 runs]
4. **The other disk** loses only the player's roster name (2 bytes, the
   same as when a player of this adventure is chosen); its record bytes
   stay. [research, pass 2; capture: the other disk differed from its copy
   in exactly those 2 bytes in all 3 runs] The back side of the ACS disk
   is never changed. [research, unconfirmed]
5. **Telling the disks apart** is [unknown]. The researcher read that the
   disk must carry the adventure-disk mark and an identity word different
   from the current adventure's, and that a never-played scenario disk
   keeps its template's identity until first played. Capture saw the
   identities (our never-played disks alike, our played ones another
   value) and saw the adventure's own disk, given as the "other", list its
   player in another column with the cursor put on "create new player";
   but setting the adventure's identity in memory to the other disk's
   changed nothing (that disk still listed and joined), so the address
   read is not what decides. What the program says when it refuses a disk
   is not known either. [research, pass 2; capture: rr19 `ota-ownid`,
   `ota-same`, `otb-same`]
   **Engine:** a front end that offers players from another adventure
   applies 3.3 when it builds the setup; the engine has no rosters or
   disks yet, so 3.2, 3.4 and 3.5 are the front end's when they come.

## 4. The profile's marks

1. Each rating has a fixed low and high bound: below the low bound it
   shows the **down mark**, at or above the high bound the **up mark**,
   otherwise none. Nothing else counts: not the other ratings, the kind of
   individual or the adventure. [research]

   | Rating | Down below | Up at or above |
   | --- | --- | --- |
   | Constitution, Life Force, Wisdom, Power, Strength, Size, Dexterity | 6 | 15 |
   | Armor skill | never | 25 |
   | Missile | 10 | 35 |
   | Dodge | 10 | 60 |
   | Melee | 15 | 45 |
   | Parry | 10 | 40 |
   | Gold | never | 250 |
   | Speed | 6 | 8 |

   [research; capture: two profiles, 28 marks, all as the table says: our
   retired player's 7 up marks (traits 21 to 27) and 7 unmarked skills
   (`stats-profile.png`), and move-basic's T with the **down mark** on
   Wisdom and Power 4, up marks on Constitution and Life Force 17 and the
   other 10 unmarked (rr13 `prof`)]
2. Creatures' ratings follow the same bounds wherever they are shown.
   [research, unconfirmed] **Engine:** `characters::mark`; the viewer shows
   the marks beside the current player's ratings.

## 5. Names

1. A name is up to 15 characters. [manual] All 15 are kept, in play and
   when retired: the record packs three characters into each pair of its
   10 bytes. [research; capture: rr19 `name-*`, 15 kept in play (5.4);
   retired, unconfirmed for 15]
2. The packing has 40 symbols: space 0, A to Z 1 to 26, the digits 1 to 9
   27 to 35, then period 36, comma 37, hyphen 38 and apostrophe 39. Each
   pair of bytes, low byte first, holds 3 symbols as first x 1,600 +
   second x 40 + third; a short name ends in spaces (zero bytes).
   [research; capture: rr13 `names`, the name bytes of 4 typed names
   decoded to the letters, digits and marks shown on screen]
3. **No key is refused**: every key but the editing keys (5.5) puts a
   symbol at the cursor and moves it on. Letters of either case are
   letters; the digits 1 to 9 are digits; the rest fold:

   | Key | Kept as |
   | --- | --- |
   | 0 | the letter O |
   | ! ? . | period |
   | + - / : ; = | hyphen |
   | , | comma |
   | ' and the double quote | apostrophe |
   | * & pound (£) up-arrow (↑) left-arrow (←) | X, A, L, I, K |
   | @ # $ % ( ) < > [ ] and space | space |

   [research, pass 2; capture: rr13 `names`, and rr19 `name-*`, 72 keys
   in 5 runs, every one taken and stored as the table says] The first
   pass's open question (? : @ and the double quote leaving nothing) was
   keys lost in feeding, not refusals: the program empties the keyboard
   buffer whenever its own scan finds no key held, so a key put in the
   buffer by the harness survives only if entry takes it first. [research,
   pass 2; capture: a control of 9 keys fed once lost 4, the same keys fed
   until taken lost none (capture.md 10.23, `typemem`)]
4. **Past 15 the cursor wraps**: the 16th character replaces the 1st, the
   17th the 2nd, and so on. A to Q typed gives PQCDEFGHIJKLMNO.
   [research, pass 2; capture: rr19, two names of 17 and 21 keys, as the
   rule gives] The garbled name strip and missing name first seen after
   17 letters (capture.md 10.18) did not come back with every key taken;
   it is not a rule.
5. **Editing keys**: cursor right and left move the cursor, DEL moves back
   and blanks that character, RETURN puts the cursor back on the first
   position; none of them ends entry. Only the joystick button ends it.
   [research, pass 2, unconfirmed]
6. **A name whose first three characters are spaces is asked for again**
   when the button is pressed. [research, pass 2; capture: rr19, 8 spaces
   then ABCDEFG, asked again]
7. **Engine:** a name given to the engine is taken character by character
   as typed keys: folded by 5.3 (any character with no C64 key becomes a
   space), wrapped past 15 by 5.4, the editing keys of 5.5 not taken (a
   front end does its own editing). The three keys 5.3 names by their C64
   key caps are the text characters £ (U+00A3, kept as L), ↑ (U+2191, I)
   and ← (U+2190, K). ASCII ^ and _ (the codes those keys share in
   PETSCII) have no key of their own on the C64 and become spaces, like
   any other character with no key.
8. **When a name is refused.** On the C64 the name is typed on the
   add-player screen, so a name of three leading spaces (5.6) is asked for
   again there, at the round's end, after the slot check of 6.2. [research,
   pass 2] **Engine:** the engine has no name screen, so it checks the
   name when the add request is given: a name whose first three
   characters are spaces after folding and wrapping is refused at once
   (no draw, and no request waits) and the front end asks again; the
   free-slot check stays at the round's end (6.4). The first player's name,
   given to `Game::new` before any round, is not checked by the engine:
   the front end asks again for a refused one before it starts the game.

## 6. Adding someone, and how many

1. **A slot is living while its first byte is not zero**, and free when
   it is zero: a dead character's slot, a retired one's (7.3, 7.4) and one
   never used. Nothing else counts, not Life Force, Speed or the square.
   So the dead don't count toward the 4. A newcomer's slot is cleared
   before anything is put in it. [research, pass 2; capture: rr19
   `four-first`, only the three first bytes poked, counted as living]
   **A newcomer takes the lowest-numbered free slot**, mid-game as at the
   start: a dead or retired player's slot is reused, so there are never
   more than 4 slots, and the newcomer's player number (and so its place
   in each round's turn list, [creatures](creatures.md) 2.2.3 and 2.2.6) is
   that slot's. With slots 1 and 3 living and 2 dead, the newcomer is
   player 2 and moves between them. [research: first pass, traced, and
   turn-order first pass (creatures 2.2.6); capture only for an empty
   game: rr13 `create`, the record in slot 1. A newcomer behind a living
   player's slot is unconfirmed]
2. **"Add Someone" always answers "I will add a player as soon as
   possible."** The request is handled at the end of the round, after any
   retirement (7.1): with a free slot the add-player screen comes; with
   none the program says the player must retire one of the adventurers
   before adding another, and adds no one. No character is rolled before
   that screen. [research, pass 2; capture: rr13 `ad-full`, and rr19
   `four-full`, `four-first`: the refusal at the round's end, 2 runs]
   **Several add requests in one round** are [unknown]: neither pass nor
   any capture asked "Add Someone" twice before a round's end, and no
   message like retiring's "another player will retire first" (7.1) was
   reported for adding. **Engine:** every request waits; at the round's end
   they are handled one after another in the order asked, each taking the
   lowest free slot then (6.1) or refused when none is left.
3. **A change in the living count either way** brings the round-end
   message of [creatures](creatures.md) 2.2.5 and 2.2.6 (restore the last
   save and bring back the dead, or F1 to go on): it compares the living
   players at the round's start and end, so slots made living in mid-round
   bring it too. That is what the first pass's poked players showed: they
   had not died. After F1 the four took turns 1, 2, 3, 4 every round.
   [research, pass 2; capture: rr19 `four-full-on`, rounds 6 to 12, 40
   samples; `four-rest`, every poke but the first bytes, gave no message]
   For the harness: poke the first bytes (and sensible Speed, Life Force
   and square), then answer F1 at that round's end (capture.md 10.23).
4. **Engine:** the request is taken at once (its name checked then, 5.8)
   and handled at the end of the round, after retirements (7.1), several
   in the order asked (6.2); with no free slot it is refused then
   (`AddRefused`), with no draw; otherwise the newcomer is rolled or set
   up in the lowest free slot (6.1), reusing a dead or retired player's
   entry rather than adding one, so the player list holds at most 4, and
   joins the next round (movement 1.4). A slot is living by the engine's
   own alive flag, which stands for the first byte.

## 7. Retiring

1. **When.** Choosing RETIRE only records the request ("I will retire you
   as soon as possible"). It is acted on at the **end of the round it was
   asked in**, after everyone still on that round's list has moved, on the
   map or in a room alike, not at the region visit's end. At a round's end
   the requests go in the order retire, add, save ([creatures](creatures.md)
   2.2.5), so a slot retired and an "add" in the same round frees the slot
   and then fills it. [research, pass 2; capture: rr19 `ret1` (map, the
   offer after U's turn), `room1` (a room with both players), `last1` (a
   room, T alone with a creature), 3 runs] Only one retirement can wait:
   a second asker is told another player will retire first, and nothing
   is recorded. A player dead by the round's end is offered nothing.
   Restoring the saved game or starting play drops a waiting request.
   [research, pass 2, unconfirmed]
2. **The offer**: the button saves the character for later adventures; F1
   warns that it will never be seen again and asks once more, where the
   button still saves and a second F1 destroys. [research, pass 2;
   capture: rr19 `ret-dest`, `last-dest`, `room-dest2`]
3. **Save.** The program takes the first roster entry with an empty name
   (of 34); with none, it says the disk is full and the character plays on
   [research, pass 2, unconfirmed for a full disk]. Life Force below
   Constitution is raised to Constitution; nothing else changes, Power
   included. The name and the whole record go into the entry: ratings,
   magic defense, gold, weapon held, armor worn, every possession, the
   picture, the joystick and the place (a room's region, room and square
   too). Nothing is left on the square. In play only the slot's first byte
   is zeroed (7.6), and the game is saved at once. Reversals due later for
   the character (magic 5.2) are dropped [research, pass 2, unconfirmed].
   [research, pass 2; capture: rr19 `ret-save`, `ret-save2`, `last-save`,
   3 runs: Life Force 3 and 4 under Constitution 11 saved as 11, Power 5
   under Wisdom 13 kept, gold 777, the blade held, the mail worn and four
   possessions in the roster's first entry, a room square kept]
   **Engine:** the same when rosters come; until then the front end keeps
   the saved character.
4. **Destroy on the world map**: nothing is put down; possessions and gold
   are lost. Only the slot's first byte changes, nothing is written to the
   roster and the game is not saved then. [research, pass 2; capture:
   rr19 `ret-dest`, memory and the disk otherwise unchanged]
   **Engine:** the same.
5. **Destroy in a room** is [unknown]. The findings said gold, blade and
   bow are put down and the mail and treasures lost; capture matched
   neither case. With the other player in the room, T's **whole kit** lay
   on its square (blade, bow, mail, both treasures and the 300 gold, in
   that order as U picked them up) in 5 of 5 runs, whatever the random
   state. With T the last player, a new player walked in later and found
   only the bow, the mail and the 300 gold (1 run); whether the newcomer's
   save and the room's reload lost the rest is not known. [capture: rr19
   `room-dest2`, `rd-0`, `rd-64`, `rd-128`, `rd-none`, `newc`, capture.md
   10.23] **Engine:** the 5-of-5 case: every possession, the held weapon
   and the worn armor land on the character's square, top of the stack,
   with the gold as one pile (inventory 2.5), nothing lost; then as 7.4.
6. **Afterwards.** The character is out of play: only the first byte of
   its slot is set to zero, so the slot is free for a newcomer at once
   (6.1); the rest stays until a newcomer clears it. The others play on
   from the next round without it. [research, pass 2; capture: rr19, U on
   every turn after, rounds 23 to 47] When the last player retires, saved
   or destroyed, the add-player screen of a game's start comes back, and
   keeps coming back while nobody is in play; a saved character is listed
   there, a destroyed one is not. [research, pass 2; capture: rr19
   `last-save`, `last-dest`] **Engine:** as when all players are dead
   ([magic](magic.md) 7.4): the engine reports nobody in play and waits
   for a new player.
7. **What the save writes.** The adventure block the save writes is an
   image of the memory around the player slots, so its + 0xE7 and + 0xE8
   are the round counter (low byte first), which a loaded game goes on
   from; capture.md 8.1's 0 to 7 were early rounds. [research, pass 2;
   capture: rr19, 22 written at round 22] Its + 0x36F to + 0x3FF is a work
   area of the on-screen target choice, saved only because it lies in that
   range; play does not read it. [research, pass 2, unconfirmed]
   **Engine:** the round counter is kept in the engine's own save (a
   replay, `docs/saves.md`); the work area is not modelled.

## 8. Left for later

- Destroy in a room (7.5): which things stay, with one thing of each kind,
  with and without another player present, and the floor read before any
  newcomer's save.
- Telling disks apart (3.5), the message for a refused disk, and a full
  roster (7.3).
- Editing keys in name entry (5.5).
- Two "Add Someone" requests in one round (6.2), and a newcomer put in a
  dead slot below a living one (6.1).

## Captures needed

1. Destroy in a room as the last player, the room read in memory at once
   (7.5).
2. "Another adventure" with the adventure's own disk, and the byte that
   decides (3.5).
3. A roster of 34, then a save (7.3).
4. A creature's ratings with marks, if the editor shows them (4.2).
5. From `move-basic-start.vsf` with slots 1 and 2 living: slot 1's first
   byte zeroed (F1 at the offer), then "Add Someone" by two players in one
   round; read which slots the newcomers' records land in and the turn
   order after (6.1, 6.2).
