use crate::dsl::{fmt, inst, r, rex, rw};
use crate::dsl::{Feature::*, Inst, Location::*};

pub fn list() -> Vec<Inst> {
    vec![
        inst("shldw", fmt("MRI", [rw(rm16), r(r16), r(imm8)]), rex([0x66, 0x0F, 0xA4]).ib(), _64b | compat),
        inst("shldw", fmt("MRC", [rw(rm16), r(r16), r(cl)]), rex([0x66, 0x0F, 0xA5]).ib(), _64b | compat),
        inst("shldl", fmt("MRI", [rw(rm32), r(r32), r(imm8)]), rex([0x0F, 0xA4]).ib(), _64b | compat),
        inst("shldq", fmt("MRI", [rw(rm64), r(r64), r(imm8)]), rex([0x0F, 0xA4]).ib().w(), _64b),
        inst("shldl", fmt("MRC", [rw(rm32), r(r32), r(cl)]), rex([0x0F, 0xA5]).ib(), _64b | compat),
        inst("shldq", fmt("MRC", [rw(rm64), r(r64), r(cl)]), rex([0x0F, 0xA5]).ib().w(), _64b),
    ]
}
