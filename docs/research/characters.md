# Research brief: new characters, joining and retiring

A request to the project's clean-room researcher (docs/research/README.md).
Licensed CC BY 4.0. Status: briefed 2026-11-14; first pass (items 1-3)
done 2026-11-14, confirmed and in the engine; pass 2 briefed 2026-11-14
(section "Pass 2").

## Why

Every rule that makes a character fight, cast or carry is now measured
and in the engine, but every character still starts with the same
made-up numbers (`docs/rules/magic.md` 8: "trait generation is a later
rule set"). The original rolls a new character's traits and skills when
"Create New Player" is chosen; the manual gives only starting ranges, one
of them lost in our copy. Skill growth needs no pass of its own: Melee,
Parry, Armor, Missile and Dodge learning came out of the combat passes
(`docs/rules/combat.md` 5.5, 8.6). What is left of the character system
is the start of a character's life and the end of it in an adventure:
how it is rolled, how one joins mid-game, and what Retire does. Capture
can show a rolled character's profile, but not the formula behind it,
and Retire works only "as soon as possible", at a moment we can't see.

## What we ask

For the C64 version, in plain language:

1. **Rolling a new character.** For each trait (Speed, Constitution,
   Strength, Dexterity, Wisdom, Size) and skill (Melee, Missile, Dodge,
   Parry, Armor): how its starting value is made, from how many rolls of
   what range, in what order the rolls are drawn from the generator the
   random-rolls brief found, and whether anything else comes in (the
   picture chosen, the adventure, the name, the number of players). The
   manual gives Speed 6 to 7, the other traits 10 to 11, Melee, Dodge and
   Parry 25 to 35, Missile 10 to 20, and an Armor skill range our copy of
   the manual lost. Captured: move-basic's player T was rolled with Speed
   6 (`docs/capture.md` 4.2).
2. **Everything else a new character starts with.** Life Force (the
   manual: equal to Constitution), Power (a retired player of ours joins
   with Power equal to its Wisdom: capture.md 8.4, magic.md 5.1), magic
   defense, gold, possessions, readied weapon and armor.
3. **The profile's up and down marks.** The manual says a mark beside a
   rating means higher or lower than average. What decides it for each
   rating, and is it the same for creatures? Captured: our retired
   player with Constitution 21, Wisdom 25, Strength 23, Size 27,
   Dexterity 24, Life Force 22 and Power 25 shows the up mark on each of
   those and none on Missile 31, Melee 30, Armor 0, Dodge 28, Parry 29 or
   Speed 7 (inputs `stats-profile.png`).
4. **Names.** The manual allows 15 characters; a retired player's record
   keeps 10 bytes (capture.md 8.1). What does the program keep of a
   longer name, in play and when retired?
5. **Adding someone mid-game.** Up to 4 play at once (the manual).
   Whether a dead character still counts toward the 4, what happens to a
   dead character's slot, and what the program does when a fifth is asked
   for. Captured: "I will add a player as soon as possible", then the
   add-player screen when the current turn ends (capture.md 10.10).
6. **Adding a player from another adventure.** The manual says it keeps
   its traits and skills but none of its possessions. What exactly is
   kept and what is reset (Life Force, Power, magic defense, gold, the
   learned skills), and where does it start?
7. **Retire.** When it takes effect after being asked for (capture.md
   8.1: "as soon as possible", later an offer to save or destroy), what
   happens to the character's possessions and gold (left on its square,
   lost, kept in the record), how the turn order goes on without it, and
   what "destroy" removes. Capture.md 8.1 lists bytes the retirement
   writes that we don't understand (adventure block + 0xE7 and
   + 0x36F..0x3FF): what are they, if they matter to play?

Enough for a first pass: 1 and 2. Item 1 is a random formula: please say
where its rolls are drawn, so the harness can save a snapshot just before
and set the generator's state for a sweep (capture.md 9, 10.9), as it did
for casting.

## How the answers come back

As docs/research/README.md 3 and 4: `findings.md` in plain language only.
Addresses only of state the harness must set or read to check the answers
(the moment before the rolls, and where the new character's ratings land),
if capture.md 9 and the decoder's player record don't already give them.

## Inputs

In `ACSVM-research/research/characters/inputs/` (copies). VICE 3.10
`x64sc` snapshots; start VICE with `-controlport2device 37` to drive the
joystick as `docs/capture.md` 2.2 describes, and see capture.md 4 for how
a scenario starts.

| File | What it is |
| --- | --- |
| `boot-play.vsf` | The program booted to "put your adventure disk in" (capture.md 4.1), with its disks |
| `move-basic-disk.d64` | Our move-basic scenario disk (an open map field, no retired player) |
| `move-basic-start.vsf` | move-basic at its first turn: T created with "Create New Player" (first picture, name T) and rolled with Speed 6 |
| `add-player.png` | The add-player screen: add from another adventure, create new player, don't add a player |
| `stats-disk.d64`, `stats-start.vsf` | A scenario with a retired player of ours (T, the stats in item 3), at its first turn |
| `stats-profile.png` | That player's profile, with its up marks |
| `retire-pristine.d64`, `retire-disk.d64` | A scenario disk before and after T retired (capture.md 8.1) |
| `retire-start.vsf` | That scenario at its first turn, before retiring |
| `create-menu-nodisk.vsf`, `create-menu-disk.d64` | The start-up add-player screen with "create new player" selected, saved without disks (restore it with the disk in drive 8, as capture.md 4.2 does): the moment before a new character is made |
| `retire-add-*.png` | The screens after it on the same disk: the selection, "press button of the joystick that will control this player", saving the game, and the first turn |
| `slot2-add.png`, `slot2-add2.png` | The add-player screen listing two retired players of ours, T and U |

## Findings

First pass (2026-11-14), items 1 to 3 with side notes on 4 to 6:
confirmed by capture (`docs/capture.md` 10.18) and specced in
[`docs/rules/characters.md`](../rules/characters.md). In short: three dice
each for Strength, Constitution, Dexterity, Wisdom and Size, a 1-in-3
second chance for the first four, everything else worked out from those;
a joining player keeps its saved Power and Life Force (capture.md 8.4 had
seen rounds of drift and healing); fixed bounds per rating for the
profile's marks.

Asked next: Retire (item 7); a player from another adventure's disk and a
fifth player, run for real (5, 6); name entry's accepted keys (4); and the
first pass's side note that creatures catch up on Power and Life Force
after rounds their room was not visited, by experiment.

Our own runs since (capture.md 10.18) leave two questions for that pass:
how name entry reads keys, since keys fed to the keyboard buffer are
sometimes lost (which state shows a key was taken or refused, so the
harness can feed until it is); and what makes a player slot count as
living, since slots poked with Constitution, Life Force, Speed and a
square still died at once, and when the "retire one of your adventurers"
refusal shows (our request with 4 slots filled was answered "I will add a
player as soon as possible").

## Pass 2

Briefed 2026-11-14. The first pass's rolling, starting values, joining and
marks are confirmed and in the engine (`docs/rules/characters.md` 1 to 4);
names, adding and another adventure's player are there as [research,
unconfirmed] or not at all (characters 3.2, 5, 6, 7). This pass is about
the end of a character's time in an adventure and the corners of joining.
Everything below has inputs in `inputs/pass2/` (table at the end of this
section). T and U are retired players of ours; our things are BLADE
(thing 59), BOW (41) and MAIL (40), as in the combat brief.

### What we ask

1. **Retire** (item 7). From `retire-menu.vsf` (T and U on the map, T's
   turn, the menu's cursor on RETIRE; T owns the blade, bow and mail,
   holds the blade, wears the mail, has magic defense 1, 300 gold and two
   treasures of ours worth 50 and 60):
   - when it takes effect. Already seen: "I will retire you as soon as
     possible", and the save-or-destroy offer for T then shows under U's
     name strip, so at the end of T's turn. Is it always the end of the
     asker's turn, or can it wait longer (in a room with creatures, at a
     region visit's end)?
   - what happens to T's possessions, held weapon, worn armor, gold and
     treasures: kept in the saved record, left on T's square, or lost;
     and the same with "destroy" (F1) instead of the button;
   - how play goes on: U's turns, the turn order, T's slot afterwards
     (free for a newcomer at once?), and what happens when the last
     player retires;
   - what the bytes capture.md 8.1 saw written mean (adventure block
     + 0xE7, 0 to 7; + 0x36F to + 0x3FF), and whether play reads them.
   Check against capture.md 8.1 (the disk diff of a retirement with no
   possessions) and the screenshots here.
2. **Another adventure's player** (item 6; characters 3.2). The start-up
   add-player screen (`create-menu-nodisk.vsf` with `create-menu-disk.d64`
   in drive 8; the cursor is `$7161`, 0 for "add from another adventure")
   and a second scenario disk, `other-disk.d64`, with a retired player V
   (Constitution 20, Life Force 9, Wisdom 20, Power 8, magic defense 1,
   1,500 gold, owning and holding the blade, wearing the mail, two
   treasures). What the program asks for and when (which disk), how it
   tells another adventure from this one (both disks come from the same
   blank adventure, so they may look alike to it: say what it compares),
   what of V's record is kept and what is reset (the first pass's:
   possessions, held weapon and worn armor gone, gold cut to at most
   1,000), whether V's record on the other disk is changed or cleared, and
   where V starts. What is kept we will confirm by capture from the record
   at `$F59F`; the flow and the comparison are the questions.
3. **Name entry** (item 4; characters 5). From `name-entry.vsf` ("type
   name and then press button", nothing typed yet): how entry reads the
   keyboard, which keys it takes and which it refuses (the first pass gave
   ? as a period, : as a hyphen and the double quote as an apostrophe; in
   our runs those and @ left nothing), and what state shows that a key
   was taken or refused, so the harness can feed a key until it is
   (characters 5.3: keys fed to the keyboard buffer are sometimes lost,
   the same ones in every repeat, though each changed the screen). Also
   what happens past 15 characters (our 17-letter name left garbled
   glyphs in the strip and no name at `$F633`).
4. **How many players** (item 5; characters 6). What makes a player slot
   count as living, and when the "retire one of your adventurers first"
   refusal shows. Captured (`ad-full-*`): slots 2 to 4 poked with
   Constitution 10, Life Force 10, Speed 6 and square 12,12 (the first 12
   lines of `ad-full-pokes.txt`), then ADD SOMEONE: "I will add a player
   as soon as possible", and at the turn's end the offer to restore the
   last saved game and resurrect a dead character, as if the poked players
   had died. Which other bytes must be set for a slot to live (enough for
   the harness to fill four), and is the refusal given at the request or
   at the turn's end? `retire-two.vsf` has two real players, to compare a
   living slot 2 with a poked one.
5. **Creatures catching up** (the first pass's side note: creatures'
   Power and Life Force rise with the rounds their room was not visited).
   Captured (`catchup-*`): AA, a Speed 0 peaceful friend of ours in the
   room (region 1), Constitution and Wisdom 20, Life Force 5, Power 2; T
   idled on the map from round 13 to round 39 (`$F65B`), then walked into
   the room (round 40). Read in AA's slot at once (`$E1A3` + 3, + 5):
   **Life Force 5 and Power 2, unchanged** after 27 rounds away; 100
   million cycles later Life Force 16 and Power 10 (healing and drift
   while visited). So no catch-up on a first entry here. Is there one at
   all, and when: a region loaded from disk against one kept in memory,
   leaving and coming back, one room of a region visited while another is
   not? Say what case would show it.

Enough for this pass: 1 and 4. Out of scope: rolling and the marks
(done), skill growth (combat 5.5).

### How the answers come back

As before: `findings.md` gets a "Pass 2" section, plain language only.
Addresses only of state the harness must set or read: a key taken or
refused (3), the bytes that make a slot living (4), anything Retire writes
that play reads (1). For each item say traced or run, and how many runs.

### Inputs (pass 2)

In `ACSVM-research/research/characters/inputs/pass2/` (copies; made by
`capture/runs/ch2/build.sh` and the `to-*.txt` scripts beside it, from
move-basic scenarios as capture.md 4 and 8). VICE 3.10 `x64sc` snapshots
saved with their disks; start VICE with `-controlport2device 37`.

| File | What it is |
| --- | --- |
| `retire-pristine.d64` | The retire scenario disk as built: T (Speed 6, the kit in item 1) and U (Speed 4), retired players of ours |
| `retire-disk.d64`, `retire-two.vsf` | The same after T started and U was added on T's first turn (capture.md 10.10): both on the map at the next turn |
| `retire-menu.vsf`, `retire-on-retire.png` | T's turn, the menu open, the cursor on RETIRE: one button press from the request |
| `retire-asked.png`, `retire-a1.png` | "I will retire you as soon as possible", then the screen after the button |
| `retire-offer.vsf`, `retire-offer.png` | "Press button to save T for further adventuring, press F1 to destroy T" under U's name strip: the moment before save or destroy |
| `create-menu-nodisk.vsf`, `create-menu-disk.d64` | The start-up add-player screen, saved without disks (restore with the disk in drive 8; capture.md 4.2), as in the first pass |
| `other-disk.d64` | A second scenario disk with the retired player V (item 2) |
| `name-disk.d64`, `name-entry.vsf`, `name-prompt.png` | "Create new player", the joystick and the first picture chosen; the name prompt waiting, nothing typed |
| `ad-full-pokes.txt`, `ad-full-will-add.png`, `ad-full-after-turn.png` | Our four-slot attempt from the first pass's `move-basic-start.vsf` (`inputs/`): the pokes and ADD SOMEONE script, "I will add a player as soon as possible", and the offer at the turn's end |
| `catchup-disk.d64`, `catchup-start.vsf` | T on the map at its first turn (round 13), AA in the room behind the map door at 12,16 (item 5) |
| `catchup-late.vsf`, `catchup-in-room.png` | The same idled to round 39, one walk south (about 9 million cycles) from entering the room; the room just after |
| `build.sh`, `to-*.txt` | How each was made |
