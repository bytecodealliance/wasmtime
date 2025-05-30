use crate::dsl::{Eflags::*, Feature::*, Inst, Location::*};
use crate::dsl::{fmt, inst, r, rex, rw, sxl, sxq, sxw};

#[rustfmt::skip] // Keeps instructions on a single line.
pub fn list() -> Vec<Inst> {
    vec![
        inst("cmppd", fmt("A", [rw(xmm1), r(xmm_m128), r(imm8)]), rex([0x66, 0x0F, 0xC2]).r().ib(), _64b | compat | sse2),
        inst("cmpps", fmt("A", [rw(xmm1), r(xmm_m128), r(imm8)]), rex([0x0F, 0xC2]).r().ib(), _64b | compat | sse),
        inst("cmpsd", fmt("A", [rw(xmm1), r(xmm_m64), r(imm8)]), rex([0x66, 0xF2, 0x0F, 0xC2]).r().ib(), _64b | compat | sse2),
        inst("cmpss", fmt("A", [rw(xmm1), r(xmm_m32), r(imm8)]), rex([0xF3, 0x0F, 0xC2]).r().ib(), _64b | compat | sse),
        inst("ucomisd", fmt("A", [r(xmm1), r(xmm_m64)]).flags(W), rex([0x66, 0x0F, 0x2E]).r(), _64b | compat | sse2),
        inst("ucomiss", fmt("A", [r(xmm1), r(xmm_m32)]).flags(W), rex([0x0F, 0x2E]).r(), _64b | compat | sse),
    ]
}
