# Rules: characters

Behaviour spec for the start of a character's life in an adventure: how a
new character is rolled, what it starts with, joining with a saved
character, the profile's up and down marks, and names. Licensed CC BY 4.0.
The engine implements it in `crates/acs-engine/src/characters.rs` (rolling
and the marks) and `game.rs` (joining). Skill growth is in
[combat](combat.md) 5.5 and 8.6; Power drift and healing in
[magic](magic.md) 5.

Tags as in [movement](movement.md): **[manual]** paraphrased from the
manual, **[unknown]** with the engine's choice as **Engine:**,
**[research]** for the clean-room researcher's first characters pass
(2026-11-14, `docs/research/characters.md`) and **[capture]** where our own
runs confirm it; "[research, unconfirmed]" means no capture of ours has
checked it yet. The confirming runs are `ACSVM-research/capture/runs/rr13/`
([capture.md](../capture.md) 10.18).

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
   **Engine:** `AddPlayer` keeps the setup's Power (its Wisdom when the
   setup gives none) and Life Force as given.
2. A character from another adventure keeps its record but loses all its
   possessions, its weapon held and armor worn with them, and its gold is
   cut to at most 1,000. [manual, in part; research, unconfirmed]
   **Engine:** not yet: there are no character files.

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
   10 bytes. [research, unconfirmed for 15]
2. The packing has 40 symbols: space 0, A to Z 1 to 26, the digits 1 to 9
   27 to 35, then period 36, comma 37, hyphen 38 and apostrophe 39. Each
   pair of bytes, low byte first, holds 3 symbols as first x 1,600 +
   second x 40 + third; a short name ends in spaces (zero bytes).
   [research; capture: rr13 `names`, the name bytes of 4 typed names
   decoded to the letters, digits and marks shown on screen]
3. A digit 0 is kept as the letter O; ! becomes a period; + / ; = a
   hyphen. [research; capture: rr13 `names`] Also seen: * becomes X, and
   [ and ] spaces. [capture: rr13 `names`] The findings also give ? as a
   period, : as a hyphen and the double quote as an apostrophe; in our
   runs those keys (and @) left nothing, but fed keys are sometimes lost
   (letters too), so whether entry refuses them is open. [research,
   unconfirmed] **Engine:** names are folded this way (lowercase to
   uppercase, the findings' folds for ? : and the double quote); any
   other character is dropped.

## 6. Adding someone, and how many

1. A slot is free when its character is dead or none was ever added: the
   dead don't count toward the 4. With 4 alive the program refuses ("retire
   one of your adventurers first") and adds no one, before any roll.
   [research, unconfirmed] **Engine:** the same (`AddRefused`, no draw).
   Asking with three more slots filled by hand (Constitution, Life Force,
   Speed and square poked) still drew "I will add a player as soon as
   possible", so the refusal, if it comes, comes later; but those poked
   characters died at once, so they may not have counted as living.
   [capture: rr13 `ad-full`, `ad-three`]

## 7. Left for later

- Retire: when it takes effect, what the character's possessions and gold
  do, the turn order after it, and the retirement's unknown bytes
  (capture.md 8.1). Brief item 7.
- From another adventure, a fifth player and name entry, by capture (3.2,
  5, 6).
- Creatures catching up after rounds their room was not visited (Power and
  Life Force rising with the rounds missed; a side note of the first pass,
  [creatures](creatures.md) 2): not in the engine until a capture.

## Captures needed

1. A character joining from another adventure's disk (3.2).
2. Name entry with 15 letters, retired; the keys that left nothing
   (? : @ and the double quote), fed until they show (5).
3. A fifth player asked for with 4 alive, and with one of them dead (6).
4. A creature's ratings with marks, if the editor shows them (4.2).
