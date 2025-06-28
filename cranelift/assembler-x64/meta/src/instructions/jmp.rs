use crate::dsl::{Customization::*, Feature::*, Inst, Location::*};
use crate::dsl::{fmt, inst, r, rex};

#[rustfmt::skip] // Keeps instructions on a single line.
pub fn list() -> Vec<Inst> {
    vec![
        inst("jmpq", fmt("M", [r(rm64)]), rex([0xFF]).digit(4), _64b).custom(Display),
    ]
}
