//! Representation of Cretonne IL functions.

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
mod funcname;
mod globalvar;
mod memflags;
mod progpoint;
mod valueloc;

pub use ir::builder::{InstBuilder, InstBuilderBase, InstInserterBase, InsertBuilder};
pub use ir::dfg::{DataFlowGraph, ValueDef};
pub use ir::entities::{Ebb, Inst, Value, StackSlot, GlobalVar, JumpTable, FuncRef, SigRef};
pub use ir::extfunc::{Signature, CallConv, ArgumentType, ArgumentExtension, ArgumentPurpose,
                      ExtFuncData};
pub use ir::funcname::FunctionName;
pub use ir::function::Function;
pub use ir::globalvar::GlobalVarData;
pub use ir::instructions::{Opcode, InstructionData, VariableArgs, ValueList, ValueListPool};
pub use ir::jumptable::JumpTableData;
pub use ir::layout::{Layout, CursorBase, Cursor};
pub use ir::memflags::MemFlags;
pub use ir::progpoint::{ProgramPoint, ProgramOrder, ExpandedProgramPoint};
pub use ir::stackslot::{StackSlots, StackSlotKind, StackSlotData};
pub use ir::types::Type;
pub use ir::valueloc::{ValueLoc, ArgumentLoc};

use binemit;
use entity::{PrimaryMap, EntityMap};
use isa;

/// Map of value locations.
pub type ValueLocations = EntityMap<Value, ValueLoc>;

/// Map of jump tables.
pub type JumpTables = PrimaryMap<JumpTable, JumpTableData>;

/// Map of instruction encodings.
pub type InstEncodings = EntityMap<Inst, isa::Encoding>;

/// Code offsets for EBBs.
pub type EbbOffsets = EntityMap<Ebb, binemit::CodeOffset>;
