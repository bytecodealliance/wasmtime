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
mod funcname;
mod extfunc;
mod builder;
mod valueloc;

pub use ir::funcname::FunctionName;
pub use ir::extfunc::{Signature, ArgumentType, ArgumentExtension, ExtFuncData};
pub use ir::types::Type;
pub use ir::entities::{Ebb, Inst, Value, StackSlot, JumpTable, FuncRef, SigRef};
pub use ir::instructions::{Opcode, InstructionData, VariableArgs};
pub use ir::stackslot::StackSlotData;
pub use ir::jumptable::JumpTableData;
pub use ir::valueloc::ValueLoc;
pub use ir::dfg::{DataFlowGraph, ValueDef};
pub use ir::layout::{Layout, Cursor};
pub use ir::function::Function;
pub use ir::builder::InstBuilder;
