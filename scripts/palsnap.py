"""Snaps tiles to the kit palette ramps.

Usage: python scripts/palsnap.py PALETTE.json OUTDIR IMAGE... [--sheet SHEET.png]
                                 [--show 3] [--swatch SWATCH.png] [--chroma 1.5]

PALETTE.json maps ramp names to lists of hex colours, light to dark (see
docs/art/pirates-palette.json; keys starting with "_" are notes). Each pixel
of each IMAGE becomes its nearest palette colour in OKLab, with chroma
(a and b) weighted by --chroma so a pixel keeps its hue family rather than
dropping to a grey of the same lightness. No dither. Alpha is kept
(semi-transparent pixels become fully opaque or transparent at 128).

Unlike sprite64.py --palette (a median-cut palette fitted to each batch),
the ramps are fixed, so every batch shares the same colours and small
colour areas (a purple arm, a red sash) keep their ramp.

--sheet writes a before-and-after contact sheet with each tile's mean
colour error, worst first. --swatch writes the ramps as rows of swatches.

Run it only on art we generated or drew (docs/design.md, art pipeline).
Needs Pillow and NumPy.
"""
import argparse
import json
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw

sys.path.insert(0, str(Path(__file__).parent))
from sprite64 import BOARD, shown  # noqa: E402


def load_ramps(path):
    data = json.loads(Path(path).read_text())
    return {k: [tuple(int(h[i:i + 2], 16) for i in (1, 3, 5)) for h in v]
            for k, v in data.items() if not k.startswith("_")}


def oklab(rgb):
    """(..., 3) sRGB 0-255 to OKLab."""
    c = rgb.astype(float) / 255
    c = np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)
    m1 = np.array([[0.4122214708, 0.5363325363, 0.0514459929],
                   [0.2119034982, 0.6806995451, 0.1073969566],
                   [0.0883024619, 0.2817188376, 0.6299787005]])
    m2 = np.array([[0.2104542553, 0.7936177850, -0.0040720468],
                   [1.9779984951, -2.4285922050, 0.4505937099],
                   [0.0259040371, 0.7827717662, -0.8086757660]])
    return np.cbrt(c @ m1.T) @ m2.T


def snap(rgba, pal, chroma):
    """rgba (H, W, 4) to the nearest palette colours; returns (tile, error)."""
    w = np.array([1.0, chroma, chroma])
    lab = oklab(rgba[..., :3]) * w
    plab = oklab(pal) * w
    d = ((lab[..., None, :] - plab) ** 2).sum(-1)
    idx = d.argmin(-1)
    out = rgba.copy()
    out[..., :3] = pal[idx]
    opaque = rgba[..., 3] >= 128
    out[..., 3] = np.where(opaque, 255, 0)
    err = np.sqrt(d.min(-1))[opaque].mean() if opaque.any() else 0.0
    return out, float(err)


def write_swatch(path, ramps, size=24):
    width = max(len(v) for v in ramps.values())
    img = Image.new("RGB", (90 + width * size, len(ramps) * size), BOARD)
    dr = ImageDraw.Draw(img)
    for r, (name, cols) in enumerate(ramps.items()):
        dr.text((4, r * size + 6), name, fill=(255, 255, 255))
        for i, c in enumerate(cols):
            dr.rectangle([90 + i * size, r * size, 90 + (i + 1) * size - 1, (r + 1) * size - 1], fill=c)
    img.save(path)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("palette", type=Path)
    ap.add_argument("outdir", type=Path)
    ap.add_argument("images", type=Path, nargs="+")
    ap.add_argument("--chroma", type=float, default=1.5, help="weight of a and b against lightness")
    ap.add_argument("--sheet", type=Path)
    ap.add_argument("--show", type=int, default=3, help="sheet scale")
    ap.add_argument("--swatch", type=Path)
    a = ap.parse_args()
    ramps = load_ramps(a.palette)
    pal = np.array([c for v in ramps.values() for c in v], np.uint8)
    a.outdir.mkdir(parents=True, exist_ok=True)
    if a.swatch:
        write_swatch(a.swatch, ramps)

    rows = []
    for f in a.images:
        rgba = np.array(Image.open(f).convert("RGBA"))
        out, err = snap(rgba, pal, a.chroma)
        Image.fromarray(out).save(a.outdir / f.name)
        rows.append((err, f.stem, rgba, out))
        print(f"{err:.4f} {f.stem}")
    if a.sheet:
        rows.sort(key=lambda r: -r[0])
        cell, pad, label = rgba.shape[0] * a.show, 8, 170
        sheet = Image.new("RGB", (label + 2 * (cell + pad) + pad, len(rows) * (cell + pad) + pad), BOARD)
        dr = ImageDraw.Draw(sheet)
        for i, (err, stem, before, after) in enumerate(rows):
            y = pad + i * (cell + pad)
            dr.text((6, y + cell // 2), f"{stem} {err:.3f}", fill=(255, 255, 255))
            sheet.paste(shown(before, a.show), (label, y))
            sheet.paste(shown(after, a.show), (label + cell + pad, y))
        sheet.save(a.sheet)
        print(a.sheet)


if __name__ == "__main__":
    main()
