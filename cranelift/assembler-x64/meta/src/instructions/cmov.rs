use crate::dsl::{Eflags::*, Feature::*, Inst, Length::*, Location::*, TupleType::*};
use crate::dsl::{evex, fmt, inst, r, rex, rw, w};

#[rustfmt::skip] // Keeps instructions on a single line.
pub fn list() -> Vec<Inst> {
    vec![
        // Note that the Intel manual lists many mnemonics for this family of
        // instructions which are duplicates of other mnemonics. The order here
        // matches the order in the manual and comments are left when variants
        // are omitted due to the instructions being duplicates of another.
        inst("cmovaw", fmt("RM", [rw(r16), r(rm16)]).flags(R), rex([0x66, 0x0f, 0x47]).r(), _64b | compat),
        inst("cmoval", fmt("RM", [rw(r32), r(rm32)]).flags(R), rex([0x0f, 0x47]).r(), _64b | compat),
        inst("cmovaq", fmt("RM", [rw(r64), r(rm64)]).flags(R), rex([0x0f, 0x47]).w().r(), _64b),
        inst("cmovaew", fmt("RM", [rw(r16), r(rm16)]).flags(R), rex([0x66, 0x0f, 0x43]).r(), _64b | compat),
        inst("cmovael", fmt("RM", [rw(r32), r(rm32)]).flags(R), rex([0x0f, 0x43]).r(), _64b | compat),
        inst("cmovaeq", fmt("RM", [rw(r64), r(rm64)]).flags(R), rex([0x0f, 0x43]).w().r(), _64b),
        inst("cmovbw", fmt("RM", [rw(r16), r(rm16)]).flags(R), rex([0x66, 0x0f, 0x42]).r(), _64b | compat),
        inst("cmovbl", fmt("RM", [rw(r32), r(rm32)]).flags(R), rex([0x0f, 0x42]).r(), _64b | compat),
        inst("cmovbq", fmt("RM", [rw(r64), r(rm64)]).flags(R), rex([0x0f, 0x42]).w().r(), _64b),
        inst("cmovbew", fmt("RM", [rw(r16), r(rm16)]).flags(R), rex([0x66, 0x0f, 0x46]).r(), _64b | compat),
        inst("cmovbel", fmt("RM", [rw(r32), r(rm32)]).flags(R), rex([0x0f, 0x46]).r(), _64b | compat),
        inst("cmovbeq", fmt("RM", [rw(r64), r(rm64)]).flags(R), rex([0x0f, 0x46]).w().r(), _64b),
        // NB: cmovc* is omitted here as it has the same encoding as cmovb*
        inst("cmovew", fmt("RM", [rw(r16), r(rm16)]).flags(R), rex([0x66, 0x0f, 0x44]).r(), _64b | compat),
        inst("cmovel", fmt("RM", [rw(r32), r(rm32)]).flags(R), rex([0x0f, 0x44]).r(), _64b | compat),
        inst("cmoveq", fmt("RM", [rw(r64), r(rm64)]).flags(R), rex([0x0f, 0x44]).w().r(), _64b),
        inst("cmovgw", fmt("RM", [rw(r16), r(rm16)]).flags(R), rex([0x66, 0x0f, 0x4f]).r(), _64b | compat),
        inst("cmovgl", fmt("RM", [rw(r32), r(rm32)]).flags(R), rex([0x0f, 0x4f]).r(), _64b | compat),
        inst("cmovgq", fmt("RM", [rw(r64), r(rm64)]).flags(R), rex([0x0f, 0x4f]).w().r(), _64b),
        inst("cmovgew", fmt("RM", [rw(r16), r(rm16)]).flags(R), rex([0x66, 0x0f, 0x4d]).r(), _64b | compat),
        inst("cmovgel", fmt("RM", [rw(r32), r(rm32)]).flags(R), rex([0x0f, 0x4d]).r(), _64b | compat),
        inst("cmovgeq", fmt("RM", [rw(r64), r(rm64)]).flags(R), rex([0x0f, 0x4d]).w().r(), _64b),
        inst("cmovlw", fmt("RM", [rw(r16), r(rm16)]).flags(R), rex([0x66, 0x0f, 0x4c]).r(), _64b | compat),
        inst("cmovll", fmt("RM", [rw(r32), r(rm32)]).flags(R), rex([0x0f, 0x4c]).r(), _64b | compat),
        inst("cmovlq", fmt("RM", [rw(r64), r(rm64)]).flags(R), rex([0x0f, 0x4c]).w().r(), _64b),
        inst("cmovlew", fmt("RM", [rw(r16), r(rm16)]).flags(R), rex([0x66, 0x0f, 0x4e]).r(), _64b | compat),
        inst("cmovlel", fmt("RM", [rw(r32), r(rm32)]).flags(R), rex([0x0f, 0x4e]).r(), _64b | compat),
        inst("cmovleq", fmt("RM", [rw(r64), r(rm64)]).flags(R), rex([0x0f, 0x4e]).w().r(), _64b),
        // NB: cmovna* is omitted here as it has the same encoding as cmovbe*
        // NB: cmovnb* is omitted here as it has the same encoding as cmovae*
        // NB: cmovnbe* is omitted here as it has the same encoding as cmova*
        // NB: cmovnc* is omitted here as it has the same encoding as cmovae*
        inst("cmovnew", fmt("RM", [rw(r16), r(rm16)]).flags(R), rex([0x66, 0x0f, 0x45]).r(), _64b | compat),
        inst("cmovnel", fmt("RM", [rw(r32), r(rm32)]).flags(R), rex([0x0f, 0x45]).r(), _64b | compat),
        inst("cmovneq", fmt("RM", [rw(r64), r(rm64)]).flags(R), rex([0x0f, 0x45]).w().r(), _64b),
        // NB: cmovng* is omitted here as it has the same encoding as cmovle*
        // NB: cmovnge* is omitted here as it has the same encoding as cmovl*
        // NB: cmovnl* is omitted here as it has the same encoding as cmovge*
        // NB: cmovnle* is omitted here as it has the same encoding as cmovg*
        inst("cmovnow", fmt("RM", [rw(r16), r(rm16)]).flags(R), rex([0x66, 0x0f, 0x41]).r(), _64b | compat),
        inst("cmovnol", fmt("RM", [rw(r32), r(rm32)]).flags(R), rex([0x0f, 0x41]).r(), _64b | compat),
        inst("cmovnoq", fmt("RM", [rw(r64), r(rm64)]).flags(R), rex([0x0f, 0x41]).w().r(), _64b),
        inst("cmovnpw", fmt("RM", [rw(r16), r(rm16)]).flags(R), rex([0x66, 0x0f, 0x4b]).r(), _64b | compat),
        inst("cmovnpl", fmt("RM", [rw(r32), r(rm32)]).flags(R), rex([0x0f, 0x4b]).r(), _64b | compat),
        inst("cmovnpq", fmt("RM", [rw(r64), r(rm64)]).flags(R), rex([0x0f, 0x4b]).w().r(), _64b),
        inst("cmovnsw", fmt("RM", [rw(r16), r(rm16)]).flags(R), rex([0x66, 0x0f, 0x49]).r(), _64b | compat),
        inst("cmovnsl", fmt("RM", [rw(r32), r(rm32)]).flags(R), rex([0x0f, 0x49]).r(), _64b | compat),
        inst("cmovnsq", fmt("RM", [rw(r64), r(rm64)]).flags(R), rex([0x0f, 0x49]).w().r(), _64b),
        // NB: cmovnz* is omitted here as it has the same encoding as cmovne*
        inst("cmovow", fmt("RM", [rw(r16), r(rm16)]).flags(R), rex([0x66, 0x0f, 0x40]).r(), _64b | compat),
        inst("cmovol", fmt("RM", [rw(r32), r(rm32)]).flags(R), rex([0x0f, 0x40]).r(), _64b | compat),
        inst("cmovoq", fmt("RM", [rw(r64), r(rm64)]).flags(R), rex([0x0f, 0x40]).w().r(), _64b),
        inst("cmovpw", fmt("RM", [rw(r16), r(rm16)]).flags(R), rex([0x66, 0x0f, 0x4a]).r(), _64b | compat),
        inst("cmovpl", fmt("RM", [rw(r32), r(rm32)]).flags(R), rex([0x0f, 0x4a]).r(), _64b | compat),
        inst("cmovpq", fmt("RM", [rw(r64), r(rm64)]).flags(R), rex([0x0f, 0x4a]).w().r(), _64b),
        // NB: cmovpe* is omitted here as it has the same encoding as cmovp*
        // NB: cmovpo* is omitted here as it has the same encoding as cmovnp*
        inst("cmovsw", fmt("RM", [rw(r16), r(rm16)]).flags(R), rex([0x66, 0x0f, 0x48]).r(), _64b | compat),
        inst("cmovsl", fmt("RM", [rw(r32), r(rm32)]).flags(R), rex([0x0f, 0x48]).r(), _64b | compat),
        inst("cmovsq", fmt("RM", [rw(r64), r(rm64)]).flags(R), rex([0x0f, 0x48]).w().r(), _64b),
        // NB: cmovz* is omitted here as it has the same encoding as cmove*

        // APX: "new data destination" forms. The legacy conditional move reads
        // and writes ModRM.reg, so a select whose "else" value is still needed
        // afterwards has to copy it into the destination first; these name a
        // separate destination instead.
        //
        // As with `imul`, the legacy opcode is an RM form, so the NDD operand
        // order is `ndd, reg, rm` and the format is MRV. These are the map 1
        // opcodes promoted into map 4. Only the widths the lowering uses are
        // defined; narrower selects already go through the 32-bit rules.
        inst("cmoval",  fmt("MRV", [r(rm32), r(r32b), w(r32a)]).flags(R), evex(L128, Full).map4().w0().nd().op(0x47).r(), _64b & apx),
        inst("cmovaq",  fmt("MRV", [r(rm64), r(r64b), w(r64a)]).flags(R), evex(L128, Full).map4().w1().nd().op(0x47).r(), _64b & apx),
        inst("cmovael", fmt("MRV", [r(rm32), r(r32b), w(r32a)]).flags(R), evex(L128, Full).map4().w0().nd().op(0x43).r(), _64b & apx),
        inst("cmovaeq", fmt("MRV", [r(rm64), r(r64b), w(r64a)]).flags(R), evex(L128, Full).map4().w1().nd().op(0x43).r(), _64b & apx),
        inst("cmovbl",  fmt("MRV", [r(rm32), r(r32b), w(r32a)]).flags(R), evex(L128, Full).map4().w0().nd().op(0x42).r(), _64b & apx),
        inst("cmovbq",  fmt("MRV", [r(rm64), r(r64b), w(r64a)]).flags(R), evex(L128, Full).map4().w1().nd().op(0x42).r(), _64b & apx),
        inst("cmovbel", fmt("MRV", [r(rm32), r(r32b), w(r32a)]).flags(R), evex(L128, Full).map4().w0().nd().op(0x46).r(), _64b & apx),
        inst("cmovbeq", fmt("MRV", [r(rm64), r(r64b), w(r64a)]).flags(R), evex(L128, Full).map4().w1().nd().op(0x46).r(), _64b & apx),
        inst("cmovel",  fmt("MRV", [r(rm32), r(r32b), w(r32a)]).flags(R), evex(L128, Full).map4().w0().nd().op(0x44).r(), _64b & apx),
        inst("cmoveq",  fmt("MRV", [r(rm64), r(r64b), w(r64a)]).flags(R), evex(L128, Full).map4().w1().nd().op(0x44).r(), _64b & apx),
        inst("cmovgl",  fmt("MRV", [r(rm32), r(r32b), w(r32a)]).flags(R), evex(L128, Full).map4().w0().nd().op(0x4f).r(), _64b & apx),
        inst("cmovgq",  fmt("MRV", [r(rm64), r(r64b), w(r64a)]).flags(R), evex(L128, Full).map4().w1().nd().op(0x4f).r(), _64b & apx),
        inst("cmovgel", fmt("MRV", [r(rm32), r(r32b), w(r32a)]).flags(R), evex(L128, Full).map4().w0().nd().op(0x4d).r(), _64b & apx),
        inst("cmovgeq", fmt("MRV", [r(rm64), r(r64b), w(r64a)]).flags(R), evex(L128, Full).map4().w1().nd().op(0x4d).r(), _64b & apx),
        inst("cmovll",  fmt("MRV", [r(rm32), r(r32b), w(r32a)]).flags(R), evex(L128, Full).map4().w0().nd().op(0x4c).r(), _64b & apx),
        inst("cmovlq",  fmt("MRV", [r(rm64), r(r64b), w(r64a)]).flags(R), evex(L128, Full).map4().w1().nd().op(0x4c).r(), _64b & apx),
        inst("cmovlel", fmt("MRV", [r(rm32), r(r32b), w(r32a)]).flags(R), evex(L128, Full).map4().w0().nd().op(0x4e).r(), _64b & apx),
        inst("cmovleq", fmt("MRV", [r(rm64), r(r64b), w(r64a)]).flags(R), evex(L128, Full).map4().w1().nd().op(0x4e).r(), _64b & apx),
        inst("cmovnel", fmt("MRV", [r(rm32), r(r32b), w(r32a)]).flags(R), evex(L128, Full).map4().w0().nd().op(0x45).r(), _64b & apx),
        inst("cmovneq", fmt("MRV", [r(rm64), r(r64b), w(r64a)]).flags(R), evex(L128, Full).map4().w1().nd().op(0x45).r(), _64b & apx),
        inst("cmovnol", fmt("MRV", [r(rm32), r(r32b), w(r32a)]).flags(R), evex(L128, Full).map4().w0().nd().op(0x41).r(), _64b & apx),
        inst("cmovnoq", fmt("MRV", [r(rm64), r(r64b), w(r64a)]).flags(R), evex(L128, Full).map4().w1().nd().op(0x41).r(), _64b & apx),
        inst("cmovnpl", fmt("MRV", [r(rm32), r(r32b), w(r32a)]).flags(R), evex(L128, Full).map4().w0().nd().op(0x4b).r(), _64b & apx),
        inst("cmovnpq", fmt("MRV", [r(rm64), r(r64b), w(r64a)]).flags(R), evex(L128, Full).map4().w1().nd().op(0x4b).r(), _64b & apx),
        inst("cmovnsl", fmt("MRV", [r(rm32), r(r32b), w(r32a)]).flags(R), evex(L128, Full).map4().w0().nd().op(0x49).r(), _64b & apx),
        inst("cmovnsq", fmt("MRV", [r(rm64), r(r64b), w(r64a)]).flags(R), evex(L128, Full).map4().w1().nd().op(0x49).r(), _64b & apx),
        inst("cmovol",  fmt("MRV", [r(rm32), r(r32b), w(r32a)]).flags(R), evex(L128, Full).map4().w0().nd().op(0x40).r(), _64b & apx),
        inst("cmovoq",  fmt("MRV", [r(rm64), r(r64b), w(r64a)]).flags(R), evex(L128, Full).map4().w1().nd().op(0x40).r(), _64b & apx),
        inst("cmovpl",  fmt("MRV", [r(rm32), r(r32b), w(r32a)]).flags(R), evex(L128, Full).map4().w0().nd().op(0x4a).r(), _64b & apx),
        inst("cmovpq",  fmt("MRV", [r(rm64), r(r64b), w(r64a)]).flags(R), evex(L128, Full).map4().w1().nd().op(0x4a).r(), _64b & apx),
        inst("cmovsl",  fmt("MRV", [r(rm32), r(r32b), w(r32a)]).flags(R), evex(L128, Full).map4().w0().nd().op(0x48).r(), _64b & apx),
        inst("cmovsq",  fmt("MRV", [r(rm64), r(r64b), w(r64a)]).flags(R), evex(L128, Full).map4().w1().nd().op(0x48).r(), _64b & apx),
    ]
}
