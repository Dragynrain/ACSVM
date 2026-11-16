# Saves and replay

Our save file: what it holds, how a game comes back from it, and what the
autosave is. Component 12 of `docs/design.md`. This is our own format, not
the original's (the original's saved characters and autosave live in the
adventure file itself; see `docs/rules/characters.md` 3 and
`docs/rules/movement.md` 6.6). Licensed CC BY 4.0. The engine implements
it in `crates/acs-engine/src/save.rs`; the viewer's walk mode saves and
loads it.

## 1. A save is a replay

1. A game is fully determined by its adventure, its seed, its rules, its
   first player and the commands played since (one seeded generator for
   every roll, `docs/design.md` Architecture). So a save holds those and
   not the game's state: replaying the commands rebuilds the state exactly.
2. The adventure itself is **not** in the save. A save is loaded over the
   same adventure file, which the user supplies as always. This keeps saves
   small (about 30 bytes a command) and keeps the adventure's data
   out of a file people may share in a bug report.
3. Tool placements (the viewer's `--place`, `Game::place`) are not
   commands, but they change the game, so the save lists them with the
   number of commands played before each. Replay makes them at the same
   point. Other tool hooks (setting Power, giving a thing, editing a slot)
   are for tests and are not saved.
4. The **autosave** is a point in the same command list: the number of
   commands played when it was made. After a death, play resumes there
   (magic 7.4): the commands up to the point are replayed and then the
   death message's button, the logged command `restore` (creatures
   2.2.6), which brings back the state of the moment the save was made
   (before the turn the saving command went on to start), clears the
   round's requests, ends the visit and begins a new cycle from the
   restored places; that cycle's first visit saves again. New commands
   carry on after `restore`, so the list stays one line of play with no
   branches. A save made after resuming holds only the commands that led
   to the current game.
   It is made where the original saves (movement 6.6, creatures 2.2.7;
   [capture: capture.md 10.25]), not at the door step:
   - when a visit begins in a region other than the previous visit's
     (the game's start counts as one: point 0); the point is the number
     of commands played when that visit's first round begins, so
     resuming replays up to the start of that round, its counter the
     visit's first round. Visits of one region in a row move nothing.
   - at the end of a round in which SAVE GAME was chosen (a logged
     command that does not end the turn), after the counter has gone up;
     the point is the commands played by then.
   - at the end of a round in which a player retired with Save
     (characters 7.3), the same way; with an add in that round, before
     the add.
   - at the end of a round in which a player given as a setup
     (`AddPlayer`, standing for a roster add) joined, after the add; an
     add of a rolled player (`CreatePlayer`) or a refused add moves
     nothing (creatures 2.2.7.4).
   The latest of these is the autosave. [capture: capture.md 10.25,
   10.28, 10.38] A SAVE GAME in the same round as a region change gives the
   same state either way; the engine takes the visit start that follows
   the round's end. A SAVE GAME in the round the last player dies (or
   the last player's Retire with Save) makes the all-dead state the
   autosave, as the original writes it after F1; a newcomer's first visit
   to another region moves the autosave on, so it comes back only when
   the newcomer starts in the region the last player died in (the world
   map) and was not taken from a roster; the original brings it back
   then too, nobody alive (creatures 2.2.7, capture.md 10.38 item 3).

## 2. The file

A JSON object (UTF-8, written on one line). Fields:

| Field | Meaning |
| --- | --- |
| `format` | Always `"acsvm-save"`. |
| `version` | The save format version, now 4. A newer version is refused; versions 1 to 3 are read as 2.1 says. |
| `engine` | The engine version that wrote it (informational). |
| `adventure` | `name`: the adventure's name; `fingerprint`: 16 hex digits identifying the decoded adventure (section 3). |
| `seed`, `rules`, `first` | As given to `Game::new`. |
| `placements` | Tool placements: `after` (commands played before it), `player`, `at` (a location). Usually empty. |
| `commands` | The command log, oldest first, in the engine's JSON form. |
| `autosave` | Commands played when the last autosave was made (0 = the start). |
| `state` | 16 hex digits identifying the game after all the commands (section 3). |

Files are named `NAME.acsave.json` by default; any name works, and a
dropped file is recognised by its `format` field, not its name.

### 2.1 Versions

- **4** (creatures 2.2.6): `restore` (`{"command":"restore"}`, the death
  message's button) is new, logged where play resumed from the autosave
  (1.4). A version 3 save is loaded as it is: it has no `restore`, its
  commands mean the same, and a game resumed under version 3 replays as
  it was played then (the saved visit going on). Saving again writes
  version 4.
  Added within version 4 (magic 2.4, capture.md 10.40): `cast` and `use`
  take an optional target square, `"at":[x,y]`, written only when given.
  Without it the command means what it always did: `target` names an
  individual (the spell acts on it, at the square it stands on then), and
  neither means the caster's own square. Earlier saves have no `at`, so
  they load, and their logs and fingerprints, as before; only the rules
  that changed with it (Kill All, summonses, Add to Room, a player's drop
  spots) can replay differently.
- **3** (creatures 2.2.7): `save_game` (`{"command":"save_game"}`, the
  menu's SAVE GAME) is new, and `autosave` is the point of 1.4: a visit
  to another region beginning, or the end of a round with SAVE GAME or a
  Retire with Save. Changed in place before any version 3 save left
  development: `save_game` no longer ends the turn.
- **2** (combat 8.8, characters 7): `fire` takes the cursor's square and
  the weapon chosen from the menu (`{"command":"fire","target":[x,y],
  "weapon":N}`, both optional), and `retire` (`{"command":"retire",
  "save":true}`, false for Destroy) is new. The front end took the
  autosave at the door step (`EnteredRegion` / `LeftRegion`). A version 2
  save is loaded as it is: its commands mean the same, it has no
  `save_game`, and its `autosave` is kept as written, so resuming goes
  back to the door step it was made at until play makes a new save
  (1.4). Saving again writes version 3.
- **1**: `fire` had no fields and was always refused, with no move and
  nothing drawn; like every command but buying and selling it ended an
  open store visit, and that was all it did. A version 1 save is loaded
  with each `fire` read as `leave_store`, which does exactly that, so the
  command count, the autosave point and the placements keep their places
  and the replayed game is the same. The log then holds `leave_store`
  where the save had `fire`, so the state fingerprint (taken over the
  whole game, its log included) differs and such a save loads with the
  diverged warning of 3.2. A version 1 save without `fire` loads as it
  always did. Saving again writes version 2.

## 3. Fingerprints and checks

1. A fingerprint is 64-bit FNV-1a over the value's JSON form, written as
   16 lower-case hex digits. The adventure's is taken over the decoded
   adventure as play began; the state's over the whole game after the last
   command.
2. Loading checks, in order:
   - `format` is `"acsvm-save"` and `version` is one we read; otherwise the
     file is refused.
   - The adventure's **name** matches; otherwise refused ("this save is for
     NAME").
   - `autosave` and every placement's `after` are within the command list,
     and every placement's player exists when it is made; otherwise
     refused as damaged.
   - The adventure's **fingerprint** matches. A mismatch with the same
     name is loaded anyway with a warning: the file may be another copy or
     platform of the adventure, or our decoder may have changed.
   - After replay, the **state** fingerprint matches. A mismatch is loaded
     anyway with a warning: the engine's rules have changed since the save
     was written (or the adventure differs), so the game played out
     differently. The replayed game is what the current engine makes of
     the same commands.
3. Commands refused on replay (a step into a wall that was open before) are
   played as the engine plays them: refused with their events, as in live
   play. Nothing is skipped.

## 4. Front ends

1. The viewer's walk mode has **Save game** (writes the file: a save
   dialog on the desktop, a download in the browser) and **Load game** (a
   file picker). Dropping a save file on the window loads it into the open
   adventure's walk. Its **SAVE GAME** button is the original's menu entry,
   the logged command of 1.4, not a file; **Resume from last save** goes
   back to the autosave (after a death in a round that also saved, the
   save before it, creatures 2.2.6) and plays `restore` there.
2. A replay is a save too: sharing a save file with its adventure name is
   enough to reproduce a bug exactly, as long as the other side owns the
   same adventure file.
3. Saving a file anywhere, at any time, is ours; the original saves only
   at a visit that changes region and at SAVE GAME (1.4). Faithful play is
   unaffected: the autosave and resume work as in
   the original whether or not the user ever saves a file.
