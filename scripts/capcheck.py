#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Checks the first player's moves at each turn's start against the weight
slowdown (inventory 3.2, 3.3) under several readings of the capacity, from
`acs-capture run mem` runs that watch the moves byte `$719A`, the turn byte
`$71FA` and the player's Constitution, Strength and Life Force (`$F59F`,
`$F5A0`, `$F5A2`; capture.md 10.39).

Each refill of `$719A` (a rise) while the turn byte is 1 is a turn start;
the Life Force read is the last value seen at or before it (`--at refill`)
or at the turn byte's change to 1 (`--at turn`). The capacity is
(Strength - the wound term) x 100 with the wound Constitution - Life Force
taken by `--wound`: `zero` (halved toward 0), `floor`, `ceil`, `exact` (no
rounding: 50 a point), and each of these `-clamped` (a wound below 0 counts
as 0). The moves: Speed when the load is at most half the capacity, else
Speed x 2 x (capacity - load) / capacity rounded down; 0 at or past it.

    python scripts/capcheck.py RUNS.jsonl... --speed 15 --load 500 [--at refill|turn] [-v]
"""
import argparse
import json
import math

WOUNDS = ["zero", "floor", "ceil", "exact"]


def term(w, how):
    base, _, clamp = how.partition("-")
    if clamp and w < 0:
        w = 0
    if base == "zero":
        return math.trunc(w / 2) * 100
    if base == "floor":
        return (w // 2) * 100
    if base == "ceil":
        return -((-w) // 2) * 100
    return w * 50


def moves(speed, strength, wound, load, how):
    cap = strength * 100 - term(wound, how)
    if cap <= 0 or load >= cap:
        return 0
    if 2 * load <= cap:
        return speed
    return speed * 2 * (cap - load) // cap


def readings(v, at):
    """(Constitution, Strength, Life Force, moves) at each turn start."""
    st = dict(zip(v["watch"], v["start"]))
    cur = dict(st)
    who = st.get("71FA")
    lf_turn = cur["F5A2"]
    out = []
    for c in v["changes"]:
        a = c["addr"]
        if a == "71FA":
            who = c["to"]
            if who == 1:
                lf_turn = cur["F5A2"]
        if a == "719A" and c["to"] > c["from"] and who == 1:
            lf = cur["F5A2"] if at == "refill" else lf_turn
            out.append((cur["F59F"] & 63, cur["F5A0"] & 31, lf & 63, c["to"]))
        cur[a] = c["to"]
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("runs", nargs="+")
    ap.add_argument("--speed", type=int, required=True)
    ap.add_argument("--load", type=int, required=True)
    ap.add_argument("--at", default="refill", choices=["refill", "turn"])
    ap.add_argument("-v", action="store_true")
    a = ap.parse_args()
    hows = WOUNDS + [w + "-clamped" for w in WOUNDS]
    fits = {h: 0 for h in hows}
    seen = {}
    total = 0
    for path in a.runs:
        for line in open(path, encoding="utf-8"):
            v = json.loads(line)
            for con, strength, lf, m in readings(v, a.at):
                total += 1
                key = (con, strength, lf, m)
                seen[key] = seen.get(key, 0) + 1
                for h in hows:
                    fits[h] += moves(a.speed, strength, con - lf, a.load, h) == m
    print("Con Str LF wound moves count | predicted by " + " ".join(hows))
    for (con, strength, lf, m), n in sorted(seen.items()):
        pred = [moves(a.speed, strength, con - lf, a.load, h) for h in hows]
        print(f"{con:3} {strength:3} {lf:3} {con - lf:5} {m:5} {n:5} | " + " ".join(f"{p:>{len(h)}}" for p, h in zip(pred, hows)))
    print(f"{total} turn starts; fits: " + ", ".join(f"{h} {fits[h]}" for h in hows))


if __name__ == "__main__":
    main()
