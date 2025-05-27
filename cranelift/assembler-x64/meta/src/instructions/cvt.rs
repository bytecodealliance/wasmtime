use crate::dsl::{Feature::*, Inst, Location::*};
use crate::dsl::{align, fmt, inst, r, rex, rw, w};

#[rustfmt::skip] // Keeps instructions on a single line.
pub fn list() -> Vec<Inst> {
    vec![
        // From 32-bit floating point.
        inst("cvtps2pd", fmt("A", [w(xmm1), r(xmm_m64)]), rex([0x0F, 0x5A]).r(), _64b | compat | sse2),
        inst("cvttps2dq", fmt("A", [w(xmm1), r(align(xmm_m128))]), rex([0xF3, 0x0F, 0x5B]).r(), _64b | compat | sse2),
        inst("cvtss2sd", fmt("A", [rw(xmm1), r(xmm_m32)]), rex([0xF3, 0x0F, 0x5A]).r(), _64b | compat | sse2),
        inst("cvtss2si", fmt("A", [w(r32), r(xmm_m32)]), rex([0xF3, 0x0F, 0x2D]).r(), _64b | compat | sse),
        inst("cvtss2si", fmt("AQ", [w(r64), r(xmm_m32)]), rex([0xF3, 0x0F, 0x2D]).w().r(), _64b | sse),
        inst("cvttss2si", fmt("A", [w(r32), r(xmm_m32)]), rex([0xF3, 0x0F, 0x2C]).r(), _64b | compat | sse),
        inst("cvttss2si", fmt("AQ", [w(r64), r(xmm_m32)]), rex([0xF3, 0x0F, 0x2C]).w().r(), _64b | sse),
        // From 64-bit floating point.
        inst("cvtpd2ps", fmt("A", [w(xmm1), r(align(xmm_m128))]), rex([0x66, 0x0F, 0x5A]).r(), _64b | compat | sse2),
        inst("cvttpd2dq", fmt("A", [w(xmm1), r(align(xmm_m128))]), rex([0x66, 0x0F, 0xE6]).r(), _64b | compat | sse2),
        inst("cvtsd2ss", fmt("A", [rw(xmm1), r(xmm_m64)]), rex([0xF2, 0x0F, 0x5A]).r(), _64b | compat | sse2),
        inst("cvtsd2si", fmt("A", [w(r32), r(xmm_m64)]), rex([0xF2, 0x0F, 0x2D]).r(), _64b | compat | sse2),
        inst("cvtsd2si", fmt("AQ", [w(r64), r(xmm_m64)]), rex([0xF2, 0x0F, 0x2D]).w().r(), _64b | sse2),
        inst("cvttsd2si", fmt("A", [w(r32), r(xmm_m64)]), rex([0xF2, 0x0F, 0x2C]).r(), _64b | compat | sse2),
        inst("cvttsd2si", fmt("AQ", [w(r64), r(xmm_m64)]), rex([0xF2, 0x0F, 0x2C]).w().r(), _64b | sse2),
        // From signed 32-bit integer.
        inst("cvtdq2ps", fmt("A", [w(xmm1), r(align(xmm_m128))]), rex([0x0F, 0x5B]).r(), _64b | compat | sse2),
        inst("cvtdq2pd", fmt("A", [w(xmm1), r(xmm_m64)]), rex([0xF3, 0x0F, 0xE6]).r(), _64b | compat | sse2),
        inst("cvtsi2ssl", fmt("A", [rw(xmm1), r(rm32)]), rex([0xF3, 0x0F, 0x2A]).r(), _64b | compat | sse),
        inst("cvtsi2ssq", fmt("A", [rw(xmm1), r(rm64)]), rex([0xF3, 0x0F, 0x2A]).w().r(), _64b | sse),
        inst("cvtsi2sdl", fmt("A", [rw(xmm1), r(rm32)]), rex([0xF2, 0x0F, 0x2A]).r(), _64b | compat | sse2),
        inst("cvtsi2sdq", fmt("A", [rw(xmm1), r(rm64)]), rex([0xF2, 0x0F, 0x2A]).w().r(), _64b | sse2),
    ]
}
