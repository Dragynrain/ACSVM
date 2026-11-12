"""Makes a texture tile seamless without AI, by image quilting (Efros and
Freeman, SIGGRAPH 2001).

Usage: python scripts/quilt.py OUTDIR IMAGE... [--patch 0.5] [--band 0.12]

1. Roll the image by half: its outer edges are now the old centre lines,
   which wrap, and the old edges meet in a seam cross through the middle.
2. Cover the cross with three patches of the image's own seam-free parts:
   a vertical strip from the image rolled only vertically, a horizontal
   strip from the image rolled only horizontally, and the original centre
   where the two strips cross.
3. Each patch side is cut along the minimum-error path through an overlap
   band (dynamic programming), so the joins run where the images already
   agree, between tree crowns or waves rather than through them.

The art is kept exactly, only rearranged. Features near the edges of the
source can be lost under the patches, so prompt for features spread evenly
across the whole image. Writes OUTDIR/<name>.png and OUTDIR/<name>-check.png
(2 x 2 repeats, original left, result right).

Run it only on art we generated or drew (docs/design.md, art pipeline).
Needs Pillow and NumPy.
"""
import argparse
from pathlib import Path

import numpy as np
from PIL import Image, ImageFilter


def min_path(err):
    """Minimum-cost top-to-bottom path through err (rows x cols): a column per row."""
    h, w = err.shape
    cost = err.copy()
    for y in range(1, h):
        prev = cost[y - 1]
        cost[y] += np.minimum(np.minimum(np.r_[np.inf, prev[:-1]], prev), np.r_[prev[1:], np.inf])
    path = np.empty(h, int)
    path[-1] = int(np.argmin(cost[-1]))
    for y in range(h - 2, -1, -1):
        x = path[y + 1]
        lo, hi = max(0, x - 1), min(w, x + 2)
        path[y] = lo + int(np.argmin(cost[y, lo:hi]))
    return path


def paste_cut(base, src, y0, y1, x0, x1, band, sides):
    """Paste src[y0:y1, x0:x1] over base, each side in `sides` ('l', 'r', 't',
    'b') cut along the minimum-error path through a band of that width."""
    P, B = src[y0:y1, x0:x1], base[y0:y1, x0:x1]
    err = ((P - B) ** 2).sum(2)
    H, W = err.shape
    inside = np.ones((H, W), bool)
    if "l" in sides:
        p = min_path(err[:, :band])
        for y in range(H):
            inside[y, :p[y]] = False
    if "r" in sides:
        p = min_path(err[:, W - band:]) + W - band
        for y in range(H):
            inside[y, p[y] + 1:] = False
    if "t" in sides:
        p = min_path(err[:band, :].T)
        for x in range(W):
            inside[:p[x], x] = False
    if "b" in sides:
        p = min_path(err[H - band:, :].T) + H - band
        for x in range(W):
            inside[p[x] + 1:, x] = False
    m = Image.fromarray((inside * 255).astype(np.uint8)).filter(ImageFilter.GaussianBlur(1.5))
    m = np.asarray(m).astype(float)[..., None] / 255
    out = base.copy()
    out[y0:y1, x0:x1] = P * m + B * (1 - m)
    return out


def quilt(img, patch=0.5, band=0.12):
    """A seamless version of img (PIL image); patch and band are fractions of its size."""
    a = np.asarray(img.convert("RGB")).astype(float)
    h, w = a.shape[:2]
    S = int(min(h, w) * patch)
    bw = int(min(h, w) * band)
    cy, cx = (h - S) // 2, (w - S) // 2
    out = np.roll(a, (h // 2, w // 2), axis=(0, 1))
    out = paste_cut(out, np.roll(a, h // 2, axis=0), 0, h, cx, cx + S, bw, "lr")
    out = paste_cut(out, np.roll(a, w // 2, axis=1), cy, cy + S, 0, w, bw, "tb")
    out = paste_cut(out, a, cy, cy + S, cx, cx + S, bw, "lrtb")
    return Image.fromarray(out.clip(0, 255).astype(np.uint8))


def repeat2(im):
    w, h = im.size
    out = Image.new("RGB", (w * 2, h * 2))
    for x in (0, w):
        for y in (0, h):
            out.paste(im, (x, y))
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("outdir", type=Path)
    ap.add_argument("images", type=Path, nargs="+")
    ap.add_argument("--patch", type=float, default=0.5)
    ap.add_argument("--band", type=float, default=0.12)
    a = ap.parse_args()
    a.outdir.mkdir(parents=True, exist_ok=True)
    for f in a.images:
        im = Image.open(f).convert("RGB")
        res = quilt(im, a.patch, a.band)
        res.save(a.outdir / f"{f.stem}.png")
        w, h = im.size
        check = Image.new("RGB", (w * 2 + 16, h), (96, 96, 104))
        check.paste(repeat2(im).resize((w, h)), (0, 0))
        check.paste(repeat2(res).resize((w, h)), (w + 16, 0))
        check.save(a.outdir / f"{f.stem}-check.png")
        print(f.stem)


if __name__ == "__main__":
    main()
