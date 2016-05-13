//! Converting Cretonne IL to text.
//!
//! The `write` module provides the `write_function` function which converts an IL `Function` to an
//! equivalent textual representation. This textual representation can be read back by the
//! `cretonne-reader` crate.

use std::io::{self, Write};
use repr::Function;
use entities::{Inst, Ebb, Value};

pub type Result = io::Result<()>;

/// Write `func` to `w` as equivalent text.
pub fn write_function(w: &mut Write, func: &Function) -> Result {
    try!(write_spec(w, func));
    try!(writeln!(w, " {{"));
    let mut any = try!(write_preamble(w, func));
    for ebb in func.ebbs_numerically() {
        if !any {
            try!(writeln!(w, ""));
        }
        try!(write_ebb(w, func, ebb));
        any = true;
    }
    writeln!(w, "}}")
}

/// Convert `func` to a string.
pub fn function_to_string(func: &Function) -> String {
    let mut buffer: Vec<u8> = Vec::new();
    // Any errors here would be out-of-memory, which should not happen with normal functions.
    write_function(&mut buffer, func).unwrap();
    // A UTF-8 conversion error is a real bug.
    String::from_utf8(buffer).unwrap()
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
        write!(w, "function \"{}\" {}", escaped(&func.name), sig)
    }
}

fn write_preamble(w: &mut Write, func: &Function) -> io::Result<bool> {
    let mut any = false;

    for ss in func.stack_slot_iter() {
        any = true;
        try!(writeln!(w, "    {} = {}", ss, func[ss]));
    }

    Ok(any)
}

// ====--------------------------------------------------------------------------------------====//
//
// Basic blocks
//
// ====--------------------------------------------------------------------------------------====//

pub fn write_arg(w: &mut Write, func: &Function, arg: Value) -> Result {
    write!(w, "{}: {}", arg, func.value_type(arg))
}

pub fn write_ebb_header(w: &mut Write, func: &Function, ebb: Ebb) -> Result {
    // Write out the basic block header, outdented:
    //
    //    ebb1:
    //    ebb1(vx1: i32):
    //    ebb10(vx4: f64, vx5: b1):
    //

    let mut args = func.ebb_args(ebb);
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
    for inst in func.ebb_insts(ebb) {
        try!(write_instruction(w, func, inst));
    }
    Ok(())
}


// ====--------------------------------------------------------------------------------------====//
//
// Instructions
//
// ====--------------------------------------------------------------------------------------====//

pub fn write_instruction(w: &mut Write, func: &Function, inst: Inst) -> Result {
    try!(write!(w, "    "));

    // First write out the result values, if any.
    let mut has_results = false;
    for r in func.inst_results(inst) {
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

    // Then the opcode and operands, depending on format.
    use instructions::InstructionData::*;
    match func[inst] {
        Nullary { opcode, .. } => writeln!(w, "{}", opcode),
        Unary { opcode, arg, .. } => writeln!(w, "{} {}", opcode, arg),
        UnaryImm { opcode, imm, .. } => writeln!(w, "{} {}", opcode, imm),
        UnaryIeee32 { opcode, imm, .. } => writeln!(w, "{} {}", opcode, imm),
        UnaryIeee64 { opcode, imm, .. } => writeln!(w, "{} {}", opcode, imm),
        UnaryImmVector { opcode, .. } => writeln!(w, "{} [...]", opcode),
        Binary { opcode, args, .. } => writeln!(w, "{} {}, {}", opcode, args[0], args[1]),
        BinaryImm { opcode, lhs, rhs, .. } => writeln!(w, "{} {}, {}", opcode, lhs, rhs),
        BinaryImmRev { opcode, lhs, rhs, .. } => writeln!(w, "{} {}, {}", opcode, lhs, rhs),
        Call { opcode, .. } => writeln!(w, "{} [...]", opcode),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::{needs_quotes, escaped};
    use repr::{Function, StackSlotData};
    use types;

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
        assert_eq!(function_to_string(&f), "function \"\" () {\n}\n");

        f.name.push_str("foo");
        assert_eq!(function_to_string(&f), "function foo() {\n}\n");

        f.make_stack_slot(StackSlotData::new(4));
        assert_eq!(function_to_string(&f),
                   "function foo() {\n    ss0 = stack_slot 4\n}\n");

        let ebb = f.make_ebb();
        assert_eq!(function_to_string(&f),
                   "function foo() {\n    ss0 = stack_slot 4\nebb0:\n}\n");

        f.append_ebb_arg(ebb, types::I8);
        assert_eq!(function_to_string(&f),
                   "function foo() {\n    ss0 = stack_slot 4\nebb0(vx0: i8):\n}\n");

        f.append_ebb_arg(ebb, types::F32.by(4).unwrap());
        assert_eq!(function_to_string(&f),
                   "function foo() {\n    ss0 = stack_slot 4\nebb0(vx0: i8, vx1: f32x4):\n}\n");
    }
}
