use crate::dsl::{EflagsMutability::*, Feature::*, Inst, Location::*};
use crate::dsl::{fmt, inst, r, rex, rw, sxl, sxq, sxw, w};

#[rustfmt::skip] // Keeps instructions on a single line.
pub fn list() -> Vec<Inst> {
    vec![
        inst("cmpb", fmt("I", [r(al), r(imm8)]), rex(0x3C).ib(), _64b | compat),
        inst("cmpw", fmt("I", [r(ax), r(imm16)]), rex([0x66, 0x3D]).iw(), _64b | compat),
        inst("cmpl", fmt("I", [r(eax), r(imm32)]), rex(0x3D).id(), _64b | compat),
        inst("cmpq", fmt("I_SXL", [r(rax), sxq(imm32)]), rex(0x3D).w().id(), _64b),
        inst("cmpb", fmt("MI", [r(rm8), r(imm8)]), rex(0x80).digit(7).ib(), _64b | compat),
        inst("cmpw", fmt("MI", [r(rm16), r(imm16)]), rex([0x66, 0x81]).digit(7).iw(), _64b | compat),
        inst("cmpl", fmt("MI", [r(rm32), r(imm32)]), rex(0x81).digit(7).id(), _64b | compat),
        inst("cmpq", fmt("MI_SXL", [r(rm64), sxq(imm32)]), rex(0x81).w().digit(7).id(), _64b),
        inst("cmpw", fmt("MI_SXW", [r(rm16), sxw(imm8)]), rex([0x66, 0x83]).digit(7).ib(), _64b | compat),
        inst("cmpl", fmt("MI_SXB", [r(rm32), sxl(imm8)]), rex(0x83).digit(7).ib(), _64b | compat),
        inst("cmpq", fmt("MI_SXB", [r(rm64), sxq(imm8)]), rex(0x83).w().digit(7).ib(), _64b),
        inst("cmpb", fmt("MR", [r(rm8), r(r8)]), rex(0x38).r(), _64b | compat),
        inst("cmpw", fmt("MR", [r(rm16), r(r16)]), rex([0x66, 0x39]).r(), _64b | compat),
        inst("cmpl", fmt("MR", [r(rm32), r(r32)]), rex(0x39).r(), _64b | compat),
        inst("cmpq", fmt("MR", [r(rm64), r(r64)]), rex(0x39).w().r(), _64b),
        inst("cmpb", fmt("RM", [r(r8), r(rm8)]), rex(0x3A).r(), _64b | compat),
        inst("cmpw", fmt("RM", [r(r16), r(rm16)]), rex([0x66, 0x3B]).r(), _64b | compat),
        inst("cmpl", fmt("RM", [r(r32), r(rm32)]), rex(0x3B).r(), _64b | compat),
        inst("cmpq", fmt("RM", [r(r64), r(rm64)]), rex(0x3B).w().r(), _64b),
        // Vector instructions
        inst("cmppd", fmt("A", [rw(xmm), r(xmm_m128), r(imm8)]), rex([0x66, 0x0F, 0xC2]).r().ib(), _64b | compat | sse2),
        inst("cmpps", fmt("A", [rw(xmm), r(xmm_m128), r(imm8)]), rex([0x0F, 0xC2]).r().ib(), _64b | compat | sse),
        inst("cmpsd", fmt("A", [rw(xmm), r(xmm_m64), r(imm8)]), rex([0x66, 0xF2, 0x0F, 0xC2]).r().ib(), _64b | compat | sse2),
        inst("cmpss", fmt("A", [rw(xmm), r(xmm_m32), r(imm8)]), rex([0xF3, 0x0F, 0xC2]).r().ib(), _64b | compat | sse),
        inst("ucomisd", fmt("A", [r(xmm), r(xmm_m64)]).flags(W), rex([0x66, 0x0F, 0x2E]).r(), _64b | compat | sse2),
        inst("ucomiss", fmt("A", [r(xmm), r(xmm_m32)]).flags(W), rex([0x0F, 0x2E]).r(), _64b | compat | sse),
    ]
}
