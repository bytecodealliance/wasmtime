//! Representation of Cretonne IR functions.

pub mod types;
pub mod entities;
pub mod condcodes;
pub mod immediates;
pub mod instructions;
pub mod stackslot;
pub mod jumptable;
pub mod dfg;
pub mod layout;
pub mod function;
mod builder;
mod extfunc;
mod extname;
mod globalvar;
mod heap;
mod libcall;
mod memflags;
mod progpoint;
mod sourceloc;
mod trapcode;
mod valueloc;

pub use ir::builder::{InsertBuilder, InstBuilder, InstBuilderBase, InstInserterBase};
pub use ir::dfg::{DataFlowGraph, ValueDef};
pub use ir::entities::{Ebb, FuncRef, GlobalVar, Heap, Inst, JumpTable, SigRef, StackSlot, Value};
pub use ir::extfunc::{AbiParam, ArgumentExtension, ArgumentPurpose, CallConv, ExtFuncData,
                      Signature};
pub use ir::extname::ExternalName;
pub use ir::function::Function;
pub use ir::globalvar::GlobalVarData;
pub use ir::heap::{HeapBase, HeapData, HeapStyle};
pub use ir::instructions::{InstructionData, Opcode, ValueList, ValueListPool, VariableArgs};
pub use ir::jumptable::JumpTableData;
pub use ir::layout::Layout;
pub use ir::libcall::LibCall;
pub use ir::memflags::MemFlags;
pub use ir::progpoint::{ExpandedProgramPoint, ProgramOrder, ProgramPoint};
pub use ir::sourceloc::SourceLoc;
pub use ir::stackslot::{StackSlotData, StackSlotKind, StackSlots};
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
