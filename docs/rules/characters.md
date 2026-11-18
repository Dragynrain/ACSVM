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
for the first pass and `rr19/` (capture.md 10.23) for the second;
`rr26/` (capture.md 10.30, 2026-11-14), `rr34/` (capture.md 10.38,
2026-11-15), `rr38/` (capture.md 10.42, 2026-11-16) and `rr39/`
(capture.md 10.43, 2026-11-16) are captures of ours with no research
pass behind them (3.1, 3.5, 6.2, 7.3, 7.5).

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
4. **The picture.** After the roll the player picks the character's
   picture, then types its name (1.1). [manual; research] Which pictures
   are offered is [unknown] (capture 8 below). **Engine:**
   `characters::player_pictures`: an Extended adventure's own list
   (`player_pictures`, content.md 3.2), else every picture a creature of
   the master list uses, in picture order, else every picture. A new
   character keeps the one chosen (`CreatePlayer { name, picture }`,
   `Newcomer::Create`) in its record (`Player::picture`); none chosen is
   the first offered, read by the front end (`Player::picture` stays
   `None`, so a game without choices fingerprints as before). The picture
   changes no rule: it retires and rejoins with the character, and a
   front end that ignores it plays the same game.

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
   The same misreading recurs in capture.md 10.43's note that a joining
   player gets Life Force = Constitution and Power = Wisdom (records of
   Life Force 30 under 40 and Power 20 under Wisdom 13 read 40 and 13):
   those values were read at the first room turn, after
   `start-retired.txt`'s long wait at the first map turn and the walk into
   the room, so rounds of drift and healing had passed, each stopping at
   Wisdom and Constitution. It is the 8.4 setup, not a second case, and
   does not change this rule. (Checked against the scripts rr39's
   `case.sh` ran: its cases start from `room.vsf`, the first room turn.)
   A player from this adventure's own roster keeps everything else too:
   possessions, weapon held, armor worn, gold and its picture (it is not
   asked for one). Choosing it clears its name from the roster.
   [research; the picture: research pass 2, unconfirmed; the roster name:
   capture, format-notes, Creatures]
   **Each place on the list keeps its roster entry**, an empty one
   included: with entry 1's name cleared (its record still there) and
   entry 2 U, the screen showed U in the second place with the cursor on
   CREATE NEW PLAYER; moving it to U and pressing joined U's own record
   (its name, Speed and Constitution), and pressing on the empty first
   place rolled a new character (section 1) as CREATE NEW PLAYER does,
   the record left in entry 1 unused. [capture: rr38 `ru-se`, `ru-s`,
   capture.md 10.42 item 5; ours, no research pass, 2026-11-16]
   **Engine:** `AddPlayer` keeps the setup's Power (its Wisdom when the
   setup gives none) and Life Force as given.
   **Its place.** Joined from the adventure's own list, the character
   comes back where it was saved (7.3: the record keeps the place, a
   room's region, room and square too); only through "another
   adventure" is the square set to not placed (3.3, 3.5: "its
   possessions and place are lost, unlike choosing it from the
   add-player screen's own list"). [research; capture: the contrast of
   3.5 only, the own list's place not yet read in a run]
   **Engine:** `Rejoin` brings back a saved record (the `Player` that
   `Retired` carried, 7.3) as a roster add: every rating, Life Force and
   Power as saved, its possessions and gold, the weapon held and the
   armor worn (each only if still among its possessions), and its place.
   The place is kept when it is a world map square, or a square inside a
   room of a region the adventure has (not a deleted room) where no
   living player and no creature stands. Otherwise (a square taken since,
   a place in one of the map's encounter areas, or a room gone) it joins
   at the adventure's start, as a new character does. Whether the
   original lets two stand on one square, or moves the newcomer, is
   [unknown]; the start is the engine's choice. Its moves are its Speed,
   as any newcomer's. Like `AddPlayer`, it saves when it comes in
   ([creatures](creatures.md) 2.2.7.4).
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
5. **No disk is refused.** Whatever disk is given as the other one, the
   program shows the same steps (3.2), lists that disk's roster and joins
   the chosen player as another adventure's player by 3.3 and 3.4, with no
   refusal and no message of its own. That holds with the other disk's
   identity word unlike the adventure's, equal to it, or equal in one
   byte only, and with a copy of the adventure's own disk: its player
   (saved there from this adventure, its square a room's) joined with
   its gold raised from 0 to 1,000, its things gone and its square the
   map's start cell, and that disk lost the name as in 3.4. [capture: ours,
   2026-11-14, rr26 `d2-*` (5 identity words and the own disk, listed
   alike) and `d2b-id-same`, `d2b-own` (taken through to play),
   capture.md 10.30] The researcher's reading, that the disk must carry an
   identity word different from the adventure's and that the same
   adventure's disk is refused, did not show in any of these [research,
   pass 2: disagreed by capture]; nor did setting the identity in memory
   change anything (rr19 `ota-ownid`, `ota-same`, `otb-same`). So a player
   of this adventure brought in this way is treated as a stranger: its
   possessions and place are lost, unlike choosing it from the add-player
   screen's own list (3.1).
   **The list keeps each roster entry's place**: an empty first entry
   leaves a gap, so a disk whose first entry is empty and second holds U
   shows U in the second column with the cursor moved one place. That is
   what rr19 read as the own disk being handled differently. [capture:
   rr26 `d2-ownu`, rr19 `ota-ownid`] On the adventure's own list the
   place chosen joins its own entry's record (3.1). Through "another
   adventure", the one run took a record that was not U's; it was not
   tried again, and the disk swap's timing (3.2) may explain it [unknown].
   Settling it needs a research pass on the swap; the engine has no
   rosters, so it may stay open (capture.md 10.43 (b)).
   **Engine:** a front end with rosters joins the record of the place
   chosen, and treats an empty place as CREATE NEW PLAYER.
   **Engine:** a front end that offers players from another adventure
   applies 3.3 when it builds the setup, for any disk, the adventure's own
   included, and checks no identity; the engine has no rosters or disks
   yet, so 3.2, 3.4 and 3.5 are the front end's when they come.

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
   [research, unconfirmed] Not reachable by capture: the game shows no
   creature's ratings, and the harness does not drive the editor
   (capture.md 10.42 item 5). **Engine:** `characters::mark`, the same
   bounds for creatures, unconfirmed; the viewer shows
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
   turn-order first pass (creatures 2.2.6); capture: rr13 `create`, the
   record in slot 1 of an empty game] **Below a living player too**: with
   slot 1 freed and slot 2 living, the newcomer went into slot 1 (slots 3
   and 4 staying free) and moved before the slot 2 player in every round
   after; with slots 1 and 2 living it went into slot 3 and moved third.
   [capture: rr29 `add/` cases b, a and c, rounds 28 to 34, capture.md
   10.33; ours, no research pass, 2026-11-14]
2. **"Add Someone" always answers "I will add a player as soon as
   possible."** The request is handled at the end of the round, after any
   retirement (7.1): with a free slot the add-player screen comes; with
   none the program says the player must retire one of the adventurers
   before adding another, and adds no one. No character is rolled before
   that screen. [research, pass 2; capture: rr13 `ad-full`, and rr19
   `four-full`, `four-first`: the refusal at the round's end, 2 runs]
   **Several add requests in one round add one player.** Every request
   gets the same "I will add a player as soon as possible", with no
   further message for a second one (unlike retiring's, 7.1); the round's
   end brings one add-player screen and one newcomer, and no other add
   screen comes in the rounds after. It is the same whether one player
   asks twice in its turn (choosing the menu uses no move) or two players
   ask once each, and with more than one slot free. When the living count
   changed in the round as well, the round-end message of 6.3 comes first
   (F1 to go on), then the one add screen. [capture: rr29 `add/` cases a,
   c and b, the screens read at each step, capture.md 10.33; ours, no
   research pass, 2026-11-14] (Superseded: the engine's earlier choice of
   handling each request in turn, which added two.)
   **Two requests with no free slot give one refusal**: "I will add a
   player as soon as possible" each time, one "you will have to retire
   one of your adventurers" at the round's end, then play on. [capture:
   mem, rr34 `i2fb1`, `i2fb2`, capture.md 10.38 item 2; ours, no research
   pass, 2026-11-15]
   **Only a roster add saves the game.** When the newcomer is chosen
   from a roster (this adventure's, 3.1, or another adventure's, 3.2) the
   program saves after the choice ("saving game", "the adventure has
   been saved"): a save point with the newcomer in and the counter gone
   up, so a later death message's button brings back the round after the
   request. A newly created player's add, and a refused add, save
   nothing, and no add ends the visit under way
   ([creatures](creatures.md) 2.2.7.4, with the order beside a SAVE GAME
   or a Retire with Save in the same round). [capture: mem, rr32
   `death/two-button`, capture.md 10.36 item 3; rr34 items 1 and 2,
   capture.md 10.38; ours, no research pass, 2026-11-15]
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
   and handled at the end of the round, after retirements (7.1); a round
   holds at most one waiting add (6.2), so a later request in the same
   round is answered alike and changes nothing (the first request's
   newcomer is the one added; in the original the newcomer is chosen on
   the one add screen); with no free slot it is
   refused then (`AddRefused`, once), with no draw; otherwise the
   newcomer is rolled or set
   up in the lowest free slot (6.1), reusing a dead or retired player's
   entry rather than adding one, so the player list holds at most 4, and
   joins the next round (movement 1.4). A newcomer given as a setup
   (`AddPlayer`) stands for a roster add and its joining moves the save
   point there; a rolled one (`CreatePlayer`) and a refusal save nothing
   (creatures 2.2.7.4, with the order of the round's saves). A slot is living by the engine's
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
   **Engine:** one command, `Command::Retire { save }`, carries both the
   request and the answer to the offer (7.2: `save` true for the button,
   false for the second F1). The original asks the question only at the
   round's end; as the offer draws nothing and nothing in play depends on
   when the answer is given, the outcome is the same. The request is
   refused (`RetireRefused`) while another waits; otherwise
   `RetireRequested`, and `Retired` at the round's end.
2. **The offer**: the button saves the character for later adventures; F1
   warns that it will never be seen again and asks once more, where the
   button still saves and a second F1 destroys. [research, pass 2;
   capture: rr19 `ret-dest`, `last-dest`, `room-dest2`]
3. **Save.** The program takes the first roster entry with an empty name
   (of 34). [research, pass 2; capture: rr26 `d3-free34`, all but entry 34
   named, T saved into entry 34, its name at creatures + 0x1850 and its
   record at + 0x1E6F, where format-notes' 25 names and 6 records a page
   put them, capture.md 10.30] **With all 34 named** it says "Sorry. The
   disk is full. <name> will keep playing." and waits for the button;
   the character stays in play and takes its turns from the next round,
   nothing is written (the disk unchanged byte for byte, so the game is
   not saved either: no save point) and the request is used up.
   [research, pass 2; capture: rr26 `d3-full34`, 1 run, T's turns in
   rounds 27 and 28] **Its Life Force is not raised**: 5 under
   Constitution 11 at the offer, still 5 just after the button, 6 as its
   next turn began (the counter unchanged), 11 only two rounds on, by
   healing ([magic](magic.md) 5.3). [capture: mem, rr34 `i6/full2`,
   capture.md 10.38 item 6; ours, no research pass, 2026-11-15; this
   settles 10.30's 11, which was healing] On a save, Life Force
   below Constitution is raised to Constitution; nothing else changes, Power
   included. The name and the whole record go into the entry: ratings,
   magic defense, gold, weapon held, armor worn, every possession, the
   picture, the joystick and the place (a room's region, room and square
   too). Nothing is left on the square. In play only the slot's first byte
   is zeroed (7.6). The adventure block is written to the disk at the same
   moment, the round counter in it (7.7) and the retired slot's first byte
   0, so the game state is saved along with the character, and that
   write is a save point like SAVE GAME's ([creatures](creatures.md)
   2.2.7): a later death message's button brings back the retire's round
   end, the counter gone up, the character still retired and the other
   players as they stood then. [capture: mem, rr24 `c2-button`,
   capture.md 10.28, 1 run; ours, no research pass, 2026-11-14: T retired
   with Save in round 24, U killed in round 26, back to counter 25, not
   the visit start of round 22] Reversals due later for
   the character (magic 5.2) are dropped [research, pass 2, unconfirmed].
   [research, pass 2; capture: rr19 `ret-save`, `ret-save2`, `last-save`,
   3 runs: Life Force 3 and 4 under Constitution 11 saved as 11, Power 5
   under Wisdom 13 kept, gold 777, the blade held, the mail worn and four
   possessions in the roster's first entry, a room square kept]
   **Engine:** the same when rosters come; until then the front end keeps
   the saved character. When rosters come, a save to a full roster leaves
   the character in play with its Life Force as it was (not raised: the
   raise goes with a save, and none is made), writes nothing and takes no
   restore point. A Retire with Save is also one of the engine's
   save points (creatures 2.2.7): taken at the round's end, it moves the
   restore point there (a `Saved` event after `Retired`), before any add
   of that round (creatures 2.2.7.4); a Destroy does not, on the map or
   in a room (7.4, 7.5). Pending reversals are dropped, as the finding
   says. Bringing a saved character back from the roster (3.1): the
   engine keeps no roster, so the front end keeps the record `Retired`
   carried and gives it back with `Rejoin` (3.1's engine note).
4. **Destroy on the world map**: nothing is put down; possessions and gold
   are lost. Only the slot's first byte changes, nothing is written to the
   roster and the game is not saved then. [research, pass 2; capture:
   rr19 `ret-dest`, memory and the disk otherwise unchanged]
   **Engine:** the same.
5. **Destroy in a room.** The character's things go down on its square,
   all at that moment (nothing changes after), whatever the random state.
   Which go down depends only on what the thing is, not on its kind of
   weapon, armor or treasure, its being held or worn, its "ACS may add"
   count, or whether another player is in play:
   1. **Never put down**: spells, and the creatures' own weapons (horn,
      hand, claw, touches), which weigh 0 and only their owner may use.
      They are lost. Every such weapon a destroy was run with had both
      marks; at a creature's death the owner-only mark alone loses a
      weapon and weight 0 alone does not ([magic](magic.md) 7.2;
      capture.md 10.43 item 10).
   2. **Every other thing goes down**: each weapon, missile weapon, armor,
      treasure and magic item, held, worn or only carried, and the gold as
      one pile. This holds with another player in the room, with another
      player alive only on the world map, and for the last player in play
      alike.
   3. **The order**: the gold first, then the possessions in thing-number
      order, the held weapon and worn armor among them, each on top of the
      last (the last one down is the top of the stack).
   4. **Not a save point**: nothing of the game is written; a later death
      message's button goes back past the destroy to the save before it
      (the room visit's start, the destroyed character back in play with
      its things).
   [capture: ours, 2026-11-14, rr26 `d1-*`, capture.md 10.30: 26 cases of
   one thing (or the gold, or rr19's whole kit) alone, with U in the room
   and with T the last player, 46 runs, the room's list read in memory at
   the destroy; the kit alike for 3 random states] [capture: mem, rr34
   item 5, capture.md 10.38; ours, no research pass, 2026-11-15: as the
   last player, blade, dagger, GOODA and bow all went down on a disk as
   built (blade's count 0, dagger's 4) and with the counts set the other
   way (blade 1, dagger 0) (`d5a-c3`, `d5a-m`); with U alive only on the
   map, all six things of the kit went down (`d5b2`); with T and U in the
   room, the button after the destroy brought back the room visit's start
   with T and its kit (`d5c-button`)]
   This replaces 10.30's third case, that the last player also loses
   things whose "ACS may add" count is 0. The things it saw lost as last
   player came from rr19's `last1` disk, where the thing numbers used are
   a spell and two creatures' weight-0 weapons, so their loss was case 1;
   our blade and treasures were never in those runs [capture: 10.38
   item 5; the earlier reading disagreed by capture]. The researcher's
   reading (gold, blade and bow put down, mail and treasures lost)
   matched none of the runs [research, pass 2: disagreed by capture].
   The rr19 runs this replaces: `room-dest2`, `rd-*`, `newc` (capture.md
   10.23).
   **Engine:** as above: spells and weapons and armor that only their
   owner may use are lost, whatever they weigh, the same test as a
   death's (magic 7.2); everything else lands on the square, top of the
   stack, the gold as one pile (inventory 2.5), whoever else is in play;
   the order is 3's; no save point. A destroy with the two marks split
   was not run [unknown]: the engine takes the death's test, which a
   creature's death showed (capture.md 10.43 item 10). (Superseded: the
   engine's earlier choice of losing only things with both marks.) Then
   as 7.4.
   5. **Waiting reversals never come due.** A player raised by a
      temporary +10 (Life Force 20 to 30, one step back taken, 29) and
      destroyed with its next step due a few rounds on kept Life Force 29
      in its slot, and the queue's entry stayed first and unchanged, for
      more than 50 rounds after, the other player playing on: the step
      was never taken. [capture: rr38 `rv-a`, `rv-b`, capture.md 10.42
      item 5; ours, no research pass, 2026-11-16] The original leaves the
      entry in the queue but never takes it; whether it holds back
      reversals queued behind it is [unknown] (reachable, not yet run:
      capture.md 10.43).
      **Engine:** the destroyed character's reversals are dropped, as on
      Save (7.3): the same in effect, except for that open case.
6. **Afterwards.** The character is out of play: only the first byte of
   its slot is set to zero, so the slot is free for a newcomer at once
   (6.1); the rest stays until a newcomer clears it. The others play on
   from the next round without it. [research, pass 2; capture: rr19, U on
   every turn after, rounds 23 to 47] When the last player retires, saved
   or destroyed, the add-player screen of a game's start comes back, and
   keeps coming back while nobody is in play; a saved character is listed
   there, a destroyed one is not. [research, pass 2; capture: rr19
   `last-save`, `last-dest`] **Engine:** as when all players are dead
   ([magic](magic.md) 7.4): the same `AllPlayersDead` event, whose log
   line says that nobody is left in play (it covers the dead and the
   retired alike), and the game waits for a new player. The saved
   character is not listed by the engine (7.3: no roster yet).
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

- Destroy in a room (7.5): the marks split (only a death was run that
  way, magic 7.2), and whether a reversal left in the queue holds back
  later ones (the engine's choices in 7.5).
- Rejoining a retired character (3.1, 7.3): the place kept on the own
  list is research, not yet read in a run; a saved square taken since is
  [unknown] (the engine joins at the start).
- Which record is taken through "another adventure" from a roster whose
  first entry is empty (3.5; the own list joins the place's entry, 3.1).
- Editing keys in name entry (5.5).

## Captures needed

1. Destroy in a room: the cases, the order, the count changed, U on the
   map and the save point captured (7.5, capture.md 10.30, 10.38), and a
   reversal waiting, never coming due (7.5.5, capture.md 10.42). The
   owner-only mark deciding a creature weapon's loss at a death
   (capture.md 10.43; 7.5.1 takes it). Still open: whether the left
   entry holds back reversals behind it, and a destroy with the marks
   split.
2. Another adventure's disk with the adventure's own identity, and the
   adventure's own disk: captured, no refusal (3.5, capture.md 10.30).
   Choosing U on the own list with an empty first entry: captured (3.1:
   U's record joins, the empty place creates; capture.md 10.42). Still
   open: the same through "another adventure" (a research pass on the
   disk swap, capture.md 10.43 (b)).
3. A roster of 34, then a save: captured (7.3, capture.md 10.30), and
   the full roster's Life Force, not raised (capture.md 10.38).
4. A creature's ratings with marks (4.2): not reachable (no screen shows
   them and the harness does not drive the editor; capture.md 10.42).
5. Two "Add Someone" requests in one round, and a newcomer in a freed
   slot below a living player: captured (6.1, 6.2, capture.md 10.33: one
   player added, in the lowest free slot, moving in its place), and two
   requests with no free slot (one refusal, capture.md 10.38).
6. A Retire with Save as a save point: captured (7.3, creatures 2.2.7,
   capture.md 10.28); a Destroy in a room is none (7.5, capture.md
   10.38). Adds as save points (6.2, creatures 2.2.7.4): captured
   (capture.md 10.38). Still open: a roster add's effect on the visit
   order.
7. A saved character joined from the own list (3.1): where it stands,
   its weapon and armor readied, in a room and on the map; and with its
   saved square taken by another player or a creature since.
8. The new character's picture (2.4): which pictures the original
   offers, in what order, and which is first.
