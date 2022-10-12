//! risc-v 64-bit Instruction Set Architecture.

use crate::ir;
use crate::ir::condcodes::IntCC;
use crate::ir::Function;

use crate::isa::riscv64::settings as riscv_settings;
use crate::isa::{Builder as IsaBuilder, TargetIsa};
use crate::machinst::{
    compile, CompiledCode, CompiledCodeStencil, MachTextSectionBuilder, Reg, SigSet,
    TextSectionBuilder, VCode,
};
use crate::result::CodegenResult;
use crate::settings as shared_settings;
use alloc::{boxed::Box, vec::Vec};
use core::fmt;
use regalloc2::MachineEnv;
use target_lexicon::{Architecture, Triple};
mod abi;
pub(crate) mod inst;
mod lower;
mod lower_inst;
mod settings;
#[cfg(feature = "unwind")]
use crate::isa::unwind::systemv;

use inst::crate_reg_eviroment;

use self::inst::EmitInfo;

/// An riscv64 backend.
pub struct Riscv64Backend {
    triple: Triple,
    flags: shared_settings::Flags,
    isa_flags: riscv_settings::Flags,
    mach_env: MachineEnv,
}

impl Riscv64Backend {
    /// Create a new riscv64 backend with the given (shared) flags.
    pub fn new_with_flags(
        triple: Triple,
        flags: shared_settings::Flags,
        isa_flags: riscv_settings::Flags,
    ) -> Riscv64Backend {
        let mach_env = crate_reg_eviroment(&flags);
        Riscv64Backend {
            triple,
            flags,
            isa_flags,
            mach_env,
        }
    }

    /// This performs lowering to VCode, register-allocates the code, computes block layout and
    /// finalizes branches. The result is ready for binary emission.
    fn compile_vcode(
        &self,
        func: &Function,
        flags: shared_settings::Flags,
    ) -> CodegenResult<(VCode<inst::Inst>, regalloc2::Output)> {
        let emit_info = EmitInfo::new(flags.clone(), self.isa_flags.clone());
        let sigs = SigSet::new::<abi::Riscv64MachineDeps>(func, &self.flags)?;
        let abi = abi::Riscv64Callee::new(func, self, &self.isa_flags, &sigs)?;
        compile::compile::<Riscv64Backend>(func, flags, self, abi, &self.mach_env, emit_info, sigs)
    }
}

impl TargetIsa for Riscv64Backend {
    fn compile_function(
        &self,
        func: &Function,
        want_disasm: bool,
    ) -> CodegenResult<CompiledCodeStencil> {
        let flags = self.flags();
        let (vcode, regalloc_result) = self.compile_vcode(func, flags.clone())?;

        let want_disasm = want_disasm || log::log_enabled!(log::Level::Debug);
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
            disasm: emit_result.disasm,
            value_labels_ranges,
            sized_stackslot_offsets,
            dynamic_stackslot_offsets,
            bb_starts: emit_result.bb_offsets,
            bb_edges: emit_result.bb_edges,
            alignment: emit_result.alignment,
        })
    }

    fn name(&self) -> &'static str {
        "riscv64"
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

    fn unsigned_add_overflow_condition(&self) -> IntCC {
        IntCC::UnsignedGreaterThanOrEqual
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
            UnwindInfoKind::Windows => None,
            _ => None,
        })
    }

    #[cfg(feature = "unwind")]
    fn create_systemv_cie(&self) -> Option<gimli::write::CommonInformationEntry> {
        Some(inst::unwind::systemv::create_cie())
    }

    fn text_section_builder(&self, num_funcs: u32) -> Box<dyn TextSectionBuilder> {
        Box::new(MachTextSectionBuilder::<inst::Inst>::new(num_funcs))
    }

    #[cfg(feature = "unwind")]
    fn map_regalloc_reg_to_dwarf(&self, reg: Reg) -> Result<u16, systemv::RegisterMappingError> {
        inst::unwind::systemv::map_reg(reg).map(|reg| reg.0)
    }

    fn function_alignment(&self) -> u32 {
        4
    }
}

impl fmt::Display for Riscv64Backend {
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
        Architecture::Riscv64(..) => {}
        _ => unreachable!(),
    }
    IsaBuilder {
        triple,
        setup: riscv_settings::builder(),
        constructor: |triple, shared_flags, builder| {
            let isa_flags = riscv_settings::Flags::new(&shared_flags, builder);
            let backend = Riscv64Backend::new_with_flags(triple, shared_flags, isa_flags);
            Ok(Box::new(backend))
        },
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::cursor::{Cursor, FuncCursor};
    use crate::ir::types::*;
    use crate::ir::{AbiParam, Function, InstBuilder, Signature, UserFuncName};
    use crate::isa::CallConv;
    use crate::settings;
    use crate::settings::Configurable;
    use core::str::FromStr;
    use target_lexicon::Triple;

    #[test]
    fn test_compile_function() {
        let name = UserFuncName::testcase("test0");
        let mut sig = Signature::new(CallConv::SystemV);
        sig.params.push(AbiParam::new(I32));
        sig.returns.push(AbiParam::new(I32));
        let mut func = Function::with_name_signature(name, sig);

        let bb0 = func.dfg.make_block();
        let arg0 = func.dfg.append_block_param(bb0, I32);

        let mut pos = FuncCursor::new(&mut func);
        pos.insert_block(bb0);
        let v0 = pos.ins().iconst(I32, 0x1234);
        let v1 = pos.ins().iadd(arg0, v0);
        pos.ins().return_(&[v1]);

        let mut shared_flags_builder = settings::builder();
        shared_flags_builder.set("opt_level", "none").unwrap();
        let shared_flags = settings::Flags::new(shared_flags_builder);
        let isa_flags = riscv_settings::Flags::new(&shared_flags, riscv_settings::builder());
        let backend = Riscv64Backend::new_with_flags(
            Triple::from_str("riscv64").unwrap(),
            shared_flags,
            isa_flags,
        );
        let buffer = backend.compile_function(&mut func, true).unwrap();
        let code = buffer.buffer.data();
        // 0:   000015b7                lui     a1,0x1
        // 4:   23458593                addi    a1,a1,564 # 0x1234
        // 8:   00b5053b                addw    a0,a0,a1
        // c:   00008067                ret
        let golden = vec![
            0xb7, 0x15, 0x0, 0x0, 0x93, 0x85, 0x45, 0x23, 0x3b, 0x5, 0xb5, 0x0, 0x67, 0x80, 0x0,
            0x0,
        ];
        assert_eq!(code, &golden[..]);
    }
}
