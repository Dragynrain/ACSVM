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
# a player of ours (section 8): Speed 15, everything else as move-basic's
acs-capture scenario move-basic C/runs/speed-15/disk.d64 --player spd=15
acs-capture script crates/acs-capture/scripts/start-retired.txt --disk C/runs/speed-15/disk.d64     --snapshot C/boot/play-nodisk.vsf --out C/runs/speed-15 --var OUT=C/runs/speed-15
acs-capture run move-basic --snapshot C/runs/speed-15/start.vsf --disk C/runs/speed-15/disk.d64     --out C/runs/speed-15 --runs 20 --speed 15
# refused moves and door cost: the same with --obstacle, then
acs-capture run refuse --snapshot C/runs/refuse/start.vsf --disk C/runs/refuse/disk.d64     --out C/runs/refuse/part0 --runs 5 --first 0 --speed 15   # parts 0-3 side by side, then
acs-capture summarize C/runs/refuse/runs.jsonl --speed 15       # over the parts' runs.jsonl joined
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
`room.vsf` for move-room, `runs.jsonl`, `summary.json`, start-up
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
whether 5 or 28 pixels of the bar were left. Our engine now rolls the
same way (magic 4.1).

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
| `LIN` register | VIC-II raster line, for counting frames | VICE's register list |

No address in the original program is used.

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
   3. waits 10 million instructions: the long message means it worked, the
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
      cost 40, 60 wide-jitter runs; the profile is read right after the
      outcome, split by outcome.
   4. *Starting Power above Wisdom 25*: one player of each Wisdom 26..31,
      the start chain and one profile read.
   5. *Drift, decay and healing* (`drift`): a Speed 1 player (short turns),
      Wisdom 20, Constitution 20; the profile read at the top of each of 40
      turns, 3 jitters per case. Cases: Power -10 and +10 (cost-0 spells,
      permanent bit); a temporary Life Force +10 at Life Force 20 (no
      healing mixed in); natural healing alone at Life Force 5; a permanent
      Life Force +10 at 20 (does it stay; which bit-6 value is temporary);
      the disease stack (temporary +10 on turn 1, permanent -10 on turn 2).
   Then the loose ends (whether taking costs exactly one step, the first
   take after entering a room ending the turn, the obstacle's 1-pixel
   drain, a room door back out, a locked door, retired slot 2).
7. **The rest of the captures-needed lists** in `docs/rules/`: turn order
   and creature phases, the adjacency slowdown, creature paths.
