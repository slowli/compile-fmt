//! Compile-time formatting and derived functionality (e.g., panics / assertions).
//!
//! # What?
//!
//! This crate allows formatting values in compile time (e.g., in `const fn`s). The formatted values
//! are not required to be constants; e.g., arguments or local vars in `const fn` can be formatted.
//! The formatting logic is space-efficient; i.e., it allocates the least amount of bytes
//! that can provably to be sufficient for all possible provided inputs. As a consequence, non-constant
//! formatted args require a [format specifier](Fmt).
//!
//! # Why?
//!
//! A guiding use case for the crate is richer dynamic compile-time panic messages. It can be used
//! in other contexts as well (including in runtime).
//!
//! # Limitations
//!
//! - Only a few types from the standard library can be formatted: integers, `char`s and `str`ings.
//! - Formatting specifiers do not support hex encoding, debug formatting etc.
//! - Padding logic assumes that any Unicode char has identical displayed width, which isn't really
//!   true (e.g., there are chars that have zero width and instead combine with the previous char).
//!   The same assumption is made by the `std` padding logic.
//!
//! # Alternatives and similar tools
//!
//! - [`const_panic`] provides functionality covering the guiding use case (compile-time panics).
//!   It supports more types and formats at the cost of being more complex. It also uses a different
//!   approach to compute produced message sizes.
//! - [`const_format`] provides general-purpose formatting of constant values. It doesn't seem to support
//!   "dynamic" / non-constant args.
//!
//! [`const_panic`]: https://crates.io/crates/const_panic
//! [`const_format`]: https://crates.io/crates/const_format/
//!
//! # Examples
//!
//! ## Basic usage
//!
//! ```
//! use compile_fmt::{compile_assert, fmt};
//!
//! const THRESHOLD: usize = 42;
//!
//! const fn check_value(value: usize) {
//!     compile_assert!(
//!         value <= THRESHOLD,
//!         "Expected ", value => fmt::<usize>(), " to not exceed ", THRESHOLD
//!     );
//!     // main logic
//! }
//! ```
//!
//! Note the formatting spec produced with [`fmt()`].
//!
//! ## Usage with dynamic strings
//!
//! ```
//! use compile_fmt::{compile_assert, clip};
//!
//! const fn check_str(s: &str) {
//!     const MAX_LEN: usize = 16;
//!     compile_assert!(
//!         s.len() <= MAX_LEN,
//!         "String '", s => clip(MAX_LEN, "…"), "' is too long; \
//!          expected no more than", MAX_LEN, " bytes"
//!     );
//!     // main logic
//! }
//!```
//!
//! See docs for macros and format specifiers for more examples.

#![no_std]
// Documentation settings.
#![doc(html_root_url = "https://docs.rs/compile-fmt/0.1.0")]
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
extern crate std;

mod argument;
mod format;
mod macros;
mod utils;

use crate::utils::ClippedStr;
pub use crate::{
    argument::{Argument, ArgumentWrapper, Ascii},
    format::{clip, clip_ascii, fmt, Fmt, FormatArgument, MaxWidth, StrFormat},
};

/// Formatted string returned by the [`compile_args!`] macro, similar to [`Arguments`](fmt::Arguments).
///
/// The type parameter specifies the compile-time upper boundary of the formatted string length in bytes.
/// It is not necessarily equal to the actual byte length of the formatted string.
#[derive(Debug)]
pub struct CompileArgs<const CAP: usize> {
    buffer: [u8; CAP],
    len: usize,
}

impl<const CAP: usize> fmt::Display for CompileArgs<CAP> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl<const CAP: usize> AsRef<str> for CompileArgs<CAP> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<const CAP: usize> CompileArgs<CAP> {
    const fn new() -> Self {
        Self {
            buffer: [0_u8; CAP],
            len: 0,
        }
    }

    const fn write_str(self, s: &str, fmt: Option<StrFormat>) -> Self {
        match fmt {
            Some(StrFormat { clip_at, clip_with }) => {
                let clipped = ClippedStr::new(s, clip_at);
                match clipped {
                    ClippedStr::Full(bytes) => self.write_str_bytes(bytes),
                    ClippedStr::Clipped(bytes) => self
                        .write_str_bytes(bytes)
                        .write_str_bytes(clip_with.as_bytes()),
                }
            }
            _ => self.write_str_bytes(s.as_bytes()),
        }
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

#[cfg(doctest)]
doc_comment::doctest!("../README.md");

#[cfg(test)]
mod tests {
    use std::{
        panic,
        string::{String, ToString},
    };

    use super::*;

    const THRESHOLD: usize = 32;

    #[test]
    fn basics() {
        const TEST: CompileArgs<32> =
            compile_args!("expected ", 1_usize, " to be greater than ", THRESHOLD);
        assert_eq!(TEST.to_string(), "expected 1 to be greater than 32");
    }

    #[test]
    fn using_chars() {
        const CHARS: CompileArgs<11> = compile_args!('H', 'i', 'ß', 'ℝ', '💣');
        assert_eq!(CHARS.to_string(), "Hißℝ💣");
    }

    #[test]
    fn using_dynamic_chars() {
        for char in ['i', 'ß', 'ℝ', '💣'] {
            let s = compile_args!("char: ", char => fmt::<char>(), "!");
            assert_eq!(s.as_str(), std::format!("char: {char}!"));
        }
    }

    #[test]
    fn clipping_strings() {
        let arg = "dynamic";
        let s = compile_args!("string: '", arg => clip(3, ""), '\'');
        assert_eq!(s.as_str(), "string: 'dyn'");

        let arg = "Tℝ💣eßt";
        let s = compile_args!("string: '", arg => clip(2, ""), '\'');
        assert_eq!(s.as_str(), "string: 'Tℝ'");
        let s = compile_args!("string: '", arg => clip(3, ""), '\'');
        assert_eq!(s.as_str(), "string: 'Tℝ💣'");
        let s = compile_args!("string: '", arg => clip(4, ""), '\'');
        assert_eq!(s.as_str(), "string: 'Tℝ💣e'");
        let s = compile_args!("string: '", arg => clip(5, ""), '\'');
        assert_eq!(s.as_str(), "string: 'Tℝ💣eß'");
    }

    #[test]
    fn clipping_strings_with_clip_chars() {
        let arg = "dynamic";
        let s = compile_args!("string: '", arg => clip(3, "-"), '\'');
        assert_eq!(s.as_str(), "string: 'dyn-'");
        let s = compile_args!("string: '", arg => clip(3, "[..]"), '\'');
        assert_eq!(s.as_str(), "string: 'dyn[..]'");
        let s = compile_args!("string: '", arg => clip(3, "…"), '\'');
        assert_eq!(s.as_str(), "string: 'dyn…'");

        let s = compile_args!("string: '", arg => clip(10, "-"), '\'');
        assert_eq!(s.as_str(), "string: 'dynamic'");
    }

    #[test]
    fn padding() {
        let num = 42_u64;
        let s = compile_args!(
            "number: [", num => fmt::<u64>().pad_left(4, ' '), "]"
        );
        assert_eq!(s.as_str(), "number: [42  ]");

        let s = compile_args!(
            "number: [", num => fmt::<u64>().pad_center(4, ' '), "]"
        );
        assert_eq!(s.as_str(), "number: [ 42 ]");

        let s = compile_args!(
            "number: [", num => fmt::<u64>().pad_right(4, '0'), "]"
        );
        assert_eq!(s.as_str(), "number: [0042]");

        let s = compile_args!(
            "number: [", num => fmt::<u64>().pad_right(4, 'ℝ'), "]"
        );
        assert_eq!(s.as_str(), "number: [ℝℝ42]");
        let s = compile_args!(
            "number: [", num => fmt::<u64>().pad_right(4, '💣'), "]"
        );
        assert_eq!(s.as_str(), "number: [💣💣42]");

        let s = compile_args!(
            "number: [", num * 10_000 => fmt::<u64>().pad_right(4, '0'), "]"
        );
        assert_eq!(s.as_str(), "number: [420000]");
    }

    #[test]
    fn clipping_and_padding() {
        let arg = "test string";
        let s = compile_args!(
            "string: [", arg => clip(4, "").pad_left(8, ' '), "]"
        );
        assert_eq!(s.as_str(), "string: [test    ]");

        let s = compile_args!(
            "string: [", arg => clip(4, "-").pad_right(8, ' '), "]"
        );
        assert_eq!(s.as_str(), "string: [   test-]");

        let s = compile_args!(
            "string: [", arg => clip(4, "…").pad_center(8, ' '), "]"
        );
        assert_eq!(s.as_str(), "string: [ test…  ]");

        let s = compile_args!(
            "string: [", arg => clip(4, "…").pad_left(8, '💣'), "]"
        );
        assert_eq!(s.as_str(), "string: [test…💣💣💣]");
        let s = compile_args!(
            "string: [", arg => clip(4, "…").pad_center(8, 'ß'), "]"
        );
        assert_eq!(s.as_str(), "string: [ßtest…ßß]");

        let s = compile_args!(
            "string: [", arg => clip(4, "…").pad_left(4, ' '), "]"
        );
        assert_eq!(s.as_str(), "string: [test…]");
    }

    #[test]
    fn ascii_strings() {
        let s: CompileArgs<11> = compile_args!("ASCII: ", Ascii::new("test"));
        assert_eq!(s.as_str(), "ASCII: test");

        let s: CompileArgs<25> = compile_args!(
            "ASCII: ", Ascii::new("test") => clip_ascii(16, "..")
        );
        // ^ 25 = "ASCII: ".len() + 16 + "..".len()
        assert_eq!(s.as_str(), "ASCII: test");

        let s: CompileArgs<10> = compile_args!(
            "ASCII: ", Ascii::new("test") => clip_ascii(2, "~")
        );
        assert_eq!(s.as_str(), "ASCII: te~");
    }

    #[test]
    #[should_panic(expected = "expected 1 to be greater than 32")]
    fn assertion() {
        let value = 1;
        compile_assert!(
            value > THRESHOLD,
            "expected ", value => fmt::<usize>(), " to be greater than ", THRESHOLD
        );
    }

    #[cfg(panic = "unwind")]
    #[test]
    fn assertion_produces_exactly_expected_string() {
        let panic_result = panic::catch_unwind(|| {
            let value = 1;
            compile_assert!(
                value > THRESHOLD,
                "expected ", value => fmt::<usize>(), " to be greater than ", THRESHOLD
            );
        });
        let panic_message = panic_result.unwrap_err();
        let panic_message = panic_message.downcast_ref::<String>().unwrap();
        assert_eq!(panic_message, "expected 1 to be greater than 32");
        // ^ `const_panic` crate fails this test; it pads the panic message with '\0' chars
    }

    const fn unwrap_result(res: Result<(), &str>) {
        if let Err(err) = res {
            compile_panic!("Encountered an error: ", err => clip(64, "…"));
        }
    }

    #[test]
    #[should_panic(expected = "Encountered an error: operation not supported")]
    fn using_panic() {
        unwrap_result(Err("operation not supported"));
    }
}
