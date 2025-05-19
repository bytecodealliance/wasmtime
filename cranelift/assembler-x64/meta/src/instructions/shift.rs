use crate::dsl::{Feature::*, Inst, Location::*};
use crate::dsl::{align, fmt, inst, r, rex, rw};

#[rustfmt::skip] // Keeps instructions on a single line.
pub fn list() -> Vec<Inst> {
    vec![
        inst("shldw", fmt("MRI", [rw(rm16), r(r16), r(imm8)]), rex([0x66, 0x0F, 0xA4]).ib(), _64b | compat),
        inst("shldw", fmt("MRC", [rw(rm16), r(r16), r(cl)]), rex([0x66, 0x0F, 0xA5]).ib(), _64b | compat),
        inst("shldl", fmt("MRI", [rw(rm32), r(r32), r(imm8)]), rex([0x0F, 0xA4]).ib(), _64b | compat),
        inst("shldq", fmt("MRI", [rw(rm64), r(r64), r(imm8)]), rex([0x0F, 0xA4]).ib().w(), _64b),
        inst("shldl", fmt("MRC", [rw(rm32), r(r32), r(cl)]), rex([0x0F, 0xA5]).ib(), _64b | compat),
        inst("shldq", fmt("MRC", [rw(rm64), r(r64), r(cl)]), rex([0x0F, 0xA5]).ib().w(), _64b),
        // Vector instructions (shift left).
        inst("psllw", fmt("A", [rw(xmm), r(align(xmm_m128))]), rex([0x66, 0x0F, 0xF1]).r(), _64b | compat | sse2),
        inst("psllw", fmt("B", [rw(xmm), r(imm8)]), rex([0x66, 0x0F, 0x71]).digit(6).ib(), _64b | compat | sse2),
        inst("pslld", fmt("A", [rw(xmm), r(align(xmm_m128))]), rex([0x66, 0x0F, 0xF2]).r(), _64b | compat | sse2),
        inst("pslld", fmt("B", [rw(xmm), r(imm8)]), rex([0x66, 0x0F, 0x72]).digit(6).ib(), _64b | compat | sse2),
        inst("psllq", fmt("A", [rw(xmm), r(align(xmm_m128))]), rex([0x66, 0x0F, 0xF3]).r(), _64b | compat | sse2),
        inst("psllq", fmt("B", [rw(xmm), r(imm8)]), rex([0x66, 0x0F, 0x73]).digit(6).ib(), _64b | compat | sse2),
        // Vector instructions (shift right).
        inst("psraw", fmt("A", [rw(xmm), r(align(xmm_m128))]), rex([0x66, 0x0F, 0xE1]).r(), _64b | compat | sse2),
        inst("psraw", fmt("B", [rw(xmm), r(imm8)]), rex([0x66, 0x0F, 0x71]).digit(4).ib(), _64b | compat | sse2),
        inst("psrad", fmt("A", [rw(xmm), r(align(xmm_m128))]), rex([0x66, 0x0F, 0xE2]).r(), _64b | compat | sse2),
        inst("psrad", fmt("B", [rw(xmm), r(imm8)]), rex([0x66, 0x0F, 0x72]).digit(4).ib(), _64b | compat | sse2),
        inst("psrlw", fmt("A", [rw(xmm), r(align(xmm_m128))]), rex([0x66, 0x0F, 0xD1]).r(), _64b | compat | sse2),
        inst("psrlw", fmt("B", [rw(xmm), r(imm8)]), rex([0x66, 0x0F, 0x71]).digit(2).ib(), _64b | compat | sse2),
        inst("psrld", fmt("A", [rw(xmm), r(align(xmm_m128))]), rex([0x66, 0x0F, 0xD2]).r(), _64b | compat | sse2),
        inst("psrld", fmt("B", [rw(xmm), r(imm8)]), rex([0x66, 0x0F, 0x72]).digit(2).ib(), _64b | compat | sse2),
        inst("psrlq", fmt("A", [rw(xmm), r(align(xmm_m128))]), rex([0x66, 0x0F, 0xD3]).r(), _64b | compat | sse2),
        inst("psrlq", fmt("B", [rw(xmm), r(imm8)]), rex([0x66, 0x0F, 0x73]).digit(2).ib(), _64b | compat | sse2),
    ]
}
