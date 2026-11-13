#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Checks the combat rules (docs/rules/combat.md, from the clean-room
researcher's first combat pass) against `acs-capture run mem --draws` runs
of the `armed` case (capture.md 10.14.7), roll by roll.

The room: T (player, Speed 6, Constitution and Life Force 40, Power 13,
Size 9, no weapon) on 4,4; EE (slot 0: an aggressive, brave enemy, Speed
3, Dexterity 13, Size 9, Melee 31, Power 13, Wisdom 13) on 6,4, holding
our blade (power 10, attack adjustment 0, no break chance). From the
snapshot: T's last idle move, then rounds of EE's turn (the top-of-turn
die, the two smartness checks of 0..9, then its steps by creaturecheck.py's
rule with T's square open at cost 1, until the chosen square is T's: the
attack), the round's end (the slot roll, a percentile when it is EE's
slot, the room's random-creature percentile) and T's turn (the die, then
its bar waited out: 6 moves, 3 with EE beside it), until the draws run out.

    python scripts/combatcheck.py RUNS.jsonl [-v]

Predicts every blow's hit, heavy blow and damage and T's Life Force after
each hit and each healing; compares them with T's Life Force byte (glitch
reads dropped) and the messages answered (two presses a miss, three a
hit).
"""
import argparse
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from creaturecheck import Draws, dist  # noqa: E402

INSIDE = (1, 13, 1, 8)
T_START, EE_START = (4, 4), (6, 4)


def clamp(v):
    return max(14, min(90, v))


def wound(con, life):
    return 30 * (con - life) // con if life < con else 0


class Out(Exception):
    pass


def simulate(d):
    """Rounds until the draws run out. Returns T's Life Force after each
    change and the presses each finished EE turn asked for (a miss two, a
    hit three)."""
    t = {"life": 40, "con": 40, "power": 13, "wis": 13}
    ee = {"melee": 31, "power": 13, "dex": 13, "size": 9, "life": 20, "con": 20}
    lives, presses, turns = [], 0, []
    ee_at = EE_START

    def heal():
        if t["life"] < t["con"]:
            t["life"] += 1
            lives.append(t["life"])

    def take(n):
        try:
            return d.take(n)
        except IndexError:
            raise Out

    try:
        if take(6) == 0:  # T's last idle move (unhurt: nothing)
            heal()
        while True:
            # EE's turn: the top-of-turn die, the two smartness checks.
            if take(6) == 1 and ee["life"] < ee["con"]:
                ee["life"] += 1
            take(10)
            take(10)
            me, stood, moves = ee_at, {ee_at}, 3
            attack = False
            while moves > 0:
                best = None
                for dx, dy in [(0, 1), (0, -1), (1, 0), (-1, 0)]:
                    sq = (me[0] + dx, me[1] + dy)
                    if not (INSIDE[0] <= sq[0] <= INSIDE[1] and INSIDE[2] <= sq[1] <= INSIDE[3]):
                        continue
                    r = take(3)
                    cost = 1 if sq == T_START else (4 if sq in stood else 2)
                    score = 3 * dist(sq, T_START) + 5 * cost + 4 * r
                    if best is None or score < best[0]:
                        best = (score, sq)
                if best[1] == T_START:
                    attack = True
                    break
                me = best[1]
                stood.add(me)
                moves -= 1
            ee_at = me
            turn = {"blows": 0, "hits": []}
            if attack:
                blows = max(1, 4 * moves // 3 + ee["dex"] // 8 - 1)
                turn["blows"] = blows
                while blows > 0 and t["life"] > 0:
                    chance = clamp(ee["melee"] + 5 * blows - 20 + ee["power"] // 4 - wound(ee["con"], ee["life"]))
                    if take(100) < chance:
                        if take(100) > ee["melee"]:
                            ee["melee"] = min(127, ee["melee"] + 1)
                        take(100)  # parry: always drawn; T holds nothing
                        p = 10
                        build = max(0, min(30, ee["dex"] + ee["size"] - 6)) - 15
                        dmg = build * p // 30 + p
                        if take(100) < ee["dex"]:
                            dmg += p // 2
                        dmg = min(dmg, 3 * p // 2)
                        share = min(dmg * t["power"] // (t["power"] + t["life"]), (t["power"] + 1) // 2)
                        t["power"] = max(0, t["power"] - 2 * share)
                        t["life"] = max(0, t["life"] - (dmg - share))
                        lives.append(t["life"])
                        turn["hits"].append(dmg)
                        presses += 3
                        blows -= 2
                    else:
                        turn["hits"].append(0)
                        presses += 2
                        blows -= 1
            else:
                for _ in range(moves):
                    if take(3) == 0 and ee["life"] < ee["con"]:
                        ee["life"] += 1
            turns.append(turn)
            simulate.presses = presses
            # The round's end: the slot roll, a percentile when it is slot
            # 0 (EE's), the room's random-creature percentile.
            if take(16) == 0:
                take(100)
            take(100)
            # T's turn: drift, the die, then its bar waited out, a roll of
            # 0..full-1 each idle move.
            if t["power"] < t["wis"]:
                t["power"] += 1
            if take(6) == 1:
                heal()
            # Halved (6 to 3) only with EE, aggressive and not a friend, on
            # one of the 4 squares beside T (creatures 2.6).
            full = 3 if dist(ee_at, T_START) == 1 else 6
            for _ in range(full):
                if take(full) == 0:
                    heal()
    except Out:
        pass
    return lives, turns


def observed_life(v):
    """T's Life Force byte with the one-sample glitch reads dropped (a
    change undone at the next sample, with EE's square read off the room
    at the same time: memory banked in, capture.md 10.14.3)."""
    ch = [c for c in v["changes"] if c["addr"] == "F5A2"]
    vals, i = [], 0
    while i < len(ch):
        c = ch[i]
        if i + 1 < len(ch) and ch[i + 1]["to"] == c["from"] and ch[i + 1]["t"] - c["t"] <= 20000:
            i += 2
            continue
        vals.append((c["t"], c["to"]))
        i += 1
    return vals


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("runs")
    ap.add_argument("-v", action="store_true")
    a = ap.parse_args()
    right = wrong = hits = misses = 0
    for text in open(a.runs, encoding="utf-8"):
        v = json.loads(text)
        d = Draws([x["raw"] for x in v["draws"]])
        simulate.presses = 0
        lives, turns = simulate(d)
        seen = [val for _, val in observed_life(v)]
        n = min(len(seen), len(lives))
        # Presses: every finished EE turn's messages were answered; the
        # run may end inside a later one.
        pressed = len(v["answered"])
        # Exact, except that a run may end inside a hit's messages (one
        # press and T's Life Force write still to come).
        ok = seen[:n] == lives[:n] and abs(len(seen) - len(lives)) <= 1 and abs(pressed - simulate.presses) <= 1
        right += ok
        wrong += not ok
        for tu in turns:
            hits += sum(1 for h in tu["hits"] if h)
            misses += sum(1 for h in tu["hits"] if not h)
        if a.v or not ok:
            print(f"run {v['run']}: {'ok ' if ok else 'BAD'} turns {turns} life {lives} presses {simulate.presses}; "
                  f"seen life {seen} presses {pressed}; draws {d.i} of {len(d.raws)}")
    print(f"{right} runs right, {wrong} wrong; {hits} hits and {misses} misses predicted")


if __name__ == "__main__":
    main()
