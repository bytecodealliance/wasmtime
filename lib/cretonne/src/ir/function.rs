//! Intermediate representation of a function.
//!
//! The `Function` struct defined in this module owns all of its extended basic blocks and
//! instructions.

use binemit::CodeOffset;
use entity_map::{EntityMap, PrimaryEntityData};
use ir::{FunctionName, Signature, Value, Inst, Ebb, StackSlots, JumpTable, JumpTableData,
         ValueLoc, DataFlowGraph, Layout};
use isa::{TargetIsa, Encoding};
use std::fmt::{self, Display, Debug, Formatter};
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
    pub stack_slots: StackSlots,

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

    /// Code offsets of the EBB headers.
    ///
    /// This information is only transiently available after the `binemit::relax_branches` function
    /// computes it, and it can easily be recomputed by calling that function. It is not included
    /// in the textual IL format.
    pub offsets: EntityMap<Ebb, CodeOffset>,
}

impl PrimaryEntityData for JumpTableData {}

impl Function {
    /// Create a function with the given name and signature.
    pub fn with_name_signature(name: FunctionName, sig: Signature) -> Function {
        Function {
            name,
            signature: sig,
            stack_slots: StackSlots::new(),
            jump_tables: EntityMap::new(),
            dfg: DataFlowGraph::new(),
            layout: Layout::new(),
            encodings: EntityMap::new(),
            locations: EntityMap::new(),
            offsets: EntityMap::new(),
        }
    }

    /// Create a new empty, anonymous function.
    pub fn new() -> Function {
        Self::with_name_signature(FunctionName::default(), Signature::new())
    }

    /// Return an object that can display this function with correct ISA-specific annotations.
    pub fn display<'a, I: Into<Option<&'a TargetIsa>>>(&'a self, isa: I) -> DisplayFunction<'a> {
        DisplayFunction(self, isa.into())
    }
}

/// Wrapper type capable of displaying a `Function` with correct ISA annotations.
pub struct DisplayFunction<'a>(&'a Function, Option<&'a TargetIsa>);

impl<'a> Display for DisplayFunction<'a> {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        write_function(fmt, self.0, self.1)
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
