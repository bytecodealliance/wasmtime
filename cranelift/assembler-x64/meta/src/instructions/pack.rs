use crate::dsl::{Feature::*, Inst, Location::*, VexLength::*};
use crate::dsl::{align, fmt, inst, r, rex, rw, vex, w};

#[rustfmt::skip] // Keeps instructions on a single line.
pub fn list() -> Vec<Inst> {
    vec![
        inst("packsswb", fmt("A", [rw(xmm1), r(align(xmm_m128))]), rex([0x66, 0x0F, 0x63]), _64b | compat | sse2).alt(avx, "vpacksswb_b"),
        inst("vpacksswb", fmt("B", [w(xmm1), r(xmm2), r(xmm_m128)]), vex(L128)._66()._0f().op(0x63), _64b | compat | avx),
        inst("packssdw", fmt("A", [rw(xmm1), r(align(xmm_m128))]), rex([0x66, 0x0F, 0x6B]), _64b | compat | sse2).alt(avx, "vpackssdw_b"),
        inst("vpackssdw", fmt("B", [w(xmm1),  r(xmm2), r(xmm_m128)]), vex(L128)._66()._0f().op(0x6B), _64b | compat | avx),
        inst("packusdw", fmt("A", [rw(xmm1), r(align(xmm_m128))]), rex([0x66, 0x0F, 0x38, 0x2B]), _64b | compat | sse41).alt(avx, "vpackusdw_b"),
        inst("vpackusdw", fmt("B", [w(xmm1), r(xmm2), r(xmm_m128)]), vex(L128)._66()._0f38().op(0x2B), _64b | compat | avx),
        inst("packuswb", fmt("A", [rw(xmm1), r(align(xmm_m128))]), rex([0x66, 0x0F, 0x67]), _64b | compat | sse2).alt(avx, "vpackuswb_b"),
        inst("vpackuswb", fmt("B", [w(xmm1), r(xmm2), r(xmm_m128)]), vex(L128)._66()._0f().op(0x67), _64b | compat | avx),
    ]
}
