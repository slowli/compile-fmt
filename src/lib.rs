use core::{fmt, slice, str};

#[macro_export]
macro_rules! const_concat {
    ($capacity:expr => $($arg:expr),+) => {{
        const __CAPACITY: usize = $capacity;
        let __arguments: &[$crate::Argument] = &[$($crate::ArgumentWrapper($arg).into_argument(),)+];
        $crate::Formatter::<__CAPACITY>::format(__arguments)
    }};
}

#[macro_export]
macro_rules! const_assert {
    ($capacity:expr => $check:expr, $($arg:tt)+) => {{
        if !$check {
            ::core::panic!("{}", $crate::const_concat!($capacity => $($arg)+).as_str());
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
        match arg {
            Argument::Str(s) => self.write_str(s),
            Argument::Int(value) => self.write_i128(value),
            Argument::UnsignedInt(value) => self.write_u128(value),
        }
    }

    pub const fn format(arguments: &[Argument]) -> Self {
        let necessary_capacity = Argument::total_len(arguments);
        const_assert!(128 =>
            CAP >= necessary_capacity,
            "Insufficient capacity (", CAP, ") provided for formatted string; expected at least ",
            necessary_capacity
        );

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
pub enum Argument {
    Str(&'static str),
    Int(i128),
    UnsignedInt(u128),
}

impl Argument {
    const fn formatted_len(&self) -> usize {
        match self {
            Self::Str(s) => s.len(),
            Self::Int(i) => (*i < 0) as usize + log_10_ceil(i.unsigned_abs()),
            Self::UnsignedInt(i) => log_10_ceil(*i),
        }
    }

    pub const fn total_len(args: &[Self]) -> usize {
        let mut total_len = 0;
        let mut arg_i = 0;
        while arg_i < args.len() {
            total_len += args[arg_i].formatted_len();
            arg_i += 1;
        }
        total_len
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
        Argument::Str(self.0)
    }
}

impl ArgumentWrapper<i128> {
    pub const fn into_argument(self) -> Argument {
        Argument::Int(self.0)
    }
}

macro_rules! impl_argument_wrapper_for_int {
    ($int:ty) => {
        impl ArgumentWrapper<$int> {
            pub const fn into_argument(self) -> Argument {
                Argument::Int(self.0 as i128)
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
        Argument::UnsignedInt(self.0)
    }
}

macro_rules! impl_argument_wrapper_for_uint {
    ($int:ty) => {
        impl ArgumentWrapper<$int> {
            pub const fn into_argument(self) -> Argument {
                Argument::UnsignedInt(self.0 as u128)
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

    use super::*;

    const THRESHOLD: usize = 32;

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
            const_concat!(32 => "expected ", 1_usize, " to be greater than ", THRESHOLD);
        assert_eq!(TEST.to_string(), "expected 1 to be greater than 32");
    }

    #[test]
    #[should_panic(expected = "expected 1 to be greater than 32")]
    fn assertion() {
        let value = 1;
        const_assert!(64 => value > THRESHOLD, "expected ", value, " to be greater than ", THRESHOLD);
    }

    #[test]
    #[should_panic(expected = "capacity (8) provided for formatted string; expected at least 32")]
    fn insufficient_capacity() {
        const_concat!(8 => "expected ", 1_usize, " to be greater than ", THRESHOLD);
    }
}
