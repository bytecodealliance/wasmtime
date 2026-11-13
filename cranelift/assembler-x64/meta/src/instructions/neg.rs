use crate::dsl::{Feature::*, Inst, Length::*, Location::*, TupleType::*};
use crate::dsl::{evex, fmt, inst, r, rex, rw, w};

#[rustfmt::skip] // Keeps instructions on a single line.
pub fn list() -> Vec<Inst> {
    vec![
        // Two's complement negation.
        inst("negb", fmt("M", [rw(rm8)]), rex(0xF6).digit(3), _64b | compat),
        inst("negw", fmt("M", [rw(rm16)]), rex([0x66, 0xF7]).digit(3), _64b | compat),
        inst("negl", fmt("M", [rw(rm32)]), rex(0xF7).digit(3), _64b | compat),
        inst("negq", fmt("M", [rw(rm64)]), rex(0xF7).w().digit(3), _64b),
        // APX: NDD (new data destination) form of `NEG`, promoted into EVEX
        // "map 4" via the extended-EVEX prefix. With `ND = 1` the architectural
        // destination is the `vvvv`-encoded register, so the written operand is
        // placed in the `V` slot while the single source stays in ModRM.rm; the
        // ModRM.reg field continues to hold the `/3` opcode extension.
        inst("negq", fmt("VM", [w(r64a), r(rm64)]), evex(L128, Full).map4().w1().nd().op(0xF7).digit(3), _64b & apx),
        inst("negl", fmt("VM", [w(r32a), r(rm32)]), evex(L128, Full).map4().w0().nd().op(0xF7).digit(3), _64b & apx),
        // One's complement negation.
        inst("notb", fmt("M", [rw(rm8)]), rex(0xF6).digit(2), _64b | compat),
        inst("notw", fmt("M", [rw(rm16)]), rex([0x66, 0xF7]).digit(2), _64b | compat),
        inst("notl", fmt("M", [rw(rm32)]), rex(0xF7).digit(2), _64b | compat),
        inst("notq", fmt("M", [rw(rm64)]), rex(0xF7).w().digit(2), _64b),
        // APX: NDD form of `NOT`, promoted into EVEX "map 4"; see `negq` above.
        inst("notq", fmt("VM", [w(r64a), r(rm64)]), evex(L128, Full).map4().w1().nd().op(0xF7).digit(2), _64b & apx),
        inst("notl", fmt("VM", [w(r32a), r(rm32)]), evex(L128, Full).map4().w0().nd().op(0xF7).digit(2), _64b & apx),
    ]
}
