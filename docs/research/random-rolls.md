# Research brief: how the original rolls its random numbers

A request to the project's clean-room *researcher* (CONTRIBUTING.md,
section 3). Licensed CC BY 4.0. Status: **first pass done** (2026-11-11), by a separate Claude session as
the researcher; findings below, the cast chance confirmed by capture.

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

**Checked by capture** (capture.md 10.9): setting `$1CC9` to each of its
256 values before a cast, items 1 and 4 predicted every first-draw
outcome at four Power/cost pairs, each with exactly one raster line in
the reported band. Items 2 and 3 agree with the researcher's re-run of
the 60 menu captures (60 of 60) but weren't tested separately.
