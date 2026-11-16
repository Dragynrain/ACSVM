#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Checks the player's melee blows and combat 7's smartness roll after
each damaging blow against `acs-capture run mem --draws` runs, roll by
roll (capture.md 10.37; the arrows' rule is 10.35's).

The runs push the stick into a target beside T from a snapshot in T's
turn: slot 0 (a creature holding nothing; its class byte `$E1A7`, Life
Force `$E1A6`, Power `$E1A8`) or, with `--target u`, the second player U
(record `$F5C4`: Life Force `$F5C7`, Power `$F5C9`). T holds our blade.
T's blows are combat 3.1's, 4 x moves / Speed + Dexterity / 8 - 1, from
the moves byte `$719A` at the run's start; each blow is combat 3's, from
the attack's first draw (the blows-left byte `$B799`'s first write).

After each damaging blow the roll of 0..9 is drawn or not by `--roll`:
- `smart`: combat 7 as 10.35 found it for arrows: drawn when the target
  (by its class before any change) is aggressive and not a friend, at
  Wisdom 6..14; for a player (`--target u`) always (`--player-roll`).
- `aggressive`: as `smart` but at any Wisdom.
- `neutral`: only for an aggressive neutral of Wisdom 6..14 still alive
  (the engine's earlier choice for blows).
`--kill-roll no` leaves it out after the blow that kills.

The target's class follows combat 7 after each damaging blow. After a
kill the attack ends and the round's end follows (the slot roll of 0..15
and the room's percentile; nothing else is in play), which must end before
T's next turn's bar fills (`$719A` rising), as `combatcheck.py --case
kill-e5` checks for arrows.

    python scripts/blowcheck.py RUNS.jsonl --cls C --wis W [--roll smart|neutral] [--kill-roll yes|no]
        [--target slot0|u] [--player-roll yes|no] [--dex 31] [--speed 6] [-v]
"""
import argparse
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from combatcheck import SAMPLE, T_START, Out, Sim, arrow_class, banked_times, blade, draw_times, fighter  # noqa: E402
from creaturecheck import Draws  # noqa: E402

TARGETS = {
    # name, square, Life Force, Power, class byte (None for a player)
    "slot0": ("FF", (5, 4), "E1A6", "E1A8", "E1A7"),
    "u": ("U", (4, 4), "F5C7", "F5C9", None),
}


def observed(v, skip):
    """Each watched byte's values in order, with the reads taken while the
    KERNAL ROM is banked in dropped (capture.md 10.14.3): `skip` is a byte
    that never changes in these runs."""
    start = dict(zip(v["watch"], v["start"]))
    banked = {c["t"] for c in v["changes"] if c["addr"] == skip and c["to"] != start[skip]}
    seqs = {k: [val] for k, val in start.items()}
    for c in v["changes"]:
        if c["t"] in banked or c["addr"] == skip:
            continue
        if seqs[c["addr"]][-1] != c["to"]:
            seqs[c["addr"]].append(c["to"])
    return seqs, banked


def check(v, a):
    name, at, lf, pw, cb = TARGETS[a.target]
    start = dict(zip(v["watch"], v["start"]))
    seen, banked = observed(v, a.skip)
    times = draw_times(v["draws"])
    first_write = next((c["t"] for c in v["changes"] if c["addr"] == "B799" and c["t"] not in banked), None)
    if first_write is None:
        return None, "no attack", {}
    d = Draws([x["raw"] for x in v["draws"]])
    d.i = next(j for j, t in enumerate(times) if t >= first_write - SAMPLE)
    t_at = T_START if a.target == "slot0" else (5, 4)
    t = fighter("T", t_at, blade(), None, 0, start["F5A2"], 40)
    t["dex"] = a.dex
    tg = fighter(name, at, None, None, 0, start[lf], a.con)
    tg["power"], tg["wis"] = start[pw], a.wis
    cls = a.cls
    vals = {lf: [tg["life"]], pw: [tg["power"]]}
    if cb:
        vals[cb] = [start[cb]]
    sim = Sim.__new__(Sim)
    sim.d, sim.vals, sim.notes = d, {}, []
    sim.counts = {k: 0 for k in ("to-hit", "counter to-hit", "parry roll", "parries", "damaging", "melee checks",
                                 "armor breaks", "weapon breaks", "gone", "stopped", "armor checks", "rolls")}
    sim.t, sim.ee, sim.ff = t, tg, None

    def push():
        now = [(lf, tg["life"]), (pw, tg["power"])]
        if cb:
            now.append((cb, (start[cb] & 0x8F) | (cls << 4)))
        for k, val in now:
            if vals[k][-1] != val:
                vals[k].append(val)
    sim.record = push

    moves = a.moves if a.moves is not None else start["719A"]
    blows = max(1, 4 * moves // a.speed + a.dex // 8 - 1)
    trace = [f"{moves} moves, {blows} blows"]
    end = used = None
    k0 = d.i
    try:
        while blows > 0 and tg["life"] > 0:
            out, dmg, _ = sim.blow(t, tg, blows, False)
            if out == "hit" and dmg > 0:
                blows -= 2
                killed = tg["life"] <= 0
                if a.target == "u":
                    take = a.player_roll == "yes"
                elif a.roll == "smart":
                    take = (cls & 3) != 3 and bool(cls & 4) and 6 <= a.wis <= 14
                elif a.roll == "aggressive":
                    take = (cls & 3) != 3 and bool(cls & 4)
                else:
                    take = cls == 1 | 4 and 6 <= a.wis <= 14 and not killed
                if killed and a.kill_roll == "no":
                    take = False
                r = d.take(10) if take else None
                if take:
                    sim.counts["rolls"] += 1
                if cb and not killed:
                    cls = arrow_class(cls, a.wis, 9 if r is None else r)
                push()
                trace.append(f"h{dmg}" + ("" if r is None else f"r{r}"))
            else:
                blows -= 1
                trace.append(out[0])
        used = d.i
        if tg["life"] <= 0:
            sim.counts["kills"] = 1
            d.take(16)
            d.take(100)
            end = d.i
    except (IndexError, Out):
        trace.append("out of draws")
    push()
    bad = [f"{k} predicted {p} seen {seen[k]}" for k, p in vals.items() if seen[k][: len(p)] != p]
    if end is None and used is not None and a.count == "yes":
        # The attack's draws: from its first to the next turn's start (the
        # turn byte $71FA changing), with no other draw between.
        turn = next((c["t"] for c in v["changes"] if c["addr"] == "71FA" and c["t"] > first_write
                     and c["t"] not in banked), None)
        if turn is not None:
            # One roll of 0..5 more comes before the turn byte changes
            # (seen in every run; read as the next mover's top-of-turn die).
            try:
                d.i = used
                d.take(6)
                used = d.i
            except IndexError:
                pass
            n = sum(1 for x in times[k0:] if x < turn)
            if n != used - k0:
                bad.append(f"the attack draws {used - k0}, the run {n} before the turn changes at {turn}")
    if end is not None:
        fill = next((c["t"] for c in v["changes"] if c["addr"] == "719A" and c["to"] > 0
                     and c["t"] > first_write), None)
        if fill is None or end > len(times) or times[end - 1] > fill or (
                end < len(times) and times[end] < fill - SAMPLE):
            bad.append(f"after the kill: the round's end ends at draw {end}, the bar fills at {fill}, "
                       f"draws at {times[max(0, end - 3):end + 2]}")
    return bad, " ".join(trace) + f", class {a.cls} to {cls}", sim.counts


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("runs")
    ap.add_argument("--target", default="slot0", choices=sorted(TARGETS))
    ap.add_argument("--cls", type=int, default=3, help="slot 0's class (alignment + 4 if aggressive)")
    ap.add_argument("--wis", type=int, default=10)
    ap.add_argument("--con", type=int, default=40, help="the target's Constitution")
    ap.add_argument("--roll", default="smart", choices=["smart", "neutral", "aggressive"])
    ap.add_argument("--kill-roll", default="yes", choices=["yes", "no"])
    ap.add_argument("--player-roll", default="yes", choices=["yes", "no"])
    ap.add_argument("--count", default="yes", choices=["yes", "no"],
                    help="check the attack's draws against those before the next turn starts")
    ap.add_argument("--dex", type=int, default=31, help="T's Dexterity")
    ap.add_argument("--speed", type=int, default=6, help="T's Speed")
    ap.add_argument("--moves", type=int, default=None, help="T's moves at the push, when $719A is not watched")
    ap.add_argument("--skip", default="E1A3", help="a byte that never changes, to drop banked reads")
    ap.add_argument("-v", action="store_true")
    a = ap.parse_args()
    right = wrong = 0
    totals = {}
    for text in open(a.runs, encoding="utf-8"):
        v = json.loads(text)
        bad, note, counts = check(v, a)
        if bad is None:
            print(f"run {v['run']}: {note}")
            wrong += 1
            continue
        ok = not bad
        right += ok
        wrong += not ok
        for k, c in counts.items():
            totals[k] = totals.get(k, 0) + c
        if a.v or not ok:
            print(f"run {v['run']}: {'ok ' if ok else 'BAD'} {note}")
            for b in bad:
                print("   ", b)
    print(f"{right} right, {wrong} wrong; " + ", ".join(f"{k} {c}" for k, c in totals.items() if c))


if __name__ == "__main__":
    main()
