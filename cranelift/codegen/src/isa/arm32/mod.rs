//! ARM 32-bit Instruction Set Architecture (Thumb-2 subset).

use crate::dominator_tree::DominatorTree;
use crate::ir::{self, Function};
use crate::isa::arm32::settings as arm32_settings;
use crate::isa::{
    Builder as IsaBuilder, FunctionAlignment, IsaFlagsHashKey, OwnedTargetIsa, TargetIsa, Type,
};
use crate::machinst::{
    CompiledCode, CompiledCodeStencil, MachTextSectionBuilder, Reg, SigSet, TextSectionBuilder,
    VCode, compile,
};
use crate::result::CodegenResult;
use crate::settings::{self as shared_settings, Flags};
use alloc::{boxed::Box, string::String, vec::Vec};
use core::fmt;
use cranelift_control::ControlPlane;
use target_lexicon::{Architecture, Triple};

mod abi;
mod inst;
mod lower;
mod settings;

#[cfg(feature = "unwind")]
use crate::isa::unwind::systemv;

use self::inst::EmitInfo;

/// An ARM32 backend.
pub struct Arm32Backend {
    triple: Triple,
    flags: shared_settings::Flags,
    isa_flags: arm32_settings::Flags,
}

impl Arm32Backend {
    /// Create a new ARM32 backend with the given (shared) flags.
    pub fn new_with_flags(
        triple: Triple,
        flags: shared_settings::Flags,
        isa_flags: arm32_settings::Flags,
    ) -> Arm32Backend {
        Arm32Backend {
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
        let sigs = SigSet::new::<abi::Arm32MachineDeps>(func, &self.flags)?;
        let abi = abi::Arm32Callee::new(func, self, &self.isa_flags, &sigs)?;
        compile::compile::<Arm32Backend>(func, domtree, self, abi, emit_info, sigs, ctrl_plane)
    }
}

impl TargetIsa for Arm32Backend {
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
        let value_labels_ranges = emit_result.value_labels_ranges;
        let buffer = emit_result.buffer;

        if let Some(disasm) = emit_result.disasm.as_ref() {
            log::debug!("disassembly:\n{disasm}");
        }

        Ok(CompiledCodeStencil {
            buffer,
            vcode: emit_result.disasm,
            value_labels_ranges,
            bb_starts: emit_result.bb_offsets,
            bb_edges: emit_result.bb_edges,
        })
    }

    fn name(&self) -> &'static str {
        "arm32"
    }

    fn dynamic_vector_bytes(&self, _ty: Type) -> u32 {
        4
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

    fn isa_flags_hash_key(&self) -> IsaFlagsHashKey<'_> {
        IsaFlagsHashKey(self.isa_flags.hash_key())
    }

    #[cfg(feature = "unwind")]
    fn emit_unwind_info(
        &self,
        result: &CompiledCode,
        kind: crate::isa::unwind::UnwindInfoKind,
    ) -> CodegenResult<Option<crate::isa::unwind::UnwindInfo>> {
        use crate::isa::unwind::UnwindInfo;
        use crate::isa::unwind::UnwindInfoKind;
        Ok(match kind {
            UnwindInfoKind::SystemV => {
                let mapper = inst::unwind::systemv::RegisterMapper;
                Some(UnwindInfo::SystemV(
                    crate::isa::unwind::systemv::create_unwind_info_from_insts(
                        &result.buffer.unwind_info[..],
                        result.buffer.data().len(),
                        &mapper,
                    )?,
                ))
            }
            UnwindInfoKind::Windows => None,
            _ => None,
        })
    }

    #[cfg(feature = "unwind")]
    fn create_systemv_cie(&self) -> Option<gimli::write::CommonInformationEntry> {
        Some(inst::unwind::systemv::create_cie())
    }

    fn text_section_builder(&self, num_funcs: usize) -> Box<dyn TextSectionBuilder> {
        Box::new(MachTextSectionBuilder::<inst::Inst>::new(num_funcs))
    }

    #[cfg(feature = "unwind")]
    fn map_regalloc_reg_to_dwarf(&self, reg: Reg) -> Result<u16, systemv::RegisterMappingError> {
        inst::unwind::systemv::map_reg(reg)
    }

    fn function_alignment(&self) -> FunctionAlignment {
        inst::Inst::function_alignment()
    }

    fn page_size_align_log2(&self) -> u8 {
        // assume 32 bytes for now (small page on an cortex-m)
        5
    }

    #[cfg(feature = "disas")]
    fn to_capstone(&self) -> Result<capstone::Capstone, capstone::Error> {
        use capstone::prelude::*;
        let mut cs = Capstone::new()
            .arm()
            .mode(arch::arm::ArchMode::Thumb)
            .detail(true)
            .build()?;

        // Thumb mode uses inline constants rather than a separate constant pool.
        // Without this option, Capstone will stop disassembling as soon as it sees
        // an inline constant that is not also a valid instruction. With this option,
        // Capstone will print a .byte directive with the bytes of the inline constant
        // and continue to the next instruction.
        cs.set_skipdata(true)?;
        Ok(cs)
    }

    fn pretty_print_reg(&self, reg: Reg, _size: u8) -> String {
        inst::regs::pretty_print_reg(reg)
    }

    fn has_native_fma(&self) -> bool {
        false
    }

    fn has_round(&self) -> bool {
        false
    }

    fn has_blendv_lowering(&self, _: Type) -> bool {
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

    fn default_argument_extension(&self) -> ir::ArgumentExtension {
        ir::ArgumentExtension::Uext
    }
}

impl fmt::Display for Arm32Backend {
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
        Architecture::Arm(_) => {}
        _ => unreachable!(),
    }
    IsaBuilder {
        triple,
        setup: arm32_settings::builder(),
        constructor: isa_constructor,
    }
}

fn isa_constructor(
    triple: Triple,
    shared_flags: Flags,
    builder: &shared_settings::Builder,
) -> CodegenResult<OwnedTargetIsa> {
    let isa_flags = arm32_settings::Flags::new(&shared_flags, builder);
    let backend = Arm32Backend::new_with_flags(triple, shared_flags, isa_flags);
    Ok(backend.wrapped())
}
