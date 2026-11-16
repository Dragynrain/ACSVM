#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Checks the adjacency slowdown (creatures 2.6; docs/capture.md 10.41)
turn by turn from `acs-capture run mem` runs that watch the turn byte
`$71FA`, the moves left `$719A` and the squares of the individuals named.

Each individual is `WHO:SPEED`: WHO 1 the first player (its square at
`$F5AC`, `$F5AD`), 16 + s creature slot s (`$E1A3` + 37 s + 13, + 14).
`--foe WHO:W+W..` names the individuals whose standing on one of the 4
squares beside WHO halves its moves. At each turn start (the turn byte
naming WHO) the moves are read once the byte has held still for
`--settle` instructions after the start; the prediction is SPEED, halved
(`--round up`, the spec, or `down`) when a foe stands beside at that
moment. Inside the turn, a fall of the moves byte by more than one in one
sample (not to 0, which an action ending the turn writes) is listed as a jump (with the squares then), so a
slowdown that comes mid-turn shows.

    python scripts/adjcheck.py RUNS.jsonl --ind 1:6 --ind 16:0 --foe 1:16 [--round up|down] [-v]
"""
import argparse
import json


def addr_of(who):
    if who == 1:
        return "F5AC", "F5AD"
    s = who - 16
    return f"{0xE1A3 + 37 * s + 13:04X}", f"{0xE1A3 + 37 * s + 14:04X}"


class Timeline:
    def __init__(self, run):
        self.start = dict(zip(run["watch"], run["start"]))
        self.ch = {}
        for c in run["changes"]:
            self.ch.setdefault(c["addr"], []).append((c["t"], c["from"], c["to"]))
        # Drop one-sample flickers (a read while other memory is banked in,
        # capture.md 10.14.3): a change undone at the next sample.
        for addr, cs in self.ch.items():
            keep, i = [], 0
            while i < len(cs):
                if i + 1 < len(cs) and cs[i + 1][2] == cs[i][1] and cs[i + 1][0] - cs[i][0] <= 20_000:
                    i += 2
                    continue
                keep.append((cs[i][0], cs[i][2]))
                i += 1
            self.ch[addr] = keep

    def at(self, addr, t):
        v = self.start.get(addr)
        for tc, to in self.ch.get(addr, []):
            if tc > t:
                break
            v = to
        return v

    def series(self, addr, t0, t1):
        return [(t, v) for t, v in self.ch.get(addr, []) if t0 <= t < t1]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("runs")
    ap.add_argument("--ind", action="append", default=[])
    ap.add_argument("--foe", action="append", default=[])
    ap.add_argument("--round", choices=["up", "down"], default="up")
    ap.add_argument("--settle", type=int, default=100_000)
    ap.add_argument("-v", action="store_true")
    a = ap.parse_args()
    speed = {int(w): int(s) for w, s in (i.split(":") for i in a.ind)}
    foes = {int(w): [int(x) for x in f.split("+")] for w, f in (i.split(":") for i in a.foe)}
    tally = {}
    jumps = []
    wrong = 0
    for line in open(a.runs, encoding="utf-8"):
        run = json.loads(line)
        tl = Timeline(run)
        turns = [(0, tl.start["71FA"])] + tl.ch.get("71FA", [])
        for k, (t0, who) in enumerate(turns):
            if who not in speed or k + 1 >= len(turns):
                continue
            t1 = turns[k + 1][0]
            sq = lambda w, t: tuple(tl.at(x, t) for x in addr_of(w))  # noqa: E731
            me = sq(who, t0)
            near = [f for f in foes.get(who, []) if None not in sq(f, t0)
                    and abs(sq(f, t0)[0] - me[0]) + abs(sq(f, t0)[1] - me[1]) == 1]
            s = speed[who]
            pred = (s + 1) // 2 if near and a.round == "up" else s // 2 if near else s
            seen = tl.at("719A", t0 + a.settle) if t0 > 0 else None
            if seen is None:
                continue
            key = (who, bool(near))
            good = seen == pred
            tally.setdefault(key, [0, 0, {}])
            tally[key][0 if good else 1] += 1
            tally[key][2][seen] = tally[key][2].get(seen, 0) + 1
            wrong += not good
            if a.v or not good:
                print(f"run {run['run']} t {t0}: {who} on {me} foes beside {near}: moves {seen}, predicted {pred}")
            prev = seen
            for t, v in tl.series("719A", t0 + a.settle, t1):
                # A rise is the next mover's refill (written before the turn
                # byte changes); a fall to 0 is a turn ended by an action.
                if v != 0 and prev - v > 1:
                    jumps.append(f"run {run['run']} t {t}: {who} moves {prev} to {v}, on {sq(who, t)}, "
                                 f"foes {[(f, sq(f, t)) for f in foes.get(who, [])]}")
                prev = v
    for (who, near), (r, w, seen) in sorted(tally.items()):
        print(f"{who} (Speed {speed[who]}) {'beside a foe' if near else 'no foe beside'}: "
              f"{r} right, {w} wrong; moves seen {dict(sorted(seen.items()))}")
    print(f"jumps inside turns: {len(jumps)}")
    for j in jumps[:20] if not a.v else jumps:
        print("  " + j)
    print(f"{wrong} turn starts wrong")


if __name__ == "__main__":
    main()
