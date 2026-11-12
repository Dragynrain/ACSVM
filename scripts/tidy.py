"""Mechanical clean-up of ramp-snapped sprite tiles, before hand-finishing.

Usage: python scripts/tidy.py PALETTE.json OUTDIR IMAGE... [--outline outline0]
                              [--dark 0.42] [--sheet SHEET.png] [--show 3]

For each IMAGE (a sprite on transparency, already snapped with palsnap.py):

1. Specks: an opaque pixel with no opaque 4-neighbour is removed.
2. Strays: an opaque pixel whose colour no 4-neighbour shares, while at least
   three of its four neighbours share one colour, takes that colour. Single
   pixels like this are k-centroid noise at 64 px, not detail (eyes are
   usually two pixels, or one pixel framed by the outline colour, which the
   three-neighbour rule leaves alone when the neighbours differ).
3. Outline: a transparent pixel next to an opaque pixel that is not already
   dark (OKLab lightness under --dark) becomes the outline colour, so every
   edge gets the style sheet's 1 px dark warm outline (section 4) outside
   the figure, which keeps its size.

Faces, hands and other detail still need a person: this only does what a
rule can. --sheet writes before and after, scaled --show times.

Run it only on art we generated or drew (docs/design.md, art pipeline).
Needs Pillow and NumPy.
"""
import argparse
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw

sys.path.insert(0, str(Path(__file__).parent))
from palsnap import load_ramps, oklab  # noqa: E402
from sprite64 import BOARD, shown  # noqa: E402

N4 = ((-1, 0), (1, 0), (0, -1), (0, 1))


def shifted(a, dy, dx, fill):
    """a shifted so out[y, x] = a[y + dy, x + dx], fill outside."""
    out = np.full_like(a, fill)
    h, w = a.shape[:2]
    ys, yd = (slice(dy, h), slice(0, h - dy)) if dy >= 0 else (slice(0, h + dy), slice(-dy, h))
    xs, xd = (slice(dx, w), slice(0, w - dx)) if dx >= 0 else (slice(0, w + dx), slice(-dx, w))
    out[yd, xd] = a[ys, xs]
    return out


def tidy(rgba, outline, dark):
    a = rgba.copy()
    op = a[..., 3] > 0
    # 1. specks
    nb = sum(shifted(op, dy, dx, False) for dy, dx in N4)
    specks = op & (nb == 0)
    a[specks, 3] = 0
    op = a[..., 3] > 0
    # 2. strays
    key = (a[..., 0].astype(np.int32) << 16) | (a[..., 1].astype(np.int32) << 8) | a[..., 2]
    key = np.where(op, key, -1)
    nks = [shifted(key, dy, dx, -1) for dy, dx in N4]
    same = sum(k == key for k in nks)
    fix = np.zeros_like(op)
    newk = np.zeros_like(key)
    for i, k in enumerate(nks):
        cnt = sum(k2 == k for k2 in nks)
        hit = op & (same == 0) & (k >= 0) & (cnt >= 3) & ~fix
        newk[hit] = k[hit]
        fix |= hit
    a[fix, 0], a[fix, 1], a[fix, 2] = newk[fix] >> 16, (newk[fix] >> 8) & 255, newk[fix] & 255
    # 3. outline outside light edges
    light = op & (oklab(a[..., :3])[..., 0] >= dark)
    touch = sum(shifted(light, dy, dx, False) for dy, dx in N4) > 0
    add = ~op & touch
    a[add, :3] = outline
    a[add, 3] = 255
    return a, {"specks": int(specks.sum()), "strays": int(fix.sum()), "outline": int(add.sum())}


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("palette", type=Path)
    ap.add_argument("outdir", type=Path)
    ap.add_argument("images", type=Path, nargs="+")
    ap.add_argument("--outline", default="outline0", help="ramp name and step, e.g. outline0")
    ap.add_argument("--dark", type=float, default=0.42, help="OKLab lightness that counts as outline already")
    ap.add_argument("--sheet", type=Path)
    ap.add_argument("--show", type=int, default=3)
    a = ap.parse_args()
    ramps = load_ramps(a.palette)
    ramp = a.outline.rstrip("0123456789")
    outline = np.array(ramps[ramp][int(a.outline[len(ramp):])], np.uint8)
    a.outdir.mkdir(parents=True, exist_ok=True)
    rows = []
    for f in a.images:
        before = np.array(Image.open(f).convert("RGBA"))
        after, n = tidy(before, outline, a.dark)
        Image.fromarray(after).save(a.outdir / f.name)
        rows.append((f.stem, before, after))
        print(f.stem, n["specks"], "specks,", n["strays"], "strays,", n["outline"], "outline px")
    if a.sheet:
        cell, pad, per = before.shape[0] * a.show, 6, 6
        n = len(rows)
        cols = min(per, n)
        sheet = Image.new("RGB", (cols * (2 * cell + 3 * pad), ((n + per - 1) // per) * (cell + 18) + pad), BOARD)
        d = ImageDraw.Draw(sheet)
        for i, (stem, b, af) in enumerate(rows):
            x, y = (i % per) * (2 * cell + 3 * pad), (i // per) * (cell + 18)
            d.text((x + pad, y + 2), stem, fill=(255, 255, 255))
            sheet.paste(shown(b, a.show), (x + pad, y + 16))
            sheet.paste(shown(af, a.show), (x + 2 * pad + cell, y + 16))
        sheet.save(a.sheet)
        print(a.sheet)


if __name__ == "__main__":
    main()
