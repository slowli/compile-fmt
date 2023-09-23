//! Formatting macros.

/// Concatenates arguments in compile time.
///
/// # Specifying arguments
///
/// Arguments to this macro must be comma-separated. The argument type must be supported; for now,
/// the supported types are:
///
/// - Signed and unsigned integers (`u8`, `i8`, `u16`, `i16`, `u32`, `i32`, `u64`, `i64`, `u128`,
///   `i128`, `usize`, `isize`)
/// - Strings (`&str`).
///
/// Due to how Rust type inference works, you might need to specify the type suffix for integer
/// literals (e.g., `42_usize` instead of `42`).
///
/// Optionally, an argument may specify its [format](crate::Fmt) as `$arg => $fmt`.
/// A format is mandatory if the argument is not a constant; e.g. if it is an argument or a local variable
/// in a `const fn`.
///
/// The value output by the macro is [`ConstArgs`](crate::ConstArgs).
///
/// # See also
///
/// - [`const_assert!`] provides a compile-time evaluated version of the `assert!` macro.
///
/// # Examples
///
/// ## Basic usage
///
/// ```
/// # use const_fmt::{const_args, ConstArgs};
/// const ARGS: ConstArgs<9> = const_args!(2_u32, " + ", 2_u32, " = ", 2_u32 + 2);
/// assert_eq!(ARGS.as_str(), "2 + 2 = 4");
/// ```
///
/// ## Usage in `const fn` with dynamic args
///
/// ```
/// use const_fmt::{const_args, Fmt};
/// use std::fmt;
///
/// const fn create_args(x: usize) -> impl fmt::Display {
///     let args = const_args!(
///         "2 * x + 3 = ", 2 * x + 3 => Fmt::of::<usize>()
///     );
///     // ^ `args` are evaluated in compile time, but are not a constant.
///     // They can still be useful e.g. for creating compile-time panic messages.
///     assert!(x < 1000, "{}", args.as_str());
///     args
/// }
///
/// let args = create_args(100);
/// assert_eq!(args.to_string(), "2 * x + 3 = 203");
/// ```
#[macro_export]
macro_rules! const_args {
    ($($arg:expr $(=> $fmt:expr)?),+) => {{
        const __CAPACITY: usize = $crate::__const_args_impl!(@total_capacity $($arg $(=> $fmt)?,)+);
        let __arguments: &[$crate::Argument] = &[
            $($crate::ArgumentWrapper($arg).into_argument()$(.with_fmt($fmt))?,)+
        ];
        $crate::ConstArgs::<__CAPACITY>::format(__arguments)
    }};
}

#[doc(hidden)] // implementation detail of `const_args`
#[macro_export]
macro_rules! __const_args_impl {
    (@total_capacity $first_arg:expr $(=> $first_fmt:expr)?, $($arg:expr $(=> $fmt:expr)?,)*) => {
        $crate::__const_args_impl!(@arg_capacity $first_arg $(=> $first_fmt)?)
            $(+ $crate::__const_args_impl!(@arg_capacity $arg $(=> $fmt)?))*
    };
    (@arg_capacity $arg:expr) => {
        $crate::ArgumentWrapper($arg).into_argument().formatted_len()
    };
    (@arg_capacity $arg:expr => $fmt:expr) => {
        $crate::Fmt::formatted_len(&$fmt)
    };
}

/// Compile-time version of the [`assert!`] macro.
///
/// The first argument of the macro must be a boolean value. The remaining arguments must be specified
/// as in the [`const_args!`] macro.
///
/// # Examples
///
/// ```
/// use const_fmt::{const_assert, Fmt};
///
/// const fn check_args(x: usize, s: &str) {
///     const MAX_STR_LEN: usize = 10;
///
///     const_assert!(
///         x < 1_000,
///         "`x` should be less than 1000 (got: ",
///         x => Fmt::of::<usize>(), ")"
///     );
///     const_assert!(
///         s.len() <= MAX_STR_LEN,
///         "String is too long (expected at most ", MAX_STR_LEN,
///         " bytes; got ", s.len() => Fmt::of::<usize>(), " bytes)"
///     );
///     // main logic...
/// }
/// ```
#[macro_export]
macro_rules! const_assert {
    ($check:expr, $($arg:tt)+) => {{
        if !$check {
            ::core::panic!("{}", $crate::const_args!($($arg)+).as_str());
        }
    }};
}
