#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Rejects disk images, adventure files and archives. ACS adventures and EA
# material must never enter this repository (see CONTRIBUTING.md).
#
# Usage:
#   scripts/check-forbidden-files.sh --staged   # files staged for commit (pre-commit hook)
#   scripts/check-forbidden-files.sh --all      # every tracked file (CI)

set -eu

PATTERN='\.(d64|dsk|adf|fpy|hrd|prg|t64|crt|zip)$'

case "${1:---all}" in
  --staged) files=$(git diff --cached --name-only --diff-filter=ACMR) ;;
  --all)    files=$(git ls-files) ;;
  *) echo "usage: $0 [--staged|--all]" >&2; exit 2 ;;
esac

bad=$(printf '%s\n' "$files" | grep -iE "$PATTERN" || true)

if [ -n "$bad" ]; then
  echo "ERROR: forbidden file types (disk images, adventures, archives):" >&2
  printf '  %s\n' $bad >&2
  echo "These must never be committed. Keep research material outside the repo." >&2
  exit 1
fi
echo "forbidden-file check: OK"
