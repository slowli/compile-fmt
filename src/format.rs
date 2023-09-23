//! `Fmt` and related types.

/// Formatting specification for an [`Argument`](crate::Argument).
///
/// A format is necessary to specify for *dynamic* arguments of [`const_args!`](crate::const_args)
/// and related macros (i.e., for arguments that are not constants). For now, the only meaningful
/// format customization is provided for strings (`&str`). All other arguments have the only
/// available format that can be created using [`fmt()`].
///
/// # Examples
///
/// ## Truncating a string to a certain width
///
/// ```
/// use const_fmt::{const_args, fmt, Fmt, ConstArgs};
///
/// // FIXME: should work with non-static strings
/// const fn truncate(s: &'static str) -> impl AsRef<str> {
///     const_args!(
///         "Truncated string: '", s => Fmt::truncated(8),
///         "', original length: ", s.len() => fmt::<usize>()
///     )
/// }
///
/// let s = truncate("very long string indeed");
/// assert_eq!(
///     s.as_ref(),
///     "Truncated string: 'very lon', original length: 23"
/// );
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Fmt<T: FormatArgument> {
    byte_width: usize,
    pub(crate) details: T::Details,
}

/// Creates a default format for a type that has known bounded formatting width.
pub const fn fmt<T>() -> Fmt<T>
where
    T: FormatArgument<Details = ()> + MaxWidth,
{
    Fmt {
        byte_width: T::MAX_WIDTH,
        details: (),
    }
}

impl Fmt<&str> {
    /// Creates a format that will truncate the value to the specified max **char** width.
    ///
    /// # Panics
    ///
    /// Panics if `width` is zero.
    pub const fn truncated(truncate_at: usize) -> Self {
        assert!(truncate_at > 0, "Truncation width must be positive");
        Self {
            byte_width: truncate_at * char::MAX_WIDTH,
            details: StrFormat { truncate_at },
        }
    }
}

impl<T: FormatArgument> Fmt<T> {
    /// Returns the formatted length of the argument in bytes.
    #[doc(hidden)] // only used by macros
    pub const fn formatted_len(&self) -> usize {
        self.byte_width
    }
}

/// Type that can be formatted. Implemented for standard integer types, `&str` and `char`.
pub trait FormatArgument {
    /// Formatting specification for the type.
    type Details: 'static + Copy;
}

impl FormatArgument for &str {
    type Details = StrFormat;
}

/// Formatting details for strings.
#[derive(Debug, Clone, Copy)]
pub struct StrFormat {
    pub(crate) truncate_at: usize,
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
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;

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
}
