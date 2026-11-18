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
| `script.rs` | A line script for driving a session by hand (`run`, `joy`, `key`, `type`, `shot`, `watch`, `track`, `dump`, `undump`, `attach`, `detach`, `reset`, `peek` (one byte, or `peek ADDR N` for N in decimal), `poke`, `until ADDR =V|!V` (run until a byte is or leaves a value), `typemem ADDR TEXT` (feed each key until the byte at ADDR changes, 10.23), `dpeek` and `drivecheck` for drive 8 (10.21)). |
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
   **Corrected (10.18):** the joining player keeps its saved Power and
   Life Force. `start-retired.txt` waits 160M cycles before its first
   look, and rounds go by in that time as the turns' moves drain; Power
   drifts 1 a round toward Wisdom and Life Force heals. Read in RAM just
   after joining, every value was the record's. To start a player hurt
   or with Power off its Wisdom, set it in the record and look at once.
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
| `$B799` | The current attack's blows left; while firing, the shots left after the current one | Researcher, combat first pass, 2026-11-13; shots seen in our `bow` runs (10.14.10); a melee attack writes its blow count (the blow under way included) at its start, then counts down (10.35) |
| `$F5B0`, `$F5B1`-`$F5B2` | The first player's Wisdom and gold (the record at `$F59F`, +0x11 and +0x12, from the stored record layout); poked by `rr10/sweep.sh` (10.17), the store screen showing and using both | Researcher, combat first pass (the record's address), 2026-11-13 |
| `$E06D`-`$E07A` | The visited region's store stock: things 1 to 32 two bits each (a count 0..3, four to a byte, thing 1 in the top two bits of `$E06D`), then things 33 to 80 one bit each from `$E075` (thing 33 the top bit); read and set by `rr11` (10.17) | Researcher, stores first pass, 2026-11-13 (`docs/research/stores.md`) |
| `$F5B6` on | The first player's possessions in the same layout (record +0x17); set by `rr11` | Researcher, stores first pass, 2026-11-13 |
| `$F5AC`, `$F5AD` | The first player's square (record +13, +14): on the store screen its square there (7,5 on arrival), in a room the room square | Researcher, stores first pass, 2026-11-13 |
| `$719A` | The moves left in the current turn | Researcher, stores first pass, 2026-11-13; seen 5, 4, 1 in `rr11` |
| `$E1A3` + 4 | A creature slot's class: bits 4 to 6 the alignment (0 thief .. 3 friend) plus 4 if aggressive; read and set by `rr8/push.sh` (10.14.10). Slot 0's Wisdom (+17, `$E1B4`) is set there too, from the stored record layout | Researcher, combat second pass, 2026-11-13 |
| `$7161` | The add-player menu's cursor: 0 another adventure, 1 create new player, 2 don't add, 3 on the listed retired players; set to 1 by `rr13/create.sh` (10.18) | Researcher, characters first pass, 2026-11-14 (`docs/research/characters.md`) |
| `$F59F` | The first player's Constitution (low 6 bits, the record's +0; the top 2 bits its magic defense), poked by `rr35` with `$F5A0` and `$F5A2` (10.39 item 4); a creature slot's +4 low 4 bits its Speed (`$E1A7`, set with the class in 10.37 and 10.39) | Researcher, combat first pass (the stored record layout), 2026-11-13 |
| `$F59F` + 1 | The first player's Strength (low 5 bits); the top 2 bits the joystick port (bit 6 the second port: the "unknown" bit of 8.3). `$F5A4` (+5) its Power. Player slot k's record is at `$F59F` + 37 x (k - 1), its name (10 bytes) at `$F633` + 10 x (k - 1) | Researcher, characters first pass, 2026-11-14 |
| `$F5AF` | The first player's place (record +16): 1 to 4 while it is in encounter area n, 0 on the open map | Researcher, encounters first pass, 2026-11-14 (`docs/research/encounters.md`) |
| `$E7F2` + 5 x (n - 1) | Encounter area n's room record (n 1..4): +0 the wall picture, +1 the size (233 while open, 0 when free) | Researcher, encounters first pass, 2026-11-14 |
| `$E0A2` + 2 x (n - 1) | Encounter area n's entrance square, column then row (41, 41 when free) | Researcher, encounters first pass, 2026-11-14 |
| `$E532` + 37 x (s - 8) | The map region's creature slot s (8..15), where encounter copies go: +0 nonzero while alive, +13, +14 the square, +16 the area | Researcher, encounters first pass, 2026-11-14 |
| `$E40A` + 37 x r, `$E65A` + r | The map's random creature record r (0..7), and its byte of terrain (high four bits) and chance code (low four); setting the chance byte before a step forces or forbids an encounter | Researcher, encounters first pass, 2026-11-14 |
| `$1C0D` in drive 8 | The drive's timer-interrupt flags (its own address space, the binary monitor's memory space 1): bit 6 comes and goes in a working drive, stays set in one that never serves the interrupt; read by `drivecheck` and every `run` and `snap` (10.21) | Researcher, stores second pass, 2026-11-14 (`docs/research/stores.md`) |
| `$F59F` + 37 x (k - 1) | Player slot k's alive byte (record +0): nonzero while alive, 0 once the player dies; read by `rr18` (10.22) | Researcher, turn-order second pass, 2026-11-14 (`docs/research/turn-order.md`) |
| `$0003` in drive 8 | The job code of the buffer the program's block commands use: 128 a read waiting to run, below 128 once done; 128 in every hung snapshot (10.21) | Researcher, stores second pass, 2026-11-14 |
| `$4F4F` | Name entry's cursor (0 to 14); it moves when a key is taken. `typemem 4F4F TEXT` feeds each key until it does (10.23) | Researcher, characters second pass, 2026-11-14 (`docs/research/characters.md`) |
| `$3789`-`$3797` | Name entry's 15 characters so far, screen codes (10.23) | Researcher, characters second pass, 2026-11-14 |
| `$F59F` + 37 x (k - 1) | Player slot k is living while its first byte is not zero; poked in 10.23 to fill four slots | Researcher, characters second pass, 2026-11-14 |
| `$E1A3` + 12 | A creature slot's round (low byte) of its last turn, 0 before its first; set in 10.23 to force a catch-up | Researcher, characters second pass, 2026-11-14 |
| Adventure block + 0xE7, + 0xE8 | The round counter as saved (22 in 10.23's save at round 22) | Researcher, characters second pass, 2026-11-14 |
| d64 0x1C800, 0x1C802 | The disk's identity word and adventure-disk mark (361 on every disk of ours) | Researcher, characters second pass, 2026-11-14 |
| `$382A` (word), `$E583` on | The visited room's list of things: the word at `$382A` points at it (`$E585` in our scenarios), the byte two before (`$E583`) is its length in bytes plus 2; each entry the square (column x 16 + row), the thing, a count, a gold entry (thing 81) 6 bytes with the amount in its last two (low byte first); a new entry goes first. Read by `scripts/dropcheck.py` (10.24) | Researcher, combat fourth pass, 2026-11-14 (`docs/research/combat.md`) |
| `$E1A3` + 37 s + 16, + 18 | Creature slot s's room byte (16 + its room of region 1) and gold (low byte), from the stored record layout; watched by `rr37` (10.41: a creature through a door, a creature's take). `$719A` holds a creature's moves on its turn as a player's (10.41) | Researcher, combat first pass (the stored record layout), 2026-11-13 |

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
   *The research findings, confirmed* (2026-11-13; `docs/research/stores.md`
   first pass). `ACSVM-research/capture/runs/rr11/`, scripts there, all
   from the `look` case (`rr10/look-room.vsf`), reading the addresses the
   researcher gave (section 9) with `peek`. On the store screen a push of
   60k cycles moves one square; 300k walks several.
   - `leave-e`, `leave-w`: from 7,5 east (or west) along row 5 to 13,5
     (1,5), then north-east (north-west) into **the exit on 14,4 (0,4)**:
     "Thank you, T. Please come back soon", the button, and T is in the
     room **on the store's square 5,4**. Moves: 5 before the bump, 4 on
     the store screen, **1 after leaving** (stores 3.2, 3.6, 3.7).
   - `lastmove`: four steps in the room (5 to 1 moves), then the bump with
     the last: 0 on the store screen, 1 after leaving.
   - `ring`: from 7,4 north to 7,1, north-west onto **6,0** and west onto
     5,0: the outer ring's gaps are floor (stores 3.3).
   - `buy-limit`: T poked to own 3 of thing 1 (`$F5B6` = 192); a push from
     the opening prompt only starts browsing (T to 7,4), then south-west
     to 6,5 and south bumps thing 1 on 6,6: bought, gold 500 to 490, the
     stock of thing 1 from 1 to 0, **T still owning 3** (stores 4.3).
   - `sell-full`: the stock of thing 1 poked to 3 (`$E06D` = 228), T to own
     one (`$F5B6` = 64); the button sells it for 4, gold 500 to 504, **the
     stock still 3**, T owning none (stores 5.3).
   Not confirmed (left [research, unconfirmed] in the spec): a bought
   weapon not readied, a held weapon offered, broken weapons going back
   into stock, stores on the world map. The first three were confirmed
   next (rr12, below).
   *Weapons in and out of stock* (2026-11-14; stores 2.5, 4.4, 5.2).
   `ACSVM-research/capture/runs/rr12/`: `all.sh` builds the cases
   (`cases.txt`, `case.sh` as rr8's), `rounds.sh` runs the combat ones,
   `buy.txt` and `sell.txt` the store ones. The combat cases are rr8's
   `once` and rr9's `mail-parry`, `bow-break` and `bow-once` with a store
   added on 2,2 (no stock), 16 runs each (`run mem`, as rr8), watching the
   stock bytes of the kit as well (`$E075` bit 0 the mail, thing 40;
   `$E076` bit 7 the bow, 41; `$E078` bit 5 the blade, 59). The stock
   bytes, like the others watched, sometimes read other values for one
   20k sample and then return (seen in every case, before and after a
   change); only values that stay are counted.
   - `once`: EE's blade disappeared in 13 runs, and in each its bit went on
     in the stock in the same 20k sample; in the 3 runs it stayed, the
     stock stayed empty. `mail-parry`: T's mail broke in 3 runs (`$F5B4`
     40 to 0) and joined the stock each time. `bow-break`: EE's bow broke
     in 3 runs and joined the stock each time (stores 2.5).
   - `bow-once`: **never got into the room.** Walking in from the map
     stopped on "THE ACTION MOVES TO TEST REGION" and stayed there 33
     million cycles later (`enter-long.txt`), twice built, with the store
     on 2,2 or 2,7. The same case without the store (`bo-nostore`)
     loaded, as rr9's did; with the bow breakable instead (`bow-break`) it
     loaded with the store. The two disks differ only in the bow's break
     chance and disappear bit (`acs dump`). Not understood; a question
     for the researcher. Its inputs (stores brief, pass 2):
     `ACSVM-research/capture/runs/rr16/`, `near.txt` from each case's
     map `start.vsf` (`bow-once`, `bo-nostore`, `bow-break`): the stick
     south 800k cycles and released leaves T one square north of the
     door (`near.vsf`); a push of 1M steps onto it (`entering.vsf`).
     3M cycles later the two that load show "ACCESSING DISK", the
     hanging one never does.
   - `buy-blade` (`buy.txt`): T holding nothing, gold 500, the blade
     poked into the stock (`$E078` = 32). From 7,5 east to 13,5, north to
     13,2, west bumps the blade on 12,2 ("purchase"), the button: "the
     storekeeper sells the BLADE to T", gold 490, the blade owned
     (`$F5C1` bit 5), the stock empty, **the hand still empty** (`$F5B3`
     0). The button continues; south twice and east into the exit on
     14,4, the button: back on 5,4 with 1 move, the hand still empty
     (stores 4.4).
   - `sell-held` (`sell.txt`): T holding the blade (`$F5B3` = 59), gold
     500, Wisdom 13. The button at the store's first prompt: "T has 1
     BLADE", "sell it for 4"; the button: gold 504, **the hand empty**,
     the blade no longer owned, and in the stock (stores 5.2).
18. **Confirming the characters first pass** (2026-11-14;
   `docs/research/characters.md`, `docs/rules/characters.md`).
   `ACSVM-research/capture/runs/rr13/`, from the brief's inputs.
   - `create.sh`: `run mem` from `create-menu-nodisk.vsf` (the start-up
     add-player screen, its disk in drive 8), `--set 7161=1` (the cursor
     on "create new player"), the random state set to each of its 256
     values, a 20k press of the button, every draw and the first slot's
     record (`$F59F`, 23 bytes) logged for 400k instructions.
     `scripts/charcheck.py` reads each draw (the program stores the state
     twice per draw, XORed with the raster line and then the roll; the
     second is checked against the generator), rolls the dice and the
     second chance from them, and compares the record: **256 of 256
     exact**, 99 with the second chance and 72 with a die drawn again,
     every draw the generator's. One run rolled Dexterity 3 under a
     larger Size: Dodge **254**, the byte wrapping (characters 1.4). A
     first try pressed the button for 100k instructions: in 5 runs the
     press seemed to run on into the next screens and the slot was
     overwritten after it was right; 20k fixed it.
   - `join.sh`, `jointime.sh`: retired players of ours (Power 20 under
     Wisdom 13, 10 under 20, 31 under 31; Life Force 5 under Constitution
     20; Power 5 under Wisdom 30) started by `start-retired.txt` with
     peeks added: on the add-player screen and after the save, Power and
     Life Force are the record's; from then on the round counter goes up
     about once per 10M cycles while the first turn's moves drain, Power
     moving 1 a round toward Wisdom (20 to 13 in 7 rounds, 5 to 19 in 14)
     and Life Force healing (5 to 20 by round 11). The 160M-cycle wait
     before 8.4's first look is the whole of 8.4's "rule" (characters
     3.1, magic 5.1).
   - `profile.txt` from `move-basic-start.vsf`: T's profile (`prof/`)
     shows the **down mark** on Wisdom 4 and Power 4, the up mark on
     Constitution and Life Force 17, none on the other 10, as the bounds
     say (characters 4.1).
   - `names.sh` (`names.py` writes each script): "create new player"
     (`poke 7161 1`; `poke 3D5A 137 87`, the disk's identity, decimal
     bytes), the joystick, the first picture, then a name typed, the
     button, and the name bytes (`$F633`, 10) read at the first turn.
     Keys fed to the keyboard buffer are sometimes lost, the same ones in
     every repeat (the emulation is deterministic), at any pause between
     them (1M to 6M cycles tried). New script command `type TEXT` feeds a
     key and waits up to 2M cycles for the screen to change, feeding it
     again up to 6 times; every key changed the screen at once, yet some
     still never showed in the name, so a lost key changes the screen too.
     What did land decodes exactly: 3 symbols to a pair of bytes, low
     byte first (first x 1,600 + second x 40 + third), A to Z 1 to 26, 1 to
     9 27 to 35, `.` 36, `,` 37, `-` 38, `'` 39, space 0; 0 kept as O; `!`
     a period; `+ / ; =` hyphens; `*` an X; `[ ]` spaces (characters 5).
     `? : @ "` left nothing, refused or lost. A name of 17 letters ended
     with garbled glyphs in the strip and no name at `$F633`: not settled.
   - `ad-full`, `ad-three`: from `move-basic-start.vsf`, slots 2 to 4 (or
     2 and 3) poked with Constitution 10, Life Force 10, Speed 6 and the
     square 12,12 (the record offsets above), then ADD SOMEONE as
     `add-second.txt`: both said "I will add a player as soon as
     possible"; at the turn's end the program offered to restore the last
     saved game and resurrect a dead character: the poked characters had
     died. A real fifth player needs four real ones (characters 6).
19. **Map encounters, the research brief's inputs** (2026-11-14;
   `docs/research/encounters.md`). `ACSVM-research/capture/runs/rr14/`.
   New scenario options: `--map-creature` (repeatable, world map random
   creature slots 0, 1, ..: the `--creature` keys plus `terrain` and
   `chance`, the stored code 0..15, default 15 = 60%; every other slot is
   cleared, the template's included) and `--patch X0,Y0,X1,Y1` (GRASS,
   terrain 5, open, a picture of sparse colour-2 dots). Map creature 0 is
   drawn as a 12 x 12 box (picture 71), the next ones solid.
   - `build.sh`: `field` (WOLF, an enemy holding our blade, on the field
     at 60%), `patch` (WOLF on a grass patch at 13..16, 10..12) and `two`
     (WOLF and HARE on the patch), each started to T's first turn by
     `start-retired.txt` (T Speed 6, holding the blade).
   - `walk.txt` from `patch`: one push east from 12,12 onto the grass
     brought the encounter area at once, bordered with the grass, a gap in
     its left side, T on the right and WOLF moving. Under the engine's
     choice (movement 7.1: the terrain stood on before the move, here the
     field, which has no creature) no roll could have hit: the terrain
     entered counts, or the roll follows the step. One run.
   - `walk5.txt` from `field`: five pushes east; the first an ordinary
     step, the area showing after the second (no border: the field's
     picture is blank).
20. **Confirming the encounters first pass** (2026-11-14;
   `docs/research/encounters.md`, `docs/rules/encounters.md`).
   `ACSVM-research/capture/runs/rr15/`, from rr14's three scenarios.
   - `sweep.sh`: `run mem` from each `start.vsf`, the random state set to
     each of its 256 values, a 300k push east (one or two steps), every
     draw logged and the section 9 encounter bytes watched (T's place and
     square, the moves left, area 1's room record and entrance, map slots
     8 and 9) for 3M instructions. `scripts/encountercheck.py` takes each
     map step's draws (the burst right after the square changes, every
     draw the generator's), rolls one percentile for each record on the
     entered terrain in record order (raw // 2, drawn again at 200 or
     more), and checks that the area opens exactly when one hits, and how.
     **`patch`** (WOLF on the grass at 60%): 256 of 256 runs exact, 354
     steps, 219 encounters, one roll per grass step, the redraws as said,
     and a roll of exactly 60 missing. **`field`** (WOLF on the field):
     256 of 256, **8 rolls on every field step, WOLF's first** (the 7
     empty records on terrain 0, rolled against 0). **`two`** (WOLF and
     HARE on the grass): 256 of 256: both hit in 87 runs (slots 8 and 9,
     on 2,3 and 3,3), WOLF alone in 65 (slot 8 on 2,3; first written here
     as 40, corrected in 10.36), HARE alone in 59 (**slot 8 on 3,3**, its
     record's square), none in 45, every run one grass step. In every encounter the
     entrance was the square stepped onto (13,12 or 14,12), T on 12,4 in
     area 1 (place 1), the moves 0, area 1's size byte 233 and its wall
     picture the grass's, and no draw between the rolls and the turn's end
     (encounters 2.1, 2.5, 2.6, 2.7, 3.1 to 3.3, 3.6, 4.1, 5.1).
   - `north`: the same from `patch` with pushes south, east and north (60k
     each, 1M apart), entering the grass from 13,13 northward: 56 of 56
     runs exact (8 lost to a VICE job that refused its connection), 168
     steps, the field steps drawing 7 rolls each (the empty records), and
     **12,4 again** in all 39 encounters (3.6).
   - `walkout.txt` (`wo/`): from `patch`, the state poked to 0 (the first
     roll hits), one push east; at T's next turn (6 moves) T poked onto
     1,5 and one push west through the door: **T on 13,12, the entrance,
     with 5 moves**, the place 0, area 1's size 0, its entrance 41,41 and
     slot 8 free (encounters 7.1, 7.2). `in.png` shows the area walled
     with the grass's dots and the gap on the left at row 5.
   Not run: a second player joining, a death in an area, all four areas
   or all eight slots in use, a creature through the door, save and
   retire inside.
21. **Confirming the stores second pass** (2026-11-14;
   `docs/research/stores.md` pass 2). `ACSVM-research/capture/runs/rr17/`.
   New in the harness: the drive's own memory (`dpeek ADDR [N]`, memory
   space 1) and `drivecheck [SAMPLES [GAP]]`, which reads drive 8's
   timer-interrupt flags (`$1C0D` there, section 9) 30 times 500
   instructions apart and calls the drive dead when bit 6 never clears.
   `run` and `snap` now restore their snapshot once before the first run
   and check the drive, warning on stderr when it is dead; the runs
   restore again, so their output is unchanged (2 `run mem` runs from
   rr12's `bow-break-turn-end.vsf` byte for byte as rr12's). `--reset-drive`
   resets drive 8 after every restore.
   - `scan.sh`: every saved snapshot of rr12 and rr16 (60), restored and
     checked. **Dead in 10, all of them from the two store-and-disappearing-
     bow disks** (rr12 `bow-once`: start, room, room-long, turn-end;
     `bo-store-far`: start, room twice; rr16 `bow-once`: start, near,
     entering): the flag set in 30 of 30 reads. Alive in the other 50,
     `bo-nostore` and `bow-break` included (clear in 29 or 30 of 30). The
     drive's job code (`$0003`) is 128, a read never started, in the 6
     dead snapshots taken after the map door, 1 in the 4 before it.
   - `noreset.txt`, `reset.txt` from rr16 `bow-once/near.vsf`: without a
     reset the drive stays dead and the push onto the door gives "THE
     ACTION MOVES TO TEST REGION" and nothing else for 24M instructions
     (job code 128, visited region `$397C` still 0). With `reset 8` first
     (2M to settle) the drive is alive and the room appears about 10M
     instructions after the push, the store on 2,2 drawn, `$397C` 1, and
     27M in EE has already fired: the stock's bow bit (`$E076` bit 7) on.
   - `fresh.txt`: a new disk from rr12's `bow-once` keys, then
     `start-retired.txt` without its dump and `enter-room.txt`'s walk in
     the same session: the drive alive at the first turn and in the room,
     and **the room loads** (`$397C` 1). rr12's `start.vsf`, saved by the
     same chain at that first turn, restores with a dead drive. So the
     hang is the restored drive, not the adventure (stores research 7.1 to
     7.3).
   - `rounds.sh`: turn-end snapshots from the reset room
     (`bow-once-turn-end.vsf`) and the fresh room (`fresh-turn-end.vsf`),
     16 runs each as rr12's (`run mem --set-random-state --draws`, EE's
     weapon `$E1B7` and the stock bytes watched, 16M instructions, three
     VICE processes). `scripts/stockcheck.py` (new) finds, run by run, when
     the thing left its holder's byte and when its stock bit went on (a
     value counted once it stays two samples): **the bow went in 32 of 32
     runs and joined the stock in every one**, in the same 20k sample or
     the next (stores 2.5, research 7.4). `combatcheck.py --case
     bow-once`: 16 of 16 right from each (16 shots each). `stockcheck.py`
     on rr12's runs gives rr12's counts again: `bow-break` 16 of 16 (3
     gone), `once` with `--thing 59` 16 of 16 (13), `mail-parry` with
     `--thing 40 --holder F5B4` 16 of 16 (3).
   - Stores on the world map (research 8): the harness can't put a store
     on a map square or in an encounter area's item list (our scenario
     writer has no thing on the map, and the area's item list has no
     address in section 9), so a store there is not reached. What it can
     read: `mapstock.txt` and `mappeek.txt` peek the stock bytes `$E06D`
     to `$E07A` with the map the visited region: **255 in all 14 bytes**
     at the first turn of rr14's `patch`, `field` and `two` and in rr16's
     `bow-once/near.vsf`, inside encounter area 1 (`patch`, state 0, one
     push east, place 1) and after walking out, as the researcher saw
     (research 8.3: every thing the store offers would be on its shelves).
     In the hung `noreset` run the same bytes read other values once the
     region change had begun.
22. **Confirming the turn-order second pass** (2026-11-14;
   `docs/research/turn-order.md` pass 2). `ACSVM-research/capture/runs/rr18/`.
   New in `run mem`: `--death-f1` answers the death message (read as
   text in the message's own colour: the one offering F1) with F1, fed
   again if it is still up 2 million instructions later; `--answer-text
   W1,W2` presses the button for any message holding one of the words,
   whoever the strip names; `--ram` reads the watched bytes from the
   `ram` bank: without it a byte under the KERNAL ROM (`$F65B`, the
   player records) reads the ROM's byte (the counter's is 3) whenever the
   program has the ROM banked in, for up to 1.5M instructions while a
   region loads. With `--answer`, an `end{run}.png` at the run's end.
   `scripts/roundcheck.py` (new) rebuilds each run's turns from the
   region, counter and current individual (a turn starts when the
   individual is set, from nobody or from another; on the map a new
   round with the same one current counts too), prints each round's list
   and checks: no individual 5 to 15 on a room region's list, no player
   left out of a room round between two it has turns in, and every death
   message after the counter went up. All runs below pass. A `mem` run
   has no jitter: each case is one run.
   - **Item 1** (`rooms/run.sh`, from rr4's `rooms2/two.vsf`; watched:
     region, counter, individual, T's and U's places, the moves left
     `$719A`). `held` (south held 40M): U in through the map door in
     round 11, T in round 12 after 4 of its 12 squares (moves left 8,
     then 0 at the door, the individual 0); the room region from round 13,
     **T (1) its first mover** at about 12.2M, then U, AA, BB (2, 16, 17),
     and T, U, AA, BB in rounds 14 and 15. `tfirst` (U's turn idled out,
     then 1M pushes): T in, then U, both in round 12; round 13 on went T,
     U, AA, BB, to round 17. `ulater`: U in round 11, T alone on the map
     in rounds 12 to 14 and in during round 14 (the map visit's second
     round of a new cycle, which its leaving ended); round 15 on went T,
     U, AA, BB, to round 18, U having no turns in rounds 12 to 14 (its
     region not marked in that cycle). So no round was missed in any of
     the three, either order, entry in round 12 or 14. Seen besides: the
     first room turn after the map door starts with the moves left at
     the door (T: 8 in `held` and `tfirst`, 1 in `ulater`), and with the
     full Speed when none were left (U: 4 after its 4 squares). `late30`
     and `late60` (south held from 30M and 60M) moved nobody in: a
     player whose turn began with the stick already held stood still;
     dropped.
   - **Item 2** (watched: region, counter, individual, T's and U's Life
     Force, alive bytes `$F59F`, `$F5C4`, places `$F5AF`, `$F5D4`, the
     queue's first entry `$F65F`). `death/f1.sh`, rr16's death disk
     answered with F1: `f1-two` (both in): EE killed U in round 22; round
     23 went on in region 1 with T, EE (no U); EE killed T; the second
     message, F1, and **the add-a-player screen** (`end0.png`; T's place
     went to 0 then). `f1-split` (T in, U on the map): EE killed T in
     round 40, region 1's first; F1; **U alone on the map every round
     from 41 to 76**, region 1 never visited again. `after/` is the same
     disk with FF (a peaceful friend, slot 1) on 12,7 behind EE: in
     `f1`, EE killed U (alive byte 0 at 22.10M) and **FF still had its
     turn** (22.54M) before the counter went to 23 (23.74M) and the
     message came (23.76M); round 23 went T, EE, FF; EE killed T and FF
     again had its turn before the counter went to 24 and the message.
     In all 5 death messages the counter had gone up 20,000 to 40,000
     instructions before; 3 came with a living player left (U's death
     with T in the room, twice; T's with U on the map), 2 with nobody. The queue's first entry read 0 throughout (nothing
     queued), so the queue's carrying on isn't shown. `button` and
     `button2` (`--answer-text CONTROL,CONTINUE`, through the "one of the
     adventurers is" and "press button of the joystick that will control
     this player" screens): the counter went to 23, the region to 255,
     then the counter back to 22 and U's Life Force and alive byte back
     to 40; about 1M instructions after the last screen region 1 was
     visited again from round 22, the same round replayed (T, U, EE
     killing U, FF), 8 times in 150M instructions, identically. rr16's
     four button runs (10.10.4's death inputs) agree up to those screens.
     Not reached: a player killed before its own turn in the same round
     (no list of ours has a creature before a player of its room, and our
     players don't fight each other).
   - **Item 3**: no room region's list in any of these 10 runs held an
     individual 5 to 15 (players 1 and 2, creatures 16 and 17); nothing of
     ours is numbered 5 to 15 in a room region, so the finding's reading
     can't be told from the first pass's by these runs. Side run
     (`area.sh`, rr14's `patch`, the random state 0, one push east into
     encounter area 1): the map region's rounds 15 to 17 went **T, then 8**
     (the WOLF copy, map slot 8), as encounters 6.1 says; the finding's
     remark that the map's numbers 8 to 15 are not used for turns
     doesn't hold for an area's creatures.
23. **Confirming the characters second pass** (2026-11-14;
   `docs/research/characters.md` pass 2). `ACSVM-research/capture/runs/rr19/`,
   from the brief's `inputs/pass2/` (copied to `in/`). Scripts only, one
   VICE each, at most four at once. New script commands: `peek ADDR N`
   (N bytes in decimal), `until ADDR =V|!V [LIMIT [STEP]]` and `typemem
   ADDR TEXT` (each key fed again, up to 8 times 1M apart, until the byte
   at ADDR changes). The snapshots saved with disks were first saved again
   without them (`nd-*/nodisk.vsf`), so that writes reach a disk file
   (2.5). A peek while the program is at the disk reads the ROM; a peek of
   `$F65B` read 3 for a moment at two rounds' ends.
   - **Retire, when** (item 1). Map (`ret1`, from `retire-menu.vsf`): T
     asked in round 21; U moved (mover `$71FA` 2), the counter went to 22
     and the save-or-destroy offer showed under U's strip, T having had no
     turn between. Room with both players (`walk-two` walked T and U in:
     the stick is shared; `room1`): asked in round 22, U moved, the offer
     at 23. Room, T the last player, with AA (`walk-late`, `last1`): asked
     in round 40, AA (16) moved, the offer at 41. So at the end of the
     round it was asked in, map or room, not at a region visit's end
     (3 runs).
   - **Save**. `ret-save`, `ret-save2` from `ret1/offer.vsf`: T's Life
     Force poked to 3 (then 4) under Constitution 11 and Power to 5 under
     Wisdom 13, and in `ret-save2` gold 777 and treasure 14 taken away
     first. The roster's first entry with an empty name (entry 1; U's
     stayed) decoded (`acs dump`) to Life Force **11**, Power **5**, gold
     777, the blade held, the mail worn, possessions 13, 40, 41, 59: the
     whole record, Life Force raised to Constitution. In memory only T's
     first byte went to 0; on the disk adventure block + 0x2B 0 and
     **+ 0xE7 22** (the round). U then had every turn (rounds 23 to 47).
     As the last player (`last-save`): the game saved and the add-player
     screen came back with T listed (the cursor on T, `$7161` 3); the
     roster record kept the room square (region 1, 4,4) and the kit.
   - **Destroy**. F1 shows "this means that T will never be seen again",
     the button brings the offer back, F1 again destroys. Map
     (`ret-dest`): T's slot unchanged but its first byte, the disk byte
     for byte unchanged, U on every turn after. Last player in the room
     (`last-dest`): the add-player screen without T, the disk unchanged.
     **In a room with U** (`room-dest2`, and `rd-0`, `rd-64`, `rd-128`,
     `rd-none` with the random state `$1CC9` set before the last F1): U
     stepped off and pushed back onto T's square, pressing the button at
     each "U takes": **blade, bow, mail, GOODB, GOODA and 300 gold, in
     each of the 5 runs**, then an empty square. **As the last player**
     (`newc`: a new player N created at the add-player screen after
     `last-dest`, walked into the room and pushed onto 4,4): **the bow,
     the mail and 300 gold**; no blade, no treasure. The finding (gold,
     blade and bow; mail and treasures lost) matches neither. Whether the
     newcomer's save and the room's reload changed the floor in `newc` is
     not known.
   - **Another adventure's player** (item 2), `ota-*` then `otb-*` from
     `create-menu-nodisk.vsf` with `other-disk.d64` (V, 1,500 gold) or
     `other300.d64` (the same V with 300, built as the brief's) in drive 8,
     `poke 3D5A 137 87`, cursor 0. The flow: "you may use the back side of
     the ACS disk", button, **"put the disk from which to copy in the disk
     drive"**, button, "accessing disk", the list with V (the cursor back
     on 0; one tap down puts it on V, `$7161` 3), button, the joystick, the
     picture, "put your adventure disk in", the swap (`attach`), the save.
     With the picture chosen V's slot read Constitution 20, magic defense
     1, Life Force 9, Wisdom 20, Power 8, square 63,64 (not placed), **no
     weapon, no armor, no possessions, gold 1,500** (`otb-1500`) and
     **1,000** from 300 (`otb-300`). Joined, V stood on 12,12 of the map,
     and the saved adventure (`otb-1500/adv.d64`) has V with 1,500. The
     other disk changed in exactly 2 bytes, V's roster name, in all 3
     runs. The swap by `attach` came too late in 2 of 4 runs (the save
     went to the other disk, or the program kept asking): 2.4's
     nondeterminism. **Identity**: the never-played `other-disk.d64` and
     `retire-pristine.d64` both carry 13,151 (our played disks 137,87).
     With `retire-disk.d64` (137,87, the adventure's own, U in its roster)
     as the "other" disk (`ota-ownid`), U was listed in the second column
     and the cursor put on CREATE NEW PLAYER (1), not back on 0 as with
     every other disk. But poking `$3D5A` to the other disk's own 13,151
     (`ota-same`, `otb-same`) changed nothing: V listed and joined as
     before. So the disk's identity is compared with the adventure's, as
     found, but `$3D5A` as poked here is not what decides.
   - **Name entry** (item 3), `name-*` from `name-entry.vsf`, `typemem
     4F4F`: 72 keys in 5 runs, **every one taken** (1 to 5 feeds; the
     cursor 0 to 14 and round to 0), none refused, `$3789` and the stored
     name (`$F633`, decoded as 10.18) alike: `A?B:C@D"EFGHIJKLMNOPQ` gave
     LMNOPQD'EFGHIJK; `0123456789!?.+-` O123456789...--; slash, colon,
     semicolon, equals, comma, apostrophe, double quote, asterisk,
     ampersand, pound, up-arrow, left-arrow, `@`, number sign and `$`
     gave ----,''XALIK and three spaces; `%()<>[]` and space all spaces.
     17 letters A to Q kept PQCDEFGHIJKLMNO and the strip was clean (10.18's
     garbling did not come back). A name of 8 spaces then ABCDEFG was
     asked for again. The control `name-plain` (9 keys fed once, 3M
     apart): C, E, G and H lost.
   - **How many players** (item 4), from `move-basic-start.vsf`, ADD
     SOMEONE in round 4: `four-full` (the first pass's pokes) and
     `four-first` (**only the three first bytes**, Constitution 10): at
     the round's end (counter 5) the restore-or-continue offer; F1 (fed
     until taken, `type`), then at once "you will have to retire one of
     your adventurers before you may add another". `four-rest` (every poke
     but the first bytes): no offer, and the add-player screen at the
     round's end. After F1 in `four-full-on` the turns went 1, 2, 3, 4
     every round (rounds 6 to 12, 40 samples); in `four-first-on` slot 2
     (Speed 0, square 0,0) held the turn for all 400M instructions. An F1
     fed once at the offer was lost, and the button then restored the
     saved game.
   - **Catching up** (item 5), `catch-B` from `walk-late/room.vsf` (T
     just in the room at round 40, AA not yet moved): AA's byte + 12
     (`$E1AF`) set to B, AA read after its first turn (round 41). With
     N = 40 - B (mod 256) the rule gives Life Force 5 + min(N / 2, 15)
     and Power 2 + min(N - 1, 18), plus the turn's own 1 each (to the
     cap of 20):

     | B | 39 | 38 | 30 | 24 | 10 | 200 | 0 |
     | --- | --- | --- | --- | --- | --- | --- | --- |
     | Life Force, Power | 6, 3 | 7, 4 | 11, 12 | 14, 18 | 20, 20 | 20, 20 | 7, 3 |

     6 of 6 as the rule gives (B = 200 the wrap, 96 rounds), and B = 0 no
     catch-up (Life Force 7: one more healed than at B = 39); the byte was
     40 after the turn every time. A real leave-and-return was not run:
     our scenario room has no way out, and nothing in the harness takes a
     player out of a region.
24. **Confirming the combat fourth pass** (2026-11-14;
   `docs/research/combat.md` pass 4). `ACSVM-research/capture/runs/rr20/`
   (the harness binary copied to `bin/`; `case.sh` builds the cases in
   `cases.txt`, `sweep.sh` items 2 and 3, `fire.sh` item 1's sweeps and
   `drop-thing`, `queue.sh` runs `fire.sh` after `sweep.sh`). No harness
   changes. Sweeps are `run mem --set-random-state --draws --ram` from a
   turn-end or cursor snapshot, two VICE processes; `scripts/dropcheck.py`
   (new) and `scripts/combatcheck.py` (the `still-*` cases, `--still-rule
   pass4|fit`, and the FIRE cases `fire-e5`, `fire-e3`, `fire-e0`,
   `ready-e5`) check them draw by draw.
   - **Item 1, the player's FIRE** (scripts, one run each, from rr9's
     `player-bow` and rr16's room snapshots: `fire1/`, `beside/`, `none/`,
     `own/`, `map/`, `ready5/`, `ready5b/`). FIRE is the third row's first
     entry (two pushes down from MOVE); it brings a cursor on T's square,
     moved a square a push; the button there fired **3 shots from 5 moves**
     (the shots-left byte `$B799` counting down), the moves then 0. With
     the foe beside (`player-bow-beside`, after the push south) the "may
     not use a missile weapon while next to" message, **no move used**;
     the stick into a creature, bow held, no melee weapon owned: nothing
     (no move, no message). Holding no missile weapon (`player-none`):
     **one move used** at once, "select a missile weapon", the menu, NONE
     ends it. Owning the blade, bow held (`player-bow-own`): the push into
     EE brought the melee ready menu (the bow not listed). On the world
     map FIRE did nothing. **Disagreed**: choosing the owned bow from the
     FIRE menu (`player-bow-ready`: T owns the bow, holds nothing) cost a
     **second** move (two in all with FIRE's own) and the shots were then
     counted from 3 moves, not from the 4 the finding traced. Cursor
     snapshots `fire1/fire-e5.vsf`, `fire-e3.vsf`, `fire-e0.vsf` (cursor
     on EE, on 7,4, on T's own square) and `ready5/ready5-e5.vsf` are the
     starts of the sweeps `fire-e5`, `fire-e3`, `fire-e0` (64, 64, 64 runs),
     `ready-e5` (32) and the draws-logged refusals `beside-draws`,
     `none-draws`, `map-draws` (4 each), from `fire.sh`. `combatcheck.py`
     runs the shots as a creature's (combat 8, from the run's first draw,
     the shooter T) and checks EE's Life Force, Power and Dodge, T's
     Missile skill, every change in order, and each shot's first draw
     against the shots-left byte's write. All strict, **no tolerance
     widened**: `fire-e5` (cursor on EE) **64/64** (192 shots, 16 damaging
     arrows), `fire-e3` (cursor on 7,4: the arrow flies on past it)
     **64/64** (192 shots, 20 damaging), `fire-e0` (cursor left on T's own
     square, aimed one south) **64/64** (192 shots, every arrow stopped),
     `ready-e5` (the bow readied from the FIRE menu first) **32/32** with
     **2 shots** a run (Dexterity 13 / 8 = 1, plus 3 moves / 2 = 1: the
     second move, above). One draw the finding doesn't name: **a roll of
     0 to 9 after every damaging arrow on EE** (combat 7's roll for an
     enemy the player damages), the second hit's included (44 arrows).
     Without it 12 and 12 runs of `fire-e5` and `fire-e3` fail, the next
     shot's draws one late; taking it only while EE is still an
     aggressive enemy (as the player's blows do) leaves 4 failing, the
     third shot's draws then one early. EE's class byte isn't watched, so
     what that roll decides isn't checked. The refusals, 4 runs each,
     alike whatever the random state: `beside-draws` (push south, FIRE
     with EE beside, the message, then east into EE): one draw in the run,
     an idle move (moves 4 to 3) before the push east, none at FIRE or at
     the push into EE (the blows-left byte `$B799` was written 2 then 0
     there, with no draw and no move); `none-draws` (FIRE, NONE): the move
     used at FIRE (5 to 4) with no draw, the run's one draw a later idle
     move; `map-draws` (FIRE on the map): one draw, an idle move, nothing
     at FIRE.
   - **Item 2, a Speed 0 creature's turn**. From rr16's turn-end
     snapshots, the pass 4 rule (the die, one smartness check for a
     friend, two for a fighter, a step roll of 0 to 2 for each open square
     beside, the step cancelled, the idle move's draw-free heal while below
     Constitution; an archer's one shot after its checks): `still-hurt`
     **256/256** (512 turns, 512 idle heals), `still-enemy` **256/256**,
     `still-whole` **64/64** (Life Force never above 40), `still-bow`
     **128/128** (128 shots, one a turn; 17 arrows hit T for damage, 12 of
     29 dodge rolls dodged, 2 stopped). rr16's 16 runs of each and rr8's
     `bow-line` (16) pass too. The control (`--still-rule fit`, 10.14.10's
     fitted sizes): `still-hurt` 204/256, `still-enemy` 59/256,
     `still-whole` 53/64, `still-bow` 0/128.
   - **Item 3, the drop square** (the room's list of things, section 9,
     watched from `$E583`; the dropper's possession or wealth byte going
     to 0 dates the drop between the last pair's draw and the next):
     `drop-wall` **256/256** (306 tries, 50 failed on the obstacle; landed
     6,3 51, 6,4 101, 6,5 52, 7,4 52), `drop-corner` **256/256** (393 tries,
     137 failed on the ring, up to 8 in a drop), `drop-beside` (WE set onto
     5,4, east of T) **128/128** (20 on T's square 4,4: an individual's
     square accepted), `burden` (rr5, gold) **256/256** (512 drops, the gold
     on the pair's square in the list, not always the dropper's own).
     `drop-thing` (WE on 6,4, treasures of ours on 7,4, 6,5 and 6,3: a
     square holding a thing) **256/256**: 125 landed on a square already
     holding a treasure (6,3 43, 6,5 42, 7,4 40), every try accepted.
25. **When the restored game is saved** (2026-11-14; ours, no research
   pass: creatures 2.2.6, magic 7.4, movement 6.6, and the planned
   captures of `docs/research/turn-order.md` "No third pass").
   `ACSVM-research/capture/runs/rr21/` (scripts `*.txt`, the generators
   `gen*.py`, `a.sh`, `b.sh`, `bsplit.sh`, `build-b.sh`; rr20's harness
   binary in `bin/`). No harness changes. Scripts build each state and
   dump it with disks (so the program reads its own saves back from the
   snapshot's disk); `run mem --ram --answer ... --answer-text
   CONTROL,CONTINUE` then lets a creature kill and presses the button at
   the death message. Markers: a player's gold (`$F5B1`, section 9) poked
   to a different value at each step, and EE's class (`$E1A7`) poked to a
   friend (182) at the first room turn and back to its aggressive 230
   before the kill. A `mem` run has no jitter: one run a case, and each
   restore below repeated itself identically while the run went on.
   `roundcheck.py`: all checks hold in the five `mem` runs.
   - **SAVE GAME** is the third row's last entry (two down, three right
     of MOVE). The button there: "I will save the game as soon as
     possible, please continue playing"; the button; "if you want to
     return to the ACS editor after the game is saved, press F1 now"; the
     button ends the turn (corrected in 10.28: the turn goes on, and the
     moves it had left ran out as idle moves). At the round's end (after FF's turn, the
     counter 25): "saving game", "the adventure has been saved", the
     button, then T's turn in round 25.
   - **(a) An explicit save mid-visit** (`walk24.txt`, `menu.txt`,
     `r25.txt` from rr18's `after/two.vsf`: both players walk in, the room
     region from round 22; T's gold 111 and SAVE GAME in round 24; gold
     222 and EE 230 at T's turn in round 25; `a-save`): EE killed U in
     round 25, the counter went to 26, the button: **the counter back to
     25, T's gold 111, U alive with Life Force 40, EE's class 182**, the
     state as saved at round 24's end; play went on from round 25 to 32
     (EE a friend, nobody died again). Control without the save
     (`nosave.txt`, the same pokes; `a-nosave`): EE killed U in round 26,
     the button: **the counter back to 22, the gold 0, T's Life Force 24,
     EE 230**, the state when the room region's visit began (as rr18's
     `button`), and round 22 replayed, U killed again, 10 restores in 200M
     instructions. So SAVE GAME writes the state the button restores.
   - **(b) One player alone in a room region over several visits**
     (`build-b.sh`: T alone, Constitution and Life Force 40, EE as rr16's
     killer, no U; `genb.py` writes `alone.txt`; `b-walk`): T walked in
     during round 13 on the map; the room region from round 14, `$397C` 1
     at every read to round 46 (T's turns at rounds 15 to 46 setting the
     gold 1 to 32, three visit ends passed). EE killed T in round 46 (no
     player left), the button (`b-button`): **the counter back to 14, the
     gold 0**, Life Force 40; round 14 replayed, T killed each time, 15
     restores in 200M. So visits of the same region one after another
     save nothing: the restore went back to the region's first round.
   - **(b) with U on the map** (`gensplit.py`, `split.txt`, `b-split`,
     from rr18's `after/two.vsf`, T alone south 3.2M as rr18's `f1-split`):
     region 1 was visited in rounds 40 to 49, 60 to 69 and 80 on, the map
     with U between; T's gold set to 1 to 25 at its turns (rounds 41 to 49,
     60 to 69, 80 to 85). EE killed T in round 85, the button
     (`b-split-button`): **the counter back to 80, T's gold 19** (the
     value it had when region 1's visit of round 80 began), T alive;
     then **the map first** (rounds 80 to 89, U alone), region 1 from
     round 90 (EE's class 182, the value poked during its visit of round
     40), the map from 100. So a visit that changes the region visited
     saves: the restore is the start of the last visit that began with a
     region change, or the last SAVE GAME, whichever came later (in the
     runs here; a save in the same round as a region change not tried),
     and the counter comes back as that visit's first round (14, 22, 80;
     25 for a save at round 24's end), not the round of the map door step.
     After the button the cycle is marked from the restored places,
     lowest region first.
   - **Declining to add a player with nobody alive** (`gendecline.py`,
     `decline.txt` from `b-walk/pre.vsf`, EE's messages answered at
     `b-f1`'s times; F1 at the death message, as `b-f1`): the add-a-player
     screen (the cursor `$7161` 1, on CREATE NEW PLAYER; at that moment
     **T's whole 37-byte record read 0**, its place and gold included).
     One tap east (`$7161` 2), the button (`decline2.txt`, twice): the
     screen went blank, came back with "accessing disk ... please wait"
     (the cursor on 3 for a moment), then the same add-a-player screen
     with the cursor back on 1, within about 2M instructions. The counter
     stayed 47, the region 1, the individual 0, for 150M instructions
     after the first decline (`decline`, read every 5M): **no turns, the
     screen comes back**, as when the last player retires (10.23). A
     peek of `$F65B` read 3 (the ROM) during the disk access; not a
     change.
26. **Encounter area item lists, a format find** (2026-11-14; ours, no
   research pass). A read-only look at the corpus found five room item
   lists at world map block + 0x810, in the room list format (a u16
   length counting itself, then 3-byte items: column x 16 + row, thing,
   parameter): room 0 (the open map) empty, rooms 1 to 4 (the encounter
   areas) each one item `05 52 n`, thing 82 on 0,5 with parameter n = 0..3
   ([encounters](rules/encounters.md) 3.5's door). Before them, at + 0x806,
   four words `00 41` .. `00 44`, and at + 0x80E a u16 0x0832 (meanings
   unknown). Our template holds the same bytes. New scenario option
   `--area-list N:X,Y,THING,PARAM/..` (repeatable) rewrites area N's list
   in the map block (`scenario::area_lists`; things 42..49 become
   treasures of ours drawn with the half marker picture, 83 the store);
   the u16 at + 0x80E is left as the template has it.
   `ACSVM-research/capture/runs/rr22/` (the harness binary in `bin/`):
   `build.sh` builds rr14's `patch` scenario (WOLF on the grass at 60 %,
   T Speed 6) with seven lists and starts each to T's first turn;
   `run.sh CASE DIR X Y D` runs `in.txt`: the random state poked to 0, 1,
   .. until one push east opens area 1, the map block's tail read from
   `$E7F2` on (area 1's room record, section 9; the bytes that follow
   are the disk's + 0x7F2 on, so the lists sit at `$E810` on), then at
   T's next turn T poked onto X,Y and a push D. One run a case
   (deterministic once the area opens).
   - `ctrl` (template list): the gap at row 5, out from 1,5 onto the
     entrance 13,12 with 5 moves (10.20 again). The room's list pointer
     `$382A` (section 9) **`$E814`**, area 1's items, in every case.
   - `treasure` (`1:0,5,82,0/7,5,42,1`): **the treasure shows on 7,5**
     and the list in memory holds both items; a push west from 8,5 brought
     "T takes AREATHINGA" (`take/`). Leaving closed the area and **marked
     the treasure's entry deleted** (its square byte 255, the length kept
     at 8), the door's entry untouched (encounters 7.2: the items deleted,
     the door kept).
   - `two` (area 1 as `treasure`, area 2 `0,5,82,1/8,6,43,1`): area 1
     shows only its own treasure; area 2's list stays as written while
     area 1 is open and after it closes. The lists are rooms 1..4 in order.
   - `store` (`1:0,5,82,0/7,3,83,0`): the store shows on 7,3; stepping
     onto it from 8,3 opened the **store screen** (`shop/`, "T has 0 to
     spend", T on 7,5): two shelf things drawn (on 12,2 and 8,6, not
     identified), the stock bytes `$E06D`-`$E07A` all 255 meanwhile.
   - `moved` (`1:0,3,82,0`): **the gap moves to row 3** (the wall's dots
     on every left-wall row but 3); out from 1,3 west onto the entrance
     13,12 with 5 moves (`out3/`); a push west from 1,5 is refused, no
     move used (`out5/`).
   - `noexit` (`1:`): **no gap**; a push west from 1,5 refused, no move
     used. The door is the list's item, not a fixed square.
   - `param` (`1:0,5,82,1`): out from 1,5 west, T came back **on 12,12,
     the map's start square** (the entrance was 14,12); with area 2's
     entrance poked to 15,11 first (`e2/`, `param2.txt`), **T came back on
     15,11**. The parameter picks the area whose entrance the door leads to
     (n: area n + 1, the words `00 41`.. at + 0x806 the likely table); an
     entrance of 41,41 (free) sends the player to the map's start. Area 1
     closed either way.
   Lists 3 and 6 bytes longer than the template's (the u16 at + 0x80E
   unchanged) caused no visible trouble in these runs.
27. **Catching up when no round has passed** (2026-11-14; ours, no research
   pass; [magic](rules/magic.md) 5.5). `ACSVM-research/capture/runs/rr23/`
   (rr19's harness binary in `bin/`, rr19's `walk-late/room.vsf` and its
   disk in `in/`). `catch.txt` is 10.23's `catch-B` with the random state
   `$1CC9` poked first; `run.sh` and `sweep.sh` run it for each byte B and
   state, one VICE at a time; `summ.sh` lists the results. AA's turn comes
   in round 40 (the counter read 40 before it and 41 after), so B = 40
   gives N = 0, B = 39 N = 1 (the control) and B = 41 N = 255 (the wrap).
   Before the turn AA has Life Force 5 (Constitution 20) and Power 2.

   | B (N) | runs | Life Force, Power after |
   | --- | --- | --- |
   | 40 (0) | 20 (`catch-40-*`) | 6, 3 (18); 7, 3 (2) |
   | 39 (1) | 20 (`catch-39-*`) | 6, 3 (16); 7, 3 (4) |
   | 0 (none) | 17 (`catch-0-*`) | 6, 3 (16); 7, 3 (1) |
   | 41 (255) | 4 (`catch-41-*`) | 20, 20 (4) |

   States 0, 85, 170, 255 and 8, 24, .. 248 (B = 0: the sweep only). So
   **N = 0 gains nothing**: Power 3 (2 and the turn's 1) in all 20, where a
   wrapped N - 1 = 255 would give 20, as B = 41 does. The byte was 40
   after every turn. The extra point of Life Force goes with the random
   state, not with B = 0 as 10.23 read it (rr19's `catch-0` ran at the
   snapshot's own state), and the states that give it differ between
   B = 0 (40), 39 (168, 184, 200, 248) and 40 (85, 136); the 9 repeats
   (`again-*`) gave the same. Why the byte moves a later roll is not known.
28. **SAVE GAME, Retire with Save and the restore point, the open cases**
   (2026-11-14; ours, no research pass; [creatures](rules/creatures.md)
   2.2.7 and "Captures needed" 1, [characters](rules/characters.md)
   "Captures needed" 6). `ACSVM-research/capture/runs/rr24/` (the harness
   binary in `bin/`, `env.sh`; scripts `c1.txt`, `c1n.txt`, `c2.txt`,
   `genc3.py` writing `c3.txt`, `c3b.txt`; `chg.py` lists a `mem` run's
   changes, answers and draws in time order, `c4.py` the draws per idle
   move). From rr21's states (10.25): `b-walk/pre.vsf` (T alone in room
   region 1 since round 14, T's turn in round 46, EE an enemy beside it),
   `walk24/t24.vsf` (T and U in the room since round 22, T's turn in round
   24), rr18's `after/two.vsf` with rr21's split route. New in `run mem`:
   `--end-dump` saves `end{run}.vsf` with disks at the run's end, to go on
   with a script. Markers as 10.25 (gold, EE's class). A `mem` run has no
   jitter: one run a case, every restore of a run the same.
   - **SAVE GAME does not end the turn.** After the menu's SAVE GAME and
     its two buttons T's prompt ("move with the joystick ...") came back
     with its moves left (4 of 6), in both states tried (`c1`, `c3`). With
     the stick pushed south from there (`c4-push`) T walked 4 squares, 4,4
     to 4,8; left alone, the 4 moves went one by one as idle moves (about
     1.85M instructions each). The save is still made at the round's end
     ("saving game", "the adventure has been saved", the button). 10.25's
     "the button ends the turn" was this drain.
   - **Item 4, its moves' draws** (`c4-save`: from `c1/asked.vsf`, T's
     Life Force set to 10 under Constitution 40, the random state 0..7,
     8 runs, `--draws`; control `c4-idle`: the same from `pre.vsf` with no
     SAVE GAME, 8 runs). **One draw each idle move** in both (32 and 40
     moves; one run's first move two), with Life Force +1 at 4 of the 32
     and 7 of the 40: SAVE GAME's moves left are ordinary idle moves and
     take the healing roll. At full Life Force (`c1-button`, 40 of 40) the
     4 idle moves drew one each as well.
   - **Item 1, SAVE GAME in the round the last player dies** (`c1.txt`:
     at T's turn in round 46 gold 77, SAVE GAME; EE killed T in round 46).
     With the button at the death message (`c1-button`): **the counter
     back to 14, gold 0, Life Force 40**, the start of the room visit
     that began with a region change (as 10.25's `b-button`), not the
     save asked for; 15 restores in 200M, all 14. With F1 (`c1-f1`,
     `c1-f1e`): **"saving game", "the adventure has been saved"**, then
     the add-a-player screen; without SAVE GAME (rr21 `b-f1`) F1 goes
     straight to that screen. So the save is made after the death message
     and only if it is answered with F1: the state saved has nobody alive.
     Then (`c1n.txt`): the button, the add-a-player screen (T's record all
     0, the counter 47), a new player N created (10.23's newcomer steps),
     N on the map (rounds to 68, gold 55 poked), in through the map door,
     gold 66 at its first room turn (round 69); EE killed N
     (`c1n-button`): **the counter back to 69, gold 55**, N alive on the
     room's entry square, 35 restores in 300M, all 69. The new player's
     map visit and room visit both begin with a region change, which
     comes after the all-dead save, so whether the button could ever
     restore that save isn't reachable this way.
   - **Item 2, Retire with Save** (`c2.txt`: T's and U's gold 111, T asks
     RETIRE in round 24; at round 24's end the offer, the button,
     "saving game", "the adventure has been saved"; U's gold 222 and EE
     230 at U's turn in round 26). EE killed U in round 26 (`c2-button`):
     **the counter back to 25, U alive with gold 111, T's slot still
     retired (first byte 0), EE 182**: the retire's round end, not the
     room visit's start (22, T alive, gold 0 as rr21's `a-nosave`). U
     then had every turn, rounds 25 to 39. So a Retire with Save is a
     save point like SAVE GAME.
   - **Item 3, SAVE GAME in the first round of a visit that began with a
     region change** (`c3.txt`: rr21's split route, region 1 visited from
     round 40 and again from 60, the map with U between; T's gold the
     turn's index, 9 at round 49; at T's turn in round 60 gold 500 and
     SAVE GAME; the save at round 60's end; `c3b.txt` T's gold 7 at round
     61, EE 230). EE killed T in round 62 (`c3-button2`): **the counter
     back to 61, gold 500**, Life Force 33, then the map first (U, rounds
     61 on): the SAVE GAME at round 60's end, not the visit start of round
     60 (gold 9). A save at a visit's last round followed by a region
     change (the engine's two `Saved` events in one round) would give the
     same counter either way (the next visit's first round) with nothing
     to poke between them; not run. Seen besides: 600 gold left T unable
     to move ("you can't move. perhaps you are carrying too much", moves
     0) with the menu coming up at each press (`c3-button`, dropped).
29. **Room doors, region to region, and carried moves beside an enemy**
   (2026-11-14; ours, no research pass: the planned captures of
   `docs/research/turn-order.md` "No third pass", [creatures](rules/creatures.md)
   2.2.4 and 2.6, [movement](rules/movement.md) 2.3 and "Captures needed"
   8, [magic](rules/magic.md) 5.3.1). `ACSVM-research/capture/runs/rr25/`
   (the harness binary in `bin/`, `env.sh`; `build.sh`, `build2.sh`,
   `build3.sh` make the disks, `room.txt`, `room2.txt`, `wait.txt`,
   `turn2.txt` the snapshots). New scenario option `--side-door one-way|
   two-way|region` (`scenario::side_door`): the side room's portal on room
   0's 7,4 as before, the same with the two-way bit (property byte 0, bit
   5) set, or one-way into room 0 (FAR ROOM, 10 x 8) of a second region,
   square 2,4 either way. New `scripts/turnstarts.py`: a `mem` run's turns
   from the region, counter, individual, moves left, square and Life Force
   (section 9), each turn's steps and idle moves and, with `--range`, which
   ranges explain each idle move's healing roll. Each draw shows in
   `--draws` as two stores, the state XOR the line, then that x 69 + 131
   (the raw number): one roll, as 10.28 counted them; the script joins the
   pair. Every disk: T alone, Speed 12, Constitution 40, Life Force 10 (15
   to 24 by the room). `room.txt` waits for a full bar, lets it drain to W
   moves, walks south through the map door until C are left and saves
   `room.vsf` at T's first room turn. A `mem` run has no jitter: one run a
   case except the random-state sweeps.
   - **Item 2, a room door into another room** (`one`, `two`: the portal
     one-way and two-way; from `r8/room.vsf`, 8 moves; `x1` holds east 3M,
     `x0` the same from `w3/wait.vsf`, 3 moves left). `x1`: 3 squares east,
     the portal with 5 moves left; **the moves went to 0, the individual to
     0, the counter up (round 8 to 9), and T's next turn began in the side
     room on 2,4 with 5 moves**, then walked them (the stick still held);
     the turn after that 12. `x0`: the portal with the last move; the next
     turn 12. The same in both, to the instruction nearly: the two-way bit
     changes nothing. So a room door into another room ends the turn like
     a map door and **carries the moves left** into the next turn's start
     (the full Speed if none), as creatures 2.2.4 says of the map door.
   - **Item 1, from one room region into another** (`reg`, the same
     runs): `x1` as `one` up to the portal, then the region byte 1 to 2,
     and T's first turn in region 2 (round 9) with the 5 moves; `x0`: the
     full 12. No round missed. With a second player (`reg2`, U Speed 4
     added on T's first map turn, `room2.txt`: T in with 8 moves, U with
     all 4, both in room 0 of region 1 from round 13; T through the portal
     with 5 left at once, `x1` 150M, `x2` 400M instructions): **U alone
     in region 1 rounds 13 to 32, T in region 2 rounds 33 to 42 (its first
     turn there with the 5 moves, 20 rounds later), U in region 1 from
     43**. So T, reaching region 2 during the cycle begun at round 13,
     waited for the next (creatures 2.2.1), where region 1 came first; the
     carried moves survive the wait.
   - **Item 3, carried moves beside an enemy** (`adj`: EE, an aggressive
     enemy of Speed 0 and Strength 1, on 5,4 east of the entry; `diag` the
     same on 5,5). T's first room turn began with **8 of 8 carried (`adj
     r8`), 7 of 7 (`adj r7`, one idle move on the map first), 8 (`diag
     r8`)**: not halved; with none left at the door (`adj r0`) it began
     with **6**, the full 12 halved as any turn beside EE; T's later turns
     beside EE 6 each.
   - **The idle healing roll's range** (`--set-random-state --draws`, Life
     Force below Constitution throughout; `turnstarts.py --range`): the
     roll is the first draw in the 300,000 instructions before the moves
     drop that falls under n (raw // (256 // n), a redraw at n or more);
     a range fits a move when that roll is 0 exactly when Life Force rose
     and no redraw follows that n would not make.

     | Turn | Runs | Idle moves | Range that fits all | Others |
     | --- | --- | --- | --- | --- |
     | `adj s8` turn 0: 8 carried, door turn 12 | 64 | 512 | **12** | 8: 486, 6: 468, 4: 415 |
     | `adj s8` turn 1: beside EE, 6 | 64 | 384 | **6** | 12: 362, 8: 369, 4: 350 |
     | `adj s0`: none carried, 6 | 32 | 192 | **6** | 12: 173, 8: 176 |
     | `adjside s3`: 3 carried, door turn 6 | 64 | 192 | **6** | 12: 176, 8: 182, 3: 161 |

     `adjside` (`build3.sh`): EE on 3,4, west of the entry, and the
     side portal; from T's second room turn (`t2/turn2.vsf`: beside EE, 6
     moves), 3 squares east onto the portal; the next turn in the side
     room began with the 3 moves. So the range is the moves the last
     refilled turn top gave, halved there when hindered: a carried turn
     keeps the door turn's (12, or 6 when that turn was beside EE), never
     the count carried; an ordinary turn beside an enemy uses its halved
     bar.
30. **Characters' open cases: destroy in a room, disk identity, a full
   roster, a real absence** (2026-11-14; ours, no research pass:
   [characters](rules/characters.md) 3.5, 6.2, 7.3, 7.5 and "Captures
   needed" 1 to 3 and 5, [magic](rules/magic.md) 5.5, and the planned
   captures of `docs/research/characters.md` "No third pass yet").
   `ACSVM-research/capture/runs/rr26/` (rr24's harness binary and
   `acs.exe` in `bin/`, `env.sh`; `gen1.py`, `gen3.py`, `gen4.py` write
   the scripts; `run1.sh`, `run1b.sh`, `run2.sh`, `run2b.sh`, `run34.sh`,
   `run4.sh` run them two VICE at a time; `sum1.py` lists item 1's room
   lists). No harness changes. Scripts only, one run a case.
   - **Destroy in a room, one thing at a time** (`d1-CASE-u` from rr19's
     `room1/offer.vsf`, T and U in the room, the offer under U's strip;
     `d1-CASE-last` from rr19's `last1/offer.vsf`, T the last player, AA
     in the room). At the offer T's gold, weapon held, armor worn and
     possessions (`$F5B1`, `$F5B3`, `$F5B4`, `$F5B6` on) poked to one
     thing alone, then F1, the button, F1; the room's list (`$382A`,
     `$E583`, `$E585` on, section 9) read before, at the moment T's first
     byte went to 0, 20M later and once the screen settled (alike in
     every run: nothing changes after the destroy). 26 cases, 46 runs;
     the list was empty before in every run.

     | Thing (kind, the thing block's "ACS may add" count) | with U | last player |
     | --- | --- | --- |
     | 300 gold | on 4,4 | on 4,4 |
     | BLADE 59 (melee, 0), held or not | on 4,4 (2) | **gone** (2) |
     | DAGGER 2 (melee, 4), MAGIC SWORD 34 (melee, 1) | on 4,4 | on 4,4 |
     | BOW 41 (missile, 8), held or not; DART 4 (missile, 15) | on 4,4 | on 4,4 |
     | MAIL 40 (armor, 1), worn or not; HELMET 54 (armor, 4) | on 4,4 | on 4,4 |
     | GOODA 13 (treasure, 0), one or two | on 4,4 (x1, x2) | **gone** |
     | KEY 73 (treasure, 4), JEWEL PACKET 6 (treasure, 2) | on 4,4 | on 4,4 |
     | SCROLL 57, FOOD CONTAINER 5 (magic items, 3 and 15) | on 4,4 | on 4,4 |
     | BLESSING 58, FLIGHT 61 (spells) | gone | gone |
     | HORN 8, HAND 12, ACID TOUCH 79 (0), CLAW 11, DEADLY TOUCH 78 (15): creatures' melee weapons, weight 0, not usable by anyone | gone | gone |
     | the kit (rr19's: gold, blade, bow, mail, GOODA, GOODB) | all six | bow, mail, gold |

     So it goes by neither kind nor worn or held. **With the other player
     present** every carriable thing goes down but spells and the
     creatures' weight-0 weapons. **As the last player** the things whose
     "ACS may add" count (thing block + 0x800, high nibble) is 0 are lost
     too (blade, GOODA, GOODB; every thing kept has a count of 1 or more;
     corrected in 10.38: on the `last1` disk things 59, 13 and 14 are a
     spell and two creatures' weapons, not ours, and our blade and
     treasures go down as last player too).
     The creatures' weapons with a count of 15 are lost either way, so
     their loss is not the count. The count as the cause is a fit to 3
     things, not tested by changing one. The kit as last player with the
     random state `$1CC9` set to 0, 85 and 170 before the last F1
     (`d1-kit-rs*-last`): bow, mail, gold in all 3, as rr19's `newc` found
     after a newcomer, so the newcomer's save changed nothing there.
     **Order**: the kit went down gold first, then GOODA, GOODB, MAIL,
     BOW, BLADE (each new entry goes first in the list): possessions in
     thing-number order, the held and worn ones among them. Snapshots
     just after the second F1, without disks:
     `d1-kit-snap-u/destroyed.vsf`, `d1-kit-snap-last/destroyed.vsf`.
   - **Disk identity** (`d2-*`, from rr19's `create-menu-nodisk.vsf`,
     `$3D5A` read 235,252 and poked to 137,87, the adventure disk's own
     word, as rr19 did; the other disk in drive 8, cursor 0). The other
     disk was rr19's `other300.d64` (V, 300 gold, the kit) with its word
     at d64 0x1C800 set to 13,151 (`id-ctrl`), **137,87, the adventure's
     own** (`id-same`), 137,151 (`id-lo`), 13,87 (`id-hi`), and 137,87
     then back to 13,151 (`id-back`, byte for byte `other300.d64`): all
     five showed "you may use the back side", "put the disk from which to
     copy", then the list with V and the cursor back on 0. The
     **adventure's own disk** (`own`: a copy of `create-menu-disk.d64`,
     its roster holding T) the same: T listed, cursor 0. rr19's
     `retire-disk.d64` (`ownu`: roster entry 1 empty, U in entry 2) put U
     in the second column and the cursor on 1, as rr19's `ota-ownid`: that
     comes from the empty first entry, not from the identity. Taken
     through (`d2b-*`, `id-b.txt` from each `list.vsf`, the adventure disk
     attached at "put your adventure disk in"): **`id-same` joined V as
     another adventure's player** (gold 300 raised to 1,000, no weapon,
     armor or possessions, on 12,12; V's 2 name bytes cleared on the other
     disk) and **`own` joined T the same way** (T's roster gold 0 raised
     to 1,000, its room square 4,4 of region 1 replaced by the map's
     12,12, its name's byte cleared on the copy). **No refusal and no
     message** in either: the finding's "the same adventure's disk is
     refused" did not show. `id-ctrl` stuck at "put your adventure disk
     in" (the `attach` too late, 2.4, as in rr19) and `ownu` took a record
     that was not U's; neither bears on the identity.
   - **A full roster** (`d3-*`, `full.txt` from rr19's `ret1/offer.vsf`,
     the map offer for T, U in play; `gen3.py` names, with copies of U's
     record, every roster entry of `retire-disk.d64` but U's: `full34` all
     34, `free34` all but entry 34). `full34`, the button: **"Sorry. The
     disk is full. T will keep playing."**, the button, and T played on
     (its turns in rounds 27 and 28, its first byte 75); **the disk
     unchanged byte for byte**, nothing saved. T's Life Force read 11 (its
     Constitution) at round 27, 5 at the offer: raised as for a save or
     healed over those 5 rounds, not separated. `free34`: "saving game",
     the save, T out of play; **T's name in entry 34** (creatures
     + 0x1850) and its whole record at + 0x1E6F (the decoder's 25 names
     and 6 records a page: entry 34's place confirmed), Life Force 11,
     adventure block + 0xE7 22.
   - **A real absence** (`ab-4`, `ab-10`, `ab-30`; `ab/`: the `loose`
     scenario with `--no-item --player name=T --creature
     name=AA,x=12,y=2,spd=0,con=20,lf=5,wis=20,pow=2`, started with
     `start-retired.txt`; `gen4.py`). T walked into the room in round 13;
     AA had turns in rounds 15 to 17 (Life Force and Power up 1 a turn,
     the byte + 12 15, 16, 17); T stepped onto the exit (room 4,5 to the
     map's 20,12) in round 18, waited on the map until round 18 + K, was
     poked onto 12,15 and pushed south through the map door. **While T
     was on the map AA's slot at `$E1A3` read all zeros** (+ 12 to + 15
     read 1, 17, 17, 17): the room region was unloaded. Back in the room
     AA came from the disk with Life Force 8, Power 5 and the byte 17
     kept, and at its first turn (round 19 + K, so N = K + 6 rounds since
     round 17):

     | K | N | Life Force, Power at its turn | the rule (8 + N / 2; 5 + (N - 1) + 1 drift) |
     | --- | --- | --- | --- |
     | 4 | 6 | 12, 11 | 11, 11 |
     | 10 | 12 | 14, 17 | 14, 17 |
     | 30 | 32 | 20, 20 | 20, 20 (the caps) |

     Power as the rule gives in 3 of 3, Life Force in 2 of 3; `ab-4`'s one
     point more is the Speed 0 creature's extra point of healing that
     10.27 found going with the random state. The byte was the current
     round after each turn. So the catch-up holds across a real absence,
     the region written to the disk and loaded back with the byte.
   - **Two "Add Someone" requests in one round** (`d5-add2`, `add2.txt`
     from rr19's `nd-menu/nodisk.vsf`: T asks RETIRE in round 21, U to
     ask ADD SOMEONE twice): not reached. The blind script lost step with
     the screens (the retire offer came at round 21's end and a menu press
     saved T); the add-player screen then came at round 22's end with T
     listed. It needs a script that waits on each screen.
31. **A store's stock across visits, saves and restores** (2026-11-14;
   ours, no research pass; [stores](rules/stores.md) 2.8 and "Captures
   needed" 1, 3 and 4). `ACSVM-research/capture/runs/rr27/` (the harness
   binary in `bin/`, `env.sh`; `gen.py` writes the scripts, `build.sh` and
   `all.sh` the disks, `kill.sh` the deaths; `diskstock.py` reads a disk's
   region stock and players with `acs dump`; `chg.py` is rr24's). New
   scenario options: `loose` now takes `--store` and `--stock`, and
   `--item-once` sets our magic item's "disappears after use" bit
   (`scenario::item_once`). Disks: rr10's `look` store (5,4) and stock
   (WARE1 10, WARE2 50 x2, WARE3 200) on the `loose` scenario, so the
   room has its exit to the map on 4,5; T (Speed 6, Wisdom 20, gold 500)
   owns GOODA (thing 13, value 100) and the item (thing 70). `shop` has
   no creature; `shopk` adds EE (an aggressive enemy with rr21's killer's
   stats, on 13,8) and gives T Constitution and Life Force 10. `build.sh`
   plays each from `boot/play-nodisk.vsf` in one session (`chain.txt`:
   `start-retired.txt` then `enter-room.txt`) and saves `room-nodisk.vsf`
   and the disk as `detach` flushed it (`room.d64`), so each script below
   writes to its own disk file and ends with `detach`. The drive was
   alive in every run. One run a case (no roll in a purchase or a sale,
   10.17).
   The store visit (`shop()` in `gen.py`): bump east; the button sells
   GOODA for 59 (gold 559, `$E070` 64: thing 13 in stock); the stick skips
   the item's offer; browse, bump WARE1 on 6,6 and buy it (gold 549,
   `$E06D` 100 to 36: thing 1 from 1 to 0); west along row 5 and out by
   0,4: T on 5,4 with one move. A diagonal push onto the room's exit, or
   held on the map, did not move T; the scripts walk straight. On the
   `shop` disk the settle after leaving ran its full 50M (the counter 18
   to 24, T alone); nothing in the stock or the purse changed meanwhile.
   - `out`: T west to 4,4, south onto the exit: the map, T on 20,12, the
     stock bytes all 255 (10.21). **The disk's region 1 stock is now
     WARE2 x2, WARE3, GOODA** (WARE1 gone, GOODA added) and T's record
     holds 549 gold, WARE1 and the item: leaving the region wrote the
     stock with the rest (`room.d64` before: WARE1, WARE2 x2, WARE3; T 500
     gold, GOODA and the item).
   - `back`: the same, then south and west onto the map door and in
     again (the region read from the disk): **`$E06D`-`$E07A` as left**
     (36, 0, 0, 64, the rest 0), and the store screen draws WARE2, WARE3
     and GOODA, 6,6 empty (`shelves.png`). The disk at the end as `out`'s.
   - `nosave` (`shopk`; EE poked to a friend, `$E1A7` 182, before the
     visit; at T's next turn, round 19, back to 230 and onto 6,4 beside T
     (`$E1B0`), `kill.vsf`; `kill.sh`: `run mem --answer T --answer-text
     CONTROL,CONTINUE`, 300M, `nosave-kill`): EE killed T in round 19; the
     button: **the counter back to 17, gold 500, GOODA owned again, WARE1
     not, the stock 100, 0, 0, 0 (WARE1 back on its shelf, GOODA gone from
     it)**, EE's class 227 as built: the room visit's start, stock
     included. The disk flushed before the death still held the visit
     start's stock.
   - `save` (the same, with SAVE GAME at T's turn in round 19, the press
     1.5M into the turn, once the prompt shows; the save at the round's
     end; the disk flushed at the "saved" message: **region 1's stock
     WARE2 x2, WARE3, GOODA, T 549 gold** and WARE1): EE killed T in round
     20; the button (`save-kill`): **the counter back to 20, gold 549, the
     stock 36, 0, 0, 64**, EE 182 (as when saved). SAVE GAME saves the
     stock and the button restores it.
   - `item` (`shop`): Use Object on the item, "THE ITEM WORKED": **the
     item gone** (`$F5C2` 4 to 0) and **not in the stock** (`$E079` 0; the
     store screen drew WARE1 to WARE3 only). A magic item used up does not
     restock (stores 2.5).
   - `twice` (`shop`, `$F5B9` poked to 128: GOODA owned twice): the
     button: "T has 2 GOODAS", "press button to sell one for 59"; the
     button: gold 559, one left, "T has 1 GOODA", "sell it for 59"; the
     button: gold 618, none left, the stock's GOODA count 2 (`$E070` 128).
     **A thing owned twice is offered once per copy** (stores 5.2).
   Not tried: a magic item set to act when picked up, bought (stores
   4.4); a store in a room of another size.
32. **The player's firing checks, a kill, another player as target, and
   creatures' combat-side open cases** (2026-11-14; ours, no research
   pass: [combat](rules/combat.md) 8.8 and "Captures needed" 6,
   [creatures](rules/creatures.md) 2.7, 8.1 and "Captures needed" 7 and
   8). `ACSVM-research/capture/runs/rr28/` (the harness binaries in
   `bin/`, `env.sh`, `mem.sh`; `case.sh` is rr20's, which leaves T on 4,5
   or 4,4 by the walk; `case2.sh` waits for a full bar and lets go as the
   room region loads, `enter2.txt`: T on 4,4 with Speed - 4 moves;
   `build2.sh` the two-player disk; `tl.py` prints a `mem` run's changes
   and draws in time order, `burden.py` and `still.py` sum the creature
   cases). New scenario option `--room-thing KIND:X,Y`
   (`scenario::room_things`: a space, custom space or custom obstacle of
   ours, the door picture, on a room 0 square). `scripts/combatcheck.py`
   gains the cases `kill-e5` (`--kill-roll yes|no`) and `fire-u`
   (`--player-roll yes|no`); `scripts/dropcheck.py` the case `drop-kinds`
   and `--refuse`. Pushes are `mem --push` schedules (the menu: the
   button, two pushes down to FIRE, the button), one run each where
   nothing is random.
   - **Which test comes first** (`bes`: T holding the bow, 2 moves, on
     4,4; XX, a Speed 0 aggressive brave enemy of Strength 1, on 5,4; from
     `bes-room.vsf`). T's held weapon `$F5B3` set to 0 (owning the bow,
     holding nothing), XX left an aggressive enemy (`ord-c6`): FIRE gave
     "may not use a missile weapon while next to XX" and **no move** (2
     left; after the button the bar drained them as two idle moves, each
     with its draw, at 9.0M and 10.8M); with the bow's possession bit
     cleared too (`$F5BF` 128 to 0: `ord-c6-own0`) the same. With XX a
     peaceful enemy (`ord-c2`, `ord-c2-own0`) FIRE took a move at once (2
     to 1) and showed "select a missile weapon", then the menu: NONE and
     BOW, or NONE alone (screens `sc-ord-c2`, `sc-own0-c2`, `sc-ord-c6`).
     So **8.8.2's test comes before 8.8.4's**, and it refuses whether a
     missile weapon is held, owned or neither.
   - **Who counts beside** (`cls-c0` .. `cls-c7`: XX's class byte `$E1A7`
     set to 128 + 16 x the class, alignment 0 thief .. 3 friend plus 4 if
     aggressive, before FIRE; T holding the bow). Refused, no move: the
     aggressive thief, neutral and enemy (c4, c5, c6). The cursor, then 2
     shots at the button (`$B799` 1, then 0): the peaceful thief, neutral,
     enemy and friend (c0 to c3) and **the aggressive friend** (c7).
     **Another player** (`bes2`: T Speed 12 holding the bow, U Speed 4,
     both walked in by rr25's `room2.txt`; `bes2fire2.txt`): T stepped
     east to 5,4 and idled, U idled its first room turn out on 4,4, and at
     T's next turn (12 moves) FIRE brought the cursor: **another player
     beside never stops FIRE**. Both as the engine says (aggressive and not
     a friend; players never). Seen on the way: at T's first room turn U,
     which came through the door after T, is not yet in the room: it
     isn't drawn, and in a first 64-run sweep at it (since deleted) no
     arrow ever reached its square; from its own first turn there on it
     is drawn and hit.
   - **The roll of 0 to 9 after an arrow that kills** (`kill`: fire-e5's
     room, T Speed 10 with 6 moves on 4,4, EE at Life Force 1; cursor
     snapshot `killsnap/fire-e5.vsf`; sweep `kill-e5`, 64 runs,
     `--set-random-state`). `combatcheck.py --case kill-e5`: **64/64 with
     the roll drawn after the killing arrow**, 46/64 without (18 runs then
     have a draw left over before T's next turn). 25 kills; at each **the
     firing ends** with shots still due (`$B799` straight to 0, the slot's
     first byte `$E1A3` to 0, "T harms EE with bow", "EE dies"), then the
     round's end (the slot roll and the room's percentile) and T's next
     turn, its bar filling before its next draw.
   - **The roll after an arrow that damages a player** (`fire-u`: from
     `bes2fire2/fire-u.vsf`, the cursor on U beside T; 64 runs, 7 shots
     each, U's record `$F5C4` + 3, + 5, + 6 watched). `combatcheck.py
     --case fire-u`: **64/64 with the roll after every damaging arrow on
     U** (288 of them, 91 dodges), 2/64 without. The engine's choice (no
     roll when the arrow damages a player) disagrees; what the roll
     decides for a player isn't seen (it has no class to change).
   - **Why choosing the bow costs a second move** (`rdy`: T Speed 10, 6
     moves, owning blade and bow, holding the blade, EE at 9,4). The moves
     `$719A` and the held weapon `$F5B3` timed against the pushes: FIRE
     took 6 to 5 at the press; **the second move goes at the moment the
     bow is chosen**, the held weapon becoming the bow in the same sample,
     and then the cursor holds the bar. Alike holding the blade
     (`rdy-blade`) or nothing (`rdy-none`); waiting 8M instructions on the
     message (`rdy-slowmsg`) or on the menu (`rdy-slowmenu`) changed
     nothing, so it is not the bar running while the menus show. NONE
     (`rdy-nonepick`) took no move at the choice (the bar then drained as
     usual). **NEW WEAPON** (the first row's third entry; its "select
     weapon" menu lists NONE, BOW and BLADE) choosing the bow took one
     move at the choice too (`nw-bow`, `nw-bow-blade`: 6 to 5), and FIRE
     with the bow then held went straight to the cursor. So the second
     move is the readying's own (combat 6.1), the same as NEW WEAPON's,
     on top of FIRE's. With the moves set to 1 or 2 (`rdy-m1`, `rdy-m2`):
     FIRE took 1 to 0 (2 to 1), the bow 0 (1 to 0) and was readied all
     the same, one shot; with 6 (`rdy-m6`) 6, 5, 4 and 3 shots (`$B799`
     2, 1, 0).
   - **The second kind of thing that refuses a drop** (`drop-kinds`: WE,
     rr20's dropper, on 6,4 with a custom obstacle of ours on 5,4, the
     side room's portal on 7,4, a custom space on 6,3 and a space on 6,5;
     sweep `drop-kinds-sw`, 256 runs from the turn-end snapshot). With the
     side room the room's list (the word at `$382A`) starts at `$E58D`,
     its length byte at `$E58B`. `dropcheck.py --case drop-kinds --refuse
     5,4`: **256/256**, 306 tries, 50 failed on the custom obstacle,
     landings 6,3 51, 6,5 50, 7,4 50, the own square 105; with no square
     refusing 209/256 (all 47 failures tries onto 5,4); with the portal,
     the custom space or the space refusing instead 164 to 169/256. So
     **the custom obstacle refuses** (the engine's guess); a portal, a
     space and a custom space take the thing. A store wasn't tried.
   - **When a creature counts as overburdened** (`burden2`: four peaceful
     friends of ours, Constitution and Life Force 10; WA, WB, WC Strength
     2 (capacity 200) and Speed 6 carrying one treasure each, of weight
     150, 101 and 100; WD Strength 1 (capacity 100) and Speed 0 carrying
     400; sweep `burden2-sw`, 8 runs, `burden.py`). In all 8: **WA's turn
     began with 3 moves** (6 x 2 x 50 / 200), **it dropped the treasure
     with its first move and took 2 steps after**; **WB began with 5,
     dropped, then 4 steps**; WC (exactly half) began with 6 and walked 6
     with no drop; **WD (Speed 0, over its capacity) never dropped**. Their
     next turns began with 6. So a creature drops once its load passes
     half its capacity (the slowdown cuts its moves below its Speed), as
     its turn's first action, using one move, then steps on with the
     rest; the drop turn's moves come from the load before the drop; a
     Speed 0 creature never drops: the research's unconfirmed reading
     (creatures 8.1.2). The engine's choice (only above the capacity, the
     moves then idled) disagrees. Seen too: in 4 of 8 runs WC, at its
     second turn, stepped beside T and dropped its treasure there
     mid-turn (a friend's gift, creatures 7.3, here a treasure; not
     looked at further).
   - **A Speed 0 fighter with its goal beside it** (`still-melee`: EE
     Speed 0, an aggressive brave enemy of Strength 10 owning and holding
     our blade, on 5,4; T on 4,4 holding nothing; sweep `still-melee-sw`,
     16 runs from `still-melee-room.vsf`, `still.py`): **64 of 64 of EE's
     turns had an attack** (`$B799` written each time; 1 blow, first
     misread from that byte as 2: see 10.35), T's Life
     Force fell 16 times in them, EE's square never changed. So it
     attacks once a turn (creatures 2.7's step is not cancelled onto the
     goal's square), as traced.
   Not tried: a Speed 0 fighter beside an exit or a thing it takes; a
   store refusing a drop; magic items in a drop.
33. **Creatures' and magic's open cases: readying with the last move, two
   adds in one round, the Speed 0 extra point** (2026-11-14; ours, no
   research pass: [creatures](rules/creatures.md) 2.7 and "Captures
   needed" 7 and 8, [characters](rules/characters.md) 6.1, 6.2 and
   "Captures needed" 5, [magic](rules/magic.md) 5.3, 5.5 and "Captures
   needed" 2). `ACSVM-research/capture/runs/rr29/` (the harness binary in
   `bin/`, `env.sh`; `case.sh` (10.24's) and `build.sh` build the cases of
   `cases.txt` and their turn-end snapshots, `sweep.sh` and `catch.sh` run
   the sweeps, `run mem --set-random-state --draws --ram`, two VICE at a
   time; `py/turns16.py` lists a creature's turns, draws and changes,
   `py/readycheck.py`, `py/ordcheck.py` and `py/diecheck.py` sort them; the
   add scripts and their snapshots in `add/`). No harness changes. A
   creature's turn shows two draws 28,000 instructions apart after its
   action whatever it did (10.27's turns too), so a turn's action is read
   from the draws before them.
   - **Readying that spends the last move** (creatures 2.7, the engine's
     "every creature's first action is taken"). EE an aggressive brave
     enemy on 9,4, Strength 10, Dexterity 13, owning our bow (power 10,
     range 7), T on the entry 4,4; sweeps from T's first turn's end.

     | Case | Runs | EE's first turn | Its later turns |
     | --- | --- | --- | --- |
     | `ready1`: Speed 1, bow owned, not held | 32 (`sw-ready1`, `sw-ready1b`) | the die, two checks, the bow readied (`$E1B7` 0 to 41) with the move (1 to 0), **then nothing**: no shot in 32 of 32 (only the turn's closing draws, with redraws in 7) | a shot each turn (break, deviation, side; T hit or dodging in some) |
     | `ready2`: Speed 2, the same | 32 (`sw-ready2`) | readied (2 to 1), then **a shot** (1 to 0) | a shot each |
     | `held1`: Speed 1, bow held | 32 (`sw-held1`) | a shot | a shot each |
     | `ready1w`: `ready1` with Life Force 10 of 20 and the obstacle on 5,4 (the line blocked) | 32 (`sw-ready1w`) | readied, **then nothing**: no step rolls, no step, no idle move's heal (Life Force +1 only in the 4 turns whose top-of-turn die gave it) | 4 step rolls and a step west or south |

     So **when readying spends a creature's last move its turn ends
     there**: no shot, no step choice, no idle move. The engine's choice
     (a shot or a step choice, else an idle move) does not hold for Speed
     above 0; with a move left the readying creature fires as usual.
   - **Several "Add Someone" requests in one round** (characters 6.2;
     `add/`, scripts run a step at a time from rr19's
     `nd-menu/nodisk.vsf`: round 21, T and U living in slots 1 and 2 on
     the map, T's turn with the menu open on RETIRE; each step's
     screenshots read before the next was written). ADD SOMEONE is one
     push north of RETIRE, and three east of MOVE in a newly opened menu;
     the menu uses no move, so a player can ask twice in one turn.
     - `a` (T asks twice in its turn, `a1.txt`): **"I will add a player as
       soon as possible" both times**. At the round's end (the counter
       already 22) **one** add-player screen; CREATE NEW PLAYER, newcomer
       N (`a3.txt`, 10.23's steps): **N in slot 3** (`$F5E9` first byte
       13), slot 4 still free; the game went on, rounds 28 to 32 taking
       turns 1, 2, 3 (`order-a`, `run mem`), no second screen.
     - `c` (T asks once, then U once, `c1.txt`): the same, both answered
       "I will add"; one screen, N in slot 3, slot 4 free, turns 1, 2, 3
       in rounds 29 to 33 (`order-c`).
     - `b` (T's turn left through MOVE; at U's turn T's first byte
       `$F59F` poked to 0, then U asks twice, `b1.txt`): both "I will
       add"; at the round's end first the restore message of 6.3 (the
       living count changed), F1, then **one** add-player screen; **N in
       slot 1** (the freed slot below U's), slots 3 and 4 free; from round
       28 **N then U** every round, rounds 28 to 34 (`order-b`).
     So **requests in one round add one player**, whoever asked and how
     often, with no message for the extra ones; the newcomer takes the
     lowest free slot, below a living player's too, and moves in that
     slot's place in the round. The engine's choice (each request
     handled, one after another) adds two.
   - **The Speed 0 creature's extra point of Life Force** (magic 5.5,
     10.27): rr23's `in/room.vsf` (AA Speed 0, Life Force 5 of 20, Power 2,
     its turn in round 40) with AA's byte (`$E1AF`) set to B = 40, 39 and
     0, every random state 0..255 (`catch-40`, `catch-39`, `catch-0`, 768
     runs, `catch.sh`). The extra point is **the top-of-turn die** (5.3.2):
     the draw just before AA's turn begins, after T's last idle move,
     heals 1 when it gives a 2 (raw // 42 = 1, a redraw at 252 or more):
     `diecheck.py` **256/256 for each B**, the states as 10.27 found them
     (B 40: 85, 136, ..; 39: 168, 184, 200, 248, ..; 0: 40, ..), 36 to 40
     a B. Each draw is the state XOR the raster line's low byte, then x 69
     + 131 (10.29), and the catch-up's work moves the die's moment: with
     B = 39 the die fell **one raster line earlier** than with B = 40 in
     all 256 runs (269 against 270 in 158, 275 against 276 in 66, ..),
     with B = 0 at another line again (307 in 158). T's draws before were
     the same in every B. So the byte changes the die's face only through
     the timing; the rule is 5.3.2's die, 1 in 6, as for any individual.
   - **A drop turn's moves** (creatures 8.1.2; `drop6`: WE Speed 6,
     Strength 1, Constitution and Life Force 10 (capacity 100), owning one
     treasure of weight 400, on 7,6; `sw-drop6`, 32 runs, the room's list
     watched as 10.24). In **32 of 32** WE's first turn began with no
     refill (moves 0), drew the die, its check and the drop's pair, put
     the treasure down and ended with no step roll and no step; its next
     turn began with 6 moves and walked 6 squares. So the moves come from
     the load before the drop, as the engine takes it.
   - **What counts as ordinary for the cancelled step** (creatures 2.7.3;
     `ord`: EE Speed 0, an aggressive brave enemy owning and holding our
     blade, on 9,4; FF a peaceful friend, Speed 0, on 10,4 beside it; T on
     4,4, 5 squares off; `sw-ord`, 64 runs, `ordcheck.py`). EE's turns:
     the die, two checks, four step rolls, and then either nothing (82
     turns: the step cancelled) or **an attack on FF** ("EE misses FF",
     the blows byte `$B799` written): in **22 of the 64 first turns**.
     EE never moved. So a square holding an individual it would strike
     is not ordinary, the step onto it is not cancelled, and a Speed 0
     fighter strikes a creature beside it that is not its goal, as the
     engine says. `--answer T` does not press through that message, so
     those 22 runs stopped there; `sw-ord2` (32 runs, `--answer-text
     CONTINUE`) played on: 96 turns of EE, **35 with an attack on FF** (7
     of them hurting it), 61 with none, EE never moving.
   Skipped: the combat-side cases another capture took (10.32);
   creatures "Captures needed" 1 (the all-dead save after a newcomer
   starts on the map: a long chain, and 10.28 found the region change
   comes after that save), 2 (two random creatures leaving at once), 3
   (mid-turn adjacency, odd Speeds), 4 and 5 (fighters' goals among
   other individuals, the first step's costs: no single-roll setup), 7's
   gifts, magic items and gold's weight; magic "Captures needed" 3 to 7
   (Kill All, Summons, Add to Room, Activate All, smartness rates): the
   scenario writer has no such spells or spaces yet.
34. **Encounter areas' open cases: door items, the door table, reopening,
   a store's shelves** (2026-11-14; ours, no research pass;
   [encounters](rules/encounters.md) 8, [stores](rules/stores.md)
   "Captures needed" 1). New scenario option `--map-bytes OFF:HEX`
   (repeatable; `scenario::map_bytes`) writes bytes into the world map
   block at a hex offset, e.g. `806:0042`. `ACSVM-research/capture/runs/rr30/`
   (the harness binary in `bin/`): `build.sh` builds rr22's `patch`
   scenario (WOLF on the grass at 60 %, T Speed 6 holding the blade) per
   case and starts it to T's first turn; `run.sh CASE` runs `in.txt`
   (the random state poked 0, 1, .. until one push east opens area 1;
   `in/in.vsf` at T's next turn); `exit.sh CASE DIR X Y D [SZ WP]` runs
   `out.txt` from it: area 2's entrance `$E0A4`,`$E0A5` poked to 15,11
   (area 2 free unless SZ/WP say otherwise), T poked onto X,Y, one push D,
   the map block's tail read from `$E7F2` as in 10.26; `reopen.sh` runs
   `reopen.txt` from an exit's `out.vsf` (states tried until a step east
   opens area 1 again); `all.sh` runs every case (two VICE at a time).
   One run a case (deterministic once the area opens); 17 cases, 24 exits
   and reopenings. Reads taken while the KERNAL ROM was banked in (bytes
   like 165 164 175 for `$F5AF` on) are dropped, as in 10.14.3.
   - **Several doors on one square** (`dbl01`: `1:0,5,82,0/0,5,82,1`;
     `dbl10` the same reversed): out from 1,5 west, `dbl01` came back on
     **14,12, area 1's own entrance** (parameter 0), `dbl10` on **15,11**
     (parameter 1, area 2's poked entrance). **The first door item in the
     list gives the parameter**, as the engine chose (3.5). Both door
     entries kept at closing.
   - **Doors off 0,5 and inside the room** (`twodoor`:
     `1:0,3,82,1/0,7,82,0/7,5,42,1`; `middoor`: `1:0,5,82,0/7,5,82,1/9,5,42,1`):
     gaps at rows 3 and 7, 0,5 wall (a push west from 1,5 refused, no
     move used, `o5`); out by 0,3 onto 15,11 (`o3`), by 0,7 onto the
     entrance 13,12 (`o7`). **A door item on the floor (7,5) is a way
     out too**, drawn as plain floor: stepping onto it from 8,5 sent T to
     15,11 (`mid`). Closing marked the treasure deleted (square 255) and
     **kept every door entry**, at 0,3, 0,7 and 7,5 alike (engine 7.2:
     every thing 82 stays). Reopened (`re-o3`): both gaps again.
   - **A door to another open area's entrance** (`join`: `1:0,5,82,1`):
     with area 2 set open by pokes (its room record `$E7F7` wall 6,
     `$E7F8` size 233, entrance 15,11; no creature in it) T came back on
     15,11 with **place 0** and 5 moves, area 2 still open (`open`), as
     with area 2 free (`free`). **The player does not join the area**, as
     the engine chose. Area 2 was opened by pokes, not by an encounter.
   - **Reopening** (`treasure`, `t80e0`, `t80e9`: `1:0,5,82,0/7,5,42,1`):
     after the treasure's entry was deleted at the first closing, area 1
     opened again with **no treasure drawn**; the list in memory held the
     deleted entry (square 255) unchanged, not compacted, in all three
     (`re-out`). A treasure listed in the file is there only until the
     area first closes (7.2).
   - **The door table at + 0x806** (template lists, area 1's door
     parameter 0). `t806a` (`806:0042`): T came back on **15,11, area
     2's entrance**. `t806p` (`806:0000`): T on **12,16, map point 0**
     (the map door's square), place 0, not through the door. `t806r`
     (`806:1055`): **"The action moves to TEST REGION"**, and T in TEST
     ROOM (region 1, room 0) on **5,5**, place 16 (`later/`). So the
     four words are an ordinary door table (format notes: byte 0 region
     x 16 + room, byte 1 x x 16 + y, region 0 a map point), the parameter
     indexing it from 0, and `00 41`..`00 44` are codes past the 64 map
     points that mean area 1..4's entrance (41,41: the start, 10.26).
   - **A parameter above 3 and + 0x80E** (`p4`: `1:0,5,82,4` with
     `80E:0042`): T came back on **15,11**: parameter 4 reads the word at
     + 0x80E as the table's fifth entry. `p4t` (the same, + 0x80E as the
     template has it, `32 08`): **"The action moves to"** the template's
     leftover region 3 (`$397C` 3), read as region 3, room 2, 0,8. The
     engine's choice (the start point for n above 3) differs.
     `t80e0`/`t80e9` (+ 0x80E 0x0000, 0x0900, the treasure list): entry,
     the treasure, the exit and the reopening as with the template's
     0x0832; the bytes stay in memory as written.
   - **Lists the decoder warns on.** `outside`
     (`1:0,5,82,0/7,12,42,1/15,3,43,1/7,5,44,1`): only 7,5's treasure
     drawn; at closing 7,12 and 7,5 marked deleted, **15,3's entry (square
     byte 0xF3) kept**. `bad` (`812:2000`: area 1's length 32, running
     over lists 2..4 and the zeros after): the area opened, its list the
     door, then the following bytes read as items (things 0, 1, 5 and 82
     on wall squares): **gaps at 0,0, 0,2 and 0,5**; out from 1,5 onto
     13,12 (parameter 0), and at closing **no entry marked deleted**. No
     crash in either. Room 0's list was not written (`--area-list` takes
     areas 1..4 only).
   - **A store in area 1** (`store`: `1:0,5,82,0/7,3,83,0`; `shop2.txt`
     to `shop6.txt` from `store/in/in.vsf`, T stepping onto the store
     from 8,3; `shop/` superseded, its first shot taken mid-draw). The
     stock bytes `$E06D`-`$E07A` read 255 throughout. Bumping shelf
     places (read with `storeread.py`, rr10's letters): **every
     carriable thing of the template is offered**: things 1 (OIL AND
     WICK, 20), 2 (DAGGER, 10), 4 (DART, 3), 33 (SCROLL, 2000), 35
     (HORSE, 2000), 40 (MACE, 35), 59 (BLADE, 10), 68 (SCROLL, 300), 72
     (MAGIC WAND, 1000); out of stock 12 (HAND, value 0) and the spells
     61, 64, 80. The two things drawn (10.26) are 33 and 59, the only
     ones whose picture (3) our scenario leaves non-blank: every thing is
     drawn, most of them blank. **Buying** (gold poked to 5000): HORSE
     for 2000, then the BLADE T already owned twice for 10 each (the
     gold 4990, 4980, the possession unchanged); **selling** the blade at
     the opening prompt paid 4 and took it; **the stock bytes never
     changed** (stores 2.7). **Leaving** by 0,4 and the button: T back
     on the store's square 7,3 in area 1 with **1 move** (`shop5`: 5
     inside, 1 just after), a push south then stepped to 7,4 and WOLF's
     turn came (stores 3.7). With the gold poked to 5000 the player came
     out unable to move ("perhaps you are carrying too much", `shop3`:
     the gold's weight), and an idle wait drained the one move
     (`shop4`). The engine's shelves in an area are empty (stores 2.7):
     the original offers everything.
   Not run: a second player joining an area for real, room 0's list, a
   death or a creature through the door.
35. **Combat open cases, the 2026-11-15 batch** (ours, no research pass:
   [combat](rules/combat.md) 3.1, 4.1.2, 5.1, 6.2, 6.3, 8.1, 8.8.3 and
   "Captures needed" 2 to 6, and disagreements of the golden tests,
   `crates/acs-engine/tests/golden/`). `ACSVM-research/capture/runs/rr31/`
   (the harness binary in `bin/`, `env.sh`; `case.sh` and `build.sh` build
   the cases of `cases.txt`: `snap`, rr20's chain to T's first turn's end,
   or `room2`, rr28's `enter2.txt`, T on 4,4 at its first room turn with 2
   of 6 moves; `sweep.sh` and `arrow.sh` the sweeps, `run mem
   --set-random-state --draws --ram --answer T`, three VICE; `rec.py`
   lists which bytes of a watched record change; `nw1.txt`, `nw2.txt`
   the player's weapon menu). No harness changes. `scripts/combatcheck.py`
   gains the cases `still-melee` (rr28), `sm-d8`, `sm-d16`, `sm-d24`,
   `bow-adj`, `ready-melee`, `brk-parry`, `mb-mail`, `mb-mmail`, the
   options `--still-first` (3.1's first term at full moves 0),
   `--build-round down|zero` (4.1.2), and for the FIRE cases `--cls`,
   `--wis`, `--arrow-rule`, `--arrow-roll` (the class byte `$E1A7`
   predicted after each damaging arrow), and checks every melee attack's
   blows against the first value `$B799` is written with.
   - **A Speed 0 fighter's blows** (EE Speed 0, an aggressive brave enemy
     of Strength 10 holding our blade, Constitution and Life Force 20, on
     5,4 beside T, who holds nothing; 32 runs each from the room snapshot,
     30 million instructions, 4 of EE's turns a run). Each turn: the die,
     two checks, a step roll for each of the four squares beside (T's
     included), then the attack (the step onto T is always chosen), then
     the round's end. Strict, every draw and every change: rr28's
     `still-melee-sw` (Dexterity 13) **16/16, 1 blow** in all 64 attacks;
     `sm-d8` **32/32, 1 blow** (128 attacks); `sm-d16` **32/32, 2 blows**
     (128); `sm-d24` **32/32, 3 blows** (128). With the first term 0
     instead of 1 `sm-d16` and `sm-d24` fit 0/32; with 4, 0/32. So 3.1's
     rule holds at Speed 0 with the first term 1. **`$B799` is written
     with the attack's blows at its start** (the blow under way counted)
     and counts down: 1 then 0 for a single blow; checked on every attack
     here and on rr9's `break-many` (4 and 6 blows). 10.32's "2 blows"
     was a misreading of that byte. **A negative Build rounds toward 0**
     (combat 4.1.2's open point): EE of Dexterity 8 (Build -4) did 9 with
     our blade (-40 / 30 taken as -1), T losing 7 Life Force and 4 Power
     a hit; rounding down (8) fits 13 of 32 `sm-d8` runs, toward zero
     32/32 (31 hits).
   - **What the roll of 0 to 9 after a damaging arrow decides** (from
     rr20's `fire1/fire-e5.vsf`, the cursor on EE at 9,4, the button at the
     run's start; EE's class byte `$E1A7` set to 128 + 16 x class, its
     Wisdom `$E1B4` set; `arrow-c0` .. `arrow-c7` at Wisdom 10, 128 runs
     each, `arrow-c5` 256; `arrow-c5w3`, `arrow-c5w20` 128 each; the class
     byte compared in full). Every run fits (`--arrow-roll smart`) with:
     **the roll is combat 7's smartness check, drawn only when the target
     is aggressive and not a friend and its Wisdom is 6 to 14**; then
     combat 7's change, the roll counting only for an aggressive neutral
     (an enemy below Wisdom - 5). Seen: peaceful thief, neutral, enemy
     turning aggressive (23 each, no roll; 128/128 each; drawing the roll
     there 113/128), a peaceful friend turning a peaceful neutral (23, no
     roll), **an aggressive friend turning an aggressive neutral with no
     roll** (23; 114/128 with the roll), aggressive thief and enemy
     unchanged with the roll (26 hits each), the aggressive neutral at
     Wisdom 10 an enemy on rolls 0 to 4 (30 of 57 hits; 256/256, with no
     change 226/256), at Wisdom 3 never and at 20 always (23 hits), both
     **with no roll** (with it 112/128 and 115/128). A peaceful neutral hit
     twice: aggressive, then on the second arrow the roll, and an enemy.
     The second player as target (`u-rec`: rr28's `fire-u` again, 64
     runs, U's whole record `$F5C4`..`$F5E8` watched, `rec.py`): only
     its Life Force, Power and Dodge change, and at U's death (48 runs)
     bytes +0, +8 and +11; **nothing else in the record moves**: for a
     player the roll (Wisdom 13) decides nothing. `combatcheck.py --case
     fire-u --player-roll yes` 64/64 again.
   - **fire-u's setup** (10.32, from rr28's `bes2/disk.d64`): T Speed 12,
     Constitution and Life Force 40, Strength 9, Dexterity 13, Wisdom and
     Power 13, Size 9, Dodge 30, Parry 26, Armor 0, Melee 31, Missile 22,
     holding and owning our bow (Power 10, range 7, adjustment 0, break
     chance 0, not magic); U the same but Speed 4 and nothing owned. T on
     5,4 with 12 moves (7 shots), U on 4,4, the cursor on U. Of 448 shots
     due **393 were fired: U died in 48 of 64 runs and the firing ended**;
     **379 reached U** (each drew its dodge roll: 96 % of the shots
     fired), 91 dodged, **288 did damage**.
   - **A weapon breaking with blows still due** (`brk-parry`: rr9's
     `break-many` with T's Parry 127, so EE's blows are parried at 90;
     both blades break chance 15; EE Dexterity 31; 32 runs from the
     turn-end snapshot). **32/32**: 70 parries, EE's blade broke 6 times,
     **4 with blows still due** (2, 3, 5 and 2 left), and each time **the
     attack ended there**; T's blade broke 3 times on parried
     counter-blows and EE's attack went on, as 6.3 says.
   - **A magic weapon against armor** (EE's blade magic; T wearing our
     mail, Power 2, break chance 15; 32 runs each). `mb-mail` (plain mail):
     **32/32 with the armor ignored**, no break roll, the full 10 or 15
     (39 damaging blows); the plain-blade reading (Power off, break roll)
     fits 6/32. `mb-mmail` (magic mail): **32/32 with the Power taken off
     and the break roll** (41 damaging blows, the mail broken 8 times);
     ignoring it 6/32. Both rows of 5.1's table.
   - **A foe truly beside an archer** (`bow-adj`: EE Speed 3 holding our
     bow on 5,4, T on 4,4; 32 runs from the room snapshot): **32/32**:
     EE's turn halved to 2 moves, no shot, a step away (to 5,5 17 times,
     5,3 9, 6,4 6), then **one shot** with the move left (Dexterity 13 / 8
     + 1 / 2); 29 arrows reached T, 17 hitting.
   - **Readying a melee weapon before stepping** (`ready-melee`: EE Speed
     3 owning our blade, holding nothing, on 5,4 beside T; 32 runs):
     **32/32**: the turn halved to 2, the blade readied (`$E1B7` 0 to 59)
     at one move, then the attack with 2 blows (4 x 1 / 2 + 1 - 1); its
     next turn 4 blows.
   - **The player's menu with no weapon held** (`p-nowpn`: T owning our
     blade, holding nothing, 2 moves, EE Speed 0 on 5,4; `nw1.txt` the
     push east, `nw2.txt` the menu, one run each, deterministic): the push
     used **no move** and showed "select a melee weapon to use", then a
     menu of NONE (chosen first) and BLADE. One push down and the button:
     **the blade readied at one move** (2 to 1, `$F5B3` 59) and **the
     attack at once**, 1 blow (`$B799` 1; 4 x 1 / 6 + 1 - 1, at least 1),
     "T misses EE". NONE (`nw-none/`): no move at the choice, no attack,
     the bar then drained. As 2.8 says.
   Not run: aggressive thieves and enemies at Wisdom below 6 or above 14
   (whether their arrow roll also goes); a player target of another
   Wisdom.
36. **Encounter areas with several players, deaths and saves inside, the
   2026-11-15 batch** (ours, no research pass; [encounters](rules/encounters.md)
   5.2, 6.1, 6.4, 7.4, 7.6, 8 and 2.4, 2.6; the golden test `two_sweep`).
   `ACSVM-research/capture/runs/rr32/` (the harness binary in `bin/`,
   `env.sh`). `build.sh` makes three disks of rr14's `patch` layout (the
   grass at 13..16, 10..12, map record 0 on it at 60 %) with T and U
   (`--player2`), started to T's first turn (`start.vsf`, its disk
   `disk1.d64`) and with U added (`two.vsf`, `add-second.txt`; T and U on
   12,12, both on joystick port 2): `pair` (HARE, a peaceful neutral, Speed
   3), `death` (WOLF, an aggressive brave enemy with Melee 100, T and U
   Life Force 6), `kill` (WOLF with Life Force 1, 50 gold, holding the
   blade; T Melee 100). `tin.sh` pushes T east with the random state 0, 1,
   .. until area 1 opens (`CASE/in/in.vsf` at the next turn);
   `step.txt`/`steploop.sh` do one player's step the same way (U's record
   is T's + 37: place `$F5D4`, square `$F5D1`,`$F5D2`). Turn orders are
   `run mem --ram` runs read with `roundcheck.py`. A `mem` run has no
   jitter: one run a case unless said.
   - **Item 1, a second player onto an open area's entrance** (`pair/join`,
     U pushed east from 12,12 onto 13,12, area 1's entrance, T inside):
     **U joins area 1** (place 1) **on 12,4, the same square as T**, no
     other area opened, no copy placed, its turn ended (moves 0).
     `pair/joindraws` (the same step, states 0..7, `--draws`): 8 of 8 had
     **no draw** between the step and the join (the first draw is T's
     next turn's). Engine (5.2): the same.
   - **Item 2, players in several areas, and the order** (`order/`, 130M
     to 150M idle each; all checks hold). The map region (`$397C` 0) holds
     everything, one counter, **one list a round** for the map and all
     areas: `split` (U on the map, T in area 1 with copy 8): **2 1 8**;
     `join` (T and U in area 1): **1 2 8**; `two` (`pair/u2`: U into area
     2 by 13,11, so T in area 1 with copy 8, U in area 2 with copy 9):
     **1 8 2 9**; `o2` (U into area 1 first, then T into area 2 by 13,11;
     copies 8 with U, 9 with T): **1 9 2 8**; `o3` (U in area 2, T out of
     area 1, which closed and freed slot 8, then T onto 13,11): **T
     joined area 2** (place 2, 12,4, its turn ended) and the order was
     **1 2 9**, U having been there first. So: the open map's players
     first; then the areas, **taken by their lowest-numbered player**, each
     its players **in number order** (not joining order) then its
     creatures by slot. Whether an area visit stops at 10 rounds can't
     show here: nothing else was in play, and region 0 never changed.
     Engine (6.1): map players in joining order, then areas 1 to 4, each
     its players in joining order and its creatures: differs in `o2` and
     `o3`.
   - **Item 3, a death in an area** (`death.sh`; `two-*` from
     `death/in/in.vsf`, U on the map; `alone-*` from `death/in1/in.vsf`, T
     alone; `--answer-text CONTINUE` presses through the hit messages).
     In all four the WOLF copy killed T (round 22 with U, 15 alone) and,
     **between the kill and the death message, area 1 closed**: size 0,
     entrance 41,41, slot 8 freed, T's place byte left 1. F1 (`two-f1`):
     U alone on the map every round from 23 to 30; (`alone-f1`) the
     add-a-player screen. The button (`two-button`, `alone-button`):
     **the last save**, U's adding at round 14 (T and U on 12,12, place
     0) or T's start at round 0 (T on 12,12): no save had been made in
     the area. Engine (7.4): the area closes, the usual death follows; the
     same.
   - **Item 4, SAVE GAME and Retire in an area.** `save1.txt` (from
     `death/in1/in.vsf`, T alone in area 1 in round 14, gold 77, SAVE GAME
     as rr24's `c1.txt`), then `save/s1-button` (`mem`, 200M): the save at
     round 14's end; the copy killed T in round 15 and **the button
     brought back round 15 with T in area 1 on 12,4 (place 1), gold 77,
     the area open (size 233, entrance 13,12) and the WOLF copy alive on
     its square at the save (8,3)**; the same round replayed 10 times. So
     the save holds the area, its entrance and the copy. `retire1.txt`
     (from `pair/in/in.vsf`: at T's turn in round 21, T in area 1 with the
     HARE copy, U on the map; RETIRE as `c2.txt`): the offer ("press
     button to save T") at the round's end, the area still open; after
     the button and "the adventure has been saved", **T's record retired
     (first byte 0) with its place 1 and square 12,4 kept, area 1 closed
     (size 0, 41,41) and the copy gone**; U's turn followed in round 22.
     Re-added from the list later (`retire/readd`, `add-second.txt` from
     U's turn; T's record alive again in round 29, gold 111): **T on
     12,12, the map's start, place 0**, not in the area. Engine (7.6):
     saves replay the log, so a restore comes back in the area as here;
     retiring closes an emptied area, as here.
   - **Item 5, all areas or all slots in use** (`full.sh`, from
     `pair/start.vsf`, states 0..15, a 300k push east, `--draws`, not
     `--ram`: runs 3 and 4 of `ctrl` and `slots` ended reading the ROM and
     are dropped). `ctrl`: every readable run met the HARE (area 1 on 13,12
     or 14,12). `areas` (all four areas set open by pokes): **16 of 16
     with no draw on any grass step**, T walking on to 14,12 and no area
     or slot changed (encounters 2.4). `slots` (slots 8..15 set alive in
     area 4, area 4 open, areas 1..3 free): **every grass step drew its
     roll**, hits included, and **no area opened**: T walked on with its
     moves, area 1 free, 14 of 14 (2.6). Engine: the same.
   - **Item 6, a creature and the area's door** (`hare.sh`, from
     `pair/in/in.vsf`, the HARE copy, a peaceful neutral, whose only goal
     is an exit, poked onto 1,5 beside the door on 0,5; states 0..7, 60M,
     two of its turns of 3 steps each): **it never stepped onto 0,5** in
     8 of 8 runs, its first step going north or south; in `order/`'s
     runs (150M) the copies wandered about rows 1 to 8 and never left. So
     a creature doesn't take the area door, nor seek it as an exit.
     Engine (6.4): creatures see the door as an exit and one that steps
     onto it is removed: differs.
   - **Item 7, a copy killed inside** (`kill1.txt` to `kill3.txt` from
     `kill/in/in.vsf`: T poked onto 3,3 beside the copy on 2,3, a push
     west, the hit message and "dies" answered). The copy's slot +0 went
     to 0 (its gold 50 and weapon left in the slot); **area 1's list in
     memory (`$E812` on) gained two entries first, on its square 2,3: the
     blade (thing 59) and a gold entry of 50**, the door after them; T's
     gold unchanged (0). T then out by 0,5: both entries **marked deleted
     (square 255)** at closing, the door kept (7.2). Engine: a dying
     creature drops what it carries and its gold as anywhere (combat), and
     closing deletes them: the same.
   - **Item 8, the two percentiles of `two`** (`scripts/encpair.py`, new,
     on rr15's `two/runs.jsonl`). Every run of the 256 took **one grass
     step**, and its draws follow the generator from the run's state and
     the raster lines alone (256 of 256: a = previous XOR the line's low
     byte, the logged line or the one before it, then a x 69 + 131): the
     first draw on line 270, the second on **27 after a hit, 38 after a
     miss, 216 after a redraw** (120, 80 and 56 runs). The counts are
     **none 45, WOLF alone 65, HARE alone 59, both 87** (the slots at the
     run's end say the same): 10.20's "WOLF alone in 40" was a slip
     (corrected there), and with it the 70 / 40 the golden test expects.
     First-roll hits 152 of 256 and second 146 of 256 give 86.7 for both
     if independent, against 87 seen: no correlation, and no rule (every
     record rolled in every run, as encountercheck.py already checks).
37. **Drops, gifts and the melee check, the 2026-11-15 batch** (ours, no
   research pass: [combat](rules/combat.md) 7 and "Captures needed" 3,
   [creatures](rules/creatures.md) 2.7, 7.3, 8.1.1, 8.1.2 and "Captures
   needed" 7 and 8, [magic](rules/magic.md) 7.2, the golden test
   `a_killed_copys_drop_lists_the_blade_first`).
   `ACSVM-research/capture/runs/rr33/` (the harness binary in `bin/`,
   `env.sh`; `case.sh` and `build.sh` build the cases of `cases.txt`, `snap`
   (rr20's chain to T's first turn's end) or `room` (T's first room turn
   under way); `build2.sh` the two-player disk `pu`; `sweep1.sh`,
   `sweep3.sh`, `sweep56.sh`, `queue2.sh`, `queue4.sh` the sweeps, `run mem
   --set-random-state --draws --ram`, at most three VICE; `burden.py` is
   rr28's, `giftsq.py` (new) tabulates where a drop or gift lands from the
   dropper and from T). No harness changes. New `scripts/blowcheck.py`
   checks the player's blows roll by roll (combat 3) with combat 7's roll
   of 0 to 9 by `--roll smart|aggressive|neutral`, `--kill-roll`,
   `--player-roll` and `--target u`; besides every value of the target it
   checks the attack's draws: from the attack's first to the change of the
   turn byte `$71FA` there are exactly the blows' draws, the rolls, and one
   roll of 0 to 5 more (in every run; read as the next mover's top-of-turn
   die), and after a kill the round's end (the slot roll, the percentile)
   ending before T's next bar fills. It passes rr8's `player-friend` and
   `player-neutral-set` (12/12 each). `scripts/dropcheck.py` gains the case
   `drop-store`.
   - **Item 1, the roll after a player's damaging blow** (`pb`: T Speed 6,
     Dexterity 31, holding our blade, on 4,4 with 5 moves at its first room
     turn, so 5 blows; FF on 5,4, Speed 0, Constitution and Life Force 40,
     holding nothing; the stick east for 300k from `pb-room.vsf`, as rr8's
     `push.sh`; FF's class byte `$E1A7` set to 16 x class and its Wisdom
     `$E1B4` per sweep). `blowcheck.py`, every draw:

     | Sweep | Target | Runs | Damaging blows | Fits (`smart`) | Without the roll |
     | --- | --- | --- | --- | --- | --- |
     | `b-c4` | aggressive thief, Wisdom 10 | 32 | 34 | **32/32**, a roll after each | 8/32 |
     | `b-c6` | aggressive enemy, Wisdom 10 | 32 | 34 | **32/32**, a roll after each | 8/32 |
     | `b-c5` | aggressive neutral, Wisdom 10 | 32 | 34 | **32/32** (control) | 27/32 (the roll only while still a neutral) |
     | `b-c4w3`, `b-c6w3` | Wisdom 3 | 32 each | 40 | **32/32 with no roll** | 9/32 with one |
     | `b-c4w20`, `b-c6w20` | Wisdom 20 | 32 each | 37 | **32/32 with no roll** | 8/32 with one |
     | `k-c4`, `k-c5`, `k-c6` | aggressive thief, neutral, enemy at Life Force 1 (the first damaging blow kills) | 16 each | 13 kills | **16/16 with the roll after the killing blow** | 6, 5, 6 of 16 |
     | `k-c1`, `k-c3` | peaceful neutral, peaceful friend at Life Force 1 | 16 each | 13 kills | **16/16 with no roll** | 13/16 with one |
     | `u-w10` | the second player U (`pu`: T Speed 12, Dexterity 31, holding the blade, on 5,4 at its second room turn with 12 moves, 6 blows; U Wisdom 10 beside on 4,4; the stick west) | 32 | 50 | **32/32 with a roll after each** (`--player-roll yes`) | 4/32 |

     So **a player's damaging blow draws combat 7's roll exactly as an
     arrow does**: for an aggressive thief, neutral or enemy at Wisdom 6 to
     14, for another player, and after a blow that kills (by the class
     before the blow); not at Wisdom 3 or 20 (aggressive thieves and
     enemies included: the arrows' open case, settled for blows), and not
     for a peaceful creature or a friend. The engine's one rule for blows
     and arrows holds. Seen on the way: with FF's Life Force poked to 100
     over its Constitution 40 (a first `b-` sweep, dropped), the damage
     split between Life Force and Power fitted neither 4.2's share from the
     Life Force nor one capped at the Constitution: not looked at further.
   - **Item 2, a killed creature's drop order** (golden test
     `a_killed_copys_drop_lists_the_blade_first`). From rr32's
     `kill/k2/b.vsf` ("EE dies" up, T on 3,3 beside the copy's square 2,3
     in area 1; `atop.txt`): area 1's list read `14 0 | 35 59 1 | 35 81 0
     254 50 0 | 5 82 0`: **the blade first, then the gold of 50, then the
     door**, the door having been the list's only entry before. At T's
     next turn the push west onto 2,3 gave **"T cannot carry more blades"**
     (T owned the blade already): the blade is the square's top thing.
     The same in an ordinary room (`dk`: EE, Life Force 1, 50 gold,
     holding our blade and owning two treasures of ours, things 50 and 51,
     on 5,4; T Melee 100 holding the blade; `dk.txt`, random state 0): the
     room's list (`$382A` = `$E585`, the length at `$E583`, empty before)
     became `17 0 | 84 59 1 | 84 51 1 | 84 50 1 | 84 81 2 254 50 0`: **the
     blade, the treasure 51, the treasure 50, then the gold**, and T's push
     onto 5,4 again "cannot carry more blades". With the area's door kept
     after the new entries, the list is top first (movement 3.3): the dead
     creature's things go down **gold first, then its possessions from the
     lowest number up**, the highest-numbered on top. The engine's choice
     (the gold on top) disagrees.
   - **Item 3, an overburdened creature with a foe beside** (creatures
     8.1.2). `fb-cN`: rr16's `drop-corner` (WE, Speed 2, Strength 1,
     carrying 400: 0 moves) set onto 5,4 beside T on 4,4 (as 10.24's
     `drop-beside`, a friend: 128/128 dropped), its class byte set to 2 + 16
     x class, 16 runs each: **no drop in any run** for the peaceful thief,
     neutral and enemy (c0 to c2) and the aggressive thief, neutral and enemy
     (c4 to c6, `fb-c6b` brave too, `fb-c6own` also owning the blade);
     **16/16 drops** for the aggressive friend (c7), its landings fitting
     `dropcheck.py` with **two** smartness checks before the pairs (16/16;
     one check 0/16). `fbn` (a peaceful neutral of ours placed on 5,4, T on
     4,4): no drop, 16/16. With moves left (`wa-cN`: rr28's `burden2`, WA
     Speed 6, load 150 of 200, set beside T; 8 runs each): as a friend
     (c3) its turn began with 3 moves, dropped, 2 steps (10.32's); as a
     peaceful neutral or an aggressive enemy (c1, c6) it began with **1**
     move, **no drop**, 1 step. A creature beside instead (`wb-X`: WB set
     onto 11,2 beside WA, T far): WB an **aggressive enemy** (102): WA began
     with 1 move, no drop, 1 step, 8/8; WB a **peaceful enemy** (38) or an
     **aggressive friend** (118): WA began with 3, dropped, 2 steps, 8/8.
     So the foe beside **stops the drop** and is 2.6's test, the one that
     halves the moves: an aggressive individual not a friend, or a player
     beside a creature that is not a friend, on the 4 squares beside.
     Diagonal does not count: `fbe` (an aggressive enemy of ours on 5,4)
     left T on 4,5 after its walk in, and dropped 16/16; swapping the class
     bytes of `fbe` and `fbn` before the run (`fbe-c1`, `fbn-c6`) changed
     nothing (16/16 and 0/16), the square deciding. Engine (8.1.2, 2.6's
     test): the same.
   - **Item 4, readying with no moves left.** `r0b` (WE Speed 2, Strength
     1, carrying 400, owning our bow, not holding it, on 9,4; T on 4,4,
     nobody beside; 16 runs, `r0b-long` 8 runs of 22M): the drop turn (0
     moves) **dropped and did not ready** (`$E1B7` 0 throughout the turn,
     16/16 and 8/8); its next turn readied the bow (`$E1B7` 41) and stepped
     on, 8/8. So a drop that leaves 0 moves ends the turn there, with no
     readying: the engine's choice (it readies all the same) disagrees.
     `rmb` (WE the same owning the blade, a peaceful neutral on 5,4 beside
     T on 4,4, so no drop: 0 moves by its load and nothing dropped; 16
     runs, 14M): **the blade readied** (`$E1B7` 59) and then only the turn's
     two closing draws (28,000 instructions apart, 10.33), no step rolls,
     no attack (`$B799` never written), 16/16; its load and square
     unchanged. So a creature of Speed above 0 slowed to 0 moves by its
     load readies and stops there, as the engine says (2.7's readying stop).
     (`r0m`, the blade with an aggressive enemy, left T diagonal: not a
     case of a foe beside; it dropped and did not ready, 16/16.)
   - **Item 5, a store and a drop** (`dst`: WE as `drop-wall` on 6,4, the
     region's store on 5,4; 256 runs). `dropcheck.py --case drop-store`:
     **256/256 with the store taking the thing**, 49 drops landing on the
     store's square (5,4 49, 6,3 37, 6,4 83, 6,5 37, 7,4 50); with the store
     refusing, 207/256. So a store does not refuse a drop, as the engine
     says (8.1.1).
   - **Item 6, a friend's gift** (`g5`: GF, a peaceful friend of Strength
     20, Speed 3, one treasure of weight 1, on 5,4 beside T on 4,4; 64
     runs). The gift is the turn's first action, with the overburdened
     drop's draws: `dropcheck.py --case drop-beside` (one smartness check,
     then the pairs, around 5,4) **64/64**: on its own square 24, 4,4 (T's
     square) 6, 5,3 8, 5,5 13, 6,4 13. `g7` (GF on 7,4; 64 runs): it stepped
     to 6,4 and 5,4 with two moves and gave with the third (`$E1C4` to 0 at
     the third move), in 38 runs; the landings around 5,4 (own 13, 4,4 9,
     6,4 9, 5,3 3, 5,5 4), never on T's other neighbours 3,4, 4,3 or 4,5.
     So **the gift's square is 8.1.1's choice around the giver**, as the
     engine (following the research) says; a giver that arrives beside
     the player gives with a move left in the same turn.
   - **Item 7, a Speed 0 creature beside a thing it takes or an exit.**
     `s0g` (TT, a peaceful thief of Speed 0 on 8,4, 50 gold on 9,4; 16
     runs): it **took the gold on its first turn** (wealth `$E1B5` 50),
     16/16, without moving. `s0x` (NN, a peaceful neutral of Speed 0 on 6,4
     beside the side room's portal on 7,4; `s0x-long`, 16 runs of 40M, four
     turns each): it **took the portal once**, at its third turn of run 12
     (into the side room, 2,4), and otherwise stayed; the same creature set
     to Speed 2 (`s2x-long`, `--set E1A7=18`) went through it in 2 of 16
     runs and otherwise wandered. So the Speed 0 step is not cancelled onto
     a thing it takes or an exit, as traced (2.7); how often an exit is
     chosen is the goal and path choice's.
38. **Characters, a heavy take and saves, the 2026-11-15 batch** (ours, no
   research pass: [creatures](rules/creatures.md) 2.2.7, 2.2.7.4 and
   "Captures needed" 1, [characters](rules/characters.md) 6.2, 7.3, 7.5
   and "Captures needed" 1, 3, 5, 6, [inventory](rules/inventory.md) 3.4
   and "Captures needed" 1). `ACSVM-research/capture/runs/rr34/` (the
   harness binaries in `bin/`, `env.sh` with `sc` for scripts and `mm` for
   `run mem --ram`; `gen.py` writes the scripts in `i1/` to `i5/`;
   `tin.sh` runs rr32's `tin.txt` until area 1 opens; `i4/build.sh`,
   `i4/run.sh`, `i4/sum.py`; `i5/build.sh`). New scenario option
   `--may-add THING:N` (repeatable; `scenario::may_add`) sets a thing's
   "ACS may add" count (thing block + 0x800 + thing - 1, high nibble).
   Turn orders read with `roundcheck.py` (all checks hold in every `mem`
   run here). Restores were brought about by a death, or by poking a
   living player's first byte to 0 at its turn (the round-end message of
   characters 6.3, answered with the button), with gold marks poked at
   each step; a `mem` run has no jitter: one run a case. Every add-player
   screen, retire offer and save message was read off the screenshots
   before the next step was scripted.
   - **Item 1, an add with a room region visited and a player on the
     map** (`i1/i1b.txt` from rr24's `c3/t60.vsf`: T's turn in round 60,
     the first round of region 1's visit that began with a region change,
     U on the map, EE a friend; T asks ADD SOMEONE; at round 60's end the
     add-player screen, CREATE NEW PLAYER, N; `i1b-order`, 300M idle):
     **region 1 went on, rounds 61 to 69 (T, EE, FF), then the map from
     70 with U and N** (N in slot 3). The visit goes on; no new cycle.
     Then (`i1c`: T's gold 61 and 62 at its turns, EE an enemy at 62; EE
     killed T in round 63; `i1c-button`): **the counter back to 60, T's
     gold 9, N's slot empty**: the region-1 visit's start, **not** the
     add's round end. The created player's add saved nothing.
     The same on rr32's `death` disk (`i1/i1d.txt`: T's first turn, round
     13, ADD SOMEONE, N created at round 13's end, no save message on any
     screen after the name; T into area 1 at round 21, `i1d-in`; the WOLF
     copy killed T; `i1d-button`): **back to round 0, the game's start,
     N gone**. 10.36 item 3's add brought back round 14 because it took U
     **from the roster**, and that add shows "saving game", "the adventure
     has been saved" (`add-second.txt`): the roster add writes the game,
     a created player's add does not.
   - **Item 2, an add with SAVE GAME or a Retire with Save, refused adds.**
     From rr32's `death/start.vsf` (T's first turn, round 13; U retired
     in the roster): ADD SOMEONE then SAVE GAME (`i2s`), or SAVE GAME then
     ADD SOMEONE (`i2s2`), in the one turn (the menu and its messages use
     no move; a `settle` at the prompt waits the bar out, so the scripts
     wait 0.5M there instead). Both orders: at round 13's end **the
     add-player screen first**, then after the add **one** "saving game",
     "the adventure has been saved". With N created (`i2s-n`) or U from
     the roster (`i2s-u`), T then into area 1 and killed (`*-in`,
     `*-button`): **back to round 14 with T and the newcomer alive**, gold
     13. So a SAVE GAME of the round saves after the add, the newcomer in.
     **Retire with Save and an add in one turn** (`i2r` from rr32's
     `death/two.vsf`, T and U on the map, T's turn in round 20: RETIRE,
     then ADD SOMEONE; gold 21 and 22): at round 20's end **the retire
     offer first** (under U's strip), the button, "saving game", "the
     adventure has been saved", **then the add-player screen** (T now
     listed, the cursor on it). N created (`i2r-n2`, N in slot 1, T's
     freed slot), then U's first byte poked 0 at round 23 and the message
     answered with the button (`i2r-n2-button`): **back to counter 21,
     slot 1 empty (T retired), no N, the gold 21 and 22**: the retire's
     save, the created newcomer not in it. T picked back from the roster
     instead (`i2r-t`, a save message after the pick; `i2r-t-button`):
     **back to 21 with T alive in slot 1**: the roster add's save is the
     later one.
     **A refused add** (`i2fa` from rr19's `move-basic-start.vsf`: rr19's
     `four-full` pokes make slots 2 to 4 living at round 4, its message
     answered with F1; at T's turn in round 6, gold 51): ADD SOMEONE once
     (`i2fb1`) or twice (`i2fb2`): **"I will add a player as soon as
     possible" each time**; at round 6's end **one** "you will have to
     retire one of your adventurers before you may add another", the
     button, and play on (the next screen the map, no second refusal).
     Slot 4's first byte then poked 0 at T's turn (round 9 or 10) and the
     message answered with the button (`i2fb1-button`, `i2fb2-button`):
     **back to round 0, the game's start, gold 0**: the refusal saved
     nothing. Two requests with no free slot give one refusal.
   - **Item 3, the all-dead save on the map** (`i3/i3a.txt` from rr32's
     `death/in1/in.vsf`: T alone in area 1, round 14; T poked onto 3,3
     beside the WOLF copy, Life Force 1, gold 77, SAVE GAME; the copy
     killed T in round 14; the death message answered F1 by `run mem
     --death-f1`, `i3c`): "saving game", "the adventure has been saved"
     (the save with nobody alive), then the add-player screen; N created
     (`i3d`, slot 1, on 12,12 of the map, region 0 throughout, gold 88
     poked at round 20); N into area 1 (`i3e`), the copy killed N in round
     22, the button (`i3e-button`): **counter 15, gold 77, the slot's
     square 3,3: the all-dead save came back**, then the slot cleared,
     nobody alive, and **the add-player screen again** (N lost). With no
     region change after it, the all-dead save is restored.
   - **Item 4, where a heavy take ends the turn** (`i4/build.sh`: the
     `loose` scenario, `--no-item --player spd=12,str=9,con=20,lf=20
     --take-weights W1,W2`, six disks `a` 100,240, `b` 110,130, `c`
     150,170, `d` 190,210, `e` 225,239, `f` 40,70, started to T's first
     room turn; `i4/run.sh DISK S`: T's Strength byte `$F5A0` poked to S
     (bit 6, the port, kept), so the capacity is S x 100; at a fresh turn
     (12 moves) a push west takes the thing on 3,4, the button; at the
     next fresh turn a step west and a push take the one on 2,4; the
     moves and the counter read after each). Strengths 3, 5, 9, 20, 31
     on every disk and 6, 7, 8 on some: **36 runs, 72 takes, every one as
     "the turn ends when the thing taken weighs more than a quarter of the
     capacity"** (`i4/sum.py`): capacity 300: 40 and 70 went on, 100 and
     up ended; 500: 110 on, 130 and up ended; 600: 130 and **150 on**, 170
     ended; 700: 150 and 170 on, 190 ended; 800: 190 on, 210 ended; 900:
     100 to 210 and **225 on**, 239 and 240 ended; 2,000 and 3,100: all
     on. The load after the take does not decide it (load 240 at capacity
     900 went on, the thing 130; load 340 ended, the thing 240), nor the
     slowdown (130 at capacity 500, load 240, ended unslowed). A take that
     goes on costs one move, as section 7 found. Not tried: wounds (Life
     Force below Constitution) in the capacity.
   - **Item 5, Destroy in a room** (`i5/build.sh`: move-basic with T,
     Speed 6, owning rr19's kit, and AA a Speed 0 friend in the room;
     `m` with `--may-add 59:1 --may-add 2:0`, `c` as built: blade 0,
     dagger 4; `d5a.txt`: into the room, at T's first room turn
     possessions blade 59, dagger 2, GOODA 13, bow 41, nothing held or
     worn, no gold; RETIRE; the offer at round 14's end (`d5o-c/offer.vsf`);
     F1, the button, F1). Both as the last player: **blade, bow, GOODA and
     dagger all on 4,4** (the list read at the destroy: 59, 41, 13, 2, top
     first), on `m` (`d5a-m`) and on `c` (`d5a-c3`; `d5a-c`, `d5a-c0` to
     `d5a-c2` lost an F1 and saved or did nothing). So the count changes
     nothing, and the last player loses nothing a player with company
     keeps. 10.30's "last player" runs came from rr19's `last1`, on the
     `catchup` disk, where things 59, 13 and 14 are **CURSE (a spell),
     STONE TOUCH and FIREBREATHING (creatures' weapons of weight 0, not
     usable by anyone)** (`acs dump`, `dump/`), not our blade and
     treasures: their loss was rule 1 (spells and creatures' weapons).
     With U alive **on the map** and T alone in the room (`d5b1` from
     rr19's `nd-two/nodisk.vsf`, T alone south 3.2M, the kit poked;
     `d5b2`: RETIRE at T's turn in round 41, destroyed at its end): **the
     kit, all six, on 4,4** (blade, bow, mail, GOODB, GOODA, 300 gold), as
     with U in the room. **A destroy is not a save point**: with T and U
     in the room (`d5c`, both south 9M; U's gold 11 at T's RETIRE in round
     23, the destroy at round 24's end, U's gold 22 and its first byte
     poked 0 at round 27; `d5c-button`) the button brought back **counter
     22, T alive with its kit, U's gold 0**: the room visit's start. The
     disk written in `d5b2`'s next round (the adventure block's round 42,
     T's first byte 0, the room's list) is the map visit's own save after
     the region change.
   - **Item 6, the full roster's Life Force** (`i6/full2.txt`, rr26's
     `d3-full34` again from its `in3/ret1-offer.vsf` and `full34.d64`, T's
     Life Force 5 of Constitution 11 at the offer): "Sorry. The disk is
     full", the button: **Life Force 5 at once, 6 a million instructions
     later** as T's turn began (the counter still 22), 11 only two rounds
     on (rr26). It is not raised: 10.30's 11 was healing.
   Not run: a reversal waiting across a destroy; a roster add's effect on
   the visit order (item 1 used a created player).
39. **Goals and exits, wounds and capacity, the 2026-11-16 batch** (ours,
   no research pass: [creatures](rules/creatures.md) 2.7, 6.2 to 6.5 and
   "Captures needed" 4 to 6, [combat](rules/combat.md) 4.3,
   [inventory](rules/inventory.md) 3.2 and "Captures needed" 2, the golden
   test `a_speed_0_creature_takes_a_thing_or_an_exit_beside_it`).
   `ACSVM-research/capture/runs/rr35/` (the harness binary in `bin/`,
   `env.sh`; `case.sh` and `build.sh` build the cases of `cases.txt`, as
   rr33's; `qa.sh` to `qd.sh` the sweeps, `run mem --set-random-state`, at
   most three VICE; `turnraws.py`, `nnturns.py`, `timeline.py` print a
   run's draws cut by the turn byte). No harness changes. New
   `scripts/goalcheck.py` checks one creature's goal and steps turn by
   turn, draw by draw (6.2 to 6.5, 2.7's Speed 0 step), with the goal
   rule, the exit's cost (`--exit-cost`, and `--exit-first` on the turn's
   first step), smartness rolls (`--pre`), halving (`--hinder`) and the
   player followed through a portal (`--player-track`) as options. New
   `scripts/capcheck.py` reads the first player's moves at each turn's
   start against the slowdown under eight readings of the wound term.
   `blowcheck.py` gains `--share life|con` (combat 4.2's share from the
   Life Force as it stands, or capped at the Constitution). Every creature
   below has Wisdom 3: no smartness roll was drawn (`--pre 0` fits, 1 or
   2 fit nothing).
   - **Item 1, a creature beside an exit** (the golden test; 10.37 item
     7). Read draw by draw, a Speed 0 turn of rr33's `s0x-long` draws the
     top-of-turn die just before the turn byte names the creature, then
     **four** step rolls of 0 to 2 (south, north, east, west: the portal
     on 7,4 is one of them), then the round's end (the slot roll and the
     percentile). The spec's step score (6.5) with the slinker's exit cost
     1 makes the exit (3 x 0 + 5 x 1 + 4 x roll = 5 to 13) beat every
     other square (16 to 24) on every turn, so the creature would always
     go through; it went in 1 of 63 turns. The runs fit **one rule**: **on
     the turn's first step an exit costs 4**; on later steps the spec's
     cost (1 for a slinker, 3 for a fighter). The Speed 0 step onto an
     exit is not cancelled (2.7), as traced. Each case below from 6,4
     beside the portal with T on 4,4 is rr33's `s0x-turn-end.vsf` with the
     class byte `$E1A7` (16 x class + Speed) and Wisdom `$E1B4` set;
     `ta-*` puts NN on 7,5 below the portal (`x-ta-room.vsf`, T's first
     room turn) and walks T through it (`--push e:300k`), so NN's one turn
     (a creature in a room no player is in took no further turn in 30M)
     has no player in the room; `s26-*` puts NN two squares off, on 7,6
     (`x-26-turn-end.vsf`).

     | Runs (`goalcheck.py`) | Creature, goal (6.2) | Runs, turns | Through | First step 4, then 1 or 3 | The spec (1 or 3 always) | 4 always |
     | --- | --- | --- | --- | --- | --- | --- |
     | rr33 `s0x-long`, `x-pn0` (states 16-47) | peaceful neutral, Speed 0, the exit | 48, 188 | 3 | **48/48** | 0/48 | 48/48 |
     | rr33 `s2x-long` | the same at Speed 2 | 16, 46 | 2 | **16/16** | 0/16 | 16/16 |
     | `x-pf0`, `x-pf2` | peaceful friend, Speed 0, 2: the player (28 against 33) | 64, 224 | 0 | **64/64** | 31/64 | 64/64 |
     | `x-an0` | aggressive neutral (fighter), Speed 0: the player (rank 1) | 16, 64 | 0 | **16/16** | 16/16 (3) | 16/16 |
     | `ta-pn0`, `ta-pf0` | peaceful neutral, friend, Speed 0, the player gone: the exit | 96, 96 | 3 | **96/96** | 3/96 | 96/96 |
     | `ta-an0` | aggressive neutral, Speed 0, the player gone: the exit (rank 96) | 48, 48 | 1 | **48/48** | 30/48 (3) | 48/48 |
     | `s26-pn2` | peaceful neutral, Speed 2, from 7,6: the exit, beside it at the second step | 32, 32 | 23 (all at the second step) | **32/32** | 32/32 (the exit never beside at a first step) | 13/32 |
     | `s26-an2` | aggressive neutral, Speed 2, from 7,6: the player; the exit beside at the second step | 32, 32 | 2 | **32/32** (3 later) | 32/32 | 30/32 (1 later: 12/32) |

     rr3's `follow` runs (10.14.1: a slinker friend stepping onto the
     portal 41 times, never at a first step) fit `creaturecheck.py`'s exit
     cost 1 in 64 of 64 runs, 2 in 58, 3 in 45, 4 in 24: the later steps'
     cost is the spec's. So in the 64-turn `s0x` series the creature went
     through only when its exit roll was 0, those of south and north 2
     and of west at least 1 (the exit then scores 20, the others 24, 24 and
     20 or more, a tie going to east before west): 1 turn in 27 by the
     rolls, 1 of 63 seen. Engine (creatures 6.5, 2.7): the exit costs 1
     (slinker) or 3 (fighter) on every step, so its Speed 0 neutral goes
     through on its first turn every time; disagrees. This is likely 6.5's
     open case ("on a turn's first step some squares holding a thing cost
     4") for exits; gold and treasures were not tried here.
   - **Item 2, fighters' goals among other individuals** (creatures
     "Captures needed" 4). EE, an aggressive enemy of Speed 3 owning no
     weapon, Wisdom 3, with T on 4,4 and FF, a friend of Speed 0, in
     reach; 32 states each, 30M (two or three of EE's turns):
     `g-f` (EE on 12,4, FF peaceful on 12,5: FF scores 6 + 1 = 7, T 0 + 8
     = 8), `g-t` (EE on 11,4, FF on 11,5: 7 against 7, a tie), `g-a` (EE on
     12,4, FF aggressive on 12,6: 4 + 2 = 6 against 8). `goalcheck.py`
     with 6.2's rule (rank + steps from the turn's first square, the
     lowest, a tie to the player looked at first), individuals' squares
     closed, halving beside T or the aggressive FF: **g-f 32/32** (64
     turns: FF the goal in 60, T in 4 once EE had wandered off), **g-t
     32/32** (70 turns, T in 69, FF in 1), **g-a 32/32** (91 turns, FF in
     89). "Always the player" fits 3, 32 and 2 of 32; "always the friend"
     28, 1 and 30. Engine (6.2, 6.3): the same.
   - **Item 3, the damage share with Life Force above Constitution**
     (combat 4.3). From rr33's `pb-room.vsf` (T, Dexterity 31, holding our
     blade, pushing east into FF on 5,4): FF a peaceful neutral (`$E1A7`
     16), Wisdom 10, its Constitution `$E1A3` set to 20 and its Life Force
     `$E1A6` to 20, 26, 40 or 60 (`d-20` .. `d-60`, 32 states each,
     1.8M). `blowcheck.py --cls 1 --con 20`, every blow and every Life
     Force and Power value: the share **from the Life Force as it stands**
     32/32 at each level (33, 33, 33, 32 damaging blows); capped at the
     Constitution 32/32 at 20 and 26 (the two agree there) but **6/32** at
     40 and at 60. So Life Force above Constitution enters the share as it
     is, as 4.2 is written and the engine does. 10.37's misfit most likely
     came from the poke: 100 does not fit the 6 bits a Life Force has (100
     is 36 in the low six).
   - **Item 4, capacity with Life Force above Constitution, and odd
     wounds** (inventory 3.2, "Captures needed" 2). `c4`: T of Speed 15 in
     the room (`c4-room.vsf`), owning thing 71 of weight 500 by its
     possession bit (`$F5C2` = 2, the row thing on 5,4 left there), its
     Strength `$F5A0` (the port bit 64 kept), Constitution `$F59F` and
     Life Force `$F5A2` poked; `capcheck.py` reads the moves byte at each
     turn's refill with the Life Force at that moment (at the turn byte's
     change instead, before the top-of-turn die's heal, fits 12 of 63). A
     load of 500 is above half of every capacity below 1,000, so the moves
     (Speed x 2 x (capacity - 500) / capacity) tell capacities 50 apart.
     `c-s9l20` .. `c-s9l30` (Strength 9, Constitution 20, Life Force 20,
     21, 22, 25, 30; 3 runs of 90M each; the Life Force never moved):
     moves 13, **15**, 15, 15, 15. `c-s12c40`, `c-s12c41` (Strength 12,
     Constitution 40 or 41, Life Force 27 or 28, healing over 240M, 6 runs
     each): wounds 13 down to 0, moves 5, 5, 8, 8, 11, 11, 13, 13, 15 ...
     (each odd wound as the even one below it). Of 169 turn starts the
     wound term **halved rounding down** (toward minus infinity, so -1 adds
     100) fits **169**; halved toward zero 160 (it misses Life Force 21);
     rounding up or no rounding 111; any reading that counts no wound below
     0 at most 133. So capacity = (Strength - floor((Constitution - Life
     Force) / 2)) x 100, Life Force above Constitution adding to it.
     Engine (3.2): halving toward zero, unclamped; it differs only at an
     odd negative wound (Life Force 21 of 20: 900 against the original's
     1,000).
   Not run: item 5 (two random creatures left at once; the adjacency
   slowdown mid-turn and with odd Speeds). `ta-an2`, `ta-pn2` (Speed 2
   with the player gone) ran too short to finish a turn (2 turns of 96).
40. **Magic actions, spaces and winning, the 2026-11-16 batch** (ours, no
   research pass: [magic](rules/magic.md) 3.1, 3.2, 3.13, 3.14 and
   "Captures needed" 3 to 6, [movement](rules/movement.md) 5.5, 6.3, 7.2
   and "Captures needed" 2, 3 and 5, [inventory](rules/inventory.md) 2.5,
   4.3, 5.1, 5.4, 5.7, 5.9 and "Captures needed" 3 to 5,
   [winning](rules/winning.md) "Captures needed" 1).
   `ACSVM-research/capture/runs/rr36/` (the harness binary in `bin/`,
   `env.sh` with `sc`; `case.sh` and `build.sh` build the cases of
   `cases1.txt` and `cases.txt` to T's first room turn under way on 4,4,
   `room.vsf`; `cast.txt` and `gen.py` cast spell 1 at cost 0 (no roll)
   with cursor taps, `drop.txt` drops, `armor.txt` NEW ARMOR,
   `py/steps.py` writes push scripts, `peek.txt` reads the arrival;
   `py/look.py` reads T's record, the 16 creature slots and the room's list
   from a log, `py/poll.py` a slot's timeline, `py/rc.py` the sweep
   `rc-sw`). At most three VICE of ours. New scenario options: `--put
   SLOT[=HEX8][@SQ/SQ..]` (repeatable; `scenario::Put`) writes a master
   thing's four property bytes (named `PUT` and two letters, messages
   cleared) and places it on room 0 squares `X,Y[,P]` (or the side room's,
   `sX,Y[,P]`), before the room's other things, so on top, earlier puts
   above later ones; `--master N:SPEC` (`scenario::master`) writes master
   creature N from the `--creature` keys (picture 74, the template's class
   kept). The menu (room or map): MOVE, USE OBJECT, NEW WEAPON, ADD
   SOMEONE on the first row; REST, DROP OBJECT, NEW ARMOR, RETIRE on the
   second; FIRE, USE POWER, PROFILE, SAVE GAME on the third. The SELECT
   SPELL and drop lists run left to right after NONE (a tap east, not
   south). Every cast here asked "move cursor to target and press button",
   room spells included; the cursor starts on the caster. Each case is
   one deterministic run (no roll is involved), read from RAM and the
   screenshots.
   - **Item 1, Kill All** (spell 0:1:59, "kill all but owners of" our
     blade; `ka`: T owning only the spell, AA (owning the blade) on 8,2,
     BB on 10,6, CC in the side room, all Speed 0 friends; `kb` the same
     with T owning the blade). Cast at T (`ka-c`): **"T dies"**, T's Life
     Force 0 and its record cleared; AA, BB and CC untouched. Cast at BB
     (`ka-bb`): **BB's Life Force 0**; T, AA and CC untouched. Cast at AA
     (`ka-aa`), or by T owning the blade at itself (`kb-c`): **nobody
     dies**. So the spell kills **its target only, unless the target owns
     the thing**: no one else in the room, the side room or the region.
     The engine's choice (everyone in the room, the caster spared only as
     an owner) disagrees.
   - **Item 2, Summons** (spell 0:2:5; master creature 5 ours, ZED, an
     enemy, Speed 3, Constitution 12; `sa`, the room empty). The creature
     is made **at the cast**: while the target prompt shows (`sb15-c`), or
     at the first press (`sa-p2`), the **lowest free slot** holds a copy
     of the master record (class byte 35: the master's enemy alignment and
     Speed; Constitution 12; room byte 16), its square still the master
     record's 8,2. The press puts it **on the target square**, in the same
     round 14: an empty square (`sa-p66`: 5,6; `sb15-c`: 6,6), a square
     with a creature on it (`sb15-27`: SB's 2,7, shared) or, after a first
     press there that left the prompt up, T's own square (`sa-p2`: 4,4;
     `sa-p3`: the cursor moved after that press gave 6,4). It took its
     first turn in round 15 (`$71FA` 16, slot 0) and walked off. With
     slots 0 to 14 in use (`sb15`) it took **slot 15**; with all 16 in use
     (`sb16-c`): **"A few sparks fizzle but TESTSPELL1 fails"** at cost 0,
     no prompt, no slot changed. So there is no wait for a later turn and
     no reserved square to step onto: the creature appears on the chosen
     square at once. The engine's choices (the next player turn, the
     caster's square or the nearest free one) disagree on the time and the
     square; the lowest free slot, the master's alignment and a full
     region refusing agree.
   - **Item 3, Add to Room** (spell 0:13:60, our treasure 60). Cast at T
     on 4,4 (`aa-c`): the room's list became `4,4:60/1`, **on the
     target's square**; T standing on a space there (`ab-c`): **on top of
     the space**; cast at the empty 6,6 (`aa-66`): **on 6,6**. A space that
     adds when moved onto (`ac-s2`, on 5,4): T stepped onto it and the
     thing went on top there (`5,4:60/1 5,4:104/0`). An obstacle that adds
     when bumped (`ad-s`, on 5,4): T stayed on 4,4 and the thing went **on
     top of the obstacle's square**. The count is 1. So the thing lands on
     top of the square where the spell takes effect, a cast's target
     square; the engine's choice agrees for spaces, obstacles and a cast
     at the caster, not for a cast at another square (it takes the
     caster's).
   - **Item 4, Activate All** (`acto`, T owning the blade, and `actn`, T
     not; on 5,4, top first: a space running Activate All; a custom space
     "when moved onto" giving 61; one "open to owners of" the blade giving
     62; one "when dropped here" (the blade) giving 63; a second Activate
     All space; a space giving 64; things 61 to 64 ours). T stepped onto
     5,4 from 4,4: owning, **61, 62 and 64** given; not owning, **61 and
     64**; 63 never; T stood on 5,4 either way (the top space lets anyone
     pass). `actp` (the last space changing Power by +5 instead): **Power
     13 to 17**, the +5 once (with the turn's drift), not twice: the
     nested Activate All does not run the rest again. The engine's choices
     (a condition met runs, "when dropped here" skipped, a nested Activate
     All skipped) agree. Not tried: an Activate All cast away from a stack.
   - **"When dropped here" without "destroy needed thing"** (inventory
     5.7; `dh`: such a custom space on 5,4 for the blade, giving 61, T
     owning the blade; `dhd` the same with the destroy bit). DROP OBJECT,
     the blade, the cursor east ("move cursor to drop spot"): `dh-e`:
     **61 given and the blade on top of the space** (`5,4:59/1
     5,4:107/59`); `dhd-e`: **61 given and the blade gone**. The engine
     agrees.
   - **Item 5, arriving and barring.** The map door leads to 4,4; on it
     (`peek.txt` at T's first room turn) a space giving 61 (`arsp`), an
     obstacle giving 61 when bumped (`arob`), a portal (`ardo`, a copy of
     the side room's door) or a custom space open only to owners of the
     blade, T not one (`arcs`): T **stood on 4,4 each time, nothing
     given, no door taken**. Stepping off and back (`arsp-ew`) gave 61;
     from 5,4 a push west into the obstacle (`arob-ew`) gave 61, T staying
     on 5,4. A space open only to owners of the blade on 5,4, T not one
     (`bar`): the push east left T on 4,4 and took no move (`bar-p`
     against an idle control `bar-c`, `$719A` read every 300,000
     instructions: the first fall came about 1.2 million instructions
     later than the control's, the drain held while the push was
     answered, then both fell alike; the next step south cost one move in
     both). So arriving by a door triggers nothing and chains to nothing
     (movement 6.3's choice), and a barring space costs nothing, like
     other refused moves. Not tried: a creature's step onto a space,
     arriving by a room door.
   - **Item 6, drops and readying.** The drop cursor starts on the
     dropper and moves only to the four squares beside it: a second tap
     (`dh-ee`, `dh-es`) or a tap toward an obstacle (`dk2-e`) left it
     where it was. **The own square takes a drop** (`dh-own`: the blade
     on 4,4 under T). `dk2` (T owning the blade and mail, on 4,4; our
     obstacle on 5,4, a copy of the side door on 3,4, the store on 4,5,
     AA on 4,3; the list's first entry is the mail): the mail landed **on
     T's own square** when the cursor was pushed toward the obstacle (it
     stayed), **on the portal** (3,4), **on the store** (4,5) and **on
     AA's square** (4,3), on top each time. Cost (`drop.txt`, `$719A`
     around every tap): the moves fell 5 to 4 at the press that chose the
     thing and were **back at 5 after the drop spot was pressed**
     (`dh-own`, `dh-e`): no net move. NEW ARMOR, the mail (`dk2-armor`):
     the moves went **5 to 0 at the choice**, the worn byte (record +21)
     40 and the next mover at once (`$71FA` 16): readying armor ends the
     turn, as inventory 4.3 has it; readying a weapon costs one move at
     the choice (10.33). Engine (inventory 5.1, 5.4, 5.9): a drop on the
     own square is allowed and free (agrees); portals and stores refuse
     (they did not here; walls and obstacles are out of the cursor's
     reach).
   - **The gold pile's own parameter** (inventory 2.5; `gp2`: a pile of
     ours on 5,4 with parameter 35 and size record 10): T took it, gold 0
     to **10**; `gp` (by a slip of ours the size record's bytes were 81
     and 10) gave **2,641** (81 + 10 x 256), its pile's parameter 35 too.
     So the amount is the size record's alone; parameter 35 changed
     nothing seen. The taken pile and its size record were both left in
     the list on square 15,15.
   - **Item 7, winning** (`win`: on 6,4, top first, our treasure 60, a
     space running Activate All, music 13, long message 1, music 1). T
     stepped to 5,4, took the treasure ("T takes PUTCI"; its list entry
     moved to 15,15, though no possession bit of T's record was seen set),
     then stepped onto 6,4 (`win-4`, `win-5`): **the long message filled
     the screen**; after the button **play went on** (T walked back west,
     rounds 17 to 19). No win screen and no end: the engine's choice
     (winning 3.1) agrees for a stack of ours; a goal the program itself
     generated was not tried.
   - **The room random creature's square** (movement 7.2; `rc`: random
     creature RR, Speed 0, room chance code 15; `rc-sw`, 32 runs from the
     map start with the random state `$1CC9` set to 0, 8, .., 248 before
     the walk in): it appeared in **21 of 32** (75 % expected), always in
     slot 0 with the random mark, on **squares all over the inside** (19
     different among the 21: 1,2 1,4 1,7 3,3 3,5 3,8 4,1 4,5 5,3 5,5 6,5
     6,8 7,1 7,5 8,6 9,7 10,1 11,6 13,8; 1,4 and 5,5 twice), never T's;
     the room held no things. The engine places it too (creatures 3.2, `place_random`; an earlier note here said it placed none, which was out of date).
41. **Creatures and movement, the open cases left, the 2026-11-16 batch**
   (ours, no research pass: [creatures](rules/creatures.md) 2.4, 2.6,
   2.7, 6.5 and "Captures needed" 2, 3, 5 and 8,
   [movement](rules/movement.md) 5.5, 6.3 and "Captures needed" 3).
   `ACSVM-research/capture/runs/rr37/` (the harness binary in `bin/`,
   `env.sh`; `case.sh` and `build.sh` build the cases of `cases.txt` to
   T's first room turn under way on 4,4, `NAME-room.vsf`, `P=1` one at a
   time; `mem.sh` runs `run mem --set-random-state --draws` from one;
   `hp.txt` and `arrive.txt` are scripts; `f2.sh` the item 1 sweep). At
   most three VICE of ours. Harness: `#[allow(clippy::large_enum_variant)]`
   on the command enum (no change in behaviour). New
   `scripts/adjcheck.py` reads each turn start's moves (`$719A`) against
   the halving rule and lists falls of more than one inside a turn;
   `scripts/memtrace.py` prints a run's watched bytes as a timeline;
   `goalcheck.py` gains `gold:X,Y:RANK` candidates (taken from beside,
   seen in the slot's wealth byte) and `--gold-first`; `fall2check.py`
   (item 1) reads two marked slots' falls.
   - **Item 1, two random creatures left at once** (creatures 2.4,
     "Captures needed" 2). rr4's `fall/side.vsf` (10.14.3: residents of
     ours in slots 0 to 6, the random creature in slot 7, T in the side
     room) with slot 0's mark set too (`--set E1A9=158`, its +6 byte 30
     with the top bit), so two random creatures stand in room 0 with no
     player; `f2/sweep`, 256 random states, 60M each (five round ends),
     every draw. `fall2check.py`, each round end's draws: T's idle
     healing roll, the slot roll; if the slot is in use, **one percentile
     for each marked slot, in slot order (0, then 7)**, each under 20
     removing a live creature; then the side room's roll. A removed
     creature's slot keeps its mark and **still draws its percentile**
     at later round ends.
     Result (256 runs, 2 jobs, about 2.7 hours): **every run's draws as
     predicted**, 1,280 round ends (643 with the slot in use), slot 0
     removed in 115 runs and slot 7 in 97, at the round end predicted,
     **both in the same round end in 14** (14 predicted). The other
     readings: the marked slots high to low 155 runs wrong, one
     percentile for both 255, a percentile only for a slot still alive
     (the engine's) 112. (`fall2check.py` first read a burst of draws as
     ending at a gap of 150,000 instructions; a removal pauses the draws
     about 200,000, so the gap is 300,000; and it takes a round end only
     where the counter goes up by one, a banked read sometimes showing
     another value across the real change.)
     So each random creature takes its own roll and both can go in one
     round. Engine (2.4): a percentile for each random creature still
     there, a removed one drawing none; the removals agree, the draws
     after a removal do not (the engine's stream runs one roll short).
   - **Item 2, the adjacency slowdown at odd Speeds and mid-turn**
     (creatures 2.6, "Captures needed" 3). `hp` (T on 4,4; EE, an
     aggressive enemy of Speed 0, Wisdom 3, on 6,4): `hp.txt` pokes T's
     Speed (record +4, `$F5A3`, 6 before) to 6, 7, 5, 3 or 1, waits for
     T's next turn, steps east to 5,4 beside EE and idles (`hp-s6` ..
     `hp-s1`, one run each). Moves at that turn's start 6, 7, 5, 3, 1; at
     once after the step **5, 6, 4, 2, 0** (one move, no halving); at the
     next two turn starts, beside EE, **3, 4, 3, 2, 1** each time: halved
     rounding up. `mc` (MM, an aggressive enemy of Speed 7 on 8,4, walking
     up to T), `mc-m`, 8 states, 40M: MM's 13 turns that began beside T
     had **4** moves, its 13 others 7; T's 15 turns that began with MM
     beside it had 3, its 7 others 6; in 13 turns MM arrived beside T with
     0 to 6 moves left and went on spending them one a step, no fall of
     more than one (`adjcheck.py`: 0 jumps). `cr` (peaceful enemies of
     Speed 1, 3, 5, 7 on the four squares beside T, the same four far off;
     T of Speed 2), `cr-m`, 8 states, 40M: beside T at a turn start **1,
     2, 3, 4** (8, 8, 8, 9 turns), elsewhere 1, 3, 5, 7 (57 turns), every
     turn start as predicted; rounding down misses 33. So 2.6's halving
     rounds up at every Speed, for players and creatures, and is taken at
     the turn's start only: walking up to a foe, either way, leaves the
     rest of the bar alone. Engine (2.6): the same.
   - **Item 3, creatures and spaces** (movement 5.5, 8.2, creatures 5.1).
     `cs`: peaceful friends of Speed 3 (Constitution 30, Life Force 20) in
     the four corners, both open squares beside each holding a space of
     ours: FA a space "invoke spell" (Life Force -5, `08420885`), FB a
     plain space open to anyone (`08020000`), FC a custom space "when
     moved onto" (-5 from the placement, `89420800`, 133), FD a custom
     space open to owners of thing 50, which FD owns (`09820805`, 50).
     `cs-m`, 8 states, 40M: in 64 creature turns **no creature left its
     corner** and no Life Force fell; each idled its bar out, healing (up
     1 to 4). So no space is open to a creature, the plain one and the
     one whose condition it meets included. Engine (5.1): the same.
     Seen once besides (item 4's `ad-ob-m` run 1): FF, a slinker friend
     standing on 1,4 beside the obstacle that bumps for -5 on 2,4, the
     player (its goal) standing on that obstacle's square, picked the
     square east (its rolls 2, 2, 0: 20 against 24 and 24, the square
     counted as one it had stood on), **its Life Force fell 16 to 11**
     and it stayed on 1,4: the obstacle's bump spell went off for a
     creature that tried to step onto its goal on an obstacle. The other
     five runs never had FF beside 2,4 with a move left. Engine: an
     individual's square is closed to a slinker, so it never bumps.
   - **Item 4, arriving by a room door** (movement 6.3, "Captures needed"
     3). `ad-sp`, `ad-ob`, `ad-cs`: on the side room's 2,4, where the
     portal on 7,4 leads, a space "invoke spell" (Life Force -5), an
     obstacle that bumps for -5, or a custom space open only to owners of
     the blade giving +5 (T not one); T of Speed 12, FF a peaceful friend
     of Speed 6 (Life Force 15) on 5,5. `arrive.txt` holds the stick east
     until T's column reads 2 (`ad-*-a`, 220,000 instructions): T **stood
     on 2,4, its Life Force unchanged** (39 and 40) each time; then
     `ad-*-m`, 2 states each, 40M: FF followed through the portal on its
     first turn in 6 of 6 runs, **landed on 2,4 with its Life Force
     unchanged**, and stepped off later; neither was ever set off by the
     square they arrived on. So arriving by a room door triggers nothing,
     for players and creatures, as by the map door (10.40 item 5).
     Engine (6.3): the same. Seen too: FF's next turn began with **3**
     moves in 5 runs and **1** in the sixth, which is its Speed 6 less the
     steps it took before the portal (3, or 5): a creature through a door
     carries its moves left into its next turn, as a player does
     (creatures 2.2.4); T's first turn after the door began with 6.
   - **Item 5, Speed 0 fighters beside an exit or gold** (creatures 2.7,
     "Captures needed" 8). `w5`: aggressive neutrals of Speed 0 (Wisdom
     3) in the corners, our obstacles on one square beside each so the
     other is the only open one: NA (owning and holding our blade, power
     10) and NC (no weapon) beside a portal (`103` on 1,2 and 1,7, to
     the side room), NB (the blade) and ND (none) beside 50 gold (13,2,
     13,7); the player 4,4 their goal. `w5-m2`, 16 states, 40M,
     `--answer-text FINDS,TAKES` (`w5-m` stalled at the first gold
     message: a Speed 0 creature's turn leaves T's name on the strip):
     **every one went through the portal or took the gold on its first
     turn**, 64 of 64 (the take from beside, its square unchanged). So
     owning a melee weapon changes nothing: the step onto an exit or a
     thing it takes stands. Engine (2.7): the same.
   - **Item 6, gold on the first step** (creatures 6.5's [unknown]). NE in
     `w5`, an aggressive neutral of Speed 0 on 8,4, 50 gold on 7,4 between
     it and T (the goal; gold rank 96, cost 3 by 6.5): `goalcheck.py
     --slot 4 --speed 0 --cand player:4,4:1 --cand gold:7,4:96` with the
     gold square at 3: **16 of 16 runs** (32 turns, 14 takes); at 4 on the
     first step 2 of 16. `g1`: TH, a peaceful thief of Speed 0 on 8,4 with
     the gold on 7,4 its goal (rank 32, cost 1 as the goal's square),
     `g1-m`, 16 states, 30M: cost 1 **16 of 16** (taken on its first turn
     every time), 4 on the first step 0 of 16. So the first step's 4 is
     the exit's alone: gold costs its rank / 32, or 1 as the goal, on
     every step. Engine (6.5): the same. Not run: the step choice's
     give-up case.
42. **Magic, items, stores and characters, the open cases left, the
   2026-11-16 batch** (ours, no research pass: [magic](rules/magic.md)
   2.4, 3.2, 3.13, 3.14, 7.2 and "Captures needed" 4 and 6,
   [inventory](rules/inventory.md) 1.3, 3.1, [stores](rules/stores.md)
   "Captures needed" 1 to 3, [characters](rules/characters.md) 3 and
   "Captures needed" 1, 2 and 4, [winning](rules/winning.md) "Captures
   needed" 1 and 2). `ACSVM-research/capture/runs/rr38/` (the harness
   binary in `bin/`, `env.sh` with `sc`; `case.sh` and `build.sh` build
   the cases of `cases.txt` and `cases2.txt` to T's first room turn under
   way on 4,4, `room.vsf`; `runs.sh` plays the run lists `runs.txt`,
   `runs3.txt`, `runs5.txt` from them, `gen.py` writing menu scripts and
   `py/steps.py` push scripts (a `.` presses the button); `py/last.py`
   reads a log block by block (T's record, the creature slots, the room's
   list); `gw.sh` the gold runs, `st-buy2.txt` and `st-buy3.txt` the
   store, `ru-add.txt`, `ru-pick.txt` and `ru-runs.sh` the roster,
   `rv-chain.sh`, `rv-a.txt` and `rv-b.txt` the reversal). Started by an
   earlier agent and resumed after a reboot: its finished builds were
   checked and the three a failed fork had cut short were built again.
   One harness change: `scenario --item-when use+pickup+drop`
   (`scenario::item_when`) sets when our magic item acts. At most three
   VICE of ours. Every case is one deterministic run read from RAM and
   the screenshots, except the summons with the random state set.
   Harness notes: peeks taken while a message waits for the button, or
   during a disk access, can read the ROM over `$E000` on (garbage, left
   out); the word at `$382A` moves while the room's list stays where it
   was at the visit's start (in `ar` and `sn`, with the side room, the
   list starts at `$E58B`, not `$E585`).
   - **Item 1, the cast's target and the summons.**
     - *Reach* (`tc`: Add to Room 60 the only spell; `tc-e9`, `tc-e12`,
       `tc-s4`, `tc-s6`, `tc-n4`, `tc-n6`, `tc-w4`, `tc-w6`, the cursor
       pushed 9 or 12 east, 4 or 6 each other way from 4,4 in the 15 x 10
       room): the thing landed on **13,4, 13,4, 4,8, 4,8, 4,1, 4,1, 1,4,
       1,4**. The cursor goes anywhere inside the room, 9 squares away
       included, and stops at the inside edge: it never reaches a wall
       square. Engine (magic 2.4): any square of the caster's room, a wall
       square refused; the same in effect, the refusal never being
       reached from the C64's cursor.
     - *Use Object* (`ta`: T owning a magic item doing Add to Room 61):
       USE OBJECT, the item, then **"move cursor to target"** as for a
       cast: no tap put 61 on T's 4,4 (`ta-use`), two taps east on 6,4
       (`ta-use2`). Engine: a use picks its target like a cast; agrees.
     - *Activate All cast* (`ta`, `tb`): choosing the spell and pressing
       ran it **with no target prompt** (`ta-sel`, `ta-actc`: the screen
       back to "move with the joystick" at once, the taps after it moving
       T). Cast by T standing on a stack it had arrived on and not set off
       (a space giving 61, then a space changing Power by +5; `tb-c`):
       **61 given and Power 13 to 17**, the turn over. So Activate All
       acts on the caster's own square and runs the stack there; on an
       empty square nothing happened (`ta-sel`). Engine (magic 2.5, 3.14):
       room spells ignore the target, and away from a stack Activate All
       runs the stack where it takes effect; agrees. (The earlier 10.40
       wording "every cast asked for a target, room spells included"
       holds for Kill All, Summons and Add to Room, not Activate All.)
       Harness note: in `ta` the SELECT SPELL list held NONE and
       TESTSPELL2 on its first row, TESTSPELL1 on the second; a tap east
       went to TESTSPELL2 (thing 79) and a second tap east stayed there,
       so `ta`'s reach runs (`ta-e9` .. `ta-w6`, `ta-act66`) all cast
       Activate All and are replaced by `tc`.
     - *A summons with no target press* (`sn`: master creature 5 ours,
       ZED; on 5,4 a space summoning it, on 3,4 an obstacle summoning it
       when bumped, on 4,3 a portal to the side room summoning it, our
       magic item summoning it when picked up or dropped on 4,5 and owned
       by T). Stepping onto the space (`sn-sp`): ZED on **5,4**, the
       space, T's square too. Bumping the obstacle from 4,4 (`sn-ob`):
       ZED on **3,4, the obstacle's own square**, T staying on 4,4.
       Through the portal (`sn-po`): T in the side room on 2,4, ZED on
       **4,3 of the first room, the portal's square**, not with T.
       Picking up the item from 4,5 (`sn-pk2`, T's ownership of it
       cleared, the take answered): ZED on **4,5, where the item lay**,
       T on 4,4. Dropping the item on T's own square (`sn-dro`): ZED on
       **4,4**; on 4,5 (`sn-drs`): **4,5**, the drop square. Each copy
       went to the lowest free slot, made at once. Engine (magic 3.2):
       the space (agrees), the bumper's own square (**disagrees**: the
       obstacle's square), the door's destination (**disagrees**: the
       portal's own square, in the room left), the picker's square
       (**disagrees**: the item's square), the drop square (agrees). (In
       `sn-dro` ZED then walked through the portal and a second ZED
       appeared in the first room on 5,2 two rounds later: a creature
       passing the portal sets it off too.)
     - *A summons of cost 20 with the region full* (`sf`: 16 slots in
       use, T Power 31 (the profile's cap on our 40), spell cost 20;
       `sg` the same with 15 in use; the random state `$1CC9` set to 0,
       85 and 170 before the menu). `sf-r0`, `sf-r85`, `sf-r170`: **"a few
       sparks fizzle but TESTSPELL1 fails"**, no prompt, no slot changed,
       **Power 31 to 11**: the cost is spent. The random state moved as in
       the open region (0 to 191 in both `sf-r0` and `sg-r0`), so the
       roll is drawn first. `sg-r85` and `sg-r170` succeeded (disk
       access, then the creature in slot 15 on T's 4,4, the cursor taps
       having come during the disk access); `sg-r0` failed its roll
       ("fizzle", Power 31 to 11) where `sf-r0` failed too, while
       `sf-r85` and `sf-r170`, whose rolls succeeded in `sg`, fizzled.
       So a full region turns a successful roll into a fizzle after the
       cost is paid. Engine (magic 3.2): slots checked first, nothing
       spent, no roll; **disagrees**.
   - **Item 2, the drop order and gold's weight.**
     - *Magic items and gold among a death's drops* (`dd`: Kill All but
       owners of 62 cast by T, who owns blade 59, treasures 13 and 14,
       our magic item 70, the spell 80 and 50 gold; AA on 6,4 owns the
       blade, treasures 50 and 51, item 70, the spell 80 (set at `$E1C6`,
       `$E1C7`) and 50 gold). Cast at AA (`dd-aa`): AA's square 6,4 held,
       top first, **70, 59, 51, 50, the gold** (thing 81, its size record
       50); cast at T's own square (`dd-t`): "T dies", 4,4 held **70, 59,
       14, 13, the gold**. The magic item goes down in thing-number
       order with the rest, the spell is lost, the gold lies under all.
       Engine (magic 7.2): agrees.
     - *Gold's weight per piece* (`gw1`, `gw4`, `gw0`: T Speed 15,
       Strength 10, Constitution = Life Force 20, capacity 1,000, nothing
       else carried; Gold's weight 1 (the template's), 4 or 0 by
       `--put 81=..`; `run mem` with T's gold `$F5B1` set at the start,
       `$719A` watched for 60 million instructions). Moves at each refill:
       weight 1 with 300, 600, 800, 1,000 gold: **15, 12, 6** and no
       refill (Speed 0); weight 4 with 100, 150, 200: **15, 12, 6**;
       weight 0 with 900 and 3,000: **15**. 18 refills in all, each the
       slowdown of inventory 3.3 for a load of gold x Gold's weight.
       Engine (inventory 3.1): agrees.
   - **Item 3, stores.**
     - *Value 0* (`st`: the room store on 4,5, stock poked to thing 1
       (value 5), 12 (the template's HAND, a melee weapon of value 0 and
       weight 0 nobody may use), 22 (a template treasure of value 0) and
       our item 70; `st-buy2` walked from 7,5). Bumping 6,7 (HAND): **"out
       of stock"**. Bumping 3,7: **"purchase KDNP'D PRINCESS for 0"**, the
       button bought it: stock to 0, T owning it, gold 500 kept. So a
       value of 0 does not keep a thing off the shelves; HAND's being out
       comes from what HAND is (neither thing was drawn on its place:
       their pictures are blank in our scenario). Engine (stores 2.7):
       an area store leaves out value-0 things; **disagrees** for a
       treasure of value 0 (rooms use the stock as stored).
     - *A magic item set to act when picked up, bought* (`st-buy3`, item
       70 showing long message 2 when picked up, value 1): offered,
       bought (gold 500 to 499, stock to 0, T owning it), **no message
       shown**, the store screen back. Engine (stores 4.4): does not act;
       agrees.
     - *A store in a room of another size* (`ss`: the store on the side
       room's 5,4, 10 x 8; `ss-in` walked in through the portal): the
       store screen as in the 15 x 10 room, T on **7,5**, thing 1 drawn on
       **6,6**, T bumping it with its last move. Engine (stores 3.3): one
       fixed layout; agrees.
   - **Item 4, Add to Room from a wall line, a door and a pick-up**
     (`ar`: Add to Room 60 run by an obstacle bumped on the wall line
     0,4, a portal on 4,5 to the side room, and our item on 5,4 when
     picked up). `ar-wall`: T walked to 1,4 and bumped 0,4: 60 on **1,4,
     T's own square**. `ar-po`: T went through to the side room's 2,4; 60
     on **4,5, the portal's square** in the room left. `ar-pk2` (T's
     ownership cleared, the take answered): 60 on **5,4, where the item
     lay**. Engine (magic 3.13): a wall square gives the nearest inside
     square with no thing and no individual (**disagrees**: the
     bumper's square, T on it), a door its destination (**disagrees**:
     its own square), a pick-up the square where it takes effect (agrees
     if read as the item's square). *Activate All away from a stack*: in
     item 1 (`tb`, `ta-sel`).
   - **Item 5, characters.**
     - *Choosing U with an empty first entry* (`ru`: the own disk's roster
       with entry 1's name cleared, its record T's (Speed 9, Constitution
       33), entry 2 U (Speed 4, Constitution 20); `ruc` the same with T
       named). The add-player screen put U in the second column and the
       cursor on CREATE NEW PLAYER (`$7161` 1), as 10.30 saw; a tap down
       (3) and east (4) put it on U, and the button **joined U's record**
       (name U, Speed 4, Constitution 20; `ru-se`). The cursor on the empty
       first place (`ru-s`, 3) went to the picture choice and **a new
       character was rolled** (Speed 7, Constitution 10, no name). So on
       the adventure's own list each place keeps its entry and an empty
       place behaves as CREATE NEW PLAYER. Not retried through the
       "another adventure" disk, where 10.30's `d2b-ownu` took another
       record (the disk swap's timing, 2.4). Engine (characters 3): no
       rosters; nothing to compare.
     - *A reversal waiting across a Destroy* (`rv`: T and U on the map,
       T casting a temporary Life Force +10 at itself, cost 0, round 21:
       Life Force 20 to 30, the first step back at round 24 (29), the
       queue's first entry (`$F65F`) then due at round 26; T asked RETIRE
       and was destroyed at the round's end, rr26's F1, button, F1;
       `rv-a`, `rv-b`). Over rounds 28 to 79, U playing on, T's slot kept
       first byte 0 and **Life Force 29**, and the queue's first 8 bytes
       stayed **26 0 239 125 1 255 255 9** throughout: the reversal never
       came due again, though round 26 passed (nine steps were left).
       Engine (characters 7.5, magic 5.2): reversals dropped on Destroy;
       agrees in effect (the original leaves the entry in place but never
       takes it; whether it blocks reversals behind it was not tried).
     - *A creature's ratings with marks*: not reached. The game shows no
       creature's ratings, and the harness does not drive the editor.
   - **Item 6, winning.**
     - *The possession bit at a take* (`pk`: our treasures 60 on 5,4
       placed with parameter 0, 61 on 4,5 with 1, 62 on 3,4 with 2;
       `pk-e2`, `pk-s2`, `pk-w2`, each take answered with the button).
       60: "T takes", its entry moved to 15,15, **no bit set**; 61 and 62:
       **the bit set** (one bit, things 33..80). So a placed thing's
       parameter is what a take gives: 0 gives nothing. 10.40's `win`
       treasure was placed with parameter 0, which explains the bit not
       seen there (as did our item 70 placed with 0 in `sn` and `ar`).
       Engine: not checked here; for the spec-writer (what a placed
       carriable's count is).
     - *A goal the program generated, and where the setting is stored*:
       not reached. It needs the editor's "Let ACS Write the Adventure",
       which the harness does not drive (only the template's making goes
       through the editor, by hand).

   No new addresses: the possession bytes (`$F5B6` on, a slot's +0x17
   on), the queue's first entry (`$F65F`) and the rest are section 9's.
   `$F65F`'s other seven bytes were only read, not interpreted.
43. **Inventory, magic, stores and characters: the [unknown]s, the
   2026-11-16 sweep** (ours, no research pass: every [unknown] left in
   [inventory](rules/inventory.md), [magic](rules/magic.md),
   [stores](rules/stores.md) and [characters](rules/characters.md)).
   `ACSVM-research/capture/runs/rr39/` (the harness binary in `bin/`,
   `env.sh`; `case.sh` and `build.sh` build the cases of `cases.txt` to
   T's first room turn under way on 4,4, `room.vsf`; `runs.sh` plays
   `runs.txt` and `runs2.txt` (a `BASE@start` line runs from the map
   start), `gen.py` menu scripts (a sixth field walks first),
   `py/steps.py` pushes, `py/probe.py` one push and the button with
   `$719A` read close around both, `mp2.txt` the map drop; `mem.sh` the
   `run mem` sweeps, `ccruns.sh` and `ccruns2.sh` the casting creature;
   `py/last.py` reads a log). At most three VICE of ours. Harness: the kit
   keys take `anyone=0` (`--blade anyone=0`: owner only, the extended
   record's bit 6 clear; `KitSpec::anyone`); `scripts/capcheck.py` takes
   `--who N --base ADDR` to read a creature slot (its turn-start reading
   misses a creature's moves, which are set just before `$71FA` changes;
   item 9 reads them directly). A retired player joining has its Life
   Force set to its Constitution and its Power to its Wisdom (the cases
   asking `lf=30` or `pow=20` started at 40 and 13), as 10.42's `sf` saw
   for Power.

   **The [unknown]s, sorted.** (a) reachable by a black-box capture as the
   harness stands; (b) only with the editor, another platform or a
   research pass; (c) not worth a run. The engine's choice is in
   brackets; the item below gives the run.
   - (a), run here: inventory 2.2 the carry limit's cost (refused, the
     bar unchanged; item 1); 2.4 a picked-up weapon replacing the one
     readied (yes, either kind) and armor not readied (item 2); 2.6 a
     creature's take of a thing placed with 0 (taken, nothing owned;
     item 8); 3.4 a creature's turn start, the die against the bar (the
     die first; item 9); 4.3 readying by a pick-up costs only the step
     (item 2); 4.4 the last copy dropped leaves nothing readied (item 2);
     5.3 a drop spell on the world map (goes off; item 5); 5.4 a drop
     toward a custom obstacle or a portal on the wall line (refused;
     item 6); 5.7 another thing dropped on a "when dropped here" space
     (lands on top; item 6); 5.9 a drop's cost on the world map (free;
     item 5); magic 3 "disappears after first use" for a failed cast
     (only a working cast removes it; item 4); 3.3 how far Increase Magic
     Defense goes (one level; item 3); 3.9 a given weapon not readied
     (item 3); 3.13 Add to Room from a portal on the wall line (the
     bumper's square; item 6); 5.1 a creature's drift, die and moves
     order (= inventory 3.4, item 9); 6.3 a smart creature casting at its
     goal, its turn ending (item 7); 6.4 a summons as hurtful (item 7);
     6.5 "half power" for magic items (Power at least half the Wisdom,
     rounded down; the turn ends; item 7); 7.2 drop spells at a death
     (not set off; item 10); stores 2.7 and characters 7.4, which of
     HAND's marks loses a creature's weapon at its death (weight 0 and
     owner only, both; item 10; the store's shelf side not run).
   - (a), not run: magic 2.5 whether Rid Room and a banish ask for a
     target (UI only: they ignore it either way); 3.14 Activate All from
     a used or picked-up item (the user's and the item's square; a
     magic item doing Activate All over a stack under the user, with a
     plain-treasure control); 5.4 reversals going by the visited
     region's rounds (needs a temporary effect left running across a
     region change); stores 5.4 a "disappears when dropped" treasure
     sold (sells, gone, never stocked: the sell flow of 10.17);
     characters 7.5 whether a destroyed player's left entry holds back
     reversals queued behind it (the `rv` chain of 10.42 with U's own
     temporary effect behind T's); inventory 5.8 how a gold drop asks
     for its amount (front end only); the store side of HAND's marks (a
     stocked blade of each kind, its shelf place found first).
   - (b): magic 6.1 when the 0..9 smartness roll of a Wisdom 6..14
     creature is drawn against its other first-decision draws (a
     random-rolls research pass: the draws' order inside a creature's
     turn); inventory 2.5 what a gold pile's own placement parameter
     means (nothing seen in play, 10.40; a research pass on the take);
     characters 3.2 the record a roster place takes through "another
     adventure" (the disk swap's timing, 10.30; the engine has no
     rosters, so a research pass on the swap or nothing).
   - (c): magic 3.12 Rid Room on the world map (nothing lies on the map
     to rid; no effect on play); creatures 2.5's and inventory 3.4's
     "the creature's slowdown itself", settled already by 10.32
     (`burden2-sw`: loads of 150 and 101 of 200 gave Speed 6 creatures 3
     and 5 moves) and seen again here (item 9).

   - **Item 1, the carry limit** (inventory 2.2; `cl`: our treasure 60
     on 5,4, thing 1 on 4,5, each placed with 1). `pc-e`: T owning 60
     (poked, `$F5C1` 16), a push east: **"T CANNOT CARRY MORE PUTCI"**,
     the button, T on 4,4, the moves **5 at the push, 5 just after the
     button, 5 a million instructions on** and 4 only at the drain's
     next fall; `pc-s`: T owning three of thing 1 (`$F5B6` 192), a push
     south: the same. The control `pc-e0` (owning nothing): "T TAKES
     PUTCI" and **4 just after the button**. So a move refused at the
     limit costs nothing. Engine (2.2): `Blocked`, the bar unchanged;
     agrees.
   - **Item 2, readying by pick-up and by drop** (inventory 2.4, 4.3,
     4.4; `rd`: T owning and holding the blade, our bow on 5,4, our mail
     on 4,5, treasure 60 on 3,4; `rdb`: T holding the bow, the blade on
     5,4). `rd-e`: the bow taken, **held 41** (the blade replaced);
     `rdb-e`: **held 59** (the bow replaced); `rd-s`: the mail taken,
     **worn 0**, held still 59. `pr-e` against `pr-w` (`py/probe.py`):
     the bow's take and the treasure's both left **4 moves just after the
     button**, from 5: the readying costs nothing over the step. `rd-drop`
     (DROP OBJECT, the blade, the own square): the blade on 4,4, **held
     0**. Engine (2.4, 4.3, 4.4): agrees on all four.
   - **Item 3, Increase Magic Defense and Give** (magic 3.3, 3.9; spells
     of cost 0 cast at T). `md` (Increase to 3): `md-c` from 0 gave **1**,
     `md-c1` (T poked to 1) gave **2**: one level a cast. Engine: one
     level, never past the parameter; agrees. `gv` (Give the blade):
     `gv-c`, T empty-handed: the blade owned, **held 0**; `gv-cb`, T
     holding the bow: the blade owned, **held still 41**. Engine: a given
     weapon is not readied; agrees.
   - **Item 4, a spell that disappears after use** (magic 3; `dsp`: our
     spell 80 Life Force +1 at cost 60 with the "disappears" bit, `--put
     80=163C0801`, T Power 13, so a 19% cast; `dsp-r0` .. `dsp-r224`, the
     random state set to 0, 32, .., 224 before the menu). One worked
     (`dsp-r64`: Life Force 40 to 41, **spell 80 gone**); seven failed,
     Power spent, **spell 80 kept**. Engine: a magic spell goes after a
     successful cast only; agrees.
   - **Item 5, a drop on the world map** (inventory 5.3, 5.9; `mp`: T on
     the map start owning our magic item, acting when dropped, Life Force
     +5). `mp-d`, `mp-d2` (DROP OBJECT, the item): **"THE THING DROPPED IS
     LOST FOREVER!"**, the item gone, **Life Force 40 to 45**: the drop
     spell goes off (engine agrees). The moves (`mp2.txt`): 4 through
     the menu and the list (the drain paused), **3 once the thing was
     chosen**, 3 after the message's button and on until the drain's
     next fall (2 about 1.5 million instructions later). In a room the
     move taken at the choice comes back when the drop spot is pressed
     (10.40 item 6); on the map there is no spot to press and it does not
     come back: **a drop on the world map costs one move**. Engine (5.9):
     free everywhere; **disagrees** for the map (2 runs).
   - **Item 6, drops and Add to Room at the walls** (inventory 5.4, 5.7,
     magic 3.13; `wp`: T owning the blade, a portal of ours on the wall
     square 0,4 doing Add to Room 60 to the side room, our custom
     obstacle on 4,5; `dh2`: T owning the blade and mail, on 5,4 a custom
     space "when dropped here" for the blade giving 61). `wp-po`: T walked
     to 1,4 and west through the wall portal to the side room's 2,4; 60
     on **1,4, T's square before the door**. Engine (3.13): the bumper's
     own square; agrees. `wp-dco` (the drop cursor pushed toward the
     custom obstacle) and `wp-dpo` (T on 1,4, the cursor pushed toward
     the wall portal): the cursor did not move and the blade landed **on
     T's own square** each time. Engine (5.4): those squares refuse; the
     same in effect, the C64's cursor never reaching them. `dh2-e` (the
     mail dropped on the space): the mail **on top of the space, 61 not
     given**. Engine (5.7): agrees.
   - **Item 7, creatures casting** (magic 6.3 to 6.5; `cc`: EE in slot 0,
     an aggressive enemy, Wisdom 20, Power 30 (29 at the snapshot),
     Speed 3, six squares from T; spells of cost 0: 80 Life Force -5, 79
     a summons of our ZED, 78 Life Force +5; our item 70, used, Life
     Force -3; what EE owns and its Power set by `--set` (`$E1C7`,
     `$E1C6`, `$E1A8`), its class by `$E1A7`; `run mem`, 2 states, 40M,
     three EE turns each). Each turn starts with drift (Power one step
     toward Wisdom), before the choice.
     - *The spell* (`cc-s5`): T's Life Force **-5 on each of EE's turns**
       from six squares off, EE never moving, its moves 3 to 0 at the
       cast. `cc-p19` (Power 19, drifted to 20): cast too. `cc-p10`,
       `cc-p15` (drifted to 11, 16): **no cast**, EE walked up. Engine
       (6.3): a smart creature with Power at least its Wisdom casts at
       its goal anywhere in the room, and its turn ends; agrees.
     - *Fitting* (`cc-fr`: EE a peaceful friend, class 51, owning only
       the +5): T's Life Force **40, 45, 50, 55** on EE's three turns:
       helpful at a friend (agrees). *The summons*: `cc-sum` (the enemy
       owning only 79): ZED made in slot 1 on EE's turn (placed on **4,5,
       beside T**), EE not moving; `cc-fsum` (the friend owning only 79):
       **ZED made as well**. Engine (6.4): a summons counts as hurtful, cast
       only at a foe: agrees for the enemy, **disagrees** for the friend,
       who summons at its friend goal.
     - *Magic items and "half power"* (EE owning only the item): Power 28
       after drift (`cc-i30`): T **-3**, EE's Power -2, and EE's moves went
       3 to 1 at the use and it **took one step with the last**; at the
       next turns the same. `cc-i10` (Power 11, 10, 11 at its three turns)
       and `cc-i9` (10, 11, 10): **used at 11, never at 10**, walking
       instead. So with Wisdom 20 an item is used at Power 11 and above,
       above half the Wisdom, not at exactly half, and the use costs two
       moves and does not end the turn. Engine (6.5): Power at least half
       the Wisdom rounded down (10 here), the turn ends; **disagrees** on
       both (4 runs at the boundary).
   - **Item 8, a creature's take of a thing placed with 0** (inventory
     2.6; `ct`: TH, a peaceful thief of Speed 0 on 8,4, our treasure 60
     on 7,4 placed with 0, 61 on 9,4 placed with 1; `ct-m`, 8 states,
     30M, watching TH's possessions `$E1C5`). Every run: **two takes
     ("TH TAKES"), both things gone from the floor, only 61's bit set**
     (6 runs took 60 first, 2 took 61 first). Engine (2.6): a take of a
     thing placed with 0 adds nothing, for creatures too; agrees, 8 of 8.
   - **Item 9, a creature's turn start** (inventory 3.4, magic 5.1,
     creatures 2.5; `cw`: CW, a peaceful friend of Speed 10, Strength 10,
     Constitution 20, Life Force 14, carrying a treasure of weight 600; by
     3.2 capacity 700 at Life Force 14 (Speed x 2 x 100 / 700: 2 moves),
     800 at 15 (5 moves)). `cw-m` (16 states, 40M) and `cw-m2` (96, 10M):
     **every first turn began with 2 moves** (112 of 112); CW dropped the
     treasure as its first action and stepped once (creatures 8.1), and
     its next turns began with 10. In **23 of the 96** (`cw-m2`; 1 of
     16 in `cw-m`) the top-of-turn die healed it 14 to 15, and each time
     the moves byte (`$719A`) was set to 2 **before** the Life Force rose
     (the moves at 8.46M, the heal at 8.48M, the turn byte 16 at 8.54M):
     the creature's bar comes from the Life Force before the die's heal.
     A player's comes after it (inventory 3.4, 10.39 item 4). Engine
     (inventory 3.4, creatures 2.5): the same order for creatures as for
     players (die, then the bar); **disagrees** for creatures (23
     healed turn starts, 5 moves predicted, 2 seen). The slowdown itself
     agrees (10.32 already).
   - **Item 10, a death's drops** (magic 7.2, characters 7.4, stores 2.7;
     `bl0`, `bln`, `blb`: AA, a Speed 0 friend on 6,4, owning and holding
     our blade, and our item 70 (`$E1C6` 4), which acts when dropped: Add
     to Room 61; T casts Kill All but owners of 62 at AA (`-aa`) or at
     itself (`bl0-t`, T owning item 70)). The blade weight 0, usable by
     anyone (`bl0-aa`, `--put 59=04000A00`): 6,4 held **70, 59**: kept.
     Weight 5, owner only (`bln-aa`, `--blade anyone=0`): **70 only**, the
     blade lost. Both (`blb-aa`): **70 only**. So "nobody may use" alone
     loses a creature's weapon at its death, weight 0 alone does not.
     Engine (characters 7.4, magic 7.2): lost only with both marks;
     **disagrees** for an owner-only weapon that weighs something. **No
     61 in any run** (`bl0-t`: T died, 4,4 held 70 only): drop spells do
     not go off at a death; agrees. (Whether the store's shelves leave
     out an owner-only weapon or a weightless one, stores 2.7, was not
     run: the blade's shelf place is not known.)

   No new addresses: T's held and worn things (`$F5B3`, `$F5B4`, record
   +20, +21), the possession bytes (`$F5B6` on, a slot's +0x17 on), the
   slot's class (`$E1A7`) and Power (`$E1A8`) are section 9's.
44. **Creatures, encounters, combat and movement: the [unknown]s, the
   2026-11-16 sweep** (ours, no research pass: every **[unknown]** left
   in [creatures](rules/creatures.md), [encounters](rules/encounters.md),
   [combat](rules/combat.md) and [movement](rules/movement.md), and the
   bump at the goal of 10.41 item 3). `ACSVM-research/capture/runs/rr40/`
   (the harness binary in `bin/`, `env.sh`; `cases.txt`, `case.sh` and
   `build.sh` build the cases to T's first room turn, `NAME-room.vsf`, as
   rr37's; `mem.sh` runs `run mem --set-random-state --draws`;
   `queue.sh`, `doors.sh`, `after.sh`, `more.sh`, `more2.sh` the runs;
   `arrive.txt`, `sp0.txt`, `sp0b.txt`, `pd.txt` scripts). At most three
   VICE of ours beside rr39's. Harness: none. New `scripts/turnlog.py`:
   a sweep's turns, individual by individual (moves at the start, Life
   Force and square at start and end), with `--bump WHO@X,Y` (a Life
   Force fall of 5 on the spot beside X,Y) and `--attacks WHO` (falls of
   the player's Life Force on WHO's turns). In `fs`, `fs2`, `mo`, `fsw`,
   `fsw2` and `fsb` the creature beside the entry square left T one
   square further south, on 4,5, after `enter-room.txt` (the stick still
   held); nothing below depends on it.

   **The [unknown]s, sorted**: (a) a black-box capture with the harness
   as it stands or a small option, (b) the editor, another platform or a
   research pass, (c) not worth a run.
   - creatures 2.4, the random mark (what clears it, does a death keep
     it; engine: kept through removal, banishment and death): (a), a
     random creature killed in its room, then the round ends' draws
     (`fall2check.py`). Not run.
   - creatures 2.5, a creature's bar (the load slowdown; before or after
     the top-of-turn die; engine: a player's rule, after the die): (a),
     item 3 below.
   - creatures 2.6 and 5.4, a creature's carried moves (beside a foe;
     through a door with no move left; engine: not halved; the full bar
     if none was left): (a), item 4. Whether the top of that turn or the
     load changes the carried count: (a), not run.
   - creatures 4.2, fighter or slinker worked out afresh each turn
     (engine: at each turn's start): (a), item 2.
   - creatures 5.1, a goal individual on a square without an obstacle
     (engine: closed): (c), 7.2's captures already have a slinker step
     round the player it seeks; the obstacle case is item 1.
   - creatures 5.3, spell doors other than the three let creatures pass,
     the spell going off for them: (a), a portal of ours through
     `--put`. Not run.
   - creatures 6.5, one more case in which the step choice gives up:
     (b), a research pass: the case is not known, so no scenario can aim
     at it.
   - creatures 7.3, a gift's drop spell acting on whoever stands on the
     square: (a), 10.37 item 6's gift with `--item-when drop`. Not run.
   - creatures 8.1, over capacity it picks nothing up; magic items in the
     drop order; gold in the load: gold in item 3 (CD); the rest (a), not
     run.
   - encounters 3.4, room 0's list on the open map: (c), empty in the
     whole corpus. The gaps an over-long list leaves in a wall, walkable
     or not: (c), malformed data no editor writes.
   - encounters 3.5, door parameters above 4, a missing destination's
     screen (also 7.1), the door item's own picture: (c), malformed data
     or drawing only. An area door into a room of region 1 rolling that
     room's random creature (engine: it does): (a), `--map-bytes` on the
     area door table with `--room-chance 15`. Not run.
   - encounters 6.1, several players on the open map with an area in
     use, and the map's 10-round visit (engine: player order; 10
     rounds): (a), two players of ours, low in play. Not run.
   - encounters 6.4, a door item on an inside square beside a creature:
     (c), every corpus area door is the stored one on the wall; the
     engine closes such squares to creatures anyway.
   - encounters 7.6, retiring one of two players in an area: (a), 10.36's
     retire chain with two players, low in play. Not run.
   - combat 4.3: (c), the superseded [unknown], settled by 10.39 item 3.
   - combat 4.4, magic defense bringing a blow to 0, and its message:
     (c), 5.4 calls the attack off at levels 2 and 3 and level 1 never
     makes 0; the message is presentation.
   - combat 5.2, armor that disappears used up by a blow it stops
     entirely: (a), item 6.
   - combat 6.2, where readying falls on a drop turn with moves left:
     (a), a creature over half its capacity owning a weapon. Not run.
   - combat 7.4 and 8's "Not run", the Wisdom roll after arrows on
     aggressive thieves and enemies and on players of Wisdom below 6 or
     above 14: (a), a state sweep with 10.32's FIRE kit. Not run.
   - combat 8.1, a creature fires only at a goal that is its foe: (a),
     item 5.
   - combat 8.8, what else misses a player that came in after the firer:
     (a), two players through doors in one round, low in play. Not run.
   - movement 2.4, a part-drained step: (a), item 7.
   - movement 2.7, by how much load and adjacency slow: (c), the
     manual's [unknown], settled by 10.39 item 4 and 10.41 item 2.
   - movement 2.8, a player of Speed 0: (a), item 8.
   - movement 4.3, terrain that triggers a spell, before or after the
     move: (b), the editor: the scenario builder writes no such terrain
     (one use in the corpus).
   - movement 5.7, destroy needed thing with non-owners access (engine:
     owners only): (a), a portal of ours through `--put`. Not run.

   - **Item 1, the bump at the goal** (creatures 5.1; 10.41 item 3 saw
     one). rr37's `ad-ob-a/arrived.vsf` (T arrived by the side portal on
     2,4, where our obstacle bumps for Life Force -5; FF, a peaceful
     friend of Speed 6, Life Force 15, on 5,5 of the first room, T its
     goal), T's Speed set to 1 (`--set F5A3=1`) for short rounds;
     `bump-m`, 256 random states, 30M each (about 5 FF turns a run), 2
     jobs, 51 minutes. `turnlog.py --bump 16@2,4`: **9 bumps in 8 of 256
     runs** (one run had 2), each a fall of 5 with FF staying put beside
     2,4. Every one came on **FF's first turn after following T through
     the portal**, the turn it began on 2,4 sharing T's square: 231 runs
     followed, and 8 of those 231 turns bumped. In the **586 other
     turns** that had FF beside 2,4 with a move left, **no bump**. State
     1 is 10.41's run: the same bump at the same moment. So the bump
     comes only after the creature has stood on the goal's square that
     turn and stepped off (6.5's cost 4 for a square stood on this turn);
     a creature that has not stood there does not try the square. Engine
     (5.1): the square open at cost 4 if stood on this turn and **1
     otherwise**, bumping about 3 times a run in 224 of 256: **wrong**
     for the cost-1 case; the original bumps in 3.5% of the turns that
     begin on the square, never otherwise.
   - **Item 2, fighter or slinker each turn** (creatures 4.2). EE, an
     aggressive cautious enemy of Speed 3 (Constitution 20) on 5,4 owning
     and holding our blade (power 3); T of Speed 1 (`--set F5A3=1`);
     `--answer T`; 4 states, 30M each. Built unwounded (Life Force 20),
     `fsw-m`: it struck T in **4 of 4** runs (32 falls); the same
     snapshot with its Life Force set to 10 at the run's start, after the
     visit began (`fsw-set`, `--set E1A6=10`): **0 of 4**, walking off.
     Built wounded (10), `fsw2-m`: 0 of 4; set to 20 at the start
     (`fsw2-set`): **4 of 4** (31 falls). Brave and unwounded (`fsb-m`):
     4 of 4. So the choice follows the Life Force of the moment, not the
     one it had when placed or when the visit began. Engine (4.2): at
     each turn's start: confirmed (16 of 16 runs). The same cases with no
     weapon (`fs-m`, `fs-set`, `fs2-m`, `fs2-set`, 15 runs) never struck,
     as an unarmed fighter never does (combat 2).
   - **Item 3, a creature's bar and its load** (creatures 2.5, 8.1).
     `wt`: peaceful friends of Speed 10, Strength 10, Constitution 20:
     CA carrying 700 (Life Force 20, capacity 1,000), CB 500, CC 700 at
     Life Force 18 (capacity 900), CD 700 gold and nothing else (Gold's
     weight 1). `wt-m`, 8 states, 40M: the first turn's moves **CA 6,
     CB 10, CC 4, CD 6, in 8 of 8 runs each**: inventory 3.3 with the
     creature's own capacity, gold counted per piece; their next turns
     began with 10. `wt2-m` (4 states, 60M, the room's list `$E583` and
     CD's gold watched): CA, CC and CD each added an entry at the top of
     their first turn (CD's a gold pile of 6 bytes, its gold going to
     0), the overburdened drop of 8.1; CB's entry came later, when it
     reached T (a gift, 7.3). Engine (2.5, 8.1): the same, gold dropped
     first as one pile: confirmed. Not reached: before or after the die
     (no creature healed at that turn's top: CC stayed 18 in 12 of 12
     runs).
   - **Item 4, a creature's carried moves** (creatures 2.6, 5.4).
     `arrive.txt` as rr37's (T east through the side portal to 2,4), then
     `run mem` from there with T at Speed 1, 30M. `dz`: FF of Speed 2 on
     6,5, the portal its second step: it followed in 3 of 4 runs and
     **began its next turn with 2**, its full bar. `d1`: Speed 3, 4 of 4
     followed with 1 left and began with **1**. `df`: FF as rr37's
     (Speed 6 on 5,5), EE an aggressive enemy of Speed 0 (no weapon) on
     the side room's 2,5 beside the arrival square: 6 of 8 followed and
     began beside EE with **3** (5 runs) or 1 (1 run), its carried moves,
     **not halved**. `fh` (control): the same FF beside the same kind of
     enemy on its first turn, nothing carried, began with **3** of 6 (2
     of 2). Engine (2.6, 5.4): carried moves not halved, the full bar
     when none were left: confirmed.
   - **Item 5, firing at a goal that is not a foe** (combat 8.1). `fb`:
     FB, an aggressive brave friend holding our bow (power 5, range 9),
     on 10,4, T the only other individual; `nb` the same as a neutral.
     8 states each, 30M: FB **never fired** (no shot count at `$B799`,
     16 turns); NB fired on its first turn in **8 of 8** runs. Engine
     (8.1): a goal not its foe is not shot at: confirmed.
   - **Item 6, armor that disappears, a blow it stops** (combat 5.2).
     `mo`: T wearing our mail (power 20, once), EE an aggressive brave
     enemy with our blade of power 5; `mo2-m`, 4 states, 30M, `--answer
     T` (`mo-m` stopped at the message on EE's first blow). In **4 of
     4** runs the mail went (`$F5B4` to 0) at a blow that took no Life
     Force, the first blow to reach it; T's first loss came at a later
     blow (in the same attack in run 3). Engine (5.2): used up by a blow
     it stops entirely: confirmed.
   - **Item 7, a part-drained step** (movement 2.4). `pd` (T alone,
     Speed 6): `pd.txt` waits for the refill to 6 and the first drain to
     5, runs Y more, then holds east until the moves reach 0. The second
     drain came between 1.75M and 1.85M after the first. Y 100k, 900k,
     1,600k, 1,750k: moves 5 at the push and **5 squares walked**; Y
     1,850k: moves 4, 4 walked. So a step is walkable however much of it
     has drained; only the whole step's drain takes it. Engine (2.4):
     whole steps: confirmed (5 of 5).
   - **Item 8, a player of Speed 0** (movement 2.8). From rr37's
     `hp-room.vsf`, T's Speed set to 0 (`sp0.txt`): at its next turn
     the moves read 0 and **the menu opened by itself**; the stick did
     nothing (T stayed on 4,4) and the turn did not end in 20M. `sp0b`:
     MOVE chosen from that menu shows the message that it can't move,
     perhaps carrying too much; the turn still did not end. Engine (2.8):
     moves refused, only Wait ends the turn: confirmed (one run each); a
     front end may open the menu at once, as the original does.

   No new addresses: the turn byte, the moves, the slots' bytes, `$B799`,
   `$F5B4` (record +21), the possession bytes and the room list are
   section 9's.
45. **Things that vanish after use, needed things used up, a creature's
   choice of spell, a long message's lines: for the Rivers of Light
   playthrough, 2026-11-17** (ours, no research pass: [movement](rules/movement.md)
   5.5 and 5.9, [magic](rules/magic.md) 3.10, 3.14 and 6.3).
   `ACSVM-research/capture/runs/rr41/` (the harness binary in `bin/`,
   `env.sh` with `sc`, `st` and `en`; `cases.txt`, `case.sh` and
   `build.sh` build the cases as rr39's; `enter2.txt` (`en`) enters the
   room again from `start.vsf` and lets the stick go as the region
   becomes 1 (`$397C`), so a Speed 15 T stops on 4,4 with 15 moves
   (`room2.vsf`; `enter-room.txt`'s 9 million instructions of stick walk
   a fast player on, over 4,5); `py/steps.py` writes the push scripts
   `st-DIRS.txt` (`st CASE DIRS` runs one from `CASE/room2.vsf` into
   `CASE-DIRS/`), `py/look.py` reads T's record and the room's list from
   their logs; `mem.sh` and `csruns.sh` the spell sweep, `py/cs.py` its
   reading). At most three VICE of ours. Harness: `scenario
   --long-message N:TEXT` (`scenario::long_message`) writes long message
   N of ours. Things are `--put`s of ours (bytes as 10.40); every gift is
   one of our treasures 21 to 29, which a player holds up to three of, so
   a second gift shows. A thing that goes leaves the room's list for
   square 15,15, as a take does (10.40). Each case is one deterministic
   run.
   - **Item 1, bit 4 of the first property byte** (`v1`, `v1b`, `v2`,
     `v2b`, `v2d`, `v3`, `v4`; T Speed 15 on 4,4, each thing over a
     spell space giving the next treasure, so what lies below shows
     itself). *Spaces*: a spell space with the bit (`v1-eew`, 5,4) gave
     21 and **went at the step onto it**: the list read after that step
     had it on 15,15, T standing on 5,4; off and back on, the space below
     gave 22. A plain space open to anyone, no spell (`v1-nsn`, 4,3): the
     same, gone at the first step, the space below working at the second.
     *Custom spaces*: "when moved onto" (`v1-sns`, 4,5) and "open to
     owners of" the blade with T owning it (`v1b-wew`, 3,4): spell, then
     gone at the step onto, the space below working next time; with the
     bit 7 choice (`v1b-ewe`, 5,4) the placement's 24 given and gone
     alike. "Open to owners of" a thing T lacks (`v1-ww`, 3,4): **refused
     twice and kept**. *Obstacles*: the plain obstacle with the bit
     (`v2-nn`, 4,3) gave 25 and went at the first bump (movement 5.9
     confirmed); custom obstacles with the bit **went at the first bump
     whether or not a spell ran** (`v2-ee`, `v2-ss`, `v2b-nn`), and a
     custom obstacle without it stayed through two bumps (`v2-ww`,
     `v2b-ww`). The square then shows the thing below (screenshot
     `v2-ee/a00e`: the space's picture beside T), which acts at the next
     step (22, 24, 26 given). *A custom obstacle's spell*: bumped, it ran
     only with **bit 6 of the second byte set** (as an obstacle's "bump
     invokes"; `v2b`: `1B41..` gave, `1B01..` gave nothing, `v2`, `v2b-nn`),
     and its parameter came from the **placement when bit 7 of the first
     byte is set** (`v2d-e`, `9B41` placed with 21: 21), else from the
     master's fourth byte (`v2b-ss`, `v2d-n`: 23, 25 whatever the
     placement), the custom space's choice (movement 5.8). A Give with
     parameter 0 gave **thing 80** (`v2b-ee`, `v2b-ww`, `v2c`, `v3`).
     *Under Activate All* (`v3-ewe`: on 5,4, top first, an Activate All
     space, then with the bit a spell space, a "when moved onto" custom
     space, an "owners of" one, an obstacle and a custom obstacle, then a
     spell space without it; T owning the blade): one step gave 21 to 24,
     80 (the custom obstacle `1B01`, parameter 0) and 26, and **all five
     with the bit went**; the next step ran only the last (26 twice).
     Under Activate All a custom obstacle runs its spell even with bit 6
     clear (`v2c-e`, `v2c-s`: `0B01` and `0B41`, both gave 80, neither
     went, having no bit 4). *Re-added* (`v4`): a spell space with the bit
     over a space doing Add to Room of it (5,4; `v4-ewewew`): step 1 gave
     21, the space went; step 2 put it **back on top, placed with 1**;
     step 3 gave 21 again (21x2) and it went again. The same through an
     Activate All between them (4,5; `v4-snsnsn`: 22, back, 22x2, gone).
     So a re-added thing works again and goes again.
     Engine (movement 5.9): only a plain obstacle goes, when bumped;
     **disagrees** for spaces, custom spaces and custom obstacles with the
     bit (they go at the step or bump that sets them off, or under
     Activate All; a refused step keeps them). Engine (5.5): a custom
     obstacle's spell always runs on a bump, its parameter from the
     placement; **disagrees** for bit 6 clear and for bit 7 clear.
   - **Item 2, needed things under Activate All** (magic 3.14; `n1`: T
     owning the blade; on 5,4 an Activate All space over two custom
     spaces "open to owners of" the blade with "destroy needed thing",
     giving 21 and 22, then a spell space giving 23; on 4,5 one such
     custom space alone, giving 24). `n1-ew`: T stepped onto 5,4 and
     stood there: **21 and 23 given, the blade gone**, 22 not given: the
     first used up the blade and the second's condition then failed.
     `n1-s` (the custom space alone): 24 given, the blade gone. So a
     needed thing is used up under Activate All as on a direct step.
     Engine (3.14): never used up there; **disagrees** (one run each).
   - **Item 3, a creature's choice of spell** (magic 6.3; `cs`: EE in
     slot 0, an aggressive enemy, Wisdom 20, Power 40, Speed 3, six
     squares from T; our spells 9 and 79 Life Force -2 cost 0, 73 and 78
     the same, 10, 74, 77 and 80 Life Force -6 cost 3, 75 -6 cost 0, 76 -2
     cost 3; what EE owns set by `--set` (`$E1BC`, `$E1C7`); `run mem`, 8
     random states, 40M, three EE turns each; `py/cs.py` reads T's Life
     Force falls and EE's Power on each turn). Pairs: `cs-p1` 79 and 80,
     `cs-p2` 77 and 78, `cs-p3` 75 and 76, `cs-p4` 9 and 74, `cs-p5` 10
     and 73. **Every turn cast the lower-numbered spell** (24 of 24
     each, 120 in all, by its cost in Power and its harm): the cheaper
     and the dearer, the weaker and the stronger, across the counted
     things (1 to 32) and the others. Two turns (`cs-p2`, `cs-p5`, state
     6) spent 3 Power with no harm, a failed cast of the same spell.
     Engine (6.3): the lowest-numbered fitting spell; **confirmed**.
   - **Item 4, a long message's lines** (magic 3.10; `lm`: a space on
     5,4 showing our long message 3, its eight stored lines of 32
     characters, a word split across lines 4 and 5). `lm-e..`
     (screenshot `a00e`): **eight screen lines, one per stored line**,
     exactly as stored: the last word of line 1 and the first of line 2
     on separate lines, the split word split. Engine: the text joined
     with nothing between lines; **disagrees** (a front end should break
     after every 32 characters).

   No new addresses: `$397C` (the region visited), T's record and
   possession bytes, the slot's bytes and the room list are section 9's.
46. **REST, 2026-11-18** (ours, no research pass: [magic](rules/magic.md)
   5.6, combat.md 8.8.1's menu). `ACSVM-research/capture/runs/rr42/` (the
   harness binary in `bin/`, `env.sh` with `sc`; `case.sh` and `mem.sh`
   as rr41's; `rest1.txt` opens the menu, one press south (REST, first
   of the second row), the button, and looks at the next turns;
   `menu.txt` stops on REST and saves `on-rest.vsf`; `py_rate.py` and
   `py_rolls.py` read the `mem` runs, a draw's face as raw / 42, the
   0..5 roll of a Speed 6 turn). Cases built by `case.sh`, T Speed 6 and
   Constitution 40 at its first room turn on 4,4: `r1` alone (Life Force
   22 by then, healed from the 10 set), `r2` with EE an aggressive enemy
   on 10,4, `r3` with EE beside on 5,4, `r4` at Life Force 40; `r1-map`
   from `r1/start.vsf`, T's turn on the world map. `mem` runs: the
   random state 0..15 (`m-rest`, `m-idle`, `m-rest-ee`) or 0..7
   (`m-full`, `m-map`), `--draws`, `--until-refill`, the button pressed
   25k from `on-rest.vsf`.
   - **REST plays out the turn's moves left as idle moves, at once, and
     the turn ends.** No message, no button. In every run the moves byte
     (`$719A`) fell 5, 4, .. 0 about 20,000 instructions apart (a
     drained idle move takes about 1.85M), the round counter moved on and
     the next turn began with its bar refilled (`r1-a`: round 14 to 15,
     the moves 6). **One draw each move**, of 0..5, the turn's full moves
     less 1 (a 6 drawn again as usual), and **Life Force +1 exactly at
     the draws of 0**: 80 moves and 15 gains over `m-rest`'s 16 runs,
     every gain after a 0 and every 0 below Constitution a gain. That is
     magic 5.3.1's idle-move roll, the same as `m-idle` (the same turn
     left to drain: 80 moves, 21 gains, one draw each); the draw after
     the last move and the next turn's die come as in a drained turn.
   - **At full Life Force** (`m-full`, 40 of 40): the same draws, no
     gain. **Beside a foe** (`r3-a`, the turn halved, 1 move left): no
     refusal, unlike FIRE (combat 8.8.2); the move went, Life Force +1,
     EE's turn came (`$71FA` 16). **A foe in the room** (`m-rest-ee`):
     as `m-rest`. **On the world map** (`m-map`): the menu has REST and
     it does the same, one draw each of the 4 moves left.
   - **So REST is the turn waited out, without the wait.** The manual's
     "6 times as long" without resting is real time: a drained turn heals
     the same, but the bar drains a move about every 7 seconds (movement
     2.4), and a walked move takes no roll. Engine: `Wait` already played
     the moves left as idle moves with their rolls (magic 5.3);
     **confirmed**, and `Wait` is REST by name now.

   No new addresses.
47. **Mimics, 2026-11-18** (ours, no research pass:
   [creatures](rules/creatures.md) 4.5). `ACSVM-research/capture/runs/rr43/`
   (the harness binary in `bin/`, `env.sh`, `case.sh` and `mem.sh` as
   rr42's; `py_rec.py` and `py_rec2.py` list a `mem` run's changes by
   record byte, dropping values undone within 60,000 instructions).
   Harness: `scenario --creature ..,mimic=1` sets bit 7 of the record's
   byte 0x09. T Speed 6, Constitution and Life Force 40, Strength 20,
   Dexterity 18, Wisdom and Power 17, Size 14, Dodge 50, Parry 40, Melee
   60, Missile 30; MM an aggressive enemy of Speed 3, Constitution and
   Life Force 12, the rest 5 or 10, two squares east of T on 4,4. `mem`
   runs watch slot 0 (`$E1A3`, 37 bytes), slot 1 and T's record
   (`$F59F`) with `--ram` (without it the bytes under the ROMs read
   passing values).
   - `m1` (mimic) and `m0` (control), 4 states each, 40M instructions:
     at MM's first turn, after the round byte (+12) was noted and before
     its first step, **slot 0 took T's bytes** +0 to +2, +5 to +10, +12,
     +15, +17 and +22 (the picture), and +4's low four bits (Speed 6) with
     its own top four kept (102), and +9 with its own bit 7 (188), in all
     4 runs; +3 (Life Force 12), +13 and +14 (the square) stayed. That
     turn it stepped 3 times, its old Speed; its third turn 6. Nothing in
     the control.
   - `m3`: AA, a friend, beside MM on 11,4, MM 6 squares from T: MM
     copied T, not AA.
   - `m2`: T with Armor skill 25, holding our blade and wearing our mail:
     MM took +8 (25), +20 and +21 (the blade and mail) and T's possession
     bits (+31, +34).
   - `x1e`, `x1f`: from after the copy, T's Strength set to 25 and
     Dexterity to 25: MM's next two turns copied nothing.
   - `m4`, `m5` (control), `m6`, `m7` (control): T with 300 gold, MM with
     7 (and in `m6`, `m7` one of our treasures). The mimic kept 7, then
     **dropped its 7 gold as a pile on its own square** (the room's list,
     `$E583`) about 540,000 instructions after the copy, then took 2 steps
     (3 without gold); its treasure's bit went with the copy and nothing
     else reached the room's list. The controls kept both.

   No new addresses.
48. **The title screen and the intro page, 2026-11-18** (ours, no
   research pass: [client](client.md) 3.3, format notes).
   `ACSVM-research/capture/runs/rr44/` (the harness binary in `bin/`,
   `env.sh`; `intro.txt` and `intro2.txt` are `start-retired.txt` and
   `start-scenario.txt` with a screenshot after every press,
   `intro3.txt` the first three; `graft.py OUT LO HI` lays Jadeworld's
   disk bytes LO..HI over our `i2` disk). Our scenario disks with an
   intro of ours (`scenario --long-message 255:TEXT`, which is the
   intro's slot): from the main menu's "put your adventure disk in",
   the **title screen** (the adventure's name, "by", the byline, the
   program's credit line, "press button to continue"), then the
   add-player screen: **no intro**, with a retired player (`i1`) or a
   new one (`i2`). A copy of Baber's *Jadeworld* (`j1`): the title
   screen, then an **intro page** (the adventure's name in a box at the
   top, the intro's eight lines in a frame, "press button to
   continue"), then the add-player screen; the same on that copy played
   once and saved (`j2`). Jadeworld with its intro text blanked (`j3`):
   the page, empty. So a flag shows it, not the text. Grafts of
   Jadeworld's bytes over `i2`: its whole adventure header (`g-hdr`,
   + 0x000 to + 0x500 of the block at `0x1C400`) shows our intro text
   under its name; the late messages and intro block (`g-tail`) does
   not; halving (`h0`..`h3`, `k0`..`k3`) leaves + 0x340 to + 0x380, and
   **header + 0x36D alone** (`b36d`, 0 to Jadeworld's 1) shows the page;
   the theme byte + 0x36E (`b36e`) and + 0x36F to + 0x380 (`b36f`) do
   not. Set to 2 and to 128 (`v2`, `v128`): shown. Every corpus
   adventure has 1, the blank disk EA's program makes 0.

   No new addresses (a disk byte, format notes).
