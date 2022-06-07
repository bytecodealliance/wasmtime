//! ARM 64-bit Instruction Set Architecture.

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
        let emit_info = EmitInfo::new(flags.clone());
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
        "riscv64"
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
    /*
    todo:: what is the difference???
        Riscv64,
        Riscv64gc,
        Riscv64imac,
    */
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
    use crate::ir::{types::*, JumpTableData};
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
        let buffer = backend.compile_function(&mut func, false).unwrap().buffer;
        let code = buffer.data();
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
        let isa_flags = riscv_settings::Flags::new(&shared_flags, riscv_settings::builder());
        let backend = Riscv64Backend::new_with_flags(
            Triple::from_str("riscv64gc").unwrap(),
            shared_flags,
            isa_flags,
        );
        let result = backend
            .compile_function(&mut func, /* want_disasm = */ false)
            .unwrap();
        let code = result.buffer.data();
        // write_to_a_file("/home/yuyang/tmp/code.bin", code);
        // 0:   00001737                lui     a4,0x1
        // 4:   23470713                addi    a4,a4,564 # 0x1234
        // 8:   00e508bb                addw    a7,a0,a4
        // c:   00089a63                bnez    a7,0x20
        //10:   00001e37                lui     t3,0x1  //bb2
        //14:   234e0e13                addi    t3,t3,564 # 0x1234
        //18:   01c8833b                addw    t1,a7,t3
        //1c:   fe031ae3                bnez    t1,0x10
        //20:   fe0898e3                bnez    a7,0x10  // bb1
        //24:   000015b7                lui     a1,0x1
        //28:   23458593                addi    a1,a1,564 # 0x1234
        //2c:   40b8853b                subw    a0,a7,a1
        //30:   00008067                ret

        let golden = vec![
            55, 23, 0, 0, 19, 7, 71, 35, 187, 8, 229, 0, 99, 154, 8, 0, 55, 30, 0, 0, 19, 14, 78,
            35, 59, 131, 200, 1, 227, 26, 3, 254, 227, 152, 8, 254, 183, 21, 0, 0, 147, 133, 69,
            35, 59, 133, 184, 64, 103, 128, 0, 0,
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
        let isa_flags = riscv_settings::Flags::new(&shared_flags, riscv_settings::builder());
        let backend = Riscv64Backend::new_with_flags(
            Triple::from_str("riscv64gc").unwrap(),
            shared_flags,
            isa_flags,
        );
        let result = backend
            .compile_function(&mut func, /* want_disasm = */ false)
            .unwrap();
        let code = result.buffer.data();
        // write_to_a_file("/home/yuyang/tmp/code.bin", code);
        // 0:   02054663                bltz    a0,0x2c
        // 4:   00206693                ori     a3,zero,2
        // 8:   02d57263                bgeu    a0,a3,0x2c
        // c:   00000697                auipc   a3,0x0
        //10:   00351f93                slli    t6,a0,0x3
        //14:   01f686b3                add     a3,a3,t6
        //18:   01068067                jr      16(a3) # 0x1c
        //1c:   00000f97                auipc   t6,0x0
        //20:   018f8067                jr      24(t6) # 0x34
        //24:   00000f97                auipc   t6,0x0
        //28:   018f8067                jr      24(t6) # 0x3c
        //2c:   00306513                ori     a0,zero,3
        //30:   00008067                ret
        //34:   00106513                ori     a0,zero,1
        //38:   00008067                ret
        //3c:   00206513                ori     a0,zero,2
        //40:   00008067                ret

        let golden = vec![
            99, 70, 5, 2, 147, 102, 32, 0, 99, 114, 213, 2, 151, 6, 0, 0, 147, 31, 53, 0, 179, 134,
            246, 1, 103, 128, 6, 1, 151, 15, 0, 0, 103, 128, 143, 1, 151, 15, 0, 0, 103, 128, 143,
            1, 19, 101, 48, 0, 103, 128, 0, 0, 19, 101, 16, 0, 103, 128, 0, 0, 19, 101, 32, 0, 103,
            128, 0, 0,
        ];

        assert_eq!(code, &golden[..]);
    }

    /*
        some time I want to write code to a file, So I can examine use gnu tool chain.
        keep it here, it is fine.
    */
    fn write_to_a_file(file: &str, data: &[u8]) {
        use std::io::Write;
        let mut file = std::fs::File::create(file).expect("create failed");
        file.write_all(data).expect("write failed");
    }
}
