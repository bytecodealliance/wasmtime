use crate::dsl::{fmt, inst, r, rex, rw, sxl, sxq};
use crate::dsl::{Feature::*, Inst, Location::*};

pub fn list() -> Vec<Inst> {
    vec![
        inst("addb", fmt("I", [rw(al), r(imm8)]), rex(0x4).ib(), _64b | compat),
        inst("addw", fmt("I", [rw(ax), r(imm16)]), rex([0x66, 0x5]).iw(), _64b | compat),
        inst("addl", fmt("I", [rw(eax), r(imm32)]), rex(0x5).id(), _64b | compat),
        inst("addq", fmt("I_SXL", [rw(rax), sxq(imm32)]), rex(0x5).w().id(), _64b),
        inst("addb", fmt("MI", [rw(rm8), r(imm8)]), rex(0x80).digit(0).ib(), _64b | compat),
        inst("addw", fmt("MI", [rw(rm16), r(imm16)]), rex([0x66, 0x81]).digit(0).iw(), _64b | compat),
        inst("addl", fmt("MI", [rw(rm32), r(imm32)]), rex(0x81).digit(0).id(), _64b | compat),
        inst("addq", fmt("MI_SXL", [rw(rm64), sxq(imm32)]), rex(0x81).w().digit(0).id(), _64b),
        inst("addl", fmt("MI_SXB", [rw(rm32), sxl(imm8)]), rex(0x83).digit(0).ib(), _64b | compat),
        inst("addq", fmt("MI_SXB", [rw(rm64), sxq(imm8)]), rex(0x83).w().digit(0).ib(), _64b),
        inst("addb", fmt("MR", [rw(rm8), r(r8)]), rex(0x0).r(), _64b | compat),
        inst("addw", fmt("MR", [rw(rm16), r(r16)]), rex([0x66, 0x1]).r(), _64b | compat),
        inst("addl", fmt("MR", [rw(rm32), r(r32)]), rex(0x1).r(), _64b | compat),
        inst("addq", fmt("MR", [rw(rm64), r(r64)]), rex(0x1).w().r(), _64b),
        inst("addb", fmt("RM", [rw(r8), r(rm8)]), rex(0x2).r(), _64b | compat),
        inst("addw", fmt("RM", [rw(r16), r(rm16)]), rex([0x66, 0x3]).r(), _64b | compat),
        inst("addl", fmt("RM", [rw(r32), r(rm32)]), rex(0x3).r(), _64b | compat),
        inst("addq", fmt("RM", [rw(r64), r(rm64)]), rex(0x3).w().r(), _64b),
        // Add with carry.
        inst("adcb", fmt("I", [rw(al), r(imm8)]), rex(0x14).ib(), _64b | compat),
        inst("adcw", fmt("I", [rw(ax), r(imm16)]), rex([0x66, 0x15]).iw(), _64b | compat),
        inst("adcl", fmt("I", [rw(eax), r(imm32)]), rex(0x15).id(), _64b | compat),
        inst("adcq", fmt("I_SXL", [rw(rax), sxq(imm32)]), rex(0x15).w().id(), _64b),
        inst("adcb", fmt("MI", [rw(rm8), r(imm8)]), rex(0x80).digit(2).ib(), _64b | compat),
        inst("adcw", fmt("MI", [rw(rm16), r(imm16)]), rex([0x66, 0x81]).digit(2).iw(), _64b | compat),
        inst("adcl", fmt("MI", [rw(rm32), r(imm32)]), rex(0x81).digit(2).id(), _64b | compat),
        inst("adcq", fmt("MI_SXL", [rw(rm64), sxq(imm32)]), rex(0x81).w().digit(2).id(), _64b),
        inst("adcl", fmt("MI_SXB", [rw(rm32), sxl(imm8)]), rex(0x83).digit(2).ib(), _64b | compat),
        inst("adcq", fmt("MI_SXB", [rw(rm64), sxq(imm8)]), rex(0x83).w().digit(2).ib(), _64b),
        inst("adcb", fmt("MR", [rw(rm8), r(r8)]), rex(0x10).r(), _64b | compat),
        inst("adcw", fmt("MR", [rw(rm16), r(r16)]), rex([0x66, 0x11]).r(), _64b | compat),
        inst("adcl", fmt("MR", [rw(rm32), r(r32)]), rex(0x11).r(), _64b | compat),
        inst("adcq", fmt("MR", [rw(rm64), r(r64)]), rex(0x11).w().r(), _64b),
        inst("adcb", fmt("RM", [rw(r8), r(rm8)]), rex(0x12).r(), _64b | compat),
        inst("adcw", fmt("RM", [rw(r16), r(rm16)]), rex([0x66, 0x13]).r(), _64b | compat),
        inst("adcl", fmt("RM", [rw(r32), r(rm32)]), rex(0x13).r(), _64b | compat),
        inst("adcq", fmt("RM", [rw(r64), r(rm64)]), rex(0x13).w().r(), _64b),
    ]
}
