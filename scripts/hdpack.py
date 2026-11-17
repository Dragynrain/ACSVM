"""Build, check and list the hashes for HD packs (docs/client.md 5).

Usage:
  python scripts/hdpack.py build ATLAS.png MANIFEST.json OUT.png
  python scripts/hdpack.py check PACK.png
  python scripts/hdpack.py hashes DUMP.json [--blank]

build   writes ATLAS with MANIFEST (the JSON object of client.md 5) in the
        iTXt chunk `acsvm-hd-pack`, after the same checks the viewer makes.
check   reads a pack back: its manifest, tile size and places.
hashes  prints each picture's hash from `acs dump FILE -o DUMP.json`, computed
        here from the client.md rule (a second implementation of it); blank
        pictures (one colour) are left out unless --blank.

A real adventure's dump and hash list come from its pictures: keep them
outside the repo. Needs Pillow.
"""
import argparse, json, sys
from PIL import Image, PngImagePlugin

KEYWORD = "acsvm-hd-pack"
VERSION = 1


def fnv1a64(data):
    h = 0xCBF29CE484222325
    for b in data:
        h = ((h ^ b) * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return h


def check_manifest(m, size):
    w, h = size
    if m.get("format") != KEYWORD or m.get("version") != VERSION:
        sys.exit(f"format {m.get('format')} version {m.get('version')}, not {KEYWORD} {VERSION}")
    t = m.get("tile")
    if not isinstance(t, int) or not 16 <= t <= 512 or w % t or h % t:
        sys.exit(f"tile {t} doesn't divide the {w} x {h} atlas")
    room = (w // t) * (h // t)
    for e in m.get("tiles", []):
        hx = e.get("hash", "")
        try:
            int(hx, 16)
        except ValueError:
            hx = ""
        if len(hx) != 16:
            sys.exit(f"hash {e.get('hash')!r} is not 16 hex digits")
        if not 0 <= e.get("at", -1) < room:
            sys.exit(f"place {e.get('at')} is past the atlas's {room}")
    return room


def build(args):
    img = Image.open(args.atlas).convert("RGBA")
    m = json.load(open(args.manifest, encoding="utf-8-sig"))
    m.setdefault("format", KEYWORD)
    m.setdefault("version", VERSION)
    room = check_manifest(m, img.size)
    info = PngImagePlugin.PngInfo()
    info.add_itxt(KEYWORD, json.dumps(m, ensure_ascii=False))
    img.save(args.out, pnginfo=info)
    print(f"{args.out}: {len(m['tiles'])} entries, {room} places of {m['tile']} px")


def check(args):
    img = Image.open(args.pack)
    text = getattr(img, "text", {}).get(KEYWORD)
    if text is None:
        sys.exit(f"{args.pack}: no {KEYWORD} chunk: not a pack")
    m = json.loads(text)
    room = check_manifest(m, img.size)
    used = sorted({e["at"] for e in m["tiles"]})
    print(f"{m['name']} ({m.get('license') or 'no license given'}): {len(m['tiles'])} entries, "
          f"{len(used)} of {room} places used, {m['tile']} px tiles")


def hashes(args):
    d = json.load(open(args.dump, encoding="utf-8-sig"))
    a = d.get("adventure", d)
    rgb = [c["rgb"] for c in a["palette"]["colors"]]
    for n, p in enumerate(a["pictures"]):
        rows = [[int(c, 16) for c in r] for r in p["rows"]]
        if not args.blank and len({v for r in rows for v in r}) <= 1:
            continue
        data = bytearray()
        for r in rows:
            for v in r:
                data += bytes((*rgb[v & 3], 255)) * p["pixel_aspect"]
        print(f"{n}\t{fnv1a64(data):016x}")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    b = sub.add_parser("build")
    b.add_argument("atlas")
    b.add_argument("manifest")
    b.add_argument("out")
    c = sub.add_parser("check")
    c.add_argument("pack")
    h = sub.add_parser("hashes")
    h.add_argument("dump")
    h.add_argument("--blank", action="store_true")
    args = ap.parse_args()
    {"build": build, "check": check, "hashes": hashes}[args.cmd](args)


if __name__ == "__main__":
    main()
