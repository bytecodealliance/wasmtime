extern crate alloc;
extern crate core;

mod struct_test;
use struct_test::{A, C, UnitStruct};

#[derive(Clone, Debug)]
pub struct B {
    pub x: u32,
}

#[derive(Clone, Debug)]
pub struct D {
    pub x: C,
}

struct Context();
impl struct_test::Context for Context {}

fn main() {
    let a = D {
        x: C::Ca {
            x: A { x: 42, y: 123 },
        },
    };
    let b = D {
        x: C::Cb { x: B { x: 42 } },
    };
    for d in [a, b] {
        match d {
            D {
                x: C::Ca { x: A { x, .. } } | C::Cb { x: B { x } },
            } => {
                assert_eq!(x, 42);
            }
        }
    }

    let unit = UnitStruct;
}
