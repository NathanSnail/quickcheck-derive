use quickcheck_derive_macros::QuickCheck;

#[derive(Clone, QuickCheck)]
struct PositiveF32(f32);

#[derive(Clone, QuickCheck)]
struct Pair(f32, f32);

#[derive(Clone, QuickCheck)]
struct Unit;

fn main() {}

#[derive(Clone, QuickCheck)]
struct Vector3 {
    x: String,
    y: String,
    z: String,
}

#[derive(Clone, QuickCheck)]
struct Thing {
    x: i32,
}

#[derive(Clone, QuickCheck)]
enum Something {
    Int(i32),
    Float(f32),
    Pair(f32, f32),
    Complex(String, f64, f32, i32, String),
}
