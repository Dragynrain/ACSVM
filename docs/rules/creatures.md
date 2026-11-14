# Rules: creatures and turn order

Behaviour spec for creatures outside combat: when they act, how a turn goes
round, random room creatures, and the first slice of the AI (goals, paths,
doors, picking up gold and treasure, the overburdened drop, readying
weapons). Licensed CC BY 4.0. The engine implements it in
`crates/acs-engine` (`creatures.rs`, `ai.rs`).

Tags as in [movement](movement.md): **[manual]** paraphrased from the
manual, **[data]** from the stored format or corpus counts, **[designer]**
from Baber's Jadeworld notes, **[unknown]** with the engine's current choice
as **Engine:**. Counts cover all 15 corpus adventures.

Combat is in [combat](combat.md): who a fighter strikes is part of its
step choice (6.5, combat 2), and the attack itself is combat 3 to 7.

## 1. Who is in a region

1. A region has 16 resident creature slots, each a full creature record that
   names its room and square. [data] No more than 16 creatures occupy a
   region at once; past that no random creature appears and no summons are
   answered. [manual] **Engine:** the 16 slots *are* the region's
   creatures: a random creature that appears takes a free slot, one that is
   removed frees it, so the limit is the slot count. Extended adventures
   may have more slots (`limits.residents_per_region`) and add a per-room
   cap (`limits.creatures_per_room`); Classic is unchanged.
2. A creature stays in its region: it takes only doors that lead somewhere in
   the same region (section 5). [manual]
3. Creatures on the world map exist only as random encounters, copies of
   the map's records in the map region's slots 8..15, standing in its
   encounter areas ([encounters](encounters.md)). [research]

## 2. Turns

1. Residents get no turn until a player enters their room. The player moves
   first; the creature then gets its turn even if the player has left the
   room meanwhile, which is how creatures follow players through doors.
   [manual]
2. Turns cycle through every player and every creature in the room.
   [manual] On the C64 the program schedules by **region** (the world map
   is region 0):
   1. A **cycle** marks every region where a living player stands, then
      visits the marked regions, lowest number first, so the world map
      first. A region a player reaches during the cycle waits for the next
      one.
   2. A **visit** is at most 10 **rounds**. It ends sooner after a round
      that leaves no living player in the region, or when the game is left
      by restoring the saved game (2.2.6). A death alone does not end it.
   3. A **round's turn list** is built at its start. On the map: every
      living player on the open map, in joining order, then the
      individuals of the encounter areas in use ([encounters](encounters.md)
      6.1). In a room region: the rooms that hold a living player (at most
      4, in player order), and in each, its players in joining order, then
      the region's creatures standing there in slot order; nothing
      numbered between the players and the region's creatures (5 to 15)
      is ever on a room's list. Turns go in list order; the dead are
      skipped and nobody is added during the round. So a creature takes
      one turn a round, after the players of its room, and a room counts
      only if a player stands in it when the round begins; a creature
      already on the list takes its turn even if the player has left.
   4. **A player who goes through a map door into a room ends its turn
      there**, whatever moves it has left. It is on the list of the first
      round of its new region's next visit, by its player number like any
      other player there; no round is missed, whichever round of the map
      visit it came in and whichever player came in first. If it was the
      last living player on the map, that visit ends at the round's end
      and, when the room's region is the next marked one, its first round
      begins with this player, so the two turns run back to back. Its
      first turn in the room starts with the moves it had left at the
      door, or its full Speed if it had none left. Two things about such a
      turn are [unknown]: the full moves the idle healing roll uses
      ([magic](magic.md) 5.3) and whether the carried moves are halved
      beside an enemy (2.6); the engine's choices are given there.
   5. **After each round**, in this order: the round counter goes up by
      one and anything queued for that round is handled; in a room region,
      the end-of-round random creature rolls (2.4); if the number of
      living players (counted over every region) changed during the round,
      either way, the death message (one, however many died); the requests
      made during the round (retiring a player, adding one, saving:
      [characters](characters.md) 6.2, 7.1, and 2.2.7); then the visit
      ends if 2.2.2 says so. A round is never cut short by a death: every
      individual still on the list takes its turn.
   6. **The death message** comes after every round in which a player
      died, whether or not another lives and wherever it stands. It offers
      the button (restore the last saved game) or F1 (go on). **F1** changes
      nothing in the scheduler: the visit, the round counter and the queue
      go on. The dead player keeps its record and place but is out of the
      cycle's marks and gets no more turns; when the visit ends, the next
      marked region is visited, or a new cycle is marked from the living
      players' places. With nobody alive anywhere, the game does not end:
      it goes to the add-a-player screen (from another adventure, a new
      character, or none), and an added player takes the first dead
      player's slot. By then every player record is cleared to 0, place
      and gold included. **Declining** (DON'T ADD A PLAYER) reads the
      disk and brings the same screen back, the cursor on CREATE NEW
      PLAYER: no turns are taken and the round counter and region stay as
      they were, for as long as nobody is added (as after the last player
      retires, [characters](characters.md) 7.6). **The button** clears the
      marks and requests, reads back the saved state (players, round
      counter, queue), shows each living player's "one of the adventurers
      is" screen, ends the visit and starts a new cycle from the restored
      places.
   7. **When the saved state is written.** The state the button restores
      is the later of two saves:
      1. **A visit that changes region**: when a visit begins in a region
         other than the one the visit before it was in, the state at that
         visit's start is saved, its round counter being the visit's first
         round. Not the moment of the door step: a player who walks through
         a map door in round 13 is saved with round 14, when the room
         region's visit begins. A visit of the same region as the one
         before it (one player alone, visit after visit) saves nothing, so
         a death there goes back to the start of the first of that run of
         visits.
      2. **SAVE GAME** (the menu's twelfth entry, last of the third row):
         it answers "I will save the game as soon as possible", offers F1
         for a return to the editor after the save, and ends the turn. The
         save is made with the round's other requests (2.2.5), so it holds
         the state at that round's end with the counter already gone up:
         chosen in round 24, it restores to the start of round 25.
      A save in the same round as a region change is [unknown]. **Engine:**
      the later of the two, as above; SAVE GAME is a command logged like
      any other, taken at the round's end; with both in one round, the
      visit start that follows the round's end is the later one.
   [research: turn-order first pass, 2026-11-12, `docs/research/turn-order.md`] Confirmed: with T in a room with three creatures and U
   on the map, the scheduler's region and round counter, read at every
   turn's start, showed U alone for rounds 21 to 39 (the rest of one map
   visit and a whole one: 19 turns), then T, AA, BB, CC for rounds 40 to
   49, U for 50 to 59, and the room again from round 60, in both runs.
   [capture: turns --peek, capture.md 10.13] And by name strip: T, then
   slots 0, 1, 2 (in slot order, not by Speed, distance or name), then T,
   in 17 phases of 6 runs and 9 of 3 with the slots swapped; T, U, then
   the creatures in 21 phases of 7 runs, never after T alone; and the
   first creature turns after both players had walked in only once both
   stood in the room at a round's start. [capture: turns, capture.md
   10.10] Two players in different rooms of one region: T in the side
   room, U and two creatures in room 0; every round went T, then U and
   the creatures, in 3 runs. [capture: turns, case rooms2, capture.md
   10.10.4]
   **The map door and the first room round (2.2.4)** [research:
   turn-order second pass, 2026-11-14, `docs/research/turn-order.md`].
   The name strip had seemed to show T missing the room region's first
   round when it came in on the map visit's last round; that was its
   map-door turn and its first room turn merged. Read from the
   scheduler's state at every turn's start: U in through the map door in
   round 11, T in round 12 after 4 of its 12 squares (its moves left 8,
   the current individual cleared at the door), and round 13, the room
   region's first, went T, U, AA, BB, as every round after; the same
   with T coming in first (both in round 12) and with T coming in during
   round 14, the second round of a later map visit (room rounds from 15,
   U idle in rounds 12 to 14 as its region was not marked). T's first
   room turn began with 8, 8 and 1 moves (what it had left), U's with 4
   (its full Speed, having used all 4). [capture: mem, cases held,
   tfirst, ulater, capture.md 10.22] This also accounts for movement 2.3
   (a lone player seems to go on with the rest of its turn). Going from
   one room region to another the same way is [research, unconfirmed]
   (traced only). **Room lists never hold 5 to 15** [research,
   unconfirmed]: none of ours could show it either way (in 10 runs no
   room list held one, but nothing of ours is numbered so; capture.md
   10.22). On the map, an encounter area's creature does take turns
   (T, then map slot 8, in rounds 15 to 17; capture.md 10.22, as
   encounters 6.1).
   **Deaths (2.2.5, 2.2.6)** [research: turn-order second pass,
   2026-11-14]. With F1 answered: EE killed U, round 23 went on in the
   room region with T and EE and no U; EE then killed T, and after the
   second message came the add-a-player screen. With T killed in the
   room region's first round and U on the map, the map was visited with
   U alone every round from 41 to 76, the room region never again. With
   a friend behind EE on the list, the friend still had its turn after
   each kill, before the counter went up. In all 5 death messages the
   counter had gone up just before; 3 came with a living player left, 2
   with nobody. With the button: the region went to none and the counter
   and Life Forces back to the visit's start, and after the "one of the
   adventurers" screens the room region was visited again from that
   round. [capture: mem, cases f1-two, f1-split, after/f1, button,
   button2, capture.md 10.22] Not shown: the queue going on after F1
   (nothing was queued). A player killed before its own turn in the same
   round is skipped like any dead individual [research, unconfirmed]: no
   list of ours had its killer before it.
   **The restore point and declining (2.2.6, 2.2.7)** [capture: mem,
   rr21, capture.md 10.25; ours, no research pass, 2026-11-14]. Gold and
   a creature's class poked to a new value at each step marked what came
   back; one run a case (a `mem` run repeats itself), every restore of a
   run the same. SAVE GAME in round 24, a kill in round 25: the button
   brought back counter 25 and round 24's end state; without the save,
   the same pokes went back to counter 22, the room visit's start, in all
   10 restores (`a-save`, `a-nosave`). T alone in a room region from
   round 14, three visit ends passed with no region change, killed in
   round 46: back to counter 14 in all 15 restores (`b-button`). A room
   region visited from rounds 40, 60 and 80 with the map visited in
   between, a kill in round 85: back to counter 80 and that visit's
   starting values, then the map first (`b-split-button`). Declining
   twice after F1 with nobody alive: "accessing disk", the same screen,
   and no turn for 150M instructions, T's record all 0 (`decline`).
   **Engine:** the rules 2.2.1 to 2.2.7. A dead player is skipped on
   every list and left out of the marks; the round finishes after a
   death. The death message, its two answers and the add-a-player
   screen are the front end's: F1 is simply going on; the button is
   loading the last save point of 2.2.7 (magic 7.4, `docs/saves.md`
   1.4); with nobody alive the engine reports it (`AllPlayersDead`),
   clears the dead players' records and waits for a new player.
   Declining needs no command: the front end shows the screen again
   and the engine takes no turn.
3. So creatures act only while a player stands in their room at a round's
   start, and move once a round however many players share the room. Two
   players in different places do not alternate: each gets its region's
   visit of up to 10 rounds in turn. [research, capture] On its next turn
   a creature catches up the Power and Life Force of the rounds it missed
   ([magic](magic.md) 5.5). [research; capture]

4. **Random creatures fall away.** Random creatures in rooms no player
   occupies are removed now and then. [manual] On the C64, after each round
   in a room region one roll of 0 to 15 picks a slot; if it holds a
   creature, each random creature of the region (resident ones never)
   takes a percentile roll, and under 20 it goes if no player is in its
   room (as settled below). So a left-behind random creature goes with chance (slots in
   use / 16) x 20% a round. [research: random-rolls third pass,
   2026-11-12, `docs/research/random-rolls.md`] The researcher read the
   percentile as one per random creature whose room was not on the
   round's list. Captured with every draw logged: the percentile is
   drawn once whenever the slot is in use, with no random creature left
   too, and a random creature goes at the end of the very round in which
   the last player left its room, though that room was on the round's
   list. So what counts is the room being empty at the round's end.
   In 256 runs from one snapshot,
   each with its own random state, the rule predicted all 1,280 round
   ends' draws and the 114 removals, 27 of them at the end of the round
   the player left in; the researcher's reading got 154 runs wrong. [capture: mem --draws, case fall,
   capture.md 10.14.3] Not captured: two random creatures at once (one
   roll or one each); a player walking into a room mid-round (by the rule
   above it protects the room's random creatures). In the combat cases
   (the player and one or two residents in one room, no random creature)
   each round end drew the slot roll and then exactly one percentile: when
   the slot was in use, this one only, never the further roll `fall`
   showed; when it was free, one percentile too. [capture: the second
   combat pass's 160 melee runs, capture.md 10.14.10]
   **Settled by the combat researcher's third pass** (2026-11-13): when
   the slot is in use, the program goes through all 16 slots and draws a
   percentile for each one **marked as a random creature** (the top bit of
   its Dodge byte); a creature without the mark draws nothing. Below 20
   the creature is removed if it is alive and not in a room with a player.
   The one percentile of the combat cases (in-use slot or free) is the
   next round's random-creature check for the player's room (3.1), drawn
   as its list is built; `fall`'s further one is the random creature's.
   [research: run, 16 replays of `fall`; traced] A removed creature's slot
   keeps the mark (its Constitution goes to 0, the mark byte stays set in
   all 114 of our removals), which is why the roll goes on after it has
   gone. [capture: fall, 256 runs] Not captured: two random creatures at
   once (one roll each, by this). **Engine:** at the end of each round of
   a room region's visit, the slot roll, then a percentile for each random
   creature still there (a removed one draws none: the engine keeps no
   mark), each removed below 20 when no living player stands in its room;
   it takes what it carries (`CreatureRemoved`, reason `left_behind`);
   what becomes of its things in the original is not known. The engine
   takes the room's random-creature roll on entering (3.1), not each
   round.
5. **The movement bar.** A creature's turn has a bar like a player's: its
   Speed after the weight slowdown ([inventory](inventory.md) 3.3), using
   the creature's own Strength, Constitution, Life Force and load. On the
   C64 it shows on the left as a player's does, 8 pixels a point of Speed
   (24, 40 and 32 pixels for Speeds 3, 5 and 4, carrying nothing), and
   drains as the creature walks. [capture: turns] The slowdown part is
   [unknown].
   **Engine:** so. A creature with Speed 0 has a bar of 0 but still takes
   one action a turn (2.7); the manual adds that it does not show its name
   on its turn. [manual]
6. **Standing next to an aggressive, unfriendly individual** shortens a
   player's bar. [manual] By how much is not given. On the C64 the bar is
   halved, and only for a creature on one of the 4 squares beside the
   player (north, south, east or west), not diagonally: a Speed 12 player
   (96 pixels) on the room's entry square showed 48 at the top of each of
   its next turns with an aggressive enemy or an aggressive neutral (Speed
   0) on the square east of it, and 96 with the same enemy diagonally next
   to it or two squares east, with an aggressive friend or a peaceful
   enemy east of it. [capture: turns, the adj- cases, 2 runs each,
   capture.md 10.10.2] **Creatures too**: at the top of any individual's
   turn, as the bar is refilled, its moves are **halved, rounding up**,
   if an aggressive individual or a player not on its side stands beside
   it (Speed 3 gives 2, 6 gives 3); the weight slowdown comes after the
   halving. [research: combat third pass, every turn start of about 1,500
   in its replays; the slowdown's order traced] An enemy of Speed 3 had 2
   moves on every turn it began beside the player, 3 otherwise, and the
   player 3 of 6 beside it. [capture: every pass 2 and 3 combat run,
   capture.md 10.16] **Engine:** so, for players (a creature on the 4
   squares beside that is Aggressive and not a Friend) and creatures (a
   foe as combat 8.1); Speed 0 stays 0. The `TurnStarted` event says so
   for a player (`hindered`); a creature's `CreatureTurn` gives the moves.
   Applied at turn start only; whether walking up to such an individual
   mid-turn shortens the rest of the bar is not captured.
   **A turn that starts with carried moves** (after a map door, 2.2.4):
   whether the moves carried over are halved when such an individual
   stands beside the player is [unknown]; no capture had one there.
   **Engine:** they are not halved, and `TurnStarted` reports `hindered`
   false for that turn.
7. **A creature's turn is a loop**: it takes one action (after the top of
   the turn: catch-up, drift and the healing die, [magic](magic.md) 5), then
   another while it has moves left. The test comes after the action, so a
   creature of **Speed 0** takes exactly **one action** every turn:
   [research: combat fourth pass, 2026-11-14, `docs/research/combat.md`,
   run]
   1. its smartness checks (6.6: one for a friend that is not a fighter,
      two for a fighter);
   2. then, holding a missile weapon with [combat](combat.md) 8.1's rule
      holding, one shot (combat 8.2: Dexterity / 8), which ends the turn;
   3. else the step choice (6.5), its roll of 0 to 2 for each open square
      beside it; the step is then **cancelled** when it has no moves left
      and the square picked is an ordinary one (cost 2 in 6.5), so it never
      walks;
   4. with no step, one **idle move**: [magic](magic.md) 5.3.1's healing
      with the turn's full moves 0, which draws nothing and heals 1 while
      Life Force is below Constitution.
   The class changes only the number of smartness checks. A peaceful friend
   and an aggressive brave enemy, each Speed 0, Constitution 40, Life Force
   30, the player 5 squares off: the die, their checks, four step rolls,
   never a step, Life Force up 1 every turn (2 with the die's 2) until 40,
   never past it; at Life Force 40 it never changed. [capture: still-hurt,
   still-enemy 256 runs each, still-whole 64, 512 turns of each of the
   first two, capture.md 10.24; rr16's and rr8's `bow-line` runs too]
   (Superseded: capture.md 10.14.10's fit of a Speed 0 friend's turn, a
   percentile, a roll of unknown size and a roll of 0 to 0: those were the
   step rolls read with other sizes; it fails most of the 10.24 runs.)
   The step is not cancelled onto the goal's own square (cost 1), an exit,
   or a thing it takes, so a Speed 0 fighter owning a melee weapon attacks
   a goal beside it once a turn: [research, unconfirmed: traced, no capture
   had one]. **Engine:** 2.7 as written, the smartness checks as 6.6's
   engine choice says. Two readings the research leaves open, both
   [unknown]:
   - **Every creature's first action is taken.** Whether 2.7's first
     action, taken whatever the moves left, holds for creatures of Speed
     above 0 too. **Engine:** it does: a creature whose readying (8.2)
     spent its last move still gets its shot or its step choice, the step
     cancelled as in 2.7.3, and with no step an idle move whose healing
     roll ([magic](magic.md) 5.3.1) runs 0 to its full moves less 1.
   - **What counts as ordinary** for the cancelled step (2.7.3).
     **Engine:** only an empty square (nothing on it, no individual).
     A square that is not the goal's but holds an individual the creature
     would strike is not ordinary, even where 6.5's table gives it cost 2,
     so that step is not cancelled.

## 3. Random room creatures

1. On entering a room there is one roll against the room's chance for its
   random creature (movement 7.2). [manual] On the C64 the chance is 5% a
   point of the room's 0..15 setting, and the roll comes once a player has
   walked in: through the map door the creature showed as the room was
   drawn, through a portal into another room (which ends the turn,
   [movement](movement.md) 2.3) as the next round began. With the chance
   at 15 (75%) one appeared in 8 of 10 walks in from the map and 6 of 10
   through a portal. [research: random-rolls third pass, 2026-11-12, `docs/research/random-rolls.md`] [capture: turns, cases rc-map and
   rc-side, capture.md 10.14] The researcher read the roll as taken each
   round as the list is built and counting only while every player in the
   room has just arrived; the map door case shows it is not lost when the
   arriving player's turn goes on. **Engine:** one percentile roll against
   the chance when a player enters a room.
2. It appears only on a square with no thing on it. [manual] On the C64
   up to 5 tries: a random column and a random row of the room's inside;
   a square with an individual or a thing is rejected; after 5 rejections
   nothing appears. [research] The squares seen were spread over the
   room's inside. [capture] **Engine:** the same.
3. It is a copy of the region's random creature record named by the room
   (one of 8), with the random mark, put in the lowest free slot, and it
   takes its turns in the room's list like any creature. [research;
   capture: slot 0 with the mark, 16 of 16 appearances] **Engine:** event
   `RoomEncounter` with the slot and square.
4. The roll is skipped when all 16 slots are in use (1.1).

## 4. Personality

1. The strategy line has three settings: Aggressive or Peaceful, Brave or
   Cautious, and Friend, Enemy, Thief or Neutral. [manual] All three are in
   the record. [data]
2. **Fighter or slinker.** A creature that is Aggressive and either Brave or
   unwounded (Life Force at least Constitution) behaves as a fighter. One
   that is Peaceful, or Cautious and wounded, behaves as a slinker. Slinkers
   never attack or cast. [manual] Worked out afresh at the start of each of
   its turns. [unknown]
3. Placed residents by type at the start (all 15 adventures, 3,248): 1,985
   fighter enemies, 632 fighter friends, 494 fighter neutrals, 105 fighter
   thieves, 16 slinker friends, 9 slinker thieves, 7 slinker neutrals and no
   slinker enemies. 6 have Speed 0. [data]
4. **Smart or dumb** (Wisdom above 14, below 6, chance in between) matters
   for casting and for a hurt neutral turning enemy: the magic and combat
   rules. Not used here.

## 5. Where a creature may step

1. Creatures never move onto a space or an obstacle, custom ones included.
   [manual] **Engine:** nor onto walls, stores, or another individual.
2. **Things it won't pick up.** Creatures never pick up magic items,
   weapons or armor; only thieves pick up treasure; all creatures except
   friends and slinker neutrals pick up gold. [manual] An AI will not walk
   onto a magic item. [designer] **Engine:** a square whose top thing is
   carriable but not one this creature picks up (spells included) is closed
   to it, and so is one it picks up but is at the carry limit for. A square
   whose top is a floor or nothing is open. Things lower in a stack do not
   matter (movement 5.3).
3. **Doors.** Creatures move onto a door only if it has no do-all, rid-room
   or add-to-room spell and it leads somewhere in the same region. [manual]
   **Engine:** a portal whose destination is a room of the same region (a
   deleted one included, see 5.5); its access applies to the creature as to
   a player (owners of / non-owners of, with the needed thing used up if the
   door destroys it); an "invoke spell" door is closed to creatures when the
   spell is Activate All, Rid Room or Add to Room. Other spell doors let them
   pass and the spell goes off for them ([magic](magic.md) 6.6). [unknown]
4. **Taking a door** costs one step and ends the creature's turn. It arrives
   on the door's destination square. Baber notes that a creature following a player can share
   the player's square for a moment. [designer] On the C64, a friend of
   ours (Speed 6) that followed the player through a portal into another
   room vanished from the first room on the step onto the portal, its turn
   ending there, and showed in the new room on its next turn on the
   portal's destination square, on top of the player standing there; it
   then stepped off. [capture: turns, case follow, 2 runs, capture.md
   10.10.3] With a creature of ours (Speed 0) standing on the destination,
   the player landed on it too, and the friend that followed landed there
   as well, three on one square, then stepped off (6 runs, the friend
   following in 5). [capture: turns, case held, capture.md 10.10.5]
   **Engine:** taking a door costs the step and ends the creature's turn;
   it arrives on the destination square whoever stands there, sharing it
   until they move. The original keeps the creature out of sight until
   its next turn; the engine places it at once.
5. A creature sent through a door into a deleted room is banished.
   [manual] **Engine:** removed (`CreatureRemoved`, reason `banished`) with
   what it carries.
6. Moves are one square to one of the 4 squares beside it, never
   diagonally (6.5). [research, capture]

## 6. Goals

1. On its turn each type looks for its targets in a fixed order: [manual]

   | Type | Seeks, in order |
   | --- | --- |
   | Fighter enemy | player; aggressive friend; peaceful friend; exit; aggressive neutral or thief; peaceful neutral or thief; gold |
   | Slinker enemy | peaceful enemy; aggressive enemy; exit; gold |
   | Fighter friend | aggressive enemy; aggressive neutral; peaceful enemy; aggressive thief; player; exit |
   | Slinker friend | player; exit |
   | Fighter neutral | aggressive friend or enemy; aggressive thief; peaceful friend or enemy; peaceful thief or aggressive neutral; peaceful neutral; gold; exit |
   | Slinker neutral | exit |
   | Fighter thief | treasure or gold; exit |
   | Slinker thief | treasure or gold; exit |

2. **Nearer beats better** and a wounded slinker seeks an exit. [manual]
   On the C64 a creature chooses **one goal at the top of its turn** (after
   the healing die) and keeps it all turn. Every candidate scores its
   **rank plus the steps** from the creature's square (columns apart plus
   rows apart, walls ignored); the lowest score wins, a tie going to the
   one looked at first (individuals by number, players first, then things
   in the room's order); nothing under 200 means no goal. Candidates are
   the other individuals standing in its room, and exits, gold and other
   treasure on its squares. A player who has left the room is not one, so
   a friend left behind goes for an exit (often the same door). The
   wounded slinker's exit is just its type's ranks: a cautious creature
   that is wounded becomes a slinker. [research: random-rolls third pass, 2026-11-12, `docs/research/random-rolls.md`] Confirmed: with each
   draw logged, the goal and step rules predicted every step of a
   peaceful friend (the player against a portal) and a peaceful neutral
   (the portal, or no goal) in all 64 runs from each of two rooms, 1,367
   steps. [capture: mem --draws, capture.md 10.14]
3. **Ranks of individuals** by the creature's type (rows) and the other's
   alignment and setting (columns; A aggressive, P peaceful); lower is more
   wanted, "-" never: [research]

   | Type | Player | A friend | P friend | A enemy | P enemy | A neutral | P neutral | A thief | P thief |
   | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
   | Fighter enemy | 0 | 4 | 6 | - | - | 80 | 84 | 80 | 84 |
   | Fighter friend | 32 | - | - | 0 | 4 | 3 | - | 5 | - |
   | Fighter neutral | 1 | 0 | 4 | 0 | 4 | 6 | 8 | 2 | 6 |
   | Fighter thief | - | - | - | - | - | - | - | - | - |
   | Slinker enemy | - | - | - | 29 | 25 | - | - | - | - |
   | Slinker friend | 26 | - | - | - | - | - | - | - | - |
   | Slinker neutral or thief | - | - | - | - | - | - | - | - | - |

4. **Ranks of things** (an exit: a door it may take, 5.3; gold and other
   treasure it would pick up, 5.2): [research]

   | Type | Exit | Gold | Other treasure |
   | --- | --- | --- | --- |
   | Fighter enemy | 96 | 128 | - |
   | Fighter neutral | 96 | 96 | - |
   | Fighter friend | 96 | - | - |
   | Fighter or slinker thief | 96 | 32 | 32 |
   | Slinker enemy | 32 | 96 | - |
   | Slinker friend or neutral | 32 | - | - |

   So a slinker friend takes an exit over the player only when the exit is
   at least 7 steps nearer; a fighter enemy an aggressive friend over the
   player at 5 steps nearer. Where these differ from the manual's lists
   (a fighter enemy puts neutrals and thieves above an exit; a fighter
   neutral seeks players), the numbers are what the C64 does.
5. **Each step** looks at the 4 squares beside the creature, in the order
   south, north, east, west, and takes a roll of 0 to 2 for each open one
   (5.1, 5.2). It steps to the open square with the lowest **3 x the steps
   from it to the goal + 5 x its cost + 4 x its roll** (a tie to the first
   looked at). With no goal it steers by the square it started the turn
   on, so it wanders about there. The cost: 1 for the goal's own square;
   an exit 1 for a slinker, 3 for a fighter; gold or a treasure it would
   take, its thing rank / 32; 4 for a square it has stood on this turn
   (its starting square too); 2 for any other. One step nearer saves 6
   and the roll adds 0, 4 or 8, so it often steps sideways or back: that
   is why paths differ from run to run. It steps every move of its bar,
   **even beside its goal**, unless an action ends its turn (7) or no
   square beside it is open, when it stands and waits the bar out (each
   idle move taking magic 5.3's healing roll). [research, capture]
   When the square it picks holds gold or a treasure it takes, it takes
   the thing from where it stands, without stepping onto it, and its turn
   ends (7.5). [capture] Confirmed with things: a peaceful thief and an
   aggressive thief going for treasure and gold, a peaceful enemy for
   gold, two treasures at the same score going to the first in the room's
   order, every step and take of 64 runs, 598 steps. [capture: mem
   --draws, case thief, capture.md 10.14.4]
   Not settled: on a turn's first step some squares holding a thing cost
   4; one more case in which the step choice gives up. [unknown]
6. **Engine:** 6.2 to 6.5 as written. Before the goal, a smartness roll
   decides whether a creature acts smart (magic 6.1). On the C64 that roll
   is taken at the top of the turn by friends, enemies and every fighter
   (4.2; a fighter thief too, [capture] 10.14.4),
   whether or not it has anything to cast; the engine rolls it only when
   it has. A fighter friend or enemy whose first check leads to nothing
   takes a second (helping an ally), right after: the "one roll more" of
   an unarmed fighter enemy (capture.md 10.14.6). [research: combat first
   pass; capture: armed, every run] A peaceful (slinker) friend takes one,
   a brave aggressive enemy two, on every Speed 0 turn (2.7). [research:
   combat fourth pass; capture: still-hurt, still-enemy, capture.md 10.24]
   **Engine:** not taken (helping allies comes with a later pass).

## 7. Reaching a goal

1. **A foe.** A fighter loves to attack certain types and may attack others
   it meets on the way. [manual] On the C64 a fighter that owns a melee
   weapon attacks by stepping into the square of the individual it
   strikes: the squares of individuals count in its step choice (6.5) at a
   cost by class, or are closed ([combat](combat.md) 2). [research:
   combat first pass, 2026-11-13] An aggressive, brave enemy of ours with
   no weapon never attacked the player beside it: it kept stepping round
   it (6.5) in 32 of 32 runs. [capture: fight, capture.md 10.14.6] Holding
   a melee weapon of ours, it attacked in every run, every step and blow
   as combat 2 and 3 say. [capture: armed, 10.14.7, 10.14.9] The one roll
   more its turn takes is the second smartness check (6.6).
   **Engine:** combat 2.
2. **Anyone else** (a slinker friend beside a player, a slinker enemy
   beside a fellow enemy): the creature keeps stepping round it until its
   bar runs out (6.5). [research, capture]
3. **A friend next to the player it sought** drops a magic item if it
   carries one. [manual] On the C64 a peaceful friend of ours (Strength
   20, a treasure of ours and 100 gold, no magic item) dropped the
   treasure on its turn beside the player and kept its gold, in 16 of 16
   runs; where it landed took rolls (as 8.1). [capture: burden2,
   capture.md 10.14.5] The gift is the same action as the overburdened
   drop, so its square is 8.1.1's choice, made **around the giver**, not
   the player; standing beside the player, the giver can put it on the
   player's square. [research, unconfirmed: combat fourth pass, traced; no
   gift run of ours checked it] **Engine:** its lowest-numbered magic item,
   else its lowest-numbered treasure (gold never), onto the square 8.1.1
   chooses around the giver (following the unconfirmed research). A drop
   spell then acts on whoever stands on that square, else on the friend
   itself ([magic](magic.md) 2.3). [unknown]
4. **Foes**, for readying (combat 6.2): [manual] **Engine:**
   - an enemy's foes are players and friends;
   - a friend's foes are enemies, aggressive neutrals and aggressive thieves;
   - neutrals and thieves treat every other individual as a foe (a fighter
     neutral is not averse to attacking anybody; thieves show no loyalty to
     other thieves). [manual]
5. **Gold and treasure.** A creature picks up gold or a treasure as
   players do ([inventory](inventory.md) 2): gold adds the pile to the
   creature's wealth (up to 25,000, the rest stays), a treasure goes into
   its possessions within the carry limits. [manual] On the C64 it takes
   the thing from the square beside it, the step it picked (6.5) not
   taken, and its turn ends; the program shows "(name) takes (thing)" or
   "(name) finds gold worth N and now has M" and waits for the button.
   [capture: thief, 64 runs] (The amounts there, 512 and 1,361, came
   from our disks lacking a pile's size record: inventory 2.5.)
   **Engine:** takes it from beside, and the turn ends.
6. **An exit**: the creature takes the door (5.4).

## 8. Things creatures do with what they carry

1. **Overburdened creatures** drop a magic item, a treasure or gold.
   [manual] At the start, 3 master creatures and 5 residents carry more than
   their capacity. [data] On the C64 (Strength 1 creatures of ours, Speed
   2, 16 runs each, all alike): one with 300 gold and treasures of weight
   400 (lower number) and 1 dropped **all** its gold on its first turn and
   the weight 400 one on its second; one with 2,000 gold dropped all
   2,000 at once; one with treasures of weight 1 (lower number) and 400
   dropped the weight 1 one first, then the other. So all the gold first,
   then the lowest-numbered treasure, whatever it weighs; one drop a
   turn, and a turn with a drop had no steps (they walked again once
   under capacity). [capture: burden, burden2, capture.md 10.14.5]
   1. **The square**, for gold and a treasure alike: tries of two rolls
      each, a roll of 0 to 2 less 1 (an offset of -1, 0 or +1), then a roll
      of 0 or 1 for the axis (0 across: -1 west, +1 east; 1 down: -1 north,
      +1 south). The try's square is the dropper's own square moved by the
      offset. An offset of 0 lands the thing at once on the **dropper's own
      square**, with no test. Otherwise the try fails on a square of the
      room's outer ring (the walls: first and last row and column) or one
      holding an obstacle, and another pair is drawn, with no limit (a
      third of all tries are offset 0, so it ends). Any other square takes
      it: one holding an individual (the player's, say) or another thing.
      So each square beside has 1 chance in 6 a try and the own square 1
      in 3. The draws of a drop turn: the healing die, the smartness checks
      (6.6), the pairs, then nothing more. [research: combat fourth pass,
      2026-11-14, `docs/research/combat.md`, run] Every draw and landing
      square, read from the room's list of things: the obstacle west of
      the dropper failing 50 of 306 tries; a corner square's two wall
      neighbours failing 137 of 393, up to 8 in one drop; 20 drops onto
      the player's square beside it; things of ours on three neighbouring
      squares, 125 drops landing on a square already holding a treasure;
      the gold of `burden` on its pair's square, not always the dropper's
      own (correcting 10.14.5's reading, which had seen only the screen).
      [capture: drop-wall, drop-corner, burden, drop-thing 256 runs each,
      drop-beside 128, capture.md 10.24] The program refuses one more kind
      of thing besides the obstacle, which the research did not name:
      [unknown]. **Engine:** the thing lands on top of the square's stack;
      the ring, an obstacle and a custom obstacle fail a try (the second
      kind taken to be the custom obstacle: a guess); every other square
      takes it. In a room whose inside is one square it goes there with no
      draw [research, unconfirmed: traced]; the engine does so, taking
      that square without drawing. On the world map there is no
      square: the thing is lost (inventory 5.3) [research, unconfirmed:
      traced].
   2. **When it drops.** Traced only: a creature drops when the weight
      slowdown (inventory 3.3) has cut its full moves below its Speed, with
      no foe beside it, and only as its turn's first action; the drop uses
      one move and, with moves left, it then steps as usual; a Speed 0
      creature never counts as overburdened. In all our drop cases the
      slowdown left 0 moves, so the drop was the turn's only action.
      [research, unconfirmed: combat fourth pass] **Engine:** for now, at
      the start of its turn, a creature whose load is above its capacity
      drops all its gold (one pile), else its lowest-numbered treasure or
      magic item, onto 8.1.1's square; one drop a turn, after which its
      moves are idled. This differs from the unconfirmed research (load
      above half the capacity, the moves after it); it changes once a
      capture with moves left after a drop settles it. A treasure that
      disappears when dropped vanishes; things that can't be dropped
      (owner-only) are never chosen. While over capacity it picks nothing
      up. [unknown] Not settled: magic items' place in the order and how
      gold counts toward the load. Which load sets the moves of a drop
      turn, the one before the drop or after, is [unknown]. **Engine:**
      the load before the drop: over capacity gives 0 moves, so the turn
      ends after the drop and its draws are those of 8.1.1 (then nothing
      more), as captured.
2. **Readying.** A creature readies an appropriate weapon: when not next to
   a foe, a missile weapon if it has one, otherwise a melee weapon if it has
   one; with several, the lowest item number. [manual] On the C64: a melee
   weapon only with a foe beside it (or as it steps into one), a missile
   weapon with none beside it, a move for each change ([combat](combat.md)
   2.6, 6.2). [research] **Engine:** combat 6.2, after the top-of-turn
   drift and die; a change emits `CreatureReadied`.
3. A safe fighter with a readied missile weapon fires at a foe goal.
   [manual] On the C64: [combat](combat.md) 8.1, Speed 0 included (2.7).
4. Casting and using magic items: [magic](magic.md) section 6.

## 9. Corpus check

The corpus test (`crates/acs-engine/tests/corpus.rs`) puts a player in each
of the 996 live rooms that hold residents or a random creature and plays a
dozen turns of waiting and wandering. With the region-visit scheduler
(2.2), a room door into another room ending the turn (movement 2.3) and
the healing and reversal rules of magic 5 (November 2026), the
creatures' own goals and steps of section 6 and the falling-away roll as
captured (2.4, November 2026), and the take from beside and the
overburdened drop as captured (7.5, 8.1), gold piles' sizes (inventory
2.5), arrivals on a held destination (5.4), melee and firing
([combat](combat.md), 2026-11-13, with the third combat pass's halving
of moves beside a foe, 2.6), creatures take 5,646 turns, make 7,116
steps, go through 115 doors, ready 591 weapons, pick up gold 18 times
and treasure 9 times, and make 40 drops (overburdened, and friends'
gifts); 1,697 attacks strike 6,013 blows (1,390 doing damage, 1,213
counter-blows), 1,054 firings loose 3,384 missiles (1,317 hits, 1,089
dodges, 1,473 stopped by a wall or obstacle), and 225 weapons or armor
break. Fewer turns than before combat because the touring player (Life
Force 11) now dies in 589 of the 996 rooms. No
creature leaves its region, stands on a wall after moving or breaks a
carry limit; none shares a square with another individual except on
arriving through a door (5.4). The random walks with
creatures replay exactly.

## 10. Left for later

- **Combat:** melee and missile fire are in [combat](combat.md).
- **Mimics** copy the opponent they meet: combat.

## Captures needed

1. Turn order: captured and researched (2.2, 2.3), players in different
   places and two players in different rooms of one region included.
   Deaths, the restore point and declining to add a player captured
   (2.2.5 to 2.2.7). Still open: a SAVE GAME in the same round as a
   region change (2.2.7).
2. Random creatures: appearing and falling away captured and researched
   (3, 2.4). Still open: two random creatures left at once.
3. The adjacency slowdown: captured (2.6: halved, 4 squares, aggressive
   and not a friend). Still open: mid-turn, odd Speeds, and a turn that
   starts with moves carried through a map door (2.2.4) beside an enemy.
4. Goal choice: researched and captured for slinker friends and neutrals
   (6.2-6.4), and for thieves and a slinker enemy going for gold and
   treasure (6.5, 7.5), and for a fighter enemy going for the player
   (combat 2). Still open: fighters' goals among other individuals.
5. Paths: researched and captured (6.5: 4 squares, scored with a roll).
   Still open: the first step's cost of some things.
6. A creature following through a door: captured (5.4: the step ends its
   turn; it lands on the destination square, sharing it with a player or a
   creature standing there). Seen in the same runs:
   creatures' paths and goals varied from run to run (a friend sometimes
   took the portal with the player still in the room), so goal choice
   takes a roll (items 4 and 5; the random-rolls brief, item 12).
7. The overburdened drop: captured (8.1: all the gold, then the
   lowest-numbered treasure, one a turn) and the square (8.1.1, capture.md
   10.24). Still open: when a creature counts as overburdened and a drop
   with moves left (8.1.2), the second kind of thing that refuses a drop,
   a friend's gift placed around the giver (7.3), magic items, gold's
   weight; which load sets a drop turn's moves (8.1.2, the engine takes
   the load before the drop).
8. A Speed 0 creature's turn (2.7): captured. Still open: a Speed 0
   fighter with its goal beside it, beside an exit or a thing it takes;
   whether a creature of Speed above 0 whose readying spends its last
   move still takes its shot, step choice or idle move (2.7, the engine
   says yes); whether a non-goal square holding an individual it would
   strike counts as ordinary for the cancelled step (2.7, the engine says
   no: only an empty square does).
