use crate::dsl::{Feature::*, Inst};
use crate::dsl::{fmt, inst, rex};

#[rustfmt::skip] // Keeps instructions on a single line.
pub fn list() -> Vec<Inst> {
    vec![
        inst("mfence", fmt("ZO", []), rex([0x0f, 0xae, 0xf0]), _64b | compat | sse2),
        inst("sfence", fmt("ZO", []), rex([0x0f, 0xae, 0xf8]), _64b | compat),
        inst("lfence", fmt("ZO", []), rex([0x0f, 0xae, 0xe8]), _64b | compat | sse2),

        inst("hlt", fmt("ZO", []), rex([0xf4]), _64b | compat),
        inst("ud2", fmt("ZO", []), rex([0x0f, 0x0b]), _64b | compat).has_trap(),
    ]
}
