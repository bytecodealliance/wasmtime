use crate::dsl::{Feature::*, Inst, Location::*, VexLength::*};
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
    ]
}
