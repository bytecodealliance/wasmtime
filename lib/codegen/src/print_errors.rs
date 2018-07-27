//! Utility routines for pretty-printing error messages.

use ir;
use ir::entities::Inst;
use ir::function::Function;
use isa::TargetIsa;
use result::CodegenError;
use std::fmt;
use std::fmt::Write;
use std::string::{String, ToString};
use verifier::VerifierError;
use write::decorate_function;

/// Pretty-print a verifier error.
pub fn pretty_verifier_error(
    func: &ir::Function,
    isa: Option<&TargetIsa>,
    err: &VerifierError,
) -> String {
    let mut w = String::new();

    match err.location {
        ir::entities::AnyEntity::Inst(_) => {}
        _ => {
            // Print the error, because the pretty_function_error below won't do it since it isn't
            // tied to an instruction.
            writeln!(w, "verifier error summary: {}\n", err.to_string()).unwrap();
        }
    }

    decorate_function(
        &mut |w, func, isa, inst, indent| pretty_function_error(w, func, isa, inst, indent, err),
        &mut w,
        func,
        isa,
    ).unwrap();
    w
}

/// Pretty-print a function verifier error.
fn pretty_function_error(
    w: &mut Write,
    func: &Function,
    isa: Option<&TargetIsa>,
    cur_inst: Inst,
    indent: usize,
    err: &VerifierError,
) -> fmt::Result {
    match err.location {
        ir::entities::AnyEntity::Inst(inst) if inst == cur_inst => {
            writeln!(
                w,
                "{1:0$}{2}",
                indent,
                "",
                func.dfg.display_inst(cur_inst, isa)
            )?;
            write!(w, "{1:0$}^", indent, "")?;
            for _c in cur_inst.to_string().chars() {
                write!(w, "~")?;
            }
            writeln!(w, " verifier {}\n", err.to_string())
        }
        _ => writeln!(
            w,
            "{1:0$}{2}",
            indent,
            "",
            func.dfg.display_inst(cur_inst, isa)
        ),
    }
}

/// Pretty-print a Cranelift error.
pub fn pretty_error(func: &ir::Function, isa: Option<&TargetIsa>, err: CodegenError) -> String {
    if let CodegenError::Verifier(e) = err {
        pretty_verifier_error(func, isa, &e)
    } else {
        err.to_string()
    }
}
