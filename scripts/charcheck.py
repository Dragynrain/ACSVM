#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Checks a new-character sweep (`acs-capture run mem --set-random-state
--draws` from the start-up add-player screen, docs/capture.md 10.18) against
the characters spec (docs/rules/characters.md 1, 2).

Each run logs every draw of the random state: the program stores the state
twice per draw (first state XOR line, then the new state), so the rolls are
the second of each pair, checked against the generator
(docs/research/random-rolls.md). A die is raw // 42 + 1, drawn again from 252;
the second-chance roll of 0..2 is raw // 85, drawn again at 255.

The record watched is player slot 1 at $F59F (the stored record layout,
capture.md 9): +0 Constitution (low 6 bits; top 2 magic defense), +1
Strength (low 5 bits), +2 Dexterity, +3 Life Force, +4 Speed (low 4 bits),
+5 Power, +6..+10 Dodge, Parry, Armor, Melee, Missile, +15 Size, +17 Wisdom
(low 5 bits), +18/+19 gold, +20 weapon, +21 armor worn.

    python scripts/charcheck.py RUNS.jsonl [-v]
"""
import argparse
import json

BASE = 0xF59F


def gen(state, line):
    return ((state ^ (line & 0xFF)) * 69 + 131) & 0xFF


def rolls(v):
    """The run's draws as raw numbers, checking each against the generator."""
    d = v["draws"]
    out, state, bad = [], v["run"] & 0xFF, 0
    for a, b in zip(d[0::2], d[1::2]):
        # The first store is the state XOR the raster line, the second the roll.
        if b["raw"] != gen(a["raw"], 0):
            bad += 1
        out.append(b["raw"])
        state = b["raw"]
    return out, bad


class Stream:
    def __init__(self, raws):
        self.raws, self.i, self.redraws = raws, 0, 0

    def roll(self, n):
        while True:
            r = self.raws[self.i] // (256 // n)
            self.i += 1
            if r < n:
                return r
            self.redraws += 1


def predict(raws):
    s = Stream(raws)

    def three():
        return sum(s.roll(6) + 1 for _ in range(3))

    t = {k: three() for k in ("str", "con", "dex", "wis", "size")}
    second = s.roll(3) == 0
    if second:
        for k in ("str", "con", "dex", "wis"):
            t[k] = max(t[k], three())
    t.update(
        speed=5 + t["dex"] // 7,
        dodge=3 * t["dex"] - t["size"],
        parry=t["dex"] + t["wis"],
        melee=t["size"] + t["str"] + t["dex"],
        missile=t["dex"] + t["str"],
        armor=0,
        lf=t["con"],
        power=t["wis"],
        md=1 if t["wis"] >= 16 else 0,
        gold=0,
        weapon=0,
        worn=0,
    )
    return t, second, s


def record(v):
    w = v["watch"]
    b = {int(a, 16) - BASE if isinstance(a, str) else a - BASE: x for a, x in zip(w, v["end"])}
    return dict(
        con=b[0] & 63, md=b[0] >> 6, str=b[1] & 31, dex=b[2], lf=b[3], speed=b[4] & 15,
        power=b[5], dodge=b[6], parry=b[7], armor=b[8], melee=b[9], missile=b[10],
        size=b[15], wis=b[17] & 31, gold=b[18] | b[19] << 8, weapon=b[20], worn=b[21],
    )


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("runs")
    ap.add_argument("-v", action="store_true")
    a = ap.parse_args()
    n = ok = seconds = genbad = redrawn = 0
    hist = {}
    for text in open(a.runs, encoding="utf-8"):
        v = json.loads(text)
        if v.get("kind") != "mem":
            continue
        n += 1
        raws, bad = rolls(v)
        genbad += bad > 0
        try:
            want, second, s = predict(raws)
        except IndexError:
            print(f"run {v['run']}: too few draws ({len(raws)})")
            continue
        got = record(v)
        diff = {k: (want[k], got[k]) for k in want if want[k] != got[k]}
        seconds += second
        redrawn += s.redraws > 0
        for k in ("str", "con", "dex", "wis", "size", "speed"):
            hist.setdefault(k, {}).setdefault(got[k], 0)
            hist[k][got[k]] += 1
        if not diff and s.i == len(raws):
            ok += 1
        if diff or s.i != len(raws) or a.v:
            print(f"run {v['run']}: draws {len(raws)} used {s.i} second {second} redraws {s.redraws} diff {diff}")
    print(f"{ok} of {n} runs exact; {seconds} took the second chance, {redrawn} had a redraw, "
          f"{genbad} with a draw off the generator")
    for k, h in hist.items():
        print(f"  {k}: " + " ".join(f"{x}:{h[x]}" for x in sorted(h)))


if __name__ == "__main__":
    main()
