# Research brief: fighters and combat

A request to the project's clean-room researcher (docs/research/README.md).
Licensed CC BY 4.0. Status: first pass done 2026-11-13 (items 1, 2, 3, 5,
6, 7, 8; item 4 traced only); melee confirmed by capture where the
inputs reach and in the engine (`docs/rules/combat.md`). Pass 2 done
2026-11-13 (all seven items; 1, 3, 5 and 6 run): confirmed by capture
except where noted under "Findings", specified in `docs/rules/combat.md`
(firing in its section 8, not in the engine yet).

## Why

Combat is the next big rule area (design.md, Phase 4). The manual gives
its inputs (design.md "Combat": skills, weapon and armor adjustments,
Dodge and Parry, weapon Power, Size, Dexterity, Life Force over
Constitution, armor Power, break chances, magic defense) but no formula,
and every formula takes rolls. The random-rolls passes showed that the
generator (`$1CC9`) can be set and every draw logged
(capture.md 10.9, 10.14), so each rule the researcher reports can be
confirmed exactly, roll by roll, the way creatures' goals and steps were.
Capture alone can't separate formulas that differ by a few points.

The creature rules so far stop where a fighter reaches a foe: the engine
only reports `CreatureConfronts` (creatures 7.1). The researcher's third
pass mentioned a second rank table used by fighters, not yet reported.

## What we ask

For the C64 version, in plain language, with the rolls each step takes
(range, and what it is compared against), in the order they are drawn:

1. **The attack decision.** When a fighter creature attacks: beside its
   chosen goal only, or anyone it meets on the way (the manual: "may
   attack others"), and how it chooses; whether attacking ends its turn or
   costs moves; the "second table" fighters use, and what it ranks.
2. **A melee attack, start to end.** To-hit: what is added and compared
   (Melee skill, the weapon's and armor's attack adjustments, Armor
   skill, the defender's Parry or Dodge); damage: what it is made of
   (weapon Power, Size, Dexterity, Life Force above Constitution, Power)
   and what armor takes off; what a hit lowers (Life Force and Power, by
   how much each). Captured: an unarmed fighter never attacked; an
   armed one attacked again and again in one turn ("EE misses T", a
   button press each).
3. **The player's attack.** Moving into an individual: when it attacks
   and when it is refused; what it costs in moves; extra attacks (Baber:
   Speed 10 or more gives an extra melee attack at a turn's end).
4. **Firing.** A missile weapon's range, the line of fire (diagonals,
   who in the line can be hit, obstacles), to-hit against Dodge, damage,
   ammunition if any; when a creature fires instead of closing in.
5. **A readied weapon.** What readying changes (Parry needs a readied
   melee weapon); what readying armor costs (Baber: the rest of the
   turn's moves).
6. **Breaking.** The 0-15% break chance of weapons and armor: when it is
   rolled, and what breaking does.
7. **After a hit.** A creature hurt by a player changing alignment or
   setting (a friend to neutral, peaceful to aggressive, a smart
   aggressive neutral to enemy): when, and the smartness roll's part.
8. **Magic defense** against weapons (4 levels: the strongest blocks
   non-magic weapons and halves magic ones).

Enough for a first pass: items 1 and 2, with the draw order, so
`armed` can be checked roll by roll (how many attacks a turn, each
to-hit and damage roll, T's Life Force after each). Then 3 and 5. Also:
the one roll more an unarmed fighter enemy's turn takes before its first
step (capture.md 10.14.6), which the goal and step rules don't give.

Also, from capture (capture.md 10.14.4, 10.14.5), small ones if they are
on the way: the square a dropped treasure lands on (an overburdened
creature's drop and a friend's gift take one to five pairs of rolls).

## How the answers come back

As docs/research/README.md 3 and 4: `findings.md` in plain language only.
Addresses only for state the harness must read to check a hit (a
creature's or player's Life Force, Power, weapon state), and only if the
creature records already named in capture.md 9 don't hold it.

## Inputs

In `ACSVM-research/research/combat/inputs/` (copies). VICE 3.10 `x64sc`
snapshots saved with their disks; start VICE with `-controlport2device
37` (capture.md 2.2). The scenario is ours (capture.md 7, 10.10): the
open field, the map door at 12,16 into a 15 x 10 room (entry 4,4).

| File | What |
| --- | --- |
| `armed-disk.d64`, `armed-turn-end.vsf`, `.png` | as `fight`, but EE owns and holds our melee weapon (thing 59, BLADE: power 10, no break chance, attack adjustment 0, usable by anyone); same moment |
| `runs/armed/round.jsonl` | 32 runs from `armed-turn-end.vsf` as below: T's Life Force went from 40 to 24..34 in 19 runs and stayed 40 in 13; 7 to 13 messages answered a run (`messages/`: run 0's first eight, "EE misses T") |
| `fight-disk.d64` | T (Speed 6, Constitution and Life Force 40, no weapon) and EE in slot 0: an aggressive, brave enemy, Speed 3, Strength 10, Constitution and Life Force 20, no weapon, on 6,4 |
| `fight-turn-end.vsf`, `.png` | T on 4,4 at the end of its first turn in the room (the bar about to run out); EE's turn comes next |
| `runs/fight/round.jsonl` | 32 runs from that snapshot, the random state set to the run's index, 16 million instructions: every draw (raw and raster line; two stores a draw), T's Life Force (`$F5A2`), EE's square (`$E1B0`, `$E1B1`), Constitution (`$E1A3`) and Life Force (`$E1A6`), and the times messages were answered. EE never attacked: it stepped round T |

Armor, a player's weapon and a missile case come in a second input set.

## Pass 2

Briefed 2026-11-13. The first pass's melee rules are in the engine; what
it only traced needs runs. Everything below has inputs in
`inputs/pass2/` (table at the end of this section). Same scenario and
moment as the first pass unless a row says otherwise: T on 4,4 at the
end of its first turn in the room, EE the `armed` enemy on 6,4 holding
our blade (power 10, adjustment 0, no break chance) unless the row says.

### What we ask

1. **Parry and the counter-blow** (findings 2.2 items 4 to 7, 2.4): run
   `parry` (T holds the blade too). Check the parry chance, the Parry
   check and its 30-in-100 Armor check, the counter-blow after a miss,
   its hit chance (the attacker's blows left) and what a damaging counter
   uses up. Already seen in 4 runs of ours: T's held weapon reads 59 at
   `$F5B3`; EE's Life Force fell 20 to 13 in two; 23 to 25 button presses
   a run against the `armed` runs' 7 to 13.
2. **Breaking and disappearing** (findings 4): `break` (the blade's break
   chance 15, both holding it: the striking weapon's break roll on a
   parry; does it end the attack; the message) and `once` (the blade set
   to disappear after use: after the first damaging blow, and does the
   attack go on).
3. **Armor** (findings 2.3, 2.6): `mail` (T wears our armor: power 2,
   adjustment -30, break chance 15, not magic) and `mail-magic` (magic,
   no break chance). The armor term in T's own parry and in EE's damage,
   the halving against a non-magic weapon, the armor's break roll and
   when it is drawn, the Armor skill check.
4. **Magic defense** (findings 2.6): `md1`, `md2` (non-magic blade) and
   `md3-magic` (a magic blade). Halving, "does nothing and the rest of
   the attack is called off", the order against armor.
5. **Firing** (findings section 6, traced only): `bow` (EE on 9,4 holds
   our missile weapon, power 10, range 7, adjustment 0, no break chance,
   and owns no melee weapon; T five squares west) and `bow-line` (the
   same with a peaceful friend FF, Speed 0, on 6,4 in the line). When it
   fires instead of closing in, the number of shots, the aim's spread in
   words (what "hundredths" are of: an angle, a slope, a step pattern),
   who is hit, the dodge, the missile flying on.
6. **The player's attack** (findings section 3, traced only): four
   snapshots with T's first turn in the room under way and a creature of
   ours on 5,4, east of T: push the stick right once. `player-friend` (T
   holds the blade; a peaceful friend: does it turn neutral, then
   aggressive on a second hit), `player-neutral` (an aggressive neutral
   with Wisdom 10: the smartness roll for turning enemy),
   `player-own` (T owns the blade but holds none: the ready message, the
   move it costs, then the attack) and `player-none` (T owns no weapon:
   nothing happens, no move used?). Also: what a player's attack does to
   T's bar, and whether the blows count comes from the moves left.
7. **Readying before stepping** (findings 1.7, traced only): any of the
   cases where it shows, or say it can't be seen from these inputs.

Enough for this pass: 1, 3, 5 and 6. Out of scope: blows on the world
map (our scenarios have no map creatures yet), the dropped treasure's
square (inputs are the first pass's `burden` runs, capture.md 10.14.5,
not copied: ask if wanted).

### How the answers come back

As before: `findings.md` gets a "Pass 2" section, plain language only,
the same rules for addresses. For each item say run or traced, and how
many runs.

### Inputs (pass 2)

In `ACSVM-research/research/combat/inputs/pass2/` (copies; made by
`capture/runs/rr8/`, capture.md 10.15). Each case has `NAME-disk.d64` and
a snapshot with its screenshot; `cases.txt` gives each case's exact
scenario keys. Our things: BLADE (thing 59), BOW (41), MAIL (40). T is
the first player's record, EE creature slot 0, FF or NN slot 1.

| Case | Snapshot | What | 16 runs (`runs/NAME.jsonl`) |
| --- | --- | --- | --- |
| `parry` | `parry-turn-end.vsf` | as `armed`, T holding the blade too | T hurt in 8, EE in 9; up to 25 presses |
| `break` | `break-turn-end.vsf` | as `parry`, the blade's break chance 15 | T's blade gone in 1 |
| `once` | `once-turn-end.vsf` | as `armed`, the blade disappears after use | EE's blade gone in 11 |
| `mail` | `mail-turn-end.vsf` | as `armed`, T wears MAIL: power 2, adjustment -30, break chance 15, not magic; T's Armor skill 0 | T hurt in 13; the mail gone in 1 |
| `mail-magic` | `mail-magic-turn-end.vsf` | as `mail`, the mail magic, no break chance | T hurt in 11 |
| `md1` | `md1-turn-end.vsf` | as `armed`, T's magic defense 1 | T hurt in 10 |
| `md2` | `md2-turn-end.vsf` | as `armed`, T's magic defense 2 | T never hurt |
| `md3-magic` | `md3-magic-turn-end.vsf` | as `armed`, T's magic defense 3, the blade magic | T hurt in 12 |
| `bow` | `bow-turn-end.vsf` | EE on 9,4 holds BOW (power 10, range 7, adjustment 0, no break chance), owns no blade | T hurt in 5; up to 6 presses |
| `bow-line` | `bow-line-turn-end.vsf` | as `bow`, a peaceful friend FF (Speed 0, Constitution and Life Force 40) on 6,4 | FF hurt in 4, T never |
| `player-friend` | `player-friend-room.vsf` | T's first turn in the room under way, holding the blade; a peaceful friend FF (Speed 0, Constitution and Life Force 40, Wisdom 13) on 5,4 | none: push right |
| `player-neutral` | `player-neutral-room.vsf` | as above, an aggressive neutral NN, Wisdom 10 | none |
| `player-own` | `player-own-room.vsf` | T owns the blade, holds none; FF on 5,4 | none |
| `player-none` | `player-none-room.vsf` | T owns nothing; a peaceful neutral NN on 5,4 | none |

The runs: the random state set to the run's index at the snapshot, 16
million instructions, every draw logged (raw and raster line, two stores
a draw), messages answered, and these bytes watched (findings section 8):
T's Life Force, Power, Parry, Armor skill, weapon and armor (`$F5A2`,
`$F5A4`, `$F5A6`, `$F5A7`, `$F5B3`, `$F5B4`), EE's Constitution, Life
Force, Power, square and weapon (`$E1A3`, `$E1A6`, `$E1A8`, `$E1B0`,
`$E1B1`, `$E1B7`), slot 1's Life Force (`$E1CB`) and the blows left
(`$B799`). Some samples read a wrong value for one sample while memory is
banked (capture.md 10.14.3): drop a change undone at the next sample.

## Findings

First pass (2026-11-13), summarized in plain language; the rules are
specified in `docs/rules/combat.md`.

- **Who attacks.** A fighter that owns a melee weapon attacks by stepping
  into the other's square: its step choice scores squares holding
  individuals by a table of costs by class (1, 3 or closed). Slinkers and
  fighters owning no melee weapon never strike. Attacking ends the turn.
  The unarmed enemy's extra roll is a second smartness check (helping an
  ally). **Confirmed** by capture (`armed`, `fight`, capture.md 10.14.9).
- **Blows**: 4 x moves left / full moves + Dexterity / 8 - 1, at least 1;
  a damaging blow uses two. **Confirmed.**
- **To-hit**: weapon and armor adjustments, Melee, 5 per blow left minus
  20, a quarter of Power, minus a wound penalty, held to 14..90.
  **Confirmed** (122 rolls). **Parry** (a held melee weapon only) and the
  **counter-blow** after a miss: the researcher's runs only.
- **Damage**: the weapon's Power scaled by Dexterity plus Size, a heavy
  blow on a roll below Dexterity, at most 1.5 x; Power takes a share,
  Life Force the rest. **Confirmed** (25 hits). Armor, magic defense,
  breaking and things that disappear after use: traced only.
- **Learning**: Melee and Parry up 1 on a roll above the skill; Armor
  skill on a roll below 65 minus it. Melee **confirmed**.
- **The player's attack**, readying costs and the alignment changes after
  a player's blow: traced only.
- **Firing**: traced only; not in the engine.

Second pass (2026-11-13). The researcher replayed our 16 runs of each
case with the draws forced to ours and checked the rules roll by roll; we
then checked them on the same runs with our own checker
(`scripts/combatcheck.py`, capture.md 10.14.10) and ran the player's
attack ourselves.

- **Parry and counter-blows** as the first pass said; a damaging counter
  uses one more of the attacker's blows. **Confirmed** (32 runs).
- **Armor**: plain armor takes its Power off; magic armor against a plain
  weapon halves (rounding up) first. Its break roll comes after every blow
  that reaches it, even at chance 0, and the attack goes on when it
  breaks. **Confirmed** (32 runs). Its term in the wearer's own to-hit and
  parry, and the Armor check: traced only.
- **Magic defense**: level 1 halves, level 2 stops the blow and the rest
  of the attack, level 3 halves a magic weapon. **Confirmed** (48 runs).
  Level 3 against a plain weapon, and the order against armor: traced.
- **Breaking and disappearing**: the striking weapon's break roll after a
  parry, **confirmed** (one break); whether it ends the attack, traced. A
  weapon set to disappear goes at its first damaging blow and **ends the
  attack**, **confirmed**. Our runs add: the creature then has one move
  fewer on its turns beside the player (cause unknown; combat 6.5).
- **Firing**: when a creature fires, 2 shots a turn here, each a break
  roll, a sideways deviation of 0 to spread - 1 hundredths of the distance
  and a side; the spread counts the shots after this one (a correction to
  the first pass); the missile reaches whoever's square it crosses and
  flies on after a dodge. **Confirmed** (32 runs, 64 shots); the exact
  arithmetic of the sideways offset is open. Walls, diagonals, a foe
  beside the shooter, breaking: traced.
- **The player's attack**: the creature's blows and the moves left to 0;
  a friend hit turns neutral, then aggressive; an aggressive neutral rolls
  0 to 9 below Wisdom minus 5 after each damaging blow to turn enemy;
  owning no weapon, nothing happens. **Confirmed** (36 runs of ours, the
  stick pushed). Owning a weapon but holding none brings a menu (NONE or a
  weapon; readying costs a move): the researcher's runs only.
- **Readying before stepping**: after the smartness checks, one move; the
  researcher's experiment only.

Next pass: the second pass's own list (findings P2.8): armor on the
wearer's side, a weapon breaking mid-attack, magic defense 3 with a plain
weapon and with armor, the rest of firing, the `player-neutral` snapshot
remade, the dropped treasure's square, map encounters; and ours: the move
a creature loses after its weapon disappears, and the round's-end
percentile (creatures 2.4).
