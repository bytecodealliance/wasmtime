use crate::dsl::{fmt, inst, r, rex, rw, sxl, sxq};
use crate::dsl::{Feature::*, Inst, LegacyPrefix::*, Location::*};

pub fn list() -> Vec<Inst> {
    vec![
        inst("andb", fmt("I", [rw(al), r(imm8)]), rex(0x24).ib(), _64b | compat),
        inst("andw", fmt("I", [rw(ax), r(imm16)]), rex(0x25).prefix(_66).iw(), _64b | compat),
        inst("andl", fmt("I", [rw(eax), r(imm32)]), rex(0x25).id(), _64b | compat),
        inst("andq", fmt("I_SXLQ", [rw(rax), sxq(imm32)]), rex(0x25).w().id(), _64b),
        inst("andb", fmt("MI", [rw(rm8), r(imm8)]), rex(0x80).digit(4).ib(), _64b | compat),
        // TODO resolve sign-extension: inst("andb", fmt("MI_SXBQ", [rw(rm8), r(imm8)]), rex(0x80).force().digit(4).ib(), _64b),
        inst("andw", fmt("MI", [rw(rm16), r(imm16)]), rex(0x81).prefix(_66).digit(4).iw(), _64b | compat),
        inst("andl", fmt("MI", [rw(rm32), r(imm32)]), rex(0x81).digit(4).id(), _64b | compat),
        inst("andq", fmt("MI_SXLQ", [rw(rm64), sxq(imm32)]), rex(0x81).w().digit(4).id(), _64b),
        // TODO resolve sign-extension: inst("andw", fmt("MI_SXBW", [rw(rm16), sxw(imm8)]), rex(0x83).force().digit(4).ib(), _64b | compat),
        inst("andl", fmt("MI_SXBL", [rw(rm32), sxl(imm8)]), rex(0x83).digit(4).ib(), _64b | compat),
        inst("andq", fmt("MI_SXBQ", [rw(rm64), sxq(imm8)]), rex(0x83).w().digit(4).ib(), _64b),
        inst("andb", fmt("MR", [rw(rm8), r(r8)]), rex(0x20).r(), _64b | compat),
        inst("andb", fmt("MR_SXBQ", [rw(rm8), r(r8)]), rex(0x20).w().r(), _64b),
        inst("andw", fmt("MR", [rw(rm16), r(r16)]), rex(0x21).prefix(_66).r(), _64b | compat),
        inst("andl", fmt("MR", [rw(rm32), r(r32)]), rex(0x21).r(), _64b | compat),
        inst("andq", fmt("MR", [rw(rm64), r(r64)]), rex(0x21).w().r(), _64b),
        inst("andb", fmt("RM", [rw(r8), r(rm8)]), rex(0x22).r(), _64b | compat),
        inst("andb", fmt("RM_SXBQ", [rw(r8), r(rm8)]), rex(0x22).w().r(), _64b),
        inst("andw", fmt("RM", [rw(r16), r(rm16)]), rex(0x23).prefix(_66).r(), _64b | compat),
        inst("andl", fmt("RM", [rw(r32), r(rm32)]), rex(0x23).r(), _64b | compat),
        inst("andq", fmt("RM", [rw(r64), r(rm64)]), rex(0x23).w().r(), _64b),
    ]
}
