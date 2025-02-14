//! Defines x64 instructions using the DSL.

mod and;
mod shld;

use crate::dsl::Inst;

#[must_use]
pub fn list() -> Vec<Inst> {
    let mut ret = Vec::new();
    ret.extend(and::list());
    ret.extend(shld::list());
    ret
}
