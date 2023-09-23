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
    format::{Fmt, MaxWidth},
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

impl<const CAP: usize> ConstArgs<CAP> {
    const fn new() -> Self {
        Self {
            buffer: [0_u8; CAP],
            len: 0,
        }
    }

    const fn write_str(self, s: &str) -> Self {
        let new_len = self.len + s.len();
        let mut buffer = self.buffer;
        let mut pos = self.len;

        while pos < new_len {
            buffer[pos] = s.as_bytes()[pos - self.len];
            pos += 1;
        }
        Self {
            buffer,
            len: new_len,
        }
    }

    /// Formats the provided sequence of [`Argument`]s.
    pub const fn format(arguments: &[Argument]) -> Self {
        // Assert argument capacities first.
        let mut arg_i = 0;
        while arg_i < arguments.len() {
            arguments[arg_i].assert_width(arg_i);
            arg_i += 1;
        }

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
    #[should_panic(expected = "expected 1 to be greater than 32")]
    fn assertion() {
        let value = 1;
        const_assert!(
            value > THRESHOLD,
            "expected ", value => Fmt::width(4), " to be greater than ", THRESHOLD
        );
    }

    #[test]
    #[should_panic(expected = "Argument #1 has insufficient byte width (4); required at least 6")]
    fn insufficient_capacity() {
        const_args!("expected ", 111_111_usize => Fmt::width(4), " to be greater than ", THRESHOLD);
    }
}
