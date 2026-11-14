# Research ledger

The state of every clean-room research pass, kept by the integration
orchestrator (`/integrate`, docs/research/README.md 7). One row per
pass; only the orchestrator edits this file, the maintainer may edit it
at any time (to unblock an escalation, set a stage back, or stop a
topic). Licensed CC BY 4.0.

daily cap: 3 passes closed per day

## Passes

| Topic | Pass | Stage | Note |
| --- | --- | --- | --- |
| random-rolls | 1-3 | closed | |
| turn-order | 1 | closed | next: the round a player misses after coming in at the map visit's end, a player's death |
| combat | 1-3 | closed | next: the player's attempt to fire, a Speed 0 friend's turn, the dropped treasure's square |
| stores | 1 | closed | next: map stores, saving the stock, the hang with a store and a disappearing bow |
| characters | 1 | closed | next: Retire, another adventure's player, name entry, creatures catching up |
| encounters | 1 | specced | confirmed and specced by the World map encounters research session (59aa038, rr15, capture.md); engine-implementer waits until that session is no longer waiting on the maintainer, to avoid two writers |
| turn-order | 2 | closed | next: no research pass; captures of ours can settle the restore point, declining to add a player, room-to-room moves, doors ending the turn, and the two carried-moves [unknown]s (brief, second pass) |
| combat | 4 | reviewed | items: the player's attempt to fire, a Speed 0 creature's turn, the dropped treasure's square; queue/combat-p4; capture.md has no entry for its rr16 inputs yet |
| stores | 2 | closed | next: none planned; saving the stock (2.8) is a capture job; reading area items is a MODEL_VERSION question for the maintainer |
| characters | 2 | reviewed | items: Retire, another adventure's player, name entry, how many players, catching up; queue/characters-p2 |

## Log

Newest first. One line per step: date, topic, pass, step, outcome.

- 2026-11-14: turn-order, 2, brief-writer, closed; no pass 3, the rest is capture work.
- 2026-11-14: turn-order, 2, checker, all PASS but fmt, whose only flag is script.rs from the characters confirmer's edit in progress; committed without script.rs, the staged tree rebuilt and tested apart.
- 2026-11-14: encounters, 1, confirmed and specced in 59aa038 by the World map encounters research session; engine step held while that session waits on the maintainer.
- 2026-11-14: turn-order, 2, engine-implementer, visit end, map door ends the turn with moves carried, add a player after all dead (engine and viewer), 6 tests; two engine choices sent to the spec as [unknown].
- 2026-11-14: turn-order, 2, spec-writer, creatures.md 2.2.2-2.2.6, movement.md 2.3/2.6, magic.md 7.4; engine-implementer started.
- 2026-11-14: turn-order, 2, capture-confirmer, items 1 and 2 confirmed (rr18, capture.md 10.22), killed-before-its-turn and item 3 unreachable; an area creature takes map turns (agrees with encounters 6.1); escalated sub-point: when the restore state is written (unsure); spec-writer started.
- 2026-11-14: combat, 4, findings-reviewer, PASS (1 player fires only through FIRE, likely, corrects pass 1; 2 Speed 0 turn; 3 drop square, 80 of 80 in the inputs); capture-confirmer started.
- 2026-11-14: combat, 4, research done; findings-reviewer started.
- 2026-11-14: characters, 2, findings-reviewer, PASS (all five answered; the garbled name strip unsure, held out; gold for another adventure's player corrects pass 1); capture-confirmer started.
- 2026-11-14: characters, 2, research done; findings-reviewer started.
- 2026-11-14: stores, 2, brief-writer, closed; no pass 3.
- 2026-11-14: stores, 2, checker, all PASS; committed (stores files only, the staged tree rebuilt and tested apart).
- 2026-11-14: stores, 2, engine-implementer, no rules change, 3 tests pin 2.5 and 2.7; spec 2.7 wording fixed (region 0 stock is always empty); area items in the model would be a MODEL_VERSION question, left for the maintainer; checker started.
- 2026-11-14 digest: closed 0 today; confirmed: stores p2 item 1; escalations none (turn-order p2's restore-state timing is unsure and held out); running: stores p2 engine-implementer, turn-order p2 capture-confirmer; researching characters p2 and combat p4; encounters p1 on hold for the World map encounters research session.
- 2026-11-14: stores, 2, spec-writer, stores.md 2.5 ([capture], bow joins stock) and 2.7 ([research, unconfirmed]); engine-implementer started.
- 2026-11-14: stores, 2, capture-confirmer, item 1 confirmed (dead drive 8 in snapshots; bow joins stock 32/32, rr17), item 2 unreachable; harness gained a drive check; spec-writer started.
- 2026-11-14: combat, 4, brief-writer, planned and queued after a nudge (combat-p4, 3 items, 4 h).
- 2026-11-14: turn-order, 2, findings-reviewer, PASS (1 no missed round: the map door ends the turn; 2 death and F1 answered, when the restore state is written unsure; 3 lists never hold 5-15, corrects pass 1); capture-confirmer started.
- 2026-11-14: turn-order, 2, research done; findings-reviewer started.
- 2026-11-14: stores, 2, findings-reviewer, PASS (item 1 answered: a lost drive interrupt in snapshots, not a game hang; item 2 not reachable in play); capture-confirmer started.
- 2026-11-14: stores, 2, research done; findings-reviewer started.
- 2026-11-14: turn-order, 2, brief-writer, planned and queued (turn-order-p2, 3 items, 4 h).
- 2026-11-14 digest: closed 0 today; confirmed 0; escalations none; researching stores p2 and characters p2; brief-writers still on turn-order p2 and combat p4; encounters p1 on hold for the World map encounters research session.
- 2026-11-14: characters, 2, brief-writer, planned and queued (characters-p2, 5 items, 4 h).
- 2026-11-14: stores, 2, brief-writer, planned and queued (stores-p2, 2 items, 3 h).
- 2026-11-14: encounters, 1, capture-confirmer, stood down unrun: the World map encounters research session is already sweeping items 1, 2, 4 (rr15, scripts/encountercheck.py).
- 2026-11-14 digest: closed 0 today; confirmed 0; escalations none; running: encounters p1 capture-confirmer, brief-writers for turn-order p2, combat p4, stores p2, characters p2.
- 2026-11-14: turn-order, combat, stores, characters: next passes planned from their closed briefs; brief-writers started.
- 2026-11-14: encounters, 1, capture-confirmer started on items 1, 2, 4.
- 2026-11-14: ledger started from README section 5.
