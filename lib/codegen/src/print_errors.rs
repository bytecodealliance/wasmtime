//! Utility routines for pretty-printing error messages.

use entity::SecondaryMap;
use ir;
use ir::entities::{AnyEntity, Inst, Value};
use ir::function::Function;
use isa::TargetIsa;
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

    decorate_function(
        &mut PrettyVerifierError(func_w.unwrap_or_else(|| Box::new(PlainWriter)), &mut errors),
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
        aliases: &SecondaryMap<Value, Vec<Value>>,
        isa: Option<&TargetIsa>,
        inst: Inst,
        indent: usize,
    ) -> fmt::Result {
        pretty_instruction_error(w, func, aliases, isa, inst, indent, &mut *self.0, self.1)
    }

    fn write_entity_definition(
        &mut self,
        w: &mut Write,
        func: &Function,
        entity: AnyEntity,
        value: &fmt::Display,
    ) -> fmt::Result {
        pretty_preamble_error(w, func, entity, value, &mut *self.0, self.1)
    }
}

/// Pretty-print a function verifier error.
fn pretty_instruction_error(
    w: &mut Write,
    func: &Function,
    aliases: &SecondaryMap<Value, Vec<Value>>,
    isa: Option<&TargetIsa>,
    cur_inst: Inst,
    indent: usize,
    func_w: &mut FuncWriter,
    errors: &mut Vec<VerifierError>,
) -> fmt::Result {
    // TODO: Use drain_filter here when it gets stabilized
    let mut i = 0;
    let mut printed_instr = false;

    while i != errors.len() {
        match errors[i].location {
            ir::entities::AnyEntity::Inst(inst) if inst == cur_inst => {
                let err = errors.remove(i);

                if !printed_instr {
                    func_w.write_instruction(w, func, aliases, isa, cur_inst, indent)?;
                    printed_instr = true;
                }

                print_error(w, indent, cur_inst.to_string(), err)?;
            }
            ir::entities::AnyEntity::Inst(_) => i += 1,
            _ => unreachable!(),
        }
    }

    if printed_instr {
        w.write_char('\n')?;
    } else {
        writeln!(
            w,
            "{1:0$}{2}",
            indent,
            "",
            func.dfg.display_inst(cur_inst, isa)
        )?;
    }

    Ok(())
}

fn pretty_preamble_error(
    w: &mut Write,
    func: &Function,
    entity: AnyEntity,
    value: &fmt::Display,
    func_w: &mut FuncWriter,
    errors: &mut Vec<VerifierError>,
) -> fmt::Result {
    // TODO: Use drain_filter here when it gets stabilized
    let indent = 4;

    let mut i = 0;
    let mut printed_entity = false;

    while i != errors.len() {
        if entity == errors[i].location {
            let err = errors.remove(i);

            if !printed_entity {
                func_w.write_entity_definition(w, func, entity, value)?;
                printed_entity = true;
            }

            print_error(w, indent, entity.to_string(), err)?;
        } else {
            i += 1
        }
    }

    if printed_entity {
        w.write_char('\n')?;
    } else {
        func_w.write_entity_definition(w, func, entity, value)?;
    }

    Ok(())
}

/// Prints ;   ^~~~~~ verifier [ERROR BODY]
fn print_error(w: &mut Write, indent: usize, s: String, err: VerifierError) -> fmt::Result {
    let indent = if indent < 1 { 0 } else { indent - 1 };

    write!(w, ";{1:0$}^", indent, "")?;
    for _c in s.chars() {
        write!(w, "~")?;
    }
    writeln!(w, " verifier {}", err.to_string())?;
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
