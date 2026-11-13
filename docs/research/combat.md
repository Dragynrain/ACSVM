# Research brief: fighters and combat

A request to the project's clean-room researcher (docs/research/README.md).
Licensed CC BY 4.0. Status: first pass done 2026-11-13 (items 1, 2, 3, 5,
6, 7, 8; item 4 traced only); melee confirmed by capture where the
inputs reach and in the engine (`docs/rules/combat.md`).

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

Next pass (inputs needed first): a missile case, armor and magic defense,
a defender holding a weapon (parries and counters by capture), the
player's attack driven by the joystick (friend, neutral, the alignment
changes), the dropped treasure's square, and blows on the world map.
