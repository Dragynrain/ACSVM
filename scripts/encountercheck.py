#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Checks a map-encounter sweep (`acs-capture run mem --set-random-state
--draws --push e:300k` from a rr14 scenario's first turn, docs/capture.md
10.20) against the encounters spec (docs/rules/encounters.md 2 to 5).

Each run logs every draw (the program stores the state twice per draw: the
state XOR the raster line, then the roll; the second is checked against the
generator, docs/research/random-rolls.md) and the watched bytes (capture.md
9): the player's place $F5AF and square $F5AC/$F5AD, the moves left $719A,
area 1's wall picture and size $E7F2/$E7F3, its entrance $E0A2/$E0A3, and
map slots 8 ($E532: alive, +13/+14 square, +16 place) and 9 ($E557..).

For every step on the open map (a change of the square while the place is
0), the burst of draws right after it (up to the first gap of 100k
instructions or the next step; an idle move's or the next turn's draws come
far later) are the step's: one
percentile (raw // 2, drawn again at 200 or more) for each record on the
entered terrain, in record order. The spec predicts the hits; the area
must open after the step exactly when any record hit, with the entrance the square
stepped onto, the player on 12,4, the moves 0 and each hit's copy in the
next slot from 8 on record r's square (2 + r mod 2, 3 + r div 2).

Bytes under the ROMs sometimes read other values for one 20k sample; a
change undone in the next sample is dropped.

    python scripts/encountercheck.py RUNS.jsonl --records 5:60[,5:60..]
        [--patch 13,10,16,12] [--empty 7] [-v]
"""
import argparse
import json

SAMPLE = 20000
BURST = 100000
SLOTS = {8: 0xE532, 9: 0xE557}


def gen(state):
    return (state * 69 + 131) & 0xFF


def rolls(v):
    """(t, raw) of each draw's roll, and how many broke the generator."""
    d = v["draws"]
    out, bad = [], 0
    for a, b in zip(d[0::2], d[1::2]):
        if b["raw"] != gen(a["raw"]):
            bad += 1
        out.append((b["t"], b["raw"]))
    return out, bad


def timeline(v):
    """Per address, the lasting changes [(t, value)], transients dropped."""
    by = {}
    for c in sorted(v["changes"], key=lambda c: c["t"]):
        by.setdefault(int(c["addr"], 16), []).append((c["t"], c["from"], c["to"]))
    out = {}
    for a, cs in by.items():
        keep, i = [], 0
        while i < len(cs):
            t, f, to = cs[i]
            n = cs[i + 1] if i + 1 < len(cs) else None
            if n and n[0] - t <= SAMPLE and n[2] == f:
                i += 2  # a one-sample misread and its undoing
                continue
            keep.append((t, to))
            i += 1
        out[a] = keep
    return out


def value(tl, addr, t, start):
    """The address's value just after time t (its first value is `start`)."""
    v = start
    for ct, to in tl.get(addr, []):
        if ct <= t:
            v = to
    return v


def first_values(v):
    # T starts on the open map at 12,12 (capture.md 10.19).
    start = {0xF5AF: 0, 0xF5AC: 12, 0xF5AD: 12}
    seen = set()
    for c in sorted(v["changes"], key=lambda c: c["t"]):
        a = int(c["addr"], 16)
        if a not in seen:
            seen.add(a)
            start[a] = c["from"]
    return start


def check(v, records, patch, empty, verbose):
    raws, bad = rolls(v)
    tl = timeline(v)
    st = first_values(v)
    errors = []
    if bad:
        errors.append(f"{bad} draws not the generator's")
    x0, y0, x1, y1 = patch

    def terrain(x, y):
        return records_terrain if (x0 <= x <= x1 and y0 <= y <= y1) else 0

    records_terrain = records[0][0] if records else 0
    # Records: ours, then `empty` unused ones on terrain 0 at 0%.
    recs = list(records) + [(0, 0)] * empty
    steps = sorted({t for a in (0xF5AC, 0xF5AD) for t, _ in tl.get(a, [])})
    moves_changes = [t for t, _ in tl.get(0x719A, [])]
    entered = None
    out = []
    for t in steps:
        if value(tl, 0xF5AF, t - 1, st.get(0xF5AF, 0)) != 0:
            break  # already in the area
        x = value(tl, 0xF5AC, t, st.get(0xF5AC, 0))
        y = value(tl, 0xF5AD, t, st.get(0xF5AD, 0))
        if value(tl, 0xF5AF, t, st.get(0xF5AF, 0)) != 0:
            break
        # The step's draws are the burst right after it: they end at the
        # first gap of more than BURST (an idle move's draw, the next turn's
        # or the next step's come far later).
        own = min([m for m in moves_changes if t <= m <= t + 2 * SAMPLE], default=t)
        end = min([m for m in moves_changes if m > own] + [10**12])
        nxt = min([u for u in steps if u > t] + [10**12])
        mine, last = [], t
        for dt, r in raws:
            if dt <= t:
                continue
            if dt >= nxt or dt - last > BURST:
                break
            mine.append(r)
            last = dt
        ter = terrain(x, y)
        hits, i, ok = [], 0, True
        for r, (rt, chance) in enumerate(recs):
            if rt != ter:
                continue
            while i < len(mine) and mine[i] >= 200:
                i += 1
            if i >= len(mine):
                ok = False
                break
            if mine[i] // 2 < chance:
                hits.append(r)
            i += 1
        if not ok:
            errors.append(f"step to {x},{y}: {len(mine)} draws, too few")
            break
        if i != len(mine):
            errors.append(f"step to {x},{y}: {len(mine) - i} draws left over")
        after = end + 3 * SAMPLE
        place = value(tl, 0xF5AF, after, st.get(0xF5AF, 0))
        out.append((x, y, hits, place))
        if hits:
            entered = (x, y, hits, end)
            if place != 1:
                errors.append(f"step to {x},{y}: hits {hits} but place {place}")
            break
        if place != 0:
            errors.append(f"step to {x},{y}: no hit but place {place}")
            break
    if entered:
        x, y, hits, end = entered
        t = end + 3 * SAMPLE
        got = {
            "square": (value(tl, 0xF5AC, t, 0), value(tl, 0xF5AD, t, 0)),
            "moves": value(tl, 0x719A, end, None),
            "entrance": (value(tl, 0xE0A2, t, 41), value(tl, 0xE0A3, t, 41)),
            "size": value(tl, 0xE7F3, t, 0),
        }
        want = {"square": (12, 4), "moves": 0, "entrance": (x, y), "size": 233}
        for k in want:
            if got[k] != want[k]:
                errors.append(f"{k} {got[k]} not {want[k]}")
        for k, r in enumerate(hits):
            base = SLOTS.get(8 + k)
            if base is None:
                continue
            sq = (value(tl, base + 13, t, 0), value(tl, base + 14, t, 0))
            alive = value(tl, base, t, 0) != 0
            place = value(tl, base + 16, t, 0)
            want_sq = (2 + r % 2, 3 + r // 2)
            if not alive or sq != want_sq or place != 1:
                errors.append(f"slot {8 + k}: alive {alive} {sq} place {place}, want record {r} on {want_sq}")
        if len(hits) < len(SLOTS):
            base = SLOTS[8 + len(hits)]
            if value(tl, base, t, 0) != 0:
                errors.append(f"slot {8 + len(hits)} in use without a hit")
    if verbose or errors:
        print(v["run"], out, errors)
    return errors, out, entered


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("runs")
    ap.add_argument("--records", required=True, help="T:C,.. terrain and chance percent, record order")
    ap.add_argument("--patch", default="13,10,16,12", help="the records' terrain on these map cells, else 0")
    ap.add_argument("--empty", type=int, default=7, help="unused records after ours (terrain 0, 0%%)")
    ap.add_argument("-v", action="store_true")
    a = ap.parse_args()
    records = [tuple(int(p) for p in r.split(":")) for r in a.records.split(",")]
    patch = tuple(int(p) for p in a.patch.split(","))
    n = ok = steps = hits = 0
    kinds = {}
    for line in open(a.runs):
        v = json.loads(line)
        errors, out, entered = check(v, records, patch, a.empty, a.v)
        n += 1
        ok += not errors
        steps += len(out)
        if entered:
            hits += 1
            k = tuple(entered[2])
            kinds[k] = kinds.get(k, 0) + 1
    print(f"{ok} of {n} runs as the spec says; {steps} map steps, {hits} encounters; hits by records {kinds}")


if __name__ == "__main__":
    main()
