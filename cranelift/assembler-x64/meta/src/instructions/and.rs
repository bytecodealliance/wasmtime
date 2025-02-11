use crate::dsl::{fmt, inst, r, rex, rw, sxl, sxq};
use crate::dsl::{Feature::*, Inst, LegacyPrefix::*, Location::*};

pub fn list() -> Vec<Inst> {
    // Note that some versions of the reference manual show `REX + <opcode>`
    // rows that (a) are only intended for documentation purposes, i.e., to note
    // that `r/m8` cannot be encoded to access byte registers AH, BH, CH, DH if
    // a REX prefix is used, and (b) have known errors indicating
    // "sign-extended" when in fact this is not the case. We skip those rows
    // here and indicate the true sign extension operations with a `_SX<from
    // width>` suffix.
    vec![
        inst("andb", fmt("I", [rw(al), r(imm8)]), rex(0x24).ib(), _64b | compat),
        inst("andw", fmt("I", [rw(ax), r(imm16)]), rex(0x25).prefix(_66).iw(), _64b | compat),
        inst("andl", fmt("I", [rw(eax), r(imm32)]), rex(0x25).id(), _64b | compat),
        inst("andq", fmt("I_SXL", [rw(rax), sxq(imm32)]), rex(0x25).w().id(), _64b),
        inst("andb", fmt("MI", [rw(rm8), r(imm8)]), rex(0x80).digit(4).ib(), _64b | compat),
        inst("andw", fmt("MI", [rw(rm16), r(imm16)]), rex(0x81).prefix(_66).digit(4).iw(), _64b | compat),
        inst("andl", fmt("MI", [rw(rm32), r(imm32)]), rex(0x81).digit(4).id(), _64b | compat),
        inst("andq", fmt("MI_SXL", [rw(rm64), sxq(imm32)]), rex(0x81).w().digit(4).id(), _64b),
        inst("andl", fmt("MI_SXB", [rw(rm32), sxl(imm8)]), rex(0x83).digit(4).ib(), _64b | compat),
        inst("andq", fmt("MI_SXB", [rw(rm64), sxq(imm8)]), rex(0x83).w().digit(4).ib(), _64b),
        inst("andb", fmt("MR", [rw(rm8), r(r8)]), rex(0x20).r(), _64b | compat),
        inst("andw", fmt("MR", [rw(rm16), r(r16)]), rex(0x21).prefix(_66).r(), _64b | compat),
        inst("andl", fmt("MR", [rw(rm32), r(r32)]), rex(0x21).r(), _64b | compat),
        inst("andq", fmt("MR", [rw(rm64), r(r64)]), rex(0x21).w().r(), _64b),
        inst("andb", fmt("RM", [rw(r8), r(rm8)]), rex(0x22).r(), _64b | compat),
        inst("andw", fmt("RM", [rw(r16), r(rm16)]), rex(0x23).prefix(_66).r(), _64b | compat),
        inst("andl", fmt("RM", [rw(r32), r(rm32)]), rex(0x23).r(), _64b | compat),
        inst("andq", fmt("RM", [rw(r64), r(rm64)]), rex(0x23).w().r(), _64b),
    ]
}
