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

## 4. Extended: the `endings` rule feature (proposed)

Our own adventures want a real ending: a credits screen, a "won" flag in the
save, and an end to replays. That is a rule feature, so it is opt-in and
named in the bundle's `features` list.

1. The adventure lists one or more **endings**. Each names a trigger thing
   (a space, custom space, obstacle or portal placed in a room) and a long
   message to show.
2. When the trigger's spell runs for a player (passing an owner-only space,
   bumping an obstacle, taking a door), the engine emits `AdventureWon` with
   the ending's index and the player, after the stack's own events.
3. After `AdventureWon` the game ignores further commands, like after
   `AllPlayersDead`. The save records which ending was reached.
4. Every Classic way of reaching the trigger still works. An engine without
   the feature plays the adventure the Classic way, with the ending as an
   ordinary message, so the feature only adds the end state.

Each ending is just a placed thing plus a message, so the Classic data model
holds it and only the `endings` list is new. It gets a `MODEL_VERSION` bump
when it is implemented.

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
