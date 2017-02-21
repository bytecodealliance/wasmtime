//! Converting Cretonne IL to text.
//!
//! The `write` module provides the `write_function` function which converts an IL `Function` to an
//! equivalent textual representation. This textual representation can be read back by the
//! `cretonne-reader` crate.

use ir::{Function, Ebb, Inst, Value, Type};
use isa::TargetIsa;
use std::fmt::{Result, Error, Write};
use std::result;

/// Write `func` to `w` as equivalent text.
/// Use `isa` to emit ISA-dependent annotations.
pub fn write_function(w: &mut Write, func: &Function, isa: Option<&TargetIsa>) -> Result {
    try!(write_spec(w, func));
    try!(writeln!(w, " {{"));
    let mut any = try!(write_preamble(w, func));
    for ebb in &func.layout {
        if any {
            try!(writeln!(w, ""));
        }
        try!(write_ebb(w, func, isa, ebb));
        any = true;
    }
    writeln!(w, "}}")
}

// ====--------------------------------------------------------------------------------------====//
//
// Function spec.
//
// ====--------------------------------------------------------------------------------------====//

fn write_spec(w: &mut Write, func: &Function) -> Result {
    write!(w, "function {}{}", func.name, func.own_signature())
}

fn write_preamble(w: &mut Write, func: &Function) -> result::Result<bool, Error> {
    let mut any = false;

    for ss in func.stack_slots.keys() {
        any = true;
        try!(writeln!(w, "    {} = {}", ss, func.stack_slots[ss]));
    }

    // Write out all signatures before functions since function declarations can refer to
    // signatures.
    for sig in func.dfg.signatures.keys() {
        any = true;
        try!(writeln!(w, "    {} = signature{}", sig, func.dfg.signatures[sig]));
    }

    for fnref in func.dfg.ext_funcs.keys() {
        any = true;
        try!(writeln!(w, "    {} = {}", fnref, func.dfg.ext_funcs[fnref]));
    }

    for jt in func.jump_tables.keys() {
        any = true;
        try!(writeln!(w, "    {} = {}", jt, func.jump_tables[jt]));
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
        try!(write!(w, "                    "));
    }

    let mut args = func.dfg.ebb_args(ebb);
    match args.next() {
        None => return writeln!(w, "{}:", ebb),
        Some(arg) => {
            try!(write!(w, "{}(", ebb));
            try!(write_arg(w, func, arg));
        }
    }
    // Remaining arguments.
    for arg in args {
        try!(write!(w, ", "));
        try!(write_arg(w, func, arg));
    }
    writeln!(w, "):")
}

pub fn write_ebb(w: &mut Write, func: &Function, isa: Option<&TargetIsa>, ebb: Ebb) -> Result {
    try!(write_ebb_header(w, func, ebb));
    for inst in func.layout.ebb_insts(ebb) {
        try!(write_instruction(w, func, isa, inst));
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
    for &arg in func.dfg[inst].arguments().iter().flat_map(|x| x.iter()) {
        let resolved = func.dfg.resolve_aliases(arg);
        if resolved != arg {
            try!(writeln!(w, "{1:0$}{2} -> {3}", indent, "", arg, resolved));
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
    try!(write_value_aliases(w, func, inst, indent));

    // Write out encoding info.
    if let Some(enc) = func.encodings.get(inst).cloned() {
        let mut s = String::with_capacity(16);
        if let Some(isa) = isa {
            try!(write!(s, "[{}]", isa.display_enc(enc)));
        } else {
            try!(write!(s, "[{}]", enc));
        }
        // Align instruction following ISA annotation to col 24.
        try!(write!(w, "{:23} ", s));
    } else {
        // No annotations, simply indent.
        try!(write!(w, "{1:0$}", indent, ""));
    }

    // Write out the result values, if any.
    let mut has_results = false;
    for r in func.dfg.inst_results(inst) {
        if !has_results {
            has_results = true;
            try!(write!(w, "{}", r));
        } else {
            try!(write!(w, ", {}", r));
        }
    }
    if has_results {
        try!(write!(w, " = "));
    }

    // Then the opcode, possibly with a '.type' suffix.
    let opcode = func.dfg[inst].opcode();

    match type_suffix(func, inst) {
        Some(suf) => try!(write!(w, "{}.{}", opcode, suf)),
        None => try!(write!(w, "{}", opcode)),
    }

    // Then the operands, depending on format.
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
        BinaryImmRev { imm, arg, .. } => writeln!(w, " {}, {}", imm, arg),
        BinaryOverflow { args, .. } => writeln!(w, " {}, {}", args[0], args[1]),
        Ternary { args, .. } => writeln!(w, " {}, {}, {}", args[0], args[1], args[2]),
        TernaryOverflow { ref data, .. } => writeln!(w, " {}", data),
        InsertLane { lane, args, .. } => writeln!(w, " {}, {}, {}", args[0], lane, args[1]),
        ExtractLane { lane, arg, .. } => writeln!(w, " {}, {}", arg, lane),
        IntCompare { cond, args, .. } => writeln!(w, " {}, {}, {}", cond, args[0], args[1]),
        FloatCompare { cond, args, .. } => writeln!(w, " {}, {}, {}", cond, args[0], args[1]),
        Jump { ref data, .. } => writeln!(w, " {}", data),
        Branch { ref data, .. } => writeln!(w, " {}", data),
        BranchTable { arg, table, .. } => writeln!(w, " {}, {}", arg, table),
        Call { ref data, .. } => writeln!(w, " {}({})", data.func_ref, data.varargs),
        IndirectCall { ref data, .. } => {
            writeln!(w, " {}, {}({})", data.sig_ref, data.arg, data.varargs)
        }
        Return { ref data, .. } => {
            if data.varargs.is_empty() {
                writeln!(w, "")
            } else {
                writeln!(w, " {}", data.varargs)
            }
        }
        ReturnReg { ref data, .. } => {
            if data.varargs.is_empty() {
                writeln!(w, " {}", data.arg)
            } else {
                writeln!(w, " {}, {}", data.arg, data.varargs)
            }
        }
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
