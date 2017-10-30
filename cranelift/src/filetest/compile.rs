//! Test command for testing the code generator pipeline
//!
//! The `compile` test command runs each function through the full code generator pipeline

use cretonne::binemit;
use cretonne::ir;
use cretonne;
use cton_reader::TestCommand;
use filetest::subtest::{SubTest, Context, Result};
use std::borrow::Cow;
use utils::pretty_error;

struct TestCompile;

pub fn subtest(parsed: &TestCommand) -> Result<Box<SubTest>> {
    assert_eq!(parsed.command, "compile");
    if !parsed.options.is_empty() {
        Err(format!("No options allowed on {}", parsed))
    } else {
        Ok(Box::new(TestCompile))
    }
}

impl SubTest for TestCompile {
    fn name(&self) -> Cow<str> {
        Cow::from("compile")
    }

    fn is_mutating(&self) -> bool {
        true
    }

    fn needs_isa(&self) -> bool {
        true
    }

    fn run(&self, func: Cow<ir::Function>, context: &Context) -> Result<()> {
        let isa = context.isa.expect("compile needs an ISA");

        // Create a compilation context, and drop in the function.
        let mut comp_ctx = cretonne::Context::new();
        comp_ctx.func = func.into_owned();

        let code_size = comp_ctx.compile(isa).map_err(|e| {
            pretty_error(&comp_ctx.func, context.isa, e)
        })?;

        dbg!(
            "Generated {} bytes of code:\n{}",
            code_size,
            comp_ctx.func.display(isa)
        );

        // Finally verify that the returned code size matches the emitted bytes.
        let mut sink = SizeSink { offset: 0 };
        binemit::emit_function(
            &comp_ctx.func,
            |func, inst, div, sink| isa.emit_inst(func, inst, div, sink),
            &mut sink,
        );

        if sink.offset != code_size {
            return Err(format!(
                "Expected code size {}, got {}",
                code_size,
                sink.offset
            ));
        }

        Ok(())
    }
}

// Code sink that simply counts bytes.
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

    fn reloc_ebb(&mut self, _reloc: binemit::Reloc, _ebb: ir::Ebb) {}
    fn reloc_func(&mut self, _reloc: binemit::Reloc, _fref: ir::FuncRef) {}
    fn reloc_globalsym(&mut self, _reloc: binemit::Reloc, _global: ir::GlobalVar) {}
    fn reloc_jt(&mut self, _reloc: binemit::Reloc, _jt: ir::JumpTable) {}
}
