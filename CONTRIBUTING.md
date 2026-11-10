# Contributing

Thanks for helping. This project reimplements a commercial program's file
format and rules, so a few rules protect it legally. Please read them before
your first change.

## 1. No EA material, ever

Never commit, attach or paste:

- disk images or adventure files (`.d64`, `.dsk`, `.adf`, `.fpy`, `.hrd`,
  `.prg`, `.t64`, `.crt`, `.zip`), including EA's built-in adventures and the
  ACS program disks;
- bytes copied out of such files, including as test fixtures or hex dumps;
- text from the ACS manual (paraphrase rules in your own words, never quote);
- builder-set art, pictures exported from real adventures, or music.

Once something is pushed to a public repository it survives in clones and
forks, so this is enforced mechanically as well:

```bash
git config core.hooksPath .githooks   # once per clone: enables the pre-commit check
```

The same check (`scripts/check-forbidden-files.sh`) runs in CI.

## 2. Tests

- Unit and synthetic tests build their input byte by byte
  (`crates/acs-core/tests/common`). Write fixtures from the layout
  constants, never from real adventure bytes.
- Tests against real adventures read a corpus folder outside the repository
  through `ACSVM_CORPUS` and must skip cleanly when it is unset.
- Run `cargo fmt`, `cargo clippy --all-targets -- -D warnings` and
  `cargo test` before sending a change.

## 3. Clean-room policy

There are three kinds of knowledge, handled differently:

1. **File-format facts** (where a field is, how text is packed). Facts are not
   copyrightable. Learn them from documented sources, from your own analysis of
   adventure files you lawfully have, or from ACS Viewer. Record findings in
   `docs/format-notes.md`.
2. **ACS Viewer code** (GPL-3.0). It may be ported, because this project is
   GPL-3.0-or-later too. Ported code keeps a header crediting Dan Boris and
   pointing to `docs/provenance.md`.
3. **Game rules recovered from the original program** (formulas, AI
   tie-breaks). Measuring behavior by playing the original (in an emulator) is
   fine for anyone. Reading or disassembling the original program's code is
   allowed only for a designated *researcher*, who writes the result into the
   behavior spec as plain-language rules, never as code or pseudo-code. Whoever
   implements a rule from that spec must not have read the original code for
   it. Say in your pull request which role you had.

## 4. Provenance

Every outside source (documents, code, palettes, measurements) goes into
`docs/provenance.md` with what it was used for. If you are unsure whether
something may be used, ask in an issue before using it.

## 5. Licensing of contributions

By contributing you agree that your code is licensed GPL-3.0-or-later, your
art, audio and sample adventures CC BY-SA 4.0, and your documentation CC BY 4.0.
Use SPDX headers (`// SPDX-License-Identifier: GPL-3.0-or-later`) in source files.

## 6. The project name

"ACSVM" is a working name. Keep it out of code and strings: it lives in the
workspace `description` in `Cargo.toml` (read via `CARGO_PKG_DESCRIPTION`) and
in `README.md`.
