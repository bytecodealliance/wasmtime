//! Converting Cretonne IL to text.
//!
//! The `write` module provides the `write_function` function which converts an IL `Function` to an
//! equivalent textual representation. This textual representation can be read back by the
//! `cretonne-reader` crate.

use ir::{Function, Ebb, Inst, Value, Type};
use std::fmt::{Result, Error, Write};
use std::result;

/// Write `func` to `w` as equivalent text.
pub fn write_function(w: &mut Write, func: &Function) -> Result {
    try!(write_spec(w, func));
    try!(writeln!(w, " {{"));
    let mut any = try!(write_preamble(w, func));
    for ebb in &func.layout {
        if any {
            try!(writeln!(w, ""));
        }
        try!(write_ebb(w, func, ebb));
        any = true;
    }
    writeln!(w, "}}")
}

// ====--------------------------------------------------------------------------------------====//
//
// Function spec.
//
// ====--------------------------------------------------------------------------------------====//

// The function name may need quotes if it doesn't parse as an identifier.
fn needs_quotes(name: &str) -> bool {
    let mut iter = name.chars();
    if let Some(ch) = iter.next() {
        !ch.is_alphabetic() || !iter.all(char::is_alphanumeric)
    } else {
        // A blank function name needs quotes.
        true
    }
}

// Use Rust's escape_default which provides a few simple \t \r \n \' \" \\ escapes and uses
// \u{xxxx} for anything else outside the ASCII printable range.
fn escaped(name: &str) -> String {
    name.chars().flat_map(char::escape_default).collect()
}

fn write_spec(w: &mut Write, func: &Function) -> Result {
    let sig = func.own_signature();
    if !needs_quotes(&func.name) {
        write!(w, "function {}{}", func.name, sig)
    } else {
        write!(w, "function \"{}\"{}", escaped(&func.name), sig)
    }
}

fn write_preamble(w: &mut Write, func: &Function) -> result::Result<bool, Error> {
    let mut any = false;

    for ss in func.stack_slots.keys() {
        any = true;
        try!(writeln!(w, "    {} = {}", ss, func.stack_slots[ss]));
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

    let mut args = func.dfg.ebb_args(ebb);
    match args.next() {
        None => return writeln!(w, "{}:", ebb),
        Some(arg) => {
            try!(write!(w, "{}(", ebb));
            try!(write_arg(w, func, arg));
        }
    }
    // Remaining args.
    for arg in args {
        try!(write!(w, ", "));
        try!(write_arg(w, func, arg));
    }
    writeln!(w, "):")
}

pub fn write_ebb(w: &mut Write, func: &Function, ebb: Ebb) -> Result {
    try!(write_ebb_header(w, func, ebb));
    for inst in func.layout.ebb_insts(ebb) {
        try!(write_instruction(w, func, inst));
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

pub fn write_instruction(w: &mut Write, func: &Function, inst: Inst) -> Result {
    try!(write!(w, "    "));

    // First write out the result values, if any.
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
        UnaryImmVector { .. } => writeln!(w, " [...]"),
        Binary { args, .. } => writeln!(w, " {}, {}", args[0], args[1]),
        BinaryImm { arg, imm, .. } => writeln!(w, " {}, {}", arg, imm),
        BinaryImmRev { imm, arg, .. } => writeln!(w, " {}, {}", imm, arg),
        BinaryOverflow { args, .. } => writeln!(w, " {}, {}", args[0], args[1]),
        Ternary { args, .. } => writeln!(w, " {}, {}, {}", args[0], args[1], args[2]),
        InsertLane { lane, args, .. } => writeln!(w, " {}, {}, {}", args[0], lane, args[1]),
        ExtractLane { lane, arg, .. } => writeln!(w, " {}, {}", arg, lane),
        IntCompare { cond, args, .. } => writeln!(w, " {}, {}, {}", cond, args[0], args[1]),
        FloatCompare { cond, args, .. } => writeln!(w, " {}, {}, {}", cond, args[0], args[1]),
        Jump { ref data, .. } => writeln!(w, " {}", data),
        Branch { ref data, .. } => writeln!(w, " {}", data),
        BranchTable { arg, table, .. } => writeln!(w, " {}, {}", arg, table),
        Call { ref data, .. } => writeln!(w, " {}", data),
        Return { ref data, .. } => {
            if data.args.is_empty() {
                writeln!(w, "")
            } else {
                writeln!(w, " {}", data.args)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{needs_quotes, escaped};
    use ir::{Function, StackSlotData};
    use ir::types;

    #[test]
    fn quoting() {
        assert_eq!(needs_quotes(""), true);
        assert_eq!(needs_quotes("x"), false);
        assert_eq!(needs_quotes(" "), true);
        assert_eq!(needs_quotes("0"), true);
        assert_eq!(needs_quotes("x0"), false);
    }

    #[test]
    fn escaping() {
        assert_eq!(escaped(""), "");
        assert_eq!(escaped("x"), "x");
        assert_eq!(escaped(" "), " ");
        assert_eq!(escaped(" \n"), " \\n");
        assert_eq!(escaped("a\u{1000}v"), "a\\u{1000}v");
    }

    #[test]
    fn basic() {
        let mut f = Function::new();
        assert_eq!(f.to_string(), "function \"\"() {\n}\n");

        f.name.push_str("foo");
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
