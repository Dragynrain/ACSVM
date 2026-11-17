// SPDX-License-Identifier: GPL-3.0-or-later

//! Reading TOML tables field by field, so every problem names its file
//! and what it is in, and unknown fields are caught (content 3.1).

use std::cell::RefCell;
use toml::{Table, Value};

/// One problem in a source folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    pub file: String,
    /// What it is in, e.g. `thing cutlass` or `room port.tavern`.
    pub at: String,
    pub message: String,
}

impl std::fmt::Display for Problem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.at.is_empty() {
            write!(f, "{}: {}", self.file, self.message)
        } else {
            write!(f, "{}: {}: {}", self.file, self.at, self.message)
        }
    }
}

/// Where problems collect.
#[derive(Default)]
pub struct Problems(pub RefCell<Vec<Problem>>);

impl Problems {
    pub fn push(&self, file: &str, at: &str, message: impl Into<String>) {
        self.0.borrow_mut().push(Problem {
            file: file.into(),
            at: at.into(),
            message: message.into(),
        });
    }
}

/// A table being read: its file, what it is, and the keys read so far.
pub struct Obj<'a, 'p> {
    pub table: &'a Table,
    pub file: String,
    pub at: String,
    pub p: &'p Problems,
    used: RefCell<Vec<String>>,
}

impl<'a, 'p> Obj<'a, 'p> {
    pub fn new(table: &'a Table, file: &str, at: impl Into<String>, p: &'p Problems) -> Obj<'a, 'p> {
        Obj {
            table,
            file: file.into(),
            at: at.into(),
            p,
            used: RefCell::new(Vec::new()),
        }
    }

    pub fn err(&self, message: impl Into<String>) {
        self.p.push(&self.file, &self.at, message);
    }

    /// A child table's reader.
    pub fn child<'b>(&self, table: &'b Table, at: impl Into<String>) -> Obj<'b, 'p> {
        Obj::new(table, &self.file, at, self.p)
    }

    pub fn has(&self, key: &str) -> bool {
        self.table.contains_key(key)
    }

    pub fn raw(&self, key: &str) -> Option<&'a Value> {
        let v = self.table.get(key)?;
        self.used.borrow_mut().push(key.to_string());
        Some(v)
    }

    pub fn str(&self, key: &str) -> Option<&'a str> {
        match self.raw(key)? {
            Value::String(s) => Some(s),
            _ => {
                self.err(format!("{key}: should be text"));
                None
            }
        }
    }

    pub fn need_str(&self, key: &str) -> Option<&'a str> {
        let s = self.str(key);
        if s.is_none() && !self.has(key) {
            self.err(format!("{key}: missing"));
        }
        s
    }

    pub fn bool(&self, key: &str) -> bool {
        match self.raw(key) {
            None => false,
            Some(Value::Boolean(b)) => *b,
            Some(_) => {
                self.err(format!("{key}: should be true or false"));
                false
            }
        }
    }

    /// An integer within `lo..=hi`, or `None` if absent (or wrong, reported).
    pub fn int(&self, key: &str, lo: i64, hi: i64) -> Option<i64> {
        match self.raw(key)? {
            Value::Integer(n) if (lo..=hi).contains(n) => Some(*n),
            Value::Integer(n) => {
                self.err(format!("{key}: {n} is outside {lo}..{hi}"));
                None
            }
            _ => {
                self.err(format!("{key}: should be a whole number"));
                None
            }
        }
    }

    pub fn int_or(&self, key: &str, lo: i64, hi: i64, default: i64) -> i64 {
        self.int(key, lo, hi).unwrap_or(default)
    }

    pub fn need_int(&self, key: &str, lo: i64, hi: i64) -> Option<i64> {
        if !self.has(key) {
            self.err(format!("{key}: missing"));
        }
        self.int(key, lo, hi)
    }

    pub fn table(&self, key: &str) -> Option<&'a Table> {
        match self.raw(key)? {
            Value::Table(t) => Some(t),
            _ => {
                self.err(format!("{key}: should be a table"));
                None
            }
        }
    }

    pub fn array(&self, key: &str) -> &'a [Value] {
        match self.raw(key) {
            None => &[],
            Some(Value::Array(a)) => a,
            Some(_) => {
                self.err(format!("{key}: should be a list"));
                &[]
            }
        }
    }

    /// `[x, y]` within `0..=max`.
    pub fn point(&self, key: &str, max: i64) -> Option<(u8, u8)> {
        let v = self.raw(key)?;
        point(v, max).or_else(|| {
            self.err(format!("{key}: should be [x, y], each 0..{max}"));
            None
        })
    }

    /// Reports every key not read. Call once the table is done.
    pub fn done(&self) {
        let used = self.used.borrow();
        for k in self.table.keys() {
            if !used.contains(k) {
                self.err(format!("{k}: not a field here"));
            }
        }
    }
}

pub fn point(v: &Value, max: i64) -> Option<(u8, u8)> {
    let Value::Array(a) = v else { return None };
    match a.as_slice() {
        [Value::Integer(x), Value::Integer(y)] if (0..=max).contains(x) && (0..=max).contains(y) => {
            Some((*x as u8, *y as u8))
        }
        _ => None,
    }
}

/// Whether `k` is a key: lowercase letters, digits and `_`, not empty.
pub fn is_key(k: &str) -> bool {
    !k.is_empty()
        && k.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}
