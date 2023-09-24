# Compile-Time Formatting

This crate allows formatting values in compile time (e.g., in `const fn`s). The formatted values
are not required to be constants; e.g., arguments or local vars in `const fn` can be formatted.
The formatting logic is space-efficient; i.e., it allocates the least amount of bytes
that can provably to be sufficient for all possible provided inputs.

## Why?

A guiding use case for the crate is richer dynamic compile-time panic messages. It can be used
in other contexts as well (including in runtime).

## Usage

Add this to your `Crate.toml`:

```toml
[dependencies]
compile-fmt = "0.1.0"
```

### Basic usage

```rust
use compile_fmt::{compile_assert, clip, fmt};

const fn check_str(s: &str) {
    const MAX_LEN: usize = 16;
    compile_assert!(
        s.len() <= MAX_LEN,
        "String '", s => clip(MAX_LEN, "…"), "' is too long; \
         expected no more than ", MAX_LEN, " bytes, got ",
        s.len() => fmt::<usize>(), " bytes"
    );
    // ^ `clip` and `fmt` specify how dynamic (non-constant) args
    // should be formatted
  
    // main logic
}

let res = std::panic::catch_unwind(|| {
    check_str("very long string indeed");
});
let err = res.unwrap_err();
let panic_message = err.downcast_ref::<String>().unwrap();
assert_eq!(
    panic_message,
    "String 'very long string…' is too long; expected no more than \
     16 bytes, got 23 bytes"
);
```

See crate docs for more examples of usage.

## Limitations

- Only a few types from the standard library can be formatted: integers, `char`s and `str`ings.
- Formatting specifiers do not support padding / alignment, hex encoding etc.

## Alternatives and similar tools

- [`const_panic`] provides functionality covering the guiding use case (compile-time panics).
  It supports more types and formats at the cost of being more complex. It also uses a different
  approach to compute produced message sizes.
- [`const_format`] provides general-purpose formatting of constant values. It doesn't seem to support
  "dynamic" / non-constant args.

[`const_panic`]: https://crates.io/crates/const_panic
[`const_format`]: https://crates.io/crates/const_format
