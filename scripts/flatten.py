"""Flattens a generated sprite's background to one pure colour.

Usage: python scripts/flatten.py OUTDIR IMAGE... [--hue 12] [--colour 255,0,255]
                                 [--compare SHEET.png]

Every pixel whose hue is within --hue (0 to 255 scale) of the background's
(the median of the border), that is saturated and not near black, and that
is connected to the image border becomes --colour. That removes the soft
ground shadow (a darker shade of the background) and the background's noise
and gradient, but not background-coloured parts inside the subject, which
the outline cuts off from the border. Used on LoRA training images so the
LoRA does not learn a baked-in shadow (docs/art/pirates-style.md section 10).

Run it only on art we generated or drew (docs/design.md, art pipeline).
Needs Pillow and NumPy.
"""
import argparse
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw

MARK = 128


def flatten(im, hue_tol=12, colour=(255, 0, 255)):
    """Returns (flattened image, fraction of pixels that are background)."""
    rgb = im.convert("RGB")
    a = np.array(rgb)
    hsv = np.array(rgb.convert("HSV")).astype(int)
    h, w = a.shape[:2]
    border = np.concatenate([hsv[0], hsv[-1], hsv[:, 0], hsv[:, -1]])
    bg_hue = int(np.median(border[:, 0]))
    dh = np.abs(hsv[..., 0] - bg_hue)
    dh = np.minimum(dh, 256 - dh)
    cand = (dh <= hue_tol) & (hsv[..., 1] > 70) & (hsv[..., 2] > 35)
    # Pillow's floodfill does nothing on a mode-L image here, so the mask is RGB.
    m = np.where(cand, 255, 0).astype(np.uint8)
    mask = Image.fromarray(np.dstack([m, m, m]))
    seeds = [(x, y) for x in range(0, w, 8) for y in (0, h - 1)]
    seeds += [(x, y) for y in range(0, h, 8) for x in (0, w - 1)]
    for xy in seeds:
        if mask.getpixel(xy) == (255, 255, 255):
            ImageDraw.floodfill(mask, xy, (MARK, MARK, MARK), thresh=0)
    bg = np.array(mask)[..., 0] == MARK
    out = a.copy()
    out[bg] = colour
    return Image.fromarray(out), float(bg.mean())


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("outdir", type=Path)
    ap.add_argument("images", type=Path, nargs="+")
    ap.add_argument("--hue", type=int, default=12)
    ap.add_argument("--colour", default="255,0,255")
    ap.add_argument("--compare", type=Path)
    a = ap.parse_args()
    colour = tuple(int(c) for c in a.colour.split(","))
    a.outdir.mkdir(parents=True, exist_ok=True)
    pairs = []
    for f in a.images:
        im = Image.open(f)
        flat, frac = flatten(im, a.hue, colour)
        flat.save(a.outdir / f.name)
        print(f"{f.stem}: background {frac:.0%}")
        pairs.append((im.convert("RGB"), flat))
    if a.compare:
        C = 256
        sheet = Image.new("RGB", (len(pairs) * (C + 8) + 8, 2 * (C + 8) + 8), (96, 96, 104))
        for i, (before, after) in enumerate(pairs):
            sheet.paste(before.resize((C, C), Image.LANCZOS), (8 + i * (C + 8), 8))
            sheet.paste(after.resize((C, C), Image.LANCZOS), (8 + i * (C + 8), C + 16))
        sheet.save(a.compare)


if __name__ == "__main__":
    main()
