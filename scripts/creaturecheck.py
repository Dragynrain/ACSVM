#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Checks creatures' goals and steps in a room against the clean-room
researcher's rule (random-rolls third pass; docs/capture.md 10.14), roll by
roll, from `acs-capture run mem --draws` runs that watch each creature's
square.

Each draw stores the state twice (state XOR line, then the raw number), so a
draw is a pair of logged stores; the raw is the second. From the snapshot the
draws go: the player's last idle move (a roll of 0..full-1), then each
creature in slot order: the top-of-turn die, the smartness roll (friends and
enemies with Wisdom 6 to 14), then per step one roll of 0..2 for each open
square beside it, south, north, east, west. A step goes to the open
neighbour with the lowest 3 x steps to the goal + 5 x the square's cost +
4 x its roll (ties in that order).

    python scripts/creaturecheck.py one RUNS.jsonl
    python scripts/creaturecheck.py follow RUNS.jsonl
"""
import argparse
import json

# Our scenarios (capture.md 10.10): the room's inside squares, the player,
# the exits, and the creatures in slot order: (start, Speed, type, Wisdom).
SCENARIOS = {
    "one": {
        "inside": (1, 13, 1, 8),
        "player": (4, 4),
        "player_moves": 6,
        "exits": [],
        "creatures": [((12, 2), 3, "slinker friend", 13), ((2, 7), 5, "slinker friend", 13),
                      ((12, 7), 4, "slinker neutral", 13)],
    },
    "follow": {
        "inside": (1, 13, 1, 8),
        "player": (4, 4),
        "player_moves": 12,
        "exits": [(7, 4)],
        "creatures": [((10, 2), 6, "slinker friend", 13), ((12, 7), 4, "slinker neutral", 13)],
    },
}
# Ranks (random-rolls findings 17.2) for our types: the player, an exit.
RANK_PLAYER = {"slinker friend": 26}
RANK_EXIT = {"slinker friend": 32, "slinker neutral": 32}
EXIT_COST = 1  # a slinker's
FLOOR, STOOD = 2, 4


def roll(raw, n):
    r = raw // (256 // n)
    return r if r < n else None


class Draws:
    def __init__(self, stores):
        # Pair the stores: (s ^ line), then raw = that x 69 + 131.
        self.raws = []
        i = 0
        while i + 1 < len(stores):
            a, b = stores[i], stores[i + 1]
            if (a * 69 + 131) & 0xFF == b:
                self.raws.append(b)
                i += 2
            else:
                i += 1
        self.i = 0

    def take(self, n):
        """A roll of 0..n-1, drawing again while out of range."""
        while True:
            if self.i >= len(self.raws):
                raise IndexError("ran out of draws")
            r = roll(self.raws[self.i], n)
            self.i += 1
            if r is not None:
                return r


def dist(a, b):
    return abs(a[0] - b[0]) + abs(a[1] - b[1])


def simulate(sc, draws):
    x0, x1, y0, y1 = sc["inside"]
    player = sc["player"]
    at = [c[0] for c in sc["creatures"]]
    paths = []
    draws.take(sc["player_moves"])  # the player's last idle move
    for k, (start, speed, kind, wisdom) in enumerate(sc["creatures"]):
        draws.take(6)  # the top-of-turn die
        if "friend" in kind or "enemy" in kind:
            if 6 <= wisdom <= 14:
                draws.take(10)  # smartness
        me = at[k]
        # The goal: rank + steps from the turn's start; under 200 or none.
        cands = []
        if kind in RANK_PLAYER:
            cands.append((RANK_PLAYER[kind] + dist(me, player), 0, player))
        for e in sc["exits"]:
            if kind in RANK_EXIT:
                cands.append((RANK_EXIT[kind] + dist(me, e), 1, e))
        cands.sort()
        has_goal = bool(cands) and cands[0][0] < 200
        # With no goal it steers by its own starting square (not a goal:
        # that square costs as one already stood on).
        goal = cands[0][2] if has_goal else me
        stood = {me}
        path = []
        for _ in range(speed):
            others = {player} | {a for j, a in enumerate(at) if j != k}
            best = None
            for d in [(0, 1), (0, -1), (1, 0), (-1, 0)]:  # S, N, E, W
                sq = (me[0] + d[0], me[1] + d[1])
                if not (x0 <= sq[0] <= x1 and y0 <= sq[1] <= y1) or sq in others:
                    continue
                r = draws.take(3)
                if sq in sc["exits"]:
                    cost = EXIT_COST
                elif has_goal and sq == goal:
                    cost = 1
                elif sq in stood:
                    cost = STOOD
                else:
                    cost = FLOOR
                score = 3 * dist(sq, goal) + 5 * cost + 4 * r
                if best is None or score < best[0]:
                    best = (score, sq)
            if best is None:
                break
            me = best[1]
            stood.add(me)
            path.append(me)
            if me in sc["exits"]:
                break  # it takes the door; its turn ends
        at[k] = me
        paths.append(path)
    return paths


def observed(v, sc):
    """Each creature's squares after each step, from its x and y bytes.
    The program also writes passing values into those bytes while it
    works (not squares: off the room), so only inside squares count, each
    when it differs from the last one counted."""
    n = len(sc["creatures"])
    x0, x1, y0, y1 = sc["inside"]
    names = v["watch"]
    pos = [[v["start"][2 * k], v["start"][2 * k + 1]] for k in range(n)]
    last = [tuple(p) for p in pos]
    paths = [[] for _ in range(n)]
    for c in v["changes"]:
        i = names.index(c["addr"])
        if i >= 2 * n:
            continue
        k, axis = divmod(i, 2)
        pos[k][axis] = c["to"]
        sq = tuple(pos[k])
        inside = x0 <= sq[0] <= x1 and y0 <= sq[1] <= y1
        if not inside or sq == last[k]:
            continue
        if paths[k] and paths[k][-1] in sc["exits"]:
            continue  # through the door: the rest is the other room
        last[k] = sq
        paths[k].append(sq)
    return paths


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("scenario", choices=SCENARIOS)
    ap.add_argument("runs")
    a = ap.parse_args()
    sc = SCENARIOS[a.scenario]
    right = wrong = steps = 0
    for text in open(a.runs, encoding="utf-8"):
        v = json.loads(text)
        stores = [d["raw"] for d in v["draws"]]
        try:
            want = simulate(sc, Draws(stores))
        except IndexError as e:
            print(f"run {v['run']}: {e}")
            wrong += 1
            continue
        # This round's turn of each creature: as many squares as predicted
        # (a run may reach the next round).
        got = [g[: len(w)] for g, w in zip(observed(v, sc), want)]
        ok = want == got
        steps += sum(len(p) for p in got)
        if ok:
            right += 1
        else:
            wrong += 1
            print(f"run {v['run']}: predicted {want}\n        seen {got}")
    print(f"{right} runs right, {wrong} wrong; {steps} creature steps seen")


if __name__ == "__main__":
    main()
