use crate::dsl::{Feature::*, Inst, Location::*};
use crate::dsl::{fmt, inst, r, rex, rw, w};

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

        inst("movd", fmt("A", [w(xmm1), r(rm32)]), rex([0x66, 0x0F, 0x6E]).r(), _64b | compat | sse2),
        inst("movq", fmt("A", [w(xmm1), r(rm64)]), rex([0x66, 0x0F, 0x6E]).r().w(), _64b | sse2),
        inst("movd", fmt("B", [w(rm32), r(xmm2)]), rex([0x66, 0x0F, 0x7E]).r(), _64b | compat | sse2),
        inst("movq", fmt("B", [w(rm64), r(xmm2)]), rex([0x66, 0x0F, 0x7E]).r().w(), _64b | sse2),

        inst("movmskps", fmt("RM", [w(r32), r(xmm2)]), rex([0x0F, 0x50]).r(), _64b | compat | sse),
        inst("movmskpd", fmt("RM", [w(r32), r(xmm2)]), rex([0x66, 0x0F, 0x50]).r(), _64b | compat | sse2),
        inst("pmovmskb", fmt("RM", [w(r32), r(xmm2)]), rex([0x66, 0x0F, 0xD7]).r(), _64b | compat | sse2),
    ]
}
