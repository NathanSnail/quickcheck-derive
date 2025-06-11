# QuickCheck Arbitrary Derive

[![crates.io](https://img.shields.io/crates/v/quickcheck-arbitrary-derive.svg)](https://crates.io/crates/quickcheck-arbitrary-derive)
[![MIT OR UNLICENSE](https://img.shields.io/badge/license-MIT%20OR%20UNLICENSE-blue.svg)](UNLICENSE.md)
[![docs.rs](https://img.shields.io/docsrs/quickcheck-arbitrary-derive?logo=rust&color=blue)](https://docs.rs/quickcheck-arbitrary-derive)

A `#[derive(QuickCheck)]` macro to automatically implement [QuickCheck](https://github.com/BurntSushi/quickcheck)’s `Arbitrary` (with `arbitrary` + `shrink`) for your types.

Dual-licensed under MIT or [UNLICENSE](https://unlicense.org/)

---

## Installation

Add to your `Cargo.toml`:

```toml
quickcheck = "1.0.3"                  # quickcheck runtime
quickcheck-derive-macros = "0.2.5"    # this derive macro
```

## Example usage

```rs
#[derive(Clone, QuickCheck, Debug)]
struct Pair<T> {
    first: T,
    second: T,
}

#[cfg(test)]
mod test {
    use crate::Pair;

    #[quickcheck_macros::quickcheck]
    fn fails(pair: Pair<isize>) {
        assert_eq!(pair.first, pair.second);
    }
}
```

You should see that the test fails, and that the minimal example produced is just (1, 0). The derive macro automatically implements both `arbitrary` and `shrink` so that the sample failing test cases are as simple as possible.

## Features

 * Implements both `arbitrary` and `shrink`
 * Works on:
   * Named Structs
   * Unnamed Structs
   * Enums
 * Supports generic type paramaters
 * Supports recursive types via `#[quickcheck(recursive = Exponential)]` enum variant attribute

## Limitations

 * Does *not* support union types
 * Does *not* support lifetime parameters
