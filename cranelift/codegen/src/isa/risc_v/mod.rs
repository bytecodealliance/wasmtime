//! ARM 64-bit Instruction Set Architecture.

use crate::ir::condcodes::IntCC;
use crate::ir::Function;
use crate::ir::MemFlags;
use crate::ir::{StackSlotData, StackSlotKind};

use crate::isa::risc_v::settings as riscv_settings;
use crate::isa::{Builder as IsaBuilder, TargetIsa};
use crate::machinst::{
    compile, MachCompileResult, MachTextSectionBuilder, TextSectionBuilder, VCode,
};
use crate::result::CodegenResult;
use crate::settings as shared_settings;
use alloc::{boxed::Box, vec::Vec};
use core::fmt;
use regalloc2::MachineEnv;
use target_lexicon::{
    Aarch64Architecture, Architecture, BinaryFormat, OperatingSystem, Riscv64Architecture, Triple,
};

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
    /// Create a new AArch64 backend with the given (shared) flags.
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

        // if let Some(disasm) = emit_result.disasm.as_ref() {
        //     log::debug!("disassembly:\n{}", disasm);
        // }

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
        "risc-v64"
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
    assert!(triple.architecture == Architecture::Aarch64(Aarch64Architecture::Aarch64));
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

use log::{Level, LevelFilter, Metadata, Record};

struct SimpleLogger(Level);

static SIMPLE_LOGGER: SimpleLogger = SimpleLogger(Level::Trace);

impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.0
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            println!("{} - {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}

pub fn init_logger() {
    log::set_logger(&SIMPLE_LOGGER)
        .map(|()| log::set_max_level(LevelFilter::max()))
        .unwrap()
}

#[cfg(test)]
mod test {
    use alloc::vec;

    use super::*;
    use crate::cursor::{Cursor, FuncCursor};
    use crate::ir::condcodes::FloatCC;
    use crate::ir::{types::*, JumpTable, JumpTableData};
    use crate::ir::{AbiParam, ExternalName, Function, InstBuilder, Signature};
    use crate::isa::CallConv;
    use crate::settings;
    use crate::settings::Configurable;
    use std::io;

    #[test]
    fn hello_world() {
        init_logger();
        let name = ExternalName::testcase("test0");
        let mut sig = Signature::new(CallConv::SystemV);
        sig.params.push(AbiParam::new(I32));
        sig.params.push(AbiParam::new(I32));
        sig.returns.push(AbiParam::new(I32));
        sig.returns.push(AbiParam::new(I32));

        let mut func = Function::with_name_signature(name, sig);
        func.create_stack_slot(StackSlotData {
            kind: StackSlotKind::ExplicitSlot,
            size: 10 * 1024 * 1024,
        });
        let bb0 = func.dfg.make_block();
        let arg0 = func.dfg.append_block_param(bb0, I32);
        let arg1 = func.dfg.append_block_param(bb0, I32);
        let mut pos = FuncCursor::new(&mut func);
        pos.insert_block(bb0);
        let v1 = pos.ins().iadd(arg0, arg1);
        let v2 = pos.ins().iconst(I32, 100);
        let v3 = pos.ins().iadd(v1, v2);

        pos.ins().return_(&[v1, v3]);
        let mut shared_flags_builder = settings::builder();
        shared_flags_builder.set("opt_level", "none").unwrap();
        let shared_flags = settings::Flags::new(shared_flags_builder);
        let isa_flags = riscv_settings::Flags::new(&shared_flags, riscv_settings::builder());
        let backend = Riscv64Backend::new_with_flags(TRIPLE.clone(), shared_flags, isa_flags);
        let result = backend
            .compile_function(&mut func, /* want_disasm = */ true)
            .unwrap();
        let _code = result.buffer.data();
        let disasm = result.disasm.unwrap();
        println!("{}", disasm);
    }

    #[test]
    fn hello_world_branch() {
        init_logger();
        let name = ExternalName::testcase("test0");
        let mut sig = Signature::new(CallConv::SystemV);
        sig.params.push(AbiParam::new(I32));
        sig.returns.push(AbiParam::new(I32));

        let mut func = Function::with_name_signature(name, sig);
        let bb0 = func.dfg.make_block();
        let bb1 = func.dfg.make_block();
        let bb2 = func.dfg.make_block();
        let arg0 = func.dfg.append_block_param(bb0, I32);
        let mut pos = FuncCursor::new(&mut func);
        pos.insert_block(bb0);

        let _ = pos.ins().brz(arg0, bb1, &[]);
        let _ = pos.ins().jump(bb2, &[]);
        pos.insert_block(bb1);
        let v1 = pos.ins().iconst(I32, 1);
        pos.ins().return_(&[v1]);

        pos.insert_block(bb2);
        let v2 = pos.ins().iconst(I32, 2);
        let _v3 = pos.ins().load(I32, MemFlags::new(), v2, 100);
        pos.ins().store(MemFlags::new(), _v3, v2, 200);
        pos.ins().atomic_load(I32, MemFlags::new(), v2);
        pos.ins().return_(&[v2]);

        let mut shared_flags_builder = settings::builder();
        shared_flags_builder.set("opt_level", "none").unwrap();
        let shared_flags = settings::Flags::new(shared_flags_builder);
        let isa_flags = riscv_settings::Flags::new(&shared_flags, riscv_settings::builder());
        let backend = Riscv64Backend::new_with_flags(TRIPLE.clone(), shared_flags, isa_flags);
        let result = backend
            .compile_function(&mut func, /* want_disasm = */ true)
            .unwrap();
        let _code = result.buffer.data();
        let disasm = result.disasm.unwrap();
        println!("{}", disasm);
        println!("{:?}", _code);
        use std::io::Write;
        let mut file = std::fs::File::create("d://xxx.bin").unwrap();
        file.write_all(_code).unwrap();
    }

    #[test]
    fn some_float_compare() {
        init_logger();
        let name = ExternalName::testcase("test0");
        let mut sig = Signature::new(CallConv::SystemV);
        sig.params.push(AbiParam::new(F32));
        sig.params.push(AbiParam::new(F32));
        sig.returns.push(AbiParam::new(F32));
        let mut func = Function::with_name_signature(name, sig);

        let bb0 = func.dfg.make_block();
        let bb1 = func.dfg.make_block();
        let bb2 = func.dfg.make_block();
        let arg0 = func.dfg.append_block_param(bb0, F32);
        let arg1 = func.dfg.append_block_param(bb0, F32);

        let mut pos = FuncCursor::new(&mut func);
        pos.insert_block(bb0);

        let v1 = pos.ins().fcmp(FloatCC::GreaterThan, arg0, arg1);

        pos.ins().brnz(v1, bb1, &[]);
        pos.ins().jump(bb2, &[]);

        pos.insert_block(bb1);
        pos.ins().return_(&[arg0]);

        pos.insert_block(bb2);

        pos.ins().return_(&[arg1]);

        let mut shared_flags_builder = settings::builder();
        shared_flags_builder.set("opt_level", "none").unwrap();
        let shared_flags = settings::Flags::new(shared_flags_builder);
        let isa_flags = riscv_settings::Flags::new(&shared_flags, riscv_settings::builder());
        let backend = Riscv64Backend::new_with_flags(TRIPLE.clone(), shared_flags, isa_flags);
        let result = backend
            .compile_function(&mut func, /* want_disasm = */ true)
            .unwrap();
        let _code = result.buffer.data();
        let disasm = result.disasm.unwrap();
        println!("{}", disasm);
        println!("{:?}", _code);
        use std::io::Write;
        let mut file = std::fs::File::create("d://xxx.bin").unwrap();
        file.write_all(_code).unwrap();
    }

    #[test]
    fn test_compile_function() {
        // let name = ExternalName::testcase("test0");
        // let mut sig = Signature::new(CallConv::SystemV);
        // sig.params.push(AbiParam::new(I32));
        // sig.returns.push(AbiParam::new(I32));
        // let mut func = Function::with_name_signature(name, sig);

        // let bb0 = func.dfg.make_block();
        // let arg0 = func.dfg.append_block_param(bb0, I32);

        // let mut pos = FuncCursor::new(&mut func);
        // pos.insert_block(bb0);
        // let v0 = pos.ins().iconst(I32, 0x1234);
        // let v1 = pos.ins().iadd(arg0, v0);
        // pos.ins().return_(&[v1]);

        // let mut shared_flags_builder = settings::builder();
        // shared_flags_builder.set("opt_level", "none").unwrap();
        // let shared_flags = settings::Flags::new(shared_flags_builder);
        // let isa_flags = aarch64_settings::Flags::new(&shared_flags, aarch64_settings::builder());
        // let backend = AArch64Backend::new_with_flags(
        //     Triple::from_str("risc_v").unwrap(),
        //     shared_flags,
        //     isa_flags,
        // );
        // let buffer = backend.compile_function(&mut func, false).unwrap().buffer;
        // let code = buffer.data();

        // // mov x1, #0x1234
        // // add w0, w0, w1
        // // ret
        // let golden = vec![
        //     0x81, 0x46, 0x82, 0xd2, 0x00, 0x00, 0x01, 0x0b, 0xc0, 0x03, 0x5f, 0xd6,
        // ];

        // assert_eq!(code, &golden[..]);
    }

    #[test]
    fn test_branch_lowering() {
        // let name = ExternalName::testcase("test0");
        // let mut sig = Signature::new(CallConv::SystemV);
        // sig.params.push(AbiParam::new(I32));
        // sig.returns.push(AbiParam::new(I32));
        // let mut func = Function::with_name_signature(name, sig);

        // let bb0 = func.dfg.make_block();
        // let arg0 = func.dfg.append_block_param(bb0, I32);
        // let bb1 = func.dfg.make_block();
        // let bb2 = func.dfg.make_block();
        // let bb3 = func.dfg.make_block();

        // let mut pos = FuncCursor::new(&mut func);
        // pos.insert_block(bb0);
        // let v0 = pos.ins().iconst(I32, 0x1234);
        // let v1 = pos.ins().iadd(arg0, v0);
        // pos.ins().brnz(v1, bb1, &[]);
        // pos.ins().jump(bb2, &[]);
        // pos.insert_block(bb1);
        // pos.ins().brnz(v1, bb2, &[]);
        // pos.ins().jump(bb3, &[]);
        // pos.insert_block(bb2);
        // let v2 = pos.ins().iadd(v1, v0);
        // pos.ins().brnz(v2, bb2, &[]);
        // pos.ins().jump(bb1, &[]);
        // pos.insert_block(bb3);
        // let v3 = pos.ins().isub(v1, v0);
        // pos.ins().return_(&[v3]);

        // let mut shared_flags_builder = settings::builder();
        // shared_flags_builder.set("opt_level", "none").unwrap();
        // let shared_flags = settings::Flags::new(shared_flags_builder);
        // let isa_flags = aarch64_settings::Flags::new(&shared_flags, aarch64_settings::builder());
        // let backend = AArch64Backend::new_with_flags(
        //     Triple::from_str("risc_v").unwrap(),
        //     shared_flags,
        //     isa_flags,
        // );
        // let result = backend
        //     .compile_function(&mut func, /* want_disasm = */ false)
        //     .unwrap();
        // let code = result.buffer.data();

        // // mov	x1, #0x1234                	// #4660
        // // add	w0, w0, w1
        // // mov	w1, w0
        // // cbnz	x1, 0x28
        // // mov	x1, #0x1234                	// #4660
        // // add	w1, w0, w1
        // // mov	w1, w1
        // // cbnz	x1, 0x18
        // // mov	w1, w0
        // // cbnz	x1, 0x18
        // // mov	x1, #0x1234                	// #4660
        // // sub	w0, w0, w1
        // // ret
        // let golden = vec![
        //     129, 70, 130, 210, 0, 0, 1, 11, 225, 3, 0, 42, 161, 0, 0, 181, 129, 70, 130, 210, 1, 0,
        //     1, 11, 225, 3, 1, 42, 161, 255, 255, 181, 225, 3, 0, 42, 97, 255, 255, 181, 129, 70,
        //     130, 210, 0, 0, 1, 75, 192, 3, 95, 214,
        // ];

        // assert_eq!(code, &golden[..]);
    }

    #[test]
    fn hello_world2() {
        init_logger();
        let name = ExternalName::testcase("test0");
        let sig = Signature::new(CallConv::SystemV);
        let mut func = Function::with_name_signature(name, sig);
        let bb0 = func.dfg.make_block();
        let mut pos = FuncCursor::new(&mut func);
        pos.insert_block(bb0);
        pos.ins().return_(&[]);
        let mut shared_flags_builder = settings::builder();
        shared_flags_builder.set("opt_level", "none").unwrap();
        let shared_flags = settings::Flags::new(shared_flags_builder);
        let isa_flags = riscv_settings::Flags::new(&shared_flags, riscv_settings::builder());
        let backend = Riscv64Backend::new_with_flags(TRIPLE.clone(), shared_flags, isa_flags);
        let result = backend
            .compile_function(&mut func, /* want_disasm = */ true)
            .unwrap();
        let _code = result.buffer.data();

        println!("xxxxxx , {}", result.disasm.unwrap());
    }

    #[test]
    fn i128_compare() {
        init_logger();
        let name = ExternalName::testcase("test0");
        let mut sig = Signature::new(CallConv::SystemV);
        sig.returns.push(AbiParam::new(I32));
        let mut func = Function::with_name_signature(name, sig);
        let bb0 = func.dfg.make_block();
        let bb1 = func.dfg.make_block();
        let bb2 = func.dfg.make_block();
        let mut pos = FuncCursor::new(&mut func);
        pos.insert_block(bb0);
        let v1 = pos.ins().iconst(I128, 100);
        let v2 = pos.ins().iconst(I128, 200);
        pos.ins()
            .br_icmp(IntCC::SignedGreaterThan, v1, v2, bb1, &[]);

        pos.ins().jump(bb2, &[]);
        pos.insert_block(bb1);
        let v3 = pos.ins().iconst(I32, 0);
        pos.ins().return_(&[v3]);

        pos.insert_block(bb2);
        let v4 = pos.ins().iconst(I32, 1);
        pos.ins().return_(&[v4]);

        let mut shared_flags_builder = settings::builder();
        shared_flags_builder.set("opt_level", "none").unwrap();
        let shared_flags = settings::Flags::new(shared_flags_builder);
        let isa_flags = riscv_settings::Flags::new(&shared_flags, riscv_settings::builder());
        let backend = Riscv64Backend::new_with_flags(TRIPLE.clone(), shared_flags, isa_flags);
        let result = backend
            .compile_function(&mut func, /* want_disasm = */ true)
            .unwrap();
        let _code = result.buffer.data();
        println!("xxxxxx , {}", result.disasm.unwrap());
    }

    #[test]
    fn br_table() {
        init_logger();
        let name = ExternalName::testcase("test0");
        let mut sig = Signature::new(CallConv::SystemV);
        let mut func = Function::with_name_signature(name, sig);
        let bb0 = func.dfg.make_block();
        let bb1 = func.dfg.make_block();
        let bb2 = func.dfg.make_block();
        let bb3 = func.dfg.make_block();
        let mut jump_table_data = JumpTableData::new();
        jump_table_data.push_entry(bb1);
        jump_table_data.push_entry(bb2);
        let jump_table = func.create_jump_table(jump_table_data);

        let mut pos = FuncCursor::new(&mut func);
        pos.insert_block(bb0);
        let v1 = pos.ins().iconst(I32, 1);
        pos.ins().br_table(v1, bb3, jump_table);
        pos.insert_block(bb1);

        pos.ins().return_(&[]);
        pos.insert_block(bb2);
        pos.ins().return_(&[]);
        pos.insert_block(bb3);
        pos.ins().return_(&[]);

        let mut shared_flags_builder = settings::builder();
        shared_flags_builder.set("opt_level", "none").unwrap();
        let shared_flags = settings::Flags::new(shared_flags_builder);
        let isa_flags = riscv_settings::Flags::new(&shared_flags, riscv_settings::builder());
        let backend = Riscv64Backend::new_with_flags(TRIPLE.clone(), shared_flags, isa_flags);
        let result = backend
            .compile_function(&mut func, /* want_disasm = */ true)
            .unwrap();
        let _code = result.buffer.data();
        println!("xxxxxx , {}", result.disasm.unwrap());
        use std::io::Write;
        let mut file = std::fs::File::create("d://xxx.bin").unwrap();
        file.write_all(_code).unwrap();
    }
}

static TRIPLE: Triple = Triple {
    architecture: Architecture::Riscv64(Riscv64Architecture::Riscv64),
    vendor: target_lexicon::Vendor::Unknown,
    operating_system: OperatingSystem::Unknown,
    environment: target_lexicon::Environment::Unknown,
    binary_format: BinaryFormat::Unknown,
};
