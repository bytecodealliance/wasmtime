use crate::dsl::{align, fmt, inst, r, rex, rw};
use crate::dsl::{Feature::*, Inst, Location::*};

#[rustfmt::skip] // Keeps instructions on a single line.
pub fn list() -> Vec<Inst> {
    vec![
        // Vector instructions.
        inst("sqrtpd", fmt("A", [rw(xmm), r(align(xmm_m128))]), rex([0x66, 0x0F, 0x51]).r(), _64b | compat | sse2),
        inst("sqrtps", fmt("A", [rw(xmm), r(align(xmm_m128))]), rex([0x0F, 0x51]).r(), _64b | compat | sse),
        inst("sqrtsd", fmt("A", [rw(xmm), r(align(xmm_m128))]), rex([0xF2, 0x0F, 0x51]).r(), _64b | compat | sse2),
        inst("sqrtss", fmt("A", [rw(xmm), r(align(xmm_m128))]), rex([0xF3, 0x0F, 0x51]).r(), _64b | compat | sse),
    ]
}
