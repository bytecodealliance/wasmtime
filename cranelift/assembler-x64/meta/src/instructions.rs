//! Defines x64 instructions using the DSL.

mod add;
mod and;
mod avg;
mod bitmanip;
mod cvt;
mod div;
mod lanes;
mod max;
mod min;
mod mov;
mod mul;
mod neg;
mod or;
mod round;
mod shift;
mod sqrt;
mod sub;
mod unpack;
mod xor;

use crate::dsl::Inst;

#[must_use]
pub fn list() -> Vec<Inst> {
    let mut all = vec![];
    all.extend(add::list());
    all.extend(and::list());
    all.extend(avg::list());
    all.extend(bitmanip::list());
    all.extend(cvt::list());
    all.extend(div::list());
    all.extend(lanes::list());
    all.extend(max::list());
    all.extend(min::list());
    all.extend(mov::list());
    all.extend(mul::list());
    all.extend(neg::list());
    all.extend(or::list());
    all.extend(round::list());
    all.extend(shift::list());
    all.extend(sqrt::list());
    all.extend(sub::list());
    all.extend(xor::list());
    all.extend(unpack::list());
    all
}
