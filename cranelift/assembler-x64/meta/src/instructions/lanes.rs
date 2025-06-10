use crate::dsl::{Feature::*, Inst, Location::*, VexLength::*};
use crate::dsl::{align, fmt, inst, r, rex, rw, vex, w};

#[rustfmt::skip] // Keeps instructions on a single line.
pub fn list() -> Vec<Inst> {
    // Note that `p{extr,ins}r{w,b}` below operate on 32-bit registers but a
    // smaller-width memory location. This means that disassembly in Capstone
    // doesn't match `rm8`, for example. For now pretend both of these are
    // `rm32` to get diassembly matching Capstone.
    let r32m8 = rm32;
    let r32m16 = rm32;

    vec![
        inst("pextrb", fmt("A", [w(r32m8), r(xmm2), r(imm8)]), rex([0x66, 0x0F, 0x3A, 0x14]).r().ib(), _64b | compat | sse41),
        inst("pextrw", fmt("A", [w(r32), r(xmm2), r(imm8)]), rex([0x66, 0x0F, 0xC5]).r().ib(), _64b | compat | sse2),
        inst("pextrw", fmt("B", [w(r32m16), r(xmm2), r(imm8)]), rex([0x66, 0x0F, 0x3A, 0x15]).r().ib(), _64b | compat | sse41),
        inst("pextrd", fmt("A", [w(rm32), r(xmm2), r(imm8)]), rex([0x66, 0x0F, 0x3A, 0x16]).r().ib(), _64b | compat | sse41),
        inst("pextrq", fmt("A", [w(rm64), r(xmm2), r(imm8)]), rex([0x66, 0x0F, 0x3A, 0x16]).w().r().ib(), _64b | sse41),

        inst("pinsrb", fmt("A", [rw(xmm1), r(r32m8), r(imm8)]), rex([0x66, 0x0F, 0x3A, 0x20]).r().ib(), _64b | compat | sse41),
        inst("pinsrw", fmt("A", [rw(xmm1), r(r32m16), r(imm8)]), rex([0x66, 0x0F, 0xC4]).r().ib(), _64b | compat | sse2),
        inst("pinsrd", fmt("A", [rw(xmm1), r(rm32), r(imm8)]), rex([0x66, 0x0F, 0x3A, 0x22]).r().ib(), _64b | compat | sse41),
        inst("pinsrq", fmt("A", [rw(xmm1), r(rm64), r(imm8)]), rex([0x66, 0x0F, 0x3A, 0x22]).r().ib().w(), _64b | sse41),

        inst("movmskps", fmt("RM", [w(r32), r(xmm2)]), rex([0x0F, 0x50]).r(), _64b | compat | sse),
        inst("movmskpd", fmt("RM", [w(r32), r(xmm2)]), rex([0x66, 0x0F, 0x50]).r(), _64b | compat | sse2),
        inst("pmovmskb", fmt("RM", [w(r32), r(xmm2)]), rex([0x66, 0x0F, 0xD7]).r(), _64b | compat | sse2),
        inst("vmovmskps", fmt("RM", [w(r32), r(xmm2)]), vex(L128)._0f().op(0x50).r(), _64b | compat | avx),
        inst("vmovmskpd", fmt("RM", [w(r32), r(xmm2)]), vex(L128)._66()._0f().op(0x50).r(), _64b | compat | avx),
        inst("vpmovmskb", fmt("RM", [w(r32), r(xmm2)]), vex(L128)._66()._0f().op(0xD7).r(), _64b | compat | avx),

        inst("vpinsrb", fmt("B", [w(xmm1), r(xmm2), r(r32m8), r(imm8)]), vex(L128)._66()._0f3a().w0().op(0x20).r().ib(), _64b | compat | avx),
        inst("vpinsrw", fmt("B", [w(xmm1), r(xmm2), r(r32m16), r(imm8)]), vex(L128)._66()._0f().w0().op(0xC4).r().ib(), _64b | compat | avx),
        inst("vpinsrd", fmt("B", [w(xmm1), r(xmm2), r(rm32), r(imm8)]), vex(L128)._66()._0f3a().w0().op(0x22).r().ib(), _64b | compat | avx),
        inst("vpinsrq", fmt("B", [w(xmm1), r(xmm2), r(rm64), r(imm8)]), vex(L128)._66()._0f3a().w1().op(0x22).r().ib(), _64b | avx),

        inst("movddup", fmt("A", [w(xmm1), r(xmm_m64)]), rex([0xF2, 0x0F, 0x12]).r(), _64b | compat | sse3),
        inst("vmovddup", fmt("A", [w(xmm1), r(xmm_m64)]), vex(L128)._f2()._0f().op(0x12).r(), _64b | compat | avx),

        inst("pblendvb", fmt("RM", [rw(xmm1), r(align(xmm_m128)), r(xmm0)]), rex([0x66, 0x0F, 0x38, 0x10]).r(), _64b | compat | sse41),
        inst("blendvps", fmt("RM0", [rw(xmm1), r(align(xmm_m128)), r(xmm0)]), rex([0x66, 0x0F, 0x38, 0x14]).r(), _64b | compat | sse41),
        inst("blendvpd", fmt("RM0", [rw(xmm1), r(align(xmm_m128)), r(xmm0)]), rex([0x66, 0x0F, 0x38, 0x15]).r(), _64b | compat | sse41),
        inst("vpblendvb", fmt("RVMR", [w(xmm1), r(xmm2), r(xmm_m128), r(xmm3)]), vex(L128)._66()._0f3a().w0().op(0x4C).r().is4(), _64b | compat | avx),
        inst("vblendvps", fmt("RVMR", [w(xmm1), r(xmm2), r(xmm_m128), r(xmm3)]), vex(L128)._66()._0f3a().w0().op(0x4A).r().is4(), _64b | compat | avx),
        inst("vblendvpd", fmt("RVMR", [w(xmm1), r(xmm2), r(xmm_m128), r(xmm3)]), vex(L128)._66()._0f3a().w0().op(0x4B).r().is4(), _64b | compat | avx),

        inst("pshufd", fmt("A", [w(xmm1), r(align(xmm_m128)), r(imm8)]), rex([0x66, 0x0F, 0x70]).r().ib(), _64b | compat | sse2),
        inst("pshuflw", fmt("A", [w(xmm1), r(align(xmm_m128)), r(imm8)]), rex([0xF2, 0x0F, 0x70]).r().ib(), _64b | compat | sse2),
        inst("pshufhw", fmt("A", [w(xmm1), r(align(xmm_m128)), r(imm8)]), rex([0xF3, 0x0F, 0x70]).r().ib(), _64b | compat | sse2),
        inst("vpshufd", fmt("A", [w(xmm1), r(xmm_m128), r(imm8)]), vex(L128)._66()._0f().op(0x70).r().ib(), _64b | compat | avx),
        inst("vpshuflw", fmt("A", [w(xmm1), r(xmm_m128), r(imm8)]), vex(L128)._f2()._0f().op(0x70).r().ib(), _64b | compat | avx),
        inst("vpshufhw", fmt("A", [w(xmm1), r(xmm_m128), r(imm8)]), vex(L128)._f3()._0f().op(0x70).r().ib(), _64b | compat | avx),
    ]
}
