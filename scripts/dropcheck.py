#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Checks where an overburdened creature's drop lands (creatures 8.1, from
the clean-room researcher's fourth combat pass, combat.md pass 4 item 3)
against `acs-capture run mem --draws --ram` runs, draw by draw
(capture.md 10.24).

The rule: after the dropper's top-of-turn die and its smartness checks,
each try is a roll of 0..2 minus 1 (an offset -1, 0 or +1) and a roll of
0..1 for the axis (0 the column, 1 the row). An offset of 0 lands on the
dropper's own square at once; a square on the room's outer ring or
holding an obstacle fails and another pair is drawn; anything else (an
individual, another thing) takes it. No limit on the tries. Gold drops the
same way.

The landing square is read from the visited room's list of things (the
word at `$382A` points at it, `$E585` in our scenarios; the byte at
`$E583` is its length plus 2; entries are the square, column x 16 + row,
the thing and a count, gold (thing 81) six bytes with the amount in its
last two). The dropper's possession or wealth byte going to 0 dates the
drop: it must fall after the last pair's draw and before the next draw.

    python scripts/dropcheck.py RUNS.jsonl --case NAME [--refuse X,Y/X,Y] [-v]
"""
import argparse
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from creaturecheck import Draws  # noqa: E402

SAMPLE = 20000  # instructions between watch reads
GOLD = 81

# The droppers in turn order: name, square, smartness checks, what it
# drops (a thing number or "gold" with the amount) and the byte that goes
# to 0 at the drop. `set` moves the dropper before the run (`--set`).
CASES = {
    # rr16 (the pass 4 inputs): WE (a peaceful friend, Strength 1, Speed 2,
    # a treasure of weight 400, thing 50); the obstacle on 5,4.
    "drop-wall": {"droppers": [("WE", (6, 4), 1, 50, "E1C4")], "obstacles": {(5, 4)}},
    "drop-corner": {"droppers": [("WE", (13, 8), 1, 50, "E1C4")], "obstacles": set()},
    # drop-corner's snapshot with WE set onto 5,4, east of T on 4,4.
    "drop-beside": {"droppers": [("WE", (5, 4), 1, 50, "E1C4")], "obstacles": set()},
    # rr20: WE on 6,4 with treasures of ours on 7,4, 6,5 and 6,3.
    "drop-thing": {"droppers": [("WE", (6, 4), 1, 50, "E1C4")], "obstacles": set()},
    # rr28 (capture.md 10.32): WE on 6,4 with fixed things of ours beside
    # it: a custom obstacle on 5,4, the side room's portal on 7,4, a custom
    # space on 6,3 and a space on 6,5. Which of them refuse a drop is
    # what the run tells: `--refuse` names the squares taken to refuse.
    # The side room moves the room's list: its length byte at $E58B, the
    # entries from $E58D.
    "drop-kinds": {"droppers": [("WE", (6, 4), 1, 50, "E1C4")], "obstacles": set(), "list": 0xE58B},
    # rr33 (capture.md 10.37): WE on 6,4 with the region's store on 5,4
    # (`--refuse 5,4` takes the store to refuse).
    "drop-store": {"droppers": [("WE", (6, 4), 1, 50, "E1C4")], "obstacles": set()},
    # rr5's burden (capture.md 10.14.5): WA 300 gold on 10,2, WB 2,000 on
    # 10,7, each dropping its gold on its first turn.
    "burden": {"droppers": [("WA", (10, 2), 1, ("gold", 300), "E1B5"), ("WB", (10, 7), 1, ("gold", 2000), "E1DA")],
               "obstacles": set()},
}


def ring(sq):
    return sq[0] in (0, 14) or sq[1] in (0, 9)


def draw_times(stores):
    out, i = [], 0
    while i + 1 < len(stores):
        a, b = stores[i]["raw"], stores[i + 1]["raw"]
        if (a * 69 + 131) & 0xFF == b:
            out.append(stores[i + 1]["t"])
            i += 2
        else:
            i += 1
    return out


def things(vals, base=0xE583):
    """The room's list of things from the watched bytes (E583, E585 on, or
    the case's `list` base)."""
    n = vals[f"{base:04X}"] - 2
    raw = [vals.get(f"{base + 2 + k:04X}") for k in range(n)]
    out, k = [], 0
    while k + 2 < len(raw) and raw[k] is not None:
        sq, th = raw[k], raw[k + 1]
        if th == GOLD:
            if k + 5 >= len(raw) or raw[k + 5] is None:
                out.append(((sq >> 4, sq & 15), th, None))
                break
            out.append(((sq >> 4, sq & 15), th, raw[k + 4] | raw[k + 5] << 8))
            k += 6
        else:
            out.append(((sq >> 4, sq & 15), th, raw[k + 2]))
            k += 3
    return out


def check(v, case, verbose, refuse=frozenset()):
    c = dict(CASES[case])
    c["obstacles"] = c["obstacles"] | refuse
    d = Draws([x["raw"] for x in v["draws"]])
    times = draw_times(v["draws"])
    start = dict(zip(v["watch"], v["start"]))
    end = dict(start)
    gone = {}
    for ch in v["changes"]:
        end[ch["addr"]] = ch["to"]
        if ch["to"] == 0 and ch["addr"] not in gone:
            gone[ch["addr"]] = ch["t"]
    before = things(start, c.get("list", 0xE583))
    after = things(end, c.get("list", 0xE583))
    bad, notes = [], []
    tries = most = 0
    fails = {"ring": 0, "obstacle": 0}
    try:
        d.take(6)  # T's last idle move
        for name, at, checks, what, byte in c["droppers"]:
            d.take(6)  # the top-of-turn die
            for _ in range(checks):
                d.take(10)
            n = 0
            while True:
                off = d.take(3) - 1
                axis = d.take(2)
                n += 1
                if off == 0:
                    sq = at
                    break
                sq = (at[0] + off, at[1]) if axis == 0 else (at[0], at[1] + off)
                if ring(sq):
                    fails["ring"] += 1
                    continue
                if sq in c["obstacles"]:
                    fails["obstacle"] += 1
                    continue
                break
            tries += n
            most = max(most, n)
            last = d.i - 1
            notes.append(f"{name} {n} tries -> {sq}")
            fails[sq] = fails.get(sq, 0) + 1
            # The drop's moment: after the last pair, before the next draw.
            t = gone.get(byte)
            if t is None:
                bad.append(f"{name}: {byte} never went to 0")
            elif not (times[last] <= t and (d.i >= len(times) or t - SAMPLE < times[d.i])):
                bad.append(f"{name}: dropped at {t}, the last pair at {times[last]}, the next draw at "
                           f"{times[d.i] if d.i < len(times) else None}")
            if isinstance(what, tuple):
                want = (sq, GOLD, what[1])
            else:
                want = (sq, what, 1)
            if want not in after or after.count(want) <= before.count(want):
                bad.append(f"{name}: predicted {want}, the list has {after}")
        d.take(16)  # the round's end: the slot roll
        d.take(100)  # and its percentile
    except IndexError:
        bad.append("ran out of draws")
    ok = not bad
    if verbose or not ok:
        print(f"run {v['run']}: {'ok ' if ok else 'BAD'} {'; '.join(notes)}")
        for b in bad:
            print("   ", b)
    return ok, tries, most, fails


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("runs")
    ap.add_argument("--case", required=True, choices=sorted(CASES))
    ap.add_argument("--refuse", default="", help="more squares that refuse a drop, X,Y joined by /")
    ap.add_argument("-v", action="store_true")
    a = ap.parse_args()
    refuse = frozenset(tuple(int(n) for n in sq.split(",")) for sq in a.refuse.split("/") if sq)
    right = wrong = tries = most = 0
    seen = {}
    for text in open(a.runs, encoding="utf-8"):
        ok, n, m, fails = check(json.loads(text), a.case, a.v, refuse)
        right += ok
        wrong += not ok
        tries += n
        most = max(most, m)
        for k, m in fails.items():
            seen[k] = seen.get(k, 0) + m
    where = ", ".join(f"{k[0]},{k[1]}: {m}" for k, m in sorted((k, m) for k, m in seen.items() if isinstance(k, tuple)))
    print(f"{a.case}: {right} runs right, {wrong} wrong; {tries} tries (at most {most} in a drop), "
          f"failed on the ring {seen['ring']}, "
          f"on an obstacle {seen['obstacle']}; landed {where}")


if __name__ == "__main__":
    main()
