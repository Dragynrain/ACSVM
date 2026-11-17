# Scripted playthroughs (`acs play`)

`acs play` plays an adventure to its end from a walkthrough script: the
Phase 4 gate's driver. It is a front end like the viewer's walk mode and
holds no rules: it sends the engine `Command`s, reads `Event`s and state,
and finds its way by trying moves on clones of the game. Licensed CC BY
4.0. Code: `crates/acs-cli/src/play.rs` (driver), `script.rs` (this
format).

```
acs play ADVENTURE --script FILE --seed N [--players 2] --save OUT.json [--trace OUT.jsonl]
```

The game starts with player 1 at the adventure's start; players 2.. join
through Add Someone before the first step (player 1 waits out the round
that brings them in, which is in the save). The save (`docs/saves.md`) is
written at the end, also after a failure; so is the trace. A real
adventure's save and trace are derived from it: write them outside the
repo.

## 1. Header

`player N key=value ...` lines, before the first step, set player N's
ratings (a `PlayerSetup`). Keys: `name`, `speed`, `strength`,
`constitution`, `life_force` (or `life`; default: the Constitution),
`dexterity`, `wisdom`, `power`, `size`, `dodge`, `parry`, `armor_skill`,
`melee`, `missile`, `magic_defense` (0..3), `gold`, `possessions`
(comma-separated things). `"..."` quotes a value with spaces. A player
not given is a sturdy test character: `PLAYER N`, Speed 7, Strength 15,
Constitution and Life Force 20, Dexterity and Wisdom 15, Size 12, skills
40 (Missile 30). There are as many players as `--players` or the highest
`player N`, whichever is more.

## 2. Steps

One step a line; `#` starts a comment. A step is for the players picked
by the last `as`: `as 1`, `as 2`, ... or `as both` (every player in turn,
player 1 first). The default is `as 1`. Whenever it is the turn of a
player the step isn't for, the driver plays `Wait` for them.

Squares are the room's own (walls included, 0,0 the top-left wall) or the
world map's cell. THING is a master thing number or name (any case).

| Step | What it does |
|---|---|
| `walk X Y` | Walks to the square (see 3). |
| `door X Y` | Walks onto the door or portal on the square and through it, then reports where the player is. |
| `goto REGION ROOM [X Y]` | Walks through doors until in that room (region 0 room 0: the world map), then `walk X Y`. |
| `take X Y` | Walks onto the square and takes what lies there (a step onto a carriable thing takes it). |
| `bump X Y` | Walks beside the square and pushes into it once (an obstacle's spell, a store, a creature). |
| `attack X Y [N]` | Pushes into whoever stands there, following it, until it dies or N of the player's turns pass (default 20; not a failure). |
| `use THING [X Y]`, `cast THING [X Y]` | `Use` / `Cast`, aimed at the square when given. |
| `drop THING [DIR]` | `Drop`, on the own square or the one in `n`, `e`, `s`, `w` (`ne`.. too). |
| `ready THING` | `Ready`. |
| `wait [N]`, `idle [N]` | `Wait` (the rest of the turn) or `Idle` (one move), N times. |
| `buy THING`, `sell THING`, `leave` | In an open store (`bump` it first); any other step leaves it, as the engine does. |
| `save` | SAVE GAME. |
| `retire save\|destroy` | RETIRE. |
| `restore` | The death message's button: the game at the engine's restore point, then `Restore`. |
| `expect at REGION ROOM X Y` | The player stands there (region 0 room 0: the map). |
| `expect owns THING`, `expect not owns THING` | |
| `expect message N` | Long message N was shown since the last `expect`. |
| `expect alive`, `expect dead` | |
| `expect life >= N` | Also `<=`, `>`, `<`, `==`. |

A refusal event (cast, drop, ready, store, retire) fails the step. A
player's death fails the step it happens in, unless the next step is
`expect dead`.

## 3. Walking

A walk is a breadth-first search over the player's moves, each tried on a
clone of the game, so a square does what the engine says. A move is kept
when the player moves and stays in the same room or on the map, and its
own events (up to the end of the player's turn) hold no lost Life Force,
death, store, map encounter or encounter area. The walk's own target may
be any of those. A step that only takes a thing is followed by the same
step again, so a walk takes what lies on its way. A turn that ends on the
way goes on after the others' `Wait`s, so the search spans turns; the
moves are played on the real game and the walk is planned again from the
real state at each of the player's turns. The game is deterministic, so a
clone's roll is the real one: a walk on the map steers round the
encounters this seed would bring.

When the target can't be reached, the failure says what stepping onto it
did (if it was tried) and lists the squares that were reachable, nearest
first.

`goto` searches the door graph: the world map's doors and the room items
with a destination, into rooms that aren't deleted. Each hop is a `door`
walk tried on the real game and rolled back when it fails or leads
elsewhere; that door is then left out and the route searched again.

## 4. Failure and output

Each step prints a line: the step and where its player stands. On a
failure the run stops and prints the step, every involved player's state
(place, Life Force, moves, gold, readied things, possessions) and the
engine's last log lines; the save and trace so far are still written and
the exit code is 1.

The trace has one JSON object a line: `{"start": ...}`, then for each
step `{"step", "line", "text", "players"}` followed by one
`{"player", "command", "events", "log"}` per command (`log` is the
engine's `describe` line for each event), and `{"failure": ...}` last
when a step failed.
