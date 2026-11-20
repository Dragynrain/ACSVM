// SPDX-License-Identifier: GPL-3.0-or-later

//! The sample adventures built in (docs/content.md 7): our own, built from
//! `adventures/` by `build.rs`, offered on the title screen.

pub struct Sample {
    pub name: &'static str,
    /// What the title screen says about it.
    pub about: &'static str,
    /// The file name it opens as.
    pub file: &'static str,
    pub bytes: &'static [u8],
}

pub const SAMPLES: &[Sample] = &[Sample {
    name: "First Voyage",
    about: "The tutorial: a harbour, a trader, a crab and a chest of coin. Under five minutes.",
    file: "first-voyage.advx",
    bytes: include_bytes!(concat!(env!("OUT_DIR"), "/first-voyage.advx")),
}];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sample_is_a_bundle_that_loads() {
        for s in SAMPLES {
            assert!(acs_bundle::is_bundle(s.file, s.bytes), "{}", s.name);
            let b = acs_bundle::read(s.bytes).unwrap_or_else(|e| panic!("{}: {e}", s.name));
            assert!(b.art.is_some(), "{}: no art", s.name);
        }
    }
}
