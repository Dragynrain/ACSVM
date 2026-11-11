# The behavior capture harness

Spec for Phase 3: measuring the original program by playing it in an
emulator. Licensed CC BY 4.0. The code lives in `crates/acs-capture`, a tool
crate outside the core and the engine. Its results feed the "Captures needed"
lists at the end of each `docs/rules/` spec.

## 1. Ground rules

1. **Black box only.** The harness author works the way a player does: it
   sends joystick and keyboard input and reads the rendered screen. It never
   reads the original program's code. It also never relies on a memory
   address found by studying that code (stat bytes, the random seed, the
   screen layout in RAM). Addresses of that kind come only from the
   clean-room researcher (CONTRIBUTING.md, section 3) as plain-language
   findings, and are listed in section 8 with their source.
2. **Allowed machine facts.** Documented C64 hardware and KERNAL facts
   (joystick lines, the CIA ports, the VIC-II raster) and VICE's own
   features (snapshots, warp, instruction stepping, the rendered frame) are
   general knowledge, not knowledge of the program.
3. **Nothing EA in the repo.** Program disks, snapshots (they contain the
   program), frames, raw logs and scenario disks stay in the research
   folder, under `capture/`. Only distilled numbers go in the repo: counts,
   fitted parameters and the scenario definition that produced them, which
   is our own code.
4. **Scenario disks: our blocks over the user's own blank disk.** The
   original refuses an adventure disk that lacks what its "make an adventure
   disk" step writes outside the adventure blocks (on the C64 that is most of
   tracks 1-5 and other areas; 64,753 bytes are identical on all 14 corpus
   adventure disks). So each scenario disk starts from a copy of a **template**:
   a blank adventure disk made by the user's own copy of the program in VICE
   (main menu F1, Fantasy builder set). Our writer then puts its own blocks
   over it (header, world map, regions, pictures, palette). The template
   is read at run time (`--template` or `ACSVM_CAPTURE_TEMPLATE`) and never
   stored in, or copied into, the repository. Things and creatures from the
   builder set stay on the scenario disk but are never placed.

## 2. Emulator

1. **C64: VICE 3.10, `x64sc`**, found through `ACSVM_VICE` (the install
   folder or the executable). It runs with the binary monitor (API version
   2) and the text remote monitor on free local ports, warp on, sound off,
   window minimized.
2. **Joystick.** VICE 3.10's binary "joyport set" command drives the *I/O
   simulation* device (device 37), not the joystick, so the harness plugs one
   into control port 2 (`-controlport2device 37`). Its lines are active low:
   the value sent is the complement of the pressed directions, and both
   ports are released (all lines high) at start and after every snapshot
   restore. Port 1 keeps its normal joystick: an I/O simulation device
   there reads as held keys. The program reads control port 2. (VICE's
   development tree, October 2026, adds a direct joystick command; switch to
   it when a release ships it.)
3. **Keyboard.** The monitor's keyboard feed puts PETSCII into the KERNAL
   buffer; the program's menus (Space, F1-F7) and name entry read it.
4. **Disks.** Disk images are always copies; the corpus and program disks
   are never attached directly. A swap goes through the text monitor's
   `attach` followed by a reset of drive 8, which makes the drive re-read
   the disk at a fixed point. Without the drive reset, whether the program
   accepted the new disk varied from run to run.
5. **Time is counted in instructions**, never wall-clock time: the harness
   stops the machine, steps it a fixed number of instructions (the monitor's
   "advance" command) and looks again. Runs are exactly reproducible from a
   snapshot and an input script. About 5 million instructions run per second
   of host time. Emulated time comes from counting frames: the harness steps
   in 1,000-instruction chunks and counts raster wraparounds (the `LIN`
   register). A PAL frame is 1/50.125 s.
6. **PC: MAME 0.289 (`ibm5150`/`ibm5160`) with a Lua script** is the second
   route. It needs IBM BIOS ROMs and the MS-DOS program, which the project
   doesn't have yet. Not built.

## 3. Pieces

| Piece | Does |
| --- | --- |
| `monitor.rs` | Binary monitor client: memory, advance, resume, snapshot dump and undump, display, palette, registers, joystick, keyboard feed, reset, quit. |
| `vice.rs` | Starts `x64sc` with the flags above, connects, `attach` through the text monitor; shuts VICE down on drop and on errors. |
| `screen.rs` | The rendered frame (8-bit indexed) to PNG; frame hashes. |
| `observe.rs` | Connected regions of one colour, as boxes: the pixel-level observer. |
| `scenario.rs` | Scenario adventures, built with the test fixture builder over the template. |
| `script.rs` | A line script for driving a session by hand (`run`, `joy`, `key`, `shot`, `watch`, `track`, `dump`, `undump`, `attach`, `reset`). |
| `experiment.rs` | Experiments, their JSON lines, the summary and our engine's prediction for the same scenario. |
| `scripts/*.txt` | Boot to the "insert adventure disk" prompt; start a scenario. |

## 4. One scenario, end to end

1. **Boot once** (`scripts/boot.txt`): autostart a copy of program disk 1,
   leave the demo, press F7, and save `boot/play.vsf` at "put your adventure
   disk in".
2. **Start the scenario** (`scripts/start-scenario.txt`): from `play.vsf`,
   attach the scenario disk, reset the drive, press the button through the
   title, choose "create new player", take the first picture, type the name
   T, and save `start.vsf` at the first turn. The player's stats are rolled
   by the program; the profile screen shows them (move-basic's player has
   Speed 6).
3. **Runs** (`acs-capture run`): each run restores `start.vsf`, idles a
   number of instructions set by the run index (jitter: run × 7,919 mod
   1,000,000, which moves the noise sources: SID voice 3, the CIA timers and
   the raster), plays the experiment's input and writes one JSON line per
   observation to `runs.jsonl`.
4. **Distil**: `summary.json` holds counts and distributions, plus our
   engine's outcome for the same scenario with default rules. The numbers,
   the scenario name and the run count go into the rule's spec.
5. **Compare**: deterministic rules must match exactly; statistical ones are
   compared with chi-square or Kolmogorov-Smirnov tests (from Phase 4 on).

```bash
acs-capture scenario move-basic DISK.d64 --template TEMPLATE.d64
acs-capture script crates/acs-capture/scripts/boot.txt --boot PROGRAM1.d64 --out DIR --var OUT=DIR
acs-capture script crates/acs-capture/scripts/start-scenario.txt --disk PROGRAM1.d64 \
    --snapshot DIR/play.vsf --out RUN --var OUT=RUN --var SCENARIO=RUN/disk.d64
acs-capture run move-basic --snapshot RUN/start.vsf --disk RUN/disk.d64 --out RUN --runs 20
```

## 5. Observation

The scenario palette is background black, colour 1 white, colour 2 red and
colour 3 green. Our pictures are plain: walls are white stripes, the marker
is a solid red square, and every creature picture is a solid green square.
So the player is the one 16 x 16 green box on the map.

1. **The world map view** starts at x = 28 in the inner screen and is made
   of 16 x 16 cells; the status area starts at y = 158. The view scrolls to
   keep the player centred, so the player's cell is read off the
   **marker** cell (terrain 2 at a known map position): the player is at
   marker + (player box - marker box) / 16.
2. **The movement bar** is a vertical green bar at x = 4..19 whose bottom is
   y = 158. Its height in pixels is the measurement: 48 when full for a
   Speed 6 player. A new bar is drawn over a few frames, so a reading is
   taken only once it stops growing.
3. **Text** is read by eye from screenshots for now. Matching 8 x 8 cells
   against the program's font will come when an experiment needs it.

## 6. Output

Under `ACSVM-research/capture/`: `template-fantasy.d64`, `boot/` (program disk
copy, `play.vsf`), `scenarios/`, and `runs/NAME/` (`disk.d64`, `start.vsf`,
`runs.jsonl`, `summary.json`, start-up screenshots).

## 7. First scenario: move-basic

One player on an open field of the world map (cells 6..24 by 6..18, walled
in, start 12,12), a marker cell at 15,14, one empty region with one room.
Each of 20 runs pushes the joystick once in each of the 8 directions (from a
fresh snapshot, at the top of a fresh turn), holds east for one whole turn,
and leaves the stick alone for one whole turn.

Results (20 runs, November 2026):

| Measurement | Original (C64) | Our engine (default rules) |
| --- | --- | --- |
| Push n, e, s, w | 80 of 80 moved one cell that way | Same |
| Push ne, nw, se, sw | 80 of 80 no move, and no bar cost | Moves diagonally |
| Bar per step | 8 of 48 pixels (one sixth) in 80 of 80 steps | One of Speed (6) |
| Steps in a turn, holding east | 6 in 20 of 20 | 6 |
| Idle turn, full bar to empty | 2,136-2,142 frames (42.6-42.7 s PAL) | No real-time drain |

The idle bar loses one pixel every 43 to 48 frames (about 44.6 on average),
so one unused step's worth of bar (8 pixels) drains in about 357 frames
(7.1 s). Whether that rate depends on Speed needs a player with another
Speed; see section 9.

## 8. Addresses in use

| Address | What | Source |
| --- | --- | --- |
| `$DC00`, `$DC01` | CIA 1 ports A and B (joystick lines), read only while setting up the joystick | Documented C64 hardware |
| `LIN` register | VIC-II raster line, for counting frames | VICE's register list |

No address in the original program is used.

## 9. Next

- **Rooms**: the same pushes inside a room (a map door into a room).
- **Other Speeds**: start-up with several jitters gives players with
  different rolled stats; read Speed from the profile screen, then measure
  steps per turn and the drain rate for each.
- **Refused moves**: push into the wall ring and an obstacle and measure the
  bar (movement 2.2).
- **The captures-needed lists** in `docs/rules/` in their priority order.
