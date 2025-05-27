use crate::dsl::{Feature::*, Inst, Location::*};
use crate::dsl::{fmt, inst, r, rex, rw};

#[rustfmt::skip] // Keeps instructions on a single line.
pub fn list() -> Vec<Inst> {
    vec![
        // Vector instructions.
        inst("unpcklps", fmt("A", [rw(xmm1), r(xmm_m128)]), rex([0xF, 0x14]).r(), _64b | compat | sse),
        inst("unpcklpd", fmt("A", [rw(xmm1), r(xmm_m128)]), rex([0x66, 0x0F, 0x14]).r(), _64b | compat | sse2),
        inst("unpckhps", fmt("A", [rw(xmm1), r(xmm_m128)]), rex([0xF, 0x15]).r(), _64b | compat | sse),
        inst("punpckhbw", fmt("A", [rw(xmm1), r(xmm_m128)]), rex([0x66, 0x0F, 0x68]).r(), _64b | compat | sse2),
        inst("punpckhwd", fmt("A", [rw(xmm1), r(xmm_m128)]), rex([0x66, 0x0F, 0x69]).r(), _64b | compat | sse2),
        inst("punpckhdq", fmt("A", [rw(xmm1), r(xmm_m128)]), rex([0x66, 0x0F, 0x6A]).r(), _64b | compat | sse2),
        inst("punpcklwd", fmt("A", [rw(xmm1), r(xmm_m128)]), rex([0x66, 0x0F, 0x61]).r(), _64b | compat | sse2),
        inst("punpcklqdq", fmt("A", [rw(xmm1), r(xmm_m128)]), rex([0x66, 0x0F, 0x6C]).r(), _64b | compat | sse2),
        inst("punpcklbw", fmt("A", [rw(xmm1), r(xmm_m128)]), rex([0x66, 0x0F, 0x60]).r(), _64b | compat | sse2),
        inst("punpckldq", fmt("A", [rw(xmm1), r(xmm_m128)]), rex([0x66, 0x0F, 0x62]).r(), _64b | compat | sse2),
        inst("punpckhqdq", fmt("A", [rw(xmm1), r(xmm_m128)]), rex([0x66, 0x0F, 0x6D]).r(), _64b | compat | sse2),
    ]
}
