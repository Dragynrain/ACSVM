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
   in 2,000-instruction chunks and reads the beam's cycle in the frame
   after each (`LIN` x 63 + `CYC`; 19,656 cycles a PAL frame, 1/50.125 s).
   No 6502 instruction takes under 2 cycles, so a chunk that seems to take
   fewer than 2 per instruction went once more round the frame. The count
   is exact while a chunk averages under about 12 cycles an instruction
   (the slowest takes 8, plus the cycles the VIC-II steals). Until November
   2026 the chunks were 1,000 instructions and only `LIN` was read; one
   `refuse` and one `weight` run gave identical output both ways. VICE
   3.10's registers have no running cycle total. Runs that record no
   frames (`weight`) don't count them.
7. **Timing.** `acs-capture run` writes `timing.jsonl` next to
   `runs.jsonl`: one line per run and a `"total"` line, with the wall time,
   the instructions advanced, instructions per host second (overall and
   inside advances), and calls and seconds per monitor command
   (`advance`, `display`, `registers`, ...). `harness_s` is the time
   outside the monitor (frame analysis, files). A short line per run goes
   to stderr. First readings (November 2026, one instance alone): a `cast`
   run is about 112 million instructions in 25 s, 99.5% of it inside
   advances at 4.4 million a second, so its fixed waits are the cost; a
   `weight` run is 312 million in 94 s, of which about 8 s are 101,000
   register reads and 9 s 14,000 frame grabs from counting frames in
   1,000-instruction chunks. With item 6's counting, `weight` takes 77 s
   (no frames counted) and one `refuse` run 90 s against 96 s; refuse's
   cost is now mostly its 25,000 frame grabs (a look every 20,000
   instructions).

   **Waiting for the screen, not the clock.** A trace of the cast flow
   (each fixed wait watched for screen changes) showed every menu
   reaction within 300,000 instructions of the input, and the menus
   holding still after that (they pause the drain). So a menu tap waits
   until the screen has not changed for a million instructions (at most
   10 million), and a tap that changes nothing stops the run with a
   screenshot (`tap-missed.png`). Opening the menu ends the same way, and
   presses again, up to three times, when the menu doesn't show: a press
   just as the turn runs out is lost. Waits on the map, where the draining
   bar keeps changing the screen, stay fixed. A cast run went from about
   112 to 28 million instructions; Power 20's wide grid (60 runs, four
   instances) from about 10.5 minutes to 2.2. The cast now comes some 85
   million instructions sooner after the jitter, so the roll is sampled at
   other moments: cost 60 worked 13 of 60 (19 before), cost 30 44 of 60
   (40 before), within the ten points section 7 gives each figure.
8. **PC: MAME 0.289 (`ibm5150`/`ibm5160`) with a Lua script** is the second
   route. It needs IBM BIOS ROMs and the MS-DOS program, which the project
   doesn't have yet. Not built.

## 3. Pieces

| Piece | Does |
| --- | --- |
| `monitor.rs` | Binary monitor client: memory, advance, resume, snapshot dump and undump, display, palette, registers, joystick, keyboard feed, reset, quit. |
| `timing.rs` | Calls and time per monitor command, instructions advanced (2.7). |
| `vice.rs` | Starts `x64sc` with the flags above, connects, `attach` through the text monitor; shuts VICE down on drop and on errors. |
| `screen.rs` | The rendered frame (8-bit indexed) to PNG; frame hashes. |
| `observe.rs` | Connected regions of one colour, as boxes: the pixel-level observer. |
| `scenario.rs` | Scenario adventures, built with the test fixture builder over the template. |
| `script.rs` | A line script for driving a session by hand (`run`, `joy`, `key`, `shot`, `watch`, `track`, `dump`, `undump`, `attach`, `detach`, `reset`). |
| `experiment.rs` | Experiments, their JSON lines, the summary and our engine's prediction for the same scenario. |
| `acts.rs` | The `acts` experiment: named tables of cases played act by act, each against a control (10.2, 10.8). |
| `power.rs` | Experiments that read the profile: its cost, a use, drift (10.6). |
| `snap.rs` | `acs-capture snap`: a snapshot just before a roll (10.11). |
| `turns.rs` | The `turns` experiment: the name strip's mover and each individual's square, as a timeline (10.10). |
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
   observation to `runs.jsonl`, and its timing to `timing.jsonl` (2.7). A
   run's result depends only on its index, so `--jobs N` splits the runs
   into N blocks of consecutive indices, each in its own process and VICE
   in `OUT/partK` (on its own copy of the disk, its stderr in
   `OUT/partK.log`), and joins their `runs.jsonl` and `timing.jsonl` in run
   order. The joined file is byte for byte what one process gives (checked
   on Power 20's 60-run cast grid with 4 and 8 jobs). Screenshots stay in
   the part folders. On the project's 12-thread machine that grid took
   139 s with 4 jobs and 87 s with 8 (about 6 minutes in one process):
   instances slow each other, but 8 still gain.
4. **Distil**: `summary.json` holds counts and distributions, plus our
   engine's outcome for the same scenario with default rules. The numbers,
   the scenario name and the run count go into the rule's spec.
5. **Compare**: deterministic rules must match exactly; statistical ones are
   compared with chi-square or Kolmogorov-Smirnov tests (from Phase 4 on).

With `C` = the research folder's `capture/`, `ACSVM_VICE` set, and
`C/boot/program1.d64` a copy of program disk 1 (the chain to `start.vsf`
takes about two minutes; a run takes 5 to 90 s alone, see 2.7; every
step is deterministic, and two runs of the chain give the same screens):

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
# a player of ours (section 8): Speed 15, everything else as move-basic's
acs-capture scenario move-basic C/runs/speed-15/disk.d64 --player spd=15
acs-capture script crates/acs-capture/scripts/start-retired.txt --disk C/runs/speed-15/disk.d64     --snapshot C/boot/play-nodisk.vsf --out C/runs/speed-15 --var OUT=C/runs/speed-15
acs-capture run move-basic --snapshot C/runs/speed-15/start.vsf --disk C/runs/speed-15/disk.d64     --out C/runs/speed-15 --runs 20 --speed 15
# refused moves and door cost: the same with --obstacle, then
acs-capture run refuse --snapshot C/runs/refuse/start.vsf --disk C/runs/refuse/disk.d64 \
    --out C/runs/refuse --runs 20 --jobs 4 --speed 15
# statistical runs: the wide jitter (10.5), split over VICE processes
acs-capture run cast --snapshot C/runs/cc-20-60/start.vsf --disk C/runs/cc-20-60/disk.d64 \
    --out C/runs/cc-20-60/wide --runs 60 --jobs 8 --jitter-step 3708203 --jitter-range 6000000
# outcome only, sampled from a menu snapshot (10.7): about 22 s for the 60
acs-capture run cast ... --runs 60 --jobs 8 --from-menu --jitter-step 3708203 --jitter-range 6000000
# turn order (10.10): creatures of ours, a second player, who moves when
acs-capture scenario move-basic C/runs/turns/two/disk.d64 --player spd=6 --player2 name=U,spd=4     --creature name=AA,x=12,y=2,spd=3 --creature name=BB,x=2,y=7,spd=5
acs-capture script crates/acs-capture/scripts/add-second.txt --disk C/runs/turns/two/disk.d64     --snapshot C/runs/turns/two/start.vsf --out C/runs/turns/two --var OUT=C/runs/turns/two
acs-capture run turns --snapshot C/runs/turns/two/two.vsf --disk C/runs/turns/two/disk.d64     --out C/runs/turns/two/turns --runs 3 --jobs 3 --turns 4 --players T,U [--enter T]
# loose ends (10.8): a table of cases on the loose scenario's disk
acs-capture scenario loose C/runs/lz-key/disk.d64 --key
acs-capture run acts --table lock --snapshot C/runs/lz-key/start.vsf --disk C/runs/lz-key/disk.d64 \
    --out C/runs/lz-key/lock --runs 5 --jitter-step 3708203 --jitter-range 6000000
# a snapshot just before a roll (10.11): the turn's end after the plan's casts
acs-capture snap turn-end --plan spell1 --snapshot C/runs/d-heal/start.vsf --disk C/runs/d-heal/disk.d64     --out C/runs/rr2/heal-turn-end.vsf
acs-capture summarize C/runs/refuse/runs.jsonl --speed 15       # re-distil a runs.jsonl
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
is a solid red square, the second marker the top half of one, and every
creature picture is a solid green square.
So the player is the one 16 x 16 green box on the map.

1. **The world map view** starts at x = 28 in the inner screen and is made
   of 16 x 16 cells; the status area starts at y = 158. The view scrolls to
   keep the player centred, so the player's cell is read off a **marker**
   cell at a known map position (terrain 2, a 16 x 16 red box, or terrain
   3, a 16 x 8 one, whichever is in view): the player is at marker +
   (player box - marker box) / 16.
2. **The movement bar** is a vertical green bar at x = 4..19 whose bottom is
   y = 158. Its height in pixels is the measurement: 8 per point of Speed
   when full (48 for Speed 6, 120 for Speed 15). A new bar is drawn over a
   few frames, so a reading is taken only once it stops growing. A bar that
   grows by more than 4 pixels is a new turn's (a Speed 1 bar is 8).
3. **Text** (`glyphs.rs`) is read by matching 8 x 8 cells against a glyph
   table taken from the program's own rendered screens. The table is the
   program's font, so it lives in the research folder
   (`capture/glyphs/profile.txt`, one line per glyph: 16 hex digits, the
   rows top to bottom, then its label) and is loaded at run time (`--glyphs`
   or `ACSVM_CAPTURE_GLYPHS`); the repository holds only the matcher.
   1. *Pixels.* A pixel of the text colour (white) is ink, a pixel of a
      sprite colour (cyan: the mark beside the profile's Wisdom and Power)
      is unknown, anything else is paper. Screenshots (PNG, any integer
      scale) are read the same way, by their colours.
   2. *The grid.* Text sits on an 8 x 8 grid that is offset from the inner
      screen's corner. With no table yet, the offset (0..7 each way) is the
      one that gives the fewest distinct non-blank cells, then the fewest
      non-blank cells: text on its grid repeats its letters, text off it
      splits into many shapes. With a table, it is the offset that matches
      the most cells.
   3. *Matching.* A cell matches a glyph when they agree on every known
      pixel. One label among the matches gives that character; none gives
      `?`; a cell with no ink reads as a space. A cell with unknown pixels
      that some glyph would also fit is flagged as doubtful, so a digit
      hidden under the mark is never read as a shorter number.
   4. *Learning* (`acs-capture glyphs learn SHOT... --glyphs T`): every
      distinct cell not in the table is added unlabelled and drawn as text
      art; the labels are filled in once by eye. `acs-capture glyphs read
      SHOT --glyphs T` prints a screen, unlabelled glyphs as `{n}`.
   5. *The profile* (menu e, e, s, s, button; button again to leave) is
      parsed into stats by finding each stat's name on a line and reading
      the number after it. Every scenario player's profile is read back
      against the record we wrote.

## 6. Output

Under `ACSVM-research/capture/`: `template-fantasy.d64`, `boot/` (program disk
copy, `play.vsf`), `scenarios/`, and `runs/NAME/` (`disk.d64`, `start.vsf`,
`room.vsf` for move-room, `runs.jsonl`, `timing.jsonl` (2.7), `summary.json`, start-up
screenshots).

## 7. First scenarios: move-basic and move-room

One player on an open field of the world map (cells 6..24 by 6..18, walled
in, start 12,12; widened to 6..34 for the Speed survey, with a second marker
at 27,14), a marker cell at 15,14, and a map door at 12,16 (terrain
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
(7.1 s). Players of other Speeds (section 8) drain at the same rate.

**Speed survey** (players of ours, section 8; 20 runs each on the widened
field, November 2026). Everything but Speed as move-basic's player:

| Speed | Pushes n/e/s/w; diagonals | Full bar | Steps per turn | Idle turn, full to empty |
| --- | --- | --- | --- | --- |
| 1 | 80 of 80 moved; 80 of 80 refused | 8 px in 20 | 1 in 20 | 369-374 frames |
| 3 | same | 24 px in 20 | 3 in 20 | 1,077-1,078 frames |
| 6 (rolled, move-basic) | same | 48 px in 20 | 6 in 20 | 2,136-2,142 frames |
| 9 | same | 72 px in 20 | 9 in 20 | 3,206-3,207 frames |
| 15 | same | 120 px in 20 | 15 in 20 | 5,328-5,334 frames |

So the full bar is 8 pixels per point of Speed and a step always costs 8
pixels. An idle turn lasts about 354 frames per point of Speed plus about
17 (7.1 s per unused step, PAL); the drain per pixel (44.3 to 46 frames)
does not depend on Speed. Our engine's prediction matched every row's
moves and steps.

**Refused moves and door cost** (experiment `refuse`, section 10.2: a
Speed 15 player of ours and an obstacle in the room; 20 runs, November
2026). Each case against its control in the same run:

| Case | Bar, case minus control | Turn, case minus control |
| --- | --- | --- |
| Map: hold west into the wall ring for 390 frames, after 6 squares | 0 in 20 | +385 to +390 frames |
| Room: the same into the room wall, after 3 squares | 0 in 20 | +384 to +405 |
| Room: the same into the obstacle, from the entry square | -1 pixel in 20 | +331 to +350 |
| Through the map door (3 squares and the door) against 3 squares | 88 against 95 pixels: one step more | +1,115 to +1,130 |

The player never moved on a refused push, so a refused move costs no step.
Holding the stick against a wall also stops the real-time drain: the turn
grows by the time held. Against the obstacle about one pixel drains (some
45 frames of the 390 count). Walking spends exactly 8 pixels a square with
no drain (6 squares leave 72 of 120). Through the door the player arrives
on the room's entry square with 88 pixels: the door step costs one step and
the turn goes on. The region loads for about 1,400 frames with the bar
hidden and not draining, which is why that turn is longer. Our engine does
the same on every count: refused moves are free, a door spends one step and
the turn continues.

**Weight slowdown** (experiment `weight`, section 10.3; 20 runs for each of
two players of ours, November 2026). Each run takes the row's things one by
one; the next turn's full bar gives the slowed Speed:

| Player | Load after the take, and the next turn's Speed (bar / 8) |
| --- | --- |
| Speed 15, capacity 1,000 | 400: 15, 500: 15, 501: 14, 600: 12, 700: 9, 800: 6, 1,000: 0 |
| Speed 10, capacity 1,500 (Strength 15) | 700: 10, 750: 10, 751: 9, 900: 8, 1,050: 6, 1,200: 4, 1,350: 2, 1,500: 0 |

Every reading was the same in all 20 runs. Above half the capacity the
slowed Speed is Speed x 2 x (capacity - load) / capacity, rounded down: a
straight line to 0 at the capacity, where the player gets no more turns it
can move in (so the loads past it were never reached). Also seen, in all
300 takes: the player **stays on its own square**; the program shows
"T TAKES ..." and waits for the button with the bar not draining; 2
million instructions after the button the bar is 15 pixels lower (the
drain alone would take about 9), so taking most likely costs one step. The
first take after entering the room also started a new turn, 20 of 20 for
both players; not understood. Our engine halved Speed above half the
capacity and moved the taker onto the square; both are now as captured
(inventory 2.1, 3.3).

**Is the randomness ours to vary?** (experiment `cast`, section 10.4;
November 2026.) Yes. A player of ours with Wisdom 20 (so Power 20, section
8) cast our spell of cost 20 once per run, from the same first-turn
snapshot after jitters of 0 to 150,461 instructions: 18 of 20 casts worked
(our long message filled the screen) and 2 failed (runs 11 and 17: "A FEW
SPARKS FIZZLE BUT TESTSPELL FAILS."), and reruns of those two failed again.
So the jitter reaches the program's random rolls, and statistical captures
can use it. 18 of 20 is far from our engine's guess for this case (Power /
(Power + cost) = 50%); the cast chance capture comes next. A spell of cost
0 worked every time it was tried. How to cast: menu, Use Power, a short
tap of the button (a held button picks the list's first entry, NONE), down
to the spell in "SELECT SPELL", a short tap again; a long-message spell
asks for no target.

**Cast chance** (experiment `cast`, section 10.5; 60 runs at each of ten
Power/cost pairs, November 2026). Players of ours with Wisdom 10, 20 and
31 (Power 10, 20 and 27: section 8.4), each owning one spell of ours:

| Power | Cost | Worked, wide jitter | Narrow jitter (first grid) | Power / (cost + 1) |
| --- | --- | --- | --- | --- |
| 10 | 10 | 51 of 60 | 60 of 60 | 91% |
| 10 | 20 | 37 | 38 | 48% |
| 20 | 10 | 60 | 60 | 100% |
| 20 | 20 | 55 | 59 | 95% |
| 20 | 30 | 40 | 25 | 65% |
| 20 | 40 | 35 | 29 | 49% |
| 20 | 60 | 19 | 17 | 33% |
| 20 | 90 | 6 | 8 | 22% |
| 27 | 31 | 50 | 25 | 84% |
| 27 | 62 | 25 | 29 | 43% |

Wide jitter is section 10.5's 0..6 million instructions; the narrow first
grid is biased (10.5) and only kept for the record. Power / (cost + 1),
that is a roll of 0..cost that works below Power, fits the wide grid best
of the candidates tried (chi-square 16.5 over the ten pairs; Power / cost
1,752, our engine's old Power / (Power + cost) 198). Its worst points are
Power 10 against cost 20 (62% seen) and Power 20 against 90 (10%); the
outcomes come in streaks along the jitter, so each figure is good to
about 10 points. Only the two outcome screens were ever seen, so a cost
above Power is cast and rolled like any other. Every cast ended the turn:
the next turn came the same 2 million instructions after the button
whether 5 or 28 pixels of the bar were left. **Superseded** (10.9): the
program's chance is 90 x Power spent / cost percent, held to 14..90,
found by the clean-room researcher and confirmed by setting the random
state. It is within 10 points of Power / (cost + 1) everywhere this grid
looked, which is why the grid couldn't tell them apart. The engine now
follows it (magic 4.1).

**Using a magic item** (experiment `item`, section 10.6.2; 3 runs for each
of two players of ours, Wisdom 10 and 20, two uses a run, November 2026).
Power read off the profile before and after each use and at each next
turn:

| Wisdom | Power: start, after use 1, next turn, after use 2, next turn |
| --- | --- |
| 10 | 10, 8, 9, 7, 8 (runs 0 and 2); 10, 9, 10, 8, 9 (run 1) |
| 20 | 20, 18, 19, 17, 18 (runs 0 and 2); 20, 19, 20, 18, 19 (run 1) |

A use costs 2 Power whatever the Wisdom, and the turn goes on: the bar
kept draining after the use (28 to 13 pixels, 20 to 5, 48 to 29) and the
next turn came when it ran out. Run 1's first use outlasted its turn (12
pixels left), so the next turn's drift (+1) shows already. The bar fell 15
to 19 pixels across a use, about 4 of them the million instructions the
harness waits after the message, so a use costs about one step. Our engine
ended the turn and cost no Power; it now costs 2 Power and a step (magic
4.2).

**Starting Power above Wisdom 25** (section 10.6.4; one player of each
Wisdom, the profile read at the top of the second and third turns): Wisdom
26 and 27 showed their own value both times; 28, 29, 30 and 31 showed 28,
then 29 (28 and 28 for Wisdom 28). With Power drifting 1 a turn toward
Wisdom, a joining player's Power is its Wisdom, at most 27, as the Wisdom
31 player of the cast grid showed on its first turn. Our engine now caps
it at 27 (magic 5.1).

**A failed cast's cost** (experiment `cast` with the profile read back,
section 10.6.3; Power 20 against cost 30, 60 wide-jitter runs, November
2026): 40 worked and 20 failed, and every one of the 60 left Power at 1 on
the profile read after the outcome (0, plus the drift of the turn the cast
ended). So a failure costs at least 19 of the 30, as much as a success;
half the cost would have left 6.

**Power, Life Force and healing over time** (experiment `drift`, section
10.6.5; a Speed 1 player of ours, Wisdom 20, Constitution 20, Life Force
20; a cost-0 spell of ours cast on the first turn, then the profile read
at the top of each of 60 turns; 3 wide-jitter runs a case, November 2026).
The first reading after a cast comes two turn ends later (Speed 1 turns
are short), so it shows two turns' worth.

| Case (spell parameter) | What the profile showed |
| --- | --- |
| Power -10 (`0x8A`) | 12, 13, ... 20 one a turn, then 20; the same in all 3 runs |
| Power +10 (`0x0A`) | 28, 27, ... 20 one a turn, then 20; the same in all 3 runs |
| Life Force +10, bit 6 set (`0x4A`) | 30, then down 1 at random turns: 10 or 11 ticks over 28 to 39 turns (31 in 101 turns, about 1 in 3); an eleventh tick in 2 runs took it to 19, back to 20 the next turn |
| Life Force +10, bit 6 clear (`0x0A`) | 30 for all 60 turns |
| Life Force -15 (`0x8F`), healing | +1 a turn from about 7 to 20, now and then +2 (4 extra points in 35 turns) |
| Disease: `0x4A` on turn 1, `0x8A` on turn 2 | 29 or 30, then 20, then 19 for one turn at each tick of the temporary part and 20 again, until turn 30 to 34 |

So Power drifts 1 a turn toward Wisdom, with no randomness; bit 6 means
temporary; the temporary part ticks away at random, about 1 turn in 3;
healing is 1 a turn (sometimes 2) whenever Life Force itself is below
Constitution, and it comes before the tick in a turn. Our engine ticked
every second turn and healed every fourth on Life Force less its temporary
part; it now follows the capture (magic 5.2, 5.3). Healing outpaces the
tick, against the manual's "about twice as fast".

**Loose ends** (experiment `acts`, section 10.8; November 2026). Steps
are counted by holding the stick after the act until the next turn's bar;
each case against its control in the same run.

| Case | Seen | Control |
| --- | --- | --- |
| Use the item, Speed 15 (10 runs) | 13 steps; after three uses 9 | 15 after one or three profile reads |
| Use the item, Speed 6 (5 runs) | 4 steps; three uses took the whole turn | 6 |
| Take a thing of weight 1 after the door and a turn's wait (Speed 6: 10 runs on each of two disks, the player owning the item or nothing) | 5 steps | 6 |
| Take at once after the door; after a step away and back; a second take in a turn (the same, and 5 runs at Speed 15) | one step less than the control every time; the turn went on in all 125 takes | the same without the take |
| Through the door out to map point 32 (10 runs) | arrived on 20,12 with 1 step left | 2 steps left after the map door |
| Push into the locked door, no key (5 runs) | refused; 2 steps left, nothing drained | 2 |
| The locked door with the key (5 runs each) | passed to 6,7 for one step; without "destroy" it passed again, with it the next push was refused | |
| Push into the obstacle 0.5, 2 or 8 million instructions after the bar shows (10 runs) | 87 pixels settled, the turn longer by the time held | 87 after the door alone; the room wall likewise 63 against 63 |
| Retired slot 2 (one run) | U picked, its profile as written | T from slot 1, as written |

So a use costs two steps, a take one, and every door one; a locked door
refuses at no cost, and "destroy needed thing" takes the key. The
obstacle costs nothing, like a wall: the pixel the first `refuse` run
saw went because that case pushed while the room was still loading.
The first take after the door did not end the turn here, unlike in the
`weight` runs, because those things were heavy: on three more disks
(`--take-weights`) whose first thing weighed 400 (stored x20), 240 (x1)
or 240 (x20), that take ended the turn every time, 15 of 15, wherever it
came in the turn (with 6 or 2 steps left), while the second thing, of
weight 1, cost one step. The `weight` runs' takes of 1 to 100 never ended
it, so the line lies between 101 and 239, or is a share of the capacity
(900 for these players).

**Healing and the temporary tick at Speed 15** (`drift`, with and without
`--walk`, 3 runs each; and 10 more Speed 1 runs of the temporary +10).
Idling its turns out (about 5,300 frames each), a Speed 15 player healed
from 6 or 7 to 20 in about 12 turns, 0 to 5 points a turn; walking all 15
steps a turn, 4 to 7 points in 25 turns. The temporary +10 ticked 8 to 11
times in 40 turns either way. So the tick follows turns, while healing
follows neither turns alone nor time (a Speed 1 player heals about 1 a
turn in turns 14 times shorter); perhaps it depends on the moves left
unused. The Speed 1 runs showed the eleventh tick (one turn at 19, then
20) in 8 of 10, 10 of 13 with the first three.

**Following through a door** (experiment `turns`, section 10.10.3;
November 2026; `C/runs/turns/follow/`). T (Speed 12) with the route east
onto the portal into SIDE ROOM (`route`, 3 runs) or staying on the entry
square (`stay`, 3 runs); AA a friend (Speed 6), CC a neutral (Speed 4).

| Seen | Runs |
| --- | --- |
| T's turn ended on the step onto the portal, with 5 steps left; T showed in SIDE ROOM at the top of its next turn | 3 of 3 |
| A creature stepping onto the portal vanished at once and the next creature moved | every time (AA in 2 route runs and 1 stay run, CC in 1 stay run) |
| AA showed in SIDE ROOM on its next turn on the portal's destination, over T, then stepped off | 2 of 2 |
| Creatures stepped straight, never diagonally | all |
| Paths and goals differed between runs from the same start: AA went for T, for the portal, or wandered, with T still in the room | all 6 |

The side room is drawn centred, so its squares read 3 right and 1 down
of their true place (T on 2,4 read as 5,5). The engine now ends a
player's turn on a room door into another room and lets a creature
arriving through a door share a player's square (movement 2.3, creatures
5.4). The varying paths point to a roll in goal or path choice: that
waits for the random-rolls pass (item 12).

**Turn order and the adjacency slowdown** (experiment `turns`, section
10.10; November 2026; disks and runs in `C/runs/turns/`). Each run's
movers come from the name strip; the creatures' bars from the left bar.

| Case | Runs | Seen |
| --- | --- | --- |
| `one`: T (Speed 6), AA, BB, CC in slots 0-2 | 6 | after each of T's turns, AA, BB, CC, then T: 17 phases |
| `slots`: BB in slot 0, AA in slot 1 | 3 | BB, AA, CC: slot order, 9 phases |
| `two`: T and U in the room | 7 | T, U, then AA, BB, CC once, then T: 21 phases, none after T's turn; no phase until both had ended a turn in the room; U stood on the entry square with T (one box) |
| `split`: T in the room, U left on the map | 5 | U took about 20 turns in a row (some 140 million instructions), then T and the creatures took round after round (5, and 15, of T's turns) and U never moved again |
| `adj-enemy`, `adj-neutral`: aggressive, Speed 0, east of T (Speed 12) | 2 each | T's bar 48 pixels at the top of each turn in the room, against 96 |
| `adj-diag`, `adj-far`, `adj-friend`, `adj-peaceful` | 2 each | 96 |

Creatures' bars were 8 pixels a point of Speed (24, 40, 32). So one
creature phase comes each round, after the last player, in slot order;
the bar is halved by an aggressive enemy or neutral on one of the 4
squares beside the player, not diagonally. Two things in the raw
timelines are the observer's, not the program's: a creature's bar is
drawn in two steps, so the first runs split some creature turns in two
(refills now count only for players), and a few looks between turns read
the strip as `?` or part of the place name. Our engine ran a phase after
each player's turn and counted 8 neighbours; it now follows the capture
(creatures 2.2, 2.6). The split case is not understood (creatures 2.3).

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
3. **Our own record works.** First the program's T record with Speed set
   to 9 was written onto a fresh scenario disk (name and record only,
   nothing at + 0xE7 or + 0x36F): it was offered, chosen and played. Now
   `acs-capture scenario move-basic OUT --player spd=15,...` builds the
   record from the decoder's field layout (byte 1 bit 6 set, as in the
   program's own player records; meaning unknown), standing on the map
   start cell, and `scripts/start-retired.txt` picks it. Players of Speed
   1, 3, 9 and 15 made this way all loaded with their Speed (section 7).
4. **Power starts at Wisdom.** A retired player of ours shows Power equal
   to its Wisdom on the profile screen once in play, whatever the record
   says: record Power 20, Wisdom 13 showed 13; 26 and 25 showed 25; 10 and
   20 showed 20. Every other stat showed as written (a player with a
   different value in each). To set a player's Power, set its Wisdom.
   Not above 25, though: Wisdom 31 showed Power 27 (and a Power bar of
   102 pixels, not 108, with a grey top). Whether Power stops at 27 or
   follows some other rule above 25 is open. Life Force is raised to
   Constitution the same way but not lowered: record Life Force 5 under
   Constitution 20 showed 20 (glyph reader, one player), while 22 over
   Constitution 21 showed 22. So a player can't start hurt; a scenario
   that needs Life Force below Constitution lowers it with a spell.
5. **Also seen:** starting play writes 2 bytes at adventure block + 0x400
   (two values seen so far; identical chains write the same ones). Entering
   the room did not write to the disk.

## 9. Addresses in use

| Address | What | Source |
| --- | --- | --- |
| `$DC00`, `$DC01` | CIA 1 ports A and B (joystick lines), read only while setting up the joystick | Documented C64 hardware |
| `LIN`, `CYC` registers | VIC-II raster line and cycle in the line, for counting frames (2.6) | VICE's register list |
| `$1CC9` | The original's random state, one byte; written before a cast by `--set-random-state` (10.9), and before other rolls by `mem` (10.12) | Clean-room researcher, findings of 2026-11-11 (`docs/research/random-rolls.md`) |
| `$F5A2` | The first player's Life Force (low 6 bits), read by `mem` (10.12) | Researcher, random-rolls second pass, 2026-11-12 |
| `$F65B`, `$F65C` | The round counter (16 bits) | Researcher, random-rolls second pass and turn-order first pass, 2026-11-12 |
| `$F65F` | The first entry of the queue of things due later: its due round, low byte (10.12) | Researcher, random-rolls second pass, 2026-11-12 |
| `$397C` | The region being visited by the turn scheduler (0 the world map) | Researcher, turn-order first pass, 2026-11-12 (`docs/research/turn-order.md`) |
| `$71FA` | The number of the individual whose turn it is | Researcher, turn-order first pass, 2026-11-12 |
| `$E1A3` on | The visited region's creature slots, 37 bytes each: +0 Constitution (0 when free), +6 bit 7 the random mark, +13 and +14 the square | Researcher, random-rolls third pass, 2026-11-12 |
| `$E1A3` on, `$F59F` on | Fighting bytes of a creature slot and of the first player's record (the stored record layout): +0 magic defense (top 2 bits), +2 Dexterity, +3 Life Force, +5 Power, +6 to +10 Dodge, Parry, Armor, Melee, Missile skills, +15 Size, +20 the weapon held, +21 the armor worn; read by `rr8/rounds.sh` (10.15) | Researcher, combat first pass, 2026-11-13 (`docs/research/combat.md`) |
| `$B799` | The current attack's blows left; while firing, the shots left after the current one | Researcher, combat first pass, 2026-11-13; shots seen in our `bow` runs (10.14.10) |
| `$F5B0`, `$F5B1`-`$F5B2` | The first player's Wisdom and gold (the record at `$F59F`, +0x11 and +0x12, from the stored record layout); poked by `rr10/sweep.sh` (10.17), the store screen showing and using both | Researcher, combat first pass (the record's address), 2026-11-13 |
| `$E1A3` + 4 | A creature slot's class: bits 4 to 6 the alignment (0 thief .. 3 friend) plus 4 if aggressive; read and set by `rr8/push.sh` (10.14.10). Slot 0's Wisdom (+17, `$E1B4`) is set there too, from the stored record layout | Researcher, combat second pass, 2026-11-13 |

These are the only addresses in the original program in use, and they
come from the researcher, never from the harness's own reading of the
program. The harness reads them (and writes `$1CC9`) only to check a
finding; the engine depends on none of them.

## 10. Next

In this order (planned November 2026):

1. **Players with chosen stats** (the weight slowdown needs Strength,
   Constitution and Life Force, cast chance needs Power).
   1. *Retired records*: done for the first slot (section 8):
      `acs-capture scenario ... --player key=value,...` writes one and
      `start-retired.txt` picks it. A second retired player would show
      slot 2 and the page layout; not needed while one player is enough.
   2. *Read the profile screen.* The 8 x 8 cells of the screen are matched
      against a glyph table taken from the program's own rendered screens
      and labelled once by eye. The table is the program's font, so it stays
      in `capture/` and is loaded at run time; the repository holds only
      the matcher. Every scenario player's stats are read back to confirm
      the record took.
   3. *Speed survey*: done (section 7).
2. **Refused moves and door cost** (movement 2.2, 2.3): done, experiment
   `refuse` (section 7). A Speed 15 player of ours and an obstacle (our own
   thing, type 10, wall picture) at room square 5,4, east of the entry
   (`--obstacle`). Every case starts at the top of a fresh turn on the map
   and records the bar once settled after its acts and the frames from the
   turn's start to its end (the bar refilling); each is compared with a
   control in the same run that does the same minus the push, so the
   real-time drain cancels. The room cases first go through the map door
   (which lets go of the stick as soon as the room shows), so they start on
   the entry square.
   - `map-wall`: walk west 6 squares to 6,12, then hold west into the wall
     ring for 2 million instructions; control `map-walk`.
   - `room-wall`: through the door, walk west 3 squares to 1,4, then hold
     west into the room wall; control `room-walk`.
   - `room-obstacle`: through the door, hold east into the obstacle;
     control `door`.
   - `door`: hold south from 12,12 until the room shows (3 squares and the
     door); control `map-walk-s3`, 3 squares south.
   Still to do: a room door back out, and a door that refuses (locked,
   needs a key).
3. **Weight slowdown** (inventory 1, 3.3 and 3.4): done (section 7),
   experiment `weight`. A
   player of ours with Strength 10, Constitution 10, Life Force 10 and
   Speed 15: capacity (10 - 0 / 2) x 100 = 1,000 by the manual's formula.
   Nine treasures of our own (things 71-79, weights x1 or x20) stand in a
   row east of the room's entry, squares 5..13 of row 4, weighing 400, 100,
   1, 99, 100, 100, 200, 1 and 500, so the load after each pick-up is 400,
   500, 501, 600, 700, 800, 1,000, 1,001 and 1,501. Each run goes through
   the map door, then steps east onto one thing at a time; after each
   pick-up it records the bar right away and lets the turn run out, and the
   next turn's full bar gives the slowed Speed (8 pixels a point).
   `--weights` on the scenario sets the row; each take shows a message the
   run answers with the button. A second player (Speed 10, Strength 15,
   capacity 1,500; loads 700 to 1,500) checks that the rule scales.
4. **Is the randomness ours to vary?** Done: yes (section 7). Experiment
   `cast`. A player of ours with Power 20 owns a spell of ours
   (thing 80, cost 20, "display long message 1", the message being our own
   "THE SPELL WORKED"; `--spell COST`; Power comes from Wisdom). Each run restores the first-turn snapshot, idles
   its jitter, casts the spell from the menu (Use Power) on the world map,
   and records a hash of the screen that follows, plus a screenshot of each
   new kind of outcome. If 20 jitters give one outcome, the jitter doesn't
   reach the program's random rolls and another noise source is needed
   before casting chances can be measured.
5. **Cast chance** (magic 4.1, 4.3; experiment `cast`): done (section 7). The status area's
   right edge holds two more bars, read like the movement bar but 4 pixels
   a point: Power (x = 276..291, blue, grey while a message waits) and Life
   Force (x = 308..319, red; its bottom runs into the red strip below
   y = 158, so it is read only above that line). Both are read as the
   number of non-black pixels in one column, counted up from y = 158.
   Trial (two runs of item 4's player, one success and one failure): the
   Power bar stayed at 80 pixels (20) through the cast and the next turns,
   while the profile screen showed Power about 2 after either outcome and
   7, then 12, over the following turns. So the Power bar does not follow
   current Power, and Power is read off the profile screen (item 6's
   drift capture needs the glyph reader). A failed cast cost as much
   Power as a working one in that trial.
   Ten Power/cost pairs, each its own scenario disk (`--player wis=P
   --spell C`; Power = Wisdom, section 8) and start chain, 60 runs each:
   Power 20 against costs 10, 20, 30, 40, 60 and 90 (the shape against
   cost), and Power 10 against 10 and 20, Power 31 against 31 and 62 (cost
   = Power and cost = 2 x Power at three Powers: does the chance follow
   the ratio, the difference or neither). Each run, from the first turn:
   1. reads the movement, Power and Life Force bars;
   2. casts the spell as in item 4;
   3. waits for the screen to settle (2.7; it was 10 million instructions
      for the grid above): the long message means it worked, the
      map still showing means it failed (a screenshot of each new kind of
      outcome screen, in case a cost above Power is refused another way);
   4. presses the button, waits 10 million instructions and reads the
      three bars again: the Power spent on a success and on a failure, and
      the movement bar left;
   5. waits for the next turn's bar (at most 80 million instructions) and
      reads the bars a third time: whether the cast ended the turn (the
      wait is short) and the first step of Power drift.
   The summary gives the successes per pair and the Power and movement
   readings per outcome. The fitted formula goes into magic 4.1.

   **Jitter range.** The first grid used section 4.3's jitter (run x 7,919,
   so 0 to 467,000 instructions, about 63 frames). Outcomes were exactly
   reproducible but not independent of the run index: for Power 20 against
   cost 60, runs 0-29 worked 15 times and runs 30-59 twice, and every pair
   below an always-works cost showed the same slide. So the roll follows
   the moment of the cast more slowly than a fresh random number would, and
   a narrow jitter range samples only a small stretch of it. A player's
   timing spreads over seconds, so the grid is run again with
   `--jitter-step 3708203 --jitter-range 6000000`: jitter = run x step mod
   range, a golden-ratio step that spreads 60 runs evenly over 6 million
   instructions (about 16 s). The range stays inside the first turn: the
   bar at the snapshot (27 pixels) drains in about 1,200 frames, some 9
   million instructions, and the menu pauses the drain.
6. **Power over time** (magic 4, 5), read off the profile screen (5.3).
   Scenario flags: `--spell COST:ACTION:PARAM` (repeatable; things 80,
   79, ..; `--spell COST` is the long-message spell of item 4) and `--item
   ACTION:PARAM`, a magic item of ours (thing 70, acts when used, the
   template's magic-item offset kept, extended record 0) that the player
   owns.
   Done (section 7), but for the loose ends below.
   1. *Is reading the profile free?* Experiment `profile-check`: done. 5
      runs from a Wisdom 20 player's first turn: the bar was the same
      before and after (27 or 28 pixels), the profile read every stat as
      written, and the rest of the turn was the control's plus about 1.2
      million instructions while the map redraws with the bar still. So the
      profile can be read every turn without changing what is measured.
      Opening the menu from the map: the button pressed between turns
      (status area blank) opens nothing, and a 25,000-instruction tap was
      missed; holding the button stops the drain and the menu opens when
      it is let go. So the harness waits for the turn's prompt, holds the
      button a million instructions, and checks for the menu's cursor
      before any tap (otherwise the taps would walk the player).
   2. *Using a magic item* (`item`): players of Wisdom 10 and 20 own an
      item that shows long message 2. Each run reads the profile, uses the
      item (menu, Use Object), answers the message, reads the profile again
      and the bar, then uses it a second time and reads once more: the
      Power cost of a use, whether it repeats, and whether using ends the
      turn.
   3. *A failed cast's cost* (`cast` with the profile): Power 20 against
      cost 30, 60 wide-jitter runs (a third fail; at cost 40 both outcomes
      would go below 0 and look alike); the profile is read right after
      the outcome, split by outcome.
   4. *Starting Power above Wisdom 25*: one player of each Wisdom 26..31,
      the start chain and one profile read.
   5. *Drift, decay and healing* (`drift`): a Speed 1 player (short turns),
      Wisdom 20, Constitution 20; the profile read at the top of each of 40
      turns, 3 jitters per case. Cases: Power -10 and +10 (cost-0 spells,
      permanent bit); a temporary Life Force +10 at Life Force 20 (no
      healing mixed in); natural healing alone at Life Force 5; a permanent
      Life Force +10 at 20 (does it stay; which bit-6 value is temporary);
      the disease stack (temporary +10 on turn 1, permanent -10 on turn 2).
   Then the loose ends: item 8.
7. **Sampling a roll from just before it** (`run cast --from-menu`, done
   November 2026). A run that replays the setup each time pays for the
   setup on every sample. So the menu is opened and the spell chosen once,
   from the first-turn snapshot, and saved as `OUT/menu.vsf` (without
   disks); each run restores it, idles its jitter there (the menu pauses
   the drain, so any range fits in the turn) and taps the button; the run
   ends at the outcome screen (no Power or turn readings: run without the
   flag for those). About 4 million instructions a run against 28.
   Does the jitter still reach the roll with the menu open? Yes, with
   short streaks along the jitter (wide range, 60 runs each, 8 jobs, 22 s
   a grid): Power 20 against cost 60 worked 20 times (33%; Power /
   (cost + 1) = 33%), against cost 30 36 times (60%; 65%). Use it for the
   statistical captures that only need an outcome; any roll reached
   through menus can be sampled this way. Exact rather than statistical
   rules would need the random state itself: see the researcher brief,
   `docs/research/random-rolls.md` (waiting for the maintainer).
8. **Loose ends** (planned November 2026; results in section 7). One experiment, `acts`, runs
   named tables of cases (`--table NAME`); `refuse` (item 2) becomes its
   first table. Every case starts at the top of a fresh turn and is
   compared with a control in the same run. Besides item 2's walk, push
   and into-room, a case can take a thing (push into it, answer the
   message), use the item, read the profile (the control for a use: it
   costs nothing, 6.1), wait for the next turn, bump n times (short pushes
   and releases), idle, go through a door to a given place and square, and
   **count steps**: hold a direction until the next turn's bar, counting
   squares and the bar after each. Counting steps right after an act gives
   the act's cost in whole steps, since walking drains nothing; the bar
   readings show whether a step needs a full 8 pixels.
   Three things the first trial runs taught the acts. A walk lets go of the
   stick as soon as the square changes: a stick still held while the view
   settles queues one more step, which the program takes later (a walk of
   one square west, then one east, ended two squares west). After a door
   the bar is hidden while the region (or the same room) loads; its
   showing again is not a new turn, so a door act waits for it. And a use's
   message is answered with a short tap, as the profile is left.
   The `loose` scenario: move-basic's field and room, a Speed 6 player of
   ours (Wisdom 20) that owns the magic item (long message 2), and in the
   room: treasures of weight 1 on 3,4 and 2,4 (west of the entry 4,4);
   a one-way portal on 4,5 to world map point 32 at 20,12 (door table
   entry 1); and a portal on 4,3 open only to owners of a key (a treasure of
   ours, thing 69) leading to room square 6,7 (entry 2). `--key` gives the
   player the key, `--lock-destroy` sets the lock's "destroy needed thing".
   Disks: no key, key, key and destroy; also `--no-item` (the player
   carries nothing) and a Speed 15 player (`--player spd=15`).
   1. *A use's step cost* (`--table use`, map): use the item, count east;
      control: the profile, count east. Then three uses against three
      profiles, on the Speed 15 player.
   2. *Taking* (`--table take`, room): after the door and a wait for the
      next turn, take west, count east; control: the same without the take.
      Why the first take ends the turn: a take right after the door; after
      the door, a step east and one back; a second take in the turn of the
      first (take west, step west, take west). On three disks: the Speed 6
      player that owns the item, one that carries nothing (the `weight`
      players carried nothing), and the Speed 15 one.
   3. *The obstacle's pixel* (`--table obstacle`, the `refuse` disk): push
      into the obstacle for 0.5, 2 and 8 million instructions, bump it five
      times, and push it after a step west and back (not straight after the
      door); each against the same into the room wall, which costs nothing.
      A drain that grows with the push is a slower drain; a fixed one is a
      cost per bump.
   4. *A room door out* (`--table exit`): after the door, south onto the
      portal; the map cell, the bar and whether the turn goes on, then
      count east on the map; control: after the door, count east.
   5. *A locked door* (`--table lock`, on each of the three disks): push
      north into the lock and count east; with the key, through it to 6,7
      and count east; with destroy, through it, back round to 5,3 and push
      west into the lock again (refused means the key went).
   6. *Retired slot 2*: a second retired player of ours (`--player2`, name
      U, other stats) in the second slot (name + 10, record + 37, as the
      decoder reads them). The add-player screen lists U right of T, under
      "don't add a player", so `start-retired2.txt` taps east once (a tap
      down went to an empty row, and the button then picked T);
      `profile-check` reads U's stats back, and T's on a copy started with
      `start-retired.txt`.
   7. *Turns or time* (`drift --walk`): a Speed 15 player, Wisdom and
      Constitution 20, Life Force -15 or a temporary +10 on the first turn;
      the profile each turn, then either idling the turn out (about 5,300
      frames) or walking its 15 steps east and back (short turns). The same
      Speed, so a rate that differs between the two follows time.
   8. *The eleventh tick*: the temporary +10 drift case again with 10 new
      runs.
9. **The random state, set** (`run cast --from-menu --set-random-state`,
   done November 2026). The clean-room researcher reported the random
   generator and its one byte of state at `$1CC9` (section 9;
   `docs/research/random-rolls.md`). Each run restores the menu snapshot
   (10.7) with no jitter, writes its index (0..255) to `$1CC9` and taps
   the button, so the 256 runs try every state once at one timing. Then a
   check: for each raster line the draw could fall on, does the reported
   generator, with magic 4.1's chance, predict every cast whose first raw
   number needed no redraw? At Power 20 against cost 10, 60 and 90 and
   Power 10 against cost 20, exactly one line did each time (38, 39, 39,
   37; inside the reported band) with 0 wrong of 200, and the next best
   line had 14 to 17 wrong. Worked over the 256 states: 231, 74, 50 and
   117, against chances of 90, 30, 20 and 45%. So the findings are
   confirmed for casting, and the earlier statistical fit (Power /
   (cost + 1)) is refuted: it makes Power 20 against cost 10 certain.
   About 35 s a pair at 6 jobs. Any other roll the researcher describes
   can be checked the same way, from a snapshot just before it.
10. **Creatures** ([creatures](rules/creatures.md) "Captures needed"),
   November 2026: turn order, the adjacency slowdown and following
   through a door done (results in section 7); players in different places
   (research brief `turn-order.md`), creature paths and goals (they take a
   roll) later.
   **Creatures of ours.** `acs-capture scenario ... --creature SPEC`
   (repeatable) puts a resident of ours in the room, in resident slots 0,
   1, .. of region 1: `name`, `x`, `y`, `align` (thief, neutral, enemy,
   friend), `aggressive`, `brave` and the `--player` stats; by default a
   Speed 3, peaceful, cautious friend with Constitution and Life Force 10.
   Each resident's picture has its own box shape (8 x 16, 16 x 8, 8 x 8,
   4 x 16, 16 x 4, 12 x 12, from the top-left corner of the square), so
   the observer tells the creatures from the players (16 x 16) and from
   each other. A peaceful, cautious friend is a slinker that seeks a
   player, then an exit (creatures 6.1); the room has no door out, so it
   walks up to a player and stops.
   **What the screen shows** (trial, November 2026, one player and three
   such creatures): the name strip under the view names whoever is
   moving, players and creatures alike, and a creature's turn has its own
   movement bar on the left, draining as it walks. After the player's
   turn ran out, the creatures moved one after another in slot order (AA,
   BB, CC, slots 0 to 2, not by Speed or distance), then the player's
   turn came again. A second player joins in the game: menu, ADD
   SOMEONE ("I will add a player as soon as possible"), and when the
   current turn ends the add-player screen shows; from CREATE NEW PLAYER,
   a tap down and one east reach the retired player U, then the button
   three times (the joystick, saving). On the map T and U then alternate.
   1. *Turn order* (experiment `turns`, done). From a first-turn snapshot each
      run samples the screen every 20,000 instructions and writes a
      timeline: a row whenever the name strip's mover, the place (room or
      map, from the strip's place name) or a square in the room changes,
      with the instructions and frames since the run began, the bar, and
      each individual's room square by box shape. A player's first turn
      walks it south through the map door into the room (the stick let go
      as soon as the strip shows the room or another mover); after that
      every turn is idled out. The run ends after the given number of
      the first player's turns. The summary gives each run's sequence of
      movers. Cases, each its own disk:
      - `one`: T (Speed 6) and three creatures, AA (12,2, Speed 3), BB
        (2,7, Speed 5), CC (12,7, Speed 4, a neutral, which with no exit
        stays put); 6 of T's turns. Who moves after T, in what order, how
        often, and whether creatures move while T is still on the map.
      - `slots`: the same squares with the slots swapped (BB in slot 0 on
        12,2, AA in slot 1 on 2,7): slot order, not name or square.
      - `two`: T and U (Speed 4) and the same three creatures; U added on
        T's first turn as above; both walk into the room on their first
        turns. Does each player's turn give the creatures a phase (the
        engine's choice, creatures 2.2), or one phase a round; do the
        creatures move after U's turn while U is still on the map; where
        U arrives when T stands on the entry square.
      - `split` (added after `two`): the same disk, but only T walks in
        (`--enter T`); U idles on the map.
   2. *The adjacency slowdown* (done; creatures 2.6; experiment `turns`, 3 of
      T's turns). T has Speed 12 (a full bar of 96 pixels), Constitution
      and Life Force 40, and walks into the room on its first turn; one
      creature of ours with Speed 0 (it never moves) and Strength 1 stands
      near the entry square 4,4. The bar at the top of T's next turns,
      against Speed 12's 96 pixels, shows any slowdown; the timeline shows
      any attack. Cases, each its own disk:
      - `adj-enemy`: an aggressive enemy on 5,4, next to the entry;
      - `adj-diag`: the same on 5,5, diagonally next to it;
      - `adj-far`: the same on 6,4, two squares away;
      - `adj-friend`: an aggressive friend on 5,4;
      - `adj-peaceful`: a peaceful enemy on 5,4;
      - `adj-neutral`: an aggressive neutral on 5,4.
      A slowdown with the enemy and none in the controls settles who
      hinders and how much; whether it is applied at the turn's start or
      while walking up to one waits for a later case.
   3. *Following through a door* (done; creatures 5.4, 6.4; "Captures needed"
      6; experiment `turns` with a route). The scenario gets a second
      room (`--side-room`): room 1 of region 1, 10 x 8, named SIDE ROOM,
      and a one-way portal of ours on room 0's square 7,4 leading to its
      square 2,4. The strip's place name now tells the two rooms apart.
      `--route NAME=MOVES` gives a player's first turn in the room a
      route: after the map door, `e3` walks 3 squares east (onto the
      portal, so into the side room); the player then idles there. T has
      Speed 12, so its first turn covers the map door (4 steps) and the
      route (3 more). Creatures of ours in room 0: AA, a peaceful friend
      (seeks a player, then an exit), Speed 6, on 10,2; CC, a peaceful
      neutral (seeks an exit), Speed 4, on 12,7. 3 runs, 4 of T's turns.
      The timeline shows whether either takes the portal, the square it
      lands on (T stands on the destination), whether its turn goes on in
      the side room, and every step on the way (paths, items 4 and 5).
      A control disk without the route (`follow-stay`: T stays on 4,4)
      shows what the same creatures do with the player still in the room.
   4. *Two players in different rooms of one region* (`rooms2`, November
      2026; creatures 2.2; experiment `turns --peek 397C,F65B,F65C,71FA`).
      The side-room disk with T (Speed 12) and U (Speed 4, added as in
      `two`), and two peaceful friends of ours (Speed 3) on 2,7 and 2,2,
      west of the entry, away from the portal. U walks into room 0 on its
      first turn; T walks in and on through the portal (`--route T=e3`),
      so both arrive during the map's visit. 3 runs, 8 of T's turns.
      Result, the same in all 3: round 13, the region's first, went U, AA,
      BB (individuals 2, 16, 17) and T had no turn; from round 14 on every
      round went T (side room), then U, AA, BB (room 0). So the rooms go
      in player order and each room's creatures follow its players, as the
      turn-order finding says; the round T missed after arriving through
      the portal is not explained (open: turn-order next pass).
      Then (`rooms2/later`, 3 runs) T took the portal on its third turn
      instead (`--route T@3=e3`: a route at the start of a player's K-th
      turn), so it came in only through the map door in round 12, the
      map visit's last (U in round 11). T again had no turn in round 13,
      in all 3 runs; after the portal, taken in round 14, it had its turn
      in every round. So the missed round goes with coming into the
      region at the map visit's end, not with the portal. (Single-player
      runs on a plain side-room disk, `rr7/portal`, couldn't show it: with
      no other mover the timeline tells turns apart only by the bar
      refilling, and it both missed and doubled turns there.)
   5. *A door destination held by a creature* (creatures 5.4, movement
      6.3; `held`, November 2026). The side-room disk with T (Speed 12,
      route `e3` into the side room), AA a peaceful friend (Speed 6) on
      room 0's 10,2, and SS (Speed 0, `room=1`) standing on the portal's
      destination 2,4 in the side room. 6 wide-jitter runs, 5 of T's
      turns, the creatures' squares and rooms peeked each turn. Result: T
      landed on 2,4 with SS in all 6; AA followed in 5 and landed on 2,4
      too, on both, then stepped off on its next turn (to 1,4, 2,3, 2,5
      or 3,6).
11. **Snapshots just before a roll** (`acs-capture snap MOMENT`, done
   November 2026), for the researcher's inputs and for state sweeps.
   From a scenario's first-turn snapshot, optionally after a drift plan's
   casts (`--plan`, one a turn, then the top of the next turn), it plays
   up to the moment and saves a snapshot with disks and a screenshot:
   `item-menu` (Use Object, the item highlighted: one tap from the use),
   `turn-end` (idling until the bar is `--bar-left` pixels, default 1:
   the turn's end a few frames away), `turn-end-walked` (`--walk N`
   squares east first) and `next-turn` (the bar just refilled). The
   random-rolls second pass's inputs were made this way, 7 in parallel in
   about 3 minutes: item use, the turn's end for drift, the temporary
   tick and healing at Speed 1, and healing at Speed 15 idled and walked
   (`C/runs/rr2/`).
12. **Confirming the random-rolls second pass** (`acs-capture run mem`,
   November 2026). `mem` restores a snapshot, sets bytes (`--set
   ADDR=VALUE`) and, with `--set-random-state`, the random state to the
   run's index, then logs every change of the watched bytes (`--watch`,
   section 9 addresses) every 20,000 instructions, for `--run-for`
   instructions or until 600,000 after the movement bar refills
   (`--until-refill`). `scripts/linefit.py` then tries every pair of
   raster lines for the draws involved, with the researcher's generator,
   and reports the pairs that predict every run that needs no redraw: as
   10.9, one pair should fit all and the rest fail.
   1. *Natural healing* (magic 5.3): from `heal15-turn-end-idle.vsf`
      (Speed 15, Life Force about 6 under Constitution 20, one idle move
      left), 256 states, until the next turn's top. Draw 1 is the idle
      move's roll of 0..14 (heals on 0), draw 2 the top-of-turn die
      (heals on 2); Life Force should gain 0, 1 or 2 as predicted.
   2. *The temporary tick* (magic 5.2): from `lf-temp-turn-end.vsf`
      (Speed 1, one entry due in round 71, the counter at 70), 256
      states. Draw 1 is the idle move's roll (of 0..0), draw 2 the tick's
      die: the entry's new due round should be 71 + the die.
   3. *Seeding*: the state in `boot/play.vsf` should read 5.
   Results (November 2026, 256 runs each, 5 jobs, a few minutes):
   healing, 253 of the 253 runs that needed no redraw gained exactly what
   the two rolls predicted (0 in 198 runs, 1 in 55, 2 in 3). The raw
   numbers are the states the watch saw, so the rolls' mappings were
   checked on every run directly; the line fit then found the first draw
   on raster line 230 in 252 runs, and the second on line 153 or 159, or
   15 when the first draw had healed (healing takes time before the
   turn's top): one fixed pair can't fit, since the second draw's moment
   depends on the first's outcome. The tick: 252 of 252 runs exact, the
   entry due 1 to 6 rounds on as the die said, with one line pair (8, 18;
   or 136, 146, the same lines' other half). The seed read 5. All three
   confirm the findings (magic 5.2, 5.3).
13. **Confirming the turn-order first pass** (`turns --peek`): the split
   case from `two.vsf` run to 25 of T's turns, reading the scheduler's
   region (`$397C`), the round counter and the current individual at each
   turn's start. The finding predicts U on the map until about 152
   million instructions (19 turns), then blocks of exactly 10 rounds,
   room and map in turn. Result (2 runs, to 25 of T's turns): U alone in
   rounds 21 to 39 (region 0), T, AA, BB, CC in rounds 40 to 49 (region
   1), U in 50 to 59, the room from 60 on, in both runs, the individual
   numbers 1, 2 and 16 to 18 as reported. The earlier `split` runs had
   only stopped before U's second block. Confirmed (creatures 2.2).
14. **Confirming the random-rolls third pass** (November 2026).
   **Every draw logged.** `mem --draws` sets a VICE checkpoint on stores
   to the random state (section 9) and steps in 2,000-instruction chunks,
   each hit ending one early; at each hit it logs the state and the raster
   line. A draw stores twice, state XOR line and then the raw number, so
   the log gives every roll's raw number and line directly, with no line
   fit. `scripts/creaturecheck.py` replays the researcher's goal and step
   rules on those numbers and compares the creatures' squares (their
   record bytes, section 9; the program also writes passing values there,
   so only inside squares count).
   1. *Goals and steps* (creatures 6): from `rr3/one-room-turn-end.vsf`
      (two peaceful friends, a peaceful neutral, no door) and
      `rr3/follow-room-turn-end.vsf` (a friend and a neutral, a portal),
      64 states each, 12 million instructions (one round). Result: every
      step predicted in every run, 768 and 599 steps (the friends going
      for the player, the neutral for the portal or, with no goal,
      wandering round its start).
   2. *Random creatures appearing* (creatures 3): `--random-creature
      SPEC` writes the region's random creature 0 (picture 70, an 8 x 8
      box) and `--room-chance` / `--side-chance` room 0's or the side
      room's chance (0..15, 5% each). At chance 15, `turns` with the slot's
      record peeked: through the map door one appeared in 8 of 10 runs as
      the room was drawn; through the portal into the side room (T's route
      `e3`) in 6 of 10 as the next round began; always in slot 0 with the
      random mark, on squares spread over the inside.
   3. *Random creatures falling away* (creatures 2.4; `fall`, November
      2026). The side-room disk with seven residents of ours (Speed 0, on
      the top row and 12,8: slots 0 to 6, so the slot roll hits half the
      time), the random creature (Speed 0) at room 0's chance 15 and a
      Speed 7 player, Constitution and Life Force 20. A script walks T
      into room 0 (the creature showed on 7,1, slot 7) and on through the
      portal, and saves `rr4/fall/side.vsf` with T in the side room. Then
      `mem --set-random-state --draws` from there, 256 states, 60 million
      instructions (five round ends, the first that of the round T left
      room 0 in), watching the creature's Constitution (`$E2A6`, slot 7),
      its mark, the round counter, the region and the current
      individual. `scripts/fallcheck.py` reads each round end's burst of
      draws: T's last idle move's healing roll (just before every round's
      end), the slot roll, the percentile when the slot is in use, then
      the side room's random-creature roll for the next round.
      Result (256 runs, 8 jobs, about 22 minutes): every one of the
      1,280 round ends drew what the rule in creatures 2.4 says, and the
      creature went in exactly the 114 runs, at exactly the round end, the
      draws predicted (27, 29, 26, 19 and 13 at the first to fifth).
      The researcher's reading (a percentile only for a random creature
      whose room was off the round's list) got 154 of the 256 runs wrong: the percentile comes whenever the slot is in use, also with
      no random creature left, and the creature went at the end of the
      round T left room 0 in. The watch's reads of `$E2A6` sometimes
      show another value for one sample and the old one at the next (most
      likely a read while other memory is banked in); the checker drops
      those.
   4. *Thieves and gold* (creatures 6.5, 7.5; `thief`, November 2026).
      New scenario keys: `--thing X,Y[,W]` puts a treasure of ours on a
      room 0 square (things 60, 61, ..; no message of its own),
      `--gold X,Y,N` a pile of gold (thing 81, item parameter N), and a
      creature of ours takes `room=` (0 or 1), `gold=` (its wealth) and
      `carry=W1+W2` (treasures of ours it owns, things 50, 51, .. across
      the residents). The program stops for a message whenever a creature
      takes something ("XX takes THING1", "YY finds gold worth 512 and now
      has 512"), so `mem --answer T` presses the button while the name
      strip names someone other than the players and the message area
      (inner rows 170 to 199) holds anything besides its background and
      black, and saves a screenshot of each of the first 16 messages a run.
      The room: T on 4,4; treasures on 12,2 and 9,7; gold piles of 10 on
      2,2 and of 5 on 11,6; XX, a peaceful thief (Speed 5) on 8,3, YY, an
      aggressive thief (Speed 4) on 12,8, ZZ, a peaceful enemy (Speed 4)
      on 2,5. From `rr5/thief-turn-end.vsf`, 64 states, 14 million
      instructions, every draw logged, the creatures' squares, gold and
      possession bytes watched. Result: `scripts/creaturecheck.py thief
      --smart fighters --take-beside` predicts all 64 runs, 598 steps,
      once two things are added to the goal and step rules: YY, a fighter
      thief, takes the smartness roll (with friends and enemies only, 9
      runs fit), and when the square a creature picks holds a thing it
      takes, it takes it from beside without moving and its turn ends
      (the possession bit set while its square stayed beside the thing;
      no step rolls after). XX's two treasures scored the same (37) and it
      went for 12,2, the first in the room's order. The gold amounts:
      YY's pile of 5 gave 512 in all 56 takes, ZZ's pile of 10 gave 1,361
      in all 42, because our disks lacked the piles' size records (item
      8).
   5. *The overburdened drop* (creatures 8.1, 7.3; `burden`, `burden2`,
      November 2026). Strength 1, Speed 2 creatures of ours: WA (300
      gold, treasures of weight 400 and 1, things 50 and 51), WB (2,000
      gold), WC (treasures of weight 1 and 400, things 52 and 53) and,
      as a control, WD (Strength 20, 100 gold and a treasure of weight 5).
      `mem --draws` from the turn's end, 16 states, three rounds; plus
      screenshots each round. Result, every run alike: WA dropped all 300
      gold on its first turn and thing 50 on its second; WB all 2,000 on
      its first; WC thing 52 (weight 1) on its first and 53 on its
      second; none of them stepped in a turn with a drop, and each walked
      once under capacity. WD walked up beside T and dropped its treasure
      (a friend's gift, 7.3), keeping its gold. The gold lay on the
      dropper's own square (hidden under it until it moved); treasures
      appeared on a square beside it (north, east or south seen), after
      one to five pairs of draws: the square looks rolled, with retries.
      Not settled: that square, magic items in the order, gold's weight.
   6. *An unarmed fighter* (creatures 7.1; `fight`, November 2026; the
      first inputs for the combat brief, `docs/research/combat.md`). T
      (Speed 6, Constitution and Life Force 40) on 4,4, EE an aggressive,
      brave enemy of ours (Speed 3, Strength 10, Constitution and Life
      Force 20, no weapon) on 6,4. From `rr6/fight-turn-end.vsf`, 32
      states, one round, every draw logged, T's Life Force and EE's square
      watched, messages answered. Result: EE never attacked (T's Life
      Force stayed 40, no message, 32 of 32); it walked up to T and kept
      stepping round it. Its steps fit the goal and step rules (fighter
      enemy, the player at rank 0) in 32 of 32 runs once its turn takes
      one roll more than the rules so far give, before its first step (a
      roll of 0 to 9 after the smartness roll, or a die before the
      top-of-turn die, both fit every run); a few more draws follow its
      steps. Both are for the combat pass. `--creature ...,sword=P` now
      gives a creature of ours our melee weapon (thing 59, power P, no
      break chance, attack adjustment 0), owned and held.
   7. *An armed fighter* (`armed`, November 2026): the same, EE holding
      our melee weapon (`sword=10`). From `rr6/armed-turn-end.vsf`, 32
      states, one round. EE attacked: messages "EE misses T" (or a hit),
      a button press each, 7 to 13 a run; T's Life Force went from 40 to
      24..34 in 19 runs and stayed 40 in 13. Copied with `fight` to
      `research/combat/inputs/` for the combat pass.
   8. *What a gold pile holds* (inventory 2.5; `ga`, `gb`, November
      2026). The thief room's amounts (5 gave 512) didn't match the piles'
      parameters. The corpus showed why: every placed gold is followed by
      a size record on square 15,14 (format-notes), which our disks
      lacked, so the program read the next two bytes. `--gold X,Y,N` now
      writes the gold and its size record. Two disks, nine piles each on
      row 7 (5,7 to 13,7): sizes 1 to 9, and 10, 16, 20, 32, 51, 0, 100,
      200 and 255; a thief of ours (Speed 15, Strength 31) beside the row
      took one a turn (`mem --answer T`, one run, 150 million
      instructions). Every message read named the size written ("XX
      finds gold worth 0 and now has 129" too), and its gold ended at 45
      and 684, the sums.
   9. *Confirming the combat first pass* (`docs/rules/combat.md`,
      2026-11-13). No new runs: `scripts/combatcheck.py` replays the
      researcher's melee rules on the 32 `armed` runs (10.14.7), every
      draw logged. A run goes: T's last idle move; EE's turn (the
      top-of-turn die, two smartness checks of 0 to 9, then its steps
      with T's square open at cost 1, until it picks T's square: the
      attack, with its blows from the moves left, each blow's to-hit,
      Melee check, parry roll and heavy-blow roll); the round's end (the
      slot roll, a percentile when it picks EE's slot 0, then one more
      percentile, the room's random-creature roll); T's turn (the die,
      then its bar waited out, 6 moves, or 3 when EE ended beside it);
      EE's next turn. Result: all 32 runs right, 31 exactly and one cut
      off inside a hit's messages: every step, the blows (4 from 3 of 3
      moves left, 2 from 2, 1 from 1), 122 blows (25 hits, 97 misses) at
      chances 34, 29, 24, 19, damage 10 or 15 (heavy), T's Life Force
      after every hit and every healing (46 values), and the button
      presses (two a miss, three a hit). T's Life Force byte (`$F5A2`)
      sometimes reads 32 for one sample while EE's square reads off the
      room (banked memory, as 10.14.3); the checker drops those. Not
      covered: parries, counter-blows, armor, breaking, the player's
      attack and the alignment changes (combat.md, Captures needed).
   10. *Confirming the second combat pass* (`docs/rules/combat.md`,
      2026-11-13). `scripts/combatcheck.py --case NAME` now plays any of
      the pass's cases (10.15) with the researcher's rules: both fighters'
      records (weapon held, armor worn, magic defense), parries,
      counter-blows, breaking, disappearing, armor, magic defense, and
      for `bow` and `bow-line` the shots. It compares every watched
      fighting byte's values (T's Life Force, Power, Parry, weapon and
      armor; EE's Life Force, Power and weapon; FF's Life Force) with the
      run's, the banked reads dropped (the samples where EE's
      Constitution byte reads another value; the sample after holds the
      real values, changed or not), and for firing that each shot's first
      draw falls where `$B799` (shots left) changes, within one 20,000
      instruction sample. It no longer counts button presses (10.14.9 did).
      Result, the 16 `round` runs of each case (10.15): **all 160 right**,
      with `armed` still 32 of 32: 524 to-hit and 101 counter-blow
      rolls, 154 parry rolls (18 parries), 154 Melee checks, 121 damaging
      blows and 15 stopped by magic defense, the mail breaking once, the
      blade breaking once, the disappearing blade 11 times (once more on
      a run's last roll); 64 shots, 15 dodge rolls (5 dodges). The same
      totals as the researcher's replay of these runs. Three things had
      to be learned from the runs, none of them combat rules:
      - *The round's end* draws the slot roll and **one** percentile,
        whichever the slot: 10.14.9's further percentile after an in-use slot
        is never there (6 runs need that; `armed` passed either way, no
        run there decided it). Creatures 2.4 has the open question.
      - *A creature whose weapon disappeared* has one move fewer on its
        later turns (5 `once` runs need it; combat 6.5).
      - *FF's turn* (a Speed 0 friend, `bow-line`): the die, two
        smartness checks of 0..9, a percentile, a roll whose size these
        runs don't show (any of 2 to 16 fits), then a roll of 0..0 that
        always heals it: a hurt FF gained 1 Life Force at that point in
        all 4 runs. Fitted, not a rule we spec.
      The arrow's path (combat 8.5): an offset compared exactly with half
      a square misses one `bow-line` shot (FF reached at deviation 17, 3
      squares out); quarter, eighth and sixteenth squares rounded down
      fit all 64 (`--reach`).
      *The player's attack* (`rr8/push.sh`): `run mem` takes `--push
      DIR:N,..`, the stick held each way for N instructions from the run's
      start. From each `player-*` room snapshot, the stick east for 300k
      instructions, 12 random states, every draw logged, cut at 1.8M
      instructions (T's attack, slot 0's turn, the round's end: T's next
      turn's prompt must not be answered, the button opens its menu).
      `--answer ZZ` presses through every message. `player-friend`: 3
      blows each run (5 of 6 moves left), every roll and slot 0's Life
      Force, Power and class byte (`$E1A7`) as combat 3 and 7 say, 12 of
      12. `player-neutral-set`: the `player-friend` snapshot with slot 0
      set to an aggressive neutral (`--set E1A7=80`) of Wisdom 10
      (`--set E1B4=10`), as the researcher did (the `player-neutral`
      snapshot has T on 4,5): one roll of 0..9 after each damaging blow,
      enemy below 5, 12 of 12. `player-none` (no `--answer`): no draw, no
      step, slot 0 unchanged, 12 of 12. `player-own` was not run (its
      menu needs the button at the right moments).
15. **Combat kit and the second combat pass's inputs** (2026-11-13).
   Scenario keys for weapons and armor of ours, for players and creatures
   alike (`--player`, `--player2`, `--creature`, `--random-creature`):
   `own=blade+bow+mail` (possession bits), `hold=blade|bow` (record byte
   0x14), `wear=mail` (0x15), `md=0..3` (magic defense, byte 0's top
   bits) and `armor=N` (Armor skill, byte 8; 0 before). `sword=P` stays:
   own and hold the blade, of power P. The things' stats are the
   scenario's: `--blade`, `--bow`, `--mail` with `power`, `adj` (a
   multiple of 5: weapons -35..40, armor -65..10), `break` (0..15),
   `range` (bow, 2..9), `magic` and `once` (disappears after use). Things
   59 (BLADE), 41 (BOW) and 40 (MAIL), extended records 0, 1 and 2, each
   written when given or when anyone has it. A retired player keeps the
   weapon it holds when it joins: T's held weapon (`$F5B3`, combat
   findings) read 59 at the turn's end, and with both holding the blade
   EE was hurt (counter-blows) in 2 of 4 runs.
   `ACSVM-research/capture/runs/rr8/`: `case.sh NAME 'ARGS' [snap|room]`
   builds one case (the scenario, the retired player's start, the walk
   into the room, then a turn-end snapshot or the room snapshot itself);
   `all.sh` the pass's cases (combat brief, pass 2), six VICE processes at
   a time, about 4 minutes each; `rounds.sh` 16 `mem --draws` runs from
   each turn-end snapshot with the fighting bytes watched.
16. **The third combat pass's inputs** (2026-11-13). The same kit and
   scripts in `ACSVM-research/capture/runs/rr9/` (`case.sh`, `all.sh`,
   `rounds.sh`; `cases.txt` the exact keys): 13 turn-end cases (armor on
   its wearer's side, with Armor skill 20, of Power 12, disappearing;
   EE's Dexterity 31 against a breakable blade; magic defense 3 with a
   plain blade and 1 with mail; the bow diagonal, behind an obstacle on
   5,4 (`--obstacle`), beside T, breakable, owned but not held,
   disappearing) and `player-bow` (a room snapshot, T holding the bow).
   16 runs of each, 16 million instructions, every draw logged. Seen in
   the watched bytes: the mail gone in 4 `mail-parry` runs and T's Armor
   skill up to 2; with Armor skill 20, 21 at most; mail of Power 12 let
   T be hurt once in 16; magic defense 3 kept a plain blade off T in all
   16; behind the obstacle EE never fired and walked; **beside T, EE
   holding only the bow hurt T in 13 of 16 runs without moving**, against
   the findings' "no firing with a foe beside" (asked in the brief); the
   owned bow never showed in EE's held-weapon byte, though T was hurt
   once; the breakable bow went in 7 runs, the disappearing one in all
   16. Copied with `fall` (10.14.3) and `burden` (10.14.5) to
   `research/combat/inputs/pass3/` for the combat brief's pass 3.
   *Confirming the third combat pass* (`docs/rules/combat.md`,
   2026-11-13). `scripts/combatcheck.py --case NAME` plays the 13 cases
   above (all but `player-bow`) with the researcher's rules, and now also
   watches T's Armor skill (`$F5A7`) and EE's square (`$E1B0`, `$E1B1`).
   New in it: every individual's moves halved, rounding up, at its turn's
   start with a foe beside (creatures 2.6), which replaces the "lost move"
   fitted on `once`; readying at the turn's start (one move); before each
   of EE's steps, the firing check (combat 8.1: range, nobody beside, the
   line of fire's 19 points); the step choice with the obstacle and with
   T's square closed to a creature owning no melee weapon; the arrow in
   sixteenths (combat 8.5); the bow breaking or disappearing ending the
   firing; nothing compared after a death (the program resets the
   records). Result: **all 208 runs right**, with every pass 2 case and
   `armed` still right (400 in all): 525 to-hit and 178 counter-blow
   rolls, 159 parry rolls (17 parries), 17 Armor checks with T's Armor
   skill after each, 140 shots, 45 dodge rolls, EE's every step (54 in
   `bow-wall`, none of them a shot by 16 million instructions). One rule
   had to be learned from the runs: armor that disappears after use
   still draws its break roll before it goes (1 `mail-once` run needs
   it). `--reach trunc` (halving the arrow's path toward the shooter)
   gets 2 `bow-beside` runs wrong. EE's held-weapon byte did show the
   readied bow in `bow-ready` (41 from 1.4 or 1.7 million instructions
   on, as the researcher pointed out): what we read as "never" were
   one-sample banked reads. In `fall` the random creature's mark
   (`$E2AC`, 158) stays set after it is removed, in all 114 removals,
   which is why the round's end still draws its percentile (creatures
   2.4).
17. **Stores, a first look** (2026-11-13; `docs/rules/stores.md`).
   Scenario keys: `--store X,Y` puts the store (thing 83, type 12, the
   marker picture) on a room 0 square; `--stock V+VxN` the region's
   stock, treasures of ours (things 1, 2, .., weight 1, the door picture)
   of value V (above 255 a multiple of 20, stored x20) and count N (1..3);
   `--player gold=N,goods=V+VxN` the player's purse and treasures of
   ours to sell (things 13, ..). `ACSVM-research/capture/runs/rr10/`:
   `case.sh NAME 'ARGS'` builds a case and saves `NAME-room.vsf` with
   T's first turn in the room under way; `bump.txt`, `cont.txt`,
   `sell.txt`, `buy.txt` are the scripts below (absolute paths: VICE
   reads a relative snapshot path from its own folder and boots a bare
   C64). Case `look`: T (gold 500, Wisdom 13, goods of value 100 and 40)
   on 4,4, the store on 5,4, stock 10, 50 x2 and 200.
   - Pushing east into the store opens a **separate screen**: no walls,
     T among the wares (three squares of the ware picture for the three
     stock lines; the count-2 line shows as one square), the room's name
     still in the strip, and "T HAS 500 TO SPEND", then "press the button
     to sell, move the stick to continue". As the project owner remembered
     (stores 3.1).
   - The stick: an invitation to browse and examine the merchandise; T
     walks about the store screen.
   - The button: "T HAS 1 GOOD1", and an offer to sell it for **50** (its
     value 100, Wisdom 13), button to sell, stick to go on.
   - **No random draw** from the button press to the offer, in 4 runs
     with different random states (`mem --draws --push fire:100k`): the
     selling price is a fixed function, readable off the screen.
   *The store sweep* (2026-11-13). New script command `poke HEXADDR
   BYTE..`; the player's Wisdom is `$F5B0` and its gold `$F5B1`-`$F5B2`
   (the first player's record, +0x11 and +0x12; section 9), and the store
   screen reads both. `scripts/storeread.py` reads the four message lines
   from screenshots with letter shapes learned from known lines (this font
   draws 0 and O alike); the gold line, poked to 23456 or 17890, teaches
   the digits. `rr10/sweep.sh CASE`: from `CASE-room.vsf`, for Wisdom
   0..31, poke Wisdom and gold, bump the store, press the button and step
   through the 12 goods with the stick, a shot each (about 2 minutes a
   case).
   - **Selling** (cases `sell`, `sell2`, `sell3`: 36 values from 1 to
     5,100): **1,152 offers, every one** as `storeread.py check` predicts
     (stores 5.1): value / 3 x (25 + Wisdom) / 25 up to a value of 2,000;
     above, the store's cut stays at 1,334 and the Wisdom bonus at
     666 x Wisdom / 25. 99, 100 and 101 all bring 33 at Wisdom 0.
   - **Buying** (`look`; `buyb`, a ware of 5,100): from the browse prompt
     (`cont.vsf`) a diagonal push south-west bumps a ware: "purchase X for
     N, by pushing the button, or move on". N is the value, at Wisdom 0,
     1, 5, 13, 20, 25 and 31 alike. The button buys ("the storekeeper
     sells the X to T"), the gold going down by N (500 to 450; 6,000 to
     900), twice for a line of 2; with too little gold, "T does not have
     enough gold for the X". Bumping an empty place: "Sorry, T, but that
     item is out of stock" (`buy3`, `buy4`): each thing seems to have its
     own shelf place, the stocked ones drawn.
   - **Selling for real** (`sold.txt`): the button sells at once, no
     confirmation (500, 550, 569), and offers the next good; the sold
     goods then show on two new shelf places. From 24,998 gold
     (`capsell.txt`) a sale of 50 left 25,000 and a sale of 19 still went
     through, the gold staying 25,000.
   Not yet seen: leaving the store and its cost in moves; selling magic
   items, weapons, armor, spells; a purchase past the thing's limit; the
   shelf place of each thing (the stocked things 1 to 3 and the sold 13
   and 14 sat at no order we could see).
   *Leaving, and selling other things* (2026-11-13). The store screen uses
   the room's grid without walls: the player appears on 7,5 after bumping
   the store on 5,4 from 4,4, and walks freely; walking into where the
   walls would be, or into some empty squares (the store's own among
   them), says the item is out of stock. No way out was found: pushes in
   all eight directions from the opening prompt (the diagonals only go
   back to it), walks to all four ends, the button at the browse prompt,
   keys (RETURN, SPACE, F1 to F7, Q, X, L, E) and 60 million instructions
   of waiting (nothing on screen changed, the movement bar included).
   Asked in `docs/research/stores.md`, with copies of the `look` inputs.
   Case `sellkit` (T owning the blade, bow and mail of value 10, our
   magic item of value 1 and a spell, and a treasure of 100): the sell
   menu offered the treasure (50), MAIL, BOW, BLADE (4 each) and the item
   (0), in thing order, and never the spell.
