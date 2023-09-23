//! [`Argument`] and related types.

use crate::{
    format::{Fmt, FormatArgument, StrFormat},
    utils::ClippedStr,
    ConstArgs,
};

#[derive(Debug, Clone, Copy)]
enum ArgumentInner<'a> {
    Str(&'a str, Option<StrFormat>),
    Char(char),
    Int(i128),
    UnsignedInt(u128),
}

impl ArgumentInner<'_> {
    const fn formatted_len(&self) -> usize {
        match self {
            Self::Str(s, None) => s.len(),
            Self::Str(s, Some(fmt)) => match ClippedStr::new(s, fmt.clip_at) {
                ClippedStr::Full(bytes) => bytes.len(),
                ClippedStr::Clipped(bytes) => bytes.len() + fmt.clip_with.len(),
            },
            Self::Char(c) => c.len_utf8(),
            Self::Int(value) => (*value < 0) as usize + log_10_ceil(value.unsigned_abs()),
            Self::UnsignedInt(value) => log_10_ceil(*value),
        }
    }
}

/// Argument in a [`const_concat!`](crate::const_args) macro.
#[derive(Debug, Clone, Copy)]
pub struct Argument<'a> {
    inner: ArgumentInner<'a>,
}

impl Argument<'_> {
    /// Returns the formatted length of the argument in bytes.
    #[doc(hidden)] // only used by crate macros
    pub const fn formatted_len(&self) -> usize {
        self.inner.formatted_len()
    }
}

const fn log_10_ceil(mut value: u128) -> usize {
    if value == 0 {
        return 1;
    }

    let mut log = 0;
    while value > 0 {
        value /= 10;
        log += 1;
    }
    log
}

impl<const CAP: usize> ConstArgs<CAP> {
    const fn write_u128(self, mut value: u128) -> Self {
        let new_len = self.len + log_10_ceil(value);
        let mut buffer = self.buffer;
        let mut pos = new_len - 1;

        loop {
            buffer[pos] = b'0' + (value % 10) as u8;
            if pos == self.len {
                break;
            }
            value /= 10;
            pos -= 1;
        }
        Self {
            buffer,
            len: new_len,
        }
    }

    const fn write_i128(self, value: i128) -> Self {
        let this = if value < 0 {
            self.write_char('-')
        } else {
            self
        };
        this.write_u128(value.unsigned_abs())
    }

    pub(crate) const fn format_arg(self, arg: Argument) -> Self {
        match arg.inner {
            ArgumentInner::Str(s, fmt) => self.write_str(s, fmt),
            // chars and ints are not affected by format so far (i.e., not clipped)
            ArgumentInner::Char(c) => self.write_char(c),
            ArgumentInner::Int(value) => self.write_i128(value),
            ArgumentInner::UnsignedInt(value) => self.write_u128(value),
        }
    }
}

/// Wrapper for an admissible argument type allowing to convert it to an [`Argument`] in compile time.
#[derive(Debug)]
pub struct ArgumentWrapper<T: FormatArgument>(T, Option<T::Details>);

impl<T: FormatArgument> ArgumentWrapper<T> {
    #[doc(hidden)] // used by crate macros
    pub const fn new(value: T) -> Self {
        Self(value, None)
    }

    #[must_use]
    #[doc(hidden)] // used by crate macros
    pub const fn with_fmt(mut self, fmt: &Fmt<T>) -> Self {
        self.1 = Some(fmt.details);
        self
    }
}

impl<'a> ArgumentWrapper<&'a str> {
    /// Performs the conversion.
    pub const fn into_argument(self) -> Argument<'a> {
        Argument {
            inner: ArgumentInner::Str(self.0, self.1),
        }
    }
}

impl ArgumentWrapper<i128> {
    /// Performs the conversion.
    pub const fn into_argument(self) -> Argument<'static> {
        Argument {
            inner: ArgumentInner::Int(self.0),
        }
    }
}

macro_rules! impl_argument_wrapper_for_int {
    ($int:ty) => {
        impl ArgumentWrapper<$int> {
            /// Performs the conversion.
            pub const fn into_argument(self) -> Argument<'static> {
                Argument {
                    inner: ArgumentInner::Int(self.0 as i128),
                }
            }
        }
    };
}

impl_argument_wrapper_for_int!(i8);
impl_argument_wrapper_for_int!(i16);
impl_argument_wrapper_for_int!(i32);
impl_argument_wrapper_for_int!(i64);
impl_argument_wrapper_for_int!(isize);

impl ArgumentWrapper<u128> {
    /// Performs the conversion.
    pub const fn into_argument(self) -> Argument<'static> {
        Argument {
            inner: ArgumentInner::UnsignedInt(self.0),
        }
    }
}

macro_rules! impl_argument_wrapper_for_uint {
    ($uint:ty) => {
        impl ArgumentWrapper<$uint> {
            /// Performs the conversion.
            pub const fn into_argument(self) -> Argument<'static> {
                Argument {
                    inner: ArgumentInner::UnsignedInt(self.0 as u128),
                }
            }
        }
    };
}

impl_argument_wrapper_for_uint!(u8);
impl_argument_wrapper_for_uint!(u16);
impl_argument_wrapper_for_uint!(u32);
impl_argument_wrapper_for_uint!(u64);
impl_argument_wrapper_for_uint!(usize);

impl ArgumentWrapper<char> {
    /// Performs the conversion.
    pub const fn into_argument(self) -> Argument<'static> {
        Argument {
            inner: ArgumentInner::Char(self.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use rand::{rngs::StdRng, Rng, SeedableRng};

    use alloc::string::ToString;

    use super::*;

    #[test]
    fn length_estimation_for_small_ints() {
        for i in 0_u8..=u8::MAX {
            assert_eq!(
                ArgumentWrapper::new(i).into_argument().formatted_len(),
                i.to_string().len(),
                "Formatted length estimated incorrectly for {i}"
            );
        }
        for i in 0_u16..=u16::MAX {
            assert_eq!(
                ArgumentWrapper::new(i).into_argument().formatted_len(),
                i.to_string().len(),
                "Formatted length estimated incorrectly for {i}"
            );
        }
        for i in i8::MIN..=i8::MAX {
            assert_eq!(
                ArgumentWrapper::new(i).into_argument().formatted_len(),
                i.to_string().len(),
                "Formatted length estimated incorrectly for {i}"
            );
        }
        for i in i16::MIN..=i16::MAX {
            assert_eq!(
                ArgumentWrapper::new(i).into_argument().formatted_len(),
                i.to_string().len(),
                "Formatted length estimated incorrectly for {i}"
            );
        }
    }

    #[test]
    fn length_estimation_for_large_ints() {
        const RNG_SEED: u64 = 123;
        const SAMPLE_COUNT: usize = 100_000;

        let mut rng = StdRng::seed_from_u64(RNG_SEED);
        for _ in 0..SAMPLE_COUNT {
            let i: u32 = rng.gen();
            assert_eq!(
                ArgumentWrapper::new(i).into_argument().formatted_len(),
                i.to_string().len(),
                "Formatted length estimated incorrectly for {i}"
            );
        }
        for _ in 0..SAMPLE_COUNT {
            let i: u64 = rng.gen();
            assert_eq!(
                ArgumentWrapper::new(i).into_argument().formatted_len(),
                i.to_string().len(),
                "Formatted length estimated incorrectly for {i}"
            );
        }
        for _ in 0..SAMPLE_COUNT {
            let i: u128 = rng.gen();
            assert_eq!(
                ArgumentWrapper::new(i).into_argument().formatted_len(),
                i.to_string().len(),
                "Formatted length estimated incorrectly for {i}"
            );
        }
        for _ in 0..SAMPLE_COUNT {
            let i: usize = rng.gen();
            assert_eq!(
                ArgumentWrapper::new(i).into_argument().formatted_len(),
                i.to_string().len(),
                "Formatted length estimated incorrectly for {i}"
            );
        }

        for _ in 0..SAMPLE_COUNT {
            let i: i32 = rng.gen();
            assert_eq!(
                ArgumentWrapper::new(i).into_argument().formatted_len(),
                i.to_string().len(),
                "Formatted length estimated incorrectly for {i}"
            );
        }
        for _ in 0..SAMPLE_COUNT {
            let i: i64 = rng.gen();
            assert_eq!(
                ArgumentWrapper::new(i).into_argument().formatted_len(),
                i.to_string().len(),
                "Formatted length estimated incorrectly for {i}"
            );
        }
        for _ in 0..SAMPLE_COUNT {
            let i: i128 = rng.gen();
            assert_eq!(
                ArgumentWrapper::new(i).into_argument().formatted_len(),
                i.to_string().len(),
                "Formatted length estimated incorrectly for {i}"
            );
        }
        for _ in 0..SAMPLE_COUNT {
            let i: isize = rng.gen();
            assert_eq!(
                ArgumentWrapper::new(i).into_argument().formatted_len(),
                i.to_string().len(),
                "Formatted length estimated incorrectly for {i}"
            );
        }
    }

    #[test]
    fn formatted_len_for_clipped_strings() {
        let arg = ArgumentInner::Str(
            "teßt",
            Some(StrFormat {
                clip_at: 2,
                clip_with: "",
            }),
        );
        assert_eq!(arg.formatted_len(), "te".len());

        let arg = ArgumentInner::Str(
            "teßt",
            Some(StrFormat {
                clip_at: 2,
                clip_with: "...",
            }),
        );
        assert_eq!(arg.formatted_len(), "te...".len());

        let arg = ArgumentInner::Str(
            "teßt",
            Some(StrFormat {
                clip_at: 2,
                clip_with: "…",
            }),
        );
        assert_eq!(arg.formatted_len(), "te…".len());

        let arg = ArgumentInner::Str(
            "teßt",
            Some(StrFormat {
                clip_at: 3,
                clip_with: "",
            }),
        );
        assert_eq!(arg.formatted_len(), "teß".len());

        let arg = ArgumentInner::Str(
            "teßt",
            Some(StrFormat {
                clip_at: 3,
                clip_with: "…",
            }),
        );
        assert_eq!(arg.formatted_len(), "teß…".len());

        let arg = ArgumentInner::Str(
            "teßt",
            Some(StrFormat {
                clip_at: 3,
                clip_with: "-",
            }),
        );
        assert_eq!(arg.formatted_len(), "teß-".len());

        for clip_at in [4, 5, 16] {
            for clip_with in ["", "...", "…"] {
                let arg = ArgumentInner::Str("teßt", Some(StrFormat { clip_at, clip_with }));
                assert_eq!(arg.formatted_len(), "teßt".len());
            }
        }
    }
}
