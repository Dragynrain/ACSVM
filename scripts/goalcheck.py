#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Checks one creature's goal and steps, turn by turn and draw by draw,
against `acs-capture run mem --draws` runs that watch its square
(capture.md 10.39). Creatures 6.2 to 6.5 and 2.7, with the choices the
capture tests as options.

Each turn of the creature (the turn byte `$71FA` equal to `--who`) draws,
inside its own window: `--pre` rolls of 0..9 (smartness checks), then for
each step one roll of 0..2 (a draw of 255 drawn again) for each open
square beside it, south, north, east, west. The goal is the candidate with
the lowest rank + steps from the turn's first square, a tie to the first
given; the step goes to the open square with the lowest 3 x steps to the
goal + 5 x cost + 4 x roll, a tie to the first looked at. Costs: an exit
`--exit-cost` (the slinker's 1 in the spec; `--exit-first` on the turn's
first step), the goal's own square 1 when
open, a square stood on this turn 4, any other 2. Individuals' squares are
closed (a slinker, or a fighter owning no melee weapon). A creature of
Speed 0 takes one step choice and the step is cancelled unless it is onto
an exit (2.7). The moves are halved, rounding up, when the turn starts
beside a square of `--hinder` (2.6).

    python scripts/goalcheck.py RUNS.jsonl --slot 0 --speed 0 --cand exit:7,4:32 --exit-cost 4
    python scripts/goalcheck.py RUNS.jsonl --slot 0 --speed 3 --cand player:4,4:0 --cand ind:12,5:6
        [--rule spec|first|NAME] [--tie first|last] [--player-track] [--pre N] [-v]

Candidates are KIND:X,Y:RANK, KIND `exit`, `player` (the first player,
followed by `$F5AC`/`$F5AD` with `--player-track`, gone once it has stood
on an exit), `ind` (another individual standing still) or `gold` (a pile
it takes from beside: its square costs RANK / 32, 1 as the goal,
`--gold-first` on the turn's first step if given; picking it ends the
turn, never cancelled at Speed 0; seen as a change of the slot's wealth
byte, record +0x12, which the run must watch; capture.md 10.41). `--rule first`
always takes the first candidate, `--rule ind` always the first `ind`
(to compare with the spec's choice), `--rule LABEL` always the candidate
so labelled; `--tie last` sends a tie to the last
candidate given instead. A fourth field, KIND:X,Y:RANK:LABEL, names the
candidate in the goal counts (capture.md 10.50).
"""
import argparse
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from adjcheck import Timeline  # noqa: E402


def raws(draws):
    """The draws' raw numbers with their times: stores pair as (state ^
    line, raw), raw = that x 69 + 131."""
    out = []
    i = 0
    while i + 1 < len(draws):
        a, b = draws[i]["raw"], draws[i + 1]["raw"]
        if (a * 69 + 131) & 0xFF == b:
            out.append((draws[i + 1]["t"], b))
            i += 2
        else:
            i += 1
    return out


def dist(a, b):
    return abs(a[0] - b[0]) + abs(a[1] - b[1])


def sq(s):
    x, y = s.split(",")
    return (int(x), int(y))


class Rolls:
    def __init__(self, rs):
        self.rs, self.i = rs, 0

    def take(self, n):
        while True:
            if self.i >= len(self.rs):
                raise IndexError("out of draws")
            r = self.rs[self.i] // (256 // n)
            self.i += 1
            if r < n:
                return r


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("runs")
    ap.add_argument("--slot", type=int, default=0)
    ap.add_argument("--who", type=int, default=None, help="its turn byte (default 16 + slot)")
    ap.add_argument("--speed", type=int, required=True)
    ap.add_argument("--cand", action="append", default=[])
    ap.add_argument("--exit-cost", type=int, default=1)
    ap.add_argument("--exit-first", type=int, default=None, help="an exit's cost on the turn's first step")
    ap.add_argument("--gold-first", type=int, default=None, help="a gold pile's cost on the turn's first step")
    ap.add_argument("--pre", type=int, default=0)
    ap.add_argument("--rule", default="spec")
    ap.add_argument("--tie", default="first", choices=["first", "last"])
    ap.add_argument("--hinder", default="", help="squares that halve its moves, X,Y/X,Y; 'player' for the player")
    ap.add_argument("--closed", default="", help="more closed squares, X,Y/X,Y")
    ap.add_argument("--inside", default="1,13,1,8")
    ap.add_argument("--player-track", action="store_true")
    ap.add_argument("--turns", type=int, default=99, help="turns a run to check")
    ap.add_argument("-v", action="store_true")
    a = ap.parse_args()
    who = 16 + a.slot if a.who is None else a.who
    x0, x1, y0, y1 = map(int, a.inside.split(","))
    inside = lambda s: x0 <= s[0] <= x1 and y0 <= s[1] <= y1  # noqa: E731
    cands = []
    labels = []
    for c in a.cand:
        kind, at, rank, *label = c.split(":")
        cands.append((kind, sq(at), int(rank)))
        labels.append(label[0] if label else kind)
    exits = [at for k, at, _ in cands if k == "exit"]
    closed = {sq(s) for s in a.closed.split("/") if s}
    hinder = [s for s in a.hinder.split("/") if s]
    ax = f"{0xE1A3 + 37 * a.slot + 13:04X}"
    ay = f"{0xE1A3 + 37 * a.slot + 14:04X}"
    aw = f"{0xE1A3 + 37 * a.slot + 0x12:04X}"
    took = 0
    right = wrong = turns = through = steps = 0
    goals = {}
    for line in open(a.runs, encoding="utf-8"):
        v = json.loads(line)
        r = raws(v["draws"])
        st = dict(zip(v["watch"], v["start"]))
        me = (st[ax], st[ay])
        tpos = [st.get("F5AC"), st.get("F5AD")]
        t_gone = False
        golds = {at for k, at, _ in cands if k == "gold"}
        ch = v["changes"]
        tl = Timeline(v)  # flicker-free values, for the wealth byte
        bounds = [(0, st["71FA"])] + [(c["t"], c["to"]) for c in ch if c["addr"] == "71FA"]
        log, ok, n = [], True, 0
        for k, (t0, w) in enumerate(bounds):
            # The player's square up to this turn.
            if a.player_track:
                for c in ch:
                    if c["t"] < t0 and c["addr"] in ("F5AC", "F5AD") and not c.get("_seen"):
                        c["_seen"] = True
                        tpos[0 if c["addr"] == "F5AC" else 1] = c["to"]
                        if tuple(tpos) in exits:
                            t_gone = True
            if w != who or n >= a.turns:
                continue
            if k + 1 >= len(bounds):
                break  # cut by the run's end
            t1 = bounds[k + 1][0]
            n += 1
            fixed = next((at for kind, at, _ in cands if kind == "player"), None)
            player = None if t_gone else (tuple(tpos) if a.player_track else fixed)
            live, names = [], []
            for (kind, at, rank), name in zip(cands, labels):
                if kind == "player":
                    if a.player_track:
                        if player is None:
                            continue
                        at = player
                if kind == "gold" and at not in golds:
                    continue
                live.append((kind, at, rank))
                names.append(name)
            shut = set(closed) | {at for kind, at, _ in live if kind not in ("exit", "gold")}
            gold_at = {at: rank for kind, at, rank in live if kind == "gold"}
            # The goal.
            if a.rule == "spec":
                tie = 1 if a.tie == "first" else -1
                order = sorted(range(len(live)), key=lambda i: (live[i][2] + dist(me, live[i][1]), tie * i))
                gi = order[0] if live and live[order[0]][2] + dist(me, live[order[0]][1]) < 200 else None
            elif a.rule == "first":
                gi = 0 if live else None
            else:
                gi = next((i for i, c in enumerate(live) if a.rule in (c[0], names[i])), None)
            goal = live[gi][1] if gi is not None else me
            gname = f"{live[gi][0]}@{goal}" if gi is not None else "none"
            moves = a.speed
            hs = [player if h == "player" else sq(h) for h in hinder]
            if any(h is not None and dist(me, h) == 1 for h in hs):
                moves = (moves + 1) // 2
            ro = Rolls([x for t, x in r if t0 <= t < t1])
            pred, cur, stood, fail = [], me, {me}, None
            try:
                for _ in range(a.pre):
                    ro.take(10)
                for step in range(max(moves, 1)):
                    best = None
                    for d in [(0, 1), (0, -1), (1, 0), (-1, 0)]:
                        s = (cur[0] + d[0], cur[1] + d[1])
                        if not inside(s) or s in shut:
                            continue
                        roll = ro.take(3)
                        if s in exits:
                            cost = a.exit_first if step == 0 and a.exit_first is not None else a.exit_cost
                        elif s in gold_at and step == 0 and a.gold_first is not None:
                            cost = a.gold_first
                        elif s in gold_at and not (gi is not None and s == goal):
                            cost = gold_at[s] // 32
                        elif gi is not None and s == goal:
                            cost = 1
                        elif s in stood:
                            cost = 4
                        else:
                            cost = 2
                        score = 3 * dist(s, goal) + 5 * cost + 4 * roll
                        if best is None or score < best[0]:
                            best = (score, s)
                    if best is None:
                        break
                    s = best[1]
                    if s in gold_at:
                        pred.append(("take", s))
                        break
                    if moves == 0 and s not in exits:
                        break  # cancelled (2.7)
                    pred.append(s)
                    stood.add(s)
                    cur = s
                    if s in exits or moves == 0:
                        break
            except IndexError:
                fail = "out of draws"
            # What it did: its squares inside the room, up to an exit.
            obs, pos = [], list(me)
            for c in ch:
                if t0 <= c["t"] < t1 and c["addr"] in (ax, ay):
                    pos[0 if c["addr"] == ax else 1] = c["to"]
                    s = tuple(pos)
                    if inside(s) and s != (obs[-1] if obs else me) and not (obs and obs[-1] in exits):
                        obs.append(s)
            if aw in tl.start and tl.at(aw, t1 - 1) != tl.at(aw, t0):
                here = obs[-1] if obs else me
                pile = next((g for g in sorted(golds) if dist(g, here) == 1), None)
                obs.append(("take", pile))
                golds.discard(pile)
                took += 1
            good = fail is None and obs == pred
            turns += 1
            steps += len(obs)
            gkey = names[gi] if gi is not None else "none"
            goals[gkey] = goals.get(gkey, 0) + 1
            log.append(f"   turn {n} from {me} goal {gname} moves {moves}: predicted {pred} seen {obs}"
                       + ("" if good else f"  << {fail or ''}"))
            ok &= good
            if pred and pred[-1] in exits and good:
                through += 1
                break
            sqs = [o for o in obs if o[0] != "take"]
            if sqs:
                me = sqs[-1]
        right += ok
        wrong += not ok
        if a.v or not ok:
            print(f"run {v['run']}: {'ok' if ok else 'WRONG'}")
            print("\n".join(log))
    print(f"{right} runs right, {wrong} wrong; {turns} turns, {steps} steps, through an exit {through}, gold taken {took}; goals {goals}")


if __name__ == "__main__":
    main()
