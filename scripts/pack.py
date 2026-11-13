"""Pack a finished kit into one atlas with a manifest (design: art pipeline step 9).

Usage: python pack.py KITDIR ARTROOT OUT [--name pirates] [--licence "CC BY-SA 4.0"]

KITDIR   the assembled set (group folders of 64 px tiles, plus sources.json
         saying where each tile came from, as the research assemble step writes it)
ARTROOT  folder searched for generation logs (*manifest*.json: lists of
         {file, seed, model, lora, prompt, src?}), picks.json files (picture name to
         chosen file) and hand-edit files (_faces.json, _things.json)
Writes OUT/<name>.png and OUT/<name>.webp (lossless), and OUT/<name>.json: per tile its
atlas box, group, source file, the generation record(s) that made it (following
Qwen-Image-Edit sources back to the first generation), the hand-edit file that
touched it, and the licence. Layers (emit, normal) and animation frames, when given
with --layer NAME=DIR, are packed in their own atlases at the same boxes.
"""
import argparse, json, re
from pathlib import Path
from PIL import Image


def index_manifests(root):
    gen = {}
    for f in root.rglob("*manifest*.json"):
        if any(p.startswith("final") for p in f.relative_to(root).parts):
            continue
        try:
            d = json.load(open(f, encoding="utf-8-sig"))
        except (ValueError, UnicodeDecodeError):
            continue
        for e in d if isinstance(d, list) else [d]:
            if isinstance(e, dict) and "file" in e:
                gen.setdefault(e["file"], dict(e, log=str(f.relative_to(root))))
    picks = {}
    for f in root.rglob("picks.json"):
        try:
            d = json.load(open(f, encoding="utf-8-sig"))
        except ValueError:
            continue
        for k, v in d.items():
            if not k.startswith("_"):
                picks.setdefault(k, []).append((f.parent.name, v))
    edits = {}
    for f in list(root.rglob("_faces.json")) + list(root.rglob("_things.json")):
        for k in json.load(open(f)):
            edits.setdefault(k, []).append(str(f.relative_to(root)))
    return gen, picks, edits


def chain(name, gen, depth=0):
    """Generation records for a raw file name, following edit sources."""
    if depth > 4 or name not in gen:
        return []
    e = gen[name]; out = [{k: e[k] for k in ("file", "model", "lora", "seed", "prompt", "log") if k in e}]
    if e.get("src"):
        out += chain(Path(str(e["src"]).replace("\\", "/")).name, gen, depth + 1)
    return out


def raw_names(stem, picks):
    names = [stem + ".png"]
    base = re.sub(r"-\d+$", "", stem)
    for folder, v in picks.get(base, []) + picks.get(stem, []):
        if isinstance(v, int):
            names.append(f"{base}-{v}.png")
        elif isinstance(v, str):
            m = re.match(r"([^\s(]+)", v.replace("\\", "/"))
            if m:
                n = Path(m.group(1)).name
                n = re.sub(r"__.*$", "", n.removesuffix(".png")) + ".png"
                names.append(n)
    return names


def main():
    ap = argparse.ArgumentParser(); ap.add_argument("kit"); ap.add_argument("root"); ap.add_argument("out")
    ap.add_argument("--name", default="pirates"); ap.add_argument("--licence", default="CC BY-SA 4.0")
    ap.add_argument("--layer", action="append", default=[], help="NAME=DIR with DIR/<group>/<tile>.<NAME>.png")
    ap.add_argument("--cols", type=int, default=16)
    a = ap.parse_args(); kit, root, out = Path(a.kit), Path(a.root), Path(a.out); out.mkdir(parents=True, exist_ok=True)
    src = json.load(open(kit / "sources.json"))
    keys = [k for k in src if k.endswith(".png") and Image.open(kit / k).size == (64, 64)]
    gen, picks, edits = index_manifests(root)
    n = len(keys); rows = (n + a.cols - 1) // a.cols
    atlas = Image.new("RGBA", (a.cols * 64, rows * 64), (0, 0, 0, 0))
    layers = {}
    for spec in a.layer:
        nm, d = spec.split("=", 1); layers[nm] = (Path(d), Image.new("RGBA", atlas.size, (0, 0, 0, 0)))
    meta = {"name": a.name, "tile": 64, "licence": a.licence, "layers": ["base"] + list(layers), "tiles": {}}
    unknown = 0
    for i, k in enumerate(keys):
        x, y = (i % a.cols) * 64, (i // a.cols) * 64
        atlas.alpha_composite(Image.open(kit / k).convert("RGBA"), (x, y))
        stem = Path(k).stem; key = k[:-4]
        g = []
        for rn in raw_names(stem, picks):
            g = chain(rn, gen)
            if g:
                break
        if not g:
            unknown += 1
            for folder, v in picks.get(re.sub(r"-\d+$", "", stem), []) + picks.get(stem, []):
                g = [{"pick": v if isinstance(v, str) else f"{stem}", "picks": f"{folder}/picks.json",
                      "note": "no generation log; the prompt is in the generating script"}]
                break
            if not g and "-" in stem and k.startswith("rooms/doors/"):  # a door laid over a wall
                door, wall = stem.split("-", 1)
                g = [{"composite": [f"rooms/doors/{door}", f"rooms/{wall}"]}]
        has = []
        for nm, (d, im) in layers.items():
            f = d / f"{key}.{nm}.png"
            if f.exists():
                im.alpha_composite(Image.open(f).convert("RGBA"), (x, y)); has.append(nm)
        meta["tiles"][key] = {"x": x, "y": y, "group": str(Path(k).parent).replace("\\", "/"), "source": src[k],
                              "generation": g, "hand_edits": edits.get(stem, []), "layers": has}
    atlas.save(out / f"{a.name}.png"); atlas.save(out / f"{a.name}.webp", lossless=True)
    for nm, (d, im) in layers.items():
        im.save(out / f"{a.name}.{nm}.png"); im.save(out / f"{a.name}.{nm}.webp", lossless=True)
    json.dump(meta, open(out / f"{a.name}.json", "w", encoding="utf-8"), indent=1, ensure_ascii=False)
    sz = (out / f"{a.name}.webp").stat().st_size
    print(f"{n} tiles, atlas {atlas.size}, webp {sz // 1024} KB, {n - unknown} traced to a generation log, {unknown} to a pick only")


if __name__ == "__main__":
    main()
