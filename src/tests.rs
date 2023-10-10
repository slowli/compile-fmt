//! General-purpose tests.

use std::{
    panic,
    string::{String, ToString},
};

use super::*;

const THRESHOLD: usize = 32;

#[test]
fn basics() {
    const TEST: CompileArgs<32> =
        compile_args!("expected ", 1_usize, " to be greater than ", THRESHOLD);
    assert_eq!(TEST.to_string(), "expected 1 to be greater than 32");
}

#[test]
fn using_chars() {
    const CHARS: CompileArgs<11> = compile_args!('H', 'i', 'ß', 'ℝ', '💣');
    assert_eq!(CHARS.to_string(), "Hißℝ💣");
}

#[test]
fn using_dynamic_chars() {
    for char in ['i', 'ß', 'ℝ', '💣'] {
        let s = compile_args!("char: ", char => fmt::<char>(), "!");
        assert_eq!(s.as_str(), std::format!("char: {char}!"));
    }
}

#[test]
fn clipping_strings() {
    let arg = "dynamic";
    let s = compile_args!("string: '", arg => clip(3, ""), '\'');
    assert_eq!(s.as_str(), "string: 'dyn'");

    let arg = "Tℝ💣eßt";
    let s = compile_args!("string: '", arg => clip(2, ""), '\'');
    assert_eq!(s.as_str(), "string: 'Tℝ'");
    let s = compile_args!("string: '", arg => clip(3, ""), '\'');
    assert_eq!(s.as_str(), "string: 'Tℝ💣'");
    let s = compile_args!("string: '", arg => clip(4, ""), '\'');
    assert_eq!(s.as_str(), "string: 'Tℝ💣e'");
    let s = compile_args!("string: '", arg => clip(5, ""), '\'');
    assert_eq!(s.as_str(), "string: 'Tℝ💣eß'");
}

#[test]
fn clipping_strings_with_clip_chars() {
    let arg = "dynamic";
    let s = compile_args!("string: '", arg => clip(3, "-"), '\'');
    assert_eq!(s.as_str(), "string: 'dyn-'");
    let s = compile_args!("string: '", arg => clip(3, "[..]"), '\'');
    assert_eq!(s.as_str(), "string: 'dyn[..]'");
    let s = compile_args!("string: '", arg => clip(3, "…"), '\'');
    assert_eq!(s.as_str(), "string: 'dyn…'");

    let s = compile_args!("string: '", arg => clip(10, "-"), '\'');
    assert_eq!(s.as_str(), "string: 'dynamic'");
}

#[test]
fn padding() {
    let num = 42_u64;
    let s = compile_args!(
        "number: [", num => fmt::<u64>().pad_left(4, ' '), "]"
    );
    assert_eq!(s.as_str(), "number: [42  ]");

    let s = compile_args!(
        "number: [", num => fmt::<u64>().pad_center(4, ' '), "]"
    );
    assert_eq!(s.as_str(), "number: [ 42 ]");

    let s = compile_args!(
        "number: [", num => fmt::<u64>().pad_right(4, '0'), "]"
    );
    assert_eq!(s.as_str(), "number: [0042]");

    let s = compile_args!(
        "number: [", num => fmt::<u64>().pad_right(4, 'ℝ'), "]"
    );
    assert_eq!(s.as_str(), "number: [ℝℝ42]");
    let s = compile_args!(
        "number: [", num => fmt::<u64>().pad_right(4, '💣'), "]"
    );
    assert_eq!(s.as_str(), "number: [💣💣42]");

    let s = compile_args!(
        "number: [", num * 10_000 => fmt::<u64>().pad_right(4, '0'), "]"
    );
    assert_eq!(s.as_str(), "number: [420000]");
}

#[test]
fn clipping_and_padding() {
    let arg = "test string";
    let s = compile_args!(
        "string: [", arg => clip(4, "").pad_left(8, ' '), "]"
    );
    assert_eq!(s.as_str(), "string: [test    ]");

    let s = compile_args!(
        "string: [", arg => clip(4, "-").pad_right(8, ' '), "]"
    );
    assert_eq!(s.as_str(), "string: [   test-]");

    let s = compile_args!(
        "string: [", arg => clip(4, "…").pad_center(8, ' '), "]"
    );
    assert_eq!(s.as_str(), "string: [ test…  ]");

    let s = compile_args!(
        "string: [", arg => clip(4, "…").pad_left(8, '💣'), "]"
    );
    assert_eq!(s.as_str(), "string: [test…💣💣💣]");
    let s = compile_args!(
        "string: [", arg => clip(4, "…").pad_center(8, 'ß'), "]"
    );
    assert_eq!(s.as_str(), "string: [ßtest…ßß]");

    let s = compile_args!(
        "string: [", arg => clip(4, "…").pad_left(4, ' '), "]"
    );
    assert_eq!(s.as_str(), "string: [test…]");
}

#[test]
fn ascii_strings() {
    let s: CompileArgs<11> = compile_args!("ASCII: ", Ascii::new("test"));
    assert_eq!(s.as_str(), "ASCII: test");

    let s: CompileArgs<25> = compile_args!(
        "ASCII: ", Ascii::new("test") => clip_ascii(16, "..")
    );
    // ^ 25 = "ASCII: ".len() + 16 + "..".len()
    assert_eq!(s.as_str(), "ASCII: test");

    let s: CompileArgs<10> = compile_args!(
        "ASCII: ", Ascii::new("test") => clip_ascii(2, "~")
    );
    assert_eq!(s.as_str(), "ASCII: te~");
}

#[test]
#[should_panic(expected = "expected 1 to be greater than 32")]
fn assertion() {
    let value = 1;
    compile_assert!(
        value > THRESHOLD,
        "expected ", value => fmt::<usize>(), " to be greater than ", THRESHOLD
    );
}

#[cfg(panic = "unwind")]
#[test]
fn assertion_produces_exactly_expected_string() {
    let panic_result = panic::catch_unwind(|| {
        let value = 1;
        compile_assert!(
            value > THRESHOLD,
            "expected ", value => fmt::<usize>(), " to be greater than ", THRESHOLD
        );
    });
    let panic_message = panic_result.unwrap_err();
    let panic_message = panic_message.downcast_ref::<String>().unwrap();
    assert_eq!(panic_message, "expected 1 to be greater than 32");
    // ^ `const_panic` crate fails this test; it pads the panic message with '\0' chars
}

const fn unwrap_result(res: Result<(), &str>) {
    if let Err(err) = res {
        compile_panic!("Encountered an error: ", err => clip(64, "…"));
    }
}

#[test]
#[should_panic(expected = "Encountered an error: operation not supported")]
fn using_panic() {
    unwrap_result(Err("operation not supported"));
}

#[test]
fn formatting_enum() {
    #[derive(Debug)]
    enum Error {
        Number(u64),
        Tuple(usize, char),
    }

    type ErrorArgs = CompileArgs<54>;

    impl Error {
        const fn fmt(&self) -> ErrorArgs {
            match *self {
                Self::Number(number) => compile_args!(
                    capacity: 54,
                    "failed with number ", number => fmt::<u64>()
                ),
                Self::Tuple(pos, ch) => compile_args!(
                    capacity: 54,
                    "failed at position ", pos => fmt::<usize>(), " on char '", ch => fmt::<char>(), "'"
                ),
            }
        }
    }

    let err = Error::Number(123).fmt();
    let args = compile_args!("Runtime error: ", &err => fmt::<&ErrorArgs>());
    assert_eq!(args.as_str(), "Runtime error: failed with number 123");
    let err = Error::Tuple(78643, 'ß');
    let args = compile_args!("Runtime error: ", &err.fmt() => fmt::<&ErrorArgs>());
    assert_eq!(
        args.as_str(),
        "Runtime error: failed at position 78643 on char 'ß'"
    );
}
