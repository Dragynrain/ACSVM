# SPDX-License-Identifier: GPL-3.0-or-later
"""Clean-room guard: a PreToolUse hook (docs/research/README.md 4 and 7).

Reads the hook payload on stdin and denies tool calls that would break the
clean room. One mode per role, given as the first argument:

  implementer  every session in the repo: never touch research/*/notes/
               (no reads, no recursive search or listing that would reach
               them), never touch EA's program disks (advcnst*), the
               Amiga version's program files (ACS.EXE, acs.1 to acs.4,
               ACS.dat), its disk images (AdventureConstructionSet_*.adf)
               or the Kickstart ROM, and no wildcard or recursive
               search that would reach them under corpus/amiga/; nor
               the rpgcodex "00 Main Master Disks" folders (EA's disks
               under other names), by name like advcnst*.
  spec-only    the engine implementer: as implementer, and nothing under
               ACSVM-research/research/ at all (no findings, no inputs) and
               no research briefs: it implements from docs/rules/ only.
  researcher   sessions in the research folder: never write into the repo,
               read only its docs/, and no shell command that names it.

The checks are a second line behind the instructions, not a sandbox: they
look at paths in tool inputs and at path-like words in shell commands.
Test cases: scripts/hooks/test_cleanroom.py.
"""
from __future__ import annotations

import fnmatch
import json
import os
import re
import shlex
import sys

REPO = "d:/projects/acsvm"
RESEARCH = "d:/projects/acsvm-research"

PATH_FIELDS = ("file_path", "path", "notebook_path")
SHELL_TOOLS = ("Bash", "PowerShell")
WRITE_TOOLS = ("Write", "Edit", "NotebookEdit")
SPEC_ONLY_AGENTS = ("engine-implementer",)

RECURSIVE = re.compile(
    r"(^|[\s;|&(])(rg|find|tree|du|fd|git\s+grep)\b"
    r"|\s-[a-zA-Z]*[rR][a-zA-Z]*\b|--recursive|-recurse\b|\s/s\b",
    re.IGNORECASE,
)


def norm(p: str, cwd: str) -> str:
    """Absolute, lower case, forward slashes, d:/ drive form."""
    p = p.strip().strip("'\"").replace("\\", "/")
    m = re.match(r"^/([a-zA-Z])(/|$)", p)  # Git Bash /d/...
    if m:
        p = m.group(1) + ":/" + p[3:]
    if not re.match(r"^[a-zA-Z]:/", p):
        p = cwd.replace("\\", "/").rstrip("/") + "/" + p
    drive, rest = p[:2], p[2:]
    parts: list[str] = []
    for seg in rest.split("/"):
        if seg in ("", "."):
            continue
        if seg == ".." and parts:
            parts.pop()
        elif seg != "..":
            parts.append(seg)
    return (drive + "/" + "/".join(parts)).lower()


def under(p: str, root: str) -> list[str] | None:
    """Segments of p below root, or None when p is not inside root."""
    if p == root:
        return []
    if p.startswith(root + "/"):
        return p[len(root) + 1:].split("/")
    return None


def matches_notes(seg: str) -> bool:
    return fnmatch.fnmatch("notes", seg) or seg == "notes"


def notes_risk(p: str, recursive: bool) -> str | None:
    """Why p (in the research folder) could reach a notes folder."""
    rel = under(p, RESEARCH)
    if rel is None:
        return None
    if len(rel) >= 3 and rel[0] == "research" and matches_notes(rel[2]):
        return "a research notes folder"
    if len(rel) >= 2 and rel[0] == "research" and rel[1] == "notes":
        return "a research notes folder"
    if recursive:
        # The root, research/ and research/TOPIC/ all contain notes below.
        if len(rel) == 0 or (rel[0] == "research" and len(rel) <= 2):
            if len(rel) == 2 and not any(ch in rel[1] for ch in "*?["):
                return "a recursive search over a research topic (its notes are inside)"
            return "a recursive search over the research folder (notes are inside)"
    return None


AMIGA = RESEARCH + "/corpus/amiga"
# EA's Amiga program files, its release disks (which hold them) and the
# Kickstart ROM, by file name.
AMIGA_PROGRAM = re.compile(
    r"^(acs\.exe|acs\.[1-4]|acs\.dat|kick[^/]*\.rom|adventureconstructionset[^/]*\.adf)$"
)
AMIGA_WHY = "EA's Amiga program files, its disks and the Kickstart ROM are for researcher sessions only"


# The rpgcodex collection's "00 Main Master Disks" folders (C64 and Amiga):
# EA's own disks under other names (the C64 program disk, Rivers of Light,
# the builder sets, "... (Main Master).d64", "... (Master).d64") and a
# blank Amiga disk. Matched on the lower-case, forward-slash text, with up to
# three other characters between the words (`main\ master`, `main*master`).
MASTER = re.compile(r"main\W{1,3}master|\(master\)\.d64")
MASTER_WHY = "the corpus's Main Master disks are EA's own, for researcher sessions only"


def amiga_risk(p: str, recursive: bool, shell: bool = False) -> str | None:
    """Why p could reach the Amiga program files or the Kickstart ROM. In a
    shell command a recursive flag may belong to another part of a pipe
    (`sort -rn`), so there only paths inside corpus/amiga/ count."""
    seg = p.rsplit("/", 1)[-1]
    if AMIGA_PROGRAM.match(seg):
        return AMIGA_WHY
    rel = under(p, AMIGA)
    if rel is not None and (recursive or any(ch in seg for ch in "*?[")):
        return AMIGA_WHY
    if recursive and not shell and p == RESEARCH + "/corpus":
        return AMIGA_WHY
    return None


def words(cmd: str) -> list[str]:
    try:
        toks = shlex.split(cmd, posix=False)
    except ValueError:
        toks = cmd.split()
    out = []
    for t in toks:
        out.extend(x for x in re.split(r"[;|&<>()=,]", t) if x)
    return [t.strip("'\"") for t in out]


def tool_paths(tool: str, ti: dict, cwd: str) -> list[tuple[str, bool]]:
    """(path, recursive) pairs. Grep searches its path recursively; Glob
    reaches only what its pattern names, recursively below a ** segment."""
    if tool == "Glob":
        pat = str(ti.get("pattern") or "")
        base = ti.get("path") or cwd
        full = norm(pat if re.match(r"^([a-zA-Z]:|/)", pat) else str(base).rstrip("/\\") + "/" + pat, cwd)
        if "**" in full:
            return [(full.split("**")[0].rstrip("/"), True), (full, False)]
        return [(full, False)]
    raw = [ti[f] for f in PATH_FIELDS if isinstance(ti.get(f), str) and ti.get(f)]
    if tool == "Grep" and not ti.get("path"):
        raw.append(cwd)
    return [(norm(r, cwd), tool == "Grep") for r in raw]


def check(mode: str, data: dict) -> str | None:
    # Inside a subagent the payload names its type: the engine implementer
    # is held to spec-only by the project-wide hook too, not only by its
    # own frontmatter hook (which needs the workspace trusted).
    if mode == "implementer" and data.get("agent_type") in SPEC_ONLY_AGENTS:
        mode = "spec-only"
    tool = data.get("tool_name", "")
    ti = data.get("tool_input") or {}
    cwd = data.get("cwd") or os.getcwd()

    if tool in SHELL_TOOLS:
        cmd = str(ti.get("command", ""))
        low = cmd.lower().replace("\\", "/")
        recursive = bool(RECURSIVE.search(cmd))
        toks = words(cmd)
        paths = [norm(t, cwd) for t in toks if "/" in t.replace("\\", "/") or t.lower().startswith(("d:", "notes"))]
        if mode in ("implementer", "spec-only"):
            if "advcnst" in low:
                return "EA's program disks (advcnst*) are for researcher sessions only"
            if MASTER.search(low):
                return MASTER_WHY
            if any(AMIGA_PROGRAM.match(t.lower().replace("\\", "/").rsplit("/", 1)[-1]) for t in toks):
                return AMIGA_WHY
            names_research = "acsvm-research" in low or under(norm(".", cwd), RESEARCH) is not None
            for p in paths:
                why = notes_risk(p, recursive) or amiga_risk(p, recursive, shell=True)
                if why:
                    return why
            if names_research and any(matches_notes(seg) for t in toks for seg in t.replace("\\", "/").split("/") if seg):
                return "a command in the research folder that names a notes folder"
            if mode == "spec-only" and ("acsvm-research/research" in low or any(under(p, RESEARCH + "/research") is not None for p in paths)):
                return "the engine implementer works from docs/rules/ only, not the research folder"
            if mode == "spec-only" and "docs/research/" in low and "docs/research/readme.md" not in low:
                return "the engine implementer works from docs/rules/ only, not research briefs"
        if mode == "researcher":
            for p in paths:
                if under(p, REPO) is not None:
                    return "researchers do not run commands on the ACSVM repository; read its docs/ with the Read tool"
            if re.search(r"d:/projects/acsvm(?!-research)\b|/d/projects/acsvm(?!-research)\b", low):
                return "researchers do not run commands on the ACSVM repository; read its docs/ with the Read tool"
        return None

    for p, recursive in tool_paths(tool, ti, cwd):
        if mode in ("implementer", "spec-only"):
            if "advcnst" in p:
                return "EA's program disks (advcnst*) are for researcher sessions only"
            if MASTER.search(p):
                return MASTER_WHY
            why = notes_risk(p, recursive) or amiga_risk(p, recursive)
            if why:
                return why
            if mode == "spec-only":
                if under(p, RESEARCH + "/research") is not None or (recursive and under(RESEARCH + "/research", p) is not None and under(p, REPO) is None):
                    return "the engine implementer works from docs/rules/ only, not the research folder"
                rel = under(p, REPO)
                if rel and rel[:2] == ["docs", "research"] and rel[2:] != ["readme.md"]:
                    return "the engine implementer works from docs/rules/ only, not research briefs"
        elif mode == "researcher":
            rel = under(p, REPO)
            if rel is None:
                continue
            if tool in WRITE_TOOLS:
                return "researchers never write to the ACSVM repository"
            if not rel or rel[0] != "docs":
                return "researchers read only the repository's docs/ (never crates/ or the rest)"
    return None


def main() -> int:
    mode = sys.argv[1] if len(sys.argv) > 1 else "implementer"
    try:
        data = json.loads(sys.stdin.buffer.read().decode("utf-8", errors="replace"))
    except Exception:
        return 0
    why = check(mode, data)
    if not why:
        return 0
    reason = (
        f"BLOCKED by the clean-room hook ({mode}): {why}. "
        "See docs/research/README.md sections 4 and 7. Do not work around this; "
        "if the task needs it, stop and report to the maintainer."
    )
    print(json.dumps({"hookSpecificOutput": {
        "hookEventName": "PreToolUse",
        "permissionDecision": "deny",
        "permissionDecisionReason": reason,
    }}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
