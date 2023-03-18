//! X86_64-bit Instruction Set Architecture.

pub use self::inst::{args, CallInfo, EmitInfo, EmitState, Inst};

use super::{OwnedTargetIsa, TargetIsa};
use crate::dominator_tree::DominatorTree;
use crate::ir::{condcodes::IntCC, Function, Type};
#[cfg(feature = "unwind")]
use crate::isa::unwind::systemv;
use crate::isa::x64::{inst::regs::create_reg_env_systemv, settings as x64_settings};
use crate::isa::Builder as IsaBuilder;
use crate::machinst::{
    compile, CompiledCode, CompiledCodeStencil, MachTextSectionBuilder, Reg, SigSet,
    TextSectionBuilder, VCode,
};
use crate::result::{CodegenError, CodegenResult};
use crate::settings::{self as shared_settings, Flags};
use alloc::{boxed::Box, vec::Vec};
use core::fmt;
use cranelift_control::ControlPlane;
use regalloc2::MachineEnv;
use target_lexicon::Triple;

mod abi;
pub mod encoding;
mod inst;
mod lower;
pub mod settings;

/// An X64 backend.
pub(crate) struct X64Backend {
    triple: Triple,
    flags: Flags,
    x64_flags: x64_settings::Flags,
    reg_env: MachineEnv,
    /// Only used during fuzz-testing. Otherwise, this is a zero-sized struct
    /// and compiled away. See [cranelift_control].
    control_plane: ControlPlane,
}

impl X64Backend {
    /// Create a new X64 backend with the given (shared) flags.
    fn new_with_flags(
        triple: Triple,
        flags: Flags,
        x64_flags: x64_settings::Flags,
        control_plane: ControlPlane,
    ) -> Self {
        let reg_env = create_reg_env_systemv(&flags);
        Self {
            triple,
            flags,
            x64_flags,
            reg_env,
            control_plane,
        }
    }

    fn compile_vcode(
        &self,
        func: &Function,
        domtree: &DominatorTree,
    ) -> CodegenResult<(VCode<inst::Inst>, regalloc2::Output)> {
        // This performs lowering to VCode, register-allocates the code, computes
        // block layout and finalizes branches. The result is ready for binary emission.
        let emit_info = EmitInfo::new(self.flags.clone(), self.x64_flags.clone());
        let sigs = SigSet::new::<abi::X64ABIMachineSpec>(func, &self.flags)?;
        let abi = abi::X64Callee::new(func, self, &self.x64_flags, &sigs)?;
        compile::compile::<Self>(
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

impl TargetIsa for X64Backend {
    fn compile_function(
        &self,
        func: &Function,
        domtree: &DominatorTree,
        want_disasm: bool,
    ) -> CodegenResult<CompiledCodeStencil> {
        let (vcode, regalloc_result) = self.compile_vcode(func, domtree)?;

        let emit_result = vcode.emit(
            &regalloc_result,
            want_disasm,
            self.flags.machine_code_cfg_info(),
        );
        let frame_size = emit_result.frame_size;
        let value_labels_ranges = emit_result.value_labels_ranges;
        let buffer = emit_result.buffer.finish();
        let sized_stackslot_offsets = emit_result.sized_stackslot_offsets;
        let dynamic_stackslot_offsets = emit_result.dynamic_stackslot_offsets;

        if let Some(disasm) = emit_result.disasm.as_ref() {
            log::trace!("disassembly:\n{}", disasm);
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

    fn flags(&self) -> &Flags {
        &self.flags
    }

    fn machine_env(&self) -> &MachineEnv {
        &self.reg_env
    }

    fn isa_flags(&self) -> Vec<shared_settings::Value> {
        self.x64_flags.iter().collect()
    }

    fn dynamic_vector_bytes(&self, _dyn_ty: Type) -> u32 {
        16
    }

    fn name(&self) -> &'static str {
        "x64"
    }

    fn triple(&self) -> &Triple {
        &self.triple
    }

    fn unsigned_add_overflow_condition(&self) -> IntCC {
        // Unsigned `<`; this corresponds to the carry flag set on x86, which
        // indicates an add has overflowed.
        IntCC::UnsignedLessThan
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
            UnwindInfoKind::Windows => Some(UnwindInfo::WindowsX64(
                crate::isa::unwind::winx64::create_unwind_info_from_insts::<
                    self::inst::unwind::winx64::RegisterMapper,
                >(&result.buffer.unwind_info[..])?,
            )),
            _ => None,
        })
    }

    #[cfg(feature = "unwind")]
    fn create_systemv_cie(&self) -> Option<gimli::write::CommonInformationEntry> {
        Some(inst::unwind::systemv::create_cie())
    }

    #[cfg(feature = "unwind")]
    fn map_regalloc_reg_to_dwarf(&self, reg: Reg) -> Result<u16, systemv::RegisterMappingError> {
        inst::unwind::systemv::map_reg(reg).map(|reg| reg.0)
    }

    fn text_section_builder(&self, num_funcs: usize) -> Box<dyn TextSectionBuilder> {
        Box::new(MachTextSectionBuilder::<inst::Inst>::new(num_funcs))
    }

    /// Align functions on x86 to 16 bytes, ensuring that rip-relative loads to SSE registers are
    /// always from aligned memory.
    fn function_alignment(&self) -> u32 {
        16
    }

    #[cfg(feature = "disas")]
    fn to_capstone(&self) -> Result<capstone::Capstone, capstone::Error> {
        use capstone::prelude::*;
        Capstone::new()
            .x86()
            .mode(arch::x86::ArchMode::Mode64)
            .syntax(arch::x86::ArchSyntax::Att)
            .build()
    }

    fn has_native_fma(&self) -> bool {
        self.x64_flags.use_fma()
    }
}

impl fmt::Display for X64Backend {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("MachBackend")
            .field("name", &self.name())
            .field("triple", &self.triple())
            .field("flags", &format!("{}", self.flags()))
            .finish()
    }
}

/// Create a new `isa::Builder`.
pub(crate) fn isa_builder(triple: Triple, control_plane: ControlPlane) -> IsaBuilder {
    IsaBuilder {
        triple,
        control_plane,
        setup: x64_settings::builder(),
        constructor: isa_constructor,
    }
}

fn isa_constructor(
    triple: Triple,
    shared_flags: Flags,
    builder: &shared_settings::Builder,
    control_plane: ControlPlane,
) -> CodegenResult<OwnedTargetIsa> {
    let isa_flags = x64_settings::Flags::new(&shared_flags, builder);

    // Check for compatibility between flags and ISA level
    // requested. In particular, SIMD support requires SSE4.2.
    if shared_flags.enable_simd() {
        if !isa_flags.has_sse3()
            || !isa_flags.has_ssse3()
            || !isa_flags.has_sse41()
            || !isa_flags.has_sse42()
        {
            return Err(CodegenError::Unsupported(
                "SIMD support requires SSE3, SSSE3, SSE4.1, and SSE4.2 on x86_64.".into(),
            ));
        }
    }

    let backend = X64Backend::new_with_flags(triple, shared_flags, isa_flags, control_plane);
    Ok(backend.wrapped())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::settings;
    use crate::settings::Configurable;

    // Check that feature tests for SIMD work correctly.
    #[test]
    fn simd_required_features() {
        let mut shared_flags_builder = settings::builder();
        shared_flags_builder.set("enable_simd", "true").unwrap();
        let shared_flags = settings::Flags::new(shared_flags_builder);
        let mut isa_builder = crate::isa::lookup_by_name("x86_64", ControlPlane::noop()).unwrap();
        isa_builder.set("has_sse3", "false").unwrap();
        isa_builder.set("has_ssse3", "false").unwrap();
        isa_builder.set("has_sse41", "false").unwrap();
        isa_builder.set("has_sse42", "false").unwrap();
        assert!(matches!(
            isa_builder.finish(shared_flags),
            Err(CodegenError::Unsupported(_)),
        ));
    }
}
