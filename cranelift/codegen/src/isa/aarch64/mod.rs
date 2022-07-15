//! ARM 64-bit Instruction Set Architecture.

use crate::ir::condcodes::IntCC;
use crate::ir::{Function, Type};
use crate::isa::aarch64::settings as aarch64_settings;
use crate::isa::{Builder as IsaBuilder, TargetIsa};
use crate::machinst::{
    compile, MachCompileResult, MachTextSectionBuilder, TextSectionBuilder, VCode,
};
use crate::result::CodegenResult;
use crate::settings as shared_settings;
use alloc::{boxed::Box, vec::Vec};
use core::fmt;
use regalloc2::MachineEnv;
use target_lexicon::{Aarch64Architecture, Architecture, Triple};

// New backend:
mod abi;
pub(crate) mod inst;
mod lower;
mod lower_inst;
mod settings;

use inst::create_reg_env;

use self::inst::EmitInfo;

/// An AArch64 backend.
pub struct AArch64Backend {
    triple: Triple,
    flags: shared_settings::Flags,
    isa_flags: aarch64_settings::Flags,
    machine_env: MachineEnv,
}

impl AArch64Backend {
    /// Create a new AArch64 backend with the given (shared) flags.
    pub fn new_with_flags(
        triple: Triple,
        flags: shared_settings::Flags,
        isa_flags: aarch64_settings::Flags,
    ) -> AArch64Backend {
        let machine_env = create_reg_env(&flags);
        AArch64Backend {
            triple,
            flags,
            isa_flags,
            machine_env,
        }
    }

    /// This performs lowering to VCode, register-allocates the code, computes block layout and
    /// finalizes branches. The result is ready for binary emission.
    fn compile_vcode(
        &self,
        func: &Function,
        flags: shared_settings::Flags,
    ) -> CodegenResult<(VCode<inst::Inst>, regalloc2::Output)> {
        let emit_info = EmitInfo::new(flags.clone());
        let abi = Box::new(abi::AArch64ABICallee::new(func, self)?);
        compile::compile::<AArch64Backend>(func, self, abi, &self.machine_env, emit_info)
    }
}

impl TargetIsa for AArch64Backend {
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
        let sized_stackslot_offsets = emit_result.sized_stackslot_offsets;
        let dynamic_stackslot_offsets = emit_result.dynamic_stackslot_offsets;

        if let Some(disasm) = emit_result.disasm.as_ref() {
            log::debug!("disassembly:\n{}", disasm);
        }

        Ok(MachCompileResult {
            buffer,
            frame_size,
            disasm: emit_result.disasm,
            value_labels_ranges,
            sized_stackslot_offsets,
            dynamic_stackslot_offsets,
            bb_starts: emit_result.bb_offsets,
            bb_edges: emit_result.bb_edges,
        })
    }

    fn name(&self) -> &'static str {
        "aarch64"
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

    fn dynamic_vector_bytes(&self, _dyn_ty: Type) -> u32 {
        16
    }

    fn unsigned_add_overflow_condition(&self) -> IntCC {
        // Unsigned `>=`; this corresponds to the carry flag set on aarch64, which happens on
        // overflow of an add.
        IntCC::UnsignedGreaterThanOrEqual
    }

    #[cfg(feature = "unwind")]
    fn emit_unwind_info(
        &self,
        result: &MachCompileResult,
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
            UnwindInfoKind::Windows => {
                // TODO: support Windows unwind info on AArch64
                None
            }
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
}

impl fmt::Display for AArch64Backend {
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
    assert!(triple.architecture == Architecture::Aarch64(Aarch64Architecture::Aarch64));
    IsaBuilder {
        triple,
        setup: aarch64_settings::builder(),
        constructor: |triple, shared_flags, builder| {
            let isa_flags = aarch64_settings::Flags::new(&shared_flags, builder);
            let backend = AArch64Backend::new_with_flags(triple, shared_flags, isa_flags);
            Ok(Box::new(backend))
        },
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::cursor::{Cursor, FuncCursor};
    use crate::ir::types::*;
    use crate::ir::{AbiParam, ExternalName, Function, InstBuilder, JumpTableData, Signature};
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
        let isa_flags = aarch64_settings::Flags::new(&shared_flags, aarch64_settings::builder());
        let backend = AArch64Backend::new_with_flags(
            Triple::from_str("aarch64").unwrap(),
            shared_flags,
            isa_flags,
        );
        let buffer = backend.compile_function(&mut func, false).unwrap().buffer;
        let code = buffer.data();

        // mov x3, #0x1234
        // add w0, w0, w3
        // ret
        let golden = vec![
            0x83, 0x46, 0x82, 0xd2, 0x00, 0x00, 0x03, 0x0b, 0xc0, 0x03, 0x5f, 0xd6,
        ];

        assert_eq!(code, &golden[..]);
    }

    #[test]
    fn test_branch_lowering() {
        let name = ExternalName::testcase("test0");
        let mut sig = Signature::new(CallConv::SystemV);
        sig.params.push(AbiParam::new(I32));
        sig.returns.push(AbiParam::new(I32));
        let mut func = Function::with_name_signature(name, sig);

        let bb0 = func.dfg.make_block();
        let arg0 = func.dfg.append_block_param(bb0, I32);
        let bb1 = func.dfg.make_block();
        let bb2 = func.dfg.make_block();
        let bb3 = func.dfg.make_block();

        let mut pos = FuncCursor::new(&mut func);
        pos.insert_block(bb0);
        let v0 = pos.ins().iconst(I32, 0x1234);
        let v1 = pos.ins().iadd(arg0, v0);
        pos.ins().brnz(v1, bb1, &[]);
        pos.ins().jump(bb2, &[]);
        pos.insert_block(bb1);
        pos.ins().brnz(v1, bb2, &[]);
        pos.ins().jump(bb3, &[]);
        pos.insert_block(bb2);
        let v2 = pos.ins().iadd(v1, v0);
        pos.ins().brnz(v2, bb2, &[]);
        pos.ins().jump(bb1, &[]);
        pos.insert_block(bb3);
        let v3 = pos.ins().isub(v1, v0);
        pos.ins().return_(&[v3]);

        let mut shared_flags_builder = settings::builder();
        shared_flags_builder.set("opt_level", "none").unwrap();
        let shared_flags = settings::Flags::new(shared_flags_builder);
        let isa_flags = aarch64_settings::Flags::new(&shared_flags, aarch64_settings::builder());
        let backend = AArch64Backend::new_with_flags(
            Triple::from_str("aarch64").unwrap(),
            shared_flags,
            isa_flags,
        );
        let result = backend
            .compile_function(&mut func, /* want_disasm = */ false)
            .unwrap();
        let code = result.buffer.data();

        // mov     x10, #0x1234                    // #4660
        // add     w12, w0, w10
        // mov     w11, w12
        // cbnz    x11, 0x20
        // mov     x13, #0x1234                    // #4660
        // add     w15, w12, w13
        // mov     w14, w15
        // cbnz    x14, 0x10
        // mov     w1, w12
        // cbnz    x1, 0x10
        // mov     x2, #0x1234                     // #4660
        // sub     w0, w12, w2
        // ret

        let golden = vec![
            138, 70, 130, 210, 12, 0, 10, 11, 235, 3, 12, 42, 171, 0, 0, 181, 141, 70, 130, 210,
            143, 1, 13, 11, 238, 3, 15, 42, 174, 255, 255, 181, 225, 3, 12, 42, 97, 255, 255, 181,
            130, 70, 130, 210, 128, 1, 2, 75, 192, 3, 95, 214,
        ];

        assert_eq!(code, &golden[..]);
    }

    #[test]
    fn test_br_table() {
        let name = ExternalName::testcase("test0");
        let mut sig = Signature::new(CallConv::SystemV);
        sig.params.push(AbiParam::new(I32));
        sig.returns.push(AbiParam::new(I32));
        let mut func = Function::with_name_signature(name, sig);

        let bb0 = func.dfg.make_block();
        let arg0 = func.dfg.append_block_param(bb0, I32);
        let bb1 = func.dfg.make_block();
        let bb2 = func.dfg.make_block();
        let bb3 = func.dfg.make_block();

        let mut pos = FuncCursor::new(&mut func);

        pos.insert_block(bb0);
        let mut jt_data = JumpTableData::new();
        jt_data.push_entry(bb1);
        jt_data.push_entry(bb2);
        let jt = pos.func.create_jump_table(jt_data);
        pos.ins().br_table(arg0, bb3, jt);

        pos.insert_block(bb1);
        let v1 = pos.ins().iconst(I32, 1);
        pos.ins().return_(&[v1]);

        pos.insert_block(bb2);
        let v2 = pos.ins().iconst(I32, 2);
        pos.ins().return_(&[v2]);

        pos.insert_block(bb3);
        let v3 = pos.ins().iconst(I32, 3);
        pos.ins().return_(&[v3]);

        let mut shared_flags_builder = settings::builder();
        shared_flags_builder.set("opt_level", "none").unwrap();
        shared_flags_builder.set("enable_verifier", "true").unwrap();
        let shared_flags = settings::Flags::new(shared_flags_builder);
        let isa_flags = aarch64_settings::Flags::new(&shared_flags, aarch64_settings::builder());
        let backend = AArch64Backend::new_with_flags(
            Triple::from_str("aarch64").unwrap(),
            shared_flags,
            isa_flags,
        );
        let result = backend
            .compile_function(&mut func, /* want_disasm = */ false)
            .unwrap();
        let code = result.buffer.data();

        //   0:   7100081f        cmp     w0, #0x2
        //   4:   54000102        b.cs    0x24  // b.hs, b.nlast
        //   8:   9a8023e9        csel    x9, xzr, x0, cs  // cs = hs, nlast
        //   c:   10000088        adr     x8, 0x1c
        //  10:   b8a95909        ldrsw   x9, [x8, w9, uxtw #2]
        //  14:   8b090108        add     x8, x8, x9
        //  18:   d61f0100        br      x8
        //  1c:   00000010        udf     #16
        //  20:   00000018        udf     #24
        //  24:   d2800060        mov     x0, #0x3                        // #3
        //  28:   d65f03c0        ret
        //  2c:   d2800020        mov     x0, #0x1                        // #1
        //  30:   d65f03c0        ret
        //  34:   d2800040        mov     x0, #0x2                        // #2
        //  38:   d65f03c0        ret

        let golden = vec![
            31, 8, 0, 113, 2, 1, 0, 84, 233, 35, 128, 154, 136, 0, 0, 16, 9, 89, 169, 184, 8, 1, 9,
            139, 0, 1, 31, 214, 16, 0, 0, 0, 24, 0, 0, 0, 96, 0, 128, 210, 192, 3, 95, 214, 32, 0,
            128, 210, 192, 3, 95, 214, 64, 0, 128, 210, 192, 3, 95, 214,
        ];

        assert_eq!(code, &golden[..]);
    }
}
