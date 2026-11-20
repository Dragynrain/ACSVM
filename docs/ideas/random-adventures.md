# Idea: generated adventures

**Status: an idea for some day, after 1.0. Not part of the plan, not
scheduled, nothing here is decided.** It records what the original
offered, what a version of ours could look like and where the hard parts
are, so the thinking isn't lost. If it is ever picked up, it gets a real
spec (in `content.md` or its own file) before any code, like everything
else. Licensed CC BY 4.0.

## 1. What the original did [manual]

The construction set could build a whole adventure on its own, or rework
an existing one, from a short page of settings (manual, pp. 16-18, with
asides in the sections on creatures, custom spaces, stores and music):

- **Overall settings.** A mood (fantasy, spy and mystery, science
  fiction), which chooses the names it invents and the pictures it builds
  walls from; a challenge from 1 (easy) to 10 (hard), where easy means
  milder creatures and more, plainer hints; a goal thing picked from the
  things list; and the adventure, author and map names, typed or left to
  the program.
- **Regions.** Up to 15, 8 if left alone. Each can have its own name, one
  of eight room styles (one of them "let the program pick") and its own
  challenge; the first region marked empty ends the list.
- **Map.** Draw a new world map, or keep the map and add a few doors.
- **Things.** Use only the things already defined, or invent some new
  ones when the list is thin.
- **Reworking an existing adventure,** region by region: write it anew,
  leave it nearly as it is, shuffle what's in it, or shuffle its rooms and
  what's in them.
- **Its own rules.** It used no custom spaces and no stores. Each thing
  and creature carries a number saying how many copies it may add
  (`model.rs`, already decoded); residents come from the creatures limited
  in number, random arrivals from the ones allowed many times. The goal
  always sits on a fixed stack at the end: the goal, then an Activate All
  Things space, music, the congratulations message and
  more music.
- **Speed.** Slow enough that the manual tells you to go do something
  else, with a running estimate on screen.

The result was an ordinary adventure, editable like any other.

## 2. What ours could be

Not a copy of EA's generator: the same settings and the same rules as
section 1, with the algorithm our own (an engine's choice throughout).
Reproducing the original byte for byte (same seed, same adventure) would
need a research pass through the program's generator and its rolls, and
nobody would gain much from it.

A sketch, nothing more:

- **A crate of its own** (`acs-gen`, say) whose output is an adventure
  source folder or model. `acs build`'s checks then catch every broken
  reference and limit, and the engine plays the result unchanged: the
  generator only has to choose.
- **Inputs** as in section 1, plus a seed. The same seed and settings give
  the same adventure.
- **Choosing by challenge:** creatures ranked by their traits and
  possessions, treasure by value, the copy limits respected.
- **Names per mood** from word lists of our own.
- **The success stack** built from the recipe in section 1.

## 3. Easy and hard

Easy:

- Names, picking by challenge, the copy limits and the success stack: all
  data, no new rules.
- Reworking with "shuffle contents only": things and creatures moved
  between a region's rooms.
- Checking the output: `acs build` exists already.

Hard:

- **Solvable on purpose.** Keys, locked doors and where the goal lies form
  a dependency graph; a graph search can prove the goal is reachable. A
  full walkthrough a test plays to the end, as our own adventures have,
  needs a real solver, which is much more.
- **Rooms and the map.** Room shapes inside a region, doors that line up,
  region entrances on the world map. "Shuffle rooms and contents" on an
  existing adventure is the fiddliest mode.
- **Being fun.** Hints and clue messages above all, and spy and mystery
  most of all, where the manual itself says messages matter most.
  Templates give something passable; good ones are a writing problem.
- **Looking like the original's output** (rooms per region, how many
  creatures, what mix of things). Unknown today. It could be measured as a
  black box by running the original's generator under the capture harness
  and counting what it writes, but only with the builder sets on the
  maintainer's own disks, and each run is slow even in warp mode.

## 4. What it could do that the original couldn't

Cheap once the base exists:

- **Instant and seeded:** a seed string is a shareable adventure; a daily
  adventure is a date as the seed.
- **Re-roll one region** and keep the rest.
- **Our art:** generate with a starter kit (pirates now, others later); a
  new mood is a word list and a kit.
- **Stores and custom spaces** allowed, as an option.
- **Extended limits:** more regions and larger rooms than 15 and 15 x 10
  (design.md, "Extended adventures").
- **Readable output:** plain TOML, a starting point an author can edit and
  diff.
- **In the browser,** since the core and engine already build for wasm.
- **Scaled for the party:** challenge adjusted for 2 to 4 hot-seat
  players.

Bigger, but natural:

- **Endless play:** the next region generated when the party leaves the
  last one. The engine already plays one region at a time, and with the
  seed in the save, replays still work. This one would be a rule feature
  of the Extended profile, with its own spec.

## 5. Open questions

- Whether to offer "rework an existing adventure" for Classic adventures
  at all, given they're other people's work; a reworked copy would be for
  the player's own use only.
- Whether a generated adventure is Classic (fits the original limits) or
  always Extended.
- What "challenge 7" should mean in numbers, without the original's
  output to compare against.
