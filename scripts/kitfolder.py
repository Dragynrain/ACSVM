"""A kit folder's art from a packed kit (docs/content.md 4.1).

Usage: python kitfolder.py PACK.json PACK.png OUTDIR [--skip kp] [--name NAME]
       [--licence "CC BY-SA 4.0"] [--credit TEXT ...]

Writes OUTDIR/atlas.png (the kept tiles, in pack order, packed again),
OUTDIR/kit.toml (name, licence, credits, atlas, tile size and the picture
list) and OUTDIR/provenance.json (each picture's record from the pack:
source, generation chain, hand edits, licence). things.toml and
creatures.toml are written by hand and left alone.

Picture keys come from tile names: kit/axe-501 -> axe (the seed suffix
dropped), terrain/deepsea -> terrain_deepsea, rooms/hull -> room_hull,
rooms/doors/woodendoor-hull -> door_woodendoor_hull, variants/crab-blue ->
crab_blue.
"""
import argparse, json, re
from pathlib import Path
from PIL import Image

PER_ROW = 16


def key_of(name):
    group, _, stem = name.rpartition("/")
    stem = stem.replace("-", "_")
    if group in ("kit", "kit2", "player", "kp"):
        return re.sub(r"_\d+$", "", stem)
    if group == "terrain":
        return "terrain_" + stem
    if group == "rooms":
        return "room_" + stem
    if group == "rooms/doors":
        return "door_" + stem
    return stem


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("pack_json")
    ap.add_argument("pack_png")
    ap.add_argument("out")
    ap.add_argument("--skip", action="append", default=[])
    ap.add_argument("--name", default="pirates")
    ap.add_argument("--licence", default="CC BY-SA 4.0")
    ap.add_argument("--credit", action="append", default=[])
    a = ap.parse_args()

    pack = json.load(open(a.pack_json, encoding="utf-8"))
    tile = pack["tile"]
    src = Image.open(a.pack_png).convert("RGBA")
    kept = [(n, t) for n, t in pack["tiles"].items() if t["group"] not in a.skip]
    keys = {}
    for n, _ in kept:
        k = key_of(n)
        if not re.fullmatch(r"[a-z0-9_]+", k):
            raise SystemExit(f"{n}: key {k!r} is not lowercase letters, digits and _")
        if k in keys:
            raise SystemExit(f"{n} and {keys[k]} both make the key {k!r}")
        keys[k] = n

    rows = (len(kept) + PER_ROW - 1) // PER_ROW
    atlas = Image.new("RGBA", (PER_ROW * tile, rows * tile), (0, 0, 0, 0))
    provenance = {}
    lines = []
    for at, (n, t) in enumerate(kept):
        box = (t["x"], t["y"], t["x"] + tile, t["y"] + tile)
        atlas.paste(src.crop(box), ((at % PER_ROW) * tile, (at // PER_ROW) * tile))
        k = key_of(n)
        lines.append(f"{k} = {at}")
        rec = {x: t[x] for x in ("group", "source", "generation", "hand_edits") if x in t}
        rec["tile"] = n
        rec["licence"] = pack.get("licence", a.licence)
        provenance[k] = rec

    out = Path(a.out)
    out.mkdir(parents=True, exist_ok=True)
    atlas.save(out / "atlas.png", optimize=True)
    json.dump(provenance, open(out / "provenance.json", "w", encoding="utf-8"), indent=1, ensure_ascii=False)
    credits = ", ".join(json.dumps(c) for c in a.credit)
    toml = [
        "# Written by scripts/kitfolder.py from the packed kit; edit the pack, not this file.",
        f"name = {json.dumps(a.name)}",
        f"license = {json.dumps(a.licence)}",
        f"credits = [{credits}]",
        'atlas = "atlas.png"',
        f"tile = {tile}",
        "",
        "[pictures]",
        *lines,
        "",
    ]
    (out / "kit.toml").write_text("\n".join(toml), encoding="utf-8")
    print(f"{len(kept)} pictures, atlas {atlas.size[0]} x {atlas.size[1]}, in {out}")


if __name__ == "__main__":
    main()
