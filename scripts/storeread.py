#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Reads the message lines of C64 screenshots (the harness's 640 x 400
shots) with letter shapes learned from lines whose text is known: the
store captures (docs/capture.md 10.17), where the player's gold line ("X
HAS N TO SPEND.") is set by us and so teaches the digits.

The message area: four lines 16 pixels apart from y = 334, cells 16 pixels
wide from x = 10, the ink the area's second most common colour. A cell is
read as the character whose learned shape it matches exactly. A shape
taught as two characters (this font draws 0 and O alike) is read as the
digit inside a number and the letter elsewhere.

    python scripts/storeread.py learn TABLE.json SHOT.png 'LINE0|LINE1|..'
    python scripts/storeread.py read TABLE.json SHOT.png...
    python scripts/storeread.py table TABLE.json SHOTDIR > offers.jsonl
    python scripts/storeread.py check offers.jsonl V1,V2,..   # stores 5.1

`learn` adds the shapes of the given lines (an empty entry skips a line).
`read` prints each shot's lines, `?` for an unknown shape. `table` reads a
sell sweep's shots (`wWgK.png`: Wisdom W, the K-th good offered) into one
JSON line each: wisdom, k, gold, good, offer (null when unread). `check`
compares a table with docs/rules/stores.md 5.1, the goods' values in the
order offered.
"""
import json
import os
import sys
from collections import Counter

from PIL import Image

X0, Y0, PITCH, ROWS, COLS = 10, 334, 16, 4, 39


def ink(im):
    area = Counter(im.crop((0, Y0, 640, Y0 + PITCH * ROWS)).getdata()).most_common(2)
    return area[1][0]


def cells(im, row, colour):
    """(column, shape) of each non-blank cell of a line."""
    out = []
    for c in range(COLS):
        x, y = X0 + c * PITCH, Y0 + row * PITCH
        bits = "".join(
            "1" if im.getpixel((x + dx, y + dy)) == colour else "0"
            for dy in range(0, 14, 2)
            for dx in range(0, 16, 2)
        )
        if "1" in bits:
            out.append((c, bits))
    return out


def load(path):
    return json.load(open(path)) if os.path.exists(path) else {}


def learn(table, shot, lines):
    im = Image.open(shot).convert("RGB")
    colour = ink(im)
    for row, text in enumerate(lines):
        if not text:
            continue
        cs = cells(im, row, colour)
        chars = text.replace(" ", "")
        if len(cs) != len(chars):
            raise SystemExit(f"{shot} line {row}: {len(cs)} cells for {chars!r}")
        for (_, bits), ch in zip(cs, chars):
            old = table.get(bits, "")
            if ch not in old:
                table[bits] = "".join(sorted(old + ch))


def read(table, shot):
    im = Image.open(shot).convert("RGB")
    colour = ink(im)
    lines = []
    for row in range(ROWS):
        # Each cell's possible characters; a gap of blank cells is a space.
        opts, last = [], None
        for c, bits in cells(im, row, colour):
            if last is not None and c > last + 1:
                opts.extend([" "] * (c - last - 1))
            opts.append(table.get(bits, "?"))
            last = c
        words, cur = [], []
        for o in opts + [" "]:
            if o == " ":
                words.append(settle(cur))
                cur = []
            else:
                cur.append(o)
        lines.append(" ".join(words).rstrip())
    return lines


def settle(cells):
    """A word's characters, a shape taught as two read as the digit when
    the word holds a digit (or nothing else), else as the letter."""
    numeric = any(len(o) == 1 and o.isdigit() for o in cells) or all(
        len(o) > 1 or o in ".," for o in cells
    )
    pick = str.isdigit if numeric else str.isalpha
    return "".join(o if len(o) == 1 else next((c for c in o if pick(c)), o[0]) for o in cells)


def offer(value, wisdom):
    """docs/rules/stores.md 5.1: the store's sell-back offer."""
    third = value // 3
    return value - min(value - third, 1334) + min(third, 666) * wisdom // 25


def main():
    if len(sys.argv) < 4 and sys.argv[1:2] != ["check"]:
        raise SystemExit(__doc__)
    action, path = sys.argv[1], sys.argv[2]
    table = {} if action == "check" else load(path)
    if action == "learn":
        learn(table, sys.argv[3], sys.argv[4].split("|"))
        json.dump(table, open(path, "w"), indent=0, sort_keys=True)
        print(f"{len(table)} shapes: {''.join(sorted(set(''.join(table.values()))))}")
    elif action == "check":
        values = [int(v) for v in sys.argv[3].split(",")]
        rows = [json.loads(line) for line in open(path)]
        bad = [r for r in rows if r["offer"] != offer(values[r["k"] - 1], r["wisdom"])]
        for r in bad:
            print("wrong:", r, "predicted", offer(values[r["k"] - 1], r["wisdom"]))
        print(f"{len(rows) - len(bad)} of {len(rows)} offers right")
    elif action == "table":
        import glob
        import re

        for shot in sorted(glob.glob(os.path.join(sys.argv[3], "w*g*.png"))):
            m = re.search(r"w(\d+)g(\d+)\.png$", shot)
            lines = read(table, shot)
            gold = re.search(r"HAS (\d+) TO SPEND", lines[0])
            good = re.search(r"HAS (\d+) (\w+)", lines[1])
            sale = re.search(r"FOR (\d+)$", lines[2])
            print(json.dumps({
                "wisdom": int(m.group(1)), "k": int(m.group(2)),
                "gold": gold and int(gold.group(1)),
                "good": good and good.group(2).replace("0", "O"),
                "offer": sale and int(sale.group(1)),
            }))
    elif action == "read":
        for shot in sys.argv[3:]:
            print(shot + ": " + " | ".join(read(table, shot)))
    else:
        raise SystemExit(__doc__)


if __name__ == "__main__":
    main()
