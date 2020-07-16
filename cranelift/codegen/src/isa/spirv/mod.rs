// jb-todo: remove these, they're just here for bring up
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(dead_code)]

mod inst;
mod abi;
mod lower;

use alloc::boxed::Box;
use crate::binemit::CodeOffset;
use crate::ir::condcodes::IntCC;
use crate::ir::Function;
use crate::ir::types::Type;
use crate::isa::Builder as IsaBuilder;
use crate::machinst::{compile, MachBackend, MachCompileResult, TargetIsaAdapter, VCode};
use crate::machinst::{MachBuffer, MachInst, MachInstEmit};
use crate::machinst::buffer::MachLabel;
use crate::machinst::MachInstLabelUse;
use crate::machinst::MachTerminator;
use crate::machinst::pretty_print::ShowWithRRU;
use crate::result::{CodegenResult, CodegenError};
use crate::settings::{self, Flags};

use inst::Inst;
use regalloc::NUM_REG_CLASSES;
use regalloc::RealRegUniverse;
use regalloc::Reg;
use regalloc::RegClass;
use regalloc::RegUsageCollector;
use regalloc::RegUsageMapper;
use regalloc::SpillSlot;
use regalloc::VirtualReg;
use regalloc::Writable;
use smallvec::SmallVec;
use target_lexicon::Architecture;
use target_lexicon::Triple;
use log::debug;


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

    fn compile_vcode(&self, func: &Function, flags: Flags) -> CodegenResult<VCode<Inst>> {
        // This performs lowering to VCode, register-allocates the code, computes
        // block layout and finalizes branches. The result is ready for binary emission.
        let abi = Box::new(abi::SpirvABIBody::new(func, flags));
        compile::compile::<Self>(&func, self, abi)
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

        
        Ok(MachCompileResult {
            buffer,
            frame_size,
            disasm: None,
        })
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

    fn unsigned_add_overflow_condition(&self) -> IntCC {
        IntCC::UnsignedLessThan
    }

    fn unsigned_sub_overflow_condition(&self) -> IntCC {
        IntCC::UnsignedGreaterThanOrEqual
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