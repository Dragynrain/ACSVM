"""Automatic checks on a finished kit (design: art pipeline step 8).

Usage: python kitcheck.py KITDIR OUTDIR [--groups kit kit2 kp ...] [--threshold 0.80]

Writes to OUTDIR:
  sheet-64.png, sheet-32.png   every tile at 1x and at half size (2x zoom), the
                               size a laptop shows, and the smallest that must read
  sheet-grey.png               lightness only: values must separate figure from ground
  sheet-deutan.png, -protan, -tritan   colour-blind simulations (Machado 2009, severity 1)
  silhouettes.txt, silhouettes.png     pairs of non-terrain tiles whose solid
                               silhouettes overlap most (IoU after centring), worst first
Terrain, walls and floors (no transparent pixels) are left out of the silhouette test, and by
default so is anything not standing (--tall): boxes and barrels share shapes and differ by colour.
"""
import argparse
from pathlib import Path
import numpy as np
from PIL import Image, ImageDraw

from palsnap import oklab

CVD = {  # Machado, Oliveira and Fernandes 2009, severity 1.0, linear RGB
    "protan": [[0.152286, 1.052583, -0.204868], [0.114503, 0.786281, 0.099216], [-0.003882, -0.048116, 1.051998]],
    "deutan": [[0.367322, 0.860646, -0.227968], [0.280085, 0.672501, 0.047413], [-0.011820, 0.042940, 0.968881]],
    "tritan": [[1.255528, -0.076749, -0.178779], [-0.078411, 0.930809, 0.147602], [0.004733, 0.691367, 0.303900]],
}


def lin(c): c = c / 255; return np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)
def srgb(c): c = np.clip(c, 0, 1); return np.round(255 * np.where(c <= 0.0031308, c * 12.92, 1.055 * c ** (1 / 2.4) - 0.055)).astype(np.uint8)


def sheet(tiles, f=None, z=1, cols=20, half=False):
    s = 32 if half else 64; cell = s * z + 4
    S = Image.new("RGBA", (cols * cell + 4, ((len(tiles) + cols - 1) // cols) * cell + 4), (96, 96, 104, 255))
    for i, (k, im) in enumerate(tiles):
        if half:
            im = im.resize((32, 32), Image.BOX)
        if f:
            im = f(im)
        S.alpha_composite(im.resize((s * z, s * z), Image.NEAREST), (4 + (i % cols) * cell, 4 + (i // cols) * cell))
    return S


def grey(im):
    a = np.array(im).astype(float); L = oklab(a[..., :3].astype(np.uint8))[..., 0]
    a[..., :3] = np.clip(L * 255, 0, 255)[..., None]; return Image.fromarray(a.astype(np.uint8))


def cvd(kind):
    M = np.array(CVD[kind])
    def f(im):
        a = np.array(im); a[..., :3] = srgb(lin(a[..., :3].astype(float)) @ M.T); return Image.fromarray(a)
    return f


def silhouette(im):
    m = np.array(im)[..., 3] > 0; ys, xs = np.nonzero(m)
    m = m[ys.min():ys.max() + 1, xs.min():xs.max() + 1]
    out = np.zeros((64, 64), bool); h, w = m.shape
    y0 = 64 - h; x0 = (64 - w) // 2  # feet on the baseline, centred
    out[y0:y0 + h, x0:x0 + w] = m
    return out


def main():
    ap = argparse.ArgumentParser(); ap.add_argument("kit"); ap.add_argument("out")
    ap.add_argument("--groups", nargs="+", default=["kit", "kit2", "kp", "player", "terrain", "rooms"])
    ap.add_argument("--threshold", type=float, default=0.80)
    ap.add_argument("--tall", type=float, default=1.6, help="silhouette test only for figures this many times taller "
                    "than wide (standing people, where the style sheet asks one read per role); 0 tests every cut-out")
    a = ap.parse_args(); kit, out = Path(a.kit), Path(a.out); out.mkdir(parents=True, exist_ok=True)
    tiles = [(f"{g}/{f.stem}", Image.open(f).convert("RGBA")) for g in a.groups for f in sorted((kit / g).glob("*.png"))]
    sheet(tiles).save(out / "sheet-64.png")
    sheet(tiles, z=2, half=True).save(out / "sheet-32.png")
    sheet(tiles, grey).save(out / "sheet-grey.png")
    for k in CVD:
        sheet(tiles, cvd(k)).save(out / f"sheet-{k}an.png" if not k.endswith("an") else out / f"sheet-{k}.png")
    def tall(im):
        ys, xs = np.nonzero(np.array(im)[..., 3]); return (ys.max() - ys.min() + 1) >= a.tall * (xs.max() - xs.min() + 1)
    figs = [(k, silhouette(im)) for k, im in tiles if (np.array(im)[..., 3] == 0).any() and (a.tall == 0 or tall(im))]
    pairs = []
    for i in range(len(figs)):
        for j in range(i + 1, len(figs)):
            A, B = figs[i][1], figs[j][1]
            iou = (A & B).sum() / max(1, (A | B).sum())
            pairs.append((iou, figs[i][0], figs[j][0]))
    pairs.sort(reverse=True)
    flagged = [p for p in pairs if p[0] >= a.threshold]
    with open(out / "silhouettes.txt", "w") as f:
        f.write(f"{len(figs)} figures, {len(flagged)} pairs at IoU >= {a.threshold}\n")
        for p in pairs[:60]:
            f.write(f"{p[0]:.3f}  {p[1]}  {p[2]}{'  FLAG' if p[0] >= a.threshold else ''}\n")
    show = pairs[:24]; z = 2; c = 64 * z + 6
    S = Image.new("RGB", (6 * c, len(show) * (c + 14)), (96, 96, 104)); d = ImageDraw.Draw(S)
    lookup = dict(tiles); sil = dict(figs)
    for r, (iou, p, q) in enumerate(show):
        y = r * (c + 14)
        d.text((4, y), f"{iou:.3f}  {p}  /  {q}", fill=(255, 255, 255))
        for j, k in enumerate((p, q)):
            S.paste(lookup[k].resize((64 * z, 64 * z), Image.NEAREST), (j * c, y + 14), lookup[k].resize((64 * z, 64 * z), Image.NEAREST))
            m = Image.fromarray((sil[k] * 255).astype(np.uint8)).resize((64 * z, 64 * z), Image.NEAREST)
            S.paste(m, ((2 + j) * c, y + 14))
        both = np.dstack([sil[p] * 255, sil[q] * 255, np.zeros((64, 64))]).astype(np.uint8)
        S.paste(Image.fromarray(both).resize((64 * z, 64 * z), Image.NEAREST), (4 * c, y + 14))
    S.save(out / "silhouettes.png")
    print(len(tiles), "tiles;", len(figs), "figures;", len(flagged), f"silhouette pairs >= {a.threshold}")
    for p in pairs[:12]:
        print(f"  {p[0]:.3f} {p[1]} {p[2]}")


if __name__ == "__main__":
    main()
