"""Draws the roadmap image for docs/design.md.

Usage: python scripts/roadmap.py docs/images/roadmap.png
Needs Pillow and the Segoe UI fonts (Windows). Edit PHASES, then rerun.
"""
import sys
from PIL import Image, ImageDraw, ImageFont

F = "C:/Windows/Fonts/"
title_f = ImageFont.truetype(F + "segoeuib.ttf", 25)
head_f = ImageFont.truetype(F + "segoeuib.ttf", 23)
body_f = ImageFont.truetype(F + "segoeui.ttf", 19)
tag_f = ImageFont.truetype(F + "segoeuib.ttf", 15)

INK, BODY, LINE = (25, 25, 25), (85, 85, 82), (195, 194, 183)
BLUE, BLUE_BG = (42, 120, 214), (229, 239, 250)
GREEN, GREEN_BG = (46, 125, 70), (226, 242, 230)

TITLE = "Eight phases to 1.1: capture and clean-room research before combat, co-op after 1.0"
PHASES = [
    ("Phase 0 \u00b7 Foundations and inspector", "done", [
        "Repo, CI and Rust workspace; disk readers and decoders for PC and C64 (components 1 to 6).",
        "Inspector CLI dumps adventures to JSON and pictures to PNG. The first public release."],
     "Gate: every available PC and C64 adventure decodes with no errors or unknown thing types"),
    ("Phase 1 \u00b7 Browser viewer", "done", [
        "WASM build with drag-and-drop loading; walk the map, rooms and portals.",
        "Original render mode with resizing, fullscreen and zoom. Phase 2 and the art kits start alongside."],
     "Gate: walk every room and portal of a test adventure in the browser, from PC and C64 files"),
    ("Phase 2 \u00b7 Rules engine from the manual", "done", [
        "Specs from the manual and the data, guesses tagged [unknown]: movement, inventory,",
        "creature turns and AI outside combat, the 15 spell actions. Walk mode plays them in the viewer."],
     "Gate: walk, carry, cast and meet creatures in a built-in adventure in the browser"),
    ("Phase 3 \u00b7 Behavior capture harness", "now", [
        "VICE runs on scenario disks of our own, read off the screen, in parallel; clean-room research",
        "sets the random state. Settled so far: movement, weight, casting, Power drift, healing."],
     "Gate: one command runs a scenario in both and diffs them; Phase 2's guesses confirmed or fixed"),
    ("Phase 4 \u00b7 Combat, formulas and hot-seat", "next", [
        "Measure, or research and confirm, rule by rule: hit and damage, skill growth, stores, spawns,",
        "characters, saves. Random formulas get a research brief first. Hot-seat; golden tests in CI."],
     "Gate: golden tests pass, and two players can finish a built-in adventure start to finish"),
    ("Phase 5 \u00b7 Modern client and audio", None, [
        "Message log, panels, touch and accessibility; Smoothed and HD pack render modes.",
        "30 original sound and music cues; desktop builds."],
     "Gate: no layout breaks on laptop, 4K and tablet; HD packs load with a coverage report"),
    ("Phase 6 \u00b7 Original content and 1.0", None, [
        "Four 128 px starter kits (pirates first), UI font, title art, two CC BY-SA sample adventures.",
        "Public playtest with long-time ACS players."],
     "Gate: 1.0 release, playable in the browser with no EA files bundled"),
    ("Phase 7 \u00b7 Online co-op (1.1)", None, [
        "Relay server, room codes, lockstep commands, desync check, reconnect, turn timer.",
        "Built on the deterministic engine and the command log from Phase 4."],
     "Gate: a 4-player online game reaches the end with no desync"),
]
LATER = "Later \u00b7 Apple II and Amiga support, then the adventure editor (2.0)"

W, X0, X1 = 1344, 42, 1302
BOX_H, GAP = 128, 98

H = 92 + len(PHASES) * (BOX_H + GAP) + 90 + 40
im = Image.new("RGB", (W, H), "white")
d = ImageDraw.Draw(im)
d.text((X0, 30), TITLE, font=title_f, fill=INK)

def dashed_rect(box, color, dash=10, gap=7, r=14):
    d.rounded_rectangle(box, radius=r, outline="white", width=2)
    x0, y0, x1, y1 = box
    for x in range(x0 + r, x1 - r, dash + gap):
        d.line([(x, y0), (min(x + dash, x1 - r), y0)], fill=color, width=2)
        d.line([(x, y1), (min(x + dash, x1 - r), y1)], fill=color, width=2)
    for y in range(y0 + r, y1 - r, dash + gap):
        d.line([(x0, y), (x0, min(y + dash, y1 - r))], fill=color, width=2)
        d.line([(x1, y), (x1, min(y + dash, y1 - r))], fill=color, width=2)
    for (cx, cy, a) in [(x0, y0, 180), (x1 - 2 * r, y0, 270), (x1 - 2 * r, y1 - 2 * r, 0), (x0, y1 - 2 * r, 90)]:
        d.arc([cx, cy, cx + 2 * r, cy + 2 * r], a, a + 90, fill=color, width=2)

y = 92
spine_x = 85
for i, (head, status, lines, gate) in enumerate(PHASES):
    box = (X0, y, X1, y + BOX_H)
    if status == "now":
        d.rounded_rectangle(box, radius=14, fill=BLUE_BG, outline=BLUE, width=3)
    else:
        d.rounded_rectangle(box, radius=14, fill="white", outline=LINE, width=2)
    d.text((X0 + 29, y + 20), head, font=head_f, fill=INK)
    if status:
        label = {"done": "DONE", "now": "IN PROGRESS", "next": "NEXT"}[status]
        fg, bg = {"done": (GREEN, GREEN_BG), "now": ("white", BLUE), "next": (BLUE, BLUE_BG)}[status]
        hw = d.textlength(head, font=head_f)
        tw = d.textlength(label, font=tag_f)
        tx = X0 + 29 + hw + 16
        d.rounded_rectangle((tx, y + 25, tx + tw + 18, y + 49), radius=11, fill=bg)
        d.text((tx + 9, y + 27), label, font=tag_f, fill=fg)
    for j, line in enumerate(lines):
        d.text((X0 + 29, y + 58 + j * 28), line, font=body_f, fill=BODY)
    # spine and gate
    top, bot = y + BOX_H, y + BOX_H + GAP
    d.line([(spine_x, top), (spine_x, bot)], fill=LINE, width=2)
    gy = (top + bot) // 2
    s = 21
    d.polygon([(spine_x, gy - s), (spine_x + s, gy), (spine_x, gy + s), (spine_x - s, gy)], fill=LINE)
    d.text((128, gy - 14), gate, font=body_f, fill=INK)
    y = bot

dashed_rect((X0, y, X1, y + 86), LINE)
d.text((X0 + 29, y + 28), LATER, font=ImageFont.truetype(F + "segoeui.ttf", 21), fill=BODY)
im.save(sys.argv[1], optimize=True)
