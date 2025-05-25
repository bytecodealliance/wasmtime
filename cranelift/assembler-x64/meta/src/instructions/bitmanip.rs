use crate::dsl::{Feature::*, Inst, Location::*};
use crate::dsl::{fmt, implicit, inst, r, rex, rw, w};

#[rustfmt::skip] // Keeps instructions on a single line.
pub fn list() -> Vec<Inst> {
    vec![
        inst("bsfw", fmt("RM", [w(r16), r(rm16)]), rex([0x66, 0x0F, 0xBC]).r(), _64b | compat),
        inst("bsfl", fmt("RM", [w(r32), r(rm32)]), rex([0x0F, 0xBC]).r(), _64b | compat),
        inst("bsfq", fmt("RM", [w(r64), r(rm64)]), rex([0x0F, 0xBC]).r().w(), _64b),

        inst("bsrw", fmt("RM", [w(r16), r(rm16)]), rex([0x66, 0x0F, 0xBD]).r(), _64b | compat),
        inst("bsrl", fmt("RM", [w(r32), r(rm32)]), rex([0x0F, 0xBD]).r(), _64b | compat),
        inst("bsrq", fmt("RM", [w(r64), r(rm64)]), rex([0x0F, 0xBD]).r().w(), _64b),

        inst("tzcntw", fmt("A", [w(r16), r(rm16)]), rex([0x66, 0xF3, 0x0F, 0xBC]).r(), _64b | compat | bmi1),
        inst("tzcntl", fmt("A", [w(r32), r(rm32)]), rex([0xF3, 0x0F, 0xBC]).r(), _64b | compat | bmi1),
        inst("tzcntq", fmt("A", [w(r64), r(rm64)]), rex([0xF3, 0x0F, 0xBC]).r().w(), _64b | bmi1),

        inst("lzcntw", fmt("RM", [w(r16), r(rm16)]), rex([0x66, 0xF3, 0x0F, 0xBD]).r(), _64b | compat | lzcnt),
        inst("lzcntl", fmt("RM", [w(r32), r(rm32)]), rex([0xF3, 0x0F, 0xBD]).r(), _64b | compat | lzcnt),
        inst("lzcntq", fmt("RM", [w(r64), r(rm64)]), rex([0xF3, 0x0F, 0xBD]).r().w(), _64b | lzcnt),

        inst("popcntw", fmt("RM", [w(r16), r(rm16)]), rex([0x66, 0xF3, 0x0F, 0xB8]).r(), _64b | compat | popcnt),
        inst("popcntl", fmt("RM", [w(r32), r(rm32)]), rex([0xF3, 0x0F, 0xB8]).r(), _64b | compat | popcnt),
        inst("popcntq", fmt("RM", [w(r64), r(rm64)]), rex([0xF3, 0x0F, 0xB8]).r().w(), _64b | popcnt),

        // Note that the Intel manual calls has different names for these
        // instructions than Capstone gives them:
        //
        // * cbtw => cbw
        // * cwtl => cwde
        // * cltq => cwqe
        // * cwtd => cwd
        // * cltd => cdq
        // * cqto => cqo
        inst("cbtw", fmt("ZO", [rw(implicit(ax))]), rex([0x66, 0x98]), _64b | compat),
        inst("cwtl", fmt("ZO", [rw(implicit(eax))]), rex([0x98]), _64b | compat),
        inst("cltq", fmt("ZO", [rw(implicit(rax))]), rex([0x98]).w(), _64b),
        inst("cwtd", fmt("ZO", [w(implicit(dx)), r(implicit(ax))]), rex([0x66, 0x99]), _64b | compat),
        inst("cltd", fmt("ZO", [w(implicit(edx)), r(implicit(eax))]), rex([0x99]), _64b | compat),
        inst("cqto", fmt("ZO", [w(implicit(rdx)), r(implicit(rax))]), rex([0x99]).w(), _64b),

        inst("bswapl", fmt("O", [rw(r32)]), rex([0x0F, 0xC8]).rd(), _64b | compat),
        inst("bswapq", fmt("O", [rw(r64)]), rex([0x0F, 0xC8]).w().ro(), _64b),
    ]
}
