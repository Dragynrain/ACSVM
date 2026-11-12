#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Checks a random-state sweep (`acs-capture run mem --set-random-state`)
against the clean-room researcher's generator (docs/research/random-rolls.md,
Findings; docs/capture.md 10.9, 10.12).

The generator, as reported: raw = ((state XOR line) x 69 + 131) mod 256, which
is also the new state, `line` being the low 8 bits of the raster line at the
draw. A roll of 0..n-1 is raw // (256 // n), drawn again when it is n or more.

Each run of the sweep set the state to its index just before the draws. The
raster line of each draw is unknown but the same in every run (the same
snapshot, the same timing), so every combination of candidate lines is tried
and the ones that predict every run without a redraw are reported.

    python scripts/linefit.py heal RUNS.jsonl --moves 15
    python scripts/linefit.py tick RUNS.jsonl --due 71

heal: draw 1 is the idle move's roll of 0..moves-1 (heals 1 on a 0), draw 2
the top-of-turn die 1..6 (heals 1 on a 2); Life Force (F5A2) is below
Constitution in the snapshot, so the gain is 0, 1 or 2.
tick: draw 1 is the idle move's roll (moves 1, no effect), draw 2 the tick's
die 1..6; the entry's due round (F65F, low byte) becomes DUE + the die.
"""
import argparse
import json


def draw(state, line):
    return ((state ^ (line & 0xFF)) * 69 + 131) & 0xFF


def roll(raw, n):
    """0..n-1, or None when the program would draw again."""
    r = raw // (256 // n)
    return r if r < n else None


def load(path):
    runs = []
    for text in open(path, encoding="utf-8"):
        v = json.loads(text)
        if v.get("kind") == "mem":
            runs.append(v)
    return sorted(runs, key=lambda v: v["run"])


def byte(v, addr, which):
    i = v["watch"].index(addr)
    return v[which][i]


def first_new(v, addr):
    for c in v["changes"]:
        if c["addr"] == addr:
            return c["to"]
    return None


def predict_heal(s, l1, l2, moves):
    r1 = draw(s, l1)
    a = roll(r1, moves)
    r2 = draw(r1, l2)
    b = roll(r2, 6)
    if a is None or b is None:
        return None
    return (a == 0) + (b + 1 == 2)


def predict_tick(s, l1, l2, due):
    r1 = draw(s, l1)
    r2 = draw(r1, l2)
    b = roll(r2, 6)
    if b is None:
        return None
    return (due + b + 1) & 0xFF


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("mode", choices=["heal", "tick"])
    ap.add_argument("runs")
    ap.add_argument("--moves", type=int, default=15)
    ap.add_argument("--due", type=int, default=71)
    a = ap.parse_args()
    runs = load(a.runs)
    obs = {}
    for v in runs:
        if a.mode == "heal":
            obs[v["state"]] = byte(v, "F5A2", "end") - byte(v, "F5A2", "start")
        else:
            obs[v["state"]] = first_new(v, "F65F")
    results = []
    for l1 in range(256):
        for l2 in range(256):
            right = wrong = 0
            for s, o in obs.items():
                p = predict_heal(s, l1, l2, a.moves) if a.mode == "heal" else predict_tick(s, l1, l2, a.due)
                if p is None:
                    continue
                if p == o:
                    right += 1
                else:
                    wrong += 1
            results.append((wrong, -right, l1, l2))
    results.sort()
    print(f"{len(obs)} runs; observed: {sorted(set(map(str, obs.values())))}")
    for wrong, right, l1, l2 in results[:5]:
        print(f"lines {l1:3d}, {l2:3d}: {-right} right, {wrong} wrong")
    best = results[0]
    exact = [r for r in results if r[0] == 0]
    print(f"exact line pairs: {len(exact)}; best has {best[0]} wrong of {best[0] - best[1]}")


if __name__ == "__main__":
    main()
