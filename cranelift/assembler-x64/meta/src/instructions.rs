//! Defines x64 instructions using the DSL.

mod add;
mod and;
mod bitmanip;
mod cvt;
mod mul;
mod neg;
mod or;
mod shift;
mod sqrt;
mod sub;
mod xor;

use crate::dsl::Inst;

#[must_use]
pub fn list() -> Vec<Inst> {
    let mut all = vec![];
    all.extend(add::list());
    all.extend(and::list());
    all.extend(bitmanip::list());
    all.extend(cvt::list());
    all.extend(mul::list());
    all.extend(neg::list());
    all.extend(or::list());
    all.extend(shift::list());
    all.extend(sqrt::list());
    all.extend(sub::list());
    all.extend(xor::list());
    all
}
