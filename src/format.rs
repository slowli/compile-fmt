//! `Fmt` and related types.

use core::fmt::Alignment;

use crate::utils::count_chars;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct FormattedLen {
    pub bytes: usize,
    pub chars: usize,
}

impl FormattedLen {
    pub const fn for_str(s: &str) -> Self {
        Self {
            bytes: s.len(),
            chars: count_chars(s),
        }
    }

    pub const fn for_char(c: char) -> Self {
        Self {
            bytes: c.len_utf8(),
            chars: 1,
        }
    }

    pub const fn ascii(bytes: usize) -> Self {
        Self {
            bytes,
            chars: bytes,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Pad {
    pub align: Alignment,
    pub width: usize,
    pub using: char,
}

impl Pad {
    pub const fn compute_padding(&self, char_count: usize) -> (usize, usize) {
        if char_count >= self.width {
            return (0, 0);
        }
        match self.align {
            Alignment::Left => (0, self.width - char_count),
            Alignment::Right => (self.width - char_count, 0),
            Alignment::Center => {
                let total_padding = self.width - char_count;
                (total_padding / 2, total_padding - total_padding / 2)
            }
        }
    }
}

/// Formatting specification for an [`Argument`](crate::Argument).
///
/// A format is necessary to specify for *dynamic* arguments of [`compile_args!`](crate::compile_args)
/// and related macros (i.e., for arguments that are not constants). For now, the only meaningful
/// format customization is provided for strings (`&str`). All other arguments have the only
/// available format that can be created using [`fmt()`].
///
/// # Examples
///
/// ## Clipping string to certain width
///
/// ```
/// use compile_fmt::{compile_args, clip, fmt};
///
/// const fn format_clipped_str(s: &str) -> impl AsRef<str> {
///     compile_args!(
///         "Clipped string: '", s => clip(8, "…"),
///         "', original length: ", s.len() => fmt::<usize>()
///     )
/// }
///
/// let s = format_clipped_str("very long string indeed");
/// assert_eq!(
///     s.as_ref(),
///     "Clipped string: 'very lon…', original length: 23"
/// );
/// ```
///
/// ## Padding
///
/// ```
/// # use compile_fmt::{compile_args, fmt};
/// const fn format_with_padding(value: u32) -> impl AsRef<str> {
///     compile_args!(
///         "Number: ", value => fmt::<u32>().pad_right(4, '0')
///     )
/// }
///
/// let s = format_with_padding(42);
/// assert_eq!(s.as_ref(), "Number: 0042");
/// let s = format_with_padding(19_999);
/// assert_eq!(s.as_ref(), "Number: 19999");
/// // ^ If the string before padding contains more chars than in the padding spec,
/// // padding is not applied at all.
/// ```
///
/// Any Unicode char can be used as padding:
///
/// ```
/// # use compile_fmt::{compile_args, fmt};
/// let s = compile_args!(
///     "Number: ", 42 => fmt::<u32>().pad_left(4, '💣')
/// );
/// assert_eq!(s.as_str(), "Number: 42💣💣");
/// ```
///
/// Strings can be padded as well:
///
/// ```
/// # use compile_fmt::{compile_args, clip};
/// const fn pad_str(s: &str) -> impl AsRef<str> {
///     compile_args!("[", s => clip(8, "").pad_center(8, ' '), "]")
/// }
///
/// assert_eq!(pad_str("test").as_ref(), "[  test  ]");
/// assert_eq!(pad_str("test!").as_ref(), "[ test!  ]");
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Fmt<T: FormatArgument> {
    /// Byte capacity of the format without taking padding into account. This is a field
    /// rather than a method in `FormatArgument` because we wouldn't be able to call this method
    /// in `const fn`s.
    capacity: FormattedLen,
    pub(crate) details: T::Details,
    pub(crate) pad: Option<Pad>,
}

/// Creates a default format for a type that has known bounded formatting width.
pub const fn fmt<T>() -> Fmt<T>
where
    T: FormatArgument<Details = ()> + MaxWidth,
{
    Fmt {
        capacity: FormattedLen {
            bytes: T::MAX_WIDTH * T::MAX_BYTES_PER_CHAR,
            chars: T::MAX_WIDTH,
        },
        details: (),
        pad: None,
    }
}

/// Creates a format that will clip the value to the specified max **char** width (not byte width!).
/// If clipped, the end of the string will be replaced with the specified replacer, which can be empty.
///
/// # Panics
///
/// Panics if `clip_at` is zero.
pub const fn clip<'a>(clip_at: usize, clip_with: &'static str) -> Fmt<&'a str> {
    assert!(clip_at > 0, "Clip width must be positive");
    Fmt {
        capacity: FormattedLen {
            bytes: clip_at * char::MAX_WIDTH + clip_with.len(),
            chars: clip_at + count_chars(clip_with),
        },
        details: StrFormat { clip_at, clip_with },
        pad: None,
    }
}

impl<T: FormatArgument> Fmt<T> {
    const fn pad(mut self, align: Alignment, width: usize, using: char) -> Self {
        let pad = Pad {
            align,
            width,
            using,
        };
        self.pad = Some(pad);
        self
    }

    /// Specifies left-aligned padding.
    #[must_use]
    pub const fn pad_left(self, width: usize, using: char) -> Self {
        self.pad(Alignment::Left, width, using)
    }

    /// Specifies right-aligned padding.
    #[must_use]
    pub const fn pad_right(self, width: usize, using: char) -> Self {
        self.pad(Alignment::Right, width, using)
    }

    /// Specifies center-aligned padding.
    #[must_use]
    pub const fn pad_center(self, width: usize, using: char) -> Self {
        self.pad(Alignment::Center, width, using)
    }

    /// Returns the byte capacity of this format in bytes.
    #[doc(hidden)] // only used by macros
    pub const fn capacity(&self) -> usize {
        if let Some(pad) = &self.pad {
            // Capacity necessary for an empty non-padded string (which we assume is always possible).
            let full_pad_capacity = pad.using.len_utf8() * pad.width;

            let max_width = if self.capacity.chars > pad.width {
                pad.width
            } else {
                self.capacity.chars
            };
            // Capacity necessary for the maximum-length string that still has padding.
            let min_pad_capacity =
                pad.using.len_utf8() * (pad.width - max_width) + max_width * T::MAX_BYTES_PER_CHAR;

            // Select maximum of `max_pad_capacity`, `min_pad_capacity` and the original capacity.
            let pad_capacity = if full_pad_capacity > min_pad_capacity {
                full_pad_capacity
            } else {
                min_pad_capacity
            };
            if pad_capacity > self.capacity.bytes {
                return pad_capacity;
            }
        }
        self.capacity.bytes
    }
}

/// Type that can be formatted. Implemented for standard integer types, `&str` and `char`.
pub trait FormatArgument {
    /// Formatting specification for the type.
    type Details: 'static + Copy;
    /// Maximum number of bytes a single char from this format can occupy.
    #[doc(hidden)] // implementation detail
    const MAX_BYTES_PER_CHAR: usize;
}

impl FormatArgument for &str {
    type Details = StrFormat;
    const MAX_BYTES_PER_CHAR: usize = 4;
}

/// Formatting details for strings.
#[derive(Debug, Clone, Copy)]
pub struct StrFormat {
    pub(crate) clip_at: usize,
    pub(crate) clip_with: &'static str,
}

/// Type that has a known upper boundary for the formatted length.
pub trait MaxWidth {
    /// Upper boundary for the formatted length in bytes.
    const MAX_WIDTH: usize;
}

macro_rules! impl_max_width_for_uint {
    ($($uint:ty),+) => {
        $(
        impl MaxWidth for $uint {
            const MAX_WIDTH: usize =
                crate::ArgumentWrapper::new(Self::MAX).into_argument().formatted_len();
        }

        impl FormatArgument for $uint {
            type Details = ();
            const MAX_BYTES_PER_CHAR: usize = 1;
        }
        )+
    };
}

impl_max_width_for_uint!(u8, u16, u32, u64, u128, usize);

macro_rules! impl_max_width_for_int {
    ($($int:ty),+) => {
        $(
        impl MaxWidth for $int {
            const MAX_WIDTH: usize =
                crate::ArgumentWrapper::new(Self::MIN).into_argument().formatted_len();
        }

        impl FormatArgument for $int {
            type Details = ();
            const MAX_BYTES_PER_CHAR: usize = 1;
        }
        )+
    };
}

impl_max_width_for_int!(i8, i16, i32, i64, i128, isize);

impl MaxWidth for char {
    const MAX_WIDTH: usize = 4;
}

impl FormatArgument for char {
    type Details = ();
    const MAX_BYTES_PER_CHAR: usize = 4;
}

#[cfg(test)]
mod tests {
    use std::string::ToString;

    use super::*;

    #[test]
    fn max_length_bound_is_correct() {
        assert_eq!(u8::MAX_WIDTH, u8::MAX.to_string().len());
        assert_eq!(u16::MAX_WIDTH, u16::MAX.to_string().len());
        assert_eq!(u32::MAX_WIDTH, u32::MAX.to_string().len());
        assert_eq!(u64::MAX_WIDTH, u64::MAX.to_string().len());
        assert_eq!(u128::MAX_WIDTH, u128::MAX.to_string().len());
        assert_eq!(usize::MAX_WIDTH, usize::MAX.to_string().len());

        assert_eq!(i8::MAX_WIDTH, i8::MIN.to_string().len());
        assert_eq!(i16::MAX_WIDTH, i16::MIN.to_string().len());
        assert_eq!(i32::MAX_WIDTH, i32::MIN.to_string().len());
        assert_eq!(i64::MAX_WIDTH, i64::MIN.to_string().len());
        assert_eq!(i128::MAX_WIDTH, i128::MIN.to_string().len());
        assert_eq!(isize::MAX_WIDTH, isize::MIN.to_string().len());
    }

    #[test]
    fn capacity_for_padded_format() {
        let format = fmt::<u8>().pad(Alignment::Right, 8, ' ');
        assert_eq!(format.capacity(), 8);
        let format = fmt::<u8>().pad(Alignment::Right, 8, 'ℝ');
        assert_eq!(format.capacity(), 24); // each padding char is 3 bytes
        let format = fmt::<u64>().pad(Alignment::Right, 8, ' ');
        assert_eq!(format.capacity(), u64::MAX.to_string().len()); // original capacity

        let format = clip(8, "").pad(Alignment::Left, 8, ' ');
        assert_eq!(format.capacity.chars, 8);
        assert_eq!(format.capacity.bytes, 32);
        assert_eq!(format.capacity(), 32);

        let format = clip(4, "").pad(Alignment::Left, 8, ' ');
        assert_eq!(format.capacity.chars, 4);
        assert_eq!(format.capacity.bytes, 16);
        assert_eq!(format.capacity(), 20); // 16 + 4 padding chars

        let format = clip(4, "").pad(Alignment::Left, 8, 'ß');
        assert_eq!(format.capacity.chars, 4);
        assert_eq!(format.capacity.bytes, 16);
        assert_eq!(format.capacity(), 24); // 16 + 4 padding chars * 2 bytes each

        let format = clip(4, "…").pad(Alignment::Left, 8, ' ');
        assert_eq!(format.capacity.chars, 5);
        assert_eq!(format.capacity.bytes, 16 + "…".len());
        assert_eq!(format.capacity(), 23); // 20 (5 chars * 4 bytes) + 3 padding chars * 4 bytes each
    }
}
