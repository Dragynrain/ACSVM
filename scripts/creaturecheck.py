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
    python scripts/creaturecheck.py thief RUNS.jsonl [--smart fighters]
    python scripts/creaturecheck.py fight RUNS.jsonl

`thief` (capture.md 10.14.4) adds things: treasures and gold piles the
creatures may seek (thing ranks), cost rank / 32 to step on and take; the
turn goes on after a take and the goal stays. `--smart` says who takes the
smartness roll: `friend-enemy` (the default, as the first scenarios) or
`fighters` (friends, enemies and every aggressive creature).
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
    "fight": {
        "inside": (1, 13, 1, 8),
        "player": (4, 4),
        "player_moves": 6,
        "exits": [],
        "creatures": [((6, 4), 3, "fighter enemy", 13)],
    },
    "thief": {
        "inside": (1, 13, 1, 8),
        "player": (4, 4),
        "player_moves": 6,
        "exits": [],
        # In the room's order: two treasures, then two gold piles.
        "things": [((12, 2), "treasure"), ((9, 7), "treasure"), ((2, 2), "gold"), ((11, 6), "gold")],
        "creatures": [((8, 3), 5, "slinker thief", 13), ((12, 8), 4, "fighter thief", 13),
                      ((2, 5), 4, "slinker enemy", 13)],
    },
}
# Ranks (random-rolls findings 17.2) for our types: the player, an exit.
RANK_PLAYER = {"slinker friend": 26, "fighter enemy": 0}
RANK_EXIT = {"slinker friend": 32, "slinker neutral": 32}
# An unarmed fighter enemy's turn takes one roll more (capture.md 10.14.6).
EXTRA_ROLL = {"fighter enemy": 10}
# Thing ranks (creatures 6.4): what each type takes and seeks.
RANK_THING = {
    "slinker thief": {"gold": 32, "treasure": 32},
    "fighter thief": {"gold": 32, "treasure": 32},
    "slinker enemy": {"gold": 96},
}
EXIT_COST = 1  # a slinker's
FLOOR, STOOD = 2, 4
# A step onto a thing it takes takes it from the square beside instead,
# and ends the turn (capture.md 10.14.4); set by --take-beside.
TAKE_BESIDE = False


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


def smart_roll(kind, smart):
    if "friend" in kind or "enemy" in kind:
        return True
    return smart == "fighters" and kind.startswith("fighter")


def simulate(sc, draws, smart="friend-enemy"):
    x0, x1, y0, y1 = sc["inside"]
    player = sc["player"]
    at = [c[0] for c in sc["creatures"]]
    paths = []
    things = list(sc.get("things", []))
    taken = []
    draws.take(sc["player_moves"])  # the player's last idle move
    for k, (start, speed, kind, wisdom) in enumerate(sc["creatures"]):
        draws.take(6)  # the top-of-turn die
        if smart_roll(kind, smart):
            if 6 <= wisdom <= 14:
                draws.take(10)  # smartness
        if kind in EXTRA_ROLL:
            # One more roll before the first step, not reported yet (a
            # 0..9 here or a die before the top-of-turn die fit all of
            # capture.md 10.14.6; the combat brief asks).
            draws.take(EXTRA_ROLL[kind])
        me = at[k]
        # The goal: rank + steps from the turn's start; under 200 or none.
        cands = []
        if kind in RANK_PLAYER:
            cands.append((RANK_PLAYER[kind] + dist(me, player), 0, player))
        for e in sc["exits"]:
            if kind in RANK_EXIT:
                cands.append((RANK_EXIT[kind] + dist(me, e), 1, e))
        ranks = RANK_THING.get(kind, {})
        for n, (sq, what) in enumerate(things):
            if what in ranks:
                cands.append((ranks[what] + dist(me, sq), 2 + n, sq))
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
                held = [w for t, w in things if t == sq]
                if held and held[0] not in ranks:
                    continue  # a thing it won't take: closed (creatures 5.2)
                r = draws.take(3)
                if sq in sc["exits"]:
                    cost = EXIT_COST
                elif has_goal and sq == goal:
                    cost = 1
                elif held:
                    cost = ranks[held[0]] // 32
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
            held = [t for t in things if t[0] == best[1]]
            if TAKE_BESIDE and held:
                # It takes the thing from where it stands, and its turn ends.
                for t in held:
                    things.remove(t)
                    taken.append((k, t))
                break
            me = best[1]
            stood.add(me)
            path.append(me)
            for t in [t for t in things if t[0] == me]:
                things.remove(t)
                taken.append((k, t))
            if me in sc["exits"]:
                break  # it takes the door; its turn ends
        at[k] = me
        paths.append(path)
    simulate.taken = taken
    return paths


def observed(v, sc):
    """Each creature's squares after each step, from its x and y bytes.
    The program also writes passing values into those bytes while it
    works (not squares: off the room), so only inside squares count, each
    when it differs from the last one counted."""
    n = len(sc["creatures"])
    x0, x1, y0, y1 = sc["inside"]
    names = v["watch"]
    # Slot k's square: +13 and +14 of its record at $E1A3 + 37 k (section 9).
    addr = {f"{0xE1A3 + 37 * k + 13 + a:04X}": (k, a) for k in range(n) for a in (0, 1)}
    start = dict(zip(names, v["start"]))
    pos = [[start[f"{0xE1A3 + 37 * k + 13:04X}"], start[f"{0xE1A3 + 37 * k + 14:04X}"]] for k in range(n)]
    last = [tuple(p) for p in pos]
    paths = [[] for _ in range(n)]
    for c in v["changes"]:
        if c["addr"] not in addr:
            continue
        k, axis = addr[c["addr"]]
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
    ap.add_argument("--smart", choices=["friend-enemy", "fighters"], default="friend-enemy")
    ap.add_argument("--take-beside", action="store_true")
    a = ap.parse_args()
    global TAKE_BESIDE
    TAKE_BESIDE = a.take_beside
    sc = SCENARIOS[a.scenario]
    right = wrong = steps = 0
    for text in open(a.runs, encoding="utf-8"):
        v = json.loads(text)
        stores = [d["raw"] for d in v["draws"]]
        try:
            want = simulate(sc, Draws(stores), a.smart)
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
