use crate::dsl::{
    fmt, inst, r, vex, w, Feature::*, Inst, Location::*, VexLength::*, VexMMMMM::*, VexPP::*,
};

pub fn list() -> Vec<Inst> {
    vec![inst(
        "vaddpd",
        fmt("B", [w(xmm1), r(xmm2), r(xmm_m128)]),
        vex(0x58).length(_128).pp(_66).mmmmm(_OF),
        _64b | compat | sse,
    )]
}
