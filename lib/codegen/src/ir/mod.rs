//! Representation of Cranelift IR functions.

mod builder;
pub mod condcodes;
pub mod dfg;
pub mod entities;
mod extfunc;
mod extname;
pub mod function;
mod globalvalue;
mod heap;
pub mod immediates;
pub mod instructions;
pub mod jumptable;
pub mod layout;
mod libcall;
mod memflags;
mod progpoint;
mod sourceloc;
pub mod stackslot;
mod table;
mod trapcode;
pub mod types;
mod valueloc;

pub use ir::builder::{InsertBuilder, InstBuilder, InstBuilderBase, InstInserterBase};
pub use ir::dfg::{DataFlowGraph, ValueDef};
pub use ir::entities::{
    Ebb, FuncRef, GlobalValue, Heap, Inst, JumpTable, SigRef, StackSlot, Table, Value,
};
pub use ir::extfunc::{AbiParam, ArgumentExtension, ArgumentPurpose, ExtFuncData, Signature};
pub use ir::extname::ExternalName;
pub use ir::function::Function;
pub use ir::globalvalue::GlobalValueData;
pub use ir::heap::{HeapData, HeapStyle};
pub use ir::instructions::{InstructionData, Opcode, ValueList, ValueListPool, VariableArgs};
pub use ir::jumptable::JumpTableData;
pub use ir::layout::Layout;
pub use ir::libcall::{get_libcall_funcref, get_probestack_funcref, LibCall};
pub use ir::memflags::MemFlags;
pub use ir::progpoint::{ExpandedProgramPoint, ProgramOrder, ProgramPoint};
pub use ir::sourceloc::SourceLoc;
pub use ir::stackslot::{StackSlotData, StackSlotKind, StackSlots};
pub use ir::table::TableData;
pub use ir::trapcode::TrapCode;
pub use ir::types::Type;
pub use ir::valueloc::{ArgumentLoc, ValueLoc};

use binemit;
use entity::{EntityMap, PrimaryMap};
use isa;

/// Map of value locations.
pub type ValueLocations = EntityMap<Value, ValueLoc>;

/// Map of jump tables.
pub type JumpTables = PrimaryMap<JumpTable, JumpTableData>;

/// Map of instruction encodings.
pub type InstEncodings = EntityMap<Inst, isa::Encoding>;

/// Code offsets for EBBs.
pub type EbbOffsets = EntityMap<Ebb, binemit::CodeOffset>;

/// Source locations for instructions.
pub type SourceLocs = EntityMap<Inst, SourceLoc>;
