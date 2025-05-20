use crate::dsl::{Feature::*, Inst, Location::*};
use crate::dsl::{fmt, implicit, inst, r, rex, rw, trap};

#[rustfmt::skip] // Keeps instructions on a single line.
pub fn list() -> Vec<Inst> {
    vec![
        inst("divb", fmt("M", [rw(implicit(ax)), r(rm8), trap()]), rex([0xF6]).digit(6), _64b | compat),
        inst("divw", fmt("M", [rw(implicit(ax)), rw(implicit(dx)), r(rm16), trap()]), rex([0x66, 0xF7]).digit(6), _64b | compat),
        inst("divl", fmt("M", [rw(implicit(eax)), rw(implicit(edx)), r(rm32), trap()]), rex([0xF7]).digit(6), _64b | compat),
        inst("divq", fmt("M", [rw(implicit(rax)), rw(implicit(rdx)), r(rm64), trap()]), rex([0xF7]).digit(6).w(), _64b),
        inst("idivb", fmt("M", [rw(implicit(ax)), r(rm8), trap()]), rex([0xF6]).digit(7), _64b | compat),
        inst("idivw", fmt("M", [rw(implicit(ax)), rw(implicit(dx)), r(rm16), trap()]), rex([0x66, 0xF7]).digit(7), _64b | compat),
        inst("idivl", fmt("M", [rw(implicit(eax)), rw(implicit(edx)), r(rm32), trap()]), rex([0xF7]).digit(7), _64b | compat),
        inst("idivq", fmt("M", [rw(implicit(rax)), rw(implicit(rdx)), r(rm64), trap()]), rex([0xF7]).digit(7).w(), _64b),
    ]
}
