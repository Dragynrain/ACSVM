#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Checks the combat rules (docs/rules/combat.md, from the clean-room
researcher's three combat passes) against `acs-capture run mem --draws`
runs, roll by roll.

Cases (capture.md 10.14.7, 10.15 and 10.16): `armed` (rr6), the second
pass's `parry`, `break`, `once`, `mail`, `mail-magic`, `md1`, `md2`,
`md3-magic`, `bow` and `bow-line` (rr8), and the third pass's
`mail-parry`, `mail-skill`, `mail-heavy`, `mail-once`, `break-many`,
`md3-plain`, `mail-md1`, `bow-diag`, `bow-wall`, `bow-beside`, `bow-break`,
`bow-ready` and `bow-once` (rr9). The room: T (player, Speed 6,
Constitution and Life Force 40, the move-basic player's stats) on 4,4 (4,5
in `bow-beside`); EE (slot 0: an aggressive, brave enemy, Speed 3,
Constitution and Life Force 20) on 6,4 holding our blade, or holding (or
owning) our bow on 9,4, 9,7 or 5,4; FF (bow-line, slot 1: a friend of
Speed 0) on 6,4; an obstacle on 5,4 in `bow-wall`.

From the turn-end snapshot: T's last idle move, then rounds of EE's turn
(Power drift, the top-of-turn die, the two smartness checks of 0..9, its
moves halved rounding up with T beside it, readying, then before each step
the firing check (combat 8.1: a held bow, T in range, T not beside, the
line of fire clear) and otherwise a step by creaturecheck.py's rule with
T's square open at cost 1 while EE owns a melee weapon, closed otherwise,
until the chosen square is T's: the attack), the round's end (the slot
roll, then the room's random-creature percentile; no random creature here)
and T's turn (drift, the die, then its bar waited out: 6 moves, 3 with EE
beside it), until the draws run out.

    python scripts/combatcheck.py RUNS.jsonl [--case NAME] [--reach trunc] [-v]

Compares the predicted values of every watched byte (T's Life Force, Power,
Parry, Armor skill, weapon and armor; EE's Life Force, Power, weapon and
square) with the run's, glitch reads dropped and nothing after a death (the
game resets the records). `--reach trunc` halves the arrow's path toward
the shooter instead of rounding down, to show the runs tell them apart.
"""
import argparse
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from creaturecheck import Draws, dist  # noqa: E402

INSIDE = (1, 13, 1, 8)
T_START = (4, 4)
BLADE, BOW, MAIL = 59, 41, 40
SAMPLE = 20000  # instructions between watch reads


def clamp(v):
    return max(14, min(90, v))


def half(v):
    return v >> 1  # rounds down, negative numbers too


class Out(Exception):
    pass


def blade(**kw):
    w = {"thing": BLADE, "power": 10, "adj": 0, "brk": 0, "magic": False, "once": False}
    w.update(kw)
    return w


MAILS = {
    "mail": {"thing": MAIL, "power": 2, "adj": -30, "brk": 15, "magic": False, "once": False},
    "mail-magic": {"thing": MAIL, "power": 2, "adj": -30, "brk": 0, "magic": True, "once": False},
    "mail-nb": {"thing": MAIL, "power": 2, "adj": -30, "brk": 0, "magic": False, "once": False},
    "mail-heavy": {"thing": MAIL, "power": 12, "adj": -30, "brk": 15, "magic": False, "once": False},
    "mail-once": {"thing": MAIL, "power": 2, "adj": -30, "brk": 0, "magic": False, "once": True},
}


def bow(**kw):
    w = {"thing": BOW, "power": 10, "adj": 0, "brk": 0, "magic": False, "once": False, "range": 7}
    w.update(kw)
    return w


def case(t_weapon=None, t_armor=None, md=0, ee_weapon=None, ee_at=(6, 4), ff=False, **kw):
    c = {"tw": t_weapon, "ta": t_armor, "md": md, "ew": ee_weapon, "eat": ee_at, "ff": ff,
         "t_at": T_START, "t_last": 6, "armor_skill": 0, "ee_dex": 13, "ee_owns": None, "obstacles": set()}
    c.update(kw)
    return c


# Pass 2 (capture.md 10.15, rr8) and pass 3 (10.16, rr9).
CASES = {
    "armed": case(None, None, 0, blade()),
    "parry": case(blade(), None, 0, blade()),
    "break": case(blade(brk=15), None, 0, blade(brk=15)),
    "once": case(None, None, 0, blade(once=True)),
    "mail": case(None, MAILS["mail"], 0, blade()),
    "mail-magic": case(None, MAILS["mail-magic"], 0, blade()),
    "md1": case(None, None, 1, blade()),
    "md2": case(None, None, 2, blade()),
    "md3-magic": case(None, None, 3, blade(magic=True)),
    "bow": case(None, None, 0, bow(), (9, 4)),
    "bow-line": case(None, None, 0, bow(), (9, 4), True),
    "mail-parry": case(blade(), MAILS["mail"], 0, blade()),
    "mail-skill": case(blade(), MAILS["mail-nb"], 0, blade(), armor_skill=20),
    "mail-heavy": case(None, MAILS["mail-heavy"], 0, blade()),
    "mail-once": case(None, MAILS["mail-once"], 0, blade()),
    "break-many": case(blade(brk=15), None, 0, blade(brk=15), ee_dex=31),
    "md3-plain": case(None, None, 3, blade()),
    "mail-md1": case(None, MAILS["mail-nb"], 1, blade()),
    "bow-diag": case(None, None, 0, bow(), (9, 7)),
    "bow-wall": case(None, None, 0, bow(), (9, 4), obstacles={(5, 4)}),
    # T on 4,5 with 1 of 3 moves left (the researcher's pass 3 P3.5).
    "bow-beside": case(None, None, 0, bow(), (5, 4), t_at=(4, 5), t_last=3),
    "bow-break": case(None, None, 0, bow(brk=15), (9, 4)),
    "bow-ready": case(None, None, 0, None, (9, 4), ee_owns=bow()),
    "bow-once": case(None, None, 0, bow(once=True), (9, 4)),
}


def inside(sq):
    return INSIDE[0] <= sq[0] <= INSIDE[1] and INSIDE[2] <= sq[1] <= INSIDE[3]


def beside(a, b):
    return dist(a, b) == 1


def tdiv(a, b):
    """Division dropping the fraction (toward 0)."""
    q = abs(a) // abs(b)
    return q if (a >= 0) == (b > 0) else -q


def line_clear(src, dst, obstacles):
    """Combat 8.1: 19 points, j/20 of the way between the squares' middles,
    in twentieths of a square; none on a wall or obstacle square."""
    for j in range(1, 20):
        sq = ((20 * src[0] + 10 + j * (dst[0] - src[0])) // 20, (20 * src[1] + 10 + j * (dst[1] - src[1])) // 20)
        if not inside(sq) or sq in obstacles:
            return False
    return True


def arrow(src, dst, dev, side, rng, obstacles, mid="floor"):
    """Combat 8.5: the squares the arrow reaches, in order, and whether a
    wall or obstacle stopped it. Sixteenths of a square throughout."""
    d = -dev if side == 1 else dev
    ax, ay = 16 * (dst[0] - src[0]), 16 * (dst[1] - src[1])
    nx, ny = ax + tdiv(ay * d, 100), ay - tdiv(ax * d, 100)
    m = max(abs(nx), abs(ny))
    ex, ey = tdiv(nx * rng * 16, m), tdiv(ny * rng * 16, m)
    p = {0: (16 * src[0] + 8, 16 * src[1] + 8)}
    p[64] = (p[0][0] + ex, p[0][1] + ey)

    def fill(i, j):
        if j - i < 2:
            return
        k = (i + j) // 2
        if mid == "floor":
            p[k] = ((p[i][0] + p[j][0]) // 2, (p[i][1] + p[j][1]) // 2)
        else:
            o = p[0]
            p[k] = tuple(o[n] + tdiv(p[i][n] + p[j][n] - 2 * o[n], 2) for n in (0, 1))
        fill(i, k)
        fill(k, j)

    fill(0, 64)
    out, seen = [], {tuple(src)}
    for k in range(1, 65):
        x, y = p[k]
        sq = (x // 16, y // 16)
        if not (0 <= sq[0] <= INSIDE[1] + 1 and 0 <= sq[1] <= INSIDE[3] + 1):
            return out, False
        ex_, ey_ = x % 16 in (0, 15), y % 16 in (0, 15)
        if not inside(sq) or sq in obstacles:
            if not (ex_ and ey_):
                return out, True
            continue
        if not ex_ and not ey_ and sq not in seen:
            seen.add(sq)
            out.append(sq)
    return out, False


def fighter(name, at, weapon, armor, md, life, con):
    # The move-basic player's stats, which our creatures share.
    return {
        "name": name, "at": at, "life": life, "con": con, "power": 13, "wis": 13, "dex": 13,
        "size": 9, "dodge": 30, "parry": 26, "armor_skill": 0, "melee": 31, "missile": 22,
        "weapon": dict(weapon) if weapon else None, "armor": dict(armor) if armor else None, "md": md,
    }


def wound(f):
    return 30 * (f["con"] - f["life"]) // f["con"] if f["life"] < f["con"] else 0


def term(f):
    a = f["armor"]
    if not a:
        return 0
    return min(a["adj"] + f["armor_skill"], max(a["adj"], 0))


def melee_weapon(f):
    w = f["weapon"]
    return w if w and "range" not in w else None


class Sim:
    def __init__(self, d, case, start, watch, reach):
        c = CASES[case]
        self.c = c
        self.d = d
        self.reach = reach
        self.obstacles = c["obstacles"]
        sv = dict(zip(watch, start))
        self.t = fighter("T", c["t_at"], c["tw"], c["ta"], c["md"], sv.get("F5A2", 40), 40)
        self.t["power"] = sv.get("F5A4", 13)
        self.t["parry"] = sv.get("F5A6", 26)
        self.t["armor_skill"] = sv.get("F5A7", c["armor_skill"])
        self.ee = fighter("EE", c["eat"], c["ew"], None, 0, sv.get("E1A6", 20), 20)
        self.ee["power"] = sv.get("E1A8", 13)
        self.ee["dex"] = c["ee_dex"]
        # What EE owns: the weapon it holds, or one it owns but doesn't hold.
        self.ee["owns"] = [w for w in (self.ee["weapon"], c["ee_owns"] and dict(c["ee_owns"])) if w]
        self.ff = fighter("FF", (6, 4), None, None, 0, 40, 40) if c["ff"] else None
        self.shot_starts = []  # the draw index at which each shot begins
        self.vals = {}  # address -> predicted values in order
        self.notes = []
        self.dead = False
        self.counts = {"to-hit": 0, "counter to-hit": 0, "parry roll": 0, "parries": 0, "damaging": 0,
                       "melee checks": 0, "armor breaks": 0, "weapon breaks": 0, "gone": 0, "stopped": 0,
                       "shots": 0, "dodge rolls": 0, "dodges": 0, "arrow hits": 0,
                       "armor checks": 0, "readied": 0, "steps": 0, "halved turns": 0, "stopped arrows": 0}
        self.record()

    def take(self, n):
        try:
            return self.d.take(n)
        except IndexError:
            raise Out

    def record(self):
        t, ee = self.t, self.ee
        now = {
            "F5A2": t["life"], "F5A4": t["power"], "F5A6": t["parry"], "F5A7": t["armor_skill"],
            "F5B3": t["weapon"]["thing"] if t["weapon"] else 0,
            "F5B4": t["armor"]["thing"] if t["armor"] else 0,
            "E1A6": ee["life"], "E1A8": ee["power"],
            "E1B7": ee["weapon"]["thing"] if ee["weapon"] else 0,
            "E1B0": ee["at"][0], "E1B1": ee["at"][1],
        }
        if self.ff:
            now["E1CB"] = self.ff["life"]
        for k, v in now.items():
            s = self.vals.setdefault(k, [])
            if not s or s[-1] != v:
                s.append(v)

    def note(self, s):
        self.notes.append(s)

    # Section 4 and 4.2.
    def damage(self, a, w, d, heavy_roll=True):
        heavy = self.take(100) < a["dex"]
        p = w["power"]
        build = max(0, min(30, a["dex"] + a["size"] - 6)) - 15
        dmg = build * p // 30 + p
        if heavy:
            dmg += half(p)
        dmg = min(dmg, 3 * p // 2)
        lost_armor = None
        ar = d["armor"]
        if ar and dmg > 0 and not (w["magic"] and not ar["magic"]):
            if not w["magic"] and ar["magic"]:
                dmg = half(dmg + 1)
            dmg -= ar["power"]
            reached = dmg >= 0
            dmg = max(0, dmg)
            if reached and self.take(100) < ar["brk"]:
                lost_armor = True
            if ar["once"]:
                lost_armor = True
        if lost_armor:
            d["armor"] = None
            self.counts["armor breaks"] += 1
            self.note(f"{d['name']}'s armor goes")
        stopped = False
        if dmg > 0:
            md = d["md"]
            if md == 1 and not w["magic"]:
                dmg = half(dmg + 1)
            elif md in (2, 3) and not w["magic"]:
                stopped = True
            elif md == 3 and w["magic"]:
                dmg = half(dmg + 1)
        if stopped:
            self.counts["stopped"] += 1
            self.record()
            return "stopped", 0
        if dmg > 0:
            share = min(dmg * d["power"] // (d["power"] + d["life"]), (d["power"] + 1) // 2)
            d["power"] = max(0, d["power"] - 2 * share)
            d["life"] = max(0, d["life"] - (dmg - share))
            self.counts["damaging"] += 1
        self.record()
        return "hit", dmg

    def learn(self, f, skill):
        if self.take(100) > f[skill] and f[skill] < 127:
            f[skill] += 1
            self.record()

    def learn_armor(self, f):
        self.counts["armor checks"] += 1
        if self.take(100) < clamp(65 - f["armor_skill"]):
            f["armor_skill"] += 1
            self.record()

    def lose(self, f):
        w = f["weapon"]
        f["weapon"] = None
        if "owns" in f:
            f["owns"] = [o for o in f["owns"] if o is not w]

    # 3.3: one blow with `n` blows left; returns (outcome, damage, stop).
    def blow(self, a, d, n, counter):
        w = a["weapon"]
        chance = clamp(w["adj"] + term(a) + a["melee"] + 5 * n - 20 + a["power"] // 4 - wound(a))
        self.counts["counter to-hit" if counter else "to-hit"] += 1
        if self.take(100) >= chance:
            return "miss", 0, False
        self.counts["melee checks"] += 1
        self.learn(a, "melee")
        if term(a) < 0:
            self.learn_armor(a)
        dw = melee_weapon(d)
        pc = 0
        if dw:
            pc = clamp(d["parry"] + half(dw["adj"]) + half(-wound(d)) + term(d) + half(d["power"]) + 15 - d["size"])
        self.counts["parry roll"] += 1
        if self.take(100) < pc:
            self.counts["parries"] += 1
            self.learn(d, "parry")
            if self.take(100) < 30 and term(d) < 0:
                self.learn_armor(d)
            if self.take(100) < w["brk"]:
                self.lose(a)
                self.counts["weapon breaks"] += 1
                self.note(f"{a['name']}'s weapon breaks")
                self.record()
                return "parried", 0, not counter
            return "parried", 0, False
        out, dmg = self.damage(a, w, d)
        if out == "stopped":
            return "stopped", 0, True
        if dmg > 0 and w["once"]:
            self.lose(a)
            self.counts["gone"] += 1
            self.note(f"{a['name']}'s weapon goes")
            self.record()
            return "hit", dmg, not counter
        return "hit", dmg, False

    def attack(self, a, d, left, full):
        dex_term = a["dex"] // 8
        blows = max(1, (4 * left // full if full else 1) + dex_term - 1)
        trace = [blows]
        while blows > 0 and a["life"] > 0 and d["life"] > 0:
            out, dmg, stop = self.blow(a, d, blows, False)
            if out == "hit" and dmg > 0:
                blows -= 2
            elif out == "miss" and melee_weapon(d) and d["life"] > 0:
                cout, cdmg, stop2 = self.blow(d, a, blows, True)
                blows -= 1
                if cout == "hit" and cdmg > 0:
                    blows -= 1
                trace.append(f"m/c{cout[0]}{cdmg or ''}")
                if stop2:
                    break
                continue
            else:
                blows -= 1
            trace.append(f"{out[0]}{dmg or ''}")
            if stop or not melee_weapon(a):
                break
        self.note(f"{a['name']} attacks: {trace}")
        self.check_dead()

    def check_dead(self):
        if self.t["life"] <= 0 or self.ee["life"] <= 0:
            self.note("death")
            self.dead = True
            raise Out

    def heal(self, f):
        if f["life"] < f["con"]:
            f["life"] += 1
            self.record()

    def drift(self, f):
        if f["power"] < f["wis"]:
            f["power"] += 1
        elif f["power"] > f["wis"]:
            f["power"] -= 1
        self.record()

    # P2.5: EE's shots at T.
    def fire(self, a, left):
        w = a["weapon"]
        shots = max(1, a["dex"] // 8) + left // 2
        for i in range(shots):
            after = shots - 1 - i
            acc = a["missile"] + w["adj"] + term(a) + half(half(a["power"]) - wound(a)) - (5 * after - 20)
            spread = clamp(100 - acc)
            self.counts["shots"] += 1
            self.shot_starts.append(self.d.i)
            if self.take(100) < w["brk"]:
                self.lose(a)
                self.counts["weapon breaks"] += 1
                self.note("the bow breaks")
                self.record()
                return
            dev = self.take(spread)
            side = self.take(2)
            squares, stopped = self.reach(a["at"], self.t["at"], dev, side, w["range"], self.obstacles)
            by_sq = {}
            for f in (self.t, self.ff):
                if f:
                    by_sq.setdefault(f["at"], []).append(f)
            hit_any = None
            for f in [f for sq in squares for f in by_sq.get(sq, [])]:
                self.learn(a, "missile")
                dc = clamp(f["dodge"] + 2 * term(f) + half(f["power"]) - wound(f) + 3 * (10 - f["size"]))
                self.counts["dodge rolls"] += 1
                if self.take(100) < dc:
                    self.counts["dodges"] += 1
                    self.learn(f, "dodge")
                    if self.take(100) < 30 and term(f) < 0:
                        self.learn_armor(f)
                    continue
                self.counts["arrow hits"] += 1
                self.damage(a, w, f)
                hit_any = f["name"]
                break
            if stopped and not hit_any:
                self.counts["stopped arrows"] += 1
            sign = "-" if side else "+"
            self.note(f"shot from {a['at']} dev {sign}{dev} of {spread}: {hit_any or ('stopped' if stopped else '-')}")
            self.check_dead()
            if w["once"]:
                self.lose(a)
                self.counts["gone"] += 1
                self.note("the bow goes")
                self.record()
                return

    def run(self):
        t, ee = self.t, self.ee
        try:
            if self.take(self.c["t_last"]) == 0:  # T's last idle move
                self.heal(t)
            while True:
                self.ee_turn()
                if self.ff:
                    self.ff_turn()
                # The round's end: the slot roll, then one percentile
                # (creatures 2.4's when the slot is EE's, else the room's
                # random-creature roll).
                self.take(16)
                self.take(100)
                # T's turn.
                self.drift(t)
                if self.take(6) == 1:
                    self.heal(t)
                # Creatures 2.6: halved, rounding up, with a foe beside.
                full = 3 if beside(ee["at"], t["at"]) else 6
                for _ in range(full):
                    if self.take(full) == 0:
                        self.heal(t)
        except Out:
            pass

    def ff_turn(self):
        # A friend of Speed 0 (fitted on bow-line, capture.md 10.15): drift,
        # the die, two smartness checks of 0..9, a percentile, a roll whose
        # size these runs don't show (taken as 0..2), then a roll of 0..0:
        # the healing roll of its one idle move, which always heals.
        ff = self.ff
        self.drift(ff)
        if self.take(6) == 1:
            self.heal(ff)
        self.take(10)
        self.take(10)
        self.take(100)
        self.take(3)
        if self.take(1) == 0:
            self.heal(ff)

    def can_fire(self, ee):
        """Combat 8.1: holding a missile weapon, the goal in range, no foe
        beside, the line clear."""
        w, t = ee["weapon"], self.t
        if not (w and "range" in w):
            return False
        r = max(abs(t["at"][0] - ee["at"][0]), abs(t["at"][1] - ee["at"][1]))
        return r <= w["range"] and not beside(ee["at"], t["at"]) and line_clear(ee["at"], t["at"], self.obstacles)

    def ee_turn(self):
        ee, t = self.ee, self.t
        self.drift(ee)
        if self.take(6) == 1:
            self.heal(ee)
        self.take(10)
        self.take(10)
        # Creatures 2.6: the turn's moves halved, rounding up, with a foe beside.
        full = 3
        if beside(ee["at"], t["at"]):
            full = (full + 1) // 2
            self.counts["halved turns"] += 1
        moves = full
        # Combat 6.2: readying, one move a change.
        kind_missile = not beside(ee["at"], t["at"])
        want = next((w for w in ee["owns"] if ("range" in w) == kind_missile), None)
        if want and ee["weapon"] is not want:
            ee["weapon"] = want
            moves -= 1
            self.counts["readied"] += 1
            self.record()
        me, stood = ee["at"], {ee["at"]}
        attack = False
        others = {self.ff["at"]} if self.ff else set()
        while moves > 0:
            if self.can_fire(ee):
                self.fire(ee, moves)
                return
            best = None
            for dx, dy in [(0, 1), (0, -1), (1, 0), (-1, 0)]:
                sq = (me[0] + dx, me[1] + dy)
                if not inside(sq) or sq in others or sq in self.obstacles:
                    continue
                if sq == t["at"] and not melee_weapon(ee):
                    continue  # closed (combat 2.3)
                r = self.take(3)
                cost = 1 if sq == t["at"] else (4 if sq in stood else 2)
                score = 3 * dist(sq, t["at"]) + 5 * cost + 4 * r
                if best is None or score < best[0]:
                    best = (score, sq)
            if best is None:
                break
            if best[1] == t["at"]:
                attack = True
                break
            me = best[1]
            stood.add(me)
            moves -= 1
            self.counts["steps"] += 1
            ee["at"] = me
            self.record()
        if attack:
            self.attack(ee, t, moves, full)
        else:
            for _ in range(moves):
                if self.take(full) == 0:
                    self.heal(ee)


def draw_times(stores):
    """The time of each draw, paired as creaturecheck.Draws pairs them."""
    out, i = [], 0
    while i + 1 < len(stores):
        a, b = stores[i]["raw"], stores[i + 1]["raw"]
        if (a * 69 + 131) & 0xFF == b:
            out.append(stores[i + 1]["t"])
            i += 2
        else:
            i += 1
    return out


def banked_times(v):
    con = dict(zip(v["watch"], v["start"])).get("E1A3")
    return {c["t"] for c in v["changes"] if c["addr"] == "E1A3" and c["to"] != con}


def observed(v):
    """Each watched byte's values in order, with the glitch reads dropped:
    the samples at which EE's Constitution byte (which never changes here)
    reads something else are memory banked in (capture.md 10.14.3). The
    sample that reads the room again holds the real values, which may have
    changed meanwhile."""
    start = dict(zip(v["watch"], v["start"]))
    banked = banked_times(v)
    seqs = {k: [val] for k, val in start.items()}
    for c in v["changes"]:
        if c["t"] in banked or c["addr"] == "E1A3":
            continue
        a = c["addr"]
        if seqs[a][-1] != c["to"]:
            seqs[a].append(c["to"])
    return seqs


REACH = {
    # Combat 8.5 (the researcher's pass 3): sixteenths, 64 halving points
    # rounded down. "trunc" rounds the halving toward the shooter instead,
    # to see whether the runs tell the two apart.
    "sixteenths": arrow,
    "trunc": lambda *a: arrow(*a, mid="trunc"),
}


# The player's attack (combat 2.7, 7; capture.md 10.15): T (holding the
# blade, 5 of 6 moves left) pushes east into slot 0 on 5,4, a creature of
# Constitution and Life Force 40 holding nothing, of this class (alignment
# 0 thief .. 3 friend, plus 4 if aggressive) and Wisdom.
PLAYER_CASES = {"player-friend": (3, 13), "player-neutral-set": (1 + 4, 10)}


def player_check(v, case):
    """T's blows from the first draw at the attack (the blows-left byte's
    first write); returns the predicted values of slot 0's Life Force,
    Power and class byte, and a note."""
    cls, wis = PLAYER_CASES[case]
    times = draw_times(v["draws"])
    banked = banked_times(v)
    start = dict(zip(v["watch"], v["start"]))
    first_write = next(c["t"] for c in v["changes"] if c["addr"] == "B799" and c["t"] not in banked)
    k = next(j for j, t in enumerate(times) if t >= first_write - SAMPLE)
    d = Draws([x["raw"] for x in v["draws"]])
    d.i = k
    t = fighter("T", T_START, blade(), None, 0, 40, 40)
    ff = fighter("FF", (5, 4), None, None, 0, start["E1A6"], 40)
    ff["power"], ff["wis"] = start["E1A8"], wis
    byte = (start["E1A7"] & 0x8F) | (cls << 4)
    vals = {"E1A6": [ff["life"]], "E1A8": [ff["power"]], "E1A7": [byte]}
    sim = Sim.__new__(Sim)
    sim.d, sim.vals, sim.notes = d, {}, []
    sim.counts = {k_: 0 for k_ in ("to-hit", "counter to-hit", "parry roll", "parries", "damaging", "melee checks",
                                   "armor breaks", "weapon breaks", "gone", "stopped", "armor checks")}
    sim.t, sim.ee, sim.ff = t, ff, None
    sim.record = lambda: None

    def push():
        for a, val in (("E1A6", ff["life"]), ("E1A8", ff["power"]), ("E1A7", (byte & 0x8F) | (cls << 4))):
            if vals[a][-1] != val:
                vals[a].append(val)

    blows = max(1, 4 * 5 // 6 + t["dex"] // 8 - 1)
    trace = [blows]
    try:
        while blows > 0 and ff["life"] > 0:
            out, dmg, _ = sim.blow(t, ff, blows, False)
            if out == "hit" and dmg > 0:
                blows -= 2
                # Combat 7: one change a damaging blow.
                align, aggressive = cls & 3, cls & 4
                if align == 3:
                    cls = 1 | aggressive
                elif not aggressive and align in (0, 1, 2):
                    cls |= 4
                elif align == 1 and aggressive:
                    smart = wis > 14 or (6 <= wis <= 14 and d.take(10) < wis - 5)
                    if smart:
                        cls = 2 | 4
                push()
            else:
                blows -= 1
            trace.append(f"{out[0]}{dmg or ''}")
    except IndexError:
        trace.append("out of draws")
    return vals, f"T attacks: {trace}, class {cls}", sim.counts


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("runs")
    ap.add_argument("--case", default="armed", choices=sorted(CASES) + sorted(PLAYER_CASES))
    ap.add_argument("--reach", default="sixteenths", choices=sorted(REACH))
    ap.add_argument("-v", action="store_true")
    a = ap.parse_args()
    right = wrong = 0
    totals = {}
    for text in open(a.runs, encoding="utf-8"):
        v = json.loads(text)
        if a.case in PLAYER_CASES:
            pred, note, counts = player_check(v, a.case)
            seen = observed(v)
            # Slot 0's own turn may follow (healing, Power drift).
            bad = [f"{k} predicted {p} seen {seen[k]}" for k, p in pred.items() if seen[k][: len(p)] != p]
            ok = not bad
            right += ok
            wrong += not ok
            for k, c in counts.items():
                totals[k] = totals.get(k, 0) + c
            if a.v or not ok:
                print(f"run {v['run']}: {'ok ' if ok else 'BAD'} {note}; seen {dict((k, seen[k]) for k in pred)}")
                for b in bad:
                    print("   ", b)
            continue
        d = Draws([x["raw"] for x in v["draws"]])
        sim = Sim(d, a.case, v["start"], v["watch"], REACH[a.reach])
        sim.run()
        seen = observed(v)
        bad = []
        # Each shot starts with the bow's break roll, as the shots-left byte
        # ($B799) is written. The byte is read every 20,000 instructions, so
        # the shot's first draw falls within the sample before the change.
        times = draw_times(v["draws"])
        banked = banked_times(v)
        writes = [c["t"] for c in v["changes"] if c["addr"] == "B799" and c["t"] not in banked]
        for k, i in enumerate(sim.shot_starts):
            if k < len(writes) and i < len(times):
                if not (writes[k] - SAMPLE <= times[i] and (i == 0 or times[i - 1] < writes[k])):
                    bad.append(f"shot {k} starts at draw {i} ({times[i]}), the byte changes at {writes[k]}")
        for addr, pred in sim.vals.items():
            if addr not in seen:
                continue
            got = seen[addr]
            n = min(len(got), len(pred))
            # The run may end a write or two before the draws do.
            # After a death the game resets the records (its death screen).
            if got[:n] != pred[:n] or (len(got) > len(pred) and not sim.dead) or len(pred) - len(got) > 2:
                bad.append(f"{addr} predicted {pred} seen {got}")
        ok = not bad
        right += ok
        wrong += not ok
        for k, c in sim.counts.items():
            totals[k] = totals.get(k, 0) + c
        if a.v or not ok:
            print(f"run {v['run']}: {'ok ' if ok else 'BAD'} draws {d.i} of {len(d.raws)}; {'; '.join(sim.notes)}")
            for b in bad:
                print("   ", b)
    print(f"{a.case}: {right} runs right, {wrong} wrong; " + ", ".join(f"{k} {c}" for k, c in totals.items() if c))


if __name__ == "__main__":
    main()
