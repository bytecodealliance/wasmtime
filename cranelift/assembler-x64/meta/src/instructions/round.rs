use crate::dsl::{Feature::*, Inst, Location::*};
use crate::dsl::{fmt, inst, r, rex, w};

#[rustfmt::skip] // Keeps instructions on a single line.
pub fn list() -> Vec<Inst> {
    vec![
        inst("roundpd", fmt("RMI", [w(xmm1), r(xmm_m128), r(imm8)]), rex([0x66, 0x0F, 0x3A, 0x09]).ib(), _64b | compat | sse41),
        inst("roundps", fmt("RMI", [w(xmm1), r(xmm_m128), r(imm8)]), rex([0x66, 0x0F, 0x3A, 0x08]).ib(), _64b | compat | sse41),
        inst("roundsd", fmt("RMI", [w(xmm1), r(xmm_m128), r(imm8)]), rex([0x66, 0x0F, 0x3A, 0x0B]).ib(), _64b | compat | sse41),
        inst("roundss", fmt("RMI", [w(xmm1), r(xmm_m128), r(imm8)]), rex([0x66, 0x0F, 0x3A, 0x0A]).ib(), _64b | compat | sse41),
        ]
}
