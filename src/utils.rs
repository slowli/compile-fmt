//! Miscellaneous utils.

use core::slice;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ClippedStr<'a> {
    Full(&'a [u8]),
    Clipped(&'a [u8]),
}

impl<'a> ClippedStr<'a> {
    /// Returns bytes corresponding to first `char_count` chars in `s`. If `s` contains less chars,
    /// it's returned in full.
    pub const fn new(s: &'a str, mut char_count: usize) -> Self {
        let s_bytes = s.as_bytes();
        let mut pos = 0;
        while pos < s_bytes.len() && char_count > 0 {
            if s_bytes[pos] < 128 {
                pos += 1;
            } else if s_bytes[pos] >> 5 == 0b_110 {
                pos += 2;
            } else if s_bytes[pos] >> 4 == 0b_1110 {
                pos += 3;
            } else if s_bytes[pos] >> 3 == 0b_11110 {
                pos += 4;
            } else {
                unreachable!(); // Invalid UTF-8 encoding
            }
            char_count -= 1;
        }
        assert!(pos <= s_bytes.len(), "Invalid UTF-8 encoding");
        // SAFETY: Slicing a byte slice with length being in bounds is safe.
        let bytes = unsafe { slice::from_raw_parts(s_bytes.as_ptr(), pos) };
        if pos < s_bytes.len() {
            Self::Clipped(bytes)
        } else {
            Self::Full(bytes)
        }
    }
}

/// Counts the number of chars in a string.
pub(crate) const fn count_chars(s: &str) -> usize {
    let s_bytes = s.as_bytes();
    let mut pos = 0;
    let mut char_count = 0;
    while pos < s_bytes.len() {
        if s_bytes[pos] < 128 {
            pos += 1;
        } else if s_bytes[pos] >> 5 == 0b_110 {
            pos += 2;
        } else if s_bytes[pos] >> 4 == 0b_1110 {
            pos += 3;
        } else if s_bytes[pos] >> 3 == 0b_11110 {
            pos += 4;
        } else {
            unreachable!(); // Invalid UTF-8 encoding
        }
        char_count += 1;
    }
    char_count
}

pub(crate) const fn assert_is_ascii(s: &str) {
    const CLIP_LEN: usize = 32;

    let s_bytes = s.as_bytes();
    let mut pos = 0;
    while pos < s_bytes.len() {
        if s_bytes[pos] < 128 {
            pos += 1;
        } else {
            crate::compile_panic!(
                "String '", s => crate::clip(CLIP_LEN, "…"), "' contains non-ASCII chars; \
                 first at position ", pos => crate::fmt::<usize>()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracting_first_chars_from_ascii_string() {
        assert_eq!(ClippedStr::new("Test", 1), ClippedStr::Clipped(b"T"));
        assert_eq!(ClippedStr::new("Test", 2), ClippedStr::Clipped(b"Te"));
        assert_eq!(ClippedStr::new("Test", 3), ClippedStr::Clipped(b"Tes"));
        for char_count in [4, 5, 8, 32, 128] {
            assert_eq!(
                ClippedStr::new("Test", char_count),
                ClippedStr::Full(b"Test")
            );
        }
    }

    #[test]
    fn extracting_first_chars_from_utf8_string() {
        assert_eq!(
            ClippedStr::new("💣Test", 1),
            ClippedStr::Clipped("💣".as_bytes())
        );
        assert_eq!(
            ClippedStr::new("💣Test", 2),
            ClippedStr::Clipped("💣T".as_bytes())
        );
        assert_eq!(
            ClippedStr::new("T💣est", 3),
            ClippedStr::Clipped("T💣e".as_bytes())
        );
        assert_eq!(
            ClippedStr::new("T💣eßtℝ", 4),
            ClippedStr::Clipped("T💣eß".as_bytes())
        );
        assert_eq!(
            ClippedStr::new("Tℝ💣eßt", 4),
            ClippedStr::Clipped("Tℝ💣e".as_bytes())
        );
        assert_eq!(
            ClippedStr::new("Tℝ💣eßt", 5),
            ClippedStr::Clipped("Tℝ💣eß".as_bytes())
        );

        for char_count in [6, 8, 32, 128] {
            assert_eq!(
                ClippedStr::new("Tℝ💣eßt", char_count),
                ClippedStr::Full("Tℝ💣eßt".as_bytes())
            );
        }
    }
}
