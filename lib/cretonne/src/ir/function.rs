//! Intermediate representation of a function.
//!
//! The `Function` struct defined in this module owns all of its extended basic blocks and
//! instructions.

use std::fmt::{self, Display, Debug, Formatter};
use ir::{FunctionName, Signature, Value, Inst, StackSlot, StackSlotData, JumpTable, JumpTableData,
         ValueLoc, DataFlowGraph, Layout};
use isa::Encoding;
use entity_map::{EntityMap, PrimaryEntityData};
use write::write_function;

/// A function.
///
/// Functions can be cloned, but it is not a very fast operation.
/// The clone will have all the same entity numbers as the original.
#[derive(Clone)]
pub struct Function {
    /// Name of this function. Mostly used by `.cton` files.
    pub name: FunctionName,

    /// Signature of this function.
    pub signature: Signature,

    /// Stack slots allocated in this function.
    pub stack_slots: EntityMap<StackSlot, StackSlotData>,

    /// Jump tables used in this function.
    pub jump_tables: EntityMap<JumpTable, JumpTableData>,

    /// Data flow graph containing the primary definition of all instructions, EBBs and values.
    pub dfg: DataFlowGraph,

    /// Layout of EBBs and instructions in the function body.
    pub layout: Layout,

    /// Encoding recipe and bits for the legal instructions.
    /// Illegal instructions have the `Encoding::default()` value.
    pub encodings: EntityMap<Inst, Encoding>,

    /// Location assigned to every value.
    pub locations: EntityMap<Value, ValueLoc>,
}

impl PrimaryEntityData for StackSlotData {}
impl PrimaryEntityData for JumpTableData {}

impl Function {
    /// Create a function with the given name and signature.
    pub fn with_name_signature(name: FunctionName, sig: Signature) -> Function {
        Function {
            name: name,
            signature: sig,
            stack_slots: EntityMap::new(),
            jump_tables: EntityMap::new(),
            dfg: DataFlowGraph::new(),
            layout: Layout::new(),
            encodings: EntityMap::new(),
            locations: EntityMap::new(),
        }
    }

    /// Create a new empty, anonymous function.
    pub fn new() -> Function {
        Self::with_name_signature(FunctionName::default(), Signature::new())
    }
}

impl Display for Function {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        write_function(fmt, self, None)
    }
}

impl Debug for Function {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        write_function(fmt, self, None)
    }
}
