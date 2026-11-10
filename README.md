# ACSVM

ACSVM (working name) is an open-source engine that plays adventures made with
*Adventure Construction Set* (Electronic Arts, 1984), in the spirit of ScummVM:
the engine is free software, and you bring the adventure files you own.

**Status: Phase 0.** The first piece is an inspector that decodes C64 and PC
adventures into JSON and exports their pictures as PNG. There is no player yet.
See [docs/design.md](docs/design.md) for the full plan.

This project is not affiliated with or endorsed by Electronic Arts. It ships no
EA program code, art, music, manual text or adventures.

## Supported files

| Platform | File | Status |
| --- | --- | --- |
| C64 | whole `.d64` disk image | Decodes all 14 adventures in the private test corpus with no errors |
| PC (MS-DOS) | `ADVEN.FPY`, `ADVEN.HRD` | Covered by synthetic tests only; **not yet tested against a real PC adventure** |
| Apple II, Amiga | `.dsk`, `.adf` | Not yet |

## Building

Requires Rust (stable, 1.85 or newer).

```bash
cargo build --release
```

The binary is `target/release/acs`.

## Using the inspector

```bash
acs dump GAME.D64                    # summary on stdout, JSON next to the input
acs dump GAME.D64 -v -o game.json    # list every region, room, thing and creature
acs pictures GAME.D64 out/ --scale 4 --sheet
```

`dump` writes the canonical adventure model as JSON (`<name>.acs.json` by
default; that pattern is git-ignored because it is derived from your files).
`pictures` writes one PNG per picture, with C64 pixels doubled in width, and
optionally a contact sheet. Warnings about odd data go to stderr; the exit code
is 2 if the decoder hit data it could not interpret.

## Testing

```bash
cargo test
```

Unit and synthetic tests build their fixtures byte by byte and need no
adventure files. Tests against real adventures read a corpus folder outside
the repository and skip unless `ACSVM_CORPUS` points to it:

```bash
ACSVM_CORPUS=/path/to/corpus cargo test --test corpus -- --nocapture
```

The corpus folder holds `c64/` (`.d64` disks, searched recursively) and
optionally `pc/` (`.fpy`/`.hrd`).

## Repository layout

- `crates/acs-core`: library with no GUI: disk readers, platform layouts,
  text codecs, picture decoders and the canonical model (serde).
- `crates/acs-cli`: the `acs` inspector binary.
- `docs/`: design snapshot, [format notes](docs/format-notes.md) and the
  [provenance record](docs/provenance.md).

## Licenses

- Code: [GPL-3.0-or-later](LICENSE). Parts of the decoder are ported from Dan
  Boris's [ACS Viewer](https://github.com/danlb2000/ACSViewer) (GPL-3.0); see
  [docs/provenance.md](docs/provenance.md).
- Art, audio and sample adventures (none yet): [CC BY-SA 4.0](LICENSES/CC-BY-SA-4.0.txt).
- Documentation: [CC BY 4.0](LICENSES/CC-BY-4.0.txt).

Contributions follow [CONTRIBUTING.md](CONTRIBUTING.md), including the
clean-room policy and the rule that no EA material ever enters the repository.
