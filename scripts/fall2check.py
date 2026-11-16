#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Checks two random creatures falling away at once (creatures 2.4;
docs/capture.md 10.41 item 1), roll by roll, from `acs-capture run mem
--draws` runs of rr4's `fall` snapshot with a second slot marked random.

The scenario (capture.md 10.14.3): residents of ours in slots 0..6 of
region 1 (Speed 0) and the random creature in slot 7, all in room 0; the
player alone in the side room. The runs set the random mark (the top bit
of record +6) on slot 0 as well, so two random creatures stand in an empty
room. Each draw stores the state twice (state XOR line, then the raw
number); the raw is the second.

At each round's end (the round counter going up) the rule predicts: the
player's last idle move's healing roll (drawn again from 252 up), a roll
of 0..15 (raw / 16) that picks a slot; if the slot holds a creature, a
percentile (raw / 2, drawn again from 200 up) for each slot carrying the
random mark, in slot order, the mark staying on a removed creature's slot;
each live one under 20 is removed (no player in its room). Then the side
room's random-creature roll for the next round (a percentile).

`--mode` picks the reading: `slot` (the default, as above), `reverse` (the
marked slots high to low), `one` (one percentile for all, every random
creature going under 20), `alive` (a percentile only for a marked slot
still alive, the engine's choice).

    python scripts/fall2check.py RUNS.jsonl [--slots 0,7] [--mode slot|reverse|one|alive] [-v]
"""
import argparse
import json

ROUND = "F65B"
GAP = 300_000  # draws closer than this belong to one burst (a removal pauses the draws some 200,000)
SLOTS = 8  # slots 0..7 in use at the start


def con_addr(slot):
    return f"{0xE1A3 + 37 * slot:04X}"


def pairs(draws):
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
    """The watch's changes of one byte without one-sample flickers."""
    cs = [c for c in changes if c["addr"] == addr]
    keep, i = [], 0
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
    while i < len(seq) and seq[i] >= 200:
        i += 1
    if i >= len(seq):
        return i, None
    return i + 1, seq[i] // 2


def check(run, marked, mode):
    raws = pairs(run["draws"])
    bs = bursts(raws)
    # The round counter goes up by one at each round end; banked reads show
    # other values for a sample, sometimes across the real change.
    ends, cur = [], dict(zip(run["watch"], run["start"]))[ROUND]
    for c in run["changes"]:
        if c["addr"] == ROUND and c["to"] == cur + 1:
            ends.append(c["t"])
            cur += 1
    seen = {}
    for s in marked:
        z = [c["t"] for c in steady_changes(run["changes"], con_addr(s)) if c["to"] == 0]
        seen[s] = z[0] if z else None
    alive = {s: True for s in range(SLOTS)}
    pred = {s: None for s in marked}
    res = {"rounds": 0, "wrong": [], "both": 0, "log": []}
    for te in ends:
        b = min(bs, key=lambda b: min(abs(t - te) for t, _ in b))
        seq = [raw for _, raw in b]
        i = 0
        while i < len(seq) and seq[i] >= 252:
            i += 1
        i += 1
        if i >= len(seq):
            res["wrong"].append(f"round end at {te}: no slot roll ({seq})")
            continue
        slot = seq[i] // 16
        i += 1
        hit = slot < SLOTS and alive[slot]
        gone, ps = [], []
        if hit:
            if mode == "one":
                i, p = percentile(seq, i)
                ps.append(p)
                if p is not None and p < 20:
                    gone = [s for s in marked if alive[s]]
            else:
                order = sorted(marked, reverse=(mode == "reverse"))
                for s in order:
                    if mode == "alive" and not alive[s]:
                        continue
                    i, p = percentile(seq, i)
                    ps.append(p)
                    if p is not None and p < 20 and alive[s]:
                        gone.append(s)
        i, _ = percentile(seq, i)  # the side room's roll
        res["rounds"] += 1
        res["log"].append(f"end {te}: slot {slot} {'hit' if hit else 'free'} percentiles {ps} gone {gone}")
        if i != len(seq):
            res["wrong"].append(f"round end at {te}: {len(seq)} draws, predicted {i} ({seq})")
        for s in gone:
            alive[s] = False
            pred[s] = te
        if len(gone) == 2:
            res["both"] += 1
    for s in marked:
        p, o = pred[s], seen[s]
        if (p is None) != (o is None) or (p is not None and abs(p - o) > 400_000):
            res["wrong"].append(f"slot {s}: removal predicted {p}, seen {o}")
    res["seen"] = seen
    return res


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("runs")
    ap.add_argument("--slots", default="0,7", help="the slots carrying the random mark")
    ap.add_argument("--mode", choices=["slot", "reverse", "one", "alive"], default="slot")
    ap.add_argument("-v", action="store_true")
    a = ap.parse_args()
    marked = [int(s) for s in a.slots.split(",")]
    runs = [json.loads(line) for line in open(a.runs, encoding="utf-8")]
    bad = rounds = both = 0
    removed = {s: 0 for s in marked}
    one_round = 0
    for r in runs:
        res = check(r, marked, a.mode)
        rounds += res["rounds"]
        both += res["both"]
        for s in marked:
            removed[s] += res["seen"][s] is not None
        ts = [res["seen"][s] for s in marked]
        if all(t is not None for t in ts) and max(ts) - min(ts) < 400_000:
            one_round += 1
        if res["wrong"]:
            bad += 1
        if (res["wrong"] and bad <= 10) or a.v:
            print(f"run {r['run']}:", "; ".join(res["wrong"]) or "ok")
            if a.v or res["wrong"]:
                print("   " + "\n   ".join(res["log"]))
    print(f"{len(runs)} runs, {rounds} round ends, removals seen {removed}, both in one round seen {one_round} "
          f"(predicted {both}), {bad} runs wrong")


if __name__ == "__main__":
    main()
