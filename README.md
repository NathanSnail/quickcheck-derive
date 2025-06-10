# quickcheck derive

[![crates.io](https://img.shields.io/crates/v/quickcheck-derive-macros.svg)](https://crates.io/crates/quickcheck-derive-macros)
[![MIT OR UNLICENSE](https://img.shields.io/badge/license-MIT%20OR%20UNLICENSE-blue.svg)](UNLICENSE.md)

A `#[derive(QuickCheck)]` macro to automatically implement [QuickCheck](https://github.com/BurntSushi/quickcheck)’s `Arbitrary` (with `arbitrary` + `shrink`) for your types.

Dual-licensed under MIT or [UNLICENSE](https://unlicense.org/)

---

## Installation

Add to your `Cargo.toml`:

```toml
quickcheck = "1.0"                  # quickcheck runtime
quickcheck-derive-macros = "0.1"    # this derive macro
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

## Limitations

 * Does *not* support union types
 * Does *not* support lifetime parameters
