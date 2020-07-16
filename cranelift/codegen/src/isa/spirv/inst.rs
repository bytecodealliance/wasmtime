use alloc::boxed::Box;
use alloc::string::String;

use regalloc::RealRegUniverse;
use target_lexicon::Triple;

use crate::binemit::CodeOffset;
use crate::ir::types::Type;
use crate::ir::types::{B1, B128, B16, B32, B64, B8, F32, F64, I128, I16, I32, I64, I8};
use crate::ir::Function;
use crate::isa::Builder as IsaBuilder;
use crate::machinst::abi::ABIBody;
use crate::machinst::buffer::MachLabel;
use crate::machinst::pretty_print::ShowWithRRU;
use crate::machinst::MachInstLabelUse;
use crate::machinst::{compile, MachBackend, MachCompileResult, TargetIsaAdapter, VCode};
use crate::machinst::{MachBuffer, MachInst, MachInstEmit};
use crate::machinst::{MachInstEmitState, MachTerminator};
use crate::result::{CodegenError, CodegenResult};
use crate::settings::{self, Flags};

use regalloc::Reg;
use regalloc::RegClass;
use regalloc::RegUsageCollector;
use regalloc::RegUsageMapper;
use regalloc::SpillSlot;
use regalloc::VirtualReg;
use regalloc::Writable;
use regalloc::NUM_REG_CLASSES;

use alloc::vec::Vec;
use rspirv::binary::Assemble;
use rspirv::dr::Operand;
use smallvec::SmallVec;
use spirv_headers::Op;
use target_lexicon::Architecture;

#[derive(Clone, Debug)]
pub(crate) struct Inst {
    op: Op,
    result_type: Option<u32>,
    result_id: Option<Writable<Reg>>,
    operands: Vec<Reg>,
}

impl Inst {
    pub(crate) fn new(
        op: Op,
        result_type: Option<u32>,
        result_id: Option<Writable<Reg>>,
        operands: Vec<Reg>,
    ) -> Self {
        Self {
            op,
            result_type,
            result_id,
            operands,
        }
    }

    pub(crate) fn type_void(result_id: Option<Writable<Reg>>) -> Inst {
        Inst::new(Op::TypeVoid, None, result_id, vec![])
    }
}

impl MachInstEmitState<Inst> for EmitState {
    fn new(_: &dyn ABIBody<I = Inst>) -> Self {
        EmitState {}
    }
}

#[derive(Debug, Default, Clone)]
pub(crate) struct EmitState {}

impl MachInstEmit for Inst {
    type State = EmitState;

    /// Unlike most ISA's, SPIR-V is relatively straight forward, all opcodes have the exact same
    /// binary representation. Since SPIR-V already has virtual registers, there is no real need
    /// for us to translate them (yet) so we just use the virtual register index from the CLIF
    /// as the SPIR-V register index for now.
    fn emit(&self, sink: &mut MachBuffer<Self>, flags: &Flags, state: &mut Self::State) {
        let mut operands = vec![];

        for operand in &self.operands {
            let idx = operand.get_index();
            operands.push(idx as u32);
        }

        let mut opcode_len = 1 + operands.len() as u32;
        if self.result_type.is_some() {
            opcode_len += 1;
        }

        if self.result_id.is_some() {
            opcode_len += 1;
        }

        sink.put4((self.op as u32) | (opcode_len << 16));

        if let Some(r) = self.result_type {
            sink.put4(r);
        }

        if let Some(r) = self.result_id {
            let idx = r.to_reg().get_index();
            sink.put4(idx as u32);
        }

        for op in operands {
            sink.put4(op);
        }
    }

    fn pretty_print(&self, mb_rru: Option<&RealRegUniverse>, state: &mut EmitState) -> String {
        use crate::alloc::string::ToString;
        "".to_string()
    }
}

impl MachInst for Inst {
    fn get_regs(&self, collector: &mut RegUsageCollector) {
        if let Some(result) = self.result_id {
            collector.add_def(result);
        }

        for reg in &self.operands {
            collector.add_use(*reg)
        }
    }

    fn ref_type_regclass(_: &settings::Flags) -> RegClass {
        RegClass::I64
    }

    fn map_regs<RUM: RegUsageMapper>(&mut self, maps: &RUM) {}

    fn is_move(&self) -> Option<(Writable<Reg>, Reg)> {
        None
    }

    fn is_term<'a>(&'a self) -> MachTerminator<'a> {
        match self.op {
            Op::Return | Op::ReturnValue => MachTerminator::Ret,
            _ => MachTerminator::None,
        }
    }

    fn is_epilogue_placeholder(&self) -> bool {
        false
    }

    fn gen_move(to_reg: Writable<Reg>, from_reg: Reg, ty: Type) -> Self {
        Inst::new(Op::Nop, None, None, vec![])
    }

    fn gen_constant(to_reg: Writable<Reg>, value: u64, ty: Type) -> SmallVec<[Self; 4]> {
        SmallVec::new()
    }

    fn gen_zero_len_nop() -> Self {
        Inst::new(Op::Nop, None, None, vec![])
    }

    fn maybe_direct_reload(&self, reg: VirtualReg, slot: SpillSlot) -> Option<Self> {
        None
    }

    fn rc_for_type(ty: Type) -> CodegenResult<RegClass> {
        match ty {
            I8 | I16 | I32 | B1 | B8 | B16 | B32 | F32 => Ok(RegClass::F32),
            F64 | B64 | I64 => Ok(RegClass::I64),
            _ => Err(CodegenError::Unsupported(format!(
                "Unexpected SSA-value type: {}",
                ty
            ))),
        }
    }

    fn gen_jump(target: MachLabel) -> Self {
        Inst::new(Op::Nop, None, None, vec![])
    }

    fn gen_nop(preferred_size: usize) -> Self {
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
    Standard,
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

    fn patch(self, buffer: &mut [u8], use_offset: CodeOffset, label_offset: CodeOffset) {}

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
