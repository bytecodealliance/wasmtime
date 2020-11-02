//! The [InstructionContext] trait describes a Cranelift instruction; a default implementation is
//! provided with [DfgInstructionContext]
use cranelift_codegen::ir::{DataFlowGraph, Inst, InstructionData, Type, Value};

/// Exposes the necessary information for understanding a single Cranelift instruction. It would be
/// nice if [InstructionData] contained everything necessary for interpreting the instruction, but
/// Cranelift's current design requires looking at other structures. A default implementation using
/// a reference to a [DataFlowGraph] is provided in [DfgInstructionContext].
pub trait InstructionContext {
    fn data(&self) -> InstructionData;
    fn args(&self) -> &[Value];
    fn type_of(&self, v: Value) -> Option<Type>;
    fn controlling_type(&self) -> Option<Type>;
}

/// Since [InstructionContext] is likely used within a Cranelift context in which a [DataFlowGraph]
/// is available, a default implementation is provided--[DfgInstructionContext].
pub struct DfgInstructionContext<'a>(Inst, &'a DataFlowGraph);

impl<'a> DfgInstructionContext<'a> {
    pub fn new(inst: Inst, dfg: &'a DataFlowGraph) -> Self {
        Self(inst, dfg)
    }
}

impl InstructionContext for DfgInstructionContext<'_> {
    fn data(&self) -> InstructionData {
        self.1[self.0].clone()
    }

    fn args(&self) -> &[Value] {
        self.1.inst_args(self.0)
    }

    fn type_of(&self, v: Value) -> Option<Type> {
        Some(self.1.value_type(v))
    }

    fn controlling_type(&self) -> Option<Type> {
        Some(self.1.ctrl_typevar(self.0))
    }
}
