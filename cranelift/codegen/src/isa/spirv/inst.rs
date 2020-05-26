
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

#[derive(Clone, Debug)]
pub(crate) enum Inst {
    Nop
}

#[derive(Debug, Default, Clone)]
pub(crate) struct MachInstEmitState {

}

impl MachInstEmit for Inst {
    type State = MachInstEmitState;

    fn emit(&self, code: &mut MachBuffer<Self>, flags: &Flags, state: &mut Self::State) {

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
        Inst::Nop
    }

    fn gen_constant(to_reg: Writable<Reg>, value: u64, ty: Type) -> SmallVec<[Self; 4]> {
        SmallVec::new()
    }

    fn gen_zero_len_nop() -> Self {
        Inst::Nop
    }

    fn maybe_direct_reload(&self, reg: VirtualReg, slot: SpillSlot) -> Option<Self> {
        None
    }

    fn rc_for_type(ty: Type) -> CodegenResult<RegClass> {
        Err(CodegenError::Unsupported(format!("rc_for_type")))
    }

    fn gen_jump(target: MachLabel) -> Self {
        Inst::Nop
    }

    fn gen_nop(preferred_size: usize) -> Self {
        Inst::Nop
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