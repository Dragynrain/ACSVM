# Clean-room research

How the project learns rules from the original program's code without
the people who implement those rules ever seeing that code. The policy
is CONTRIBUTING.md section 3; this is the working process, first run in
November 2026 (`random-rolls.md`). Licensed CC BY 4.0.

## 1. Roles

- **Implementer.** Builds the engine, the capture harness and the rules
  specs, from the manual, the data, captures and research findings.
  Never reads the original program's code. Every session working in
  this repository is an implementer.
- **Researcher.** Reads, disassembles and traces the original program
  and reports what it does in plain language. Works only in the research
  folder (`D:\projects\ACSVM-research`), never in this repository, and
  never implements the rules it reports. An AI researcher is a separate,
  fresh Claude session started in the research folder (so it loads no
  project instructions, memory or engine context), used for nothing else.
- **Maintainer.** Starts each research pass and authorizes the
  researcher to read the program for it, reviews the findings, and
  decides what goes into the specs.

One person or session never holds both the implementer and researcher
roles for the same rule. A session that has read the program's code for
one rule is a researcher from then on.

## 2. When to research and when to capture

Capture (`docs/capture.md`) is cheaper and comes first. Research is for
what capture can't settle:

| Situation | Use |
| --- | --- |
| Deterministic and visible on screen (movement, weight, costs in steps) | Capture: one run shows it |
| Random, and statistics can't separate the candidate rules (60 runs a point gives about ±10 points) | Research the rule, then confirm it exactly by capture |
| The harness needs a hidden value it can't see (the random state, a stat not on any screen) | Research its address, for the harness only |
| Edge cases capture can't reach (clamps, rounding, overflow, rare branches) | Research, then a targeted capture |
| Deterministic and visible, but the rule depends on many cases at once (goal choice, path tie-breaks, turn scheduling across places) | Capture a few cases first; if they don't fit a simple rule, research the rule from the cases seen, then confirm it with a few targeted captures |
| The manual or the data already says it | Neither: cite it |

The cast chance is the worked example. The statistical fit was within 10
points everywhere the grids looked, and it was still the wrong rule: the
program never goes above 90% or below 14%.

**When to stop capturing.** Hand a question to research when a capture
series has run two rounds of cases without converging on a rule (each
new case needs a new hypothesis), when the rule needs more than about a
dozen hand-built scenarios to pin down, or when the candidate rules
differ only in cases the harness can't set up. The healing rate
(magic 5.3) and players in different places (creatures 2.3) went that
way: a few captures each showed behaviour no simple rule fits.

**What a pass needs.** The cheapest input is a snapshot taken just
before the behaviour (`acs-capture snap`, capture.md 10.11), with the
captured runs it must explain. The first pass found the generator in a
few hours from such a snapshot; without one, much of a pass goes on
getting the program to the right moment.

## 3. The loop

1. **Brief** (implementer). Write `docs/research/NAME.md` from the
   template below: the questions in rule terms, what is already captured
   to check against, and what is out of scope. Put copies of the inputs
   (snapshots, scenario disks, captured runs) in
   `ACSVM-research/research/NAME/inputs/`. Snapshots taken just before
   the behaviour in question save the researcher the most time.
2. **Research pass** (researcher). The maintainer starts a fresh session
   in the research folder with the prompt template below and a time box.
   Private notes, scripts and disassembly go in
   `ACSVM-research/research/NAME/notes/`. The only hand-off is
   `ACSVM-research/research/NAME/findings.md`.
3. **Review** (implementer and maintainer). Read `findings.md` and only
   that file. Check it is plain language: no code, pseudo-code,
   disassembly, opcodes or byte listings. Arithmetic in words and the
   addresses the brief asked for are fine. Anything else goes back to
   the researcher.
4. **Confirm by capture** (implementer). A finding is a hypothesis until
   the original, run as a black box, agrees with it. Where the finding
   gives a hidden state's address, set the state and check outcomes
   exactly (capture.md 10.9: all 256 random states before a cast). Record
   the experiment in capture.md and any address in its section 9, with
   the researcher as the source.
5. **Spec** (implementer). Write the rule into `docs/rules/`, tagged
   **[research]** (with the date and the brief) and **[capture]** for
   the confirmation. A finding not yet confirmed is tagged
   "[research, unconfirmed]" and the engine's choice says so.
6. **Engine** (implementer). Implement from the spec, never from the
   findings file, with tests on the rule's edge cases.
7. **Close the pass.** Summarize the findings, in plain language, under
   "Findings" in the brief, with what was confirmed and what is open.
   Plan the next pass there too.

## 4. Isolation rules

1. Implementer sessions never open, list or search
   `ACSVM-research/research/*/notes/`.
2. The researcher never writes to this repository, never commits, and
   reads the repository only for what the brief points at (the brief,
   the specs it names, capture.md).
3. Findings carry rules and the addresses of hidden state, never the
   program's code in any form. Addresses are for the harness only; the
   engine never depends on one.
4. The researcher may read the program disks (`advcnst*.d64`), memory
   dumps and snapshots for the pass the maintainer authorized, in the
   research folder only. For everyone else the rule in CLAUDE.md stands.
5. Nothing derived from the program goes into the repository except
   plain-language rules and the listed addresses.

## 5. Briefs

| Brief | Status |
| --- | --- |
| [random-rolls.md](random-rolls.md): the generator, its state, rolls, casting | First pass done (November 2026); the cast chance confirmed and in the engine. Second pass done 2026-11-12 (items 7-13): healing and the temporary tick confirmed by capture and in the engine. Third pass done 2026-11-12: creatures' goals and steps and random creatures appearing confirmed by capture and in the engine; falling away captured, its percentile corrected (one roll; rooms empty at the round's end). Next: fighters (combat) |
| [turn-order.md](turn-order.md): the scheduler, players in different places, creature phases | First pass done 2026-11-12: the region-visit scheduler confirmed by capture and in the engine. Two rooms in one region captured; next: the round a player misses after a portal, a player's death |
| [combat.md](combat.md): fighters' attack decision, melee, firing, readied weapons, breaking | Briefed November 2026; first inputs (`fight` unarmed, `armed` with a melee weapon) ready. Not started |

## 6. Templates

**A brief** (`docs/research/NAME.md`):

```markdown
# Research brief: TOPIC

A request to the project's clean-room researcher (docs/research/README.md).
Licensed CC BY 4.0. Status: not started.

## Why
What capture can't settle, and what knowing it would let us do.

## What we ask
Numbered questions in rule terms. Name what is already captured, so each
answer can be checked. Say which answers are enough for a first pass.

## How the answers come back
As docs/research/README.md 3 and 4: findings.md in plain language only.

## Inputs
What is in ACSVM-research/research/NAME/inputs/ and what each shows.

## Findings
None yet.
```

**The researcher prompt** (pasted by the maintainer into a fresh session
started in `D:\Projects\ACSVM-research`). Fill in the capitals;
`random-rolls.md`'s first pass used this, with its VICE notes:

```text
You are the clean-room researcher for ACSVM, an open-source reimplementation of
Electronic Arts' Adventure Construction Set (1984). You study the original C64
program and report what it does in plain language. Other sessions implement the
rules from your reports and must never see the original's code.

AUTHORIZATION (from me, the maintainer): for this session only you may run,
inspect, disassemble and trace EA's C64 program (program disks, memory dumps,
VICE snapshots), entirely inside D:\Projects\ACSVM-research. The rule against
decoding the program disks still holds everywhere else.

READ FIRST (read-only; never edit anything under D:\Projects\ACSVM, never run git there):
- D:\Projects\ACSVM\docs\research\README.md   the process and its rules
- D:\Projects\ACSVM\docs\research\NAME.md     your brief
- SPECS AND CAPTURE SECTIONS THE BRIEF NAMES
Do not read D:\Projects\ACSVM\crates.

INPUTS: D:\Projects\ACSVM-research\research\NAME\inputs\ (copies). DESCRIBE EACH.
Program disks: advcnst1.d64 and advcnst2.d64 in D:\Projects\ACSVM-research\corpus\c64\
ACS-Jadeworld(September 2023 update)\ (copy disk 1: capture\boot\program1.d64). VICE 3.10:
E:\emu\vice\bin\x64sc.exe (binary and text monitors; capture.md 2 for the joystick).

THIS PASS (timebox HOURS): BRIEF ITEMS. Check your answers against WHAT IS CAPTURED.

OUTPUT: notes and tools in research\NAME\notes\ (never read by implementers);
your only hand-off is research\NAME\findings.md: plain language, no code,
pseudo-code, disassembly, opcodes or byte listings; addresses only where the
brief asks. For each finding give how sure you are and how you know (traced,
watchpoint, experiment), "not found" or "unsure" where honest, and what a next
pass should look at. Don't write to the ACSVM repo. End with a five-line summary.
```
