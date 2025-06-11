// This is all just testing if the macro works, the actual code is inside quickcheck-derive-macros

use quickcheck::{Arbitrary, Gen};
use quickcheck_arbitrary_derive::QuickCheck;
use quickcheck_macros::quickcheck;

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

#[derive(Clone, QuickCheck, Debug, Eq, PartialEq)]
enum Tree<T> {
    #[quickcheck(recursive = Exponential)]
    Branch(Vec<Tree<T>>),
    Leaf(T),
}

#[derive(Clone, QuickCheck, Debug, Eq, PartialEq)]
enum List<T> {
    Node(T, Box<List<T>>),
    Tail,
}

impl<T: Clone> Tree<T> {
    fn flip(&self) -> Self {
        match self {
            Tree::Branch(branch) => {
                Tree::Branch(branch.iter().map(|child| child.flip()).rev().collect())
            }
            Tree::Leaf(v) => Tree::Leaf(v.clone()),
        }
    }

    fn size(&self) -> usize {
        match self {
            Tree::Branch(vec) => vec.iter().map(|child| child.size()).sum(),
            Tree::Leaf(_) => 1,
        }
    }
}

#[cfg(test)]
mod test {
    use quickcheck_macros::quickcheck;

    use crate::{Pair, Tree};

    #[quickcheck]
    fn recursive(tree: Tree<isize>) {
        dbg!(tree.size());
        assert_eq!(tree, tree.flip().flip());
    }

    #[quickcheck]
    fn fails(pair: Pair<isize>) {
        assert_eq!(pair.first, pair.second);
    }
}

fn main() {}
