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
/// - Strings (`&str`)
/// - [`Ascii`](crate::Ascii) strings
/// - Chars (`char`)
/// - References to [`CompileArgs`](crate::CompileArgs).
///
/// Due to how Rust type inference works, you might need to specify the type suffix for integer
/// literals (e.g., `42_usize` instead of `42`).
///
/// Optionally, an argument may specify its [format](crate::Fmt) as `$arg => $fmt`.
/// A format is mandatory if the argument is not a constant; e.g. if it is an argument or a local variable
/// in a `const fn`.
///
/// The value output by the macro is [`CompileArgs`](crate::CompileArgs).
///
/// # Specifying capacity
///
/// You can specify capacity of the returned `CompileArgs` by prefacing arguments with `capacity: $cap,`.
/// Here, `$cap` is a constant expression of type `usize`. The specified capacity must be greater or equal
/// that the capacity inferred from the arguments; the macro will fail with a compilation error otherwise.
///
/// # See also
///
/// - [`compile_panic!`](crate::compile_panic) provides a version of the `panic!` macro with support
///   of dynamic arguments.
/// - [`compile_assert!`](crate::compile_assert) provides a version of the `assert!` macro with support
///   of dynamic arguments.
///
/// # Examples
///
/// ## Basic usage
///
/// ```
/// # use compile_fmt::{compile_args, CompileArgs};
/// const ARGS: CompileArgs<9> = compile_args!(2_u32, " + ", 2_u32, " = ", 2_u32 + 2);
/// assert_eq!(ARGS.as_str(), "2 + 2 = 4");
/// ```
///
/// ## Usage in `const fn` with dynamic args
///
/// ```
/// use compile_fmt::{compile_args, fmt};
/// use std::fmt;
///
/// const fn create_args(x: usize) -> impl fmt::Display {
///     let args = compile_args!(
///         "2 * x + 3 = ", 2 * x + 3 => fmt::<usize>()
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
///
/// ## Usage with explicit capacity
///
/// ```
/// # use compile_fmt::compile_args;
/// let args = compile_args!(capacity: 16, "Value: ", 42_i32);
/// assert_eq!(args.as_str(), "Value: 42");
/// ```
///
/// Insufficient specified capacity will lead to a compilation error:
///
/// ```compile_fail
/// # use compile_fmt::compile_args;
/// let args = compile_args!(capacity: 4, "Value: ", 42_i32);
/// ```
///
/// The error message will include details about the necessary capacity:
///
/// ```text
/// error[E0080]: evaluation of constant value failed
///    -->
///     |
///     |     let args = compile_args!(capacity: 4, "Value: ", 42_i32);
///     |                ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
/// the evaluated program panicked at 'Insufficient capacity (4 bytes)
/// provided for `compile_args` macro; it requires at least 9 bytes'
/// ```
#[macro_export]
macro_rules! compile_args {
    (capacity: $cap:expr, $($arg:expr $(=> $fmt:expr)?),+) => {{
        const __CAPACITY: usize = $cap;
        const _: () = {
            let required_capacity = $crate::__compile_args_impl!(@total_capacity $($arg $(=> $fmt)?,)+);
            $crate::CompileArgs::<__CAPACITY>::assert_capacity(required_capacity);
        };
        $crate::CompileArgs::<__CAPACITY>::format(&[
            $($crate::ArgumentWrapper::new($arg)$(.with_fmt($fmt))?.into_argument(),)+
        ]) as $crate::CompileArgs<__CAPACITY>
        // ^ The type hint sometimes helps in const contexts
    }};
    ($($arg:expr $(=> $fmt:expr)?),+) => {{
        const __CAPACITY: usize = $crate::__compile_args_impl!(@total_capacity $($arg $(=> $fmt)?,)+);
        $crate::CompileArgs::<__CAPACITY>::format(&[
            $($crate::ArgumentWrapper::new($arg)$(.with_fmt($fmt))?.into_argument(),)+
        ]) as $crate::CompileArgs<__CAPACITY>
        // ^ The type hint sometimes helps in const contexts
    }};
}

#[doc(hidden)] // implementation detail of `compile_args`
#[macro_export]
macro_rules! __compile_args_impl {
    (@total_capacity $first_arg:expr $(=> $first_fmt:expr)?, $($arg:expr $(=> $fmt:expr)?,)*) => {
        $crate::__compile_args_impl!(@arg_capacity $first_arg $(=> $first_fmt)?)
            $(+ $crate::__compile_args_impl!(@arg_capacity $arg $(=> $fmt)?))*
    };
    (@arg_capacity $arg:expr) => {
        $crate::ArgumentWrapper::new($arg).into_argument().formatted_len()
    };
    (@arg_capacity $arg:expr => $fmt:expr) => {
        $crate::Fmt::capacity(&$fmt)
    };
}

/// Version of the [`panic!`] macro with the ability to format args in compile time.
///
/// Arguments have the same syntax as in the [`compile_args!`] macro.
///
/// # Examples
///
/// ```
/// use compile_fmt::{compile_panic, clip};
///
/// const fn unwrap_result(res: Result<(), &str>) {
///     if let Err(err) = res {
///         compile_panic!("Encountered an error: ", err => clip(64, "…"));
///     }
/// }
/// ```
#[macro_export]
macro_rules! compile_panic {
    ($($arg:tt)+) => {
        ::core::panic!("{}", $crate::compile_args!($($arg)+).as_str());
    };
}

/// Version of the [`assert!`] macro with the ability to format args in compile time.
///
/// The first argument of the macro must be a boolean value. The remaining arguments have the same syntax
/// as in the [`compile_args!`] macro.
///
/// # Examples
///
/// ```
/// use compile_fmt::{compile_assert, fmt};
///
/// const fn check_args(x: usize, s: &str) {
///     const MAX_STR_LEN: usize = 10;
///
///     compile_assert!(
///         x < 1_000,
///         "`x` should be less than 1000 (got: ",
///         x => fmt::<usize>(), ")"
///     );
///     compile_assert!(
///         s.len() <= MAX_STR_LEN,
///         "String is too long (expected at most ", MAX_STR_LEN,
///         " bytes; got ", s.len() => fmt::<usize>(), " bytes)"
///     );
///     // main logic...
/// }
/// ```
#[macro_export]
macro_rules! compile_assert {
    ($check:expr, $($arg:tt)+) => {{
        if !$check {
            ::core::panic!("{}", $crate::compile_args!($($arg)+).as_str());
        }
    }};
}
