# Research brief: whose turn comes next

A request to the project's clean-room researcher (docs/research/README.md).
Licensed CC BY 4.0. Status: first pass done 2026-11-12, confirmed by
capture and in the engine.

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
the region's first round a player who had just come through a portal
into another room had no turn (3 of 3 runs; for a next pass); random
creatures appearing and falling away (creatures 2.4, 3).
