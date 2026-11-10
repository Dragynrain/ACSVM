// SPDX-License-Identifier: GPL-3.0-or-later

//! Core library: reads Adventure Construction Set adventures into a
//! platform-neutral model. No GUI, no I/O beyond reading the input file.
//!
//! ```no_run
//! use acs_core::{decode, Image};
//! let image = Image::open("GAME.D64".as_ref(), None)?;
//! let decoded = decode(&image)?;
//! println!("{}", decoded.adventure.name);
//! # Ok::<(), acs_core::Error>(())
//! ```

pub mod bits;
pub mod decode;
pub mod disk;
pub mod error;
pub mod graphics;
pub mod layout;
pub mod model;
pub mod scene;
pub mod text;

pub use decode::{decode, Decoded};
pub use disk::{Image, Platform};
pub use error::{Diagnostic, DiagnosticKind, Error, Result, Severity};
pub use model::Adventure;
