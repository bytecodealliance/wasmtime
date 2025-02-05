use crate::dsl::{fmt, inst, r, rex, rw, sxl, sxq};
use crate::dsl::{Feature::*, Inst, LegacyPrefix::*, Location::*};

pub fn list() -> Vec<Inst> {
    vec![
        inst("subb", fmt("I", [rw(al), r(imm8)]), rex(0x2C).ib(), _64b | compat),
        inst("subw", fmt("I", [rw(ax), r(imm16)]), rex(0x2D).prefix(_66).iw(), _64b | compat),
        inst("subl", fmt("I", [rw(eax), r(imm32)]), rex(0x2D).id(), _64b | compat),
        inst("subq", fmt("I_SXL", [rw(rax), sxq(imm32)]), rex(0x2D).w().id(), _64b),
        inst("subb", fmt("MI", [rw(rm8), r(imm8)]), rex(0x80).digit(5).ib(), _64b | compat),
        inst("subw", fmt("MI", [rw(rm16), r(imm16)]), rex(0x81).prefix(_66).digit(5).iw(), _64b | compat),
        inst("subl", fmt("MI", [rw(rm32), r(imm32)]), rex(0x81).digit(5).id(), _64b | compat),
        inst("subq", fmt("MI_SXL", [rw(rm64), sxq(imm32)]), rex(0x81).w().digit(5).id(), _64b),
        inst("subl", fmt("MI_SXB", [rw(rm32), sxl(imm8)]), rex(0x83).digit(5).ib(), _64b | compat),
        inst("subq", fmt("MI_SXB", [rw(rm64), sxq(imm8)]), rex(0x83).w().digit(5).ib(), _64b),
        inst("subb", fmt("MR", [rw(rm8), r(r8)]), rex(0x28).r(), _64b | compat),
        inst("subw", fmt("MR", [rw(rm16), r(r16)]), rex(0x29).prefix(_66).r(), _64b | compat),
        inst("subl", fmt("MR", [rw(rm32), r(r32)]), rex(0x29).r(), _64b | compat),
        inst("subq", fmt("MR", [rw(rm64), r(r64)]), rex(0x29).w().r(), _64b),
        inst("subb", fmt("RM", [rw(r8), r(rm8)]), rex(0x2A).r(), _64b | compat),
        inst("subw", fmt("RM", [rw(r16), r(rm16)]), rex(0x2B).prefix(_66).r(), _64b | compat),
        inst("subl", fmt("RM", [rw(r32), r(rm32)]), rex(0x2B).r(), _64b | compat),
        inst("subq", fmt("RM", [rw(r64), r(rm64)]), rex(0x2B).w().r(), _64b),
        // Subtract with borrow.
        inst("sbbb", fmt("I", [rw(al), r(imm8)]), rex(0x1C).ib(), _64b | compat),
        inst("sbbw", fmt("I", [rw(ax), r(imm16)]), rex(0x1D).prefix(_66).iw(), _64b | compat),
        inst("sbbl", fmt("I", [rw(eax), r(imm32)]), rex(0x1D).id(), _64b | compat),
        inst("sbbq", fmt("I_SXL", [rw(rax), sxq(imm32)]), rex(0x1D).w().id(), _64b),
        inst("sbbb", fmt("MI", [rw(rm8), r(imm8)]), rex(0x80).digit(3).ib(), _64b | compat),
        inst("sbbw", fmt("MI", [rw(rm16), r(imm16)]), rex(0x81).prefix(_66).digit(3).iw(), _64b | compat),
        inst("sbbl", fmt("MI", [rw(rm32), r(imm32)]), rex(0x81).digit(3).id(), _64b | compat),
        inst("sbbq", fmt("MI_SXL", [rw(rm64), sxq(imm32)]), rex(0x81).w().digit(3).id(), _64b),
        inst("sbbl", fmt("MI_SXB", [rw(rm32), sxl(imm8)]), rex(0x83).digit(3).ib(), _64b | compat),
        inst("sbbq", fmt("MI_SXB", [rw(rm64), sxq(imm8)]), rex(0x83).w().digit(3).ib(), _64b),
        inst("sbbb", fmt("MR", [rw(rm8), r(r8)]), rex(0x18).r(), _64b | compat),
        inst("sbbw", fmt("MR", [rw(rm16), r(r16)]), rex(0x19).prefix(_66).r(), _64b | compat),
        inst("sbbl", fmt("MR", [rw(rm32), r(r32)]), rex(0x19).r(), _64b | compat),
        inst("sbbq", fmt("MR", [rw(rm64), r(r64)]), rex(0x19).w().r(), _64b),
        inst("sbbb", fmt("RM", [rw(r8), r(rm8)]), rex(0x1A).r(), _64b | compat),
        inst("sbbw", fmt("RM", [rw(r16), r(rm16)]), rex(0x1B).prefix(_66).r(), _64b | compat),
        inst("sbbl", fmt("RM", [rw(r32), r(rm32)]), rex(0x1B).r(), _64b | compat),
        inst("sbbq", fmt("RM", [rw(r64), r(rm64)]), rex(0x1B).w().r(), _64b),
    ]
}
