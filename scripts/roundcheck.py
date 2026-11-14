#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Turn timelines from `acs-capture run mem` runs (docs/capture.md 10.22).

The runs watch the scheduler's bytes (capture.md section 9): the region
visited (397C), the round counter (F65B), the current individual (71FA),
and optionally players' Life Force, alive bytes and places. This rebuilds,
run by run, the turns in order: a turn starts whenever the current
individual changes to a number other than 0 (0 is nobody: between turns,
and while a region loads), so two turns of one individual with nobody
between count as two.

Prints each round's region and list, the death messages answered (and
with what), and checks:
  - no round lists an individual 5 to 15 in a region other than 0;
  - in a room region, every player seen there in the round before and
    after has a turn in it (the "missed round" of 10.10.4), unless it died;
  - each death message comes after the counter went up (the counter
    changed between the last turn and the message).

A new round with the same individual current (one player alone on the
map) counts as a new turn too, on the map only (in a room region the
last mover stays current while the counter goes up), and unless the region changes before the
individual or the counter does again.

Runs watched without `--ram` read a byte under the KERNAL ROM as the ROM's
byte whenever the program has the ROM banked in: `--rom F65B=3` drops
those readings (the round counter's ROM byte is 3).

Usage: python scripts/roundcheck.py RUNS.jsonl [--rom ADDR=V,..] [-v]
"""
import json
import sys

REGION, ROUND, WHO = "397C", "F65B", "71FA"


ROM = {}


def timeline(run):
    """Values of each watched byte as (t, value) steps, from the changes."""
    start = dict(zip(run["watch"], run["start"]))
    steps = {a: [(0, v)] for a, v in start.items()}
    for c in run["changes"]:
        if ROM.get(c["addr"]) == c["to"]:
            continue
        if steps[c["addr"]][-1][1] != c["to"]:
            steps[c["addr"]].append((c["t"], c["to"]))
    return steps


def at(steps, t):
    v = steps[0][1]
    for tt, x in steps:
        if tt > t:
            break
        v = x
    return v


def turns(run):
    s = timeline(run)
    out = []
    starts = sorted({t for t, _ in s[WHO]} | {t for t, _ in s[ROUND]})
    for t in starts:
        who = at(s[WHO], t)
        if who == 0:
            continue
        # A turn start: the individual set at t (from nobody or another).
        real = any(tt == t and w == who for tt, w in s[WHO][1:])
        if out and out[-1]["who"] == who and not real:
            if out[-1]["round"] == at(s[ROUND], t) or at(s[REGION], t) != 0:
                continue
            # The counter went up with the same individual current: a new
            # turn (one player alone on the map) unless the region changes
            # before the individual does (the scheduler moving on).
            nxt = min(
                [tt for tt, w in s[WHO] if tt > t and w != who] + [tt for tt, _ in s[ROUND] if tt > t],
                default=None,
            )
            moved = [tt for tt, _ in s[REGION] if tt > t and (nxt is None or tt < nxt)]
            if moved:
                continue
            # Nor if another individual becomes current before the counter
            # goes up again: the counter went up during the last turn of
            # the round, and the scheduler only then moved on.
            nround = min((tt for tt, _ in s[ROUND] if tt > t), default=None)
            if any(w and w != who and tt > t and (nround is None or tt < nround) for tt, w in s[WHO]):
                continue
        out.append({"t": t, "who": who, "round": at(s[ROUND], t), "region": at(s[REGION], t)})
    return out, s


def rounds(ts):
    out = []
    for x in ts:
        if not out or out[-1]["round"] != x["round"]:
            out.append({"round": x["round"], "region": x["region"], "list": [], "t": x["t"]})
        out[-1]["list"].append(x["who"])
        if x["region"] != out[-1]["region"]:
            out[-1]["region"] = f'{out[-1]["region"]}/{x["region"]}'
    return out


def main():
    argv = sys.argv[1:]
    if "--rom" in argv:
        i = argv.index("--rom")
        for kv in argv[i + 1].split(","):
            a, v = kv.split("=")
            ROM[a.upper()] = int(v)
        del argv[i:i + 2]
    args = [a for a in argv if not a.startswith("-")]
    verbose = "-v" in argv
    bad = 0
    for line in open(args[0]):
        run = json.loads(line)
        ts, s = turns(run)
        rs = rounds(ts)
        print(f'run {run["run"]}: {len(ts)} turns, {len(rs)} rounds, ran {run["ran"]}')
        for r in rs:
            note = ""
            if r["region"] not in (0, "0") and any(5 <= w <= 15 for w in r["list"]):
                note = "  <- individual 5..15 in a room list"
                bad += 1
            print(f'  round {r["round"]:3} region {r["region"]}: {" ".join(map(str, r["list"]))}{note}'
                  + (f'  (t {r["t"]})' if verbose else ""))
        # A player in a room region with a turn in the rounds either side
        # of one it lacks (and alive there: its own Life Force not 0).
        room = [r for r in rs if r["region"] not in (0, "0", 255)]
        for a, b, c in zip(room, room[1:], room[2:]):
            if not (a["round"] + 1 == b["round"] == c["round"] - 1):
                continue
            for p in range(1, 5):
                if p in a["list"] and p in c["list"] and p not in b["list"]:
                    print(f'  MISSED: player {p} has no turn in round {b["round"]}')
                    bad += 1
        for ans in run.get("answered", []):
            t = ans["t"]
            # The last real turn start: the individual changing (not a
            # new round of the same one).
            last = max((tt for tt, w in s[WHO] if w and tt <= t), default=0)
            r_then, r_now = at(s[ROUND], last), at(s[ROUND], t)
            how = "F1" if ans.get("f1") else "button"
            if ans.get("f1"):
                order = "counter up first" if r_now == r_then + 1 else f"counter {r_then} -> {r_now}"
                print(f'  death message at {t} (round {r_now}, region {at(s[REGION], t)}), answered {how}: {order}')
                if r_now != r_then + 1:
                    bad += 1
            elif verbose:
                print(f'  message at {t} answered {how}')
        if verbose:
            for a in run["watch"]:
                if a not in (REGION, ROUND, WHO):
                    print(f'  {a}: ' + " ".join(f"{v}@{t}" for t, v in s[a][:40]))
    print("checks: " + ("all hold" if bad == 0 else f"{bad} failed"))
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
