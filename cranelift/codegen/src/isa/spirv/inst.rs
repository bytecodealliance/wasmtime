
use alloc::boxed::Box;
use alloc::string::String;

use regalloc::RealRegUniverse;
use target_lexicon::Triple;

use crate::ir::Function;
use crate::isa::Builder as IsaBuilder;
use crate::machinst::pretty_print::ShowWithRRU;
use crate::machinst::{compile, MachBackend, MachCompileResult, TargetIsaAdapter, VCode};
use crate::settings::{self, Flags};
use crate::result::{CodegenResult, CodegenError};
use crate::machinst::{MachBuffer, MachInst, MachInstEmit};
use crate::machinst::MachTerminator;
use crate::machinst::buffer::MachLabel;
use crate::ir::types::Type;
use crate::binemit::CodeOffset;
use crate::machinst::MachInstLabelUse;
use crate::ir::types::{B1, B128, B16, B32, B64, B8, F32, F64, I128, I16, I32, I64, I8};

use regalloc::NUM_REG_CLASSES;
use regalloc::RegUsageCollector;
use regalloc::RegUsageMapper;
use regalloc::Writable;
use regalloc::Reg;
use regalloc::SpillSlot;
use regalloc::RegClass;
use regalloc::VirtualReg;

use smallvec::SmallVec;
use target_lexicon::Architecture;
use spirv_headers::Op;
use rspirv::dr::Operand;
use rspirv::binary::Assemble;
use alloc::vec::Vec;

#[derive(Clone, Debug)]
pub(crate) struct Inst {
    op: Op,
    result_type: Option<u32>,
    result_id: Option<u32>,
    operands: Vec<Operand>,
}

impl Inst {
    pub(crate) fn new(op: Op, result_type: Option<u32>, result_id: Option<u32>, operands: Vec<Operand>) -> Self {
        Self {
            op, result_type, result_id, operands
        }
    }
}

#[derive(Debug, Default, Clone)]
pub(crate) struct MachInstEmitState {

}

impl MachInstEmit for Inst {
    type State = MachInstEmitState;

    fn emit(&self, sink: &mut MachBuffer<Self>, flags: &Flags, state: &mut Self::State) {
        sink.put4(0x07230203); // SPIR-V magic header
        sink.put4(0x05); // HLSL, but not really - needs SPIR-V update
        sink.put4(5); // GLCompute
        sink.put4(0); // Logical addressing mode
        sink.put4(1); // GLSL450
        sink.put4(0x00020011); // OpCapability 
        sink.put4(0x1); //  Shader

        let mut operands = vec![];
        
        for operand in &self.operands {
            operands.extend(operand.assemble());
        }
        
        let mut opcode_len = 1 + operands.len() as u32;
        if let Some(r) = self.result_type {
            opcode_len += 1;
        }

        if let Some(r) = self.result_id {
            opcode_len += 1;
        }

        sink.put4((self.op as u32) | (opcode_len << 16));

        if let Some(r) = self.result_type {
            sink.put4(r);
        }
        if let Some(r) = self.result_id {
            sink.put4(r);
        }
        for op in operands {
            sink.put4(op);
        }
    }
}


impl MachInst for Inst {
    fn get_regs(&self, collector: &mut RegUsageCollector) {}

    fn map_regs<RUM: RegUsageMapper>(&mut self, maps: &RUM) {}

    fn is_move(&self) -> Option<(Writable<Reg>, Reg)> {
        None
    }

    fn is_term<'a>(&'a self) -> MachTerminator<'a> {
        MachTerminator::None
    }

    fn is_epilogue_placeholder(&self) -> bool {
        false
    }

    fn gen_move(to_reg: Writable<Reg>, from_reg: Reg, ty: Type) -> Self {
        dbg!(to_reg);
        dbg!(from_reg);
        Inst::new(Op::Nop, None, None, vec![])
    }

    fn gen_constant(to_reg: Writable<Reg>, value: u64, ty: Type) -> SmallVec<[Self; 4]> {
        dbg!(to_reg);
        dbg!(value);
        SmallVec::new()
    }

    fn gen_zero_len_nop() -> Self {
        Inst::new(Op::Nop, None, None, vec![])
    }

    fn maybe_direct_reload(&self, reg: VirtualReg, slot: SpillSlot) -> Option<Self> {
        dbg!(reg);
        dbg!(slot);
        None
    }

    fn rc_for_type(ty: Type) -> CodegenResult<RegClass> {
        match ty {
            I8 | I16 | I32 | B1 | B8 | B16| B32 | F32 => Ok(RegClass::F32),
            F64 | B64 | I64 => Ok(RegClass::I64),
            _ => Err(CodegenError::Unsupported(format!(
                "Unexpected SSA-value type: {}",
                ty
            ))),
        }
    }


    fn gen_jump(target: MachLabel) -> Self {
        dbg!(target);
        Inst::new(Op::Nop, None, None, vec![])
    }

    fn gen_nop(preferred_size: usize) -> Self {
        dbg!(preferred_size);
        Inst::new(Op::Nop, None, None, vec![])
    }

    fn reg_universe(flags: &Flags) -> RealRegUniverse {
        RealRegUniverse {
            regs: vec![],
            allocable: 0,
            allocable_by_class: [None; NUM_REG_CLASSES],
        }
    }

    fn worst_case_size() -> CodeOffset {
        0
    }

    type LabelUse = LabelUse;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LabelUse {
    Standard
}

impl MachInstLabelUse for LabelUse {
    const ALIGN: CodeOffset = 4;

    fn max_pos_range(self) -> CodeOffset {
        0
    }

    fn max_neg_range(self) -> CodeOffset {
        0
    }

    fn patch_size(self) -> CodeOffset {
        4
    }

    fn patch(self, buffer: &mut [u8], use_offset: CodeOffset, label_offset: CodeOffset) {
        
    }

    fn supports_veneer(self) -> bool {
        false
    }

    fn veneer_size(self) -> CodeOffset {
        4
    }

    fn generate_veneer(
        self,
        buffer: &mut [u8],
        veneer_offset: CodeOffset,
    ) -> (CodeOffset, LabelUse) {
        (0, Self::Standard)
    }
}

impl ShowWithRRU for Inst {
    fn show_rru(&self, mb_rru: Option<&RealRegUniverse>) -> String {
        format!("nop")
    }
}