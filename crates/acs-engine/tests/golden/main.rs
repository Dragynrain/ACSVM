// SPDX-License-Identifier: GPL-3.0-or-later

//! Golden tests: the engine against the original's captured outcomes.
//!
//! Each test rebuilds one capture of `docs/capture.md` on a synthetic
//! adventure shaped like the capture's scenario disks ([`world`]), runs the
//! engine, and compares with the counts the capture recorded, which are
//! written here with the section they come from. Deterministic captures must
//! match exactly; random ones are run over [`stats::SEEDS`] fixed seeds and
//! compared within the limits [`stats`] explains. Only distilled numbers
//! from our own captures are here, nothing from the original program.
//!
//! When a rule changes on purpose, the capture it disagrees with says so:
//! fix the engine or, if a newer capture overrules the old one, change the
//! numbers here with the newer section's reference.

#[path = "../../../acs-core/tests/common/mod.rs"]
mod common;

mod stats;
#[allow(dead_code)]
mod world;

mod casting;
mod combat;
mod creatures;
mod healing;
mod movement;
mod rest;
mod traces;
