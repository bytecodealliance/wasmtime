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

pub use ir::types::{Type, FunctionName, Signature};
pub use ir::entities::{Ebb, Inst, Value, StackSlot, JumpTable};
pub use ir::instructions::{Opcode, InstructionData};
pub use ir::stackslot::StackSlotData;
pub use ir::jumptable::JumpTableData;
pub use ir::dfg::{DataFlowGraph, ValueDef};
pub use ir::layout::Layout;
pub use ir::function::Function;
