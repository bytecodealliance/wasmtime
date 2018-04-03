//! ARM ABI implementation.

use super::registers::{D, GPR, Q, S};
use ir;
use isa::RegClass;
use regalloc::RegisterSet;
use settings as shared_settings;

/// Legalize `sig`.
pub fn legalize_signature(
    _sig: &mut ir::Signature,
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
        match ty.bits() {
            32 => S,
            64 => D,
            128 => Q,
            _ => panic!("Unexpected {} ABI type for arm32", ty),
        }
    }
}

/// Get the set of allocatable registers for `func`.
pub fn allocatable_registers(_func: &ir::Function) -> RegisterSet {
    unimplemented!()
}
