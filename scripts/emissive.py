"""Emissive layers: the pixels of a tile that give light, as their own image.

Usage: python emissive.py PALETTE.json SPEC.json TILES OUT [--moods MOODS.json --mood night --preview P.png]

SPEC (docs/art/pirates-emissive.json): {"group/tile": {"codes": [...], "box": [x0,y0,x1,y1],
"px": [[x, y, code], ...], "strength": 0-1}}. Codes are ramp short names as
the research tools print them (o0, s1, g2 ...). Writes OUT/group/tile.emit.png:
glowing pixels in their glow colour, alpha = strength, all else clear.

The engine draws a mood-recoloured tile, then adds the emit layer on top at
full brightness plus a soft halo (section 14); --preview shows that: day,
the mood alone, and the mood with the glow, per tile.
"""
import argparse, json
from pathlib import Path
import numpy as np
from PIL import Image, ImageFilter

from palsnap import load_ramps

AB = {"outline": "o", "sea": "s", "sand": "n", "skin": "k", "wood": "w", "leather": "l", "rust": "r",
      "gold": "g", "jungle": "j", "moss": "m", "stone": "t", "kraken": "q"}


def codes(palette):
    return {f"{AB[k]}{i}": c for k, v in load_ramps(palette).items() for i, c in enumerate(v)}


def emit_layer(tile, spec, col):
    a = np.array(tile.convert("RGBA")); out = np.zeros_like(a)
    s = int(round(255 * spec.get("strength", 1.0)))
    x0, y0, x1, y1 = spec.get("box", [0, 0, a.shape[1] - 1, a.shape[0] - 1])
    for c in spec.get("codes", []):
        m = np.all(a[..., :3] == col[c], -1) & (a[..., 3] > 0)
        m[:y0] = False; m[y1 + 1:] = False; m[:, :x0] = False; m[:, x1 + 1:] = False
        out[m, :3] = a[m, :3]; out[m, 3] = s
    for x, y, c in spec.get("px", []):
        out[y, x, :3] = col[c]; out[y, x, 3] = s
    return Image.fromarray(out)


def glow(base, emit, halo=0.6):
    """What the engine shows: base, plus the emit pixels, plus a soft halo."""
    b = np.array(base.convert("RGBA")).astype(float); e = np.array(emit).astype(float)
    big = emit.resize((emit.width * 4, emit.height * 4), Image.NEAREST).filter(ImageFilter.GaussianBlur(6))
    h = np.array(big.resize(emit.size, Image.BOX)).astype(float)
    k = (h[..., 3:] / 255) * halo
    b[..., :3] = np.minimum(255, b[..., :3] + h[..., :3] * k)
    ea = e[..., 3:] / 255
    b[..., :3] = b[..., :3] * (1 - ea) + e[..., :3] * ea
    return Image.fromarray(b.astype(np.uint8))


def main():
    ap = argparse.ArgumentParser()
    for n in ("palette", "spec", "tiles", "out"): ap.add_argument(n)
    ap.add_argument("--moods"); ap.add_argument("--mood", default="night"); ap.add_argument("--preview")
    a = ap.parse_args()
    col = codes(a.palette); spec = json.load(open(a.spec)); out = Path(a.out)
    rows = []
    for key, sp in spec.items():
        if key.startswith("_"):
            continue
        tile = Image.open(Path(a.tiles) / f"{key}.png").convert("RGBA")
        e = emit_layer(tile, sp, col)
        (out / key).parent.mkdir(parents=True, exist_ok=True); e.save(out / f"{key}.emit.png")
        rows.append((key, tile, e)); print(key, int((np.array(e)[..., 3] > 0).sum()), "px")
    if a.preview and a.moods:
        from mood import recolour, hexc
        md = json.load(open(a.moods))[a.mood]
        names = list(col); cols = np.array([col[n] for n in names], np.uint8)
        mc = np.array([hexc(md[f"{[k for k, v in AB.items() if v == n[0]][0]}{n[1:]}"]) for n in names])
        z = 4; S = Image.new("RGBA", (3 * (64 * z + 8), len(rows) * (64 * z + 8)), (96, 96, 104, 255))
        for i, (key, tile, e) in enumerate(rows):
            night = recolour(tile, cols, mc); lit = glow(night, e)
            for j, im in enumerate((tile, night, lit)):
                bg = Image.new("RGBA", tile.size, (40, 40, 48, 255)) if j else Image.new("RGBA", tile.size, (96, 96, 104, 255))
                bg.alpha_composite(im)
                S.paste(bg.resize((64 * z, 64 * z), Image.NEAREST), (j * (64 * z + 8), i * (64 * z + 8)))
        S.save(a.preview); print("preview", a.preview)


if __name__ == "__main__":
    main()
