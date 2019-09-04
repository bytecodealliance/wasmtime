use crate::cdsl::operands::{OperandKind, OperandKindBuilder as Builder, OperandKindFields};

pub struct EntityRefs {
    /// A reference to an extended basic block in the same function.
    /// This is primarliy used in control flow instructions.
    pub ebb: OperandKind,

    /// A reference to a stack slot declared in the function preamble.
    pub stack_slot: OperandKind,

    /// A reference to a global value.
    pub global_value: OperandKind,

    /// A reference to a function signature declared in the function preamble.
    /// This is used to provide the call signature in a call_indirect instruction.
    pub sig_ref: OperandKind,

    /// A reference to an external function declared in the function preamble.
    /// This is used to provide the callee and signature in a call instruction.
    pub func_ref: OperandKind,

    /// A reference to a jump table declared in the function preamble.
    pub jump_table: OperandKind,

    /// A reference to a heap declared in the function preamble.
    pub heap: OperandKind,

    /// A reference to a table declared in the function preamble.
    pub table: OperandKind,

    /// A variable-sized list of value operands. Use for Ebb and function call arguments.
    pub varargs: OperandKind,
}

impl EntityRefs {
    pub fn new() -> Self {
        Self {
            ebb: create("ebb", "An extended basic block in the same function.")
                .default_member("destination")
                .build(),

            stack_slot: create("stack_slot", "A stack slot").build(),

            global_value: create("global_value", "A global value.").build(),

            sig_ref: create("sig_ref", "A function signature.").build(),

            func_ref: create("func_ref", "An external function.").build(),

            jump_table: create("jump_table", "A jump table.")
                .default_member("table")
                .build(),

            heap: create("heap", "A heap.").build(),

            table: create("table", "A table.").build(),

            varargs: Builder::new("variable_args", OperandKindFields::VariableArgs)
                .doc(
                    r#"
                        A variable size list of `value` operands.

                        Use this to represent arguments passed to a function call, arguments
                        passed to an extended basic block, or a variable number of results
                        returned from an instruction.
                    "#,
                )
                .build(),
        }
    }
}

/// Small helper to initialize an OperandBuilder with the right kind, for a given name and doc.
fn create(name: &'static str, doc: &'static str) -> Builder {
    Builder::new(name, OperandKindFields::EntityRef).doc(doc)
}
