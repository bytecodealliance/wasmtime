use crate::dsl::{Feature::*, Inst, Location::*, VexLength::*};
use crate::dsl::{align, fmt, inst, r, rex, rw, vex, w};

#[rustfmt::skip] // Keeps instructions on a single line.
pub fn list() -> Vec<Inst> {
    vec![
        inst("pmaddubsw", fmt("A", [rw(xmm1), r(align(xmm_m128))]), rex([0x66, 0x0F, 0x38, 0x04]), _64b | compat | ssse3).alt(avx, "vpmaddubsw_b"),
        inst("vpmaddubsw", fmt("B", [w(xmm1), r(xmm2), r(xmm_m128)]), vex(L128)._66()._0f38().op(0x04), _64b | compat | avx),
        inst("pmaddwd", fmt("A", [rw(xmm1), r(align(xmm_m128))]), rex([0x66, 0x0F, 0xF5]), _64b | compat | sse2).alt(avx, "vpmaddwd_b"),
        inst("vpmaddwd", fmt("B", [w(xmm1), r(xmm2), r(xmm_m128)]), vex(L128)._66()._0f().op(0xF5), _64b | compat | avx),
     ]
}
