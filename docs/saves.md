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
4. The **autosave** the original makes on entering or leaving a region
   (movement 6.6) is a point in the same command list: the number of
   commands played when it was made. After a death, play resumes there
   (magic 7.4), and new commands carry on from that point, so the list
   stays one line of play with no branches. A save made after resuming
   holds only the commands that led to the current game.

## 2. The file

A JSON object (UTF-8, written on one line). Fields:

| Field | Meaning |
| --- | --- |
| `format` | Always `"acsvm-save"`. |
| `version` | The save format version, now 1. A newer version is refused. |
| `engine` | The engine version that wrote it (informational). |
| `adventure` | `name`: the adventure's name; `fingerprint`: 16 hex digits identifying the decoded adventure (section 3). |
| `seed`, `rules`, `first` | As given to `Game::new`. |
| `placements` | Tool placements: `after` (commands played before it), `player`, `at` (a location). Usually empty. |
| `commands` | The command log, oldest first, in the engine's JSON form. |
| `autosave` | Commands played when the last autosave was made (0 = the start). |
| `state` | 16 hex digits identifying the game after all the commands (section 3). |

Files are named `NAME.acsave.json` by default; any name works, and a
dropped file is recognised by its `format` field, not its name.

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
   adventure's walk.
2. A replay is a save too: sharing a save file with its adventure name is
   enough to reproduce a bug exactly, as long as the other side owns the
   same adventure file.
3. Saving anywhere, at any time, is ours; the original saves only at region
   changes. Faithful play is unaffected: the autosave and resume work as in
   the original whether or not the user ever saves a file.
