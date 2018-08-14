//! Utility routines for pretty-printing error messages.

use ir;
use ir::entities::Inst;
use ir::function::Function;
use isa::{RegInfo, TargetIsa};
use result::CodegenError;
use std::boxed::Box;
use std::fmt;
use std::fmt::Write;
use std::string::{String, ToString};
use std::vec::Vec;
use verifier::{VerifierError, VerifierErrors};
use write::{decorate_function, FuncWriter, PlainWriter};

/// Pretty-print a verifier error.
pub fn pretty_verifier_error<'a>(
    func: &ir::Function,
    isa: Option<&TargetIsa>,
    func_w: Option<Box<FuncWriter + 'a>>,
    errors: VerifierErrors,
) -> String {
    let mut errors = errors.0;
    let mut w = String::new();

    // TODO: Use drain_filter here when it gets stabilized
    let mut i = 0;

    while i != errors.len() {
        if let ir::entities::AnyEntity::Inst(_) = errors[i].location {
            let err = errors.remove(i);

            writeln!(w, "Miscellaneous error: {}\n", err).unwrap()
        } else {
            i += 1;
        }
    }

    decorate_function(
        &mut PrettyVerifierError(func_w.unwrap_or(Box::new(PlainWriter)), &mut errors),
        &mut w,
        func,
        isa,
    ).unwrap();
    w
}

struct PrettyVerifierError<'a>(Box<FuncWriter + 'a>, &'a mut Vec<VerifierError>);

impl<'a> FuncWriter for PrettyVerifierError<'a> {
    fn write_instruction(
        &mut self,
        w: &mut Write,
        func: &Function,
        isa: Option<&TargetIsa>,
        inst: Inst,
        indent: usize,
    ) -> fmt::Result {
        pretty_function_error(w, func, isa, inst, indent, &mut *self.0, self.1)
    }

    fn write_preamble(
        &mut self,
        w: &mut Write,
        func: &Function,
        regs: Option<&RegInfo>,
    ) -> Result<bool, fmt::Error> {
        self.0.write_preamble(w, func, regs)
    }
}

/// Pretty-print a function verifier error.
fn pretty_function_error(
    w: &mut Write,
    func: &Function,
    isa: Option<&TargetIsa>,
    cur_inst: Inst,
    indent: usize,
    func_w: &mut FuncWriter,
    errors: &mut Vec<VerifierError>,
) -> fmt::Result {
    // TODO: Use drain_filter here when it gets stabilized
    let mut i = 0;

    while i != errors.len() {
        match errors[i].location {
            ir::entities::AnyEntity::Inst(inst) if inst == cur_inst => {
                let err = errors.remove(i);

                func_w.write_instruction(w, func, isa, cur_inst, indent)?;
                write!(w, "{1:0$}^", indent, "")?;
                for _c in cur_inst.to_string().chars() {
                    write!(w, "~")?;
                }
                writeln!(w, " verifier {}\n", err.to_string())?;
            }
            ir::entities::AnyEntity::Inst(_) => i += 1,
            _ => unreachable!(),
        }
    }

    Ok(())
}

/// Pretty-print a Cranelift error.
pub fn pretty_error(func: &ir::Function, isa: Option<&TargetIsa>, err: CodegenError) -> String {
    if let CodegenError::Verifier(e) = err {
        pretty_verifier_error(func, isa, None, e)
    } else {
        err.to_string()
    }
}
