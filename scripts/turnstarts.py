#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""A `run mem` timeline by turn (docs/capture.md 10.29).

Reads a runs.jsonl whose watch holds the region (397C), the round counter
(F65B), the current individual (71FA), the moves left (719A), the first
player's square (F5AC, F5AD) and Life Force (F5A2), all section 9 addresses,
and prints for each run one line per turn start (the individual set, or the
moves refilled): the region, the round, who, the moves it starts with and
the square; then each move of that turn (a step when the square changes
with the moves, an idle move otherwise) and the Life Force changes.

With `--draws` runs and `--range N,N..`, each idle move's roll (each draw
is logged as two stores, `draws_of`) is tested against each range n
(`roll_ok`): a roll of raw // (256 // n), drawn again at n or more, heals
on 0 (magic 5.3.1). Printed per turn of the first player (its k-th in the run, with its
starting moves) and range: the idle moves it predicts right (heal or not,
the last draw needing no redraw) out of those with Life Force below
Constitution (`--con`).

    python scripts/turnstarts.py RUNS.jsonl [--range 12,8,6,4 --con 40] [-v]
"""
import argparse
import json


def load(path):
    runs = []
    for text in open(path, encoding="utf-8"):
        v = json.loads(text)
        if v.get("kind") == "mem":
            runs.append(v)
    return sorted(runs, key=lambda v: v["run"])


def draws_of(v):
    """The draws, one per roll: the generator stores the state twice (the
    state XOR the line, then that x 69 + 131, the raw number), and the
    watch logs both; a store that the next one 'finishes' is dropped."""
    d = v.get("draws", [])
    out = []
    i = 0
    while i < len(d):
        if (i + 1 < len(d) and d[i + 1]["t"] - d[i]["t"] <= 4_000
                and d[i + 1]["raw"] == (d[i]["raw"] * 69 + 131) & 0xFF):
            out.append(d[i + 1])
            i += 2
        else:
            out.append(d[i])
            i += 1
    return out


def events(v):
    ev = [(c["t"], 0, c["addr"], c["from"], c["to"]) for c in v["changes"]]
    ev += [(d["t"], 1, "draw", d["raw"], d["line"]) for d in draws_of(v)]
    # Changes are read at the look after the chunk holding a draw, so a
    # draw at the same t comes first.
    return sorted(ev, key=lambda e: (e[0], -e[1]))


def turns(v):
    """Turn records: start state, then moves (kind, t, left, draws, healed)."""
    names = v["watch"]
    cur = dict(zip(names, v["start"]))
    out = []
    turn = None
    draws = []
    heals = []

    def new(t, who, moves, refill=False):
        return {"t": t, "region": cur.get("397C"), "round": cur.get("F65B"), "who": who, "moves": moves,
                "sq": (cur.get("F5AC"), cur.get("F5AD")), "acts": [], "refill": refill}

    if cur.get("71FA"):
        # The run starts inside a turn.
        turn = new(0, cur["71FA"], cur.get("719A"))
        out.append(turn)
    for t, _, a, f, to in events(v):
        if a == "draw":
            draws.append((t, f))
            continue
        cur[a] = to
        if a == "71FA" and to != 0:
            turn = new(t, to, cur.get("719A"))
            out.append(turn)
        elif a == "719A" and to > f:
            # A refill: a new turn of the same individual when 71FA stays
            # (with 71FA 0, the turn starts when it is set).
            if not cur.get("71FA"):
                pass
            elif turn is not None and turn["acts"]:
                turn = new(t, cur.get("71FA"), to, True)
                out.append(turn)
            elif turn is not None:
                turn["moves"] = to
            draws = []
            heals = []
        elif a == "719A" and to < f - 1 and turn is not None:
            # Moves dropped to 0 at once: the turn ended with moves left.
            turn["ended_with"] = f
        elif a == "719A" and to < f and turn is not None:
            # Healing comes after the idle move's draw, before the drop;
            # a heal before that draw is the turn top's.
            last = draws[-1][0] if draws else None
            healed = sum(1 for h in heals if last is not None and h >= last)
            turn["acts"].append({"t": t, "left": to, "draws": [r for _, r in draws],
                                 "lf": cur.get("F5A2"), "sq": (cur.get("F5AC"), cur.get("F5AD")),
                                 "healed": healed, "timed": draws, "heals": heals})
            draws = []
            heals = []
        elif a == "F5A2" and to == f + 1:
            heals.append(t)
    # A refill while the individual of the visit before is still set
    # (a region change) is not a turn: the turn starts when it is set.
    out = [tr for i, tr in enumerate(out)
           if tr["acts"] or "ended_with" in tr or i + 1 == len(out) or out[i + 1]["t"] - tr["t"] > 1_000_000]
    # A move is a step when the square changed within a look or so of the
    # moves' drop (the two are read in the same pass, either order).
    moved = [(c["t"], c["to"]) for c in v["changes"] if c["addr"] in ("F5AC", "F5AD")]
    for tr in out:
        for m in tr["acts"]:
            near = [t for t, _ in moved if abs(t - m["t"]) <= 60_000]
            m["step"] = bool(near)
            if near:
                m["sq"] = square_at(v, max(near))
    return out


def square_at(v, t):
    sq = dict(zip(v["watch"], v["start"]))
    for c in v["changes"]:
        if c["t"] <= t:
            sq[c["addr"]] = c["to"]
    return (sq.get("F5AC"), sq.get("F5AD"))


def roll_ok(m, n):
    """Whether range n explains idle move m. Its roll is drawn in the 300,000
    instructions before the moves drop (a round's end may draw just after
    it, at the drop); for range n the roll is the first draw there under n,
    any before it being redraws. It must heal exactly when it is 0, and no
    draw may follow it within 20,000 (a redraw n would not make)."""
    win = [(t, r) for t, r in m["timed"] if m["t"] - t <= 300_000]
    for i, (t, raw) in enumerate(win):
        r = raw // (256 // n)
        if r >= n:
            continue
        healed = sum(1 for h in m["heals"] if h >= t)
        redrawn = i + 1 < len(win) and win[i + 1][0] - t <= 20_000
        return (r == 0) == bool(healed) and not redrawn
    return False


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("runs")
    ap.add_argument("--range", default="")
    ap.add_argument("--con", type=int, default=0)
    ap.add_argument("-v", action="store_true")
    a = ap.parse_args()
    ranges = [int(x) for x in a.range.split(",") if x]
    # Tallies per turn: T's k-th turn of the run (0 the one it starts in)
    # and its starting moves.
    totals = {}
    for v in load(a.runs):
        print(f"run {v['run']} state {v['state']}")
        k = -1
        for tr in turns(v):
            k += tr["who"] == 1
            idle = [m for m in tr["acts"] if not m.get("step")]
            steps = len(tr["acts"]) - len(idle)
            print(f"  t={tr['t']:>10} region {tr['region']} round {tr['round']} who {tr['who']} "
                  f"moves {tr['moves']} at {tr['sq']}: {steps} steps, {len(idle)} idle"
                  + (f", ended with {tr['ended_with']} left" if "ended_with" in tr else ""))
            for m in tr["acts"]:
                if a.v:
                    kind = "step" if m.get("step") else "idle"
                    print(f"      {m['t']:>10} {kind} left {m['left']} at {m['sq']} draws {m['draws']} "
                          f"lf {m['lf']} +{m['healed']}")
            if ranges and tr["who"] == 1:
                for m in idle:
                    if not m["draws"] or (a.con and m["lf"] - len(m["heals"]) >= a.con):
                        continue
                    for n in ranges:
                        t = totals.setdefault((k, tr["moves"]), {}).setdefault(n, [0, 0])
                        t[1] += 1
                        t[0] += roll_ok(m, n)
    for (k, moves), by in sorted(totals.items()):
        text = ", ".join(f"range {n} {ok} of {all_}" for n, (ok, all_) in by.items())
        print(f"T's turn {k} (moves {moves} at its start), idle moves: {text}")


if __name__ == "__main__":
    main()
