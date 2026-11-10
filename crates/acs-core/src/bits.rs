// SPDX-License-Identifier: GPL-3.0-or-later

//! Bit-field and bounds-checked byte access.

use crate::error::{Error, Result};

/// Bits `lo..=hi` of `b`, shifted down. `field(0b1011_0000, 4, 7) == 0b1011`.
#[inline]
pub const fn field(b: u8, lo: u8, hi: u8) -> u8 {
    let width = hi - lo + 1;
    let mask = if width >= 8 { 0xFF } else { (1u8 << width) - 1 };
    (b >> lo) & mask
}

/// Bit `n` of `b`.
#[inline]
pub const fn bit(b: u8, n: u8) -> bool {
    (b >> n) & 1 == 1
}

/// A bounds-checked view over an image. Every read names what it was for, so
/// an out-of-range error says which structure was being decoded.
#[derive(Clone, Copy)]
pub struct Bytes<'a> {
    data: &'a [u8],
}

impl<'a> Bytes<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn block(&self, addr: usize, len: usize, what: &'static str) -> Result<&'a [u8]> {
        self.data
            .get(addr..addr.saturating_add(len))
            .ok_or(Error::OutOfBounds {
                addr,
                len,
                size: self.data.len(),
                what,
            })
    }

    pub fn u8(&self, addr: usize, what: &'static str) -> Result<u8> {
        Ok(self.block(addr, 1, what)?[0])
    }

    pub fn u16le(&self, addr: usize, what: &'static str) -> Result<u16> {
        let b = self.block(addr, 2, what)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields() {
        assert_eq!(field(0b1011_0110, 0, 3), 0b0110);
        assert_eq!(field(0b1011_0110, 4, 7), 0b1011);
        assert_eq!(field(0b1011_0110, 5, 6), 0b01);
        assert_eq!(field(0xFF, 0, 7), 0xFF);
        assert!(bit(0x80, 7));
        assert!(!bit(0x7F, 7));
    }

    #[test]
    fn bounds() {
        let data = [1u8, 2, 3];
        let b = Bytes::new(&data);
        assert_eq!(b.u16le(1, "t").unwrap(), 0x0302);
        assert!(b.block(2, 2, "t").is_err());
        assert!(b.block(usize::MAX, 2, "t").is_err());
    }
}
