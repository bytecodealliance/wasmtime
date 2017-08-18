//! Intermediate representation of a function.
//!
//! The `Function` struct defined in this module owns all of its extended basic blocks and
//! instructions.

use entity::{PrimaryMap, EntityMap};
use ir;
use ir::{FunctionName, CallConv, Signature, DataFlowGraph, Layout};
use ir::{InstEncodings, ValueLocations, JumpTables, StackSlots, EbbOffsets};
use isa::TargetIsa;
use std::fmt;
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

    /// Global variables referenced.
    pub global_vars: PrimaryMap<ir::GlobalVar, ir::GlobalVarData>,

    /// Heaps referenced.
    pub heaps: PrimaryMap<ir::Heap, ir::HeapData>,

    /// Jump tables used in this function.
    pub jump_tables: JumpTables,

    /// Data flow graph containing the primary definition of all instructions, EBBs and values.
    pub dfg: DataFlowGraph,

    /// Layout of EBBs and instructions in the function body.
    pub layout: Layout,

    /// Encoding recipe and bits for the legal instructions.
    /// Illegal instructions have the `Encoding::default()` value.
    pub encodings: InstEncodings,

    /// Location assigned to every value.
    pub locations: ValueLocations,

    /// Code offsets of the EBB headers.
    ///
    /// This information is only transiently available after the `binemit::relax_branches` function
    /// computes it, and it can easily be recomputed by calling that function. It is not included
    /// in the textual IL format.
    pub offsets: EbbOffsets,
}

impl Function {
    /// Create a function with the given name and signature.
    pub fn with_name_signature(name: FunctionName, sig: Signature) -> Function {
        Function {
            name,
            signature: sig,
            stack_slots: StackSlots::new(),
            global_vars: PrimaryMap::new(),
            heaps: PrimaryMap::new(),
            jump_tables: PrimaryMap::new(),
            dfg: DataFlowGraph::new(),
            layout: Layout::new(),
            encodings: EntityMap::new(),
            locations: EntityMap::new(),
            offsets: EntityMap::new(),
        }
    }

    /// Create a new empty, anonymous function with a native calling convention.
    pub fn new() -> Function {
        Self::with_name_signature(FunctionName::default(), Signature::new(CallConv::Native))
    }

    /// Return an object that can display this function with correct ISA-specific annotations.
    pub fn display<'a, I: Into<Option<&'a TargetIsa>>>(&'a self, isa: I) -> DisplayFunction<'a> {
        DisplayFunction(self, isa.into())
    }
}

/// Wrapper type capable of displaying a `Function` with correct ISA annotations.
pub struct DisplayFunction<'a>(&'a Function, Option<&'a TargetIsa>);

impl<'a> fmt::Display for DisplayFunction<'a> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write_function(fmt, self.0, self.1)
    }
}

impl fmt::Display for Function {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write_function(fmt, self, None)
    }
}

impl fmt::Debug for Function {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write_function(fmt, self, None)
    }
}
