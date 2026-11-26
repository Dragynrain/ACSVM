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
      living player on the open map, then the encounter areas in use, by
      their lowest-numbered player, each its players in number order and
      then its creatures ([encounters](encounters.md) 6.1). In a room
      region: the rooms that hold a living player (at most
      4, in player order), and in each, its players in number (slot) order (a newcomer in a reused slot goes by its slot: [characters](characters.md) 6.1), then
      the region's creatures standing there in slot order; nothing
      numbered between the players and the region's creatures (5 to 15)
      is ever on a room's list. Turns go in list order; the dead are
      skipped and nobody is added during the round. So a creature takes
      one turn a round, after the players of its room, and a room counts
      only if a player stands in it when the round begins; a creature
      already on the list takes its turn even if the player has left.
   4. **A player who goes through a door into another room ends its turn
      there**, whatever moves it has left: a map door into a room, a room
      door (one-way or two-way) into another room of the same region, or
      a room door into a room of another region. The door's step is spent
      and the round counter goes up as usual at the round's end. Its next
      turn starts in the new room with the moves it had left after the
      door's step, or its full Speed if it had none left; those carried
      moves are not halved beside an enemy (2.6). In a new region it is
      on the list of the first round of that region's next visit, by its
      player number like any other player there; no round is missed,
      whichever round of the visit it came in and whichever player came
      in first. If it was the last living player in the old region, that
      visit ends at the round's end and, when the new region is the next
      marked one, its first round begins with this player, so the two
      turns run back to back. If the new region is not visited again in
      the running cycle (2.2.1), the player waits for the next cycle and
      keeps its carried moves meanwhile. In the same region it simply
      takes its next turn in its new room's place on the next round's
      list (2.2.3). The idle healing roll of a carried turn is given in
      [magic](magic.md) 5.3.1. **A creature** that takes a door carries
      its moves the same way: its next turn starts with the moves it had
      left after the door's step, or its full Speed if none (5.4).
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
      is the latest of the game's start (the add-a-player screen's first
      player in, counter 0) and four saves:
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
         it answers "I will save the game as soon as possible" and offers
         F1 for a return to the editor after the save. **It does not end
         the turn**: the prompt comes back with the moves the turn had
         left, the player may go on walking, and moves left unused run
         out as ordinary idle moves, each with its healing roll
         ([magic](magic.md) 5.3). The save is made with the round's other
         requests (2.2.5), so it holds the state at that round's end with
         the counter already gone up: chosen in round 24, it restores to
         the start of round 25. Chosen in the first round of a visit that
         began with a region change, it is the later save of the two, and
         the button brings back its round end, not the visit's start.
      3. **A Retire with Save**: the adventure block it writes at the
         round's end ([characters](characters.md) 7.3, 7.7) is a save
         point like SAVE GAME's. The button brings back that round's end:
         the counter gone up, the retired slot's first byte 0 (the
         character still out of play), every other record as it stood
         then. A Retire with Destroy on the world map writes nothing
         (characters 7.4).
      4. **An Add Someone from a roster answered in play**: when a
         round's end brings in a player chosen from a roster (this
         adventure's own list or another adventure's disk,
         [characters](characters.md) 3.1, 3.2), the program shows
         "saving game" and "the adventure has been saved" after the
         choice, and that write is a save point: it holds the newcomer on
         its starting square and the counter already gone up, so an add
         asked in round 13 restores to the start of round 14 with the
         newcomer there. **The add of a newly created player saves
         nothing**: no save message follows its name, and a later death's
         button goes back past the add to the save before it (the visit's
         start, or the game's start), the newcomer's slot empty. A
         **refused add** (no free slot, characters 6.2) saves nothing
         either; two requests with no free slot give one refusal. **No
         add ends the visit**: the visit goes on to its end by 2.2.2 and
         the cycle runs on, the newcomer's region visited in its turn (a
         newcomer on the map with a room region being visited waits for
         the map's visit). This is Add Someone from the menu only: the
         player added on the add-a-player screen at the game's start is
         in the game's start save already.
         **With the round's other saves.** With SAVE GAME in the same
         round, asked before or after the add: the add-a-player screen
         comes first, then one save, holding the newcomer, created or
         from the roster alike. With a Retire with Save in the same
         round: the retire's offer and its save come first, then the
         add-a-player screen (the retired character listed there). A
         created newcomer is not in that save; a newcomer from the roster
         is, by its own save, which comes after the retire's and is the
         later one. [capture: mem, rr32 and rr34, capture.md 10.36 item 3
         and 10.38 items 1 and 2; ours, no research pass, 2026-11-15; the
         runs are listed after 2.2.7 below]
         **Engine:** a newcomer given as a setup (`AddPlayer`,
         `Newcomer::Join`, the front end's roster or another adventure's
         player) stands for a roster add and saves; a rolled one
         (`CreatePlayer`, `Newcomer::Create`) does not. At a round's end,
         in the order of 2.2.5: a Retire with Save saves (its `Saved`
         event, before the add); then the add (a refusal, `AddRefused`
         once, saves nothing); then one save, with one `Saved` event, if
         SAVE GAME was asked or a `Join` newcomer came in, holding the
         newcomer. The visit goes on. A round with all three (a Retire
         with Save, an add and SAVE GAME) was not run; the engine makes
         the two saves above. On the add-a-player screen with nobody in
         play the same holds: a `Join` saves when it brings the newcomer
         in (characters 3.2 step 5), a `Create` does not.
      **A save in the round of a death.** The death message comes before
      the round's requests (2.2.5). Its button clears them (2.2.6), so it
      restores the save from before that round, never a SAVE GAME (or, by
      2.2.5, a Retire with Save or a roster add, 2.2.7.4) asked during it.
      Answered with F1, the
      requests go on: a SAVE GAME of that round is written then ("saving
      game", "the adventure has been saved"), an add of that round brings
      its newcomer in (and saves if it is from a roster, 2.2.7.4), and
      with nobody alive the add-a-player screen follows, so the state
      saved has nobody alive (the **all-dead save**). A player added then
      starts on the world map. With the last player dead in a room
      region, its first visit is a region change and its save is the
      later one: a death then goes back to that visit or a later save,
      never to the all-dead save. **With the last player dead on the
      map**, a created newcomer starting there makes no region change and
      no save, so the all-dead save is still the latest: when the
      newcomer is killed in turn, the button brings back that round's
      end (its counter and state, nobody alive), clears the records and
      shows the add-a-player screen again, the newcomer lost. [capture:
      mem, rr34 `i3e-button`, capture.md 10.38 item 3; ours, no research
      pass, 2026-11-15]
      **A save in a visit's last round followed by a region change** gives
      the next visit's first round on the counter either way, with nothing
      played between the two, so it makes no difference which one counts.
      **Engine:** the latest of the game's start and the four saves
      above (the add's as 2.2.7.4 says). SAVE GAME is a
      command logged like any other. It records the request and changes
      nothing else: the turn goes on with the moves it had, and the
      player's next commands walk them or let them run out (`Wait`,
      `Idle`), one healing roll per idle move. The save is taken at the
      round's end (a `Saved` event); asked twice in one round, it saves
      once. A Retire with Save, taken at the round's end (characters 7.3),
      makes a save point too, with a `Saved` event after `Retired`. F1's
      offer to return to the editor is not made: the engine has no editor,
      and the save is the same either way. With SAVE GAME and a region
      change in one round there are two `Saved` events; the visit start
      that follows the round's end is the later one and is the save point
      (the same state either way). A save in the round the last player
      died, or by the last player's Retire with Save, is made as 2.2.5
      reads: the state with nobody in play becomes the save point, and
      the newcomer's first visit saves over it whenever it starts in
      another region; when it starts in the same region (the map), the
      all-dead save stays the point unless the newcomer came from a
      roster (2.2.7.4), and restoring it brings back nobody in play: the
      engine clears the records, reports `AllPlayersDead` again and waits
      for a new player. The button at that round's death message restores
      the save before it, since the button clears the round's requests
      (2.2.6).
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
   (a lone player seems to go on with the rest of its turn).
   **Room doors and region to region (2.2.4)** [capture: mem, rr25,
   capture.md 10.29; ours, no research pass, 2026-11-14; the case the
   turn-order second pass traced only]. A Speed 12 player with 8 moves on
   its first room turn walked 3 squares east, the third onto a portal
   into a second room, with 5 left: its moves and the current individual
   went to 0, the counter went from 8 to 9, and its next turn began in
   the second room with 5 moves (the turn after, 12); onto the portal
   with its last move, the next turn began with 12. A one-way and a
   two-way portal did the same, nearly to the instruction. A one-way
   portal into a room of a second region did the same again, the region
   going from 1 to 2 and the first turn there in round 9, with 5 moves or
   12. With a second player U in region 1 since round 13, T went through
   such a portal with 5 moves left at once: U alone in region 1 for
   rounds 13 to 32, T in region 2 for rounds 33 to 42 (its first turn
   there with the 5 moves), U again from 43. **Room lists never hold 5 to 15** [research,
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
   **SAVE GAME, Retire with Save and the death round (2.2.7)** [capture:
   mem, rr24, capture.md 10.28; ours, no research pass, 2026-11-14]. One
   run a case, every restore of a run the same. After SAVE GAME and its
   two buttons the prompt came back with 4 of 6 moves left (two states);
   pushed, T walked 4 squares; left alone, the 4 went one by one as idle
   moves. With `--draws`, one draw per idle move with or without SAVE
   GAME (32 and 40 moves; Life Force +1 at 4 and at 7 of them), and one
   each at full Life Force too. 10.25's "the button ends the turn" was
   these moves running out. SAVE GAME in round 46, T (alone) killed in
   round 46: the button brought back counter 14, the room visit's start,
   in all 15 restores (`c1-button`). F1 instead: "saving game", "the
   adventure has been saved", then the add-a-player screen (without the
   save F1 goes straight there); a newcomer created, walked to the room
   and killed there: back to counter 69, its room visit's start, in all
   35 restores (`c1n-button`). Retire with Save by T in round 24, U killed
   in round 26: back to counter 25, T still retired, U and a creature's
   class as at round 24's end, not the visit start of round 22
   (`c2-button`, 1 run). SAVE GAME at round 60, the first of a visit
   that began with a region change, a kill in round 62: back to counter
   61 and the gold set just before the save, not the visit's start
   (`c3-button2`, 1 run).
   **A roster add in play as a save point (2.2.7.4)** [capture: mem, rr32,
   capture.md 10.36 item 3; ours, no research pass, 2026-11-15]. U was
   added with ADD SOMEONE on T's first turn, in round 13 (`two.vsf`, as
   10.10); T then walked into encounter area 1 with U left on the map,
   both in region 0 all the while, and no SAVE GAME, retirement or region
   change followed. The area's copy killed T in round 22, and the button
   brought back round 14, T and U both on the map's start square 12,12
   (place 0), not the game's start. The same case with T alone (no add)
   went back to round 0, the game's start. 1 run each. That add took U
   from the roster and showed the save messages. **Created and roster
   adds, the round's other saves, the all-dead save (2.2.7, 2.2.7.4)**
   [capture: mem, rr34, capture.md 10.38 items 1 to 3; ours, no research
   pass, 2026-11-15; one run a case, gold poked at each step to mark what
   came back]. A created player added in round 60, the first round of a
   room region's visit, with U on the map: the room region went on for
   rounds 61 to 69, then the map from 70 with U and the newcomer; T
   killed in round 63, the button brought back counter 60, the visit's
   start, the newcomer's slot empty (`i1c-button`). A created player
   added on the map in round 13, no save message: a death in round 21
   went back to round 0 (`i1d-button`). SAVE GAME and ADD SOMEONE in one
   turn, in both orders: the add screen, then one save; a death later
   went back to round 14 with the newcomer, created (`i2s-n`) or from the
   roster (`i2s-u`), alive. RETIRE then ADD SOMEONE in one turn: the
   retire's offer and save, then the add screen; with a created newcomer
   the button went back to counter 21 without it (`i2r-n2-button`), with
   T taken back from the roster to counter 21 with T in play
   (`i2r-t-button`). With four living, one or two ADD SOMEONE: one
   refusal at the round's end, and a later restore went back to round 0
   (`i2fb1-button`, `i2fb2-button`). T killed alone on the map after a
   SAVE GAME, F1: the save, the add screen; a created newcomer killed on
   the map in round 22: the button brought back counter 15, gold 77,
   nobody alive, then the add screen again (`i3e-button`). The earlier
   reading, that every add in play saves (from 10.36 alone), was too
   broad.
   **Engine:** the rules 2.2.1 to 2.2.7. A dead player is skipped on
   every list and left out of the marks; the round finishes after a
   death. The death message, its two answers and the add-a-player
   screen are the front end's: F1 is simply going on; the button is
   loading the last save point of 2.2.7 (magic 7.4, `docs/saves.md`
   1.4) and then the logged command `Restore`, which brings back the
   state as it was at the save (before the turn that followed it),
   clears the marks and requests, ends the visit and marks a new cycle
   from the restored places, lowest region first (2.2.6; capture.md
   10.25 `b-split-button`: the map first). The scheduler's region is
   then none, so the new cycle's first visit saves again (2.2.7.1). With
   nobody alive, in play or in the save, the engine reports it (`AllPlayersDead`),
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
   capture.md 10.14.3] Not captured: a player walking into a room
   mid-round (by the rule above it protects the room's random creatures).
   In the combat cases
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
   gone. [capture: fall, 256 runs]
   **Two random creatures at once** [capture: mem --draws, rr37
   `f2/sweep`, `scripts/fall2check.py`, capture.md 10.41 item 1; ours, no
   research pass, 2026-11-16]. The `fall` snapshot with slot 0 marked as
   well as slot 7, so two random creatures stood in a room with no player,
   swept over 256 random states for five round ends each. At every round
   end: the slot roll, and when that slot was in use **one percentile for
   each marked slot, lowest slot first** (0, then 7), each under 20
   removing that slot's creature if it was still alive. A slot whose
   creature had already gone **still drew its percentile** at every later
   round end where the slot roll hit. Every run's draws fitted (1,280
   round ends, 643 with the slot in use); slot 0 went in 115 runs, slot 7
   in 97, each at the round end predicted, and **both went at the same
   round end in 14 runs**, as predicted. Read the other ways, the slots
   high to low got 155 runs wrong, one percentile for both 255, a
   percentile only for a slot whose creature is still alive 112.
   So the rule, in full:
   1. At the end of each round of a room region's visit, a roll of 0 to
      15 picks a slot. If no creature is in that slot, nothing more.
   2. Otherwise each slot carrying the random mark, from slot 0 up, draws
      a percentile, whether or not a creature is still in it.
   3. Under 20, the slot's creature is removed if it is alive and no
      living player stands in its room. Several can go at one round end.
   4. The mark is set on a slot when a random room creature is put there
      (3.3). Removal keeps it (above). What clears it, and whether a
      death keeps it too, is [unknown]: none of these runs put a new
      creature into a marked empty slot. (Reachable, not yet run: a
      random creature killed in its room, then the round ends' draws read
      as above; capture.md 10.44.)
   **Engine:** so. The engine keeps the mark per slot, apart from the
   creature: a removal, and (its choice) a banishment and a death, leave
   it set, so a
   marked empty slot draws its percentile at each round end where the
   slot roll hits, and a draw under 20 there does nothing. Engine's choice
   for the unseen part: putting a creature into a slot sets the mark if
   it is a random room creature (3.3) or a map encounter copy
   ([encounters](encounters.md)), and clears it for anything else (a
   summons, [magic](magic.md) 3.2; an added resident); placed residents
   start unmarked. A creature removed this way takes what it carries
   (`CreatureRemoved`, reason `left_behind`); what becomes of its things
   in the original is not known. (Superseded: the engine's percentile for
   live random creatures only, which ran one roll short after a removal.)
   The engine takes the room's random-creature roll on entering (3.1),
   not each round.
5. **The movement bar.** A creature's turn has a bar like a player's: its
   Speed after the weight slowdown ([inventory](inventory.md) 3.3), using
   the creature's own Strength, Constitution, Life Force and load. On the
   C64 it shows on the left as a player's does, 8 pixels a point of Speed
   (24, 40 and 32 pixels for Speeds 3, 5 and 4, carrying nothing), and
   drains as the creature walks. [capture: turns] The slowdown is a
   player's: loads of 150 and 101 of a capacity of 200 gave Speed 6
   creatures 3 and 5 moves (8.1.2) [capture: rr28 `burden2-sw`,
   capture.md 10.32], and a Speed 10 creature at capacity 700 carrying
   600 had 2 [capture: rr39 `cw-m`, capture.md 10.43 item 9]. **Gold in
   the load** counts as a player's does, per piece times Gold's weight
   ([inventory](inventory.md) 3.1): four peaceful friends of Speed 10,
   Strength 10 and Constitution 20 began their first turn with 6 moves
   carrying 700 (capacity 1,000), 10 carrying 500, 4 carrying 700 at Life
   Force 18 (capacity 900), and 6 carrying 700 gold of weight 1 and
   nothing else, in 8 of 8 runs each; each began its next turn with 10,
   the three over half their capacity having dropped (8.1.2), the gold
   carrier its gold as one pile. [capture: mem, rr40 `wt-m`, `wt2-m`,
   capture.md 10.44 item 3; ours, no research pass, 2026-11-16] A player's
   bar is set after the top-of-turn healing die, from the Life Force it
   leaves; **a creature's is set before the die**, from the Life Force
   it had: in 24 turn starts of 112 the die healed the creature (14 to
   15 of Constitution 20, which would have raised its moves from 2 to 5),
   and each began with 2, the moves set before the heal
   ([inventory](inventory.md) 3.4; [capture: rr35, capture.md 10.39 item
   4; mem, rr39 `cw-m`, `cw-m2`, capture.md 10.43 item 9; ours, no
   research pass, 2026-11-16]). **Engine:** for a creature, catch-up,
   drift, the bar, then the die; for a player, drift, the die, then the
   bar. Where the catch-up falls against the bar is [unknown] (the
   engine keeps it first). (Superseded: the engine's earlier choice of
   the player's order for creatures.)
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
   halving, and both after the healing die for a player, before it for a
   creature (inventory 3.4, 2.5). [research: combat third pass, every turn start of about 1,500
   in its replays; the slowdown's order traced] An enemy of Speed 3 had 2
   moves on every turn it began beside the player, 3 otherwise, and the
   player 3 of 6 beside it. [capture: every pass 2 and 3 combat run,
   capture.md 10.16] **Engine:** so, for players (a creature on the 4
   squares beside that is Aggressive and not a Friend) and creatures (a
   foe as combat 8.1); Speed 0 stays 0. The `TurnStarted` event says so
   for a player (`hindered`); a creature's `CreatureTurn` gives the moves.
   **Odd Speeds and mid-turn** [capture: mem, rr37 `hp-s6` to `hp-s1`,
   `mc-m`, `cr-m`, `scripts/adjcheck.py`, capture.md 10.41 item 2; ours,
   no research pass, 2026-11-16]. The halving rounds up at every Speed,
   for players and creatures alike, and is taken **only at the turn's
   start**: an individual that comes beside a foe during its turn, by its
   own step or the foe's, loses only the step it spends and keeps the
   rest of its bar. A player of Speed 6, 7, 5, 3 and 1 stepped beside an
   aggressive enemy: right after the step it had 5, 6, 4, 2 and 0 moves
   (one gone, nothing halved), and each of its next two turns, still
   beside the enemy, began with 3, 4, 3, 2 and 1. An aggressive enemy of
   Speed 7 walking up to the player began its turns beside the player
   with 4 moves (7 otherwise), the player its turns beside it with 3 of 6;
   arriving beside the player with 0 to 6 moves left, it went on spending
   them one a step (no drop of more than one inside a turn). Peaceful
   enemies of Speed 1, 3, 5 and 7 beside a player of Speed 2 began their
   turns with 1, 2, 3 and 4, the same creatures elsewhere with their full
   Speed; rounding down misses 33 of the turn starts. **Engine:** so: the
   test and the halving at the turn's start only, never during a turn.
   **A turn that starts with carried moves** (after a door into another
   room, 2.2.4) is not halved, whoever stands beside the player; a turn
   after such a door with no moves carried starts at the full bar halved,
   like any other hindered turn. A Speed 12 player came through the map
   door with 8 moves left (7 after one idle move on the map) onto an
   entry square with an aggressive enemy of Speed 0 east of it, and with
   8 with the enemy diagonally next to it: its first room turn began with
   8, 7 and 8. Through the door with none left, the turn began with 6;
   every later turn beside the enemy began with 6. [capture: mem, rr25,
   cases adj, diag, capture.md 10.29; ours, no research pass,
   2026-11-14] **Engine:** so; `TurnStarted` reports `hindered` false
   for a turn that starts with carried moves. **A creature's carried turn
   is not halved either**: a friend of Speed 6 that followed the player
   through a portal onto a square beside an aggressive enemy of Speed 0
   began its next turn there with its carried moves, 3 (5 runs) or 1 (1
   run), where the same friend beside the same kind of enemy with nothing
   carried began with 3 of 6 (2 of 2). [capture: mem, rr40 `df`, `fh`,
   capture.md 10.44 item 4; ours, no research pass, 2026-11-16]
   **Engine:** so, as for a player (5.4).
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
      and the square picked is an ordinary one (cost 2 in 6.5, and empty:
      see below), so it never walks;
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
   a goal beside it once a turn. [research: traced] An aggressive brave
   enemy of Speed 0 holding our blade, the player beside it: an attack on
   every one of 64 turns, its square never changing. [capture: rr28
   `still-melee-sw`, 16 runs, capture.md 10.32; ours, 2026-11-14] Each
   attack is **1 blow** at Dexterity 13 ([combat](combat.md) 3.1 with the
   first term 1 at full moves 0), and 1, 2 and 3 at Dexterity 8, 16 and
   24. [capture: rr28 `still-melee-sw`, rr31 `sm-d8`, `sm-d16`, `sm-d24`,
   capture.md 10.35; ours, 2026-11-15] (Superseded: the "2 blows" first
   read from these runs, a misreading of the blows-left byte.)
   Beside a thing it takes or an exit the step stands too: a peaceful
   thief of Speed 0 with 50 gold on the square beside it took the gold on
   its first turn without moving (16 of 16 runs), and a peaceful neutral
   of Speed 0 beside a portal went through it once in 16 runs of four
   turns each, staying put otherwise (the same creature at Speed 2 went
   through in 2 of 16 and wandered). How often an exit is picked is the
   goal and step choice's (6). [capture: rr33 `s0g`, `s0x-long`,
   `s2x-long`, capture.md 10.37 item 7; ours, no research pass,
   2026-11-15] Its one step choice is a turn's first, where an exit costs
   4 (6.5), so the exit wins only on a lucky set of rolls: the exit's 0,
   south's and north's 2, west's at least 1 (the exit then scoring 20
   against 24, 24 and at least 20, east looked at before west): 1 turn in
   63 seen; when it does win, the step onto
   it stands and the creature goes through. [capture: rr33 `s0x-long`,
   rr35 `ta-pn0`, `ta-pf0`, `ta-an0`, capture.md 10.39 item 1; ours, no
   research pass, 2026-11-16] **A Speed 0 fighter** beside an exit or
   gold, owning a melee weapon or not, does the same: aggressive neutrals
   of Speed 0 in the corners, the one other open square beside each a
   portal or 50 gold, two holding and owning our blade and two unarmed,
   the player (their goal) across the room, went through the portal or
   took the gold from beside on their first turn in 64 of 64 runs. With
   one open square there is one candidate, and the step onto an exit or a
   thing it takes is never cancelled. [capture: mem, rr37 `w5-m2`,
   capture.md 10.41 item 5; ours, no research pass, 2026-11-16]
   **Engine:** 2.7 as
   written, the smartness checks as 6.6's
   engine choice says, with these two readings settled by capture:
   - **Readying that spends the last move ends the turn**, above Speed 0.
     The one action taken whatever the moves left is a Speed 0
     creature's only. A creature of Speed above 0 that readies a weapon
     (8.2) with its last move does nothing more that turn: no shot, no
     step choice and its rolls, no idle move and its healing roll. With a
     move left after readying it goes on as usual (a shot, say). An
     aggressive brave enemy of Speed 1 owning a bow it was not holding,
     the player 5 squares off, readied the bow with its one move and then
     drew only the turn's closing rolls, in 32 of 32 runs, and again in 32
     of 32 with the line of fire blocked and its Life Force below
     Constitution (no step rolls, no step, no heal but the top-of-turn
     die's); it fired or stepped on each later turn. Of Speed 2, it
     readied with its first move and fired with its second; of Speed 1
     already holding the bow, it fired on its first turn. [capture: mem
     `--draws`, rr29 `sw-ready1`, `sw-ready1b`, `sw-ready1w`, `sw-ready2`,
     `sw-held1`, 160 runs, capture.md 10.33; ours, no research pass,
     2026-11-14] **Engine:** so: a creature of Speed above 0 left with 0
     moves after readying ends its turn there. (Superseded: the engine's
     earlier choice, that its shot or step choice, else an idle move,
     still came.) **A creature slowed to 0 moves by its load**
     ([movement](movement.md) 2.7) counts as Speed above 0 here: it
     readies and stops. One of Speed 2, Strength 1, carrying 400 and
     owning our blade, a peaceful neutral kept from dropping by the
     player beside it (8.1.2), readied the blade and then drew only the turn's
     two closing rolls: no step rolls, no attack, its load and square
     unchanged, in 16 of 16 runs. [capture: rr33 `rmb`, capture.md 10.37
     item 4; ours, no research pass, 2026-11-15] **Engine:** so: the
     stored Speed decides, and readying ends its turn. (A drop that
     leaves it 0 moves ends the turn before any readying: 8.1.2.)
   - **What counts as ordinary** for the cancelled step (2.7.3): only an
     empty square, nothing on it and no individual. A square that is not
     the goal's but holds an individual the creature would strike is not
     ordinary, even where 6.5's table gives it cost 2, so that step is not
     cancelled: the creature strikes there. An aggressive brave enemy of
     Speed 0 holding our blade, a peaceful friend of Speed 0 beside it
     and the player (its goal) 5 squares off, attacked the friend in 22 of
     64 first turns and in 35 of 96 turns of 32 runs played on, did
     nothing in the rest, and never stepped. [capture: mem `--draws`, rr29
     `sw-ord`, `sw-ord2`, capture.md 10.33; ours, no research pass,
     2026-11-14] **Engine:** so.

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
   room's inside. [capture] A sweep of 32 walks into an empty room (chance
   15) with the random state set to 32 spread values beforehand: the
   creature appeared in 21 (75% expected), on 19 different inside squares
   from one side of the room to the other, two of them twice, never the
   player's square. [capture: rr36 `rc-sw`, capture.md 10.40; ours, no
   research pass, 2026-11-16] That fits the tries above; which square
   each random state gives was not checked draw by draw, so the order of
   the column and row rolls is [research, unconfirmed]. **Engine:** the
   same: up to 5 tries of a random inside column, then row, each rejected
   when an individual or a thing is on the square.
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
   never attack or cast. [manual] On the C64 it is worked out afresh from
   the Life Force of the moment, at each of its turns, not from the Life
   Force it had when placed or when the visit began: an aggressive
   cautious enemy holding a melee weapon of ours struck the player in 4
   of 4 runs built unwounded and in none with its Life Force set to half
   its Constitution after the visit had begun (it walked off), and the
   other way round when built wounded and healed by a poke (0 of 4, then
   4 of 4); built brave, it struck in 4 of 4. [capture: mem, rr40
   `fsw-m`, `fsw-set`, `fsw2-m`, `fsw2-set`, `fsb-m`, 16 of 16 runs,
   capture.md 10.44 item 2; ours, no research pass, 2026-11-16] The same
   cases unarmed never struck, as an unarmed fighter never does
   ([combat](combat.md) 2.3). **Engine:** at the start of each of its
   turns.
3. Placed residents by type at the start (all 15 adventures, 3,248): 1,985
   fighter enemies, 632 fighter friends, 494 fighter neutrals, 105 fighter
   thieves, 16 slinker friends, 9 slinker thieves, 7 slinker neutrals and no
   slinker enemies. 6 have Speed 0. [data]
4. **Smart or dumb** (Wisdom above 14, below 6, chance in between) matters
   for casting and for a hurt neutral turning enemy: the magic and combat
   rules. Not used here.
5. **Mimics.** A creature with the Mimics choice mirrors the opponents it
   meets. [manual] The choice is bit 7 of record byte 0x09 (format
   notes). [data] On the C64, at the mimic's first turn with the player
   in its room, after its moves for the turn are set (an enemy of Speed
   3 copying a player of Speed 6 took 3 steps that turn and 6 the next
   unhindered one), its record becomes the player's: Constitution with
   its magic defense, Strength, Dexterity, Speed, Wisdom, Power, Size,
   Dodge, Parry, Armor skill, Melee, Missile, the weapon held, the armor
   worn, every possession bit and the picture, and the round of its last
   turn (5.5 of [magic](magic.md)) becomes the player's 0. It keeps its
   own Life Force, square, room, strategy (alignment, aggressive and
   brave) and its Mimics bit. Things it owned that the player doesn't
   are gone, not dropped. Its gold is not the player's: it **drops all
   its gold** on its own square as the turn's first action, a move (2
   steps after it against 3 without gold), and owns none. The copy is
   made from a distance (2 and 6 squares off, before any step), and of
   the player, not of a friend standing beside it. **Once:** a player's
   Strength and Dexterity raised after the copy were not copied at the
   mimic's next two turns. [capture: rr43, 18 runs, capture.md 10.47;
   ours, no research pass, 2026-11-18] Without the choice the same
   creature copies nothing (the control in every case). Not run, the
   engine's choices standing [unknown]: which player is copied when
   several are in the room (**Engine:** the lowest-numbered living one);
   whether "once" is per visit or for good, and where the program keeps
   it (**Engine:** for good, the creature's mimic flag cleared); a
   friendly or peaceful mimic (**Engine:** any mimic copies); whether the
   gold drop draws for its square (**Engine:** its own square, no draw).
   **Engine:** so (`ai.rs`, `mimic_copy`), with an event
   `CreatureMimicked`.

## 5. Where a creature may step

1. Creatures never move onto a space or an obstacle, custom ones included.
   [manual] On the C64 no space is open to a creature, not a plain one
   open to anyone, nor a custom one whose condition it meets: peaceful
   friends of Speed 3 in the four corners, both squares beside each
   holding a space of ours (one invoking a spell, one plain, a custom one
   "when moved onto", a custom one open to owners of a thing the friend
   owned), never left their corners in 64 turns and idled their bars out.
   [capture: mem, rr37 `cs-m`, capture.md 10.41 item 3; ours, no research
   pass, 2026-11-16] **Engine:** so, and nor onto walls, stores, or
   another individual.
   **An obstacle under the goal: the bump after a shared door.** A
   player comes to stand on an obstacle's square by arriving there
   through a door (movement 6.3). A creature that followed it through
   the same door lands on that square too (5.4), and only such a
   creature, in that turn, bumps the obstacle; one that merely stands
   beside the square never does.
   - Seen: a player arrived by a side portal on the square of an obstacle
     that bumps for Life Force -5; a peaceful friend of Speed 6, the
     player its goal, followed it through the portal. Over 256 random
     states its first turn after following, begun on the obstacle's
     square sharing it with the player, came in 231 runs, and **8 of
     those 231 turns bumped** (9 bumps, one turn bumping twice): the
     friend, having stepped off, picked the square again in a later step
     choice of the same turn, its Life Force fell by 5 and it stayed
     where it was. In the **586 other turns** with the friend beside the
     square and a move left, it never bumped. [capture: mem, rr40
     `bump-m`, `scripts/turnlog.py --bump`, capture.md 10.44 item 1; rr37
     `ad-ob-m` run 1, capture.md 10.41 item 3, the same bump; ours, no
     research pass, 2026-11-16]
   - **Engine:** a narrow rule for this case only. When a creature that
     does not strike (combat 2.3) has an individual for its goal, that
     individual stands on a square whose top thing is an obstacle or a
     custom obstacle, and **the creature began this turn on that same
     square** (sharing it after a door, 5.4), the square is open to it in
     every step choice of this turn once it has stepped off, at 6.5's
     cost 4 for a square stood on this turn. On any other turn, and for
     any creature that has not stood there this turn, the square is
     closed, as any individual's square is to a creature that does not
     strike. Picking it is a bump: the creature stays put, the obstacle's
     bump spell, if it has one, goes off acting on the creature, as a
     door's spell does ([magic](magic.md) 6.6; a custom obstacle's only
     with its spell bit set, movement 5.5), an obstacle or custom
     obstacle with the disappearing bit vanishes (movement 5.9), and the
     move is spent, the turn going on.
     Whether cost 4 gives the original's rate (about 3.5% of such turns
     bumping) is for the golden test to check against rr40's count. (The
     custom obstacle and the move spent are the engine's choices, not
     seen. Superseded: the engine's earlier cost 1 for a creature that
     had not stood on the square, which bumped from beside it about 3
     times a run where the original never did.)
   - **A goal's square holding an individual and no obstacle** stays
     closed to a creature that does not strike: not worth a run of its
     own, as the captures of 7.2 already have a slinker stepping round the
     player it seeks (capture.md 10.44).
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
   pass and the spell goes off for them ([magic](magic.md) 6.6). A
   summoning portal: a creature walked through it and a second copy
   appeared [capture: rr38 `sn-dro`, capture.md 10.42 item 1]; the copy
   was seen two rounds later on a square beside the portal, so the square
   it was made on is not pinned (the engine uses the portal's). Other spell
   kinds: [unknown] (reachable, not yet run: a portal of ours with another
   spell, put in the room; capture.md 10.44).
   An encounter area's door item is not a door to creatures: they never
   step onto it ([encounters](encounters.md) 6.4). [capture]
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
   **Moves carried through the door.** The creature's next turn starts
   with the moves it had left after the door's step, as a player's does
   (2.2.4): a friend of Speed 6 that followed the player through a portal
   on its first turn began its next turn with 3 moves in 5 runs and 1 in
   the sixth, its Speed less the steps it had taken up to and onto the
   portal (3, or 5). [capture: mem, rr37 `ad-sp-m`, `ad-ob-m`, `ad-cs-m`,
   6 of 6 runs, capture.md 10.41 item 4; ours, no research pass,
   2026-11-16] **With none left**, its next turn starts at its full bar:
   a friend of Speed 2 whose second step was the portal began its next
   turn with 2 (3 runs of 4 followed), one of Speed 3 that went through
   with 1 left began with 1 (4 of 4); and carried moves beside a foe are
   not halved (2.6). [capture: mem, rr40 `dz`, `d1`, `df`, capture.md
   10.44 item 4; ours, no research pass, 2026-11-16] Not seen [unknown]:
   whether the top of that turn (catch-up, drift, the die) or the weight
   slowdown changes the carried count (reachable, not yet run, capture.md
   10.44).
   **Engine:** taking a door costs the step and ends the creature's turn;
   it arrives on the destination square whoever stands there, sharing it
   until they move. The original keeps the creature out of sight until
   its next turn; the engine places it at once. Its next turn takes the
   top of the turn as usual (catch-up, drift, the die, [magic](magic.md)
   5), then starts with the moves left after the door's step, or its full
   bar (2.5, 2.6) if none; carried moves are not halved beside a foe and
   not cut again by the load, and the turn's full moves for the idle
   healing roll stay those of the door turn, all as for a player (2.2.4,
   2.6, magic 5.3.1). The carried count is kept until that turn, the next
   visit or cycle included.
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
   steps. [capture: mem --draws, capture.md 10.14] So too for a fighter
   choosing between individuals: an aggressive enemy of Speed 3 with no
   weapon, the player and a friend both in reach, went for the lower rank
   plus steps every turn, the player on a tie: a peaceful friend at 7
   against the player's 8 was the goal in 60 of 64 turns (the player in
   the other 4, once it had wandered off), a tie at 7 went to the player
   in 69 of 70 turns, an aggressive friend at 6 against 8 was the goal in
   89 of 91; 6.2's rule fitted 32 of 32 runs in each case, "always the
   player" 3, 32 and 2, "always the friend" 28, 1 and 30. [capture: rr35
   `g-f`, `g-t`, `g-a`, `scripts/goalcheck.py`, capture.md 10.39 item 2;
   ours, no research pass, 2026-11-16] So too for a fighter friend and a
   fighter neutral (aggressive, Speed 3, no weapon) among the player and
   others standing still, 6.3's rows: 6.2's rule fitted every run of 17
   cases, 544 runs and 1,192 turns, with the two candidates' scores 1 or
   2 apart either way or tied. A tie went to the one looked at first, the
   player or else the lower slot, whatever the ranks (a fighter friend
   took an aggressive neutral, rank 3, over an aggressive enemy, 0, at the
   same score in 60 of 64 turns, the other 4 once it had wandered); a
   fighter friend went for any foe before the player, rank 32, and for
   the player only with none in the room (a peaceful neutral and a
   peaceful friend are no candidates); a fighter neutral went for the
   player at rank 1 against an aggressive friend at 0 or an aggressive
   thief at 2 by the score. [capture: rr47 `f1a` to `f3n`, `n4a` to
   `n6t`, `scripts/goalcheck.py`, capture.md 10.50; ours, no research
   pass, 2026-11-20] **Engine:** so.
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
   [capture] A fighter enemy two steps from a portal with no player in
   the room went for a peaceful neutral (84 + 2) or an aggressive thief
   (80 + 3) over the portal (96 + 2) in 32 of 32 runs each, never the
   portal the manual's order would pick. [capture: rr47 `e1`, `e2`,
   capture.md 10.50; ours, no research pass, 2026-11-20] **Engine:** so.
5. **Each step** looks at the 4 squares beside the creature, in the order
   south, north, east, west, and takes a roll of 0 to 2 for each open one
   (5.1, 5.2). It steps to the open square with the lowest **3 x the steps
   from it to the goal + 5 x its cost + 4 x its roll** (a tie to the first
   looked at). With no goal it steers by the square it started the turn
   on, so it wanders about there. The cost: 1 for the goal's own square;
   an exit 4 on the turn's first step (see below), else 1 for a slinker,
   3 for a fighter; gold or a treasure it would
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
   **An exit on the turn's first step costs 4**, for a slinker and a
   fighter alike, whether the exit is its goal or not (so 4 replaces the
   goal's own square's 1 there too). The turn's first step is its first
   step choice: the one scored from the square the turn began on, before
   any step has been taken that turn. Every later step choice of the turn
   scores an exit 1 (slinker) or 3 (fighter) as above. A Speed 0 creature
   makes only that first step choice (2.7), so for it an exit always
   costs 4. Read draw by draw, a creature by a portal fitted this rule in
   every run (Speed 0 and 2; peaceful neutral, peaceful friend and
   aggressive neutral; the player in the room or gone, the goal the exit
   or the player), 352 runs and 730 turns, where the cost 1 or 3 on every
   step fitted 144 of 352 (none of 48 for the Speed 0 peaceful neutral
   going for the exit, which went through in 3 of 188 turns, not every
   turn). Started two squares off, the portal beside it only at its second
   step, its choices there fitted the cost 1 or 3 in 64 of 64 runs (the
   slinker took the portal in 23 of 32), a cost of 4 on every step 43 of
   64. A slinker friend following the player onto a portal,
   never at a first step, fitted the cost 1 in 64 of 64 runs (4 in 24).
   [capture: rr35 `x-pf0`, `x-pf2`, `x-an0`, `ta-pn0`, `ta-pf0`,
   `ta-an0`, `s26-pn2`, `s26-an2` and rr33 `s0x-long`, `x-pn0`,
   `s2x-long`, `scripts/goalcheck.py --exit-first`; rr3 `follow`;
   capture.md 10.39 item 1; ours, no research pass, 2026-11-16]
   **Engine:** so. (Superseded: the engine's cost 1 or 3 on every step,
   with which a Speed 0 slinker beside an exit always went through on its
   first turn.)
   **The first step's 4 is an exit's alone.** Gold or a treasure it would
   take costs its thing rank / 32 on every step, the first included, and 1
   when it is the goal's own square. An aggressive neutral of Speed 0
   (gold rank 96, so cost 3) with 50 gold between it and the player, its
   goal: the gold scored at 3 fitted 16 of 16 runs (32 turns, 14 takes),
   at 4 on the first step 2 of 16. A peaceful thief of Speed 0 whose goal
   was 50 gold beside it (cost 1) took it on its first turn in 16 of 16
   runs; 4 on the first step fitted none. [capture: mem `--draws`, rr37
   `w5-m2`, `g1-m`, `scripts/goalcheck.py --cand gold:..`, capture.md
   10.41 item 6; ours, no research pass, 2026-11-16] **Engine:** so.
   Not settled: one more case in which the step choice gives up. [unknown]
   It needs a research pass: the case is not known, so no capture can aim
   at it (capture.md 10.44).
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
   player's square. [research: combat fourth pass, 2026-11-14, traced]
   A peaceful friend of Strength 20 and Speed 3 with one treasure, beside
   the player, gave it as its turn's first action with the overburdened
   drop's draws (one smartness check, then the pairs around its own
   square): 64 of 64 runs fitted, landing on its own square 24 times, the
   player's square 6 and its other neighbours 34. Starting three squares
   off, it stepped twice to come beside the player and gave with its
   third move in the same turn (38 of 64 runs), the thing landing only around the giver,
   never on the player's other neighbours. [capture: rr33 `g5`, `g7`, 64
   runs each, capture.md 10.37 item 6; ours] **Engine:** its
   lowest-numbered magic item, else its lowest-numbered treasure (gold
   never), onto the square 8.1.1 chooses around the giver, using a move
   (one that disappears when dropped vanishes, as in 8.1.2); a giver that
   arrives beside the player with a move left gives in the same turn, and
   the gift ends its turn, any moves left idling (2.7.4). [unknown: the
   captures fit the gift's own draws, not what follows it] A drop
   spell then acts on whoever stands on that square, else on the friend
   itself ([magic](magic.md) 2.3). [unknown] (Reachable, not yet run: a
   gift with a drop spell; capture.md 10.44.)
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
      of thing besides the obstacle, which the research did not name: it
      is the **custom obstacle**. With a custom obstacle, a portal, a
      custom space and a space on four squares around the dropper, every
      run fitted only with the custom obstacle failing its tries (50 of
      306); the portal, the custom space and the space took the thing.
      [capture: rr28 `drop-kinds-sw`, 256 of 256 runs, capture.md 10.32;
      ours, 2026-11-14] **A store takes a drop**: with the region's
      store beside the dropper, every run fitted only with the store's
      square taking the thing (256 of 256; refusing it, 207), 49 drops
      landing there. [capture: rr33 `dst`, `dropcheck.py --case
      drop-store`, capture.md 10.37 item 5; ours, 2026-11-15]
      **Engine:** the thing lands on top of the square's stack; the ring,
      an obstacle and a custom obstacle fail a try; every other square
      takes it, a store included. In a room whose inside is one square it goes there with no
      draw [research, unconfirmed: traced]; the engine does so, taking
      that square without drawing. On the world map there is no
      square: the thing is lost (inventory 5.3) [research, unconfirmed:
      traced].
   2. **When it drops.** A creature is **overburdened** when the weight
      slowdown (inventory 3.3) has cut its full moves below its Speed: for
      any Speed above 0, a load above half its capacity (exactly half is
      not). It then drops as its turn's **first action**, using one move,
      and with moves left it steps on as usual with the rest. The moves of
      a drop turn come from the load before the drop; the next turn's from
      the load after it. A creature of **Speed 0** never counts as
      overburdened, however much it carries. [research: combat fourth
      pass, 2026-11-14, traced] Four peaceful friends of Speed 6, each with
      one treasure: loads of 150 and 101 of a capacity of 200 began their
      turns with 3 and 5 moves, dropped the treasure with the first and
      then took 2 and 4 steps; a load of exactly 100 of 200 began with 6,
      walked 6 and dropped nothing; a Speed 0 creature carrying 400 of a
      capacity of 100 never dropped. The droppers' next turns began with
      6. [capture: rr28 `burden2-sw`, 8 runs, all alike, capture.md 10.32;
      ours, 2026-11-14] In our earlier drop cases (8.1) the slowdown left
      0 moves, so the drop was the turn's only action. A Speed 6 creature
      carrying 400 of a capacity of 100 began its turn with 0 moves, drew
      the die, its check and the drop's pairs, dropped the treasure and
      drew no step roll and took no step; its next turn, with the load
      gone, began with 6 moves and walked 6: the drop turn's moves are the
      load before the drop's. [capture: mem `--draws`, rr29 `sw-drop6`, 32
      of 32 runs, capture.md 10.33; ours, 2026-11-14]
      **A foe beside stops the drop**, and the foe is 2.6's test, the one
      that halves a creature's moves: an aggressive individual, or a
      player beside a creature that is not a friend, on one of the 4
      squares beside it; a diagonal neighbour does not count. [research:
      combat fourth pass, traced] A creature carrying 400 of a capacity
      of 100 (0 moves) set beside the player never dropped as a peaceful
      or aggressive thief, neutral or enemy (16 runs each, brave or owning
      a weapon alike), and dropped in 16 of 16 as an aggressive friend
      (with two smartness checks before the pairs, as a fighter, 6.6).
      With the player diagonal to it, it dropped 16 of 16; beside, 0 of
      16; swapping a peaceful neutral's and an aggressive enemy's class
      between the two cases changed nothing, the square deciding. With moves left
      (Speed 6, a load of 150 of 200), beside the player as a peaceful
      neutral or an aggressive enemy it began with 1 move, did not drop
      and stepped once; as a friend it began with 3, dropped and stepped
      twice. With the player far and another creature beside it: an
      aggressive enemy stopped the drop (1 move, a step); a peaceful
      enemy or an aggressive friend did not (3 moves, a drop, 2 steps).
      [capture: rr33 `fb-c0` to `fb-c7`, `fbn`, `fbe`, `wa-`, `wb-`
      sweeps, capture.md 10.37 item 3; ours, 2026-11-15] **A drop that
      leaves 0 moves ends the turn**: no readying follows. A creature of
      Speed 2 carrying 400 of 100 and owning a bow it did not hold, nobody
      beside, dropped and did not ready (16 of 16, and 8 of 8 in longer
      runs); its next turn readied the bow and stepped on. [capture: rr33
      `r0b`, `r0b-long`, capture.md 10.37 item 4] **Engine:** so: at the
      top of its turn, after the healing die and the smartness checks
      (6.6), a creature of Speed above 0 whose load is above half its
      capacity, with no foe beside it (2.6's test), drops all its gold
      (one pile), else its lowest-numbered treasure or magic item, onto
      8.1.1's square; one drop a turn, using one move. With no move left
      the turn ends there; otherwise its readying (8.2) and its steps
      follow with the moves left, those having been set from the load
      before the drop. With a foe beside it there is no drop, and the
      turn goes on as any other (readying included: 2.7). (Superseded:
      the engine's earlier choices of dropping only above the capacity,
      with the moves then idled, and of readying after a drop that left 0
      moves.) A treasure that disappears when dropped vanishes; things
      that can't be dropped (owner-only) are never chosen. While over
      capacity it picks nothing up. [unknown] Not settled: magic items'
      place in the order (both reachable, not yet run, capture.md
      10.44). **Gold counts toward the load** per piece times Gold's
      weight, as a player's (2.5): a creature carrying only gold, over
      half its capacity, dropped all of it as one pile at the top of its
      first turn, its gold going to 0, and the two others over half their
      capacity dropped a thing at the top of theirs. [capture: mem, rr40
      `wt2-m`, the room's list and the gold watched, capture.md 10.44 item
      3; ours, no research pass, 2026-11-16] **Engine:** so.
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

## Captures needed

1. Turn order: captured and researched (2.2, 2.3), players in different
   places and two players in different rooms of one region included.
   Deaths, the restore point and declining to add a player captured
   (2.2.5 to 2.2.7), and SAVE GAME's turn, a Retire with Save as a save
   point and the saves of a death round (capture.md 10.28), and room doors
   and region to region ending the turn and carrying the moves left
   (2.2.4, capture.md 10.29). Adds in play (2.2.7.4: a roster add saves,
   a created one and a refusal do not, no add ends the visit), their
   order beside a SAVE GAME or a Retire with Save, and the all-dead save
   coming back on the map (2.2.7): captured (capture.md 10.36, 10.38).
   Still open: a Retire with Save, an add and a SAVE GAME in one round; a
   roster add's effect on the visit order (10.38 used a created player).
2. Random creatures: appearing and falling away captured and researched
   (3, 2.4); the square swept over 32 random states (3.2, capture.md
   10.40); two random creatures left at once (2.4: a percentile for each
   marked slot in slot order, a removed one's included; capture.md
   10.41). Still open: the square of each random state, draw by draw;
   what clears a slot's random mark (2.4.4; reachable, not yet run,
   capture.md 10.44).
3. The adjacency slowdown: captured (2.6: halved, 4 squares, aggressive
   and not a friend), and a turn that starts with carried moves beside an
   enemy (not halved, capture.md 10.29); odd Speeds (rounding up) and
   coming beside a foe mid-turn (no halving), for players and creatures
   (capture.md 10.41); a creature's carried turn beside a foe (not
   halved, 2.6, 5.4, capture.md 10.44).
4. Goal choice: researched and captured for slinker friends and neutrals
   (6.2-6.4), and for thieves and a slinker enemy going for gold and
   treasure (6.5, 7.5), and for a fighter enemy going for the player
   (combat 2), and for a fighter enemy choosing between the player and a
   friend in reach (6.2: rank plus steps, a tie to the player; capture.md
   10.39), and for fighter friends and neutrals among the player and
   other individuals, ties to the first looked at, and a fighter enemy
   taking a neutral or thief over an exit (6.2-6.4, capture.md 10.50).
   Not run: a fighter enemy's gold (128) against an exit or a neutral
   (the manual and the numbers agree there); fighter thieves have no
   individual ranks.
5. Paths: researched and captured (6.5: 4 squares, scored with a roll),
   and an exit's cost of 4 on the turn's first step (6.5, capture.md
   10.39); gold costing its rank / 32 on the first step too, 1 as the
   goal (6.5, capture.md 10.41); a goal's square holding an individual
   on an obstacle (5.1: a bump only on the turn after a shared door,
   never from beside; capture.md 10.44), on no obstacle not worth a run.
   Fighter or slinker afresh each turn (4.2, capture.md 10.44). Still
   open: the step choice's give-up case (6.5, a research pass); the
   golden test's check of the bump's rate at cost 4 (5.1).
6. A creature following through a door: captured (5.4: the step ends its
   turn; it lands on the destination square, sharing it with a player or a
   creature standing there; its next turn starts with the moves it had
   left, capture.md 10.41); through a door with no move left (its full
   bar next turn, capture.md 10.44). Still open, reachable but not run
   (capture.md 10.44): whether the top of the carried turn or the load
   changes the carried count (5.4); spell doors other than the three
   (5.3). Creatures never stepping onto spaces, custom ones included
   (5.1): captured (capture.md 10.41). Seen in the same runs:
   creatures' paths and goals varied from run to run (a friend sometimes
   took the portal with the player still in the room), so goal choice
   takes a roll (items 4 and 5; the random-rolls brief, item 12).
7. The overburdened drop: captured (8.1: all the gold, then the
   lowest-numbered treasure, one a turn) and the square (8.1.1, capture.md
   10.24); when a creature counts as overburdened, a drop with moves
   left, the load that sets a drop turn's moves (8.1.2, again in 10.33)
   and the custom obstacle refusing a drop (8.1.1): captured (capture.md
   10.32). A store taking a drop (8.1.1), a foe beside stopping the drop
   and who counts (8.1.2: 2.6's test, diagonal not), a drop that leaves 0
   moves ending the turn without readying (8.1.2), and a friend's gift
   placed around the giver (7.3): captured (capture.md 10.37). Gold in
   the load, per piece, and its drop as one pile (2.5, 8.1): captured
   (capture.md 10.44). Still open, reachable but not run (capture.md
   10.44): magic items' place in the order and picking nothing up over
   capacity (8.1.2), where readying falls after a drop with moves left
   (combat 6.2), and a gift's drop spell acting on whoever stands on the
   square (7.3).
8. A Speed 0 creature's turn (2.7) and a Speed 0 fighter with its goal
   beside it (capture.md 10.32): captured. A creature of Speed above 0
   whose readying spends its last move (its turn ends there) and what
   counts as ordinary for the cancelled step (only an empty square):
   captured (2.7, capture.md 10.33). A creature slowed to 0 moves by its
   load readying and stopping, and a Speed 0 thief taking gold beside it
   and a Speed 0 neutral taking a portal beside it (2.7): captured
   (capture.md 10.37). How often a Speed 0 creature beside an exit takes
   it (its one step choice a first step, the exit costing 4, 6.5), a
   Speed 0 aggressive neutral with no weapon included: captured (capture.md
   10.39). A Speed 0 fighter, owning a melee weapon or not, taking the
   one open square beside it when that is an exit or gold (2.7):
   captured (capture.md 10.41).
