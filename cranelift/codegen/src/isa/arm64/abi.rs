//! ARM 64 ABI implementation.

use super::registers::{FPR, GPR};
use crate::ir;
use crate::isa::RegClass;
use crate::regalloc::RegisterSet;
use crate::settings as shared_settings;
use alloc::borrow::Cow;

/// Legalize `sig`.
pub fn legalize_signature(
    _sig: &mut Cow<ir::Signature>,
    _flags: &shared_settings::Flags,
    _current: bool,
) {
    unimplemented!()
}

/// Get register class for a type appearing in a legalized signature.
pub fn regclass_for_abi_type(ty: ir::Type) -> RegClass {
    if ty.is_int() {
        GPR
    } else {
        FPR
    }
}

/// Get the set of allocatable registers for `func`.
pub fn allocatable_registers(_func: &ir::Function) -> RegisterSet {
    unimplemented!()
}
