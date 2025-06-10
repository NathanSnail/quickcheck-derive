// This is all just testing if the macro works, the actual code is inside quickcheck-derive-macros

use quickcheck_arbitrary_derive::QuickCheck;

#[derive(Clone, QuickCheck)]
struct Paired(f32, f32);

#[derive(Clone, QuickCheck)]
struct Unit;

#[derive(Clone, QuickCheck)]
struct Vector3 {
    x: f32,
    y: i32,
    z: u32,
}

#[derive(Clone, QuickCheck)]
struct Stringy {
    x: i32,
    y: String,
    z: String,
}

#[derive(Clone, QuickCheck)]
enum Basic {
    A,
    B,
}

#[derive(Clone, QuickCheck)]
enum Something {
    Int(i32),
    Float(f32),
    Pair(f32, f32),
    Complex(String, f64, f32, i32, String),
}

#[derive(Clone, QuickCheck)]
struct Generic<T: Clone> {
    a: T,
    b: T,
    // lifetimes have issues, as quickcheck requires `+ 'static`
    //thing: Useless<'a>,
}

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
fn main() {}
