//! Compile-time formatting and derived functionality (e.g., panics / assertions).
//!
//! # What?
//!
//! This crate allows formatting values in compile time (e.g., in `const fn`s). The formatted values
//! are not required to be constants; e.g., arguments or local vars in `const fn` can be formatted.
//!
//! Features:
//!
//! - Zero dependencies.
//! - Unconditionally `#[no_std]`-compatible.
//! - The formatting logic is space-efficient; i.e., it allocates the least amount of bytes
//!   that can provably to be sufficient for all possible provided inputs. As a consequence, non-constant
//!   formatted args require a [format specifier](Fmt).
//! - Does not rely on proc macros. This makes the library more lightweight.
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
//!          expected no more than ", MAX_LEN, " bytes"
//!     );
//!     // main logic
//! }
//!```
//!
//! ## Printing dynamically-sized messages
//!
//! `compile_args!` allows specifying capacity of the produced message. This is particularly useful
//! when formatting enums (e.g., to compile-format errors):
//!
//! ```
//! # use compile_fmt::{compile_args, fmt, CompileArgs};
//! #[derive(Debug)]
//! enum Error {
//!     Number(u64),
//!     Tuple(usize, char),
//! }
//!
//! type ErrorArgs = CompileArgs<55>;
//! // ^ 55 is the exact lower boundary on capacity. It's valid to specify
//! // a greater value, e.g. 64.
//!
//! impl Error {
//!     const fn fmt(&self) -> ErrorArgs {
//!         match *self {
//!             Self::Number(number) => compile_args!(
//!                 capacity: ErrorArgs::CAPACITY,
//!                 "don't like number ", number => fmt::<u64>()
//!             ),
//!             Self::Tuple(pos, ch) => compile_args!(
//!                 "don't like char '", ch => fmt::<char>(), "' at position ",
//!                 pos => fmt::<usize>()
//!             ),
//!         }
//!     }
//! }
//!
//! // `Error::fmt()` can be used as a building block for more complex messages:
//! let err = Error::Tuple(1_234, '?');
//! let message = compile_args!("Operation failed: ", &err.fmt() => fmt::<&ErrorArgs>());
//! assert_eq!(
//!     message.as_str(),
//!     "Operation failed: don't like char '?' at position 1234"
//! );
//! ```
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
#[cfg(test)]
mod tests;
mod utils;

#[doc(hidden)]
pub use crate::argument::{Argument, ArgumentWrapper};
pub use crate::{
    argument::Ascii,
    format::{clip, clip_ascii, fmt, Fmt, FormatArgument, MaxLength, StrLength},
};
use crate::{format::StrFormat, utils::ClippedStr};

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
    /// Capacity of these arguments in bytes.
    pub const CAPACITY: usize = CAP;

    #[doc(hidden)] // Implementation detail of the `compile_args` macro
    #[track_caller]
    pub const fn assert_capacity(required_capacity: usize) {
        compile_assert!(
            CAP >= required_capacity,
            "Insufficient capacity (", CAP => fmt::<usize>(), " bytes) provided \
             for `compile_args` macro; it requires at least ", required_capacity => fmt::<usize>(), " bytes"
        );
    }

    const fn new() -> Self {
        Self {
            buffer: [0_u8; CAP],
            len: 0,
        }
    }

    const fn write_str(self, s: &str, fmt: Option<StrFormat>) -> Self {
        match fmt {
            Some(StrFormat { clip_at, using }) => {
                let clipped = ClippedStr::new(s, clip_at);
                match clipped {
                    ClippedStr::Full(bytes) => self.write_str_bytes(bytes),
                    ClippedStr::Clipped(bytes) => self
                        .write_str_bytes(bytes)
                        .write_str_bytes(using.as_bytes()),
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
    #[doc(hidden)] // implementation detail of crate macros
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

impl<const CAP: usize> FormatArgument for &CompileArgs<CAP> {
    type Details = ();
    const MAX_BYTES_PER_CHAR: usize = 4;
}

impl<const CAP: usize> MaxLength for &CompileArgs<CAP> {
    const MAX_LENGTH: StrLength = StrLength::both(CAP);
    // ^ Here, the byte length is exact and the char length is the pessimistic upper boundary.
}

#[cfg(doctest)]
doc_comment::doctest!("../README.md");
