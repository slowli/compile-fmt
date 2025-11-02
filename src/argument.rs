//! [`Argument`] and related types.

use core::fmt;

use crate::{
    format::{Fmt, FormatArgument, Pad, StrFormat, StrLength},
    utils::{assert_is_ascii, count_chars, ClippedStr},
    CompileArgs,
};

#[derive(Debug, Clone, Copy)]
enum ArgumentInner<'a> {
    Str(&'a str, Option<StrFormat>),
    Char(char),
    Int(i128),
    UnsignedInt(u128),
}

impl ArgumentInner<'_> {
    const fn formatted_len(&self) -> StrLength {
        match self {
            Self::Str(s, None) => StrLength::for_str(s),
            Self::Str(s, Some(fmt)) => match ClippedStr::new(s, fmt.clip_at) {
                ClippedStr::Full(_) => StrLength::for_str(s),
                ClippedStr::Clipped(bytes) => StrLength {
                    bytes: bytes.len() + fmt.using.len(),
                    chars: fmt.clip_at + count_chars(fmt.using),
                },
            },
            Self::Char(c) => StrLength::for_char(*c),
            Self::Int(value) => {
                let bytes = (*value < 0) as usize + log_10_ceil(value.unsigned_abs());
                StrLength::both(bytes)
            }
            Self::UnsignedInt(value) => StrLength::both(log_10_ceil(*value)),
        }
    }
}

/// Generalized argument in crate macros.
#[doc(hidden)] // implementation detail of crate macros
#[derive(Debug, Clone, Copy)]
pub struct Argument<'a> {
    inner: ArgumentInner<'a>,
    pad: Option<Pad>,
}

impl Argument<'_> {
    /// Returns the formatted length of the argument in bytes.
    pub const fn formatted_len(&self) -> usize {
        let non_padded_len = self.inner.formatted_len();
        if let Some(pad) = &self.pad {
            if pad.width > non_padded_len.chars {
                let pad_char_count = pad.width - non_padded_len.chars;
                pad_char_count * pad.using.len_utf8() + non_padded_len.bytes
            } else {
                // The non-padded string is longer than the pad width; it won't be padded
                non_padded_len.bytes
            }
        } else {
            non_padded_len.bytes
        }
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

impl<const CAP: usize> CompileArgs<CAP> {
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

    pub(crate) const fn format_arg(mut self, arg: Argument) -> Self {
        let pad_after = 'compute_pad: {
            if let Some(pad) = &arg.pad {
                // Check if the argument must be padded.
                let non_padded_len = arg.inner.formatted_len();
                if pad.width > non_padded_len.chars {
                    let (pad_before, pad_after) = pad.compute_padding(non_padded_len.chars);
                    let mut count = 0;
                    while count < pad_before {
                        self = self.write_char(pad.using);
                        count += 1;
                    }
                    break 'compute_pad Some((pad_after, pad.using));
                }
            }
            None
        };

        self = match arg.inner {
            ArgumentInner::Str(s, fmt) => self.write_str(s, fmt),
            // chars and ints are not affected by format so far (i.e., not clipped)
            ArgumentInner::Char(c) => self.write_char(c),
            ArgumentInner::Int(value) => self.write_i128(value),
            ArgumentInner::UnsignedInt(value) => self.write_u128(value),
        };
        if let Some((pad_after, using)) = pad_after {
            let mut count = 0;
            while count < pad_after {
                self = self.write_char(using);
                count += 1;
            }
        }
        self
    }
}

/// ASCII string wrapper.
///
/// This wrapper is useful for non-constant strings if it can be ensured that a string consists
/// entirely of ASCII chars. This allows decreasing capacity requirements for [`CompileArgs`]
/// involving such strings. In the general case, `CompileArgs` logic must assume that each char
/// in a string can require up to 4 bytes; in case of `Ascii` strings, this is reduced to
/// 1 byte per char.
///
/// # Examples
///
/// ```
/// use compile_fmt::{clip, clip_ascii, compile_args, Ascii, CompileArgs};
///
/// let s: CompileArgs<10> = compile_args!(
///     "[", Ascii::new("test") => clip_ascii(8, "").pad_left(8, ' '), "]"
/// );
/// assert_eq!(s.as_str(), "[test    ]");
///
/// // The necessary capacity for generic UTF-8 strings is greater
/// // (34 bytes instead of 10):
/// let s: CompileArgs<34> = compile_args!(
///     "[", "test" => clip(8, "").pad_left(8, ' '), "]"
/// );
/// assert_eq!(s.as_str(), "[test    ]");
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Ascii<'a>(pub(crate) &'a str);

impl<'a> Ascii<'a> {
    /// Wraps the provided string if it consists entirely of ASCII chars.
    ///
    /// # Panics
    ///
    /// Panics if the string contains non-ASCII chars.
    pub const fn new(s: &'a str) -> Self {
        assert_is_ascii(s);
        Self(s)
    }
}

/// Wrapper for an admissible argument type allowing to convert it to an [`Argument`] in compile time.
#[doc(hidden)] // implementation detail of crate macros
pub struct ArgumentWrapper<T: FormatArgument> {
    value: T,
    fmt: Option<Fmt<T>>,
}

impl<T> fmt::Debug for ArgumentWrapper<T>
where
    T: FormatArgument + fmt::Debug,
    T::Details: fmt::Debug,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ArgumentWrapper")
            .field("value", &self.value)
            .field("fmt", &self.fmt)
            .finish()
    }
}

impl<T: FormatArgument> ArgumentWrapper<T> {
    pub const fn new(value: T) -> Self {
        Self { value, fmt: None }
    }

    #[must_use]
    pub const fn with_fmt(mut self, fmt: Fmt<T>) -> Self {
        self.fmt = Some(fmt);
        self
    }
}

impl<'a> ArgumentWrapper<&'a str> {
    /// Performs the conversion.
    pub const fn into_argument(self) -> Argument<'a> {
        let (str_fmt, pad) = match self.fmt {
            Some(Fmt { details, pad, .. }) => (Some(details), pad),
            None => (None, None),
        };
        Argument {
            inner: ArgumentInner::Str(self.value, str_fmt),
            pad,
        }
    }
}

impl<'a> ArgumentWrapper<Ascii<'a>> {
    /// Performs the conversion.
    pub const fn into_argument(self) -> Argument<'a> {
        let (str_fmt, pad) = match self.fmt {
            Some(Fmt { details, pad, .. }) => (Some(details), pad),
            None => (None, None),
        };
        Argument {
            inner: ArgumentInner::Str(self.value.0, str_fmt),
            pad,
        }
    }
}

impl<'a, const CAP: usize> ArgumentWrapper<&'a CompileArgs<CAP>> {
    /// Performs the conversion.
    pub const fn into_argument(self) -> Argument<'a> {
        Argument {
            inner: ArgumentInner::Str(self.value.as_str(), None),
            pad: None,
        }
    }
}

impl ArgumentWrapper<i128> {
    /// Performs the conversion.
    pub const fn into_argument(self) -> Argument<'static> {
        let pad = match self.fmt {
            Some(Fmt { pad, .. }) => pad,
            None => None,
        };
        Argument {
            inner: ArgumentInner::Int(self.value),
            pad,
        }
    }
}

macro_rules! impl_argument_wrapper_for_int {
    ($int:ty) => {
        impl ArgumentWrapper<$int> {
            /// Performs the conversion.
            pub const fn into_argument(self) -> Argument<'static> {
                let pad = match self.fmt {
                    Some(Fmt { pad, .. }) => pad,
                    None => None,
                };
                Argument {
                    inner: ArgumentInner::Int(self.value as i128),
                    pad,
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
        let pad = match self.fmt {
            Some(Fmt { pad, .. }) => pad,
            None => None,
        };
        Argument {
            inner: ArgumentInner::UnsignedInt(self.value),
            pad,
        }
    }
}

macro_rules! impl_argument_wrapper_for_uint {
    ($uint:ty) => {
        impl ArgumentWrapper<$uint> {
            /// Performs the conversion.
            pub const fn into_argument(self) -> Argument<'static> {
                let pad = match self.fmt {
                    Some(Fmt { pad, .. }) => pad,
                    None => None,
                };
                Argument {
                    inner: ArgumentInner::UnsignedInt(self.value as u128),
                    pad,
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
        let pad = match self.fmt {
            Some(Fmt { pad, .. }) => pad,
            None => None,
        };
        Argument {
            inner: ArgumentInner::Char(self.value),
            pad,
        }
    }
}

#[cfg(test)]
mod tests {
    use rand::{rngs::StdRng, Rng, SeedableRng};

    use super::*;
    use core::fmt::Alignment;
    use rand::distr::uniform::{UniformSampler, UniformUsize};
    use std::string::ToString;

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
            let i: u32 = rng.random();
            assert_eq!(
                ArgumentWrapper::new(i).into_argument().formatted_len(),
                i.to_string().len(),
                "Formatted length estimated incorrectly for {i}"
            );
        }
        for _ in 0..SAMPLE_COUNT {
            let i: u64 = rng.random();
            assert_eq!(
                ArgumentWrapper::new(i).into_argument().formatted_len(),
                i.to_string().len(),
                "Formatted length estimated incorrectly for {i}"
            );
        }
        for _ in 0..SAMPLE_COUNT {
            let i: u128 = rng.random();
            assert_eq!(
                ArgumentWrapper::new(i).into_argument().formatted_len(),
                i.to_string().len(),
                "Formatted length estimated incorrectly for {i}"
            );
        }

        let sampler = UniformUsize::new_inclusive(0, usize::MAX).unwrap();
        for _ in 0..SAMPLE_COUNT {
            let i = sampler.sample(&mut rng);
            assert_eq!(
                ArgumentWrapper::new(i).into_argument().formatted_len(),
                i.to_string().len(),
                "Formatted length estimated incorrectly for {i}"
            );
        }

        for _ in 0..SAMPLE_COUNT {
            let i: i32 = rng.random();
            assert_eq!(
                ArgumentWrapper::new(i).into_argument().formatted_len(),
                i.to_string().len(),
                "Formatted length estimated incorrectly for {i}"
            );
        }
        for _ in 0..SAMPLE_COUNT {
            let i: i64 = rng.random();
            assert_eq!(
                ArgumentWrapper::new(i).into_argument().formatted_len(),
                i.to_string().len(),
                "Formatted length estimated incorrectly for {i}"
            );
        }
        for _ in 0..SAMPLE_COUNT {
            let i: i128 = rng.random();
            assert_eq!(
                ArgumentWrapper::new(i).into_argument().formatted_len(),
                i.to_string().len(),
                "Formatted length estimated incorrectly for {i}"
            );
        }

        for _ in 0..SAMPLE_COUNT {
            let i = sampler.sample(&mut rng).cast_signed();
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
                using: "",
            }),
        );
        assert_eq!(arg.formatted_len(), StrLength::for_str("te"));

        let arg = ArgumentInner::Str(
            "teßt",
            Some(StrFormat {
                clip_at: 2,
                using: "...",
            }),
        );
        assert_eq!(arg.formatted_len(), StrLength::for_str("te..."));

        let arg = ArgumentInner::Str(
            "teßt",
            Some(StrFormat {
                clip_at: 2,
                using: "…",
            }),
        );
        assert_eq!(arg.formatted_len(), StrLength::for_str("te…"));

        let arg = ArgumentInner::Str(
            "teßt",
            Some(StrFormat {
                clip_at: 3,
                using: "",
            }),
        );
        assert_eq!(arg.formatted_len(), StrLength::for_str("teß"));

        let arg = ArgumentInner::Str(
            "teßt",
            Some(StrFormat {
                clip_at: 3,
                using: "…",
            }),
        );
        assert_eq!(arg.formatted_len(), StrLength::for_str("teß…"));

        let arg = ArgumentInner::Str(
            "teßt",
            Some(StrFormat {
                clip_at: 3,
                using: "-",
            }),
        );
        assert_eq!(arg.formatted_len(), StrLength::for_str("teß-"));

        for clip_at in [4, 5, 16] {
            for using in ["", "...", "…"] {
                let arg = ArgumentInner::Str("teßt", Some(StrFormat { clip_at, using }));
                assert_eq!(arg.formatted_len(), StrLength::for_str("teßt"));
            }
        }
    }

    #[test]
    fn formatted_len_with_padding() {
        let argument = Argument {
            inner: ArgumentInner::Str("teßt", None),
            pad: Some(Pad {
                align: Alignment::Left,
                width: 8,
                using: ' ',
            }),
        };
        assert_eq!(argument.formatted_len(), "teßt    ".len());

        let argument = Argument {
            inner: ArgumentInner::Str("teßt", None),
            pad: Some(Pad {
                align: Alignment::Left,
                width: 8,
                using: '💣',
            }),
        };
        assert_eq!(argument.formatted_len(), "teßt💣💣💣💣".len());

        for pad_width in 1..=4 {
            let argument = Argument {
                inner: ArgumentInner::Str("teßt", None),
                pad: Some(Pad {
                    align: Alignment::Left,
                    width: pad_width,
                    using: ' ',
                }),
            };
            assert_eq!(argument.formatted_len(), "teßt".len());
        }
    }

    #[test]
    fn formatted_len_with_padding_and_clipping() {
        let inner = ArgumentInner::Str(
            "teßt",
            Some(StrFormat {
                clip_at: 3,
                using: "…",
            }),
        );
        let argument = Argument {
            inner,
            pad: Some(Pad {
                align: Alignment::Left,
                width: 8,
                using: ' ',
            }),
        };
        assert_eq!(argument.formatted_len(), "teß…    ".len());

        let argument = Argument {
            inner,
            pad: Some(Pad {
                align: Alignment::Left,
                width: 8,
                using: '💣',
            }),
        };
        assert_eq!(argument.formatted_len(), "teß…💣💣💣💣".len());

        for pad_width in 1..=4 {
            let argument = Argument {
                inner,
                pad: Some(Pad {
                    align: Alignment::Left,
                    width: pad_width,
                    using: ' ',
                }),
            };
            assert_eq!(argument.formatted_len(), "teß…".len());
        }
    }

    #[test]
    #[should_panic(expected = "String 'teß…' contains non-ASCII chars; first at position 2")]
    fn ascii_panic() {
        Ascii::new("teß…");
    }
}
