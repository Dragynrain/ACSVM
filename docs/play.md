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
player the step isn't for, the driver plays `Wait` for them. Each
player's script starts at a turn with moves: a turn of theirs with no
moves left before their first step that needs one (the first after
joining, say) is waited out, so no leading `wait` is needed.

Squares are the room's own (walls included, 0,0 the top-left wall) or the
world map's cell. THING is a master thing number or name (any case).

| Step | What it does |
|---|---|
| `walk X Y` | Walks to the square (see 3). |
| `door X Y` | Walks onto the door or portal on the square and through it (also one into another square of the same room), then reports where the player is. |
| `goto REGION ROOM [X Y]` | Walks through doors until in that room (region 0 room 0: the world map), then `walk X Y` (see 3). |
| `take X Y` | Walks beside the square and pushes into it once; the step succeeds when the player picks something up (a take leaves the taker on its own square). |
| `bump X Y` | Walks beside the square and pushes into it once (an obstacle's spell, a store, a creature). |
| `attack X Y [N]` | Attacks whoever stands there: that individual, followed wherever it goes in the room (the walk to it is planned afresh each turn), until it dies or N of the player's turns pass, walking ones included (default 20; not a failure). A turn with no moves left (a wound or a load) is waited out, and counts. |
| `fire X Y [N]` | Fires the readied missile weapon (FIRE) at whoever stands there: that individual, aimed at the square it stands on at each of the player's turns, until it dies or N of the player's turns pass (default 20; not a failure); a turn with no moves left is waited out, and counts. See 3.1. |
| `use THING [X Y]`, `cast THING [X Y]` | `Use` / `Cast`, aimed at the square when given. |
| `drop THING [DIR]` | `Drop`, on the own square or the one in `n`, `e`, `s`, `w` (`ne`.. too). |
| `ready THING` | `Ready`. |
| `wait [N]` or `rest [N]`, `idle [N]` | `Wait`, the menu's REST (the rest of the turn, magic 5.6), or `Idle` (one move), N times. |
| `buy THING`, `sell THING`, `leave` | In an open store (`bump` it first); any other step leaves it, as the engine does. |
| `save` | SAVE GAME. |
| `retire save\|destroy` | RETIRE. |
| `restore` | The death message's button: the game at the engine's restore point, then `Restore`. |
| `expect at REGION ROOM X Y` | The player stands there (region 0 room 0: the map). |
| `expect owns THING`, `expect not owns THING` | |
| `expect message N` | Long message N was shown during the last action step (any step but `expect` and `as`); several `expect`s after it all read the same messages. |
| `expect alive`, `expect dead` | |
| `expect won` | The adventure won (winning 4) by this player. |
| `expect area [N]` | The player stands in an encounter area (area N). |
| `expect life >= N` | Also `<=`, `>`, `<`, `==`. |

`walk`, `door`, `goto`, `take` and `bump` take `avoid-takes` after their
numbers (`walk 5 3 avoid-takes`): the walk then never steps onto a square
that would pick something up, except a `walk`'s own target (see 3).

`walk` also takes `encounter` (`walk 28 35 encounter`, with or without
`avoid-takes`, in any order): the walk is meant to be drawn into a world
map encounter on the way and stay in its area (see 3.2). The steps after
it (`attack`, `fire`, `door` to the area's door) run in the area.
`expect area` checks that the player stands in an encounter area,
`expect area N` in area N (1..4).

A refusal event (cast, drop, ready, store, retire, fire) fails the step. A
player's death fails the step it happens in, unless the next step is
`expect dead`.

## 3. Walking

A walk is a breadth-first search over the player's moves, each tried on a
clone of the game, so a square does what the engine says. A move is kept
when the player moves and stays in the same room or on the map, and its
own events (up to the end of the player's turn) hold no lost Life Force,
death, store, map encounter or encounter area. The walk's own target may
be any of those. A walk to beside a square (`take`, `bump`, `attack`)
never steps onto that square. By default a step that only takes a thing
is followed by the same step again, so a walk takes what lies on its way;
with `avoid-takes` such a step is refused instead (the failure lists the
takes it left alone). A search state is the square plus a fingerprint of
what the player carries and what the room's squares hold, so a square is
visited again once a step changed the room or the player's things (a
key picked up, a space's spell that clears the way); the search stops at
20,000 states. A turn that ends on the
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
elsewhere; that door is then left out and the route searched again. The
fallback is limited so a `goto` can't wander: a new route may be no longer
than what the first route had left plus 2 doors, never goes back into a
place this `goto` already passed through, and once the player is in the
target region it never leaves it. When no route is left the step fails
with the doors that didn't work and why.

`attack` and `fire` find whoever stands on the square in the player's
own room, an encounter area included: there the foe is the copy the
encounter put in the map's own region 0 (`Game::region(0)`).

### 3.1 Firing

`fire` sends the engine's `Fire` command with the foe's square as the
target and no weapon chosen, so the readied missile weapon fires; the
driver checks nothing the engine checks. A fire the engine refuses (a foe
beside the player, the world map, a target outside the walls) fails the
step with the engine's reason; so does a FIRE with no missile weapon held
(the engine's NONE, a move used), and a foe that leaves the room. Firing
ends the player's turn, so N turns are N volleys. The driver does not
walk into range or line: the engine has no range or line rule that
refuses a shot (a short or blocked shot simply misses), so a script that
wants the player nearer first walks there. A foe that comes beside
between volleys makes the next fire a refusal, so the step fails there
(a melee foe is an `attack`'s).

### 3.2 Walks into encounters

A `walk ... encounter` searches as a walk does, except that a step whose
own events draw the player into an encounter area (an encounter rolled
after a map step, or an open area's entrance joined) is an end of the
walk, like the target. Of the ends the search meets it takes the one on
the shortest way to the target: the least moves to the step plus the
moves left from the stepped square to the target (diagonals or not as
the rules say, across the map's wrap when it wraps). On a tie an
encounter wins over the target; among encounters, the first found. The
search stops as soon as no further end can do better. The step succeeds
when the player ends in an encounter area and fails, saying so, when the
walk reached the target with no encounter on the way. A step that kills
the player is never an end.

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
