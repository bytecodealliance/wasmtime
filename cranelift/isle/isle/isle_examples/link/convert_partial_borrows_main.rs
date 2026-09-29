extern crate alloc;
extern crate core;

mod convert_partial_borrows;
use convert_partial_borrows::{A, W};

struct Context;
impl convert_partial_borrows::Context for Context {
    fn a_to_w(&mut self, arg0: &A) -> Option<W> {
        match arg0 {
            A::B => Some(W::X),
            A::C => Some(W::Y),
        }
    }

    fn build(&mut self, arg0: &W) -> u32 {
        match arg0 {
            W::X => 0,
            W::Y => 1,
        }
    }
}

fn main() {
    convert_partial_borrows::constructor_entry(&mut Context, &A::B);
}
