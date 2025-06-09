use quickcheck::Gen;
use quickcheck_derive_macros::QuickCheck;

#[derive(Clone, QuickCheck)]
struct Thing {
    x: i32,
}

#[derive(Clone, QuickCheck)]
struct PositiveF32(f32);

#[derive(Clone, QuickCheck)]
struct Unit;

#[derive(Clone, QuickCheck)]
enum Something {
    Int(i32),
    Float(f32),
}

fn main() {
    let x: ::core::primitive::u64 = 3;

    let mut g2 = Gen::new(10);
    let g = &mut g2;
    println!("Hello, world!");
}
