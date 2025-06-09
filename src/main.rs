use std::clone;

use quickcheck::{Arbitrary, Gen};
use quickcheck_derive_macros::QuickCheck;

fn shrink(v3: Vector3) -> Box<dyn Iterator<Item = Vector3>> {
    let v3_x = v3.clone();
    let v3_y = v3.clone();
    let v3_z = v3.clone();
    Box::new(
        v3.x.shrink()
            .map(move |e| Vector3 {
                x: e,
                y: v3_x.y.clone(),
                z: v3_x.z.clone(),
            })
            .chain(
                v3.y.shrink()
                    .map(move |e| Vector3 {
                        x: v3_y.x.clone(),
                        y: e,
                        z: v3_y.z.clone(),
                    })
                    .chain(v3.z.shrink().map(move |e| Vector3 {
                        x: v3_z.x.clone(),
                        y: v3_z.y.clone(),
                        z: e,
                    })),
            ),
    )
}

#[derive(Clone, QuickCheck)]
struct PositiveF32(f32);

#[derive(Clone, QuickCheck)]
struct Pair(f32, f32);

#[derive(Clone, QuickCheck)]
struct Unit;

#[derive(Clone, QuickCheck)]
enum Something {
    Int(i32),
    Float(f32),
    Pair(f32, f32),
}

fn main() {
    println!("Hello, world!");
}

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
