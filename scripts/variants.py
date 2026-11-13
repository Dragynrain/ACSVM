"""Recolour variants of kit tiles by ramp swaps or a mood (docs/art/pirates-style.md section 17).

Usage: python variants.py PALETTE.json MOODS.json SPEC.json TILES OUT [--sheet S.png]
SPEC (docs/art/pirates-variants.json): {"name": {"base": "group/tile", "swap": [[from, to, first, last]...]}
or {"base": ..., "mood": "cursed"}}. Swaps are on exact kit colours, so the variant stays on the ramps.
Writes OUT/<name>.png; --sheet shows base and variant side by side at 4x.
"""
import argparse, json
from pathlib import Path
import numpy as np
from PIL import Image

from palsnap import load_ramps


def swap(a, ramps, pairs):
    out = a.copy()
    for fr, to, lo, hi in pairs:
        src, dst = ramps[fr], ramps[to]; n = len(src)
        for i, c in enumerate(src):
            j = lo + round(i * (hi - lo) / max(1, n - 1))
            m = np.all(a[..., :3] == c, -1) & (a[..., 3] > 0)
            out[m, :3] = dst[j]
    return out


def main():
    ap = argparse.ArgumentParser()
    for n in ("palette", "moods", "spec", "tiles", "out"): ap.add_argument(n)
    ap.add_argument("--sheet")
    a = ap.parse_args(); ramps = load_ramps(a.palette); moods = json.load(open(a.moods))
    spec = json.load(open(a.spec)); out = Path(a.out); out.mkdir(parents=True, exist_ok=True); rows = []
    for name, sp in spec.items():
        if name.startswith("_"):
            continue
        base = np.array(Image.open(Path(a.tiles) / f"{sp['base']}.png").convert("RGBA"))
        if "swap" in sp:
            v = swap(base, ramps, sp["swap"])
        else:
            md = moods[sp["mood"]]; v = base.copy()
            for r, cs in ramps.items():
                for i, c in enumerate(cs):
                    h = md[f"{r}{i}"]; m = np.all(base[..., :3] == c, -1)
                    v[m, :3] = [int(h[k:k + 2], 16) for k in (1, 3, 5)]
        Image.fromarray(v).save(out / f"{name}.png"); rows.append((base, v)); print(name)
    if a.sheet:
        z = 4; c = 64 * z + 8
        S = Image.new("RGBA", (4 * c, ((len(rows) + 1) // 2) * c), (96, 96, 104, 255))
        for i, (b, v) in enumerate(rows):
            for j, im in enumerate((b, v)):
                t = Image.fromarray(im).resize((64 * z, 64 * z), Image.NEAREST)
                S.alpha_composite(t, (((i % 2) * 2 + j) * c, (i // 2) * c))
        S.save(a.sheet)


if __name__ == "__main__":
    main()
