//! IBM Z 64-bit Instruction Set Architecture.

use crate::dominator_tree::DominatorTree;
use crate::ir::condcodes::IntCC;
use crate::ir::{Function, Type};
use crate::isa::s390x::settings as s390x_settings;
#[cfg(feature = "unwind")]
use crate::isa::unwind::systemv::RegisterMappingError;
use crate::isa::{Builder as IsaBuilder, TargetIsa};
use crate::machinst::{
    compile, CompiledCode, CompiledCodeStencil, MachTextSectionBuilder, Reg, SigSet,
    TextSectionBuilder, VCode,
};
use crate::result::CodegenResult;
use crate::settings as shared_settings;
use alloc::{boxed::Box, vec::Vec};
use core::fmt;
use cranelift_control::ControlPlane;
use regalloc2::MachineEnv;
use target_lexicon::{Architecture, Triple};

// New backend:
mod abi;
pub(crate) mod inst;
mod lower;
mod settings;

use inst::create_machine_env;

use self::inst::EmitInfo;

/// A IBM Z backend.
pub struct S390xBackend {
    triple: Triple,
    flags: shared_settings::Flags,
    isa_flags: s390x_settings::Flags,
    machine_env: MachineEnv,
    /// Only used during fuzz-testing. Otherwise, this is a zero-sized struct
    /// and compiled away. See [cranelift_control].
    control_plane: ControlPlane,
}

impl S390xBackend {
    /// Create a new IBM Z backend with the given (shared) flags.
    pub fn new_with_flags(
        triple: Triple,
        flags: shared_settings::Flags,
        isa_flags: s390x_settings::Flags,
        control_plane: ControlPlane,
    ) -> S390xBackend {
        let machine_env = create_machine_env(&flags);
        S390xBackend {
            triple,
            flags,
            isa_flags,
            machine_env,
            control_plane,
        }
    }

    /// This performs lowering to VCode, register-allocates the code, computes block layout and
    /// finalizes branches. The result is ready for binary emission.
    fn compile_vcode(
        &self,
        func: &Function,
        domtree: &DominatorTree,
    ) -> CodegenResult<(VCode<inst::Inst>, regalloc2::Output)> {
        let emit_info = EmitInfo::new(self.isa_flags.clone());
        let sigs = SigSet::new::<abi::S390xMachineDeps>(func, &self.flags)?;
        let abi = abi::S390xCallee::new(func, self, &self.isa_flags, &sigs)?;
        compile::compile::<S390xBackend>(
            func,
            domtree,
            self,
            abi,
            emit_info,
            sigs,
            self.control_plane.clone(),
        )
    }
}

impl TargetIsa for S390xBackend {
    fn compile_function(
        &self,
        func: &Function,
        domtree: &DominatorTree,
        want_disasm: bool,
    ) -> CodegenResult<CompiledCodeStencil> {
        let flags = self.flags();
        let (vcode, regalloc_result) = self.compile_vcode(func, domtree)?;

        let emit_result = vcode.emit(&regalloc_result, want_disasm, flags.machine_code_cfg_info());
        let frame_size = emit_result.frame_size;
        let value_labels_ranges = emit_result.value_labels_ranges;
        let buffer = emit_result.buffer.finish();
        let sized_stackslot_offsets = emit_result.sized_stackslot_offsets;
        let dynamic_stackslot_offsets = emit_result.dynamic_stackslot_offsets;

        if let Some(disasm) = emit_result.disasm.as_ref() {
            log::debug!("disassembly:\n{}", disasm);
        }

        Ok(CompiledCodeStencil {
            buffer,
            frame_size,
            vcode: emit_result.disasm,
            value_labels_ranges,
            sized_stackslot_offsets,
            dynamic_stackslot_offsets,
            bb_starts: emit_result.bb_offsets,
            bb_edges: emit_result.bb_edges,
            alignment: emit_result.alignment,
        })
    }

    fn name(&self) -> &'static str {
        "s390x"
    }

    fn triple(&self) -> &Triple {
        &self.triple
    }

    fn flags(&self) -> &shared_settings::Flags {
        &self.flags
    }

    fn machine_env(&self) -> &MachineEnv {
        &self.machine_env
    }

    fn isa_flags(&self) -> Vec<shared_settings::Value> {
        self.isa_flags.iter().collect()
    }

    fn dynamic_vector_bytes(&self, _dyn_ty: Type) -> u32 {
        16
    }

    fn unsigned_add_overflow_condition(&self) -> IntCC {
        // The ADD LOGICAL family of instructions set the condition code
        // differently from normal comparisons, in a way that cannot be
        // represented by any of the standard IntCC values.  So we use a
        // dummy value here, which gets remapped to the correct condition
        // code mask during lowering.
        IntCC::UnsignedGreaterThan
    }

    #[cfg(feature = "unwind")]
    fn emit_unwind_info(
        &self,
        result: &CompiledCode,
        kind: crate::machinst::UnwindInfoKind,
    ) -> CodegenResult<Option<crate::isa::unwind::UnwindInfo>> {
        use crate::isa::unwind::UnwindInfo;
        use crate::machinst::UnwindInfoKind;
        Ok(match kind {
            UnwindInfoKind::SystemV => {
                let mapper = self::inst::unwind::systemv::RegisterMapper;
                Some(UnwindInfo::SystemV(
                    crate::isa::unwind::systemv::create_unwind_info_from_insts(
                        &result.buffer.unwind_info[..],
                        result.buffer.data().len(),
                        &mapper,
                    )?,
                ))
            }
            _ => None,
        })
    }

    #[cfg(feature = "unwind")]
    fn create_systemv_cie(&self) -> Option<gimli::write::CommonInformationEntry> {
        Some(inst::unwind::systemv::create_cie())
    }

    #[cfg(feature = "unwind")]
    fn map_regalloc_reg_to_dwarf(&self, reg: Reg) -> Result<u16, RegisterMappingError> {
        inst::unwind::systemv::map_reg(reg).map(|reg| reg.0)
    }

    fn text_section_builder(&self, num_funcs: usize) -> Box<dyn TextSectionBuilder> {
        Box::new(MachTextSectionBuilder::<inst::Inst>::new(num_funcs))
    }

    fn function_alignment(&self) -> u32 {
        4
    }

    #[cfg(feature = "disas")]
    fn to_capstone(&self) -> Result<capstone::Capstone, capstone::Error> {
        use capstone::prelude::*;
        let mut cs = Capstone::new()
            .sysz()
            .mode(arch::sysz::ArchMode::Default)
            .build()?;

        cs.set_skipdata(true)?;

        Ok(cs)
    }

    fn has_native_fma(&self) -> bool {
        true
    }
}

impl fmt::Display for S390xBackend {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("MachBackend")
            .field("name", &self.name())
            .field("triple", &self.triple())
            .field("flags", &format!("{}", self.flags()))
            .finish()
    }
}

/// Create a new `isa::Builder`.
pub fn isa_builder(triple: Triple, control_plane: ControlPlane) -> IsaBuilder {
    assert!(triple.architecture == Architecture::S390x);
    IsaBuilder {
        triple,
        control_plane,
        setup: s390x_settings::builder(),
        constructor: |triple, shared_flags, builder, control_plane| {
            let isa_flags = s390x_settings::Flags::new(&shared_flags, builder);
            let backend =
                S390xBackend::new_with_flags(triple, shared_flags, isa_flags, control_plane);
            Ok(backend.wrapped())
        },
    }
}
