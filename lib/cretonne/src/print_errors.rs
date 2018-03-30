//! Utility routines for pretty-printing error messages.

use ir;
use isa::TargetIsa;
use result::CtonError;
use std::fmt::Write;
use verifier;

/// Pretty-print a verifier error.
pub fn pretty_verifier_error(
    func: &ir::Function,
    isa: Option<&TargetIsa>,
    err: &verifier::Error,
) -> String {
    let mut msg = err.to_string();
    match err.location {
        ir::entities::AnyEntity::Inst(inst) => {
            write!(msg, "\n{}: {}\n\n", inst, func.dfg.display_inst(inst, isa)).unwrap()
        }
        _ => msg.push('\n'),
    }
    write!(msg, "{}", func.display(isa)).unwrap();
    msg
}

/// Pretty-print a Cretonne error.
pub fn pretty_error(func: &ir::Function, isa: Option<&TargetIsa>, err: CtonError) -> String {
    if let CtonError::Verifier(e) = err {
        pretty_verifier_error(func, isa, &e)
    } else {
        err.to_string()
    }
}
