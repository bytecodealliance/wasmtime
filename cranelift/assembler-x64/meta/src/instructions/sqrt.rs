use crate::dsl::{Feature::*, Inst, Location::*};
use crate::dsl::{align, fmt, inst, r, rex, rw, w};

#[rustfmt::skip] // Keeps instructions on a single line.
pub fn list() -> Vec<Inst> {
    vec![
        // Vector instructions.
        inst("sqrtss", fmt("A", [rw(xmm1), r(xmm_m32)]), rex([0xF3, 0x0F, 0x51]).r(), _64b | compat | sse),
        inst("sqrtsd", fmt("A", [rw(xmm1), r(xmm_m64)]), rex([0xF2, 0x0F, 0x51]).r(), _64b | compat | sse2),
        inst("sqrtps", fmt("A", [w(xmm1), r(align(xmm_m128))]), rex([0x0F, 0x51]).r(), _64b | compat | sse),
        inst("sqrtpd", fmt("A", [w(xmm1), r(align(xmm_m128))]), rex([0x66, 0x0F, 0x51]).r(), _64b | compat | sse2),
    ]
}
