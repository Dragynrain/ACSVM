# Research brief: new characters, joining and retiring

A request to the project's clean-room researcher (docs/research/README.md).
Licensed CC BY 4.0. Status: briefed 2026-11-14; first pass (items 1-3)
done 2026-11-14, confirmed and in the engine.

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
