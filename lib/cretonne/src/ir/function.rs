//! Intermediate representation of a function.
//!
//! The `Function` struct defined in this module owns all of its extended basic blocks and
//! instructions.

use binemit::CodeOffset;
use entity::{PrimaryMap, EntityMap};
use ir;
use ir::{ExternalName, CallConv, Signature, DataFlowGraph, Layout};
use ir::{InstEncodings, ValueLocations, JumpTables, StackSlots, EbbOffsets, SourceLocs};
use ir::{Ebb, JumpTableData, JumpTable, StackSlotData, StackSlot, SigRef, ExtFuncData, FuncRef,
         GlobalVarData, GlobalVar, HeapData, Heap};
use isa::{TargetIsa, EncInfo};
use std::fmt;
use write::write_function;

/// A function.
///
/// Functions can be cloned, but it is not a very fast operation.
/// The clone will have all the same entity numbers as the original.
#[derive(Clone)]
pub struct Function {
    /// Name of this function. Mostly used by `.cton` files.
    pub name: ExternalName,

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

    /// Source locations.
    ///
    /// Track the original source location for each instruction. The source locations are not
    /// interpreted by Cretonne, only preserved.
    pub srclocs: SourceLocs,
}

impl Function {
    /// Create a function with the given name and signature.
    pub fn with_name_signature(name: ExternalName, sig: Signature) -> Self {
        Self {
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
            srclocs: EntityMap::new(),
        }
    }

    /// Clear all data structures in this function.
    pub fn clear(&mut self) {
        self.signature.clear(ir::CallConv::Native);
        self.stack_slots.clear();
        self.global_vars.clear();
        self.heaps.clear();
        self.jump_tables.clear();
        self.dfg.clear();
        self.layout.clear();
        self.encodings.clear();
        self.locations.clear();
        self.offsets.clear();
        self.srclocs.clear();
    }

    /// Create a new empty, anonymous function with a native calling convention.
    pub fn new() -> Self {
        Self::with_name_signature(ExternalName::default(), Signature::new(CallConv::Native))
    }

    /// Creates a jump table in the function, to be used by `br_table` instructions.
    pub fn create_jump_table(&mut self, data: JumpTableData) -> JumpTable {
        self.jump_tables.push(data)
    }

    /// Inserts an entry in a previously declared jump table.
    pub fn insert_jump_table_entry(&mut self, jt: JumpTable, index: usize, ebb: Ebb) {
        self.jump_tables[jt].set_entry(index, ebb);
    }

    /// Creates a stack slot in the function, to be used by `stack_load`, `stack_store` and
    /// `stack_addr` instructions.
    pub fn create_stack_slot(&mut self, data: StackSlotData) -> StackSlot {
        self.stack_slots.push(data)
    }

    /// Adds a signature which can later be used to declare an external function import.
    pub fn import_signature(&mut self, signature: Signature) -> SigRef {
        self.dfg.signatures.push(signature)
    }

    /// Declare an external function import.
    pub fn import_function(&mut self, data: ExtFuncData) -> FuncRef {
        self.dfg.ext_funcs.push(data)
    }

    /// Declares a global variable accessible to the function.
    pub fn create_global_var(&mut self, data: GlobalVarData) -> GlobalVar {
        self.global_vars.push(data)
    }

    /// Declares a heap accessible to the function.
    pub fn create_heap(&mut self, data: HeapData) -> Heap {
        self.heaps.push(data)
    }

    /// Return an object that can display this function with correct ISA-specific annotations.
    pub fn display<'a, I: Into<Option<&'a TargetIsa>>>(&'a self, isa: I) -> DisplayFunction<'a> {
        DisplayFunction(self, isa.into())
    }

    /// Find a presumed unique special-purpose function parameter value.
    ///
    /// Returns the value of the last `purpose` parameter, or `None` if no such parameter exists.
    pub fn special_param(&self, purpose: ir::ArgumentPurpose) -> Option<ir::Value> {
        let entry = self.layout.entry_block().expect("Function is empty");
        self.signature.special_param_index(purpose).map(|i| {
            self.dfg.ebb_params(entry)[i]
        })
    }

    /// Get an iterator over the instructions in `ebb`, including offsets and encoded instruction
    /// sizes.
    ///
    /// The iterator returns `(offset, inst, size)` tuples, where `offset` if the offset in bytes
    /// from the beginning of the function to the instruction, and `size` is the size of the
    /// instruction in bytes, or 0 for unencoded instructions.
    ///
    /// This function can only be used after the code layout has been computed by the
    /// `binemit::relax_branches()` function.
    pub fn inst_offsets<'a>(&'a self, ebb: Ebb, encinfo: &EncInfo) -> InstOffsetIter<'a> {
        assert!(
            !self.offsets.is_empty(),
            "Code layout must be computed first"
        );
        InstOffsetIter {
            encinfo: encinfo.clone(),
            encodings: &self.encodings,
            offset: self.offsets[ebb],
            iter: self.layout.ebb_insts(ebb),
        }
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

/// Iterator returning instruction offsets and sizes: `(offset, inst, size)`.
pub struct InstOffsetIter<'a> {
    encinfo: EncInfo,
    encodings: &'a InstEncodings,
    offset: CodeOffset,
    iter: ir::layout::Insts<'a>,
}

impl<'a> Iterator for InstOffsetIter<'a> {
    type Item = (CodeOffset, ir::Inst, CodeOffset);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|inst| {
            let size = self.encinfo.bytes(self.encodings[inst]);
            let offset = self.offset;
            self.offset += size;
            (offset, inst, size)
        })
    }
}
