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
