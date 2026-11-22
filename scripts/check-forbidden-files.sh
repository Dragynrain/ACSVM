#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Rejects disk images, adventure files and archives. ACS adventures and EA
# material must never enter this repository (see CONTRIBUTING.md).
#
# Usage:
#   scripts/check-forbidden-files.sh --staged   # files staged for commit (pre-commit hook)
#   scripts/check-forbidden-files.sh --all      # every tracked file (CI)
#   scripts/check-forbidden-files.sh --dir DIR  # every file under DIR (the built site, content.md 8)

set -eu

# Disk and tape images (C64, Amiga, PC), emulator snapshots (a VICE .vsf
# holds the program in memory), ROMs and archives; and by name, the Amiga
# version's program files and its extensionless adventure file.
PATTERN='\.(d64|d71|d81|g64|x64|dsk|adf|adz|dms|ipf|hdf|fpy|hrd|prg|t64|crt|vsf|rom|zip|7z|rar|lha|lzh|gz|tar)$'
NAMES='(^|/)(adventure|acs\.exe|acs\.[1-4]|acs\.dat|acs\.master|acs\.msgs)$'

# Paths unquoted: with quoting on, git writes a non-ASCII path as
# "Ch\303\242teau.d64", and the closing quote hides the extension.
case "${1:---all}" in
  --staged) files=$(git -c core.quotePath=false diff --cached --name-only --diff-filter=ACMR) ;;
  --all)    files=$(git -c core.quotePath=false ls-files) ;;
  --dir)    [ -d "${2:-}" ] || { echo "$0: --dir needs a directory" >&2; exit 2; }
            files=$(find "$2" -type f) ;;
  *) echo "usage: $0 [--staged|--all|--dir DIR]" >&2; exit 2 ;;
esac

bad=$(printf '%s\n' "$files" | grep -iE "$PATTERN|$NAMES" || true)

if [ -n "$bad" ]; then
  echo "ERROR: forbidden files (disk images, snapshots, ROMs, adventures, archives):" >&2
  printf '%s\n' "$bad" | sed 's/^/  /' >&2
  echo "These must never be committed. Keep research material outside the repo." >&2
  exit 1
fi
echo "forbidden-file check: OK"
