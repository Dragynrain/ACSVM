# SPDX-License-Identifier: GPL-3.0-or-later
"""Install the research-side kit into the research folder.

    python scripts/research-kit/install.py [RESEARCH_DIR]

Copies agents/, skills/ and settings.json into RESEARCH_DIR/.claude/
(default D:/Projects/ACSVM-research) and makes RESEARCH_DIR/queue/. The
kit's master copy is here in the repo; rerun after changing it. See
docs/research/README.md 7.
"""
from __future__ import annotations

import shutil
import sys
from pathlib import Path

KIT = Path(__file__).resolve().parent


def main() -> int:
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else "D:/Projects/ACSVM-research")
    if not dest.is_dir():
        print(f"no such folder: {dest}", file=sys.stderr)
        return 1
    claude = dest / ".claude"
    for sub in ("agents", "skills"):
        src = KIT / sub
        for f in src.rglob("*"):
            if f.is_file():
                out = claude / sub / f.relative_to(src)
                out.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(f, out)
                print(f"  {out}")
    shutil.copy2(KIT / "settings.json", claude / "settings.json")
    print(f"  {claude / 'settings.json'}")
    (dest / "queue").mkdir(exist_ok=True)
    print(f"  {dest / 'queue'}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
