use crate::dsl::{Feature::*, Inst, Length::*, Location::*};
use crate::dsl::{align, fmt, inst, r, rex, vex, w};

#[rustfmt::skip] // Keeps instructions on a single line.
pub fn list() -> Vec<Inst> {
    vec![
        inst("pabsb", fmt("A", [w(xmm1), r(align(xmm_m128))]), rex([0x66, 0x0F, 0x38, 0x1C]), _64b | compat | ssse3).alt(avx, "vpabsb_a"),
        inst("vpabsb", fmt("A", [w(xmm1), r(xmm_m128)]), vex(L128)._66()._0f38().op(0x1C), _64b | compat | avx),
        inst("pabsw", fmt("A", [w(xmm1), r(align(xmm_m128))]), rex([0x66, 0x0F, 0x38, 0x1D]), _64b | compat | ssse3).alt(avx, "vpabsw_a"),
        inst("vpabsw", fmt("A", [w(xmm1), r(xmm_m128)]), vex(L128)._66()._0f38().op(0x1D), _64b | compat | avx),
        inst("pabsd", fmt("A", [w(xmm1), r(align(xmm_m128))]), rex([0x66, 0x0F, 0x38, 0x1E]), _64b | compat | ssse3).alt(avx, "vpabsd_a"),
        inst("vpabsd", fmt("A", [w(xmm1), r(xmm_m128)]), vex(L128)._66()._0f38().op(0x1E), _64b | compat | avx),
    ]
}
