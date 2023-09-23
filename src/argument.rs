//! [`Argument`] and related types.

use crate::{format::Fmt, ConstArgs};

#[derive(Debug, Clone, Copy)]
enum ArgumentInner {
    Str(&'static str),
    Int(i128),
    UnsignedInt(u128),
}

impl ArgumentInner {
    const fn formatted_len(&self) -> usize {
        match self {
            Self::Str(s) => s.len(),
            Self::Int(value) => (*value < 0) as usize + log_10_ceil(value.unsigned_abs()),
            Self::UnsignedInt(value) => log_10_ceil(*value),
        }
    }
}

/// Argument in a [`const_concat!`](crate::const_args) macro.
#[derive(Debug, Clone, Copy)]
pub struct Argument {
    inner: ArgumentInner,
    fmt: Option<Fmt>,
}

impl Argument {
    /// Returns the formatted length of the argument in bytes.
    #[doc(hidden)] // only used by crate macros
    pub const fn formatted_len(&self) -> usize {
        if let Some(fmt) = &self.fmt {
            fmt.formatted_len()
        } else {
            self.inner.formatted_len()
        }
    }

    pub(crate) const fn assert_width(&self, arg_index: usize) {
        if let Some(fmt) = &self.fmt {
            let fmt_len = fmt.formatted_len();
            let inherent_len = self.inner.formatted_len();
            crate::const_assert!(
                fmt_len >= inherent_len,
                "Argument #", arg_index => Fmt::of::<usize>(),
                " has insufficient byte width (", fmt_len => Fmt::of::<usize>(),
                "); required at least ", inherent_len => Fmt::of::<usize>()
            );
        }
    }

    /// Specifies the format for this argument.
    #[must_use]
    #[doc(hidden)] // only used by crate macros
    pub const fn with_fmt(mut self, fmt: Fmt) -> Self {
        self.fmt = Some(fmt);
        self
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
        let this = if value < 0 { self.write_str("-") } else { self };
        this.write_u128(value.unsigned_abs())
    }

    pub(crate) const fn format_arg(self, arg: Argument) -> Self {
        match arg.inner {
            ArgumentInner::Str(s) => self.write_str(s),
            ArgumentInner::Int(value) => self.write_i128(value),
            ArgumentInner::UnsignedInt(value) => self.write_u128(value),
        }
    }
}

/// Wrapper for an admissible argument type allowing to convert it to an [`Argument`] in compile time.
#[derive(Debug)]
pub struct ArgumentWrapper<T>(pub T);

impl ArgumentWrapper<&'static str> {
    /// Performs the conversion.
    pub const fn into_argument(self) -> Argument {
        Argument {
            inner: ArgumentInner::Str(self.0),
            fmt: None,
        }
    }
}

impl ArgumentWrapper<i128> {
    /// Performs the conversion.
    pub const fn into_argument(self) -> Argument {
        Argument {
            inner: ArgumentInner::Int(self.0),
            fmt: None,
        }
    }
}

macro_rules! impl_argument_wrapper_for_int {
    ($int:ty) => {
        impl ArgumentWrapper<$int> {
            /// Performs the conversion.
            pub const fn into_argument(self) -> Argument {
                Argument {
                    inner: ArgumentInner::Int(self.0 as i128),
                    fmt: None,
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
    pub const fn into_argument(self) -> Argument {
        Argument {
            inner: ArgumentInner::UnsignedInt(self.0),
            fmt: None,
        }
    }
}

macro_rules! impl_argument_wrapper_for_uint {
    ($int:ty) => {
        impl ArgumentWrapper<$int> {
            /// Performs the conversion.
            pub const fn into_argument(self) -> Argument {
                Argument {
                    inner: ArgumentInner::UnsignedInt(self.0 as u128),
                    fmt: None,
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

#[cfg(test)]
mod tests {
    use rand::{rngs::StdRng, Rng, SeedableRng};

    use alloc::string::ToString;

    use super::*;

    #[test]
    fn length_estimation_for_small_ints() {
        for i in 0_u8..=u8::MAX {
            assert_eq!(
                ArgumentWrapper(i).into_argument().formatted_len(),
                i.to_string().len(),
                "Formatted length estimated incorrectly for {i}"
            );
        }
        for i in 0_u16..=u16::MAX {
            assert_eq!(
                ArgumentWrapper(i).into_argument().formatted_len(),
                i.to_string().len(),
                "Formatted length estimated incorrectly for {i}"
            );
        }
        for i in i8::MIN..=i8::MAX {
            assert_eq!(
                ArgumentWrapper(i).into_argument().formatted_len(),
                i.to_string().len(),
                "Formatted length estimated incorrectly for {i}"
            );
        }
        for i in i16::MIN..=i16::MAX {
            assert_eq!(
                ArgumentWrapper(i).into_argument().formatted_len(),
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
                ArgumentWrapper(i).into_argument().formatted_len(),
                i.to_string().len(),
                "Formatted length estimated incorrectly for {i}"
            );
        }
        for _ in 0..SAMPLE_COUNT {
            let i: u64 = rng.gen();
            assert_eq!(
                ArgumentWrapper(i).into_argument().formatted_len(),
                i.to_string().len(),
                "Formatted length estimated incorrectly for {i}"
            );
        }
        for _ in 0..SAMPLE_COUNT {
            let i: u128 = rng.gen();
            assert_eq!(
                ArgumentWrapper(i).into_argument().formatted_len(),
                i.to_string().len(),
                "Formatted length estimated incorrectly for {i}"
            );
        }
        for _ in 0..SAMPLE_COUNT {
            let i: usize = rng.gen();
            assert_eq!(
                ArgumentWrapper(i).into_argument().formatted_len(),
                i.to_string().len(),
                "Formatted length estimated incorrectly for {i}"
            );
        }

        for _ in 0..SAMPLE_COUNT {
            let i: i32 = rng.gen();
            assert_eq!(
                ArgumentWrapper(i).into_argument().formatted_len(),
                i.to_string().len(),
                "Formatted length estimated incorrectly for {i}"
            );
        }
        for _ in 0..SAMPLE_COUNT {
            let i: i64 = rng.gen();
            assert_eq!(
                ArgumentWrapper(i).into_argument().formatted_len(),
                i.to_string().len(),
                "Formatted length estimated incorrectly for {i}"
            );
        }
        for _ in 0..SAMPLE_COUNT {
            let i: i128 = rng.gen();
            assert_eq!(
                ArgumentWrapper(i).into_argument().formatted_len(),
                i.to_string().len(),
                "Formatted length estimated incorrectly for {i}"
            );
        }
        for _ in 0..SAMPLE_COUNT {
            let i: isize = rng.gen();
            assert_eq!(
                ArgumentWrapper(i).into_argument().formatted_len(),
                i.to_string().len(),
                "Formatted length estimated incorrectly for {i}"
            );
        }
    }
}
