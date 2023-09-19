use core::{fmt, slice, str};

#[macro_export]
macro_rules! const_concat {
    ($($arg:expr $(=> $fmt:expr)?),+) => {{
        const __CAPACITY: usize = $crate::const_concat!(@total_capacity $($arg $(=> $fmt)?,)+);
        let __arguments: &[$crate::Argument] = &[
            $($crate::ArgumentWrapper($arg).into_argument()$(.with_fmt($fmt))?,)+
        ];
        $crate::Formatter::<__CAPACITY>::format(__arguments)
    }};
    (@total_capacity $first_arg:expr $(=> $first_fmt:expr)?, $($arg:expr $(=> $fmt:expr)?,)*) => {
        $crate::const_concat!(@arg_capacity $first_arg $(=> $first_fmt)?)
            $(+ $crate::const_concat!(@arg_capacity $arg $(=> $fmt)?))*
    };
    (@arg_capacity $arg:expr) => {
        $crate::ArgumentWrapper($arg).into_argument().formatted_len()
    };
    (@arg_capacity $arg:expr => $fmt:expr) => {
        $crate::Fmt::formatted_len(&$fmt)
    };
}

#[macro_export]
macro_rules! const_assert {
    ($check:expr, $($arg:tt)+) => {{
        if !$check {
            ::core::panic!("{}", $crate::const_concat!($($arg)+).as_str());
        }
    }};
}

#[derive(Debug)]
pub struct Formatter<const CAP: usize> {
    buffer: [u8; CAP],
    len: usize,
}

impl<const CAP: usize> fmt::Display for Formatter<CAP> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl<const CAP: usize> Formatter<CAP> {
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

    const fn format_arg(self, arg: Argument) -> Self {
        match arg.inner {
            ArgumentInner::Str(s) => self.write_str(s),
            ArgumentInner::Int(value) => self.write_i128(value),
            ArgumentInner::UnsignedInt(value) => self.write_u128(value),
        }
    }

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

    pub const fn as_str(&self) -> &str {
        // SAFETY: Safe by construction
        unsafe {
            let written_slice = slice::from_raw_parts(self.buffer.as_ptr(), self.len);
            str::from_utf8_unchecked(written_slice)
        }
    }
}

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

#[derive(Debug, Clone, Copy)]
pub struct Argument {
    inner: ArgumentInner,
    fmt: Option<Fmt>,
}

impl Argument {
    /// Returns the formatted length of the argument in bytes.
    #[doc(hidden)]
    pub const fn formatted_len(&self) -> usize {
        if let Some(fmt) = &self.fmt {
            fmt.formatted_len()
        } else {
            self.inner.formatted_len()
        }
    }

    const fn assert_width(&self, arg_index: usize) {
        if let Some(fmt) = &self.fmt {
            let fmt_len = fmt.formatted_len();
            let inherent_len = self.inner.formatted_len();
            const_assert!(
                fmt_len >= inherent_len,
                "Argument #", arg_index => Fmt::of::<usize>(),
                " has insufficient byte width (", fmt_len => Fmt::of::<usize>(),
                "); required at least ", inherent_len => Fmt::of::<usize>()
            );
        }
    }

    #[must_use]
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

#[derive(Debug)]
pub struct ArgumentWrapper<T>(pub T);

impl ArgumentWrapper<&'static str> {
    pub const fn into_argument(self) -> Argument {
        Argument {
            inner: ArgumentInner::Str(self.0),
            fmt: None,
        }
    }
}

impl ArgumentWrapper<i128> {
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

#[derive(Debug, Clone, Copy)]
pub struct Fmt {
    width: usize,
}

impl Fmt {
    pub const fn width(width: usize) -> Self {
        Self { width }
    }

    pub const fn of<T: MaxWidth>() -> Self {
        Self {
            width: T::MAX_WIDTH,
        }
    }

    /// Returns the formatted length of the argument in bytes.
    #[doc(hidden)]
    pub const fn formatted_len(&self) -> usize {
        self.width
    }
}

pub trait MaxWidth {
    const MAX_WIDTH: usize;
}

macro_rules! impl_max_width_for_uint {
    ($($uint:ty),+) => {
        $(
        impl MaxWidth for $uint {
            const MAX_WIDTH: usize =
                ArgumentWrapper(Self::MAX).into_argument().formatted_len();
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
                ArgumentWrapper(Self::MIN).into_argument().formatted_len();
        }
        )+
    };
}

impl_max_width_for_int!(i8, i16, i32, i64, i128, isize);

#[cfg(test)]
mod tests {
    use rand::{rngs::StdRng, Rng, SeedableRng};

    use super::*;

    const THRESHOLD: usize = 32;

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
        const SAMPLE_COUNT: usize = 1_000_000;

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

    #[test]
    fn basics() {
        const TEST: Formatter<32> =
            const_concat!("expected ", 1_usize, " to be greater than ", THRESHOLD);
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
        const_concat!("expected ", 111111_usize => Fmt::width(4), " to be greater than ", THRESHOLD);
    }
}
