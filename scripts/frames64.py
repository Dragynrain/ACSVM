"""Animation frames (a video or a frame folder) to 64 px tiles that don't jitter.

Usage: python frames64.py PALETTE.json IN OUTDIR [--keep 6] [--fig 60] [--name N] [--drop-last]

IN is an animated WebP/GIF or a folder of frames on flat magenta. Each frame
is cut out as in sprite64.py; then all frames are cropped by one shared box
(the union of their silhouettes) and shrunk by one shared factor, so the
creature keeps its place and size from frame to frame, then snapped to the
kit ramps and tidied like the still tiles. --keep picks that many frames,
evenly spaced, from the loop (the last frame of a boomerang equals the first,
so it is dropped; --drop-last drops it anyway, for a first-last-frame loop whose
end only comes close to its start). Writes OUTDIR/N-<t>.png and a strip N-strip.png.
"""
import argparse
from pathlib import Path
import numpy as np
from PIL import Image, ImageSequence

from sprite64 import cut_out, k_centroid
from palsnap import load_ramps, snap
from tidy import tidy


def load_frames(src):
    src = Path(src)
    if src.is_dir():
        return [Image.open(f).convert("RGB") for f in sorted(src.glob("*.png"))]
    return [f.convert("RGB") for f in ImageSequence.Iterator(Image.open(src))]


def shrink(crop, bs, oh, ow):
    out = np.zeros((oh, ow, 4), np.uint8)
    for oy in range(oh):
        for ox in range(ow):
            blk = crop[int(oy * bs):int((oy + 1) * bs), int(ox * bs):int((ox + 1) * bs)].reshape(-1, 4)
            op = blk[blk[:, 3] > 0]
            if len(op) * 2 >= len(blk) and len(op):
                out[oy, ox, :3] = k_centroid(op[:, :3]); out[oy, ox, 3] = 255
    return out


def main():
    ap = argparse.ArgumentParser(); ap.add_argument("palette"); ap.add_argument("src"); ap.add_argument("out")
    ap.add_argument("--keep", type=int, default=6); ap.add_argument("--fig", type=int, default=60)
    ap.add_argument("--name"); ap.add_argument("--tile", type=int, default=64)
    ap.add_argument("--drop-last", action="store_true", help="the clip ends on its start image (a first-last-frame loop)")
    a = ap.parse_args(); out = Path(a.out); out.mkdir(parents=True, exist_ok=True)
    name = a.name or Path(a.src).stem
    fr = load_frames(a.src)
    if len(fr) > 2 and (a.drop_last or np.abs(np.asarray(fr[0], float) - np.asarray(fr[-1], float)).mean() < 2):
        fr = fr[:-1]
    idx = np.linspace(0, len(fr), a.keep, endpoint=False).astype(int) if a.keep < len(fr) else range(len(fr))
    cut = [cut_out(fr[i]) for i in idx]
    union = np.zeros(cut[0].shape[:2], bool)
    for c in cut:
        union |= c[..., 3] > 0
    ys, xs = np.nonzero(union)
    y0, y1, x0, x1 = ys.min(), ys.max() + 1, xs.min(), xs.max() + 1
    h, w = y1 - y0, x1 - x0; bs = max(h, w) / a.fig
    oh, ow = max(1, round(h / bs)), max(1, round(w / bs))
    ramps = load_ramps(a.palette); pal = np.array([c for v in ramps.values() for c in v], np.uint8)
    outline = np.array(ramps["outline"][0], np.uint8)
    tiles = []
    for t, c in enumerate(cut):
        small = shrink(c[y0:y1, x0:x1], bs, oh, min(ow, a.tile))
        res = np.zeros((a.tile, a.tile, 4), np.uint8)
        oy, ox = a.tile - 1 - oh, (a.tile - small.shape[1]) // 2
        res[oy:oy + oh, ox:ox + small.shape[1]] = small
        res = snap(res, pal, 1.5)[0]
        res = tidy(res, outline, 0.42)[0]  # tidy.py's default dark threshold
        Image.fromarray(res).save(out / f"{name}-{t}.png"); tiles.append(res)
    strip = Image.new("RGBA", (a.tile * len(tiles), a.tile), (0, 0, 0, 0))
    for t, r in enumerate(tiles):
        strip.alpha_composite(Image.fromarray(r), (t * a.tile, 0))
    strip.save(out / f"{name}-strip.png")
    print(name, len(fr), "frames in,", len(tiles), "kept, box", (x0, y0, x1, y1))


if __name__ == "__main__":
    main()
