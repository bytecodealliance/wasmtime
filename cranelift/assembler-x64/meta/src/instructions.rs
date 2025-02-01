//! Defines x64 instructions using the DSL.

mod and;

use crate::dsl::Inst;

#[must_use]
pub fn list() -> Vec<Inst> {
    and::list()
}
