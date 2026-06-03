extern crate alloc;
extern crate core;

mod tuple_variants;
use tuple_variants::*;

struct Context;
impl tuple_variants::Context for Context {}

fn main() {
    assert!(matches!(
        constructor_lower(&mut Context, &D::Da(A(1, 2))),
        B::Ba { x: 1 }
    ));
    assert!(matches!(
        constructor_lower(&mut Context, &D::Db(B::Ba { x: 6 })),
        B::Bb(6, 6, 6)
    ));
    assert!(matches!(
        constructor_lower(&mut Context, &D::Db(B::Bb(1, 2, 3))),
        B::Ba { x: 3 }
    ));
    assert!(matches!(
        constructor_lower(&mut Context, &D::Dc),
        B::Ba { x: 42 }
    ));

    let _ = UnitStruct;
    assert_eq!(size_of::<UnitStruct>(), 0);
    assert_eq!(size_of::<UninhabitedEnum>(), 0);
}
