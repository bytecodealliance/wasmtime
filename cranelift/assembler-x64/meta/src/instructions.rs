//! Defines x64 instructions using the DSL.

mod add;
mod and;
mod cvt;
mod neg;
mod or;
mod shift;
mod sub;
mod xor;

use crate::dsl::Inst;

#[must_use]
pub fn list() -> Vec<Inst> {
    let mut all = vec![];
    all.extend(add::list());
    all.extend(and::list());
    all.extend(cvt::list());
    all.extend(neg::list());
    all.extend(or::list());
    all.extend(shift::list());
    all.extend(sub::list());
    all.extend(xor::list());
    all
}
