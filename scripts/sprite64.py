"""Turns generated art into small sprite tiles.

Usage: python scripts/sprite64.py OUTDIR IMAGE... [--tile 64] [--fig 60]
                                  [--palette N] [--sheet SHEET.png] [--show 3]

For each IMAGE (a figure on a flat background, as the prompt templates in
docs/art/ ask for):

1. Cut out: background connected to the image border becomes transparent,
   plus any strongly magenta fringe pixels left by anti-aliasing.
2. Crop to the figure and shrink it to --fig px tall with k-centroid
   downscaling: each output pixel takes the dominant colour of its source
   block (k-means, largest cluster), not the average, so outlines stay dark
   and small features keep their colour.
3. Place it on a --tile square transparent tile, centred, feet on the
   bottom row but one. Wider-than-tile figures are clipped at the right.

--palette N snaps every tile to one N-colour palette shared by the batch
(median cut, no dither) and writes it as OUTDIR/palette.png. --sheet writes
a contact sheet: per image the old whole-frame average next to the new tile
(and the snapped tile with --palette), scaled --show times.

Run it only on art we generated or drew (see docs/design.md, art pipeline):
never on EA or other third-party pictures. Needs Pillow and NumPy.
"""
import argparse
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw

BOARD = (96, 96, 104)
MARKER = (1, 254, 3)  # flood-fill marker; not a colour the art uses


def cut_out(im):
    """RGBA array with the border-connected flat background transparent."""
    rgb = im.convert("RGB")
    w, h = rgb.size
    work = rgb.copy()
    seeds = [(x, y) for x in range(0, w, 16) for y in (0, h - 1)]
    seeds += [(x, y) for y in range(0, h, 16) for x in (0, w - 1)]
    for xy in seeds:
        if work.getpixel(xy) != MARKER:
            ImageDraw.floodfill(work, xy, MARKER, thresh=110)
    a = np.array(rgb)
    bg = np.all(np.array(work) == MARKER, axis=2)
    r, g, b = (a[..., i].astype(int) for i in range(3))
    fringe = (r > 120) & (b > 100) & (g < 0.55 * np.minimum(r, b))
    alpha = np.where(bg | fringe, 0, 255).astype(np.uint8)
    return np.dstack([a, alpha])


def k_centroid(px, k=3, iters=6):
    """Dominant colour of an (N, 3) block of pixels."""
    px = px.astype(float)
    if len(px) <= k:
        return px.mean(0)
    cent = px[np.linspace(0, len(px) - 1, k).astype(int)]
    for _ in range(iters):
        lab = np.argmin(((px[:, None] - cent[None]) ** 2).sum(2), 1)
        for j in range(k):
            if np.any(lab == j):
                cent[j] = px[lab == j].mean(0)
    return cent[np.argmax(np.bincount(lab, minlength=k))]


def to_tile(rgba, tile, fig):
    """Crop to the figure, shrink it to fig px tall, place it on the tile."""
    ys, xs = np.nonzero(rgba[..., 3])
    if len(ys) == 0:
        raise ValueError("nothing left after cutting out the background")
    crop = rgba[ys.min():ys.max() + 1, xs.min():xs.max() + 1]
    h, w = crop.shape[:2]
    bs = h / fig
    oh, ow = fig, max(1, round(w / bs))
    out = np.zeros((oh, ow, 4), np.uint8)
    for oy in range(oh):
        for ox in range(ow):
            blk = crop[int(oy * bs):int((oy + 1) * bs), int(ox * bs):int((ox + 1) * bs)].reshape(-1, 4)
            opaque = blk[blk[:, 3] > 0]
            if len(opaque) * 2 >= len(blk):
                out[oy, ox, :3] = k_centroid(opaque[:, :3])
                out[oy, ox, 3] = 255
    res = np.zeros((tile, tile, 4), np.uint8)
    ow = min(ow, tile)
    x0, y0 = (tile - ow) // 2, tile - 1 - oh
    res[y0:y0 + oh, x0:x0 + ow] = out[:, :ow]
    return res


def make_palette(tiles, n):
    px = np.concatenate([t[t[..., 3] > 0][:, :3] for t in tiles])
    strip = Image.fromarray(px[None].astype(np.uint8))
    return strip.quantize(colors=n, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE)


def snap(t, pal):
    q = Image.fromarray(t[..., :3]).quantize(palette=pal, dither=Image.Dither.NONE).convert("RGB")
    return np.dstack([np.array(q), t[..., 3]])


def shown(arr, scale):
    img = Image.new("RGB", (arr.shape[1], arr.shape[0]), BOARD)
    if arr.shape[2] == 4:
        img.paste(Image.fromarray(arr[..., :3]), mask=Image.fromarray(arr[..., 3]))
    else:
        img = Image.fromarray(arr)
    return img.resize((arr.shape[1] * scale, arr.shape[0] * scale), Image.NEAREST)


def write_sheet(path, rows, tile, scale):
    """rows: (name, [arrays]) per image; one row per image."""
    cell, pad, label = tile * scale, 8, 160
    cols = max(len(r[1]) for r in rows)
    sheet = Image.new("RGB", (label + cols * (cell + pad) + pad, len(rows) * (cell + pad) + pad), BOARD)
    d = ImageDraw.Draw(sheet)
    for ri, (name, arrs) in enumerate(rows):
        y = pad + ri * (cell + pad)
        d.text((6, y + cell // 2), name, fill=(255, 255, 255))
        for ci, arr in enumerate(arrs):
            sheet.paste(shown(arr, scale), (label + ci * (cell + pad), y))
    sheet.save(path)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("outdir", type=Path)
    ap.add_argument("images", type=Path, nargs="+")
    ap.add_argument("--tile", type=int, default=64)
    ap.add_argument("--fig", type=int, default=60, help="figure height in px")
    ap.add_argument("--palette", type=int, default=0, help="snap to one N-colour palette")
    ap.add_argument("--sheet", type=Path)
    ap.add_argument("--show", type=int, default=3, help="sheet scale")
    a = ap.parse_args()
    if a.fig >= a.tile:
        ap.error("--fig must be smaller than --tile")
    a.outdir.mkdir(parents=True, exist_ok=True)

    done = []
    for f in a.images:
        im = Image.open(f)
        old = np.array(im.convert("RGB").resize((a.tile, a.tile), Image.BOX))
        done.append((f.stem, old, to_tile(cut_out(im), a.tile, a.fig)))
        print(f.stem, flush=True)

    pal = make_palette([t for _, _, t in done], a.palette) if a.palette else None
    if pal:
        cols = np.array(pal.getpalette()[:3 * a.palette], np.uint8).reshape(1, -1, 3)
        Image.fromarray(cols).resize((cols.shape[1] * 8, 8), Image.NEAREST).save(a.outdir / "palette.png")
    rows = []
    for stem, old, t in done:
        final = snap(t, pal) if pal else t
        Image.fromarray(final).save(a.outdir / f"{stem}.png")
        rows.append((stem, [old, t] + ([final] if pal else [])))
    if a.sheet:
        write_sheet(a.sheet, rows, a.tile, a.show)
        print(a.sheet)


if __name__ == "__main__":
    main()
