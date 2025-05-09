use crate::dsl::{Feature::*, Inst, Location::*};
use crate::dsl::{align, fmt, inst, r, rex, w};

#[rustfmt::skip] // Keeps instructions on a single line.
pub fn list() -> Vec<Inst> {
    vec![
        // Vector instructions.
        inst("sqrtpd", fmt("A", [w(xmm), r(align(xmm_m128))]), rex([0x66, 0x0F, 0x51]).r(), _64b | compat | sse2),
        inst("sqrtps", fmt("A", [w(xmm), r(align(xmm_m128))]), rex([0x0F, 0x51]).r(), _64b | compat | sse),
    ]
}
