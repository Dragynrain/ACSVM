# Research brief: whose turn comes next

A request to the project's clean-room researcher (docs/research/README.md).
Licensed CC BY 4.0. Status: first pass done 2026-11-12, confirmed by
capture and in the engine. Pass 2 briefed and queued 2026-11-14.

## Why

Captures settled turn order inside one room: the players move in the
order they joined, then each creature in a room with a player moves once,
in slot order, and the first player comes round again
([creatures](../rules/creatures.md) 2.2; `docs/capture.md` section 7,
"Turn order"). They did not settle what happens when players are in
different places. With player T in a room with three creatures and player
U on the world map, U took about 20 turns in a row (some 140 million
instructions), then T and the creatures took round after round and U
never moved again, in all 5 runs (creatures 2.3). No simple rule fits,
and more scenarios would only add cases. Knowing the scheduler lets the
engine run multi-player games the way the original does.

## What we ask

For the C64 version, in plain language:

1. **The scheduler.** How the program picks the next individual to move:
   what it keeps (a current player, a current place or group, counters,
   timers) and when each changes.
2. **Players in different places.** Why U on the map got a long run of
   turns while T in the room got none, what ends that run, and why U then
   gets no more turns. Is U still in the game (would it move again later,
   and when)?
3. **Creature phases.** When a room's creatures get their phase: after
   the last player of a round, after the turn of a player in their room,
   or something else; whether a room a player only passed through counts;
   why, with both players walking in from the map, the first phase came
   only after both had ended a turn in the room.
4. **The world map.** Whether random encounters or anything else on the
   map takes turns in the same scheduler.

Enough for a first pass: items 1 and 2.

Check against the captured runs below: one player (T, AA, BB, CC every
round), two players in the room (T, U, AA, BB, CC), and the split case.

## How the answers come back

As docs/research/README.md 3 and 4: `findings.md` in plain language only.
Addresses only of state the harness would set or read to test the answer
(for example the current place or a counter), and only if needed.

## Inputs

In `ACSVM-research/research/turn-order/inputs/`. VICE 3.10 `x64sc`
snapshots saved with their disks; start VICE with `-controlport2device 37`
to drive the joystick as `docs/capture.md` 2.2 describes. The scenario is
ours (capture.md 7 and 10.10): an open world map field, start 12,12, a map
door at 12,16 into a 15 x 10 room (entry square 4,4) with three creatures
of ours: AA (slot 0, 12,2, Speed 3, a peaceful friend), BB (slot 1, 2,7,
Speed 5, a peaceful friend) and CC (slot 2, 12,7, Speed 4, a peaceful
neutral). Players T (Speed 6, joined first) and U (Speed 4).

| File | What |
| --- | --- |
| `scenario-disk.d64` | the scenario disk |
| `two-players.vsf` | T's turn on the map, U just added; both on 12,12. Holding south 4 squares walks T into the room |
| `split-streak.vsf`, `.png` | from `two-players.vsf`, T walked south through the door, U idle on the map: some 43 million instructions in, U's turn on the map, early in its run of turns |
| `runs/one/runs.jsonl` | one player, 3 runs: `movers` and `turns` (who moved, when, the fullest bar) |
| `runs/two/runs.jsonl` | both players walk into the room, 1 run |
| `runs/split/`, `runs/split15/` | T in the room, U on the map: 3 runs to 5 of T's turns, 2 runs to 15 |

In the runs, `t` is instructions since the snapshot after the run's
jitter. A few looks between turns read the name strip as `?` or part of
the place name: those are the observer's misreads, not movers.

## Pass 2

Briefed 2026-11-14. The first pass's scheduler is confirmed by capture
and in the engine (creatures 2.2, 2.3; capture.md 10.13). This pass
takes two things it left open. New inputs are in `inputs/pass2/` (table
at the end).

### What we ask

1. **The missed round.** Two players in different rooms of one region
   (capture.md 10.10.4, `rooms2`): U (Speed 4) walked through the map
   door into room 0 in round 11, T (Speed 12) in round 12, the map
   visit's last. In round 13, the room region's first, the list went U,
   AA, BB and T had no turn; from round 14 every round went T, U, AA, BB.
   It happened in all 6 runs, whether T went on through the portal into
   the side room on the same turn (`first`) or stayed in room 0 and took
   the portal on its third turn (`later`); the portal taken inside the
   region's visit cost no round. Why is T left off round 13's list: is it
   the round T came in, the last round of the map visit, the order the
   two players came in, or something kept per player (a place or region
   not yet brought up to date, a flag)? Would a player coming in through
   a map door in any other round of the map visit miss a round too, and
   one going from one room region to another?
2. **A player's death.** The first pass found that a visit ends after a
   round in which the number of living players went down. What else a
   death does to the scheduler: whether the round goes on after it (the
   rest of the list, the creatures), what of the dead player's place on
   later lists and in the cycle's marks, which region is visited next,
   and whether the round counter and the queue of things due later carry
   on. When a player dies and another lives (in the same room, or on the
   map), does the game simply go on with the living one, and when does
   the "restore the last saved game" offer come (only when no living
   player is left, or at every death)? The `death` runs below show what
   happened in ours.
3. Traced only, if cheap: individuals 5 to 15 on room lists (the first
   pass saw them numbered so but no run of ours has them).

Enough for this pass: 1 and 2.

### How the answers come back

As before: `findings.md` gets a "Pass 2" section, plain language only.
Addresses only of state the harness would read or set to test an answer
(for example a per-player flag), and only if needed. For each item say
run or traced, and how many runs.

### Inputs (pass 2)

In `ACSVM-research/research/turn-order/inputs/pass2/` (copies; made by
`capture/runs/rr4/rooms2/` and `capture/runs/rr16/`). Start VICE as for
pass 1. Round numbers are the round counter (`$F65B`); `t` is
instructions from the snapshot.

| Folder | What |
| --- | --- |
| `rooms2/` | item 1. `scenario-disk.d64`: the side-room disk (capture.md 10.10.3-4): room 0 of region 1 as pass 1's room, room 1 (SIDE ROOM, 10 x 8) behind a one-way portal of ours on room 0's 7,4 to its 2,4; T Speed 12, U Speed 4; AA and BB, peaceful friends of Speed 3, on room 0's 2,7 and 2,2. `two.vsf` (`two-map.png`): T's turn on the map, U just added, both on 12,12, a few turns before the behaviour (holding south 4 squares walks a player in) |
| `rooms2/runs/first/` | 3 runs from `two.vsf`, 8 of T's turns: U walks in on its first turn, T walks in and on through the portal (`e3`) on its first. Each turn's start with the region (`$397C`), round counter and current individual (`$71FA`). Round 13, the region's first: U, AA, BB, no T; then T, U, AA, BB every round |
| `rooms2/runs/later/` | the same, T taking the portal only on its third turn: U came in in round 11, T in round 12 (the map visit's last); round 13 again without T, in all 3 |
| `death/` | item 2. `scenario-disk.d64`: pass 1's room with T (Speed 6) and U (Speed 4, Constitution and Life Force 40) and one creature of ours, EE, an aggressive brave enemy on 5,4 beside the entry square 4,4 (Speed 6, Strength and Dexterity 31, Melee 127, a sword of Power 31). `two.vsf` (`two-map.png`): as `rooms2`, T's turn on the map with U just added. `build.sh`, `mem.sh`: how they were made (the `turns` runs in `build.sh` stalled at the death message and were dropped) |
| `death/runs/split/` | 2 runs (identical) from `two.vsf`, T walking in on its first turn, U idle on the map, 500 million instructions, every change of `$397C`, `$F65B`, `$71FA`, T's and U's Life Force (`$F5A2`, `$F5C7`; reads alternate with the ROM's bytes under them, 32 and 177, ignore those). U alone rounds 21 to 39; region 1 from round 40; T's Life Force 0 at about 164.6 million; the message "press button to restore the last saved game and resurrect your dead character, F1 to continue playing from here" (`answer0-05.png`), which our harness answered with the button: the round counter went to 41 and the region byte to 255, then back to round 40 and T's Life Force 22 (the saved game), and a "one of the adventurers is T" screen (`answer0-07.png`); nothing changed after it to the run's end |
| `death/runs/two/` | 2 runs (identical): both walk in (the stick held 10 million instructions), the room region from round 22; EE killed U (Life Force 40 to 1 to 0 by about 22.1 million) while T lived; the same message, answered the same way: round 23, region 255, then round 22 and the saved Life Forces. So the offer came with a living player left. F1 (go on from here) was not tried: what the scheduler does then is the main part of item 2 |

(T's Life Force was set to 1 on the disk but read 22 after joining.)

## Findings

**First pass, 2026-11-12** (items 1 and 2 in full, 3 and 4 from reading;
the hand-off reviewed for plain language and accepted). Summary:

1. **The scheduler goes by region.** Each cycle marks the regions with a
   living player and visits them lowest number first, the world map
   first; regions reached during the cycle wait for the next.
2. **A visit is at most 10 rounds**, fewer if no living player is left in
   the region or a player died that round.
3. **A round's list is built at its start:** on the map its players; in a
   room region the rooms with a player (at most 4), each with its players
   then its creatures in slot order. Requests (add, retire, save) wait for
   the round's end.
4. **U's 19 turns** were the rest of one map visit and a whole one; the
   map and the room then take 10-round blocks in turn. U was never
   dropped.
5. **Addresses** (harness only): the region visited `$397C`, the marks
   `$FE78` on, the round counter `$F65B`, the current individual `$71FA`,
   the round's list `$FF15` on.

**Checked by capture** (capture.md 10.13): reading the region and round
counter at every turn's start over 25 of T's turns, in 2 runs, showed
the predicted blocks exactly. Spec: creatures 2.2, 2.3 tagged [research]
and [capture]; the engine follows them. **Open:** the visit after a player dies;
individuals 5 to 15 on room lists. Captured since (capture.md 10.10.4,
10.14): two players in different rooms of one region go room by room in
player order, each room's creatures after its players, except that in
the region's first round a player who had come in through the map door
in the map visit's last round had no turn (6 of 6 runs, with or without
a portal after; a portal taken inside the visit costs no round); random
creatures appearing and falling away (creatures 2.4, 3). **Next pass:**
why that player misses the region's first round (the snapshots
`rr4/rooms2/two.vsf` and the `later` runs show it), and what a player's
death does to the visit.
