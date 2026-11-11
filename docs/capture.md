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
   findings, and are listed in section 9 with their source.
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
   buffer; the program's menus (Space, F1-F7) and name entry read it. Name
   entry drops keys fed before it is ready, well after its cursor shows;
   `start-scenario.txt` waits for it. The in-game command menu ignores keys
   and takes the stick: a push moves the cursor once at once and again
   about 40,000 instructions later, so menu taps last 25,000 instructions
   for up and down (left and right are slower to repeat).
4. **Disks.** Disk images are always copies; the corpus and program disks
   are never attached directly. Attaching through the text monitor is not
   deterministic: when the text monitor disconnects, the machine runs
   freely for a host-timed moment (the program counter moves), so whether
   the program accepted the new disk varied from run to run, drive reset or
   not. So a scenario starts with VICE started on the scenario disk and
   `boot/play-nodisk.vsf` restored: a snapshot saved without disk images
   leaves the attached disk in place. Then drive 8 is reset.
5. **Writes reach the disk file only when VICE writes it.** A snapshot
   saved *with* disk images carries its own copy of the disk: after it is
   restored, the program writes to that copy, not to the `--disk` file. To
   capture what the program writes, run from a snapshot without disks
   (`dump FILE nodisk`) and end the script with `detach`, which flushes
   the drive's track buffer to the file.
6. **Time is counted in instructions**, never wall-clock time: the harness
   stops the machine, steps it a fixed number of instructions (the monitor's
   "advance" command) and looks again. Runs are exactly reproducible from a
   snapshot and an input script. About 5 million instructions run per second
   of host time. Emulated time comes from counting frames: the harness steps
   in 1,000-instruction chunks and counts raster wraparounds (the `LIN`
   register). A PAL frame is 1/50.125 s.
7. **PC: MAME 0.289 (`ibm5150`/`ibm5160`) with a Lua script** is the second
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
| `script.rs` | A line script for driving a session by hand (`run`, `joy`, `key`, `shot`, `watch`, `track`, `dump`, `undump`, `attach`, `detach`, `reset`). |
| `experiment.rs` | Experiments, their JSON lines, the summary and our engine's prediction for the same scenario. |
| `scripts/*.txt` | Boot to the "insert adventure disk" prompt; start a scenario; walk into the room. |

## 4. One scenario, end to end

1. **Boot once** (`scripts/boot.txt`): autostart a copy of program disk 1,
   leave the demo, press F7, and save `boot/play.vsf` at "put your adventure
   disk in", and `boot/play-nodisk.vsf` without the disk images.
2. **Start the scenario** (`scripts/start-scenario.txt`): VICE starts
   on the scenario disk and restores `play-nodisk.vsf` (2.4); the script
   resets the drive, presses the button through the title, chooses "create
   new player", takes the first picture, types the name T, and saves
   `start.vsf` at the first turn. The player's stats are rolled by the
   program (move-basic's player has Speed 6). A scenario with a retired
   player of our own (section 8) chooses that player instead. For
   move-room, `scripts/enter-room.txt` then walks south
   through the map door and saves `room.vsf`.
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

With `C` = the research folder's `capture/`, `ACSVM_VICE` set, and
`C/boot/program1.d64` a copy of program disk 1 (the chain to `start.vsf`
takes about two minutes, 20 runs about six; every step is deterministic,
and two runs of the chain give the same screens):

```bash
acs-capture script crates/acs-capture/scripts/boot.txt --boot C/boot/program1.d64 --out C/boot --var OUT=C/boot
acs-capture scenario move-basic C/runs/move-basic/disk.d64 --template C/template-fantasy.d64
acs-capture script crates/acs-capture/scripts/start-scenario.txt --disk C/runs/move-basic/disk.d64 \
    --snapshot C/boot/play-nodisk.vsf --out C/runs/move-basic --var OUT=C/runs/move-basic
acs-capture run move-basic --snapshot C/runs/move-basic/start.vsf --disk C/runs/move-basic/disk.d64 \
    --out C/runs/move-basic --runs 20
# move-room: the same disk and start in C/runs/move-room, then
acs-capture script crates/acs-capture/scripts/enter-room.txt --disk C/runs/move-room/disk.d64 \
    --snapshot C/runs/move-room/start.vsf --out C/runs/move-room --var OUT=C/runs/move-room
acs-capture run move-room --snapshot C/runs/move-room/room.vsf --disk C/runs/move-room/disk.d64 \
    --out C/runs/move-room --runs 20
```

**Making the template** (once, by hand with `acs-capture script`): main menu
F1, tap the stick down three times (about 40k instructions each) to
"Fantasy builder set", push right, hold the button, and insert a blank
35-track `.d64` when asked. Then swap in a copy of program disk 2 and the
new disk as the prompts ask, holding the button about 4M instructions each
time, until "the adventure disk is ready" (about ten swaps).

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
   y = 158. Its height in pixels is the measurement: 8 per point of Speed
   when full (48 for Speed 6, 72 for Speed 9). A new bar is drawn over a few frames, so a reading is
   taken only once it stops growing.
3. **Text** is read by eye from screenshots for now. Matching 8 x 8 cells
   against the program's font will come when an experiment needs it.

## 6. Output

Under `ACSVM-research/capture/`: `template-fantasy.d64`, `boot/` (program disk
copy, `play.vsf`), `scenarios/`, and `runs/NAME/` (`disk.d64`, `start.vsf`,
`room.vsf` for move-room, `runs.jsonl`, `summary.json`, start-up
screenshots).

## 7. First scenarios: move-basic and move-room

One player on an open field of the world map (cells 6..24 by 6..18, walled
in, start 12,12), a marker cell at 15,14, and a map door at 12,16 (terrain
12, door 0) into a 15 x 10 room, region 1 room 0, arriving on square 4,4.
`move-basic` runs on the map from the first turn; `move-room` runs from a
snapshot taken after walking south through the door. Each of 20 runs pushes
the joystick once in each of the 8 directions (from a fresh snapshot, at the
top of a fresh turn), holds east for one whole turn, and leaves the stick
alone for one whole turn.

Results (20 runs each, November 2026):

| Measurement | World map | Room | Our engine before | Our engine now |
| --- | --- | --- | --- | --- |
| Push n, e, s, w | 80 of 80 moved one square that way | 80 of 80 | Same | Same |
| Push ne, nw, se, sw | 80 of 80 no move, no bar cost | 80 of 80 | Moved diagonally | Refused, no cost |
| Steps in a turn, holding east | 6 in 20 of 20 | 6 in 20 of 20 | 6 | 6 |
| Bar per step (full bar / steps) | 8 of 48 pixels | 8 of 48 pixels | One of Speed (6) | Same |
| Idle turn, full bar to empty | 2,136-2,142 frames | 2,141-2,142 frames | No real-time drain | Same |

The engine's `Rules::diagonals` now defaults to off. Reading the bar right
after a single step is unreliable, because the bar is redrawn a few frames
after the player moves (in rooms more than on the map), so the per-step cost
is taken as the full bar over the steps in one turn.

The idle bar loses one pixel every 43 to 48 frames (about 44.6 on average),
so one unused step's worth of bar (8 pixels) drains in about 357 frames
(7.1 s). A Speed 9 player (section 8) drains at the same rate per pixel.

## 8. Players of our own: retired records

The program rolls a new player's stats. Experiments need stats we choose, so
a scenario disk can carry a **retired player** of ours, which the start-up
screen offers next to "create new player".

1. **What retiring writes** (black box: create T on a scenario disk, enter
   the room, choose Retire, then "press button to save T"; the disk diffed
   against a copy taken before). Retire is not immediate: the program says
   it will retire the player as soon as possible, a little later offers to
   save or destroy T, then saves the adventure. The writes:
   - retired name 1 (10 bytes) at creatures + 0x1700 and retired record 1
     (37 bytes) at creatures + 0x1900, both exactly the bytes of player
     slot 1 in the adventure block (record at + 0x2B, name at + 0xBF), so
     the decoder's layout holds for the first slot. The record kept T's
     square (room 0 of region 1, 4,4) and decoded to the stats the profile
     shows;
   - adventure block byte + 0xE7: 0 to 7; and 145 bytes at + 0x36F..0x3FF
     (template fill to what looks like bitmap data). Not understood; not
     needed (item 3).
   Format details are in `format-notes.md`.
2. **Starting with it.** The add-player screen lists the retired player
   under the menu with the cursor on it; the button picks it, a second
   button press answers "press button of the joystick that will control
   this player", the program saves the adventure, and a third press starts
   play on the world map's start cell. Taking the player off the list
   clears its name on the disk.
3. **Our own record works.** The program's T record with Speed set to 9,
   written onto a fresh scenario disk (name and record only, nothing at
   + 0xE7 or + 0x36F), was offered, chosen and played. Speed 9: full bar 72
   pixels, 9 steps per turn holding east, 8 pixels per step; an idle full
   bar empties in 3,206 frames (44.5 frames per pixel, as for Speed 6: the
   drain per pixel does not depend on Speed, so a whole idle turn lasts
   about 357 frames per point of Speed). One run, November 2026.
4. **Also seen:** starting play writes 2 bytes at adventure block + 0x400
   (two values seen so far; identical chains write the same ones). Entering
   the room did not write to the disk.

## 9. Addresses in use

| Address | What | Source |
| --- | --- | --- |
| `$DC00`, `$DC01` | CIA 1 ports A and B (joystick lines), read only while setting up the joystick | Documented C64 hardware |
| `LIN` register | VIC-II raster line, for counting frames | VICE's register list |

No address in the original program is used.

## 10. Next

In this order (planned November 2026):

1. **Players with chosen stats** (the weight slowdown needs Strength,
   Constitution and Life Force, cast chance needs Power).
   1. *Retired records*: done for the first slot (section 8). Next: the
      scenario writer takes a player's stats and writes the record, and
      `start-scenario.txt` gets a variant that picks it. A second retired
      player would show slot 2 and the page layout; not needed while one
      player is enough.
   2. *Read the profile screen.* The 8 x 8 cells of the screen are matched
      against a glyph table taken from the program's own rendered screens
      and labelled once by eye. The table is the program's font, so it stays
      in `capture/` and is loaded at run time; the repository holds only
      the matcher. Every scenario player's stats are read back to confirm
      the record took.
   3. *Speed survey.* Players with Speed 1, 3 and 15 (6 and 9 are done):
      full bar height, steps per turn holding east, the idle drain rate,
      20 runs each.
2. **Refused moves** (movement 2.2): on the map, a few steps and then a
   push into the wall ring; in a room, a push into the room wall and into
   an obstacle (our own thing as a room item). Bars are read once settled,
   against a control run that idles for the same number of frames, so the
   real-time drain can be subtracted.
3. **Door cost** (movement 2.3): the map door from the square north of it
   (bar before, bar settled in the room, steps left in that turn), and a
   room door back out. A door that refuses (locked, needs a key) comes
   later.
4. **Weight slowdown** (inventory 1): our own things of chosen weights (x1
   and x20) in a room, picked up by stepping on them; steps per turn at
   loads of 0, just under and just over half the capacity, at capacity and
   past it.
5. **Is the randomness ours to vary?** Before any statistical capture: 20
   casts from 20 jitters, to check that the outcomes vary. If they don't,
   find another noise source first.
6. **The rest of the captures-needed lists** in `docs/rules/`: turn order
   and creature phases, the adjacency slowdown, creature paths, drift and
   heal rates, cast chance.
