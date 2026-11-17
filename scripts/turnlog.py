#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Turn-by-turn table of a `run mem` sweep (docs/capture.md 10.44): for
each individual named, at each of its turns, the moves at the start (the
moves byte `$719A` once the turn byte `$71FA` has held still for
`--settle` instructions), its Life Force (record +3, low 6 bits) and
square at the start and at the turn's end, and the moves used.

Individuals are WHO as in adjcheck.py: 1 the first player, 16 + s
creature slot s. Watch `71FA,719A` and the individual's Life Force and
square bytes (`$F5A2`, `$F5AC`, `$F5AD` for the player; `$E1A3` + 37 s
+ 3, + 13, + 14 for slot s).

`--bump WHO@X,Y` counts the bumps at a goal on an obstacle (creatures
5.1): a turn of WHO in which its Life Force falls by `--hurt` (default 5)
at one sample while its square stays the same, standing beside X,Y.
`--attacks WHO` counts falls of the first player's Life Force during
WHO's turns (blows or shots on the player).

    python scripts/turnlog.py RUNS.jsonl --ind 16 [--ind 1] [--bump 16@2,4] [--attacks 16] [-v]
"""
import argparse
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from adjcheck import Timeline, addr_of  # noqa: E402


def lf_of(who):
    return "F5A2" if who == 1 else f"{0xE1A3 + 37 * (who - 16) + 3:04X}"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("runs")
    ap.add_argument("--ind", type=int, action="append", default=[])
    ap.add_argument("--settle", type=int, default=100_000)
    ap.add_argument("--bump", action="append", default=[])
    ap.add_argument("--hurt", type=int, default=5)
    ap.add_argument("--attacks", type=int, action="append", default=[])
    ap.add_argument("-v", action="store_true")
    a = ap.parse_args()
    bumps = {}
    for b in a.bump:
        w, sq = b.split("@")
        bumps[int(w)] = tuple(int(x) for x in sq.split(","))
    inds = set(a.ind) | set(bumps) | set(a.attacks)
    starts = {}
    bump_runs, bump_total, runs_n, bump_turns_beside = {}, 0, 0, 0
    attack_runs, attack_falls = {}, 0
    for line in open(a.runs, encoding="utf-8"):
        run = json.loads(line)
        runs_n += 1
        tl = Timeline(run)
        turns = [(0, tl.start["71FA"])] + tl.ch.get("71FA", [])
        rows = []
        for k, (t0, who) in enumerate(turns):
            if who not in inds or k + 1 >= len(turns) or t0 == 0:
                continue
            t1 = turns[k + 1][0]
            xa, ya = addr_of(who)
            lf = lf_of(who)
            mv = tl.at("719A", t0 + a.settle)
            sq0 = (tl.at(xa, t0 + a.settle), tl.at(ya, t0 + a.settle))
            sq1 = (tl.at(xa, t1 - 1), tl.at(ya, t1 - 1))
            l0, l1 = tl.at(lf, t0 + a.settle), tl.at(lf, t1 - 1)
            note = ""
            if a.v or who in a.ind:
                starts.setdefault(who, {}).setdefault(mv, 0)
                starts[who][mv] += 1
            if who in bumps:
                bx, by = bumps[who]
                n = 0
                prev = tl.at(lf, t0)
                for t, v in tl.series(lf, t0, t1):
                    here = (tl.at(xa, t), tl.at(ya, t))
                    before = (tl.at(xa, t - 20_000), tl.at(ya, t - 20_000))
                    if prev is not None and prev - v == a.hurt and here == before \
                            and None not in here and abs(here[0] - bx) + abs(here[1] - by) == 1:
                        n += 1
                    prev = v
                # A turn with a chance: beside the square with a move left
                # at some sample of the turn.
                for t in [t0 + a.settle] + [t for t, _ in tl.ch.get(xa, []) + tl.ch.get(ya, [])
                                            + tl.ch.get("719A", []) if t0 + a.settle < t < t1]:
                    here = (tl.at(xa, t), tl.at(ya, t))
                    if None not in here and abs(here[0] - bx) + abs(here[1] - by) == 1 \
                            and (tl.at("719A", t) or 0) > 0:
                        bump_turns_beside += 1
                        note += " beside"
                        break
                if n:
                    bump_runs[run["run"]] = bump_runs.get(run["run"], 0) + n
                    bump_total += n
                    note += f" BUMP x{n}"
            if who in a.attacks:
                prev = tl.at("F5A2", t0)
                n = 0
                for t, v in tl.series("F5A2", t0, t1):
                    if prev is not None and v < prev:
                        n += 1
                    prev = v
                if n:
                    attack_runs[run["run"]] = attack_runs.get(run["run"], 0) + n
                    attack_falls += n
                    note += f" T hurt x{n}"
            rows.append(f"  t {t0:>9} {who:>2}: moves {mv}, LF {l0}->{l1}, square {sq0}->{sq1}{note}")
        if a.v:
            print(f"run {run['run']} (state {run.get('state')}):")
            print("\n".join(rows))
    for who, d in sorted(starts.items()):
        print(f"{who}: moves at turn starts {dict(sorted(d.items(), key=lambda kv: (kv[0] is None, kv[0] or 0)))}")
    if bumps:
        print(f"bumps: {bump_total} in {len(bump_runs)} of {runs_n} runs "
              f"(per run {sorted(bump_runs.values())}); turns beside the square with a move left: {bump_turns_beside}")
    if a.attacks:
        print(f"player hurt on those turns: {attack_falls} falls in {len(attack_runs)} of {runs_n} runs")


if __name__ == "__main__":
    main()
