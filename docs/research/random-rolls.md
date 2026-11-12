# Research brief: how the original rolls its random numbers

A request to the project's clean-room *researcher* (CONTRIBUTING.md,
section 3). Licensed CC BY 4.0. Status: **first pass done** (2026-11-11),
by a separate Claude session as the researcher; findings below, the cast
chance confirmed by capture. **Second pass** (items 7 to 13) done
2026-11-12; healing and the temporary tick confirmed by capture and in
the engine. **Third pass** (items 14 to 17) done 2026-11-12; goals,
steps and random creatures appearing confirmed by capture and in the
engine.

## Why

The capture harness (`docs/capture.md`) measures the original as a black
box. Deterministic rules are cheap to measure: one run shows them. Random
ones are not. We can't see or set the program's random state, so we vary
it indirectly (idling a different number of instructions before each run,
the *jitter*) and infer the rule from many outcomes. The cast chance took
60 runs at each of ten Power/cost pairs, and each figure is still only good
to about 10 points (capture.md 7 and 10.5). Rounding and edge cases can't
be pinned down that way, and combat will need many more such formulas.

If the harness could read and set the random state, it could try each
possible roll once and get every random rule exactly. Statistics would
then only cross-check it.

## What we ask

For the **C64 version** (the one the harness runs, in VICE 3.10), in plain
language:

1. **The generator.** What state it keeps (how many bytes), how it makes
   the next number from that state, and how long its sequence is before
   it repeats.
2. **What feeds it.** Whether it ever mixes in outside values (the SID's
   voice 3 oscillator, the CIA timers, the raster line, the jiffy clock,
   key or joystick timing) and, if so, when.
3. **When it advances.** Only when a roll is taken, or also on its own
   (every frame or interrupt, while a menu or prompt waits, during other
   players' and creatures' turns). We have seen that idling in the command
   menu changes the next cast's outcome (capture.md 10.7). We'd like to
   know why.
4. **From a raw number to a roll.** How a roll in a range (for example
   0..cost) is made from the generator's output: remainder, scaling,
   retrying out-of-range values, or something else. Say whether some
   results in the range come up more often than others.
5. **Where the state lives.** The memory addresses of the generator's
   state, so the harness can read and set it. This is the one thing we ask
   for that is not a rule. Capture.md section 9 lists it with you as the
   source.
6. **Who takes rolls.** A list of the game actions that take a roll, how
   many each takes, and what each is compared against, in rule terms
   ("a roll of 0..cost; the cast works when it is below Power"). Start
   with casting a spell, using a magic item, Power drift and natural
   healing, which we have captured and can check you against. Then combat
   (to-hit, damage, armour), creature movement and choices, random
   creatures, and anything else you find.

Write "not found" or "unsure" wherever that is the honest answer. A
partial answer to items 1, 3 and 5 is already enough for the harness to
start.

### Second pass (briefed 2026-11-12)

The first pass answered items 1, 3 and 5, and 4 and 6 for casting (see
Findings). This pass takes the other rolls we have captured, so each
answer can be checked, and the open questions about the generator. All
of it is the C64 version, in the snapshots under Inputs.

7. **Using a magic item.** Does a use take a roll at all? If so, how
   many, of what range, and what is it compared against? Captured: every
   use worked; a use costs 2 Power at Wisdom 10 and 20, and two steps of
   the movement bar; it does not end the turn ([magic](../rules/magic.md)
   4.2).
8. **Power drift.** Does the end of a turn take a roll for drift?
   Captured: Power moved exactly 1 toward Wisdom at the end of every turn,
   in every run, from 10 and from 30 (magic 5.1). We expect no roll; say
   so if there is one.
9. **Temporary Life Force wearing off.** The roll, its range and what it
   is compared with; when in the turn it is taken; whether anything else
   (Speed, time, the size of the temporary part) changes the chance.
   Captured: a temporary +10 lost 1 point at about 1 turn in 3 for a
   Speed 1 player and 1 in 4 for a Speed 15 one, idling or walking alike
   (magic 5.2). Also the **eleventh tick**: in 10 of 13 runs, after the
   10 points were gone, one more tick took Life Force to 1 below
   Constitution and healing put it back the next turn. Why?
10. **Natural healing**, the open one (magic "Captures needed" 2). When
    is healing checked (the end of a turn, each step or frame, the moves
    a turn leaves unused, something else), what is rolled, and how much
    is healed? No per-turn rule fits the captures: a Speed 1 player
    healed 1 a turn, now and then 2 (4 extra in 35 turns); a Speed 15
    player that let its turns run out healed from 6 to 20 in about 12
    turns, 0 to 5 points a turn; walking all 15 squares every turn, it
    healed only 4 to 7 points in 25 turns (magic 5.3). The rule should
    explain all three. Is the test made on Life Force itself, temporary
    part and all, as it seems?
11. **Seeding.** The random state's value at power-on, and whether
    anything on the way to the first turn sets it (loading the program,
    the title, loading the adventure, creating or choosing a player, the
    first turn's start) or only advances it by taking rolls.
12. **Rolls between turns.** Which rolls are taken between one player
    turn and the next: in our scenarios (one player, no creatures in the
    room), and per creature in a room with creatures (the kind and number
    of rolls a creature's turn takes outside combat: square choice, goal
    choice, random creatures appearing, removal of left-behind random
    creatures). Combat rolls wait for a later pass.
13. **The raster band.** Why a draw from the cast menu lands on raster
    lines 37 to 42 (what the program does between the button and the
    draw), and whether rolls taken at the end of a turn or from other
    screens have such a band too, so the harness can predict the line it
    will mix in for each roll it sweeps.

For items 9 and 10, if the rule depends on hidden state (a counter, the
temporary part of Life Force, a timer), give its address too, as for the
random state: the harness then sets it, as it sets `$1CC9`, for an exact
check. Addresses stay for the harness only.

Enough for this pass: items 10, 12 and 13. Items 7 to 9 and 11 should be
short once the end of a turn is traced.

### Third pass (briefed 2026-11-12)

The second pass and the turn-order pass (`turn-order.md`) settled the
generator's users outside rooms and the scheduler. This pass is the rolls
a room with creatures takes. The creatures' behaviour so far is in
[creatures](../rules/creatures.md) sections 2, 5 and 6 (most of it the
engine's guesses, tagged [unknown]); what we captured is in
`docs/capture.md` section 7 ("Turn order", "Following through a door").

14. **A creature's turn.** Which rolls a creature's turn takes, in order,
    and what each decides: the goal it picks (the manual's ordered lists
    by type, creatures 6.1, and "nearer beats better", 6.3: how much
    nearer), the square it steps to among equally good ones (6.6), when
    it stops, and whether it ever wanders with no goal. Say whether a
    creature's turn also takes the players' top-of-turn die and idle-move
    healing rolls (second pass, item 12, from reading only). Captured:
    from the same snapshot, a peaceful friend (Speed 6) went for the
    player in some runs, for the portal (an exit) in others, and once
    wandered off through it with the player still in the room; a
    peaceful neutral with no exit walked up to the player; creatures
    never stepped diagonally.
15. **The roll at a round's end in a room** (second pass, item 12): the
    roll of 0 to 15 and the percentile rolls under 20 that may follow.
    What do they do: random creatures appearing, random creatures
    falling away ("random creatures in rooms no player occupies are
    removed now and then", creatures 2.4), something else? The turn-order
    pass also saw one roll a round against a value of each listed room,
    and one picking a creature slot with a 1-in-5 roll: the same?
16. **Random creatures appearing** when a player enters a room: the roll
    against the room's chance (creatures 3.1), how the square is chosen
    (3.2: "only on a square with no thing on it"), which slot it takes
    (3.3), and when left-behind ones go (2.4). None of our scenario rooms
    has a random creature; reading is enough for a first answer, and say
    what scenario would let the harness check it.
17. **The 8 percentile rolls per walked square** (second pass, 11.1): what
    the table is and what can appear, on the map and in a room.
18. **From loading to the first turn**: which rolls loading the adventure
    and choosing the player take (low priority).

Enough for this pass: items 14 and 15. For each roll, say what decides
its range (a count of goals, of squares, a fixed 100) so the harness can
sweep the random state just before it, as it did for healing.

## How the answers come back

1. **Plain language only.** No code, pseudo-code, disassembly, byte dumps
   or listings, not even in an appendix. Arithmetic in words is fine ("the
   remainder after dividing by cost + 1").
2. **Where it goes.** Rules go into the matching `docs/rules/` spec
   (magic, a new combat spec, and so on). Tag each one **[research]** with
   the date, say which version you studied, and say how sure you are.
   Addresses go into `docs/capture.md` section 9. A short summary goes
   here, under "Findings".
3. **Your notes stay outside the repo**, in the research folder
   (`D:\projects\ACSVM-research`), and are never shown to whoever
   implements the rules.
4. **Each finding is a hypothesis until a capture confirms it.** The
   harness will set the state the finding describes, take the roll and
   check the outcome against the findings and against the statistical
   captures already made. Where the two disagree, the capture wins and we
   come back to you.

## Before starting

These were the maintainer's decisions, not the researcher's. Decided
2026-11-11: the researcher is a separate, fresh Claude session started in
the research folder; the program-disk exception is in CLAUDE.md for
researcher sessions only; the C64 version first. The process is now
`README.md` in this folder.

1. **Who the researcher is.** It must be someone who will not implement the
   rules they report, in the engine or the harness. The person or AI
   session that has been building the engine and harness (including the
   one that wrote this brief) can't take the role. If an AI session does
   the research, it should be a separate session that can see the program
   disks and this brief, and whose only output is the findings.
2. **The program disks.** `CLAUDE.md` currently says EA's program disks
   (`advcnst*.d64`) are never decoded for analysis. This study needs that
   rule to make an exception for the researcher, working only in the
   research folder. Nobody else's rule changes.
3. **Which version.** The C64 version is the one we capture and own. A
   second version (MS-DOS, which we would first need a copy of) would only
   come later, to check whether the ports roll the same way.

## Inputs

In `ACSVM-research/research/random-rolls/inputs/`. Every snapshot is a
VICE 3.10 `x64sc` snapshot of a scenario of ours (`docs/capture.md`
section 7: an open field of the world map, start 12,12, a room south
through a map door), saved with its disk images, so it loads on its own.
The players are retired players of ours (capture.md section 8) with the
stats named; control port 2 has VICE's I/O simulation device in it, as
capture.md 2.2 says.

First pass (top level): `first-turn.vsf` and `scenario-disk.d64` (Power
20, a cost-60 spell), `menu-spell-chosen.vsf` (the spell chosen in
SELECT SPELL: the next button tap casts), `from-menu-runs.jsonl` (60
captured casts from that snapshot, with their jitter).

Second pass (`pass2/`, made with `acs-capture snap`, capture.md 10.11):

| File | The moment | Items |
| --- | --- | --- |
| `item-menu.vsf` | Wisdom 20, owns a magic item of ours (shows a long message when used): Use Object chosen and the item highlighted in SELECT OBJECT; the next button tap uses it | 7 |
| `pow-down-turn-end.vsf` | Speed 1, Wisdom 20, Power 10 (a cost-0 spell of ours took 10 off on turn 1), idling: the movement bar 1 pixel from empty, the turn about to end | 8, 12, 13 |
| `lf-temp-turn-end.vsf` | Speed 1, Constitution 20, Life Force 30 of which 10 temporary (a spell on turn 1), idling, the turn about to end | 9, 12, 13 |
| `heal-turn-end.vsf` | Speed 1, Constitution 20, Life Force about 6 (a permanent -15 on turn 1), idling, the turn about to end | 10, 12 |
| `heal15-turn-end-idle.vsf` | Speed 15, Constitution 20, Life Force about 6, idling, the turn about to end | 10 |
| `heal15-turn-end-walked.vsf` | the same player after walking 14 squares east this turn: 1 step left | 10 |
| `heal15-next-turn.vsf` | the same player at the top of the turn after the spell | 10 |
| `play.vsf` | the program booted from program disk 1, at "put your adventure disk in" (no adventure loaded yet) | 11 |
| `*-start.vsf`, `*-disk.d64` | each scenario's first player turn and its disk (`it-20`, `d-pow-down`, `d-lf-temp`, `d-heal`, `heal15`) | 11, and to replay |
| `*.png` | a screenshot of each snapshot | |

Captured runs to check against (`pass2/runs/`, `runs.jsonl` with one
JSON object a line, and `summary.json`; in drift runs `turns` lists the
profile screen read at the top of each turn: `pow`, `lf`, `wis`, `con`):

| Folder | What | Spec |
| --- | --- | --- |
| `item-10`, `item-20` | two uses a run, Wisdom 10 and 20, 3 runs each: Power before and after each use | magic 4.2 |
| `drift-pow-down`, `drift-pow-up` | Power 10 and 30 against Wisdom 20, 60 turns, 3 runs | magic 5.1 |
| `drift-lf-temp`, `drift-lf-perm`, `drift-disease`, `tick11` | the temporary +10, a permanent +10, the disease stack; `tick11` is 10 more runs of the temporary +10 | magic 5.2 |
| `drift-heal` | Speed 1, healing from about 6, 3 runs | magic 5.3 |
| `heal15-idle`, `heal15-walk`, `temp15-idle`, `temp15-walk` | Speed 15: healing, and the temporary +10, turns idled out or walked (15 squares), 3 runs each | magic 5.2, 5.3 |

Each run began from its scenario's `start.vsf` after idling its jitter
(the `jitter` field, in instructions; capture.md 4.3).

Third pass (`pass3/`): scenarios of ours with creatures of ours in the
room behind the map door (capture.md 10.10). Every creature is a resident
of region 1; its picture is a small block whose shape tells it apart.

| File | The moment | Items |
| --- | --- | --- |
| `one-room-turn-end.vsf` | T (Speed 6) on the room's entry square 4,4, its bar 1 pixel from empty; the creatures move next: AA (slot 0, 12,2, Speed 3), BB (slot 1, 2,7, Speed 5), both peaceful, cautious friends, and CC (slot 2, 12,7, Speed 4), a peaceful neutral. The room has no door out | 14, 15 |
| `follow-room-turn-end.vsf` | T (Speed 12) on 4,4, bar about to run out; a one-way portal on 7,4 (an exit) into a second room; AA (slot 0, 10,2, Speed 6, a peaceful friend) and CC (slot 1, 12,7, Speed 4, a peaceful neutral) | 14, 15 |
| `*-disk.d64`, `*-start.vsf` | each scenario's disk and first turn (on the map) | to replay |
| `*.png` | screenshots of the snapshots | |

Captured runs (`pass3/runs/`; `turns` experiment, one JSON object a run:
`movers` in order, and `rows`, a timeline whenever the mover or a square
changes, `creatures` as [slot, [x, y]]; in the second room squares read 3
right and 1 down of their true place):

| Folder | What |
| --- | --- |
| `one` | the `one` scenario, T idling in the room, 3 runs: the creatures' paths |
| `follow-stay`, `follow-route` | the follow scenario, T idling on 4,4, or walking onto the portal, 3 runs each: AA's and CC's paths varied between runs |

## Findings

**First pass, 2026-11-11** (C64, as running in the scenario snapshots;
brief items 1, 3 and 5, parts of 2 and 4, and 6 for casting only). The
researcher's own notes stay in the research folder. Summary of the
plain-language hand-off:

1. **The generator** keeps one byte of state, at `$1CC9`. A step takes the
   state, combines it bit by bit (exclusive or) with the low 8 bits of the
   current raster line, multiplies by 69, adds 131 and keeps the remainder
   after dividing by 256: that is both the raw number (0..255) and the new
   state. Without the raster part it would visit all 256 values; with it,
   every draw depends on when it is taken.
2. **It advances only when a roll is taken**: not on a timer, not while a
   menu waits. Idling in a menu changes the next roll only by moving the
   draw to another raster line. From a given screen the draw lands in a
   narrow band of lines (37 to 42 from the cast menu), which is why the
   statistical grids came in streaks. A redraw comes almost a frame later.
3. **A roll of 0..n-1**: step = 256 / n rounded down; raw / step rounded
   down, drawn again while it is n or more. Even for evenly spread raw
   numbers. A percentile roll is n = 100 (raw 200..255 are drawn again).
   Also seen: a 1..6 die and a 3..18 sum of three; users not yet traced.
4. **Casting**: see `docs/rules/magic.md` 4.1 (cost 0 always works; Power
   spent first; chance 90 x Power spent / cost held to 14..90; works when
   the percentile roll is below it).
5. **Not found yet**: where the state is seeded; why the band falls where
   it does; whether rolls are taken between turns; the other users of the
   roll (about 30 call sites: item use, drift, healing, combat, creatures).
   These are the second pass, items 7 to 13.

**Second pass, 2026-11-12** (items 7 to 13; the researcher's hand-off
reviewed for plain language and accepted). Summary:

7. **Using a magic item** takes no roll: 2 Power and two moves, Wisdom
   not consulted.
8. **Power drift** takes no roll: 1 toward Wisdom at the top of each turn.
9. **Temporary Life Force** is a queued reversal: 1 point back every 1 to
   6 rounds (a die), for every count from N down to 0, so N + 1 steps:
   the eleventh tick. The queue's round counter and entries are at
   `$F65B` and `$F65F` on.
10. **Natural healing**: each idle move rolls 0 to (the turn's full
   moves - 1) and heals on a 0; the top of each turn rolls a die and
   heals on a 2; both only below Constitution, on Life Force itself.
11. **Seeding**: none. The state is 5 after loading and changes only by
   rolls.
12. **Between turns**: per idle move the healing roll; per walked square
   8 percentile rolls against a table (likely random encounters; not
   traced); at a round's end one die per reversal due and, in rooms, a
   roll of 0 to 15 and maybe more (likely random creatures); at the top
   of a turn the healing die. Creatures' rolls not watched yet.
13. **The raster band** comes from the joystick being read once a frame,
   on line 221; rolls after a button press land some 130 lines later.
   Rolls at a turn's end or while walking fall anywhere in the frame.

**Third pass, 2026-11-12** (items 14 to 17; item 18 not done; the
hand-off reviewed and accepted: plain language, with rank and cost tables
as rule values). Summary:

14. **A creature's turn** draws the top-of-turn die, a smartness roll (0
    to 9, passed below Wisdom - 5) for fighters, friends and enemies, then
    for each step one roll of 0 to 2 per open square beside it. Its goal,
    chosen once at the turn's top, is the lowest rank plus straight steps,
    by tables of type against class (creatures 6.3, 6.4); under 200 or
    none. Each step goes to the 4-neighbour with the lowest 3 x steps to
    the goal + 5 x cost + 4 x roll; it steps all its moves.
15. **After each round in a room** a roll of 0 to 15 picks a slot; if in
    use, each random creature outside the round's rooms is removed on a
    percentile roll under 20.
16. **A random creature appears** on a percentile roll against 5 x the
    room's chance, in the lowest free slot, on one of up to 5 random
    inside squares.
17. **On the world map** each walked square rolls for each of the map's 8
    random-creature records on its terrain (the encounters).

**Checked by capture** (capture.md 10.14): with every draw logged, the
goal and step rules predicted all 1,367 creature steps in 128 runs; a
random creature appeared at chance 75% in 8 and 6 of 10 runs, through the
map door and a portal, in the lowest free slot with its mark. The
researcher's arrival-mark condition did not stop the map door case.
Falling away (item 15), checked later with every draw logged (capture.md
10.14.3; 256 states, 1,280 round ends, 114 removals, all predicted): the
slot roll is as found, but the percentile is one roll whenever the slot
is in use (also with no random creature left), and under 20 every random
creature in a room with no player at the round's end goes, its room on
the round's list or not. Creatures 2.4 follows the capture. **Open, for a
next pass:** fighters (attacks, firing, the second table), the first-step
cost of some things, the map encounters, item 18.

**Checked by capture** (capture.md 10.12): healing, 253 of 253 runs of a
256-state sweep exact; the tick's delay, 252 of 252; the seed, 5. Specs:
magic 5.1-5.3 tagged [research] and [capture]; the engine follows them.
**Open, for a third pass:** creatures' rolls in a room (square and goal
choice, the round-end roll), the 8 rolls per walked square, the rolls
from loading to the first turn, the tick's edge cases.

**Checked by capture** (capture.md 10.9): setting `$1CC9` to each of its
256 values before a cast, items 1 and 4 predicted every first-draw
outcome at four Power/cost pairs, each with exactly one raster line in
the reported band. Items 2 and 3 agree with the researcher's re-run of
the 60 menu captures (60 of 60) but weren't tested separately.
