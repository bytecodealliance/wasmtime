use crate::dsl::{Feature::*, Inst, Location::*};
use crate::dsl::{align, fmt, inst, r, rex, rw};

#[rustfmt::skip] // Keeps instructions on a single line.
pub fn list() -> Vec<Inst> {
    vec![
        // Floating-point minimum. Note, this has some tricky NaN and sign bit
        // behavior; see `max.rs`.
        inst("minss", fmt("A", [rw(xmm), r(xmm_m32)]), rex([0xF3, 0x0F, 0x5D]).r(), _64b | compat | sse),
        inst("minsd", fmt("A", [rw(xmm), r(xmm_m64)]), rex([0xF2, 0x0F, 0x5D]).r(), _64b | compat | sse2),
        inst("minps", fmt("A", [rw(xmm), r(align(xmm_m128))]), rex([0x0F, 0x5D]).r(), _64b | compat | sse),
        inst("minpd", fmt("A", [rw(xmm), r(align(xmm_m128))]), rex([0x66, 0x0F, 0x5D]).r(), _64b | compat | sse2),
    ]
}
