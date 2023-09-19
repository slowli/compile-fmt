//! `Fmt` and related types.

#[derive(Debug, Clone, Copy)]
pub struct Fmt {
    width: usize,
}

impl Fmt {
    /// Creates a format with the specified max byte width.
    pub const fn width(width: usize) -> Self {
        Self { width }
    }

    /// Creates a format for the specified type.
    pub const fn of<T: MaxWidth>() -> Self {
        Self {
            width: T::MAX_WIDTH,
        }
    }

    /// Returns the formatted length of the argument in bytes.
    #[doc(hidden)] // only used by macros
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
                crate::ArgumentWrapper(Self::MAX).into_argument().formatted_len();
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
                crate::ArgumentWrapper(Self::MIN).into_argument().formatted_len();
        }
        )+
    };
}

impl_max_width_for_int!(i8, i16, i32, i64, i128, isize);

#[cfg(test)]
mod tests {
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
