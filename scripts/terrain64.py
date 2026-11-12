"""Turns generated terrain art into 64 px map squares.

Usage: python scripts/terrain64.py OUTDIR IMAGE... [--kind tex|place]
                                   [--crop 0.75] [--shrink kc|box] [--tile 64]
                                   [--candidates] [--repeat 3]

--kind tex (default): a texture that repeats across the map. Crop the centre
(--crop; 0.5 to 0.75 makes features big enough for a 64 px square), make it
seamless with scripts/quilt.py, then shrink.
--kind place: a single place (port, fort, cove) drawn edge to edge. Crop and
shrink only.

--shrink kc keeps bold outlined shapes crisp (each pixel takes its block's
dominant colour, as in scripts/sprite64.py); --shrink box (plain average)
keeps small thin features such as shells, which k-centroid breaks into dots.

--candidates writes every crop x shrink combination (crops 0.75 and 0.5 for
textures, 1.0 and 0.8 for places) as OUTDIR/<name>__<crop><shrink>.png and a
contact sheet OUTDIR/candidates.png (textures as --repeat x --repeat
repeats), to pick from by eye.

Run it only on art we generated or drew (docs/design.md, art pipeline).
Needs Pillow and NumPy.
"""
import argparse
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw

sys.path.insert(0, str(Path(__file__).parent))
from quilt import quilt  # noqa: E402
from sprite64 import k_centroid  # noqa: E402


def crop_centre(im, frac):
    w, h = im.size
    k = int(min(w, h) * frac)
    x0, y0 = (w - k) // 2, (h - k) // 2
    return im.crop((x0, y0, x0 + k, y0 + k))


def shrink(im, how, T):
    if how == "box":
        return im.convert("RGB").resize((T, T), Image.BOX)
    a = np.asarray(im.convert("RGB"))
    bs = a.shape[0] / T
    out = np.zeros((T, T, 3), np.uint8)
    for y in range(T):
        for x in range(T):
            blk = a[int(y * bs):int((y + 1) * bs), int(x * bs):int((x + 1) * bs)].reshape(-1, 3)
            out[y, x] = k_centroid(blk)
    return Image.fromarray(out)


def make(im, kind, frac, how, T):
    src = crop_centre(im, frac)
    if kind == "tex":
        src = quilt(src)
    return shrink(src, how, T)


def repeat(t, n, scale=2):
    T = t.size[0]
    big = Image.new("RGB", (T * n, T * n))
    for x in range(n):
        for y in range(n):
            big.paste(t, (x * T, y * T))
    return big.resize((T * n * scale, T * n * scale), Image.NEAREST)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("outdir", type=Path)
    ap.add_argument("images", type=Path, nargs="+")
    ap.add_argument("--kind", choices=["tex", "place"], default="tex")
    ap.add_argument("--crop", type=float, default=0.75)
    ap.add_argument("--shrink", choices=["kc", "box"], default="kc")
    ap.add_argument("--tile", type=int, default=64)
    ap.add_argument("--candidates", action="store_true")
    ap.add_argument("--repeat", type=int, default=3)
    a = ap.parse_args()
    a.outdir.mkdir(parents=True, exist_ok=True)
    if not a.candidates:
        for f in a.images:
            make(Image.open(f), a.kind, a.crop, a.shrink, a.tile).save(a.outdir / f"{f.stem}.png")
            print(f.stem)
        return
    crops = (0.75, 0.5) if a.kind == "tex" else (1.0, 0.8)
    combos = [(c, s) for c in crops for s in ("kc", "box")]
    rows = []
    for f in a.images:
        im = Image.open(f).convert("RGB")
        cells = []
        for c, s in combos:
            t = make(im, a.kind, c, s, a.tile)
            tag = f"c{int(c * 100)}{s}"
            t.save(a.outdir / f"{f.stem}__{tag}.png")
            cells.append((tag, repeat(t, a.repeat) if a.kind == "tex" else t.resize((a.tile * 3,) * 2, Image.NEAREST)))
        rows.append((f.stem, im, cells))
        print(f.stem)
    C = rows[0][2][0][1].size[0]
    W = 8 + 192 + 8 + len(combos) * (C + 8)
    sheet = Image.new("RGB", (W, len(rows) * (C + 22) + 8), (96, 96, 104))
    d = ImageDraw.Draw(sheet)
    for i, (stem, im, cells) in enumerate(rows):
        y = 8 + i * (C + 22)
        d.text((8, y), stem, fill=(255, 255, 255))
        sheet.paste(im.resize((192, 192), Image.LANCZOS), (8, y + 14))
        for j, (tag, img) in enumerate(cells):
            x = 208 + j * (C + 8)
            d.text((x, y), tag, fill=(220, 220, 220))
            sheet.paste(img, (x, y + 14))
    sheet.save(a.outdir / "candidates.png")


if __name__ == "__main__":
    main()
