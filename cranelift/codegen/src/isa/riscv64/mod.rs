//! risc-v 64-bit Instruction Set Architecture.

use crate::ir::condcodes::IntCC;
use crate::ir::Function;

use crate::isa::riscv64::settings as riscv_settings;
use crate::isa::{Builder as IsaBuilder, TargetIsa};
use crate::machinst::{
    compile, MachCompileResult, MachTextSectionBuilder, TextSectionBuilder, VCode,
};
use crate::result::CodegenResult;
use crate::settings as shared_settings;
use alloc::{boxed::Box, vec::Vec};
use core::fmt;
use regalloc2::MachineEnv;
use target_lexicon::{Architecture, Triple};

// New backend:
mod abi;
pub(crate) mod inst;
mod lower;
mod lower_inst;
mod settings;

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
        let abi = Box::new(abi::Riscv64Callee::new(func, flags, self.isa_flags())?);
        compile::compile::<Riscv64Backend>(func, self, abi, &self.mach_env, emit_info)
    }
}

impl TargetIsa for Riscv64Backend {
    fn compile_function(
        &self,
        func: &Function,
        want_disasm: bool,
    ) -> CodegenResult<MachCompileResult> {
        let flags = self.flags();
        let (vcode, regalloc_result) = self.compile_vcode(func, flags.clone())?;

        let want_disasm = want_disasm || log::log_enabled!(log::Level::Debug);
        let emit_result = vcode.emit(&regalloc_result, want_disasm, flags.machine_code_cfg_info());
        let frame_size = emit_result.frame_size;
        let value_labels_ranges = emit_result.value_labels_ranges;
        let buffer = emit_result.buffer.finish();
        let stackslot_offsets = emit_result.stackslot_offsets;
        if want_disasm {
            log::info!("compiler code:{}", emit_result.disasm.clone().unwrap());
        }
        Ok(MachCompileResult {
            buffer,
            frame_size,
            disasm: emit_result.disasm,
            value_labels_ranges,
            stackslot_offsets,
            bb_starts: emit_result.bb_offsets,
            bb_edges: emit_result.bb_edges,
        })
    }

    fn name(&self) -> &'static str {
        "riscv64gc"
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
        _result: &MachCompileResult,
        _kind: crate::machinst::UnwindInfoKind,
    ) -> CodegenResult<Option<crate::isa::unwind::UnwindInfo>> {
        unimplemented!()
    }

    #[cfg(feature = "unwind")]
    fn create_systemv_cie(&self) -> Option<gimli::write::CommonInformationEntry> {
        unimplemented!()
    }

    fn text_section_builder(&self, num_funcs: u32) -> Box<dyn TextSectionBuilder> {
        Box::new(MachTextSectionBuilder::<inst::Inst>::new(num_funcs))
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
    use crate::ir::{AbiParam, ExternalName, Function, InstBuilder, Signature};
    use crate::isa::CallConv;
    use crate::settings;
    use crate::settings::Configurable;
    use core::str::FromStr;
    use target_lexicon::Triple;

    #[test]
    fn test_compile_function() {
        let name = ExternalName::testcase("test0");
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
            Triple::from_str("riscv64gc").unwrap(),
            shared_flags,
            isa_flags,
        );
        let buffer = backend.compile_function(&mut func, true).unwrap();
        // println!("xxxx : {}", buffer.disasm.unwrap());
        let code = buffer.buffer.data();
        // write_to_a_file("/home/yuyang/tmp/code.bin", code);
        //0:   000015b7                lui     a1,0x1
        //4:   23458593                addi    a1,a1,564 # 0x1234
        //8:   00b5053b                addw    a0,a0,a1
        //c:   00008067                ret
        let golden = vec![
            183, 21, 0, 0, 147, 133, 69, 35, 59, 5, 181, 0, 103, 128, 0, 0,
        ];

        assert_eq!(code, &golden[..]);
    }
}
