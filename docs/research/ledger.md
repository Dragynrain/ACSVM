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
| encounters | 1 | checked | engine already matched 59aa038's spec; 5 tests pin it; close waits for tomorrow (daily cap) |
| turn-order | 2 | closed | next: no research pass; captures of ours can settle the restore point, declining to add a player, room-to-room moves, doors ending the turn, and the two carried-moves [unknown]s (brief, second pass) |
| combat | 4 | checked | engine for creature firing, the Speed 0 turn and the drop square only; ESCALATED: the player's fire (combat 8.8) changes Command::Fire and so the save command log; fire draw sweeps fire-e3/e0/ready-e5 still running in rr20 |
| stores | 2 | closed | next: none planned; saving the stock (2.8) is a capture job; reading area items is a MODEL_VERSION question for the maintainer |
| characters | 2 | closed | next: no research pass; capture work: destroy in a room by kind, disk identity, a full roster, a real absence. ESCALATED: Retire in the engine (replay log) and the catch-up (unknown_0c, MODEL_VERSION) |

## Log

Newest first. One line per step: date, topic, pass, step, outcome.

- 2026-11-14: encounters, 1, engine-implementer, no code change, 5 tests pin the captured behaviour; tests and clippy pass; committed; close tomorrow.
- 2026-11-14: encounters, 1, the World map session has closed; engine-implementer started on 59aa038's spec changes.
- 2026-11-14: escalations, the player's Fire (combat 8.8) and Retire (characters 7) in the engine and walk mode, save version 2 (a version 1 fire loads as leave_store); checker all PASS; engine choices recorded in the spec; committed.
- 2026-11-14: the maintainer approved taking on all five escalations in any order: the player's Fire and Retire (save version 1 to 2), the catch-up and area items (MODEL_VERSION 3 to 4), and capturing the restore point. No push.
- 2026-11-14 digest: combat p4 committed (48655f5) and checked; its close waits for tomorrow (daily cap of 3 reached). Nothing running. Escalations waiting: turn-order restore point; area items in the model (MODEL_VERSION); characters Retire (replay log) and catch-up (MODEL_VERSION); combat player's Fire (save command log). Encounters p1 engine step held for the World map session.
- 2026-11-14: combat, 4, checker, all PASS; rr20 fully checked (player fire sweeps strict, a 0..9 roll after each damaging arrow); commit waits for the spec-writer's last edits.
- 2026-11-14: combat, 4, engine-implementer, creature firing with no moves, the Speed 0 turn, the drop square, 5 tests; three readings sent to the spec as [unknown]; checker started.
- 2026-11-14: combat, 4, spec-writer, combat.md 2.7, 2.8, 8.1, 8.2, new 8.8; creatures.md 2.5, 2.7, 8.1.1-8.1.2; magic 5.3.1; the player's fire escalated (save command log); engine-implementer started on the rest.
- 2026-11-14 digest: closed today 3 (stores p2, turn-order p2, characters p2): cap reached, no new passes; running: combat p4 spec-writer, rr20 sweeps (fire draws, drop-thing); escalations waiting: turn-order restore point, area items in the model (MODEL_VERSION), characters Retire (replay log) and catch-up (MODEL_VERSION); encounters p1 engine step held for the World map session.
- 2026-11-14: combat, 4, capture-confirmer, items 2 and 3 confirmed, item 1 by scripts with one sub-point disagreed (capture.md 10.24); the fire draw sweeps and drop-thing still running in rr20; spec-writer started.
- 2026-11-14: characters, 2, brief-writer, closed; no pass 3. Daily cap reached (3 closed): no new passes today.
- 2026-11-14: combat, 4, capture-confirmer, cut off mid-way: items 2, 3 and most of 1 confirmed (rr20), one sub-point disagreed (choosing a bow from FIRE costs a second move; first disagreement); sweeps still queued; a second confirmer started to finish and write capture.md 10.24.
- 2026-11-14: characters, 2, checker, all PASS; committed without the combat confirmer's scripts in progress.
- 2026-11-14: characters, 2, spec gaps settled (a newcomer takes the lowest free slot); engine reuses the slot, test added; checker started.
- 2026-11-14: characters, 2, engine-implementer, names (5) and player count (6) in, adds wait for the round's end, no save or model change; spec gaps (which slot a newcomer takes) sent to the spec-writer.
- 2026-11-14: characters, 2, spec-writer, characters.md 3, 5, 6, new 7; magic 5.5; format-notes; Retire and catch-up escalated (saves, MODEL_VERSION); engine-implementer started on names and player count.
- 2026-11-14: characters, 2, capture-confirmer, items 2-5 confirmed (rr19, capture.md 10.23); item 1 partly: Destroy in a room disagreed (whole kit stays, 5 of 5), first disagreement; disk identity address had no effect; spec-writer started.
- 2026-11-14 digest: closed today 2 (stores p2, turn-order p2; cap 3); confirmed: stores p2 item 1, turn-order p2 items 1-2, encounters p1 (by the World map session); escalations: when the restored state is written (turn-order), area items in the model (MODEL_VERSION, stores); running: characters p2 and combat p4 capture-confirmers; encounters p1 engine step held while the World map session waits on the maintainer.
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
