// SPDX-License-Identifier: GPL-3.0-or-later
//
// Text rules follow ACS Viewer's AcsLib TextDecoder (Copyright (C) 2016 Dan
// Boris, GPL-3.0-or-later) and Boris's format draft v0.5. See docs/provenance.md.

//! Text codecs: single-byte text and base-40 packed names.
//!
//! Single-byte text stores `@ A..Z [ \ ] ^ _` as 0..=31 (the C64 screen-code
//! order) and everything else as plain ASCII, so a byte below 32 decodes as
//! `byte + 64`. Packed text stores three characters per little-endian 16-bit
//! word as `c1 * 1600 + c2 * 40 + c3`, using [`PACKED_CHARSET`].

/// The 40 characters available to packed names (things, creatures, rooms).
pub const PACKED_CHARSET: [char; 40] = [
    ' ', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T',
    'U', 'V', 'W', 'X', 'Y', 'Z', '1', '2', '3', '4', '5', '6', '7', '8', '9', '.', ',', '-', '\'',
];

/// Decodes one single-byte character. Returns `None` for bytes with the high
/// bit set, which never belong to editor-written text.
#[inline]
pub fn decode_ascii_byte(b: u8) -> Option<char> {
    match b {
        0..=31 => Some(char::from(b + 64)),
        32..=127 => Some(char::from(b)),
        _ => None,
    }
}

/// Decodes single-byte text. Bytes with the high bit set become U+FFFD.
pub fn decode_ascii(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&b| decode_ascii_byte(b).unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect()
}

/// Encodes text the way the editor stores it: letters and `@[\]^_` as 0..=31,
/// everything else as ASCII. Returns `None` for characters outside 0..=127.
/// The result is padded with spaces (or truncated) to `len` bytes.
pub fn encode_ascii(text: &str, len: usize) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(len);
    for c in text.chars() {
        let code = u32::from(c);
        let b = match code {
            64..=95 => (code - 64) as u8,
            0..=127 => code as u8,
            _ => return None,
        };
        out.push(b);
    }
    out.resize(len, b' ');
    Some(out)
}

/// Decodes packed text. A word whose digits fall outside the charset ends the
/// string (ACS Viewer does the same); the second value reports whether that happened.
pub fn decode_packed(bytes: &[u8]) -> (String, bool) {
    let mut s = String::with_capacity(bytes.len() / 2 * 3);
    for pair in bytes.chunks_exact(2) {
        let v = u16::from_le_bytes([pair[0], pair[1]]) as usize;
        let (c1, rest) = (v / 1600, v % 1600);
        let (c2, c3) = (rest / 40, rest % 40);
        if c1 >= 40 {
            return (s, false);
        }
        s.push(PACKED_CHARSET[c1]);
        s.push(PACKED_CHARSET[c2]);
        s.push(PACKED_CHARSET[c3]);
    }
    (s, true)
}

/// Encodes `text` into `len` bytes of packed text (`len` must be even),
/// padding with spaces. Lowercase letters are folded to uppercase. Returns
/// `None` if a character is not in [`PACKED_CHARSET`] or the text is too long.
pub fn encode_packed(text: &str, len: usize) -> Option<Vec<u8>> {
    assert!(len % 2 == 0, "packed text length must be even");
    let max_chars = len / 2 * 3;
    let mut codes = Vec::with_capacity(max_chars);
    for c in text.chars() {
        let c = c.to_ascii_uppercase();
        codes.push(PACKED_CHARSET.iter().position(|&p| p == c)? as u16);
    }
    if codes.len() > max_chars {
        return None;
    }
    codes.resize(max_chars, 0);
    let mut out = Vec::with_capacity(len);
    for t in codes.chunks_exact(3) {
        let v = t[0] * 1600 + t[1] * 40 + t[2];
        out.extend_from_slice(&v.to_le_bytes());
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_low_codes_are_letters() {
        assert_eq!(decode_ascii(&[0x08, 0x09, b'!', b' ', 0x00]), "HI! @");
        assert_eq!(decode_ascii(&[0x80]), "\u{FFFD}");
    }

    #[test]
    fn ascii_round_trip() {
        let text = "THE DRAGON'S LAIR, 1984!";
        let enc = encode_ascii(text, 32).unwrap();
        assert_eq!(enc.len(), 32);
        assert_eq!(enc[0], 20); // 'T' stored as a screen code
        assert_eq!(decode_ascii(&enc).trim_end(), text);
    }

    #[test]
    fn packed_known_word() {
        // "ABC" = 1*1600 + 2*40 + 3 = 1683 = 0x0693
        assert_eq!(decode_packed(&[0x93, 0x06]), ("ABC".to_string(), true));
    }

    #[test]
    fn packed_round_trip_and_padding() {
        let enc = encode_packed("Iron Sword", 10).unwrap();
        assert_eq!(enc.len(), 10);
        let (s, ok) = decode_packed(&enc);
        assert!(ok);
        assert_eq!(s, "IRON SWORD     ");
    }

    #[test]
    fn packed_rejects_bad_input() {
        assert!(encode_packed("A+B", 10).is_none());
        assert!(encode_packed("SIXTEEN CHARS 16", 10).is_none());
        // 0xFFFF / 1600 = 40: outside the charset
        assert_eq!(
            decode_packed(&[0x93, 0x06, 0xFF, 0xFF]),
            ("ABC".to_string(), false)
        );
    }
}
