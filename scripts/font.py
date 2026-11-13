"""Build the pirate kit UI font from its text source.

Usage: python font.py SOURCE.txt OUTDIR [--scale N] [--text "sample"]

SOURCE: blocks of '= C' (C a character, or 'space') followed by equal rows of
'.' and '#' (see docs/art/pirates-font.txt). Writes to OUTDIR:
  font.png          atlas, printable ASCII and Latin-1 in 16 columns, white on clear
  font-outline.png  the same with a 1 px dark outline (text over pictures)
  font-bold.png     bold display weight (each glyph overlaid 1 px right, 1 px wider),
                    and font-bold-outline.png; boxes in font.json under "bold"
  font.json         per glyph: x, y, w in the atlas; cell height, baseline, advance gap
  font.bdf          bitmap font for other tools
  preview.png       sample text at --scale in the kit's ink colours
"""
import argparse, json
from pathlib import Path
from PIL import Image

GAP = 1        # px between glyphs
BASELINE = 9   # rows above the baseline (row 0 holds accents on capitals)
LINE = 12      # line height


def load(path):
    glyphs, cur = {}, None
    for line in Path(path).read_text(encoding="utf-8").splitlines():
        if line.startswith("# ") or not line.strip():  # comments; glyph rows have no spaces
            cur = None
            continue
        if line.startswith("= "):
            name = line[2:]
            cur = " " if name == "space" else chr(int(name[2:], 16)) if name.startswith("U+") else name
            glyphs[cur] = []
        elif cur is not None:
            glyphs[cur].append(line)
    h = {len(r) for r in glyphs.values()}
    assert len(h) == 1, f"glyph heights differ: {h}"
    for c, rows in glyphs.items():
        assert len({len(r) for r in rows}) == 1, f"ragged glyph {c!r}"
    return glyphs, h.pop()


def embolden(glyphs):
    """Bold weight: each glyph ORed with itself shifted 1 px right, 1 px wider."""
    out = {}
    for c, rows in glyphs.items():
        out[c] = ["".join("#" if (r + ".")[i] == "#" or ("." + r)[i] == "#" else "." for i in range(len(r) + 1))
                  for r in rows]
    return out


def draw(text, glyphs, h, ink, outline=None):
    lines = text.split("\n")
    width = max(sum(len(glyphs[c][0]) + GAP for c in ln) for ln in lines) + 2
    im = Image.new("RGBA", (width, LINE * len(lines) + 2), (0, 0, 0, 0)); px = im.load()
    for li, ln in enumerate(lines):
        x = 1
        for c in ln:
            rows = glyphs[c]
            for y, r in enumerate(rows):
                for dx, v in enumerate(r):
                    if v == "#":
                        px[x + dx, 1 + li * LINE + y] = ink
            x += len(rows[0]) + GAP
    if outline:
        a = im.copy(); ap = a.load()
        for y in range(im.height):
            for x in range(im.width):
                if ap[x, y][3] == 0 and any(0 <= x + i < im.width and 0 <= y + j < im.height and ap[x + i, y + j][3]
                                            for i, j in ((1, 0), (-1, 0), (0, 1), (0, -1))):
                    px[x, y] = outline
    return im


def main():
    ap = argparse.ArgumentParser(); ap.add_argument("source"); ap.add_argument("outdir")
    ap.add_argument("--scale", type=int, default=4)
    ap.add_argument("--text", default="THE KING'S PARDON\nThe quick brown fox jumps over the lazy dog.\n"
                    "Sabine Duroc gives you 12 doubloons (and a map)!\nLife Force 18/20  Gold 340  [Ready: cutlass]\n"
                    "Ça, señor! Garçon, où est l'équipage? Größe Æsir Øresund ¿Qué? «déjà vu» £5 ±3°")
    ap.add_argument("--title", default="The King's Pardon")
    a = ap.parse_args(); out = Path(a.outdir); out.mkdir(parents=True, exist_ok=True)
    glyphs, h = load(a.source)
    chars = [chr(c) for c in list(range(32, 127)) + list(range(160, 256))]
    missing = [c for c in chars if c not in glyphs]
    assert not missing, f"missing glyphs: {missing}"
    bold = embolden(glyphs)
    cw = max(len(g[0]) for g in bold.values()) + 2
    cols, rows = 16, (len(chars) + 15) // 16
    meta = {"height": h, "baseline": BASELINE, "line": LINE, "gap": GAP, "glyphs": {}, "bold": {}}
    for name, gs, ol in (("font", glyphs, None), ("font-outline", glyphs, (58, 26, 18, 255)),
                         ("font-bold", bold, None), ("font-bold-outline", bold, (58, 26, 18, 255))):
        atlas = Image.new("RGBA", (cols * cw, rows * (h + 2)), (0, 0, 0, 0))
        for i, c in enumerate(chars):
            x, y = (i % cols) * cw, (i // cols) * (h + 2)
            g = draw(c, gs, h, (255, 255, 255, 255), ol).crop((0, 0, len(gs[c][0]) + 2, h + 2))
            atlas.alpha_composite(g, (x, y))
            if ol is None:
                meta["glyphs" if gs is glyphs else "bold"][c] = {"x": x + 1, "y": y + 1, "w": len(gs[c][0])}
        atlas.save(out / f"{name}.png")
    json.dump(meta, open(out / "font.json", "w", encoding="utf-8"), indent=1, ensure_ascii=False)
    bdf = ["STARTFONT 2.1", "FONT -acsvm-pirates-medium-r-normal--10-100-75-75-p-50-iso10646-1",
           f"SIZE {h} 75 75", f"FONTBOUNDINGBOX {cw} {h} 0 {BASELINE - h}",
           "STARTPROPERTIES 2", f"FONT_ASCENT {BASELINE}", f"FONT_DESCENT {h - BASELINE}", "ENDPROPERTIES",
           f"CHARS {len(chars)}"]
    for c in chars:
        rows_ = glyphs[c]; w = len(rows_[0])
        bdf += [f"STARTCHAR U+{ord(c):04X}", f"ENCODING {ord(c)}", f"SWIDTH {(w + GAP) * 100} 0",
                f"DWIDTH {w + GAP} 0", f"BBX {w} {h} 0 {BASELINE - h}", "BITMAP"]
        for r in rows_:
            v = int(r.replace(".", "0").replace("#", "1").ljust(8 * ((w + 7) // 8), "0"), 2)
            bdf.append(f"{v:0{2 * ((w + 7) // 8)}X}")
        bdf.append("ENDCHAR")
    bdf.append("ENDFONT")
    (out / "font.bdf").write_text("\n".join(bdf) + "\n", encoding="utf-8")
    # Preview: plain ink on parchment, and outlined light text on a dark sea blue.
    t1 = draw(a.text, glyphs, h, (58, 26, 18, 255))
    t2 = draw(a.text, glyphs, h, (251, 241, 207, 255), (28, 20, 25, 255))
    t3 = draw(a.title, bold, h, (246, 222, 74, 255), (28, 20, 25, 255))
    t3 = t3.resize((t3.width * 2, t3.height * 2), Image.NEAREST)
    pv = Image.new("RGBA", (max(t1.width, t2.width, t3.width) + 8, t1.height + t2.height + t3.height + 18),
                   (251, 241, 207, 255))
    pv.paste((30, 72, 120, 255), (0, t1.height + 6, pv.width, pv.height))
    pv.alpha_composite(t1, (4, 3)); pv.alpha_composite(t2, (4, t1.height + 9))
    pv.alpha_composite(t3, (4, t1.height + t2.height + 14))
    at = Image.open(out / "font.png"); bg = Image.new("RGBA", at.size, (30, 72, 120, 255)); bg.alpha_composite(at)
    sheet = Image.new("RGBA", (max(pv.width, bg.width), pv.height + bg.height + 4), (96, 96, 104, 255))
    sheet.paste(pv, (0, 0)); sheet.paste(bg, (0, pv.height + 4))
    sheet.resize((sheet.width * a.scale, sheet.height * a.scale), Image.NEAREST).save(out / "preview.png")
    print(len(chars), "glyphs, height", h, "->", out)


if __name__ == "__main__":
    main()
