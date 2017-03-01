//! RISC-V ABI implementation.
//!
//! This module implements the RISC-V calling convention through the primary `legalize_signature()`
//! entry point.

use ir::Signature;
use settings as shared_settings;

/// Legalize `sig` for RISC-V.
pub fn legalize_signature(_sig: &mut Signature, _flags: &shared_settings::Flags) {
    // TODO: Actually do something.
}
