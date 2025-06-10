use crate::dsl::{Customization::*, Feature::*, Inst, Location::*};
use crate::dsl::{fmt, inst, rex, rw};

#[rustfmt::skip] // Keeps instructions on a single line.
pub fn list() -> Vec<Inst> {
    vec![
        // Note that for xchg the "MR" variants are omitted from the Intel
        // manual as they have the exact same encoding as the "RM" variant.
        // Additionally the "O" variants are omitted as they're just exchanging
        // registers which isn't needed by Cranelift at this time.
        //
        // Also note that these have a custom display implementation to swap the
        // order of the operands to match what Capstone prints.
        inst("xchgb", fmt("RM", [rw(r8), rw(m8)]), rex(0x86).r(), _64b | compat).custom(Display),
        inst("xchgw", fmt("RM", [rw(r16), rw(m16)]), rex([0x66, 0x87]).r(), _64b | compat).custom(Display),
        inst("xchgl", fmt("RM", [rw(r32), rw(m32)]), rex(0x87).r(), _64b | compat).custom(Display),
        inst("xchgq", fmt("RM", [rw(r64), rw(m64)]), rex(0x87).w().r(), _64b).custom(Display),
    ]
}
