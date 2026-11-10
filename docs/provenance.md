# Provenance

Every outside source used by this project, and how it was used. Licensed CC BY 4.0.

| Source | License / status | Used for | How |
| --- | --- | --- | --- |
| [ACS Viewer](https://github.com/danlb2000/ACSViewer) by Dan Boris, `AcsLib` (commit `7d744a8`, 2020-03-21) | GPL-3.0-or-later | Block base addresses, field layouts of things, creatures, regions, rooms, room items, terrain and world map doors; text decoding; PC and C64 picture byte layouts; the PC 16-colour values | **Ported** into `crates/acs-core/src/{layout,decode,text,graphics}.rs`, which carry a credit header. Rewritten in Rust; several layouts corrected or generalised against the corpus (see `format-notes.md`). Its test data file `AcsLibTest/RiversOfLight.hrd` is an EA adventure and was **not** used or copied. Its C64 colour table (from VICE/CCS) was not used. |
| ACS file format draft v0.5 by Dan Boris (2016) | Document, no license stated | Orientation, cross-check against AcsLib | Read only. Where it disagrees with AcsLib or the corpus, the difference is recorded in `format-notes.md`. No text copied. |
| *Adventure Construction Set* manual (EA, 1984), private OCR copy | Copyright EA | Background on what fields mean (e.g. what happens with a door to a deleted room) | Read only, paraphrased. Never quoted or shipped. |
| John Baber's Jadeworld adventures (14 C64 `.d64` disks, 2023 update), private copy | No license stated | Private test corpus: validating the decoder, settling vague parts of the format | Decoded locally through `ACSVM_CORPUS`. Only counts and statistics appear in `format-notes.md`; no bytes, names or pictures from them are committed. |
| EA's *Rivers of Light*, PC (`ACS.FPY` from the user's copy of the MS-DOS release), private copy | Copyright EA | Private PC test corpus: validating PC offsets, pictures and doors | Decoded locally from `ACSVM_CORPUS/pc`. Only counts and statistics appear in `format-notes.md`; nothing from it is committed. |
| EA's ACS program disks (`advcnst1.d64`, `advcnst2.d64`) | Copyright EA | Only to confirm the decoder rejects a non-adventure disk | `advcnst1.d64` is opened by one corpus test that asserts decoding fails. Neither disk is otherwise decoded. |
| C64 colours: Philip "Pepto" Timmermann's measured PAL palette (pepto.de/projects/colorvic/2001) | Measured colour values (facts) | RGB for the 16 C64 colours | Values typed into `graphics.rs` with a citation. |
| PC colours: standard CGA/EGA 16-colour values | Facts | RGB for the 16 PC colours | Same values as ACS Viewer. |
| 1541 `.d64` geometry (sectors per track) | Facts, widely documented | Track/sector offsets in `disk.rs` | Written from general knowledge. |
| Rust crates used by the viewer (eframe/egui, rfd, png, wasm-bindgen) and trunk | MIT / Apache-2.0 | GUI, file dialogs, PNG output, web build | Ordinary dependencies from crates.io. No format knowledge came from them. The room-drawing rules in `format-notes.md` come from corpus statistics only. |
| GPL-3.0, CC BY 4.0 and CC BY-SA 4.0 texts | Their own terms | `LICENSE`, `LICENSES/` | Downloaded unchanged from gnu.org and creativecommons.org. |
| `docs/design.md` | CC BY 4.0, project's own | Plan | Snapshot of the living design document. |

## Clean-room status

No code in this repository was derived from the original ACS program (no
disassembly has been read). All format work so far comes from ACS Viewer, the
draft and analysis of adventure data files.
