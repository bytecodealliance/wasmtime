use crate::cdsl::operands::{OperandKind, OperandKindBuilder as Builder, OperandKindFields};

pub(crate) struct EntityRefs {
    /// A reference to an extended basic block in the same function.
    /// This is primarliy used in control flow instructions.
    pub(crate) ebb: OperandKind,

    /// A reference to a stack slot declared in the function preamble.
    pub(crate) stack_slot: OperandKind,

    /// A reference to a global value.
    pub(crate) global_value: OperandKind,

    /// A reference to a function signature declared in the function preamble.
    /// This is used to provide the call signature in a call_indirect instruction.
    pub(crate) sig_ref: OperandKind,

    /// A reference to an external function declared in the function preamble.
    /// This is used to provide the callee and signature in a call instruction.
    pub(crate) func_ref: OperandKind,

    /// A reference to a jump table declared in the function preamble.
    pub(crate) jump_table: OperandKind,

    /// A reference to a heap declared in the function preamble.
    pub(crate) heap: OperandKind,

    /// A reference to a table declared in the function preamble.
    pub(crate) table: OperandKind,

    /// A variable-sized list of value operands. Use for Ebb and function call arguments.
    pub(crate) varargs: OperandKind,
}

impl EntityRefs {
    pub fn new() -> Self {
        Self {
            ebb: create(
                "destination",
                "ir::Ebb",
                "An extended basic block in the same function.",
            )
            .build(),

            stack_slot: create("stack_slot", "ir::StackSlot", "A stack slot").build(),

            global_value: create("global_value", "ir::GlobalValue", "A global value.").build(),

            sig_ref: create("sig_ref", "ir::SigRef", "A function signature.").build(),

            func_ref: create("func_ref", "ir::FuncRef", "An external function.").build(),

            jump_table: create("table", "ir::JumpTable", "A jump table.").build(),

            heap: create("heap", "ir::Heap", "A heap.").build(),

            table: create("table", "ir::Table", "A table.").build(),

            varargs: Builder::new("", "&[Value]", OperandKindFields::VariableArgs)
                .with_doc(
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
fn create(format_field_name: &'static str, rust_type: &'static str, doc: &'static str) -> Builder {
    Builder::new(format_field_name, rust_type, OperandKindFields::EntityRef).with_doc(doc)
}
