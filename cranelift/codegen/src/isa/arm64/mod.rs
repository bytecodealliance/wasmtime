//! ARM 64-bit Instruction Set Architecture.

use crate::ir::Function;
use crate::isa::Builder as IsaBuilder;
use crate::isa::TargetIsa;
use crate::machinst::{
    compile, MachBackend, MachCompileResult, ShowWithRRU, TargetIsaAdapter, VCode,
};
use crate::result::CodegenResult;
use crate::settings;

use alloc::boxed::Box;
use std::str::FromStr;

use regalloc::RealRegUniverse;
use target_lexicon::Triple;

// New backend:
mod abi;
mod inst;
mod lower;

use inst::create_reg_universe;

/// An ARM64 backend.
pub struct Arm64Backend {
    flags: settings::Flags,
}

impl Arm64Backend {
    /// Create a new ARM64 backend with the given (shared) flags.
    pub fn new_with_flags(flags: settings::Flags) -> Arm64Backend {
        Arm64Backend { flags }
    }

    fn compile_vcode(&self, mut func: Function, flags: &settings::Flags) -> VCode<inst::Inst> {
        // This performs lowering to VCode, register-allocates the code, computes
        // block layout and finalizes branches. The result is ready for binary emission.
        let abi = Box::new(abi::ARM64ABIBody::new(&func));
        compile::compile::<Arm64Backend>(&mut func, self, abi, flags)
    }
}

impl MachBackend for Arm64Backend {
    fn compile_function(
        &self,
        func: Function,
        want_disasm: bool,
    ) -> CodegenResult<MachCompileResult> {
        let flags = self.flags();
        let vcode = self.compile_vcode(func, flags);
        let sections = vcode.emit();
        let frame_size = vcode.frame_size();

        let disasm = if want_disasm {
            Some(vcode.show_rru(Some(&create_reg_universe())))
        } else {
            None
        };

        Ok(MachCompileResult {
            sections,
            frame_size,
            disasm,
        })
    }

    fn name(&self) -> &'static str {
        "arm64"
    }

    fn triple(&self) -> Triple {
        FromStr::from_str("arm64").unwrap()
    }

    fn flags(&self) -> &settings::Flags {
        &self.flags
    }

    fn reg_universe(&self) -> RealRegUniverse {
        create_reg_universe()
    }
}

/// Create a new `isa::Builder`.
pub fn isa_builder(triple: Triple) -> IsaBuilder {
    IsaBuilder {
        triple,
        setup: settings::builder(),
        constructor: isa_constructor,
    }
}

fn isa_constructor(
    _: Triple,
    shared_flags: settings::Flags,
    _arch_flag_builder: settings::Builder,
) -> Box<dyn TargetIsa> {
    let backend = Arm64Backend::new_with_flags(shared_flags);
    Box::new(TargetIsaAdapter::new(backend))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::binemit::{NullRelocSink, NullStackmapSink, NullTrapSink};
    use crate::cursor::{Cursor, FuncCursor};
    use crate::ir::types::*;
    use crate::ir::{AbiParam, ExternalName, Function, InstBuilder, Signature};
    use crate::isa::CallConv;
    use crate::settings;
    use crate::settings::Configurable;

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

        let mut shared_flags = settings::builder();
        shared_flags.set("opt_level", "none").unwrap();
        let backend = Arm64Backend::new_with_flags(settings::Flags::new(shared_flags));
        let sections = backend.compile_function(func, false).unwrap().sections;
        let code = &sections.sections[0].data;

        // stp x29, x30, [sp, #-16]!
        // mov x29, sp
        // mov x1, #0x1234
        // add w0, w0, w1
        // mov sp, x29
        // ldp x29, x30, [sp], #16
        // ret
        let golden = vec![
            0xfd, 0x7b, 0xbf, 0xa9, 0xfd, 0x03, 0x00, 0x91, 0x81, 0x46, 0x82, 0xd2, 0x00, 0x00,
            0x01, 0x0b, 0xbf, 0x03, 0x00, 0x91, 0xfd, 0x7b, 0xc1, 0xa8, 0xc0, 0x03, 0x5f, 0xd6,
        ];

        assert_eq!(code, &golden);
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

        let mut shared_flags = settings::builder();
        shared_flags.set("opt_level", "none").unwrap();
        let backend = Arm64Backend::new_with_flags(settings::Flags::new(shared_flags));
        let result = backend
            .compile_function(func, /* want_disasm = */ false)
            .unwrap();
        let code = &result.sections.sections[0].data;

        // stp	x29, x30, [sp, #-16]!
        // mov	x29, sp
        // mov	x1, x0
        // mov  x0, #0x1234
        // add	w1, w1, w0
        // mov	w2, w1
        // cbz	x2, ...
        // mov	w2, w1
        // cbz	x2, ...
        // sub	w0, w1, w0
        // mov	sp, x29
        // ldp	x29, x30, [sp], #16
        // ret
        // add	w2, w1, w0
        // mov	w2, w2
        // cbnz	x2, ... <---- compound branch (cond / uncond)
        // b ...        <----

        let golden = vec![
            0xfd, 0x7b, 0xbf, 0xa9, 0xfd, 0x03, 0x00, 0x91, 0xe1, 0x03, 0x00, 0xaa, 0x80, 0x46,
            0x82, 0xd2, 0x21, 0x00, 0x00, 0x0b, 0xe2, 0x03, 0x01, 0x2a, 0xe2, 0x00, 0x00, 0xb4,
            0xe2, 0x03, 0x01, 0x2a, 0xa2, 0x00, 0x00, 0xb5, 0x20, 0x00, 0x00, 0x4b, 0xbf, 0x03,
            0x00, 0x91, 0xfd, 0x7b, 0xc1, 0xa8, 0xc0, 0x03, 0x5f, 0xd6, 0x22, 0x00, 0x00, 0x0b,
            0xe2, 0x03, 0x02, 0x2a, 0xc2, 0xff, 0xff, 0xb5, 0xf7, 0xff, 0xff, 0x17,
        ];

        assert_eq!(code, &golden);
    }
}
