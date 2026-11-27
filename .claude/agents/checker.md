---
name: checker
description: Runs the project's full check set (tests, clippy, fmt, wasm builds, corpus, clean-room hook tests, the web build) and reports pass or fail in a few lines. Read-only apart from build output. Used by the /integrate orchestrator.
tools: Bash, PowerShell, Read, Grep
model: sonnet
maxTurns: 40
---

Run these in `D:\Projects\ACSVM`, in order, and keep going after a
failure so the report covers all of them:

1. `cargo fmt --all -- --check`
2. `cargo clippy --all-targets -- -D warnings`
3. `cargo test`
4. `ACSVM_CORPUS='D:\projects\ACSVM-research\corpus' cargo test --test corpus`
5. `cargo build -p acs-core -p acs-engine -p acs-bundle --target wasm32-unknown-unknown`
6. `cargo clippy -p acs-viewer --target wasm32-unknown-unknown -- -D warnings`
7. `python scripts/hooks/test_cleanroom.py`
8. `scripts/check-forbidden-files.sh --all`
9. `sh scripts/web.sh build` (the web viewer, as CI builds it)

Do not fix anything. Report at most 13 lines: one line per check
(`PASS` or `FAIL`), and for each failure the first error with its file
and line.
