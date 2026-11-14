#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Checks that a weapon or armor that leaves its holder joins the visited
region's store stock (docs/rules/stores.md 2.5), run by run, from
`acs-capture run mem --watch` runs (capture.md 10.17 rr12, 10.21 rr17).

    python scripts/stockcheck.py RUNS.jsonl [--thing 41] [--holder E1B7] [-v]

`--holder` is the watched byte that holds the thing's number while it is
held or worn (EE's weapon `$E1B7`, T's armor `$F5B4`; capture.md section
9); the thing's stock bit is the researcher's layout (things 33 to 80 one
bit each from `$E075`, thing 33 the top bit), which must be watched too.

The watched bytes sometimes read other values for one sample and then
return (capture.md 10.17); a value counts only once it has stayed for two
samples. For each run: when the holder byte left the thing (to anything
else) and when the stock bit went on, each the first such lasting change.
A run agrees when both happen in the same sample or the bit one sample
later (the two are written moments apart), or neither happens.
"""
import argparse
import json
import sys

SAMPLE = 20000  # instructions between watch reads


def stock_bit(thing):
    if not 33 <= thing <= 80:
        sys.exit("things 33 to 80 only (one bit each)")
    i = thing - 33
    return 0xE075 + i // 8, 0x80 >> (i % 8)


def lasting(changes, addr, start, ran):
    """The (t, value) series of one byte, its one-sample glitches dropped
    (and a change in the run's last sample, which can't be told from one)."""
    ch = sorted((c["t"], c["to"]) for c in changes if c["addr"] == addr and c["t"] < ran)
    out = []
    v = start
    for k, (t, to) in enumerate(ch):
        nxt = ch[k + 1][0] if k + 1 < len(ch) else None
        if nxt is not None and nxt - t <= SAMPLE:
            continue  # gone again by the next sample
        if to != v:
            out.append((t, to))
            v = to
    return out


def first(series, start, pred):
    v = start
    for t, to in series:
        if not pred(v) and pred(to):
            return t
        v = to
    return None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("runs")
    ap.add_argument("--thing", type=int, default=41)
    ap.add_argument("--holder", default="E1B7")
    ap.add_argument("-v", action="store_true")
    a = ap.parse_args()
    saddr, mask = stock_bit(a.thing)
    skey, hkey = f"{saddr:04X}", a.holder.upper()
    agree = gone = 0
    total = 0
    for line in open(a.runs):
        j = json.loads(line)
        if "changes" not in j:
            continue
        total += 1
        start = {k.upper(): v for k, v in zip(j["watch"], j["start"])}
        if skey not in start or hkey not in start:
            sys.exit(f"run {j['run']}: {skey} and {hkey} must be watched")
        h0, s0 = start[hkey], start[skey]
        if h0 != a.thing:
            print(f"run {j['run']}: the holder byte starts at {h0}, not {a.thing}")
        if s0 & mask:
            print(f"run {j['run']}: the stock bit is already on")
        left = first(lasting(j["changes"], hkey, h0, j["ran"]), h0, lambda v: v != a.thing)
        on = first(lasting(j["changes"], skey, s0, j["ran"]), s0, lambda v: bool(v & mask))
        ok = left == on or (left is not None and on is not None and 0 < on - left <= SAMPLE)
        agree += ok
        gone += left is not None
        if a.v or not ok:
            print(f"run {j['run']:3}: left the holder at {left}, stock bit on at {on}"
                  f"{'' if ok else '  DIFFERS'}")
    print(f"{agree} of {total} runs agree; the thing left its holder in {gone}")
    sys.exit(0 if agree == total else 1)


if __name__ == "__main__":
    main()
