use crate::dsl::{Feature::*, Inst, Location::*, VexLength::*};
use crate::dsl::{align, fmt, inst, r, rex, rw, sxl, sxq, sxw, vex, w};

#[rustfmt::skip] // Keeps instructions on a single line.
pub fn list() -> Vec<Inst> {
    vec![
        // Move integers to and from GPR and memory locations. Note that, in
        // 64-bit mode, `r/m8` can not be encoded to access the following byte
        // registers if a REX prefix is used: AH, BH, CH, DH. Only moves that
        // overwrite all 64 bits are considered "write-only"; smaller-width
        // moves indicate that upper bits are preserved by being "read-write."
        inst("movb", fmt("MR", [w(rm8), r(r8)]), rex(0x88).r(), _64b | compat),
        inst("movw", fmt("MR", [w(rm16), r(r16)]), rex([0x66, 0x89]).r(), _64b | compat),
        inst("movl", fmt("MR", [w(rm32), r(r32)]), rex(0x89).r(), _64b | compat),
        inst("movq", fmt("MR", [w(rm64), r(r64)]), rex(0x89).w().r(), _64b),
        inst("movb", fmt("RM", [w(r8), r(rm8)]), rex(0x8A).r(), _64b | compat),
        inst("movw", fmt("RM", [w(r16), r(rm16)]), rex([0x66, 0x8B]).r(), _64b | compat),
        inst("movl", fmt("RM", [w(r32), r(rm32)]), rex(0x8B).r(), _64b | compat),
        inst("movq", fmt("RM", [w(r64), r(rm64)]), rex(0x8B).w().r(), _64b),
        inst("movb", fmt("OI", [w(r8), r(imm8)]), rex(0xB0).rb().ib(), _64b | compat),
        inst("movw", fmt("OI", [w(r16), r(imm16)]), rex([0x66, 0xB8]).rw().iw(), _64b | compat),
        inst("movl", fmt("OI", [w(r32), r(imm32)]), rex(0xB8).rd().id(), _64b | compat),
        // Capstone disassembles this (and only this) slightly differently.
        inst("movabsq", fmt("OI", [w(r64), r(imm64)]), rex(0xB8).w().ro().io(), _64b),
        inst("movb", fmt("MI", [w(rm8), r(imm8)]), rex(0xC6).digit(0).ib(), _64b | compat),
        inst("movw", fmt("MI", [w(rm16), r(imm16)]), rex([0x66, 0xC7]).digit(0).iw(), _64b | compat),
        inst("movl", fmt("MI", [w(rm32), r(imm32)]), rex(0xC7).digit(0).id(), _64b | compat),
        inst("movq", fmt("MI_SXL", [w(rm64), sxq(imm32)]), rex(0xC7).w().digit(0).id(), _64b),

        // Move integers with sign extension. These are defined as `movsx` in
        // the x64 reference manual but Capstone (and likely other tools)
        // disassemble this as `movs{from}{to}`.
        inst("movsbw", fmt("RM", [w(r16), sxw(rm8)]), rex([0x66, 0x0F, 0xBE]).r(), _64b | compat),
        inst("movsbl", fmt("RM", [w(r32), sxl(rm8)]), rex([0x0F, 0xBE]).r(), _64b | compat),
        inst("movsbq", fmt("RM", [w(r64), sxq(rm8)]), rex([0x0F, 0xBE]).w().r(), _64b),
        inst("movsww", fmt("RM", [w(r16), sxl(rm16)]), rex([0x66, 0x0F, 0xBF]).r(), _64b | compat),
        inst("movswl", fmt("RM", [w(r32), sxl(rm16)]), rex([0x0F, 0xBF]).r(), _64b | compat),
        inst("movswq", fmt("RM", [w(r64), sxq(rm16)]), rex([0x0F, 0xBF]).w().r(), _64b),
        inst("movslq", fmt("RM", [w(r64), sxl(rm32)]), rex(0x63).w().r(), _64b),

        // Move integers with zero extension. These are defined as `movzx` in
        // the x64 reference manual but Capstone (and likely other tools)
        // disassemble this as `movz{from}{to}`.
        inst("movzbw", fmt("RM", [w(r16), sxw(rm8)]), rex([0x66, 0x0F, 0xB6]).r(), _64b | compat),
        inst("movzbl", fmt("RM", [w(r32), sxl(rm8)]), rex([0x0F, 0xB6]).r(), _64b | compat),
        inst("movzbq", fmt("RM", [w(r64), sxq(rm8)]), rex([0x0F, 0xB6]).w().r(), _64b),
        inst("movzww", fmt("RM", [w(r16), sxl(rm16)]), rex([0x66, 0x0F, 0xB7]).r(), _64b | compat),
        inst("movzwl", fmt("RM", [w(r32), sxl(rm16)]), rex([0x0F, 0xB7]).r(), _64b | compat),
        inst("movzwq", fmt("RM", [w(r64), sxq(rm16)]), rex([0x0F, 0xB7]).w().r(), _64b),

        // Move integers between GPR and XMM locations. From the reference
        // manual: "when the destination operand is an XMM register, the source
        // operand is written to the low doubleword of the register, and the
        // register is zero-extended to 128 bits."
        inst("movd", fmt("A", [w(xmm1), r(rm32)]), rex([0x66, 0x0F, 0x6E]).r(), _64b | compat | sse2),
        inst("movq", fmt("A", [w(xmm1), r(rm64)]), rex([0x66, 0x0F, 0x6E]).r().w(), _64b | sse2),
        inst("movd", fmt("B", [w(rm32), r(xmm2)]), rex([0x66, 0x0F, 0x7E]).r(), _64b | compat | sse2),
        inst("movq", fmt("B", [w(rm64), r(xmm2)]), rex([0x66, 0x0F, 0x7E]).r().w(), _64b | sse2),
        inst("vmovd", fmt("A", [w(xmm1), r(rm32)]), vex(L128)._66()._0f().w0().op(0x6E).r(), _64b | compat | avx),
        inst("vmovq", fmt("A", [w(xmm1), r(rm64)]), vex(L128)._66()._0f().w1().op(0x6E).r(), _64b | avx),
        inst("vmovd", fmt("B", [w(rm32), r(xmm2)]), vex(L128)._66()._0f().w0().op(0x7E).r(), _64b | compat | avx),
        inst("vmovq", fmt("B", [w(rm64), r(xmm2)]), vex(L128)._66()._0f().w1().op(0x7E).r(), _64b | avx),

        // Move floating-point values to and from XMM locations. Some
        // memory-loading versions of `movs*` clear the upper bits of the XMM
        // destination.
        //
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

        // Move aligned 128-bit values to and from XMM locations.
        inst("movapd", fmt("A", [w(xmm1), r(align(xmm_m128))]), rex([0x66, 0x0F, 0x28]).r(), _64b | sse2),
        inst("movapd", fmt("B", [w(align(xmm_m128)), r(xmm1)]), rex([0x66, 0x0F, 0x29]).r(), _64b | sse2),
        inst("movaps", fmt("A", [w(xmm1), r(align(xmm_m128))]), rex([0x0F, 0x28]).r(), _64b | sse),
        inst("movaps", fmt("B", [w(align(xmm_m128)), r(xmm1)]), rex([0x0F, 0x29]).r(), _64b | sse),
        inst("movdqa", fmt("A", [w(xmm1), r(align(xmm_m128))]), rex([0x66, 0x0F, 0x6F]).r(), _64b | sse2),
        inst("movdqa", fmt("B", [w(align(xmm_m128)), r(xmm1)]), rex([0x66, 0x0F, 0x7F]).r(), _64b | sse2),

        // Move unaligned 128-bit values to and from XMM locations.
        inst("movupd", fmt("A", [w(xmm1), r(xmm_m128)]), rex([0x66, 0x0F, 0x10]).r(), _64b | sse2),
        inst("movupd", fmt("B", [w(xmm_m128), r(xmm1)]), rex([0x66, 0x0F, 0x11]).r(), _64b | sse2),
        inst("movups", fmt("A", [w(xmm1), r(xmm_m128)]), rex([0x0F, 0x10]).r(), _64b | sse),
        inst("movups", fmt("B", [w(xmm_m128), r(xmm1)]), rex([0x0F, 0x11]).r(), _64b | sse),
        inst("movdqu", fmt("A", [w(xmm1), r(xmm_m128)]), rex([0xF3, 0x0F, 0x6F]).r(), _64b | sse2),
        inst("movdqu", fmt("B", [w(xmm_m128), r(xmm1)]), rex([0xF3, 0x0F, 0x7F]).r(), _64b | sse2),

        // Move and extend packed integers to and from XMM locations with sign extension.
        inst("pmovsxbw", fmt("A", [w(xmm1), r(xmm_m64)]), rex([0x66, 0x0F, 0x38, 0x20]).r(), _64b | compat | sse41),
        inst("pmovsxbd", fmt("A", [w(xmm1), r(xmm_m32)]), rex([0x66, 0x0F, 0x38, 0x21]).r(), _64b | compat | sse41),
        inst("pmovsxbq", fmt("A", [w(xmm1), r(xmm_m16)]), rex([0x66, 0x0F, 0x38, 0x22]).r(), _64b | compat | sse41),
        inst("pmovsxwd", fmt("A", [w(xmm1), r(xmm_m64)]), rex([0x66, 0x0F, 0x38, 0x23]).r(), _64b | compat | sse41),
        inst("pmovsxwq", fmt("A", [w(xmm1), r(xmm_m32)]), rex([0x66, 0x0F, 0x38, 0x24]).r(), _64b | compat | sse41),
        inst("pmovsxdq", fmt("A", [w(xmm1), r(xmm_m64)]), rex([0x66, 0x0F, 0x38, 0x25]).r(), _64b | compat | sse41),

        // Move and extend packed integers to and from XMM locations with zero extension.
        inst("pmovzxbw", fmt("A", [w(xmm1), r(xmm_m64)]), rex([0x66, 0x0F, 0x38, 0x30]).r(), _64b | compat | sse41),
        inst("pmovzxbd", fmt("A", [w(xmm1), r(xmm_m32)]), rex([0x66, 0x0F, 0x38, 0x31]).r(), _64b | compat | sse41),
        inst("pmovzxbq", fmt("A", [w(xmm1), r(xmm_m16)]), rex([0x66, 0x0F, 0x38, 0x32]).r(), _64b | compat | sse41),
        inst("pmovzxwd", fmt("A", [w(xmm1), r(xmm_m64)]), rex([0x66, 0x0F, 0x38, 0x33]).r(), _64b | compat | sse41),
        inst("pmovzxwq", fmt("A", [w(xmm1), r(xmm_m32)]), rex([0x66, 0x0F, 0x38, 0x34]).r(), _64b | compat | sse41),
        inst("pmovzxdq", fmt("A", [w(xmm1), r(xmm_m64)]), rex([0x66, 0x0F, 0x38, 0x35]).r(), _64b | compat | sse41),
    ]
}
