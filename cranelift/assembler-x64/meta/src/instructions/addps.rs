use crate::dsl::{fmt, inst, r, vex, w, Feature::*, Inst, Location::*, VexLength::*, VexMMMMM::*};

pub fn list() -> Vec<Inst> {
    vec![inst(
        "vaddps",
        fmt("B", [w(xmm1), r(xmm2), r(xmm_m128)]),
        vex(0x58).length(_128).mmmmm(_OF),
        _64b | compat | sse,
    )]
}
