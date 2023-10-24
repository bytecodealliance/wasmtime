//! ZkASM Instruction Set Architecture.

use crate::dominator_tree::DominatorTree;
use crate::ir;
use crate::ir::{Function, Type};
use crate::isa::zkasm::settings as zkasm_settings;
use crate::isa::{Builder as IsaBuilder, FunctionAlignment, TargetIsa};
use crate::machinst::{
    compile, CompiledCode, CompiledCodeStencil, MachInst, MachTextSectionBuilder, SigSet,
    TextSectionBuilder, VCode,
};
use crate::result::CodegenResult;
use crate::settings as shared_settings;
use alloc::{boxed::Box, vec::Vec};
use core::fmt;
use cranelift_control::ControlPlane;
use target_lexicon::{Architecture, Triple};
mod abi;
pub(crate) mod inst;
mod lower;
mod settings;

use self::inst::EmitInfo;

/// The zkasm backend.
pub struct ZkAsmBackend {
    triple: Triple,
    flags: shared_settings::Flags,
    isa_flags: zkasm_settings::Flags,
}

impl ZkAsmBackend {
    /// Create a new zkasm backend with the given (shared) flags.
    pub fn new_with_flags(
        triple: Triple,
        flags: shared_settings::Flags,
        isa_flags: zkasm_settings::Flags,
    ) -> ZkAsmBackend {
        ZkAsmBackend {
            triple,
            flags,
            isa_flags,
        }
    }

    /// This performs lowering to VCode, register-allocates the code, computes block layout and
    /// finalizes branches. The result is ready for binary emission.
    fn compile_vcode(
        &self,
        func: &Function,
        domtree: &DominatorTree,
        ctrl_plane: &mut ControlPlane,
    ) -> CodegenResult<(VCode<inst::Inst>, regalloc2::Output)> {
        let emit_info = EmitInfo::new(self.flags.clone(), self.isa_flags.clone());
        let sigs = SigSet::new::<abi::ZkAsmMachineDeps>(func, &self.flags)?;
        let abi = abi::ZkAsmCallee::new(func, self, &self.isa_flags, &sigs)?;
        compile::compile::<ZkAsmBackend>(func, domtree, self, abi, emit_info, sigs, ctrl_plane)
    }
}

impl TargetIsa for ZkAsmBackend {
    fn compile_function(
        &self,
        func: &Function,
        domtree: &DominatorTree,
        want_disasm: bool,
        ctrl_plane: &mut ControlPlane,
    ) -> CodegenResult<CompiledCodeStencil> {
        let (vcode, regalloc_result) = self.compile_vcode(func, domtree, ctrl_plane)?;

        let want_disasm = want_disasm || log::log_enabled!(log::Level::Debug);
        let emit_result = vcode.emit(&regalloc_result, want_disasm, &self.flags, ctrl_plane);
        let frame_size = emit_result.frame_size;
        let value_labels_ranges = emit_result.value_labels_ranges;
        let buffer = emit_result.buffer;
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
        })
    }

    fn name(&self) -> &'static str {
        "zkasm"
    }
    fn dynamic_vector_bytes(&self, _dynamic_ty: ir::Type) -> u32 {
        16
    }

    fn triple(&self) -> &Triple {
        &self.triple
    }

    fn flags(&self) -> &shared_settings::Flags {
        &self.flags
    }

    fn isa_flags(&self) -> Vec<shared_settings::Value> {
        self.isa_flags.iter().collect()
    }

    #[cfg(feature = "unwind")]
    fn emit_unwind_info(
        &self,
        _result: &CompiledCode,
        _kind: crate::isa::UnwindInfoKind,
    ) -> CodegenResult<Option<crate::isa::unwind::UnwindInfo>> {
        Ok(None)
    }

    fn text_section_builder(&self, num_funcs: usize) -> Box<dyn TextSectionBuilder> {
        Box::new(MachTextSectionBuilder::<inst::Inst>::new(num_funcs))
    }

    fn function_alignment(&self) -> FunctionAlignment {
        inst::Inst::function_alignment()
    }

    #[cfg(feature = "disas")]
    fn to_capstone(&self) -> Result<capstone::Capstone, capstone::Error> {
        Err(capstone::Error::UnsupportedArch)
    }

    fn has_native_fma(&self) -> bool {
        true
    }

    fn has_x86_blendv_lowering(&self, _: Type) -> bool {
        false
    }

    fn has_x86_pshufb_lowering(&self) -> bool {
        false
    }

    fn has_x86_pmulhrsw_lowering(&self) -> bool {
        false
    }

    fn has_x86_pmaddubsw_lowering(&self) -> bool {
        false
    }
}

impl fmt::Display for ZkAsmBackend {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("MachBackend")
            .field("name", &self.name())
            .field("triple", &self.triple())
            .field("flags", &format!("{}", self.flags()))
            .finish()
    }
}

/// Create a new `isa::Builder`.
pub fn isa_builder(triple: Triple) -> IsaBuilder {
    match triple.architecture {
        Architecture::ZkAsm => {}
        _ => unreachable!(),
    }
    IsaBuilder {
        triple,
        setup: zkasm_settings::builder(),
        constructor: |triple, shared_flags, builder| {
            let isa_flags = zkasm_settings::Flags::new(&shared_flags, builder);
            let backend = ZkAsmBackend::new_with_flags(triple, shared_flags, isa_flags);
            Ok(backend.wrapped())
        },
    }
}
