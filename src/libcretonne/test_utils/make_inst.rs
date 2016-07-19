//! Helper functions for generating dummy instructions.

use repr::Function;
use repr::entities::{Ebb, Inst, NO_VALUE};
use repr::instructions::{InstructionData, Opcode, VariableArgs, JumpData, BranchData};
use repr::types;

pub fn jump(func: &mut Function, dest: Ebb) -> Inst {
    func.dfg.make_inst(InstructionData::Jump {
        opcode: Opcode::Jump,
        ty: types::VOID,
        data: Box::new(JumpData {
            destination: dest,
            arguments: VariableArgs::new(),
        }),
    })
}

pub fn branch(func: &mut Function, dest: Ebb) -> Inst {
    func.dfg.make_inst(InstructionData::Branch {
        opcode: Opcode::Brz,
        ty: types::VOID,
        data: Box::new(BranchData {
            arg: NO_VALUE,
            destination: dest,
            arguments: VariableArgs::new(),
        }),
    })
}
