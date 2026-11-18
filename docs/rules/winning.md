# Rules: winning and the end of an adventure

Behaviour spec for how an adventure ends well: the goal, the "success stack"
and what the engine does when a player reaches it. Losing (every player dead)
is in [magic](magic.md) 7. Licensed CC BY 4.0. Nothing here needs new engine
code for Classic adventures; section 4 proposes an Extended rule feature.

Tags as in [movement](movement.md): **[manual]** paraphrased from the
manual, **[data]** from the stored format or corpus counts, **[unknown]**
with the engine's choice as **Engine:**. Counts cover all 15 corpus
adventures.

## 1. The goal

1. An adventure's goal is to find one particular object. [manual] The goal
   is a setting of "Let ACS Write the Adventure": the designer picks it from
   the master thing list, or ACS picks one. [manual]
2. When ACS finishes an adventure, it puts the goal on top of a **success
   stack**: the goal, then an *Activate All Things* space, a *Play Music*
   space (the endless theme for the adventure's mood), a *Display Message*
   space (a congratulations page), and a second *Play Music* space (a short
   fanfare). [manual]
3. **How it plays.** The player picks up the goal, which uncovers the space
   below it. Stepping onto that space runs the rest of the stack: the theme
   starts, the message appears, and closing the message plays the fanfare,
   which ends the theme. [manual] All of this is ordinary stack behaviour,
   already in [movement](movement.md) 5.5 and [magic](magic.md) (*Activate
   All*, messages, music).
4. **No win check is described.** The manual names no score, no win screen
   and no rule that ends the game when the goal is found. The success stack
   is made of ordinary things, and the congratulations is an ordinary long
   message. [manual: absent] On the C64 a success stack of ours (a
   treasure on top, then an Activate All space, a theme, a long message
   and a fanfare) played so: the player took the treasure, stepped onto
   the space, the long message filled the screen, and after the button
   play went on, the player walking about for rounds afterwards; no win
   screen and no end. [capture: rr36 `win-4`, `win-5`, capture.md 10.40
   item 7; ours, no research pass, 2026-11-16] In that run taking the
   treasure set no possession bit in the player's record. That is
   explained: we had placed the treasure with parameter 0, and a take
   adds the placed parameter, so a 0 is taken off the floor and owned by
   no one; placed with 1 or 2 the same treasures set the bit
   ([inventory](inventory.md) 2.7). [capture: rr38 `pk-e2`, `pk-s2`,
   `pk-w2`, capture.md 10.42 item 6; ours, no research pass, 2026-11-16]
   So a designer's goal placed with 0 would leave the finder not owning
   it, which matters only to conditions on owning it (2.3). A goal the
   program itself generated could not be tried: making one needs the
   editor's "Let ACS Write the Adventure", which the harness does not
   drive. [capture: not reached, capture.md 10.42 item 6] **Engine:**
   a generated goal plays as any placed thing.
5. **No stored goal has been found.** The decoder reads no goal field, and
   nothing in the known layout holds one (format notes). [data] Whether the
   goal setting is saved with the adventure, in one of the header bytes not
   yet understood, is [unknown].

## 2. Endings in the corpus

1. No corpus adventure keeps the generated success stack with a treasure on
   top. [data] All 15 were built by hand or heavily edited.
2. Designers build their own ending scenes from the same parts: 12 stacks
   combine *Activate All* with *Play Music* and a long message. Their top
   things are 7 obstacles, 4 custom spaces and 1 space, often with *Give*,
   *Add to Room* or *Rid Room* lower in the stack. [data] Some of these are
   mid-adventure scenes rather than endings; the data can't tell them apart.
3. Treasure matters through stores and through the conditions designers put
   on spaces and doors ("open only to owners of"), not through a score.
   [manual, data]

## 3. What the engine does

1. **Engine:** Classic adventures have no win state. A goal or success stack
   plays out as ordinary things; the engine emits the usual events (pick-up,
   spell, music, message) and play goes on. A player can keep walking around
   after the congratulations, as on the C64 for a stack built by hand
   (1.4). [capture: rr36, capture.md 10.40 item 7] Whether a generated
   goal behaves the same is [unknown]; the engine treats it the same.
2. The front end may treat a long message as the ending only when the
   adventure says so (section 4). It doesn't guess from the message text.
3. Saving, retiring and resuming work as before ([magic](magic.md) 7.4,
   [movement](movement.md) 6.6).

## 4. Extended: the `endings` rule feature

Our own adventures want a real ending: an end screen, a won state in the
save, and an end to play. That is a rule feature, so it is opt-in and
named in the bundle's `features` list (`endings`, content 2). Decided for
1.0 (2026-11-18), the one rule feature 1.0 ships.

1. The adventure lists one or more **endings**. Each names a **trigger**:
   a master thing of kind space, custom space, obstacle, custom obstacle
   or portal, placed in a given room (region and room), and a long
   message, the ending's own text.
2. When a spell held by the trigger runs for a player there (as the thing
   stepped on, bumped or passed: the spell sources space, custom space,
   obstacle, custom obstacle and portal, in that room), the game is won:
   after the command's own events, the stack's included, the engine
   emits `AdventureWon` with the ending's index and the player. A trigger
   with no spell never fires, and a creature setting it off doesn't win.
   A trigger lower in a stack that an Activate All above it runs counts
   (it runs as the space or obstacle it is), so the trigger may sit
   anywhere in an ending's stack. The first ending in the list that
   matches is the one won.
3. After `AdventureWon` the game ignores every further command and logs
   none (`Game::won` gives the ending and the player). A save is a replay
   (saves 1), so it brings the won state back by itself.
4. Every Classic way of reaching the trigger still works, and the
   trigger's spell still runs first. An engine without the feature plays
   the adventure the Classic way, with the ending as an ordinary message,
   so the feature only adds the end state.
5. **The model.** `Adventure::endings`: each `{ region, room, thing,
   message }` (region 1-based and room 0-based as the engine's places,
   the message 1-based). Empty for Classic and not written when empty.
   **No `MODEL_VERSION` bump** (the plan had one): the field is new and
   optional, so no field changes meaning, as with `player_pictures`; and
   a bump would change every adventure's fingerprint and leave every save
   reading as made over another adventure (saves 2). A bundle with
   endings must list `endings` in `features`, and one listing it must
   have endings.
6. **The front end** shows the ending's message as the end screen, with
   the bundle's credits, and offers a new game; the board stays as it
   was.

## Captures needed

1. Whether picking up the generated goal, or running the success stack, ends
   the game, shows a win screen or changes anything saved: for a stack of
   ours, captured (1.4: the message, then play goes on; capture.md 10.40).
   Still open: a goal ACS generated (not reachable by the harness, which
   does not drive the editor; capture.md 10.42) and what a save holds
   afterwards. The possession bit at the take: settled (1.4, the placed
   parameter, capture.md 10.42).
2. Where the "Let ACS Write" goal setting is stored, if anywhere: not
   reachable by capture for the same reason (capture.md 10.42); left to
   the format (1.5).
