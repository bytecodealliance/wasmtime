use crate::dsl::{Feature::*, Inst, Location::*};
use crate::dsl::{align, fmt, inst, r, rex, rw};

#[rustfmt::skip] // Keeps instructions on a single line.
pub fn list() -> Vec<Inst> {
    vec![
        // Floating-point maximum. Note, from the manual: "if the values being
        // compared are both 0.0s (of either sign), the value in the second
        // operand (source operand) is returned. If a value in the second
        // operand is an SNaN, then SNaN is forwarded unchanged to the
        // destination (that is, a QNaN version of the SNaN is not returned). If
        // only one value is a NaN (SNaN or QNaN) for this instruction, the
        // second operand (source operand), either a NaN or a valid
        // floating-point value, is written to the result. If instead of this
        // behavior, it is required that the NaN source operand (from either the
        // first or second operand) be returned, the action of MAXPS can be
        // emulated using a sequence of instructions, such as, a comparison
        // followed by AND, ANDN, and OR."
        inst("maxss", fmt("A", [rw(xmm1), r(xmm_m32)]), rex([0xF3, 0x0F, 0x5F]).r(), _64b | compat | sse),
        inst("maxsd", fmt("A", [rw(xmm1), r(xmm_m64)]), rex([0xF2, 0x0F, 0x5F]).r(), _64b | compat | sse2),
        inst("maxps", fmt("A", [rw(xmm1), r(align(xmm_m128))]), rex([0x0F, 0x5F]).r(), _64b | compat | sse),
        inst("maxpd", fmt("A", [rw(xmm1), r(align(xmm_m128))]), rex([0x66, 0x0F, 0x5F]).r(), _64b | compat | sse2),
        // Packed integer maximum.
        inst("pmaxsb", fmt("A", [rw(xmm1), r(align(xmm_m128))]), rex([0x66, 0x0F, 0x38, 0x3C]).r(), _64b | compat | sse41),
        inst("pmaxsw", fmt("A", [rw(xmm1), r(align(xmm_m128))]), rex([0x66, 0x0F, 0xEE]).r(), _64b | compat | sse2),
        inst("pmaxsd", fmt("A", [rw(xmm1), r(align(xmm_m128))]), rex([0x66, 0x0F, 0x38, 0x3D]).r(), _64b | compat | sse41),
        inst("pmaxub", fmt("A", [rw(xmm1), r(align(xmm_m128))]), rex([0x66, 0x0F, 0xDE]).r(), _64b | compat | sse2),
        inst("pmaxuw", fmt("A", [rw(xmm1), r(align(xmm_m128))]), rex([0x66, 0x0F, 0x38, 0x3E]).r(), _64b | compat | sse41),
        inst("pmaxud", fmt("A", [rw(xmm1), r(align(xmm_m128))]), rex([0x66, 0x0F, 0x38, 0x3F]).r(), _64b | compat | sse41),
    ]
}
