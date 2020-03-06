//! Fold operations on constants at compile time.
#![allow(clippy::float_arithmetic)]

use cranelift_codegen::{
    cursor::{Cursor, FuncCursor},
    ir::{self, dfg::ValueDef, InstBuilder},
};
// use rustc_apfloat::{
//     ieee::{Double, Single},
//     Float,
// };

enum ConstImm {
    Bool(bool),
    I64(i64),
    Ieee32(f32), // Ieee32 and Ieee64 will be replaced with `Single` and `Double` from the rust_apfloat library eventually.
    Ieee64(f64),
}

impl ConstImm {
    fn unwrap_i64(self) -> i64 {
        if let Self::I64(imm) = self {
            imm
        } else {
            panic!("self did not contain an `i64`.")
        }
    }

    fn evaluate_truthiness(self) -> bool {
        match self {
            Self::Bool(b) => b,
            Self::I64(imm) => imm != 0,
            _ => panic!(
                "Only a `ConstImm::Bool` and `ConstImm::I64` can be evaluated for \"truthiness\""
            ),
        }
    }
}

/// Fold operations on constants.
///
/// It's important to note that this will not remove unused constants. It's
/// assumed that the DCE pass will take care of them.
pub fn fold_constants(func: &mut ir::Function) {
    let mut pos = FuncCursor::new(func);

    while let Some(_block) = pos.next_block() {
        while let Some(inst) = pos.next_inst() {
            use self::ir::InstructionData::*;
            match pos.func.dfg[inst] {
                Binary { opcode, args } => {
                    fold_binary(&mut pos.func.dfg, inst, opcode, args);
                }
                Unary { opcode, arg } => {
                    fold_unary(&mut pos.func.dfg, inst, opcode, arg);
                }
                Branch { opcode, .. } => {
                    fold_branch(&mut pos, inst, opcode);
                }
                _ => {}
            }
        }
    }
}

fn resolve_value_to_imm(dfg: &ir::DataFlowGraph, value: ir::Value) -> Option<ConstImm> {
    let original = dfg.resolve_aliases(value);

    let inst = match dfg.value_def(original) {
        ValueDef::Result(inst, _) => inst,
        ValueDef::Param(_, _) => return None,
    };

    use self::ir::{InstructionData::*, Opcode::*};
    match dfg[inst] {
        UnaryImm {
            opcode: Iconst,
            imm,
        } => Some(ConstImm::I64(imm.into())),
        UnaryIeee32 {
            opcode: F32const,
            imm,
        } => {
            // See https://doc.rust-lang.org/std/primitive.f32.html#method.from_bits for caveats.
            let ieee_f32 = f32::from_bits(imm.bits());
            Some(ConstImm::Ieee32(ieee_f32))
        }
        UnaryIeee64 {
            opcode: F64const,
            imm,
        } => {
            // See https://doc.rust-lang.org/std/primitive.f32.html#method.from_bits for caveats.
            let ieee_f64 = f64::from_bits(imm.bits());
            Some(ConstImm::Ieee64(ieee_f64))
        }
        UnaryBool {
            opcode: Bconst,
            imm,
        } => Some(ConstImm::Bool(imm)),
        _ => None,
    }
}

fn evaluate_binary(opcode: ir::Opcode, imm0: ConstImm, imm1: ConstImm) -> Option<ConstImm> {
    use core::num::Wrapping;

    match opcode {
        ir::Opcode::Iadd => {
            let imm0 = Wrapping(imm0.unwrap_i64());
            let imm1 = Wrapping(imm1.unwrap_i64());
            Some(ConstImm::I64((imm0 + imm1).0))
        }
        ir::Opcode::Isub => {
            let imm0 = Wrapping(imm0.unwrap_i64());
            let imm1 = Wrapping(imm1.unwrap_i64());
            Some(ConstImm::I64((imm0 - imm1).0))
        }
        ir::Opcode::Imul => {
            let imm0 = Wrapping(imm0.unwrap_i64());
            let imm1 = Wrapping(imm1.unwrap_i64());
            Some(ConstImm::I64((imm0 * imm1).0))
        }
        ir::Opcode::Udiv => {
            let imm0 = Wrapping(imm0.unwrap_i64());
            let imm1 = Wrapping(imm1.unwrap_i64());
            if imm1.0 == 0 {
                panic!("Cannot divide by a zero.")
            }
            Some(ConstImm::I64((imm0 / imm1).0))
        }
        ir::Opcode::Fadd => match (imm0, imm1) {
            (ConstImm::Ieee32(imm0), ConstImm::Ieee32(imm1)) => Some(ConstImm::Ieee32(imm0 + imm1)),
            (ConstImm::Ieee64(imm0), ConstImm::Ieee64(imm1)) => Some(ConstImm::Ieee64(imm0 + imm1)),
            _ => unreachable!(),
        },
        ir::Opcode::Fsub => match (imm0, imm1) {
            (ConstImm::Ieee32(imm0), ConstImm::Ieee32(imm1)) => Some(ConstImm::Ieee32(imm0 - imm1)),
            (ConstImm::Ieee64(imm0), ConstImm::Ieee64(imm1)) => Some(ConstImm::Ieee64(imm0 - imm1)),
            _ => unreachable!(),
        },
        ir::Opcode::Fmul => match (imm0, imm1) {
            (ConstImm::Ieee32(imm0), ConstImm::Ieee32(imm1)) => Some(ConstImm::Ieee32(imm0 * imm1)),
            (ConstImm::Ieee64(imm0), ConstImm::Ieee64(imm1)) => Some(ConstImm::Ieee64(imm0 * imm1)),
            _ => unreachable!(),
        },
        ir::Opcode::Fdiv => match (imm0, imm1) {
            (ConstImm::Ieee32(imm0), ConstImm::Ieee32(imm1)) => Some(ConstImm::Ieee32(imm0 / imm1)),
            (ConstImm::Ieee64(imm0), ConstImm::Ieee64(imm1)) => Some(ConstImm::Ieee64(imm0 / imm1)),
            _ => unreachable!(),
        },
        _ => None,
    }
}

fn evaluate_unary(opcode: ir::Opcode, imm: ConstImm) -> Option<ConstImm> {
    match opcode {
        ir::Opcode::Fneg => match imm {
            ConstImm::Ieee32(imm) => Some(ConstImm::Ieee32(-imm)),
            ConstImm::Ieee64(imm) => Some(ConstImm::Ieee64(-imm)),
            _ => unreachable!(),
        },
        ir::Opcode::Fabs => match imm {
            ConstImm::Ieee32(imm) => Some(ConstImm::Ieee32(imm.abs())),
            ConstImm::Ieee64(imm) => Some(ConstImm::Ieee64(imm.abs())),
            _ => unreachable!(),
        },
        _ => None,
    }
}

fn replace_inst(dfg: &mut ir::DataFlowGraph, inst: ir::Inst, const_imm: ConstImm) {
    use self::ConstImm::*;
    match const_imm {
        I64(imm) => {
            let typevar = dfg.ctrl_typevar(inst);
            dfg.replace(inst).iconst(typevar, imm);
        }
        Ieee32(imm) => {
            dfg.replace(inst)
                .f32const(ir::immediates::Ieee32::with_bits(imm.to_bits()));
        }
        Ieee64(imm) => {
            dfg.replace(inst)
                .f64const(ir::immediates::Ieee64::with_bits(imm.to_bits()));
        }
        Bool(imm) => {
            let typevar = dfg.ctrl_typevar(inst);
            dfg.replace(inst).bconst(typevar, imm);
        }
    }
}

/// Fold a binary instruction.
fn fold_binary(
    dfg: &mut ir::DataFlowGraph,
    inst: ir::Inst,
    opcode: ir::Opcode,
    args: [ir::Value; 2],
) {
    let (imm0, imm1) = if let (Some(imm0), Some(imm1)) = (
        resolve_value_to_imm(dfg, args[0]),
        resolve_value_to_imm(dfg, args[1]),
    ) {
        (imm0, imm1)
    } else {
        return;
    };

    if let Some(const_imm) = evaluate_binary(opcode, imm0, imm1) {
        replace_inst(dfg, inst, const_imm);
    }
}

/// Fold a unary instruction.
fn fold_unary(dfg: &mut ir::DataFlowGraph, inst: ir::Inst, opcode: ir::Opcode, arg: ir::Value) {
    let imm = if let Some(imm) = resolve_value_to_imm(dfg, arg) {
        imm
    } else {
        return;
    };

    if let Some(const_imm) = evaluate_unary(opcode, imm) {
        replace_inst(dfg, inst, const_imm);
    }
}

fn fold_branch(pos: &mut FuncCursor, inst: ir::Inst, opcode: ir::Opcode) {
    let (cond, block, args) = {
        let values = pos.func.dfg.inst_args(inst);
        let inst_data = &pos.func.dfg[inst];
        (
            match resolve_value_to_imm(&pos.func.dfg, values[0]) {
                Some(imm) => imm,
                None => return,
            },
            inst_data.branch_destination().unwrap(),
            values[1..].to_vec(),
        )
    };

    let truthiness = cond.evaluate_truthiness();
    let branch_if_zero = match opcode {
        ir::Opcode::Brz => true,
        ir::Opcode::Brnz => false,
        _ => unreachable!(),
    };

    if (branch_if_zero && !truthiness) || (!branch_if_zero && truthiness) {
        pos.func.dfg.replace(inst).jump(block, &args);
        // remove the rest of the block to avoid verifier errors
        while let Some(next_inst) = pos.func.layout.next_inst(inst) {
            pos.func.layout.remove_inst(next_inst);
        }
    } else {
        pos.remove_inst_and_step_back();
    }
}
