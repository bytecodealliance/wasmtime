use crate::dsl::{Customization::*, Eflags::*, Feature::*, Inst, Location::*, VexLength::*};
use crate::dsl::{align, fmt, inst, r, rex, rw, vex, w};

#[rustfmt::skip] // Keeps instructions on a single line.
pub fn list() -> Vec<Inst> {
    vec![
        inst("pcmpeqb", fmt("A", [rw(xmm1), r(align(xmm_m128))]), rex([0x66, 0x0F, 0x74]), _64b | compat | sse2),
        inst("pcmpeqw", fmt("A", [rw(xmm1), r(align(xmm_m128))]), rex([0x66, 0x0F, 0x75]), _64b | compat | sse2),
        inst("pcmpeqd", fmt("A", [rw(xmm1), r(align(xmm_m128))]), rex([0x66, 0x0F, 0x76]), _64b | compat | sse2),
        inst("pcmpeqq", fmt("A", [rw(xmm1), r(align(xmm_m128))]), rex([0x66, 0x0F, 0x38, 0x29]), _64b | compat | sse41),
        inst("pcmpgtb", fmt("A", [rw(xmm1), r(align(xmm_m128))]), rex([0x66, 0x0F, 0x64]), _64b | compat | sse2),
        inst("pcmpgtw", fmt("A", [rw(xmm1), r(align(xmm_m128))]), rex([0x66, 0x0F, 0x65]), _64b | compat | sse2),
        inst("pcmpgtd", fmt("A", [rw(xmm1), r(align(xmm_m128))]), rex([0x66, 0x0F, 0x66]), _64b | compat | sse2),
        inst("pcmpgtq", fmt("A", [rw(xmm1), r(align(xmm_m128))]), rex([0x66, 0x0F, 0x38, 0x37]), _64b | compat | sse42),

        // AVX versions
        inst("vpcmpeqb", fmt("B", [w(xmm1), r(xmm2), r(xmm_m128)]), vex(L128)._66()._0f().op(0x74), _64b | compat | avx),
        inst("vpcmpeqw", fmt("B", [w(xmm1), r(xmm2), r(xmm_m128)]), vex(L128)._66()._0f().op(0x75), _64b | compat | avx),
        inst("vpcmpeqd", fmt("B", [w(xmm1), r(xmm2), r(xmm_m128)]), vex(L128)._66()._0f().op(0x76), _64b | compat | avx),
        inst("vpcmpeqq", fmt("B", [w(xmm1), r(xmm2), r(xmm_m128)]), vex(L128)._66()._0f38().op(0x29), _64b | compat | avx),
        inst("vpcmpgtb", fmt("B", [w(xmm1), r(xmm2), r(xmm_m128)]), vex(L128)._66()._0f().op(0x64), _64b | compat | avx),
        inst("vpcmpgtw", fmt("B", [w(xmm1), r(xmm2), r(xmm_m128)]), vex(L128)._66()._0f().op(0x65), _64b | compat | avx),
        inst("vpcmpgtd", fmt("B", [w(xmm1), r(xmm2), r(xmm_m128)]), vex(L128)._66()._0f().op(0x66), _64b | compat | avx),
        inst("vpcmpgtq", fmt("B", [w(xmm1), r(xmm2), r(xmm_m128)]), vex(L128)._66()._0f38().op(0x37), _64b | compat | avx),
        inst("cmppd", fmt("A", [rw(xmm1), r(xmm_m128), r(imm8)]), rex([0x66, 0x0F, 0xC2]).r().ib(), _64b | compat | sse2).custom(Display),
        inst("cmpps", fmt("A", [rw(xmm1), r(xmm_m128), r(imm8)]), rex([0x0F, 0xC2]).r().ib(), _64b | compat | sse).custom(Display),
        inst("cmpsd", fmt("A", [rw(xmm1), r(xmm_m64), r(imm8)]), rex([0x66, 0xF2, 0x0F, 0xC2]).r().ib(), _64b | compat | sse2).custom(Display),
        inst("cmpss", fmt("A", [rw(xmm1), r(xmm_m32), r(imm8)]), rex([0xF3, 0x0F, 0xC2]).r().ib(), _64b | compat | sse).custom(Display),
        inst("ucomisd", fmt("A", [r(xmm1), r(xmm_m64)]).flags(W), rex([0x66, 0x0F, 0x2E]).r(), _64b | compat | sse2),
        inst("ucomiss", fmt("A", [r(xmm1), r(xmm_m32)]).flags(W), rex([0x0F, 0x2E]).r(), _64b | compat | sse),
    ]
}
