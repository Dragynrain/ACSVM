# Playtest guide

Thank you for trying ACSVM. It is a new engine for *Adventure
Construction Set* adventures that runs in your browser, and before 1.0
we want people who know the original to play it and tell us where it
goes wrong.

**Play it:** <https://dragynrain.github.io/ACSVM/>. Nothing is
installed, and any file you open is read on your own machine and never
uploaded. The page counts visits and a few events anonymously, with no
cookies ([what it counts](client.md#11-counts)).

## What to try

1. **First Voyage**, the tutorial on the title screen: about fifteen
   minutes, with notices that explain each step.
2. **Your own adventures**: a C64 `.d64` disk image, an Amiga `.adf`
   disk image (or the `adventure` file from one), or a PC
   `ADVEN.FPY` / `ADVEN.HRD`. Apple II disks don't open yet. Drag the file onto the page or use
   **Open...**. Fan adventures are especially welcome. Every adventure
   you bring is a check of the rules we can't run ourselves.
3. **Hot-seat**: Add Someone brings in up to four characters, who take
   turns, as in the original.

Keys: the arrows or WASD (or the number pad) move, and so does a click
on a square beside you; Space or Enter ends the turn, Escape cancels.
Diagonal steps (QEZC) are off, as in the original; tick Diagonal moves
for them. On a phone or tablet the on-screen pad appears when you touch
the screen.

Your options and your last game are kept in the browser: open the same
adventure again (or Play First Voyage) and you are back where you were.
Restart begins a new game.

## What we want to hear about

- **Anything that plays differently from the original**: a fight that
  goes oddly, a creature that moves wrongly, a spell, a store price, a
  door, an encounter. Say whether you remember it or have checked it
  again in an emulator. Both help; the second helps most.
- **Anything that breaks**: a crash, a hang, a wrong picture, text
  running off the screen, a file that won't open.
- **Anything confusing**: in the viewer, or in First Voyage.

## How to report

Use **Help > Report a problem...** in the viewer, or open
<https://github.com/Dragynrain/ACSVM/issues/new/choose> and pick a form
(a free GitHub account is needed). Each form asks for the build, shown
at the foot of the title screen and in the Help menu (for example
`0.1.0 (abc1234)`).

**A save file helps most.** In walk mode, **Save to file…** writes the
seed and every command played, so we can replay your game move for
move up to the problem. For First Voyage, attach it to the issue. For
an EA adventure or someone else's fan adventure, please don't attach
the save or the adventure to a public issue: describe what happened,
and say in the issue that you have a save; we will ask for it
privately.
