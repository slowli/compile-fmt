//! Compile-time formatting.
//!
//! FIXME: use cases, impl details, examples

#![no_std]
// Linter settings.
#![warn(missing_debug_implementations, missing_docs, bare_trait_objects)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::module_name_repetitions
)]

use core::{fmt, slice, str};
#[cfg(test)]
extern crate alloc;

mod argument;
mod format;
mod macros;

pub use crate::{
    argument::{Argument, ArgumentWrapper},
    format::{fmt, Fmt, FormatArgument, MaxWidth, StrFormat},
};

/// Formatted string returned by the [`const_args!`] macro, similar to [`Arguments`](fmt::Arguments).
///
/// The type parameter specifies the compile-time upper boundary of the formatted string length in bytes.
/// It is not necessarily equal to the actual byte length of the formatted string.
#[derive(Debug)]
pub struct ConstArgs<const CAP: usize> {
    buffer: [u8; CAP],
    len: usize,
}

impl<const CAP: usize> fmt::Display for ConstArgs<CAP> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl<const CAP: usize> AsRef<str> for ConstArgs<CAP> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<const CAP: usize> ConstArgs<CAP> {
    const fn new() -> Self {
        Self {
            buffer: [0_u8; CAP],
            len: 0,
        }
    }

    const fn write_str(self, s: &str, fmt: Option<StrFormat>) -> Self {
        let truncated_str_bytes = match fmt {
            Some(StrFormat { truncate_at }) => first_chars(s, truncate_at),
            _ => s.as_bytes(),
        };
        self.write_str_bytes(truncated_str_bytes)
    }

    const fn write_str_bytes(self, s_bytes: &[u8]) -> Self {
        let new_len = self.len + s_bytes.len();
        let mut buffer = self.buffer;
        let mut pos = self.len;

        while pos < new_len {
            buffer[pos] = s_bytes[pos - self.len];
            pos += 1;
        }
        Self {
            buffer,
            len: new_len,
        }
    }

    /// Writes a char to this string. Largely copied from the standard library with minor changes.
    #[allow(clippy::cast_possible_truncation)] // false positive
    const fn write_char(self, c: char) -> Self {
        const TAG_CONT: u8 = 0b_1000_0000;
        const TAG_TWO_BYTES: u8 = 0b_1100_0000;
        const TAG_THREE_BYTES: u8 = 0b_1110_0000;
        const TAG_FOUR_BYTES: u8 = 0b_1111_0000;

        let new_len = self.len + c.len_utf8();
        let mut buffer = self.buffer;
        let pos = self.len;
        let code = c as u32;
        match c.len_utf8() {
            1 => {
                buffer[pos] = code as u8;
            }
            2 => {
                buffer[pos] = (code >> 6 & 0x_1f) as u8 | TAG_TWO_BYTES;
                buffer[pos + 1] = (code & 0x_3f) as u8 | TAG_CONT;
            }
            3 => {
                buffer[pos] = (code >> 12 & 0x_0f) as u8 | TAG_THREE_BYTES;
                buffer[pos + 1] = (code >> 6 & 0x_3f) as u8 | TAG_CONT;
                buffer[pos + 2] = (code & 0x_3f) as u8 | TAG_CONT;
            }
            4 => {
                buffer[pos] = (code >> 18 & 0x_07) as u8 | TAG_FOUR_BYTES;
                buffer[pos + 1] = (code >> 12 & 0x_3f) as u8 | TAG_CONT;
                buffer[pos + 2] = (code >> 6 & 0x_3f) as u8 | TAG_CONT;
                buffer[pos + 3] = (code & 0x_3f) as u8 | TAG_CONT;
            }
            _ => unreachable!(),
        }

        Self {
            buffer,
            len: new_len,
        }
    }

    /// Formats the provided sequence of [`Argument`]s.
    pub const fn format(arguments: &[Argument]) -> Self {
        let mut this = Self::new();
        let mut arg_i = 0;
        while arg_i < arguments.len() {
            this = this.format_arg(arguments[arg_i]);
            arg_i += 1;
        }
        this
    }

    /// Returns the `str` value of this formatter.
    pub const fn as_str(&self) -> &str {
        unsafe {
            // SAFETY: This is equivalent to `&self.buffer[..self.len]`, only works in compile time.
            let written_slice = slice::from_raw_parts(self.buffer.as_ptr(), self.len);
            // SAFETY: Safe by construction; written bytes form a valid `str`.
            str::from_utf8_unchecked(written_slice)
        }
    }
}

/// Returns bytes corresponding to first `char_count` chars in `s`. If `s` contains less chars,
/// it's returned in full.
const fn first_chars(s: &str, mut char_count: usize) -> &[u8] {
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
    unsafe { slice::from_raw_parts(s_bytes.as_ptr(), pos) }
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;

    use super::*;

    const THRESHOLD: usize = 32;

    #[test]
    fn basics() {
        const TEST: ConstArgs<32> =
            const_args!("expected ", 1_usize, " to be greater than ", THRESHOLD);
        assert_eq!(TEST.to_string(), "expected 1 to be greater than 32");
    }

    #[test]
    fn using_chars() {
        const CHARS: ConstArgs<11> = const_args!('H', 'i', 'ß', 'ℝ', '💣');
        assert_eq!(CHARS.to_string(), "Hißℝ💣");
    }

    #[test]
    fn using_dynamic_chars() {
        for char in ['i', 'ß', 'ℝ', '💣'] {
            let s = const_args!("char: ", char => fmt::<char>(), "!");
            assert_eq!(s.as_str(), alloc::format!("char: {char}!"));
        }
    }

    #[test]
    fn truncating_strings() {
        let arg = "dynamic";
        let s = const_args!("string: '", arg => Fmt::truncated(3), '\'');
        assert_eq!(s.as_str(), "string: 'dyn'");

        let arg = "Tℝ💣eßt";
        let s = const_args!("string: '", arg => Fmt::truncated(2), '\'');
        assert_eq!(s.as_str(), "string: 'Tℝ'");
        let s = const_args!("string: '", arg => Fmt::truncated(3), '\'');
        assert_eq!(s.as_str(), "string: 'Tℝ💣'");
        let s = const_args!("string: '", arg => Fmt::truncated(4), '\'');
        assert_eq!(s.as_str(), "string: 'Tℝ💣e'");
        let s = const_args!("string: '", arg => Fmt::truncated(5), '\'');
        assert_eq!(s.as_str(), "string: 'Tℝ💣eß'");
    }

    #[test]
    fn extracting_first_chars_from_ascii_string() {
        assert_eq!(first_chars("Test", 1), b"T");
        assert_eq!(first_chars("Test", 2), b"Te");
        assert_eq!(first_chars("Test", 3), b"Tes");
        for char_count in [4, 5, 8, 32, 128] {
            assert_eq!(first_chars("Test", char_count), b"Test");
        }
    }

    #[test]
    fn extracting_first_chars_from_utf8_string() {
        assert_eq!(first_chars("💣Test", 1), "💣".as_bytes());
        assert_eq!(first_chars("💣Test", 2), "💣T".as_bytes());
        assert_eq!(first_chars("T💣est", 3), "T💣e".as_bytes());
        assert_eq!(first_chars("T💣eßtℝ", 4), "T💣eß".as_bytes());
        assert_eq!(first_chars("Tℝ💣eßt", 4), "Tℝ💣e".as_bytes());
        assert_eq!(first_chars("Tℝ💣eßt", 5), "Tℝ💣eß".as_bytes());

        for char_count in [6, 8, 32, 128] {
            assert_eq!(first_chars("Tℝ💣eßt", char_count), "Tℝ💣eßt".as_bytes());
        }
    }

    #[test]
    #[should_panic(expected = "expected 1 to be greater than 32")]
    fn assertion() {
        let value = 1;
        const_assert!(
            value > THRESHOLD,
            "expected ", value => fmt::<usize>(), " to be greater than ", THRESHOLD
        );
    }
}
