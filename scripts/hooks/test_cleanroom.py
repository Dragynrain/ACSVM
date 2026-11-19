# SPDX-License-Identifier: GPL-3.0-or-later
"""Cases for the clean-room hook. Run: python scripts/hooks/test_cleanroom.py"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from cleanroom import check  # noqa: E402

REPO = "D:\\Projects\\ACSVM"
RES = "D:\\Projects\\ACSVM-research"
NOTES = RES + "\\research\\encounters\\notes\\trace.txt"
FINDINGS = RES + "\\research\\encounters\\findings.md"
AMIGA = RES + "\\corpus\\amiga\\AdventureConstructionSet\\data"


def call(tool, cwd=REPO, **ti):
    return {"tool_name": tool, "tool_input": ti, "cwd": cwd}


# (mode, payload, blocked?)
CASES = [
    # implementer: notes are off limits however they are reached
    ("implementer", call("Read", file_path=NOTES), True),
    ("implementer", call("Read", file_path="/d/Projects/ACSVM-research/research/combat/notes/a.md"), True),
    ("implementer", call("Read", file_path=FINDINGS), False),
    ("implementer", call("Read", file_path=RES + "\\research\\encounters\\inputs\\runs.jsonl"), False),
    ("implementer", call("Glob", pattern="**/*.md", path=RES), True),
    ("implementer", call("Glob", pattern="research/*/findings.md", path=RES), False),
    ("implementer", call("Glob", pattern="research/*/*", path=RES), True),
    ("implementer", call("Grep", pattern="roll", path=RES + "\\research\\combat"), True),
    ("implementer", call("Grep", pattern="roll", path=RES + "\\research\\combat\\findings.md"), False),
    ("implementer", call("Grep", pattern="roll", path=RES + "\\research\\combat\\inputs"), False),
    ("implementer", call("Grep", pattern="roll"), False),
    ("implementer", call("Bash", command="cat /d/Projects/ACSVM-research/research/combat/notes/x.txt"), True),
    ("implementer", call("Bash", command="ls /d/Projects/ACSVM-research/research/combat"), False),
    ("implementer", call("Bash", command="ls /d/Projects/ACSVM-research/research/combat/*"), True),
    ("implementer", call("Bash", command="grep -r roll /d/Projects/ACSVM-research/research"), True),
    ("implementer", call("Bash", command="grep -rn roll /d/Projects/ACSVM-research/capture/runs"), False),
    ("implementer", call("Bash", command="cd /d/Projects/ACSVM-research/research/combat && cat notes/x"), True),
    ("implementer", call("PowerShell", command="Get-ChildItem -Recurse D:\\Projects\\ACSVM-research"), True),
    ("implementer", call("Bash", command="cargo test --test corpus -- --nocapture"), False),
    ("implementer", call("Bash", command="python scripts/combatcheck.py /d/Projects/ACSVM-research/capture/runs/rr13/runs.jsonl"), False),
    ("implementer", call("Read", file_path=RES + "\\corpus\\c64\\x\\advcnst1.d64"), True),
    ("implementer", call("Bash", command="ls /d/Projects/ACSVM-research/corpus/c64/*/advcnst*"), True),
    ("implementer", call("Edit", file_path=REPO + "\\docs\\rules\\combat.md"), False),
    # implementer: the Amiga program files and the Kickstart ROM
    ("implementer", call("Read", file_path=AMIGA + "\\ACS.EXE"), True),
    ("implementer", call("Read", file_path=AMIGA + "\\acs.3"), True),
    ("implementer", call("Read", file_path=AMIGA + "\\c\\ACS.dat"), True),
    ("implementer", call("Read", file_path=RES + "\\corpus\\amiga\\kick13.rom"), True),
    ("implementer", call("Read", file_path=AMIGA + "\\adventure"), False),
    ("implementer", call("Grep", pattern="x", path=RES + "\\corpus\\amiga"), True),
    ("implementer", call("Grep", pattern="x", path=RES + "\\corpus"), True),
    ("implementer", call("Glob", pattern="corpus/amiga/**", path=RES), True),
    ("implementer", call("Bash", command="xxd /d/Projects/ACSVM-research/corpus/amiga/AdventureConstructionSet/data/acs.1"), True),
    ("implementer", call("Bash", AMIGA, command="cat ACS.EXE"), True),
    ("implementer", call("Bash", command="ls /d/Projects/ACSVM-research/corpus/amiga/AdventureConstructionSet/data/*"), True),
    ("implementer", call("Bash", command="ls /d/Projects/ACSVM-research/corpus/amiga/AdventureConstructionSet/data"), False),
    ("implementer", call("Bash", command="ACSVM_CORPUS='D:\\projects\\ACSVM-research\\corpus' cargo test --test corpus 2>&1 | sort -rn"), False),
    ("implementer", call("Bash", command="grep -r x /d/Projects/ACSVM-research/corpus/amiga"), True),
    ("researcher", call("Read", RES, file_path=AMIGA + "\\ACS.EXE"), False),
    ("researcher", call("Bash", RES, command="xxd corpus/amiga/kick13.rom"), False),
    # spec-only: the engine implementer sees docs/rules, not findings or briefs
    ("spec-only", call("Read", file_path=FINDINGS), True),
    ("spec-only", call("Read", file_path=RES + "\\research\\encounters\\inputs\\a.png"), True),
    ("spec-only", call("Read", file_path=REPO + "\\docs\\research\\encounters.md"), True),
    ("spec-only", call("Read", file_path=REPO + "\\docs\\research\\README.md"), False),
    ("spec-only", call("Read", file_path=REPO + "\\docs\\rules\\encounters.md"), False),
    ("spec-only", call("Edit", file_path=REPO + "\\crates\\acs-engine\\src\\step.rs"), False),
    ("spec-only", call("Grep", pattern="Encounter", path=REPO + "\\crates"), False),
    ("spec-only", call("Grep", pattern="Encounter", path=RES), True),
    ("spec-only", call("Bash", command="cat /d/Projects/ACSVM-research/research/encounters/findings.md"), True),
    ("spec-only", call("Bash", command="cat docs/research/encounters.md"), True),
    ("spec-only", call("Bash", command="cargo test -p acs-engine"), False),
    # the engine implementer subagent is spec-only under the project hook too
    ("implementer", dict(call("Read", file_path=FINDINGS), agent_type="engine-implementer"), True),
    ("implementer", dict(call("Read", file_path=FINDINGS), agent_type="capture-confirmer"), False),
    ("implementer", call("Glob", pattern="research/**/*.md", path=RES), True),
    ("implementer", call("Glob", pattern="research/combat/inputs/**", path=RES), False),
    # researcher: read the repo's docs only, never write or run commands there
    ("researcher", call("Read", RES, file_path=REPO + "\\docs\\research\\encounters.md"), False),
    ("researcher", call("Read", RES, file_path=REPO + "\\crates\\acs-engine\\src\\step.rs"), True),
    ("researcher", call("Grep", RES, pattern="x", path=REPO), True),
    ("researcher", call("Grep", RES, pattern="x", path=REPO + "\\docs"), False),
    ("researcher", call("Write", RES, file_path=REPO + "\\docs\\rules\\x.md"), True),
    ("researcher", call("Write", RES, file_path=FINDINGS), False),
    ("researcher", call("Read", RES, file_path=NOTES), False),
    ("researcher", call("Bash", RES, command="git -C /d/Projects/ACSVM log"), True),
    ("researcher", call("Bash", RES, command="cat ../ACSVM/crates/acs-engine/src/magic.rs"), True),
    ("researcher", call("Bash", RES, command="python research/encounters/notes/dis.py corpus/c64/x/advcnst1.d64"), False),
    ("researcher", call("PowerShell", RES, command="Get-Content D:\\Projects\\ACSVM\\docs\\capture.md"), True),
]


def main() -> int:
    bad = 0
    for mode, payload, want in CASES:
        why = check(mode, payload)
        if bool(why) != want:
            bad += 1
            ti = payload["tool_input"]
            print(f"FAIL {mode} {payload['tool_name']} {ti}: blocked={bool(why)} ({why})")
    print(f"{len(CASES) - bad}/{len(CASES)} cases pass")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
