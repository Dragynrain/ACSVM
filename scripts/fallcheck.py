#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Checks random creatures falling away (creatures 2.4; docs/capture.md
10.14.3) against the clean-room researcher's rule, roll by roll, from
`acs-capture run mem --draws` runs of the `fall` scenario.

The scenario: residents of ours in slots 0..6 of region 1 (Speed 0), a random
creature (Speed 0) in slot 7 of room 0, and the player alone in the side room
from the snapshot on. The player idles: its last idle move's healing roll
(magic 5.3) comes just before each round's end, in the same burst. Each draw stores the state twice (state XOR line, then
the raw number), so a draw is a pair of logged stores; the raw is the second.

At each round's end (the round counter going up) the rule predicts: a roll of
0..15 (raw / 16) picks a slot; if the slot holds a creature, each random
creature whose room was not on the round's list takes a percentile roll
(raw / 2, drawn again from 200 up) and is removed below 20. Then the side
room's random-creature roll for the next round (a percentile, its chance 0).
The first round end after the snapshot is the round in which the player left
room 0, so the random creature's room was on that round's list.

`--mode` picks the reading: `unlisted` (the percentile roll only for a
random creature whose room was not on the list), `listed` (one for every
random creature, a listed one never removed by it) or `one` (the default:
one percentile roll whenever the slot is in use, random creatures or not;
under 20 every random creature off the list goes) or `empty` (as `one`, but
what counts is that no player stands in the creature's room at the round's
end: the first round end removes too).

    python scripts/fallcheck.py RUNS.jsonl [--mode unlisted|listed|one|empty]
"""
import argparse
import json

RESIDENTS = 7  # slots 0..6
RANDOM_SLOT = 7
CON = "E2A6"  # the random creature's slot: +0 Constitution (0 when free)
ROUND = "F65B"
GAP = 150_000  # draws closer than this belong to one burst


def pairs(draws):
    """The raw numbers with their times: every second store."""
    return [(d["t"], d["raw"]) for d in draws[1::2]]


def bursts(raws):
    out, cur, last = [], [], None
    for t, raw in raws:
        if last is not None and t - last > GAP:
            out.append(cur)
            cur = []
        cur.append((t, raw))
        last = t
    if cur:
        out.append(cur)
    return out


def steady_changes(changes, addr):
    """The watch's changes of one byte without the one-sample flickers the
    monitor sometimes reads (a value and back 20,000 instructions later)."""
    cs = [c for c in changes if c["addr"] == addr]
    keep = []
    i = 0
    while i < len(cs):
        c = cs[i]
        if i + 1 < len(cs) and cs[i + 1]["from"] == c["to"] and cs[i + 1]["to"] == c["from"] \
                and cs[i + 1]["t"] - c["t"] <= 20_000:
            i += 2
            continue
        keep.append(c)
        i += 1
    return keep


def percentile(seq, i):
    """Index after a percentile roll starting at seq[i], and its value."""
    while i < len(seq) and seq[i] >= 200:
        i += 1
    if i >= len(seq):
        return i, None
    return i + 1, seq[i] // 2


def check(run, mode):
    raws = pairs(run["draws"])
    bs = bursts(raws)
    ends = [c["t"] for c in steady_changes(run["changes"], ROUND) if c["to"] == c["from"] + 1]
    removed_at = [c["t"] for c in steady_changes(run["changes"], CON) if c["to"] == 0]
    alive = True
    first = True
    result = {"rounds": 0, "wrong": [], "removed": None, "predicted": None}
    for te in ends:
        # The burst holding the draw nearest the counter's change.
        b = min(bs, key=lambda b: min(abs(t - te) for t, _ in b))
        seq = [raw for _, raw in b]
        listed = first and mode != "empty"
        first = False
        # The player's last idle move (a roll of 0..6, drawn again from 252
        # up) comes just before the round's end.
        i = 0
        while i < len(seq) and seq[i] >= 252:
            i += 1
        i += 1
        if i >= len(seq):
            result["wrong"].append(f"round end at {te}: no slot roll ({seq})")
            continue
        slot = seq[i] // 16
        i += 1
        gone = False
        hit = slot < RESIDENTS + (1 if alive else 0)
        if mode in ("one", "empty"):
            draw = hit
        elif mode == "listed":
            draw = hit and alive
        else:
            draw = hit and alive and not listed
        if draw:
            i, p = percentile(seq, i)
            if p is not None and p < 20 and alive and not listed:
                gone = True
        i, _ = percentile(seq, i)  # the side room's roll
        result["rounds"] += 1
        if i != len(seq):
            result["wrong"].append(f"round end at {te}: {len(seq)} draws, predicted {i} ({seq})")
        if gone:
            alive = False
            result["predicted"] = te
    if removed_at:
        result["removed"] = removed_at[0]
    pred, seen = result["predicted"], result["removed"]
    if (pred is None) != (seen is None) or (pred is not None and abs(pred - seen) > 400_000):
        result["wrong"].append(f"removal predicted {pred}, seen {seen}")
    return result


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("runs")
    ap.add_argument("--mode", choices=["unlisted", "listed", "one", "empty"], default="one")
    a = ap.parse_args()
    runs = [json.loads(line) for line in open(a.runs)]
    bad = removed = rounds = 0
    for r in runs:
        res = check(r, a.mode)
        rounds += res["rounds"]
        removed += res["removed"] is not None
        if res["wrong"]:
            bad += 1
            if bad <= 10:
                print(f"run {r['run']}:", "; ".join(res["wrong"]))
    print(f"{len(runs)} runs, {rounds} round ends, {removed} removals seen, {bad} runs wrong")


if __name__ == "__main__":
    main()
