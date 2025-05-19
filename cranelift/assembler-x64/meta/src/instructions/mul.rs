use crate::dsl::{Feature::*, Inst, Location::*};
use crate::dsl::{align, fmt, implicit, inst, r, rex, rw, sxl, sxq, sxw, w};

#[rustfmt::skip] // Keeps instructions on a single line.
pub fn list() -> Vec<Inst> {
    vec![
        // Multiply unsigned; low bits in `rax`, high bits in `rdx`.
        inst("mulb", fmt("M", [rw(implicit(ax)), r(rm8)]), rex(0xF6).digit(4), _64b | compat),
        inst("mulw", fmt("M", [rw(implicit(ax)), w(implicit(dx)), r(rm16)]), rex([0x66, 0xF7]).digit(4), _64b | compat),
        inst("mull", fmt("M", [rw(implicit(eax)), w(implicit(edx)), r(rm32)]), rex(0xF7).digit(4), _64b | compat),
        inst("mulq", fmt("M", [rw(implicit(rax)), w(implicit(rdx)), r(rm64)]), rex(0xF7).w().digit(4), _64b),
        // Multiply signed; low bits in `rax`, high bits in `rdx`.
        inst("imulb", fmt("M", [rw(implicit(ax)), r(rm8)]), rex(0xF6).digit(5), _64b | compat),
        inst("imulw", fmt("M", [rw(implicit(ax)), w(implicit(dx)), r(rm16)]), rex([0x66, 0xF7]).digit(5), _64b | compat),
        inst("imull", fmt("M", [rw(implicit(eax)), w(implicit(edx)), r(rm32)]), rex(0xF7).digit(5), _64b | compat),
        inst("imulq", fmt("M", [rw(implicit(rax)), w(implicit(rdx)), r(rm64)]), rex(0xF7).w().digit(5), _64b),
        inst("imulw", fmt("RM", [rw(r16), r(rm16)]), rex([0x66, 0x0F, 0xAF]), _64b | compat),
        inst("imull", fmt("RM", [rw(r32), r(rm32)]), rex([0x0F, 0xAF]), _64b | compat),
        inst("imulq", fmt("RM", [rw(r64), r(rm64)]), rex([0x0F, 0xAF]).w(), _64b),
        inst("imulw", fmt("RMI_SXB", [w(r16), r(rm16), sxw(imm8)]), rex([0x66, 0x6B]).ib(), _64b | compat),
        inst("imull", fmt("RMI_SXB", [w(r32), r(rm32), sxl(imm8)]), rex(0x6B).ib(), _64b | compat),
        inst("imulq", fmt("RMI_SXB", [w(r64), r(rm64), sxq(imm8)]), rex(0x6B).w().ib(), _64b),
        inst("imulw", fmt("RMI", [w(r16), r(rm16), r(imm16)]), rex([0x66, 0x69]).iw(), _64b | compat),
        inst("imull", fmt("RMI", [w(r32), r(rm32), r(imm32)]), rex(0x69).id(), _64b | compat),
        inst("imulq", fmt("RMI_SXL", [w(r64), r(rm64), sxq(imm32)]), rex(0x69).w().id(), _64b),
        // Vector instructions.
        inst("mulss", fmt("A", [rw(xmm), r(xmm_m32)]), rex([0xF3, 0x0F, 0x59]).r(), _64b | compat | sse),
        inst("mulsd", fmt("A", [rw(xmm), r(xmm_m64)]), rex([0xF2, 0x0F, 0x59]).r(), _64b | compat | sse2),
        inst("mulps", fmt("A", [rw(xmm), r(align(xmm_m128))]), rex([0x0F, 0x59]).r(), _64b | compat | sse),
        inst("mulpd", fmt("A", [rw(xmm), r(align(xmm_m128))]), rex([0x66, 0x0F, 0x59]).r(), _64b | compat | sse2),
        inst("pmuldq", fmt("A", [rw(xmm), r(align(xmm_m128))]), rex([0x66, 0x0F, 0x38, 0x28]).r(), _64b | compat | sse41),
        inst("pmulhrsw", fmt("A", [rw(xmm), r(align(xmm_m128))]), rex([0x66, 0x0F, 0x38, 0x0B]).r(), _64b | compat | ssse3),
        inst("pmulhuw", fmt("A", [rw(xmm), r(align(xmm_m128))]), rex([0x66, 0x0F, 0xE4]).r(), _64b | compat | sse2),
        inst("pmulhw", fmt("A", [rw(xmm), r(align(xmm_m128))]), rex([0x66, 0x0F, 0xE5]).r(), _64b | compat | sse2),
        inst("pmulld", fmt("A", [rw(xmm), r(align(xmm_m128))]), rex([0x66, 0x0F, 0x38, 0x40]).r(), _64b | compat | sse41),
        inst("pmullw", fmt("A", [rw(xmm), r(align(xmm_m128))]), rex([0x66, 0x0F, 0xD5]).r(), _64b | compat | sse2),
        inst("pmuludq", fmt("A", [rw(xmm), r(align(xmm_m128))]), rex([0x66, 0x0F, 0xF4]).r(), _64b | compat | sse2),
    ]
}
