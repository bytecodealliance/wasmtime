use crate::dsl::{align, fmt, inst, r, rex, rw, vex, w, Feature::*, Inst, Location::*, VexLength::*, VexMMMMM::*};

pub fn list() -> Vec<Inst> {
    vec![
        inst("addps", fmt("A", [rw(xmm1), r(align(rm128))]), rex([0x0F, 0x58]).r(), _64b | compat | sse),
        inst(
            "vaddps",
            fmt("B", [w(xmm1), r(xmm2), r(xmm_m128)]),
            vex(0x58).length(_128).mmmmm(_OF),
            _64b | compat | sse,
        ),
    ]
}
