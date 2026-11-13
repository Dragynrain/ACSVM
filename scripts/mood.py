"""Mood ramps: one colour table per region mood over the kit palette.

Usage:
  python mood.py PALETTE.json MOODS.json            write the tables
  python mood.py PALETTE.json MOODS.json --preview OUT.png IMG...   tables, then
      every IMG in every mood, one row per image (images snapped to the ramps)

A mood maps each ramp colour to a new colour, so the engine recolours a
picture with a lookup and nothing else (docs/art/pirates-style.md section 13).
The tables are computed in OKLab from the parameters below:
  L      lightness: L' = L * scale + add, then pulled toward 0.5 by (1 - contrast)
  C      chroma (saturation) scale
  tint   (a, b) added in OKLab, weighted by tint_w: blue night, green curse
  red    scale on positive a (red), so underwater loses its reds first
  toward colour every result is mixed toward, by `mix` (fog, deep water)
Colours on the emissive layer are not dimmed by a mood: the engine draws
them over the moody picture at full strength (section 13).
"""
import argparse, json
from pathlib import Path
import numpy as np
from PIL import Image

from palsnap import load_ramps, oklab

MOODS = {
    "night":      dict(scale=0.5, add=0.0, contrast=0.95, C=0.6, tint=(-0.012, -0.045), tint_w=1.0, red=0.8,
                       toward="#17223f", mix=0.12),
    "fog":        dict(scale=0.55, add=0.33, contrast=0.85, C=0.45, tint=(0.0, -0.01), tint_w=1.0, red=1.0,
                       toward="#c8ccd2", mix=0.3),
    "cursed":     dict(scale=0.78, add=0.0, contrast=1.1, C=0.5, tint=(-0.035, 0.03), tint_w=1.0, red=0.6,
                       toward="#2e3a2c", mix=0.12),
    "underwater": dict(scale=0.78, add=0.02, contrast=0.9, C=0.75, tint=(-0.04, -0.03), tint_w=1.0, red=0.3,
                       toward="#1e5a72", mix=0.25),
}


def oklab_to_srgb(lab):
    m2i = np.array([[1.0, 0.3963377774, 0.2158037573],
                    [1.0, -0.1055613458, -0.0638541728],
                    [1.0, -0.0894841775, -1.2914855480]])
    m1i = np.array([[4.0767416621, -3.3077115913, 0.2309699292],
                    [-1.2684380046, 2.6097574011, -0.3413193965],
                    [-0.0041960863, -0.7034186147, 1.7076147010]])
    lin = (lab @ m2i.T) ** 3 @ m1i.T
    lin = np.clip(lin, 0, 1)
    c = np.where(lin <= 0.0031308, lin * 12.92, 1.055 * lin ** (1 / 2.4) - 0.055)
    return np.clip(np.round(c * 255), 0, 255).astype(np.uint8)


def hexc(s):
    return np.array([int(s[i:i + 2], 16) for i in (1, 3, 5)], np.uint8)


def apply(rgb, p):
    lab = oklab(rgb.reshape(-1, 3))
    L, a, b = lab[:, 0], lab[:, 1], lab[:, 2]
    L = L * p["scale"] + p["add"]
    L = 0.5 + (L - 0.5) * p["contrast"]
    a = np.where(a > 0, a * p["red"], a) * p["C"] + p["tint"][0] * p["tint_w"]
    b = b * p["C"] + p["tint"][1] * p["tint_w"]
    lab = np.stack([np.clip(L, 0, 1), a, b], 1)
    t = oklab(hexc(p["toward"])[None])[0]
    lab = lab * (1 - p["mix"]) + t * p["mix"]
    return oklab_to_srgb(lab).reshape(rgb.shape)


def tables(palette):
    ramps = load_ramps(palette)
    names = [f"{k}{i}" for k, v in ramps.items() for i in range(len(v))]
    cols = np.array([c for v in ramps.values() for c in v], np.uint8)
    out = {"_note": "Mood ramps for the pirate kit: each kit colour (ramp name and step) to its colour in "
                    "that mood. Built by scripts/mood.py; docs/art/pirates-style.md section 13. CC BY 4.0.",
           "_params": MOODS}
    for m, p in MOODS.items():
        new = apply(cols, p)
        out[m] = {n: "#%02x%02x%02x" % tuple(int(x) for x in c) for n, c in zip(names, new)}
    return out, names, cols


def recolour(img, cols, mood_cols):
    a = np.array(img.convert("RGBA")); rgb = a[..., :3].reshape(-1, 3)
    # snap to the kit first (picture may be off-palette after scaling), then look up
    d = ((rgb[:, None, :].astype(int) - cols[None].astype(int)) ** 2).sum(2)
    idx = d.argmin(1)
    a[..., :3] = mood_cols[idx].reshape(a.shape[:2] + (3,))
    return Image.fromarray(a)


def main():
    ap = argparse.ArgumentParser(); ap.add_argument("palette"); ap.add_argument("moods")
    ap.add_argument("--preview"); ap.add_argument("images", nargs="*")
    a = ap.parse_args()
    out, names, cols = tables(a.palette)
    Path(a.moods).write_text(json.dumps(out, indent=1) + "\n")
    print("wrote", a.moods, list(MOODS))
    if a.preview and a.images:
        ims = [Image.open(f).convert("RGBA") for f in a.images]
        w = max(i.width for i in ims); h = sum(i.height for i in ims)
        sheet = Image.new("RGBA", ((w + 8) * (len(MOODS) + 1), h + 8 * len(ims)), (96, 96, 104, 255))
        y = 0
        for im in ims:
            sheet.alpha_composite(im, (0, y))
            for k, m in enumerate(MOODS):
                mc = np.array([hexc(out[m][n]) for n in names])
                sheet.alpha_composite(recolour(im, cols, mc), ((w + 8) * (k + 1), y))
            y += im.height + 8
        sheet.save(a.preview); print("preview", a.preview)


if __name__ == "__main__":
    main()
