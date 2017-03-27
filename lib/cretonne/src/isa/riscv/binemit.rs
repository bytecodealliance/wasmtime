//! Emitting binary RISC-V machine code.

use binemit::{CodeSink, bad_encoding};
use ir::{Function, Inst, InstructionData};

include!(concat!(env!("OUT_DIR"), "/binemit-riscv.rs"));

/// R-type instructions.
///
///   31     24  19  14     11 6
///   funct7 rs2 rs1 funct3 rd opcode
///       25  20  15     12  7      0
///
/// Encoding bits: `opcode[6:2] | (funct3 << 5) | (funct7 << 8)`.
fn recipe_r<CS: CodeSink + ?Sized>(func: &Function, inst: Inst, sink: &mut CS) {
    if let InstructionData::Binary { args, .. } = func.dfg[inst] {
        let bits = func.encodings[inst].bits();
        let rs1 = func.locations[args[0]].unwrap_reg();
        let rs2 = func.locations[args[1]].unwrap_reg();
        let rd = func.locations[func.dfg.first_result(inst)].unwrap_reg();

        // 0-6: opcode
        let mut i = 0x3;
        i |= (bits as u32 & 0x1f) << 2;
        // 7-11: rd
        i |= (rd as u32 & 0x1f) << 7;
        // 12-14: funct3
        i |= ((bits as u32 >> 5) & 0x7) << 12;
        // 15-19: rs1
        i |= (rs1 as u32 & 0x1f) << 15;
        // 20-24: rs1
        i |= (rs2 as u32 & 0x1f) << 20;
        // 25-31: funct7
        i |= ((bits as u32 >> 8) & 0x7f) << 25;

        sink.put4(i);
    } else {
        panic!("Expected Binary format: {:?}", func.dfg[inst]);
    }
}

fn recipe_rshamt<CS: CodeSink + ?Sized>(_func: &Function, _inst: Inst, _sink: &mut CS) {
    unimplemented!()
}

fn recipe_i<CS: CodeSink + ?Sized>(_func: &Function, _inst: Inst, _sink: &mut CS) {
    unimplemented!()
}

fn recipe_iret<CS: CodeSink + ?Sized>(_func: &Function, _inst: Inst, _sink: &mut CS) {
    unimplemented!()
}
