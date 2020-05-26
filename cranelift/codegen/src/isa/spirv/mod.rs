// jb-todo: remove these, they're just here for bring up
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(dead_code)]

use alloc::boxed::Box;

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
enum SpirvInst {
    Nop
}

#[derive(Debug, Default, Clone)]
struct MachInstEmitState {

}

impl MachInstEmit for SpirvInst {
    type State = MachInstEmitState;

    fn emit(&self, code: &mut MachBuffer<Self>, flags: &Flags, state: &mut Self::State) {

    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LabelUse {
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

impl MachInst for SpirvInst {
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
        SpirvInst::Nop
    }

    fn gen_constant(to_reg: Writable<Reg>, value: u64, ty: Type) -> SmallVec<[Self; 4]> {
        SmallVec::new()
    }

    fn gen_zero_len_nop() -> Self {
        SpirvInst::Nop
    }

    fn maybe_direct_reload(&self, reg: VirtualReg, slot: SpillSlot) -> Option<Self> {
        None
    }

    fn rc_for_type(ty: Type) -> CodegenResult<RegClass> {
        Err(CodegenError::Unsupported(format!("rc_for_type")))
    }

    fn gen_jump(target: MachLabel) -> Self {
        SpirvInst::Nop
    }

    fn gen_nop(preferred_size: usize) -> Self {
        SpirvInst::Nop
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

pub(crate) struct SpirvBackend {
    triple: Triple,
    flags: Flags,
    reg_universe: RealRegUniverse,
}

impl SpirvBackend {
    fn new_with_flags(triple: Triple, flags: Flags) -> Self {
        let reg_universe = RealRegUniverse {
            regs: vec![],
            allocable: 0,
            allocable_by_class: [None; NUM_REG_CLASSES],
        };

        Self {
            triple,
            flags,
            reg_universe,
        }
    }

    fn compile_vcode(&self, func: &Function, flags: Flags) -> CodegenResult<VCode<SpirvInst>> {
        // This performs lowering to VCode, register-allocates the code, computes
        // block layout and finalizes branches. The result is ready for binary emission.
        //let abi = Box::new(abi::X64ABIBody::new(&func, flags));
        //compile::compile::<Self>(&func, self, abi)

        Err(CodegenError::Unsupported(format!("compile_vcode")))
    }
}

impl MachBackend for SpirvBackend {
    fn compile_function(
        &self,
        func: &Function,
        want_disasm: bool,
    ) -> CodegenResult<MachCompileResult> {
         let flags = self.flags();
         let vcode = self.compile_vcode(func, flags.clone())?;
         let buffer = vcode.emit();
         let buffer = buffer.finish();
         let frame_size = vcode.frame_size();

        // let disasm = if want_disasm {
        //     Some(vcode.show_rru(Some(&create_reg_universe_systemv(flags))))
        // } else {
        //     None
        // };

        Err(CodegenError::Unsupported(format!("compile_function")))
    }

    fn flags(&self) -> &Flags {
        &self.flags
    }

    fn name(&self) -> &'static str {
        "spirv"
    }

    fn triple(&self) -> Triple {
        self.triple.clone()
    }

    fn reg_universe(&self) -> &RealRegUniverse {
        &self.reg_universe
    }
}


/// Create a new `isa::Builder`.
pub fn isa_builder(triple: Triple) -> IsaBuilder {
    assert!(triple.architecture == Architecture::Spirv);
    IsaBuilder {
        triple,
        setup: settings::builder(),
        constructor: |triple, shared_flags, _| {
            let backend = SpirvBackend::new_with_flags(triple, shared_flags);
            Box::new(TargetIsaAdapter::new(backend))
        },
    }
}