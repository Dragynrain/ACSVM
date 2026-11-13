"""Normal maps for kit tiles, for torch light at night (docs/art/pirates-style.md section 15).

Usage: python normals.py OUT TILE.png... [--root DIR] [--depth 2.0] [--bevel 3]
       [--moods MOODS.json --mood night --preview P.png]

No AI and no painting: a height field is built per tile from
  - lightness (OKLab L): lit faces are drawn light, so light means raised;
  - a bevel from the silhouette: height rises over the first --bevel pixels
    inside the tile's outline, so figures and objects read as rounded.
Terrain (no transparent pixels) gets lightness only. Height is smoothed
1 px, then normals come from its slope: n = (-dh/dx, -dh/dy, 1/depth),
stored as RGB = (n + 1) / 2 * 255, alpha copied from the tile.
Writes OUT/<name>.normal.png (paths under --root keep their sub-folders).
--preview: each tile at night, lit by a warm point light at three places.
"""
import argparse, json
from pathlib import Path
import numpy as np
from PIL import Image

from palsnap import oklab


def inner_distance(mask, cap):
    """Pixels from the nearest outside pixel (4-neighbour), up to cap."""
    d = np.zeros(mask.shape); cur = mask.copy()
    for i in range(int(np.ceil(cap))):
        d += cur
        p = np.pad(cur, 1)
        cur = cur & p[:-2, 1:-1] & p[2:, 1:-1] & p[1:-1, :-2] & p[1:-1, 2:]
    return d


def blur(h, sigma):
    r = int(np.ceil(sigma * 2)); x = np.arange(-r, r + 1); k = np.exp(-x ** 2 / (2 * sigma ** 2)); k /= k.sum()
    p = np.pad(h, r, mode="edge")
    p = sum(k[i] * p[:, i:i + h.shape[1]] for i in range(len(k)))
    return sum(k[i] * p[i:i + h.shape[0]] for i in range(len(k)))


def normal_map(tile, depth=2.0, bevel=3):
    a = np.array(tile.convert("RGBA")).astype(float)
    alpha = a[..., 3] > 0
    L = oklab(a[..., :3].astype(np.uint8))[..., 0]
    h = L.copy()
    if not alpha.all():
        d = inner_distance(alpha, bevel)
        h = 0.6 * L + 0.4 * np.clip(d / bevel, 0, 1)
        h[~alpha] = 0
    h = blur(h, 0.8)
    gy, gx = np.gradient(h)
    n = np.stack([-gx * depth * 4, -gy * depth * 4, np.ones_like(h)], -1)
    n /= np.linalg.norm(n, axis=-1, keepdims=True)
    rgb = ((n + 1) / 2 * 255).round().astype(np.uint8)
    out = np.dstack([rgb, a[..., 3].astype(np.uint8)])
    return Image.fromarray(out), n


def light(img, n, lx, ly, lz=14.0, colour=(255, 200, 130), power=3.2, ambient=0.3):
    """Point light at (lx, ly, lz) in tile pixels; Lambert on the normal map."""
    a = np.array(img.convert("RGBA")).astype(float)
    H, W = n.shape[:2]; yy, xx = np.mgrid[0:H, 0:W]
    v = np.stack([lx - xx, ly - yy, np.full((H, W), lz)], -1)
    dist = np.linalg.norm(v, axis=-1, keepdims=True); v /= dist
    lam = np.clip((n * v).sum(-1, keepdims=True), 0, 1)
    fall = 1 / (1 + (dist / 26) ** 2)
    k = ambient + power * lam * fall * (np.array(colour) / 255)
    a[..., :3] = np.clip(a[..., :3] * k, 0, 255)
    return Image.fromarray(a.astype(np.uint8))


def main():
    ap = argparse.ArgumentParser(); ap.add_argument("out"); ap.add_argument("tiles", nargs="+")
    ap.add_argument("--root"); ap.add_argument("--depth", type=float, default=2.0)
    ap.add_argument("--bevel", type=float, default=3)
    ap.add_argument("--moods"); ap.add_argument("--mood", default="night"); ap.add_argument("--preview")
    a = ap.parse_args(); out = Path(a.out); rows = []
    for f in a.tiles:
        f = Path(f); rel = f.relative_to(a.root) if a.root else Path(f.name)
        tile = Image.open(f).convert("RGBA"); nm, n = normal_map(tile, a.depth, a.bevel)
        dst = out / rel.with_suffix(".normal.png"); dst.parent.mkdir(parents=True, exist_ok=True); nm.save(dst)
        rows.append((tile, nm, n))
    print(len(rows), "normal maps ->", out)
    if a.preview and a.moods:
        from mood import recolour, hexc
        from palsnap import load_ramps
        md = json.load(open(a.moods))[a.mood]; R = load_ramps(Path(a.moods).with_name("pirates-palette.json"))
        names = [f"{k}{i}" for k, v in R.items() for i in range(len(v))]
        cols = np.array([c for v in R.values() for c in v], np.uint8); mc = np.array([hexc(md[x]) for x in names])
        z = 3; c = 64 * z + 6
        S = Image.new("RGBA", (5 * c, len(rows) * c), (96, 96, 104, 255))
        for i, (tile, nm, n) in enumerate(rows):
            night = recolour(tile, cols, mc)
            ims = [tile, nm] + [light(night, n, x, y) for x, y in ((8, 8), (32, 40), (58, 20))]
            for j, im in enumerate(ims):
                bg = Image.new("RGBA", tile.size, (30, 30, 38, 255)); bg.alpha_composite(im)
                S.paste(bg.resize((64 * z, 64 * z), Image.NEAREST), (j * c, i * c))
        S.save(a.preview); print("preview", a.preview)


if __name__ == "__main__":
    main()
