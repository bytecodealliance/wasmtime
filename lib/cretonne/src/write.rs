//! Converting Cretonne IL to text.
//!
//! The `write` module provides the `write_function` function which converts an IL `Function` to an
//! equivalent textual representation. This textual representation can be read back by the
//! `cretonne-reader` crate.

use ir::{Function, Ebb, Inst, Value, Type};
use isa::{TargetIsa, RegInfo};
use std::fmt::{self, Result, Error, Write};
use std::result;

/// Write `func` to `w` as equivalent text.
/// Use `isa` to emit ISA-dependent annotations.
pub fn write_function(w: &mut Write, func: &Function, isa: Option<&TargetIsa>) -> Result {
    let regs = isa.map(TargetIsa::register_info);
    let regs = regs.as_ref();

    write_spec(w, func, regs)?;
    writeln!(w, " {{")?;
    let mut any = write_preamble(w, func, regs)?;
    for ebb in &func.layout {
        if any {
            writeln!(w, "")?;
        }
        write_ebb(w, func, isa, ebb)?;
        any = true;
    }
    writeln!(w, "}}")
}

// ====--------------------------------------------------------------------------------------====//
//
// Function spec.
//
// ====--------------------------------------------------------------------------------------====//

fn write_spec(w: &mut Write, func: &Function, regs: Option<&RegInfo>) -> Result {
    write!(w, "function {}{}", func.name, func.signature.display(regs))
}

fn write_preamble(w: &mut Write,
                  func: &Function,
                  regs: Option<&RegInfo>)
                  -> result::Result<bool, Error> {
    let mut any = false;

    for ss in func.stack_slots.keys() {
        any = true;
        writeln!(w, "    {} = {}", ss, func.stack_slots[ss])?;
    }

    // Write out all signatures before functions since function declarations can refer to
    // signatures.
    for sig in func.dfg.signatures.keys() {
        any = true;
        writeln!(w,
                 "    {} = signature{}",
                 sig,
                 func.dfg.signatures[sig].display(regs))?;
    }

    for fnref in func.dfg.ext_funcs.keys() {
        any = true;
        writeln!(w, "    {} = {}", fnref, func.dfg.ext_funcs[fnref])?;
    }

    for jt in func.jump_tables.keys() {
        any = true;
        writeln!(w, "    {} = {}", jt, func.jump_tables[jt])?;
    }

    Ok(any)
}

// ====--------------------------------------------------------------------------------------====//
//
// Basic blocks
//
// ====--------------------------------------------------------------------------------------====//

pub fn write_arg(w: &mut Write, func: &Function, arg: Value) -> Result {
    write!(w, "{}: {}", arg, func.dfg.value_type(arg))
}

pub fn write_ebb_header(w: &mut Write, func: &Function, ebb: Ebb) -> Result {
    // Write out the basic block header, outdented:
    //
    //    ebb1:
    //    ebb1(vx1: i32):
    //    ebb10(vx4: f64, vx5: b1):
    //

    // If we're writing encoding annotations, shift by 20.
    if !func.encodings.is_empty() {
        write!(w, "                    ")?;
    }

    let mut args = func.dfg.ebb_args(ebb);
    match args.next() {
        None => return writeln!(w, "{}:", ebb),
        Some(arg) => {
            write!(w, "{}(", ebb)?;
            write_arg(w, func, arg)?;
        }
    }
    // Remaining arguments.
    for arg in args {
        write!(w, ", ")?;
        write_arg(w, func, arg)?;
    }
    writeln!(w, "):")
}

pub fn write_ebb(w: &mut Write, func: &Function, isa: Option<&TargetIsa>, ebb: Ebb) -> Result {
    write_ebb_header(w, func, ebb)?;
    for inst in func.layout.ebb_insts(ebb) {
        write_instruction(w, func, isa, inst)?;
    }
    Ok(())
}


// ====--------------------------------------------------------------------------------------====//
//
// Instructions
//
// ====--------------------------------------------------------------------------------------====//

// Should `inst` be printed with a type suffix?
//
// Polymorphic instructions may need a suffix indicating the value of the controlling type variable
// if it can't be trivially inferred.
//
fn type_suffix(func: &Function, inst: Inst) -> Option<Type> {
    let constraints = func.dfg[inst].opcode().constraints();

    if !constraints.is_polymorphic() {
        return None;
    }

    // If the controlling type variable can be inferred from the type of the designated value input
    // operand, we don't need the type suffix.
    // TODO: Should we include the suffix when the input value is defined in another block? The
    // parser needs to know the type of the value, so it must be defined in a block that lexically
    // comes before this one.
    if constraints.use_typevar_operand() {
        return None;
    }

    // This polymorphic instruction doesn't support basic type inference.
    // The controlling type variable is required to be the type of the first result.
    let rtype = func.dfg.value_type(func.dfg.first_result(inst));
    assert!(!rtype.is_void(),
            "Polymorphic instruction must produce a result");
    Some(rtype)
}

// Write out any value aliases appearing in `inst`.
fn write_value_aliases(w: &mut Write, func: &Function, inst: Inst, indent: usize) -> Result {
    for &arg in func.dfg[inst].arguments(&func.dfg.value_lists) {
        let resolved = func.dfg.resolve_aliases(arg);
        if resolved != arg {
            writeln!(w, "{1:0$}{2} -> {3}", indent, "", arg, resolved)?;
        }
    }
    Ok(())
}

fn write_instruction(w: &mut Write,
                     func: &Function,
                     isa: Option<&TargetIsa>,
                     inst: Inst)
                     -> Result {
    // Indent all instructions to col 24 if any encodings are present.
    let indent = if func.encodings.is_empty() { 4 } else { 24 };

    // Value aliases come out on lines before the instruction using them.
    write_value_aliases(w, func, inst, indent)?;

    // Write out encoding info.
    if let Some(enc) = func.encodings.get(inst).cloned() {
        let mut s = String::with_capacity(16);
        if let Some(isa) = isa {
            write!(s, "[{}", isa.display_enc(enc))?;
            // Write value locations, if we have them.
            if !func.locations.is_empty() {
                let regs = isa.register_info();
                for r in func.dfg.inst_results(inst) {
                    write!(s,
                           ",{}",
                           func.locations.get(r).cloned().unwrap_or_default().display(&regs))?
                }
            }
            write!(s, "]")?;
        } else {
            write!(s, "[{}]", enc)?;
        }
        // Align instruction following ISA annotation to col 24.
        write!(w, "{:23} ", s)?;
    } else {
        // No annotations, simply indent.
        write!(w, "{1:0$}", indent, "")?;
    }

    // Write out the result values, if any.
    let mut has_results = false;
    for r in func.dfg.inst_results(inst) {
        if !has_results {
            has_results = true;
            write!(w, "{}", r)?;
        } else {
            write!(w, ", {}", r)?;
        }
    }
    if has_results {
        write!(w, " = ")?;
    }

    // Then the opcode, possibly with a '.type' suffix.
    let opcode = func.dfg[inst].opcode();

    match type_suffix(func, inst) {
        Some(suf) => write!(w, "{}.{}", opcode, suf)?,
        None => write!(w, "{}", opcode)?,
    }

    // Then the operands, depending on format.
    let pool = &func.dfg.value_lists;
    use ir::instructions::InstructionData::*;
    match func.dfg[inst] {
        Nullary { .. } => writeln!(w, ""),
        Unary { arg, .. } => writeln!(w, " {}", arg),
        UnaryImm { imm, .. } => writeln!(w, " {}", imm),
        UnaryIeee32 { imm, .. } => writeln!(w, " {}", imm),
        UnaryIeee64 { imm, .. } => writeln!(w, " {}", imm),
        UnaryImmVector { ref data, .. } => writeln!(w, " {}", data),
        UnarySplit { arg, .. } => writeln!(w, " {}", arg),
        Binary { args, .. } => writeln!(w, " {}, {}", args[0], args[1]),
        BinaryImm { arg, imm, .. } => writeln!(w, " {}, {}", arg, imm),
        BinaryOverflow { args, .. } => writeln!(w, " {}, {}", args[0], args[1]),
        Ternary { args, .. } => writeln!(w, " {}, {}, {}", args[0], args[1], args[2]),
        TernaryOverflow { ref data, .. } => writeln!(w, " {}", data),
        InsertLane { lane, args, .. } => writeln!(w, " {}, {}, {}", args[0], lane, args[1]),
        ExtractLane { lane, arg, .. } => writeln!(w, " {}, {}", arg, lane),
        IntCompare { cond, args, .. } => writeln!(w, " {}, {}, {}", cond, args[0], args[1]),
        FloatCompare { cond, args, .. } => writeln!(w, " {}, {}, {}", cond, args[0], args[1]),
        Jump { destination, ref args, .. } => {
            if args.is_empty() {
                writeln!(w, " {}", destination)
            } else {
                writeln!(w,
                         " {}({})",
                         destination,
                         DisplayValues(args.as_slice(pool)))
            }
        }
        Branch { destination, ref args, .. } => {
            let args = args.as_slice(pool);
            if args.len() == 1 {
                writeln!(w, " {}, {}", args[0], destination)
            } else {
                writeln!(w,
                         " {}, {}({})",
                         args[0],
                         destination,
                         DisplayValues(&args[1..]))
            }
        }
        BranchTable { arg, table, .. } => writeln!(w, " {}, {}", arg, table),
        Call { func_ref, ref args, .. } => {
            writeln!(w,
                     " {}({})",
                     func_ref,
                     DisplayValues(args.as_slice(&func.dfg.value_lists)))
        }
        IndirectCall { sig_ref, ref args, .. } => {
            let args = args.as_slice(pool);
            writeln!(w,
                     " {}, {}({})",
                     sig_ref,
                     args[0],
                     DisplayValues(&args[1..]))
        }
        Return { ref args, .. } => {
            if args.is_empty() {
                writeln!(w, "")
            } else {
                writeln!(w,
                         " {}",
                         DisplayValues(args.as_slice(&func.dfg.value_lists)))
            }
        }
        ReturnReg { ref args, .. } => {
            writeln!(w,
                     " {}",
                     DisplayValues(args.as_slice(&func.dfg.value_lists)))
        }
    }
}

/// Displayable slice of values.
struct DisplayValues<'a>(&'a [Value]);

impl<'a> fmt::Display for DisplayValues<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result {
        for (i, val) in self.0.iter().enumerate() {
            if i == 0 {
                write!(f, "{}", val)?;
            } else {
                write!(f, ", {}", val)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use ir::{Function, FunctionName, StackSlotData};
    use ir::types;

    #[test]
    fn basic() {
        let mut f = Function::new();
        assert_eq!(f.to_string(), "function \"\"() {\n}\n");

        f.name = FunctionName::new("foo".to_string());
        assert_eq!(f.to_string(), "function foo() {\n}\n");

        f.stack_slots.push(StackSlotData::new(4));
        assert_eq!(f.to_string(),
                   "function foo() {\n    ss0 = stack_slot 4\n}\n");

        let ebb = f.dfg.make_ebb();
        f.layout.append_ebb(ebb);
        assert_eq!(f.to_string(),
                   "function foo() {\n    ss0 = stack_slot 4\n\nebb0:\n}\n");

        f.dfg.append_ebb_arg(ebb, types::I8);
        assert_eq!(f.to_string(),
                   "function foo() {\n    ss0 = stack_slot 4\n\nebb0(vx0: i8):\n}\n");

        f.dfg.append_ebb_arg(ebb, types::F32.by(4).unwrap());
        assert_eq!(f.to_string(),
                   "function foo() {\n    ss0 = stack_slot 4\n\nebb0(vx0: i8, vx1: f32x4):\n}\n");
    }
}
