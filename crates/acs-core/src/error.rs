// SPDX-License-Identifier: GPL-3.0-or-later

//! Error and diagnostic types.

use serde::{Deserialize, Serialize};
use std::fmt;

/// A failure that stops decoding: the input cannot be an adventure of the
/// requested platform, or a structure points outside the file.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("cannot read {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error(
        "cannot tell the platform of {0:?}; use a known extension (.d64, .fpy, .hrd) or pass a platform"
    )]
    UnknownPlatform(String),
    #[error("{platform} image is {actual} bytes; expected {expected}")]
    BadSize {
        platform: &'static str,
        actual: usize,
        expected: String,
    },
    #[error("read of {len} byte(s) at 0x{addr:05X} ({what}) runs past the end of the {size}-byte image")]
    OutOfBounds {
        addr: usize,
        len: usize,
        size: usize,
        what: &'static str,
    },
    #[error("not an adventure: {0}")]
    NotAnAdventure(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// How serious a [`Diagnostic`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Unusual but representable, e.g. a field value the editor should never write.
    Warning,
    /// Data the decoder could not interpret; the model holds a raw fallback.
    Error,
}

/// What kind of oddity a [`Diagnostic`] reports. Tests match on this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticKind {
    UnknownThingType,
    UnknownSpellAction,
    BadReference,
    BadText,
    BadRoomItems,
    BadPalette,
    Other,
}

/// A non-fatal finding made while decoding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub kind: DiagnosticKind,
    /// Where in the model, e.g. `thing 12` or `region 3 room 4 item 2`.
    pub location: String,
    pub message: String,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sev = match self.severity {
            Severity::Warning => "warning",
            Severity::Error => "error",
        };
        write!(f, "{sev}: {}: {}", self.location, self.message)
    }
}

/// Collects diagnostics during a decode.
#[derive(Debug, Default)]
pub struct Diagnostics(pub Vec<Diagnostic>);

impl Diagnostics {
    pub fn push(
        &mut self,
        severity: Severity,
        kind: DiagnosticKind,
        location: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.0.push(Diagnostic {
            severity,
            kind,
            location: location.into(),
            message: message.into(),
        });
    }

    pub fn warn(&mut self, kind: DiagnosticKind, location: impl Into<String>, msg: impl Into<String>) {
        self.push(Severity::Warning, kind, location, msg);
    }

    pub fn error(&mut self, kind: DiagnosticKind, location: impl Into<String>, msg: impl Into<String>) {
        self.push(Severity::Error, kind, location, msg);
    }
}
