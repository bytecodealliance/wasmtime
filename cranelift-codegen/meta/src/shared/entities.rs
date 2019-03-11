use crate::cdsl::operands::{OperandKind, OperandKindBuilder as Builder, OperandKindFields};

/// Small helper to initialize an OperandBuilder with the right kind, for a given name and doc.
fn create(name: &'static str, doc: &'static str) -> Builder {
    Builder::new(name, OperandKindFields::EntityRef).doc(doc)
}

pub fn define() -> Vec<OperandKind> {
    let mut kinds = Vec::new();

    // A reference to an extended basic block in the same function.
    // This is primarliy used in control flow instructions.
    let ebb = create("ebb", "An extended basic block in the same function.")
        .default_member("destination")
        .finish();
    kinds.push(ebb);

    // A reference to a stack slot declared in the function preamble.
    let stack_slot = create("stack_slot", "A stack slot").finish();
    kinds.push(stack_slot);

    // A reference to a global value.
    let global_value = create("global_value", "A global value.").finish();
    kinds.push(global_value);

    // A reference to a function signature declared in the function preamble.
    // This is used to provide the call signature in a call_indirect instruction.
    let sig_ref = create("sig_ref", "A function signature.").finish();
    kinds.push(sig_ref);

    // A reference to an external function declared in the function preamble.
    // This is used to provide the callee and signature in a call instruction.
    let func_ref = create("func_ref", "An external function.").finish();
    kinds.push(func_ref);

    // A reference to a jump table declared in the function preamble.
    let jump_table = create("jump_table", "A jump table.")
        .default_member("table")
        .finish();
    kinds.push(jump_table);

    // A reference to a heap declared in the function preamble.
    let heap = create("heap", "A heap.").finish();
    kinds.push(heap);

    // A reference to a table declared in the function preamble.
    let table = create("table", "A table.").finish();
    kinds.push(table);

    // A variable-sized list of value operands. Use for Ebb and function call arguments.
    let varargs = Builder::new("variable_args", OperandKindFields::VariableArgs)
        .doc(
            r#"
            A variable size list of `value` operands.

            Use this to represent arguments passed to a function call, arguments
            passed to an extended basic block, or a variable number of results
            returned from an instruction.
        "#,
        )
        .finish();
    kinds.push(varargs);

    return kinds;
}
