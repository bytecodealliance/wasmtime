//! Helper functions for generating dummy instructions.

use ir::{Function, Ebb, Inst, Opcode};
use ir::entities::NO_VALUE;
use ir::instructions::{InstructionData, ReturnData, VariableArgs, JumpData, BranchData};
use ir::types;

pub fn jump(func: &mut Function, dest: Ebb) -> Inst {
    func.dfg.make_inst(InstructionData::Jump {
        opcode: Opcode::Jump,
        ty: types::VOID,
        data: Box::new(JumpData {
            destination: dest,
            varargs: VariableArgs::new(),
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
            varargs: VariableArgs::new(),
        }),
    })
}

pub fn ret(func: &mut Function) -> Inst {
    func.dfg.make_inst(InstructionData::Return {
        opcode: Opcode::Return,
        ty: types::VOID,
        data: Box::new(ReturnData { varargs: VariableArgs::new() }),
    })
}
