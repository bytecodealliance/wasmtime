//! Test command for testing the code generator pipeline
//!
//! The `compile` test command runs each function through the full code generator pipeline

use crate::subtest::{run_filecheck, Context, SubTest};
use cranelift_codegen;
use cranelift_codegen::binemit::{self, CodeInfo};
use cranelift_codegen::ir;
use cranelift_reader::TestCommand;
use log::info;
use std::borrow::Cow;

struct TestCompile;

pub fn subtest(parsed: &TestCommand) -> anyhow::Result<Box<dyn SubTest>> {
    assert_eq!(parsed.command, "compile");
    if !parsed.options.is_empty() {
        anyhow::bail!("No options allowed on {}", parsed);
    }
    Ok(Box::new(TestCompile))
}

impl SubTest for TestCompile {
    fn name(&self) -> &'static str {
        "compile"
    }

    fn is_mutating(&self) -> bool {
        true
    }

    fn needs_isa(&self) -> bool {
        true
    }

    fn run(&self, func: Cow<ir::Function>, context: &Context) -> anyhow::Result<()> {
        let isa = context.isa.expect("compile needs an ISA");
        let mut comp_ctx = cranelift_codegen::Context::for_function(func.into_owned());

        // With `MachBackend`s, we need to explicitly request dissassembly results.
        comp_ctx.set_disasm(true);

        let CodeInfo { total_size, .. } = comp_ctx
            .compile(isa)
            .map_err(|e| crate::pretty_anyhow_error(&comp_ctx.func, e))?;

        info!(
            "Generated {} bytes of code:\n{}",
            total_size,
            comp_ctx.func.display()
        );

        let disasm = comp_ctx
            .mach_compile_result
            .as_ref()
            .unwrap()
            .disasm
            .as_ref()
            .unwrap();
        run_filecheck(&disasm, context)
    }
}

/// Code sink that simply counts bytes.
struct SizeSink {
    offset: binemit::CodeOffset,
}

impl binemit::CodeSink for SizeSink {
    fn offset(&self) -> binemit::CodeOffset {
        self.offset
    }

    fn put1(&mut self, _: u8) {
        self.offset += 1;
    }

    fn put2(&mut self, _: u16) {
        self.offset += 2;
    }

    fn put4(&mut self, _: u32) {
        self.offset += 4;
    }

    fn put8(&mut self, _: u64) {
        self.offset += 8;
    }

    fn reloc_external(
        &mut self,
        _srcloc: ir::SourceLoc,
        _reloc: binemit::Reloc,
        _name: &ir::ExternalName,
        _addend: binemit::Addend,
    ) {
    }
    fn reloc_constant(&mut self, _: binemit::Reloc, _: ir::ConstantOffset) {}
    fn trap(&mut self, _code: ir::TrapCode, _srcloc: ir::SourceLoc) {}
    fn begin_jumptables(&mut self) {}
    fn begin_rodata(&mut self) {}
    fn end_codegen(&mut self) {}
}
