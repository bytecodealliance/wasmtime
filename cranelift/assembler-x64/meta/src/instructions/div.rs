use crate::dsl::{Feature::*, Inst, Location::*, VexLength::*, align};
use crate::dsl::{fmt, implicit, inst, r, rex, rw, vex};

#[rustfmt::skip] // Keeps instructions on a single line.
pub fn list() -> Vec<Inst> {
    vec![
        inst("divb", fmt("M", [rw(implicit(ax)), r(rm8)]), rex([0xF6]).digit(6), _64b | compat).has_trap(),
        inst("divw", fmt("M", [rw(implicit(ax)), rw(implicit(dx)), r(rm16)]), rex([0x66, 0xF7]).digit(6), _64b | compat).has_trap(),
        inst("divl", fmt("M", [rw(implicit(eax)), rw(implicit(edx)), r(rm32)]), rex([0xF7]).digit(6), _64b | compat).has_trap(),
        inst("divq", fmt("M", [rw(implicit(rax)), rw(implicit(rdx)), r(rm64)]), rex([0xF7]).digit(6).w(), _64b).has_trap(),
        inst("idivb", fmt("M", [rw(implicit(ax)), r(rm8)]), rex([0xF6]).digit(7), _64b | compat).has_trap(),
        inst("idivw", fmt("M", [rw(implicit(ax)), rw(implicit(dx)), r(rm16)]), rex([0x66, 0xF7]).digit(7), _64b | compat).has_trap(),
        inst("idivl", fmt("M", [rw(implicit(eax)), rw(implicit(edx)), r(rm32)]), rex([0xF7]).digit(7), _64b | compat).has_trap(),
        inst("idivq", fmt("M", [rw(implicit(rax)), rw(implicit(rdx)), r(rm64)]), rex([0xF7]).digit(7).w(), _64b).has_trap(),
        // Vector instructions.
        inst("divps", fmt("A", [rw(xmm1), align(xmm_m128)]), rex([0xF, 0x5E]).r(), _64b | compat | sse),
        inst("divpd", fmt("A", [rw(xmm1), align(xmm_m128)]), rex([0x66, 0x0F, 0x5E]).r(), _64b | compat | sse2),
        inst("divss", fmt("A", [rw(xmm1), r(xmm_m32)]), rex([0xF3, 0xF, 0x5E]).r(), _64b | compat | sse),
        inst("divsd", fmt("A", [rw(xmm1), r(xmm_m64)]), rex([0xF2, 0xF, 0x5E]).r(), _64b | compat | sse2),
        inst("vdivps", fmt("B", [rw(xmm1), r(xmm2), r(xmm_m128)]), vex(L128)._0f().op(0x5E).r(), _64b | compat | avx),
        inst("vdivpd", fmt("B", [rw(xmm1), r(xmm2), r(xmm_m128)]), vex(L128)._66()._0f().op(0x5E).r(), _64b | compat | avx),
        inst("vdivss", fmt("B", [rw(xmm1), r(xmm2), r(xmm_m32)]), vex(L128)._f3()._0f().op(0x5E).r(), _64b | compat | avx),
        inst("vdivsd", fmt("B", [rw(xmm1), r(xmm2), r(xmm_m64)]), vex(L128)._f2()._0f().op(0x5E).r(), _64b | compat | avx),
    ]
}
