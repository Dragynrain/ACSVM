"""Simple tile animation from finished tiles (docs/art/pirates-style.md section 16).

Usage: python animate.py PALETTE.json SPEC.json TILES OUT [--gif PREVIEW.gif --scale 4]

SPEC (docs/art/pirates-anim.json), per "group/tile":
  wave     rows shift sideways by round(amp * sin(2pi (y / period + t / frames))), wrapping,
           so a seamless tile stays seamless (period must divide the tile height)
  cycle    colour cycling: for each sequence s, pixels of kit colour s[0] become
           s[t % len(s)] in frame t, inside box [x0, y0, x1, y1] if given
  bob      the whole picture moves up and down by amp px (0, -amp, 0, +amp ...)
  flutter  inside box, column x moves up and down by a sine along the flag, scaled
           from 0 at the pole (box left edge) to amp at the free end
Writes OUT/group/tile-<t>.png per frame and OUT/group/tile.json ({"ms", "frames"}).
--gif: every animation side by side, looping, at --scale.
"""
import argparse, json, math
from pathlib import Path
import numpy as np
from PIL import Image

from emissive import codes


def frames(tile, sp, col):
    a = np.array(tile.convert("RGBA")); n = sp["frames"]; out = []
    H, W = a.shape[:2]
    for t in range(n):
        f = a.copy()
        k = sp["kind"]
        if k == "wave":
            assert H % sp["period"] == 0, "period must divide the tile height"
            for y in range(H):
                d = int(round(sp["amp"] * math.sin(2 * math.pi * (y / sp["period"] + t / n))))
                f[y] = np.roll(a[y], d, axis=0)
        elif k == "cycle":
            x0, y0, x1, y1 = sp.get("box", [0, 0, W - 1, H - 1])
            reg = a[y0:y1 + 1, x0:x1 + 1]; fr = f[y0:y1 + 1, x0:x1 + 1]
            for s in sp["seq"]:
                m = np.all(reg[..., :3] == col[s[0]], -1) & (reg[..., 3] > 0)
                fr[m, :3] = col[s[t % len(s)]]
        elif k == "bob":
            d = int(round(sp["amp"] * math.sin(2 * math.pi * t / n)))
            f = np.zeros_like(a)
            if d >= 0: f[:H - d] = a[d:]
            else: f[-d:] = a[:H + d]
        elif k == "flutter":
            x0, y0, x1, y1 = sp["box"]
            for x in range(x0, x1 + 1):
                u = (x - x0) / max(1, x1 - x0)
                d = int(round(sp["amp"] * u * math.sin(2 * math.pi * ((x - x0) / sp["period"] - t / n))))
                col_ = a[y0:y1 + 1, x].copy(); new = np.zeros_like(col_)
                if d > 0: new[d:] = col_[:-d]
                elif d < 0: new[:d] = col_[-d:]
                else: new = col_
                f[y0:y1 + 1, x] = new
        else:
            raise ValueError(k)
        out.append(Image.fromarray(f))
    return out


def main():
    ap = argparse.ArgumentParser()
    for n in ("palette", "spec", "tiles", "out"): ap.add_argument(n)
    ap.add_argument("--gif"); ap.add_argument("--scale", type=int, default=4)
    a = ap.parse_args(); col = codes(a.palette); spec = json.load(open(a.spec)); out = Path(a.out)
    anims = []
    for key, sp in spec.items():
        if key.startswith("_"):
            continue
        tile = Image.open(Path(a.tiles) / f"{key}.png").convert("RGBA")
        fs = frames(tile, sp, col); d = (out / key).parent; d.mkdir(parents=True, exist_ok=True)
        name = Path(key).name
        for t, f in enumerate(fs):
            f.save(d / f"{name}-{t}.png")
        json.dump({"ms": sp["ms"], "frames": len(fs)}, open(d / f"{name}.json", "w"))
        anims.append((sp["ms"], fs)); print(key, len(fs), "frames")
    if a.gif:
        # one shared clock at 20 ms ticks; each animation shows the frame due at that time
        z = a.scale; cell = 64 * z + 8; cols = 6
        rows = (len(anims) + cols - 1) // cols
        total = 1
        for ms, fs in anims:
            total = math.lcm(total, ms * len(fs))
        total = min(total, 6000); step = 20; gif = []
        for tm in range(0, total, step):
            S = Image.new("RGBA", (cols * cell, rows * cell), (40, 40, 48, 255))
            for i, (ms, fs) in enumerate(anims):
                f = fs[(tm // ms) % len(fs)].resize((64 * z, 64 * z), Image.NEAREST)
                S.alpha_composite(f, ((i % cols) * cell + 4, (i // cols) * cell + 4))
            gif.append(S.convert("RGB"))
        gif[0].save(a.gif, save_all=True, append_images=gif[1:], duration=step, loop=0)
        print("gif", a.gif, len(gif), "frames")


if __name__ == "__main__":
    main()
