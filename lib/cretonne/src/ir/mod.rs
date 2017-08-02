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
mod memflags;
mod progpoint;
mod valueloc;

pub use ir::funcname::FunctionName;
pub use ir::extfunc::{Signature, CallConv, ArgumentType, ArgumentExtension, ArgumentPurpose,
                      ExtFuncData};
pub use ir::types::Type;
pub use ir::entities::{Ebb, Inst, Value, StackSlot, JumpTable, FuncRef, SigRef};
pub use ir::instructions::{Opcode, InstructionData, VariableArgs, ValueList, ValueListPool};
pub use ir::stackslot::{StackSlots, StackSlotKind, StackSlotData};
pub use ir::jumptable::JumpTableData;
pub use ir::valueloc::{ValueLoc, ArgumentLoc};
pub use ir::dfg::{DataFlowGraph, ValueDef};
pub use ir::layout::{Layout, Cursor};
pub use ir::function::Function;
pub use ir::builder::InstBuilder;
pub use ir::progpoint::{ProgramPoint, ProgramOrder, ExpandedProgramPoint};
pub use ir::memflags::MemFlags;
pub use ir::builder::InstBuilderBase;

use binemit;
use entity_map::EntityMap;
use isa;

/// Map of value locations.
pub type ValueLocations = EntityMap<Value, ValueLoc>;

/// Map of jump tables.
pub type JumpTables = EntityMap<JumpTable, JumpTableData>;

/// Map of instruction encodings.
pub type InstEncodings = EntityMap<Inst, isa::Encoding>;

/// Code offsets for EBBs.
pub type EbbOffsets = EntityMap<Ebb, binemit::CodeOffset>;
