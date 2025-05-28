use crate::dsl::{Feature::*, Inst, Location::*};
use crate::dsl::{align, fmt, inst, r, rex, rw, w};

#[rustfmt::skip] // Keeps instructions on a single line.
pub fn list() -> Vec<Inst> {
    vec![
        inst("movd", fmt("A", [w(xmm1), r(rm32)]), rex([0x66, 0x0F, 0x6E]).r(), _64b | compat | sse2),
        inst("movq", fmt("A", [w(xmm1), r(rm64)]), rex([0x66, 0x0F, 0x6E]).r().w(), _64b | sse2),
        inst("movd", fmt("B", [w(rm32), r(xmm2)]), rex([0x66, 0x0F, 0x7E]).r(), _64b | compat | sse2),
        inst("movq", fmt("B", [w(rm64), r(xmm2)]), rex([0x66, 0x0F, 0x7E]).r().w(), _64b | sse2),

        // Note that `movss` and `movsd` only have an "A" and "C" modes listed
        // in the Intel manual but here they're split into "*_M" and "*_R" to
        // model the different regalloc behavior each one has. Notably the
        // memory-using variant does the usual read or write the memory
        // depending on the instruction, but the "*_R" variant both reads and
        // writes the destination register because the upper bits are preserved.
        //
        // Additionally "C_R" is not specified here since it's not needed over
        // the "A_R" variant and it's additionally not encoded correctly as the
        // destination must be modeled in the ModRM:r/m byte, not the ModRM:reg
        // byte. Currently our encoding based on format doesn't account for this
        // special case, so it's just dropped here.
        inst("movss", fmt("A_M", [w(xmm1), r(m32)]), rex([0xF3, 0x0F, 0x10]).r(), _64b | sse),
        inst("movss", fmt("A_R", [rw(xmm1), r(xmm2)]), rex([0xF3, 0x0F, 0x10]).r(), _64b | sse),
        inst("movss", fmt("C_M", [w(m64), r(xmm1)]), rex([0xF3, 0x0F, 0x11]).r(), _64b | sse),
        inst("movsd", fmt("A_M", [w(xmm1), r(m32)]), rex([0xF2, 0x0F, 0x10]).r(), _64b | sse2),
        inst("movsd", fmt("A_R", [rw(xmm1), r(xmm2)]), rex([0xF2, 0x0F, 0x10]).r(), _64b | sse2),
        inst("movsd", fmt("C_M", [w(m64), r(xmm1)]), rex([0xF2, 0x0F, 0x11]).r(), _64b | sse2),

        inst("movapd", fmt("A", [w(xmm1), r(align(xmm_m128))]), rex([0x66, 0x0F, 0x28]).r(), _64b | sse2),
        inst("movapd", fmt("B", [w(align(xmm_m128)), r(xmm1)]), rex([0x66, 0x0F, 0x29]).r(), _64b | sse2),
        inst("movaps", fmt("A", [w(xmm1), r(align(xmm_m128))]), rex([0x0F, 0x28]).r(), _64b | sse),
        inst("movaps", fmt("B", [w(align(xmm_m128)), r(xmm1)]), rex([0x0F, 0x29]).r(), _64b | sse),
        inst("movupd", fmt("A", [w(xmm1), r(xmm_m128)]), rex([0x66, 0x0F, 0x10]).r(), _64b | sse2),
        inst("movupd", fmt("B", [w(xmm_m128), r(xmm1)]), rex([0x66, 0x0F, 0x11]).r(), _64b | sse2),
        inst("movups", fmt("A", [w(xmm1), r(xmm_m128)]), rex([0x0F, 0x10]).r(), _64b | sse),
        inst("movups", fmt("B", [w(xmm_m128), r(xmm1)]), rex([0x0F, 0x11]).r(), _64b | sse),
        inst("movdqa", fmt("A", [w(xmm1), r(align(xmm_m128))]), rex([0x66, 0x0F, 0x6F]).r(), _64b | sse2),
        inst("movdqa", fmt("B", [w(align(xmm_m128)), r(xmm1)]), rex([0x66, 0x0F, 0x7F]).r(), _64b | sse2),
        inst("movdqu", fmt("A", [w(xmm1), r(xmm_m128)]), rex([0xF3, 0x0F, 0x6F]).r(), _64b | sse2),
        inst("movdqu", fmt("B", [w(xmm_m128), r(xmm1)]), rex([0xF3, 0x0F, 0x7F]).r(), _64b | sse2),
    ]
}
