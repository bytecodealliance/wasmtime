//! ARM 64 ABI implementation.

use ir;
use isa::RegClass;
use settings as shared_settings;
use super::registers::{GPR, FPR};

/// Legalize `sig`.
pub fn legalize_signature(_sig: &mut ir::Signature,
                          _flags: &shared_settings::Flags,
                          _current: bool) {
    unimplemented!()
}

/// Get register class for a type appearing in a legalized signature.
pub fn regclass_for_abi_type(ty: ir::Type) -> RegClass {
    if ty.is_int() { GPR } else { FPR }
}
