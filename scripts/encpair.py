#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""The two percentiles of a two-record map step, run by run (docs/capture.md
10.36, item 8): from a `two` sweep (10.20, `run mem --set-random-state
--draws`, WOLF and HARE both on the grass), every draw of the first grass
step, checked against the generator alone.

A draw is stored twice: a = previous state XOR the raster line's low byte,
then the roll b = a x 69 + 131 mod 256 (docs/research/random-rolls.md). The
run's random state was set to its index, so the first a must be
index XOR line (the logged line, or the one before it: the harness reads
the raster just after the program does). Each roll is raw // 2, drawn again at 200 or more.

Prints the 2 x 2 counts, the raster lines the draws came on, and how many
runs' pairs follow from the state and the lines alone; with -v a line per
run (state, raws, lines, WOLF hit, HARE hit).

    python scripts/encpair.py RUNS.jsonl [--chance 60] [-v]
"""
import argparse
import json
from collections import Counter

BURST = 100000


def gen(a):
    return (a * 69 + 131) & 0xFF


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("runs")
    ap.add_argument("--chance", type=int, default=60)
    ap.add_argument("-v", action="store_true")
    a = ap.parse_args()
    counts, lines, follows, n = Counter(), Counter(), 0, 0
    for row in open(a.runs):
        v = json.loads(row)
        n += 1
        step = min(c["t"] for c in v["changes"] if c["addr"] in ("F5AC", "F5AD"))
        d = [x for x in v["draws"] if x["t"] > step]
        burst = [d[0]] if d else []
        for x in d[1:]:
            if x["t"] - burst[-1]["t"] > BURST:
                break
            burst.append(x)
        pairs = list(zip(burst[0::2], burst[1::2]))
        state, ok, rolls = v["state"], True, []
        for x, y in pairs:
            # The logged line is read just after the program's read: it may
            # be one line on.
            if (x["raw"] ^ state) not in (x["line"] & 0xFF, (x["line"] - 1) & 0xFF) \
                    or y["raw"] != gen(x["raw"]):
                ok = False
            state = y["raw"]
            rolls.append(y["raw"])
        follows += ok
        # Two percentiles in record order, each skipping raws of 200 up.
        kept = [r for r in rolls if r < 200]
        wolf = len(kept) > 0 and kept[0] // 2 < a.chance
        hare = len(kept) > 1 and kept[1] // 2 < a.chance
        counts[(wolf, hare)] += 1
        lines[tuple(x["line"] for x, _ in pairs)] += 1
        if a.v:
            print(v["state"], rolls, [x["line"] for x, _ in pairs], int(wolf), int(hare), "" if ok else "NOT the generator")
    print(f"{n} runs, {follows} with every draw the generator's from the run's state and the raster lines")
    print("none", counts[(False, False)], "WOLF alone", counts[(True, False)],
          "HARE alone", counts[(False, True)], "both", counts[(True, True)])
    print("raster lines of the step's draws:", dict(lines.most_common(12)))


if __name__ == "__main__":
    main()
