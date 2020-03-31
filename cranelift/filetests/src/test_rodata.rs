//! Test command for verifying the rodata emitted after each function
//!
//! The `rodata` test command runs each function through the full code generator pipeline

use crate::subtest::{run_filecheck, Context, SubTest, SubtestResult};
use cranelift_codegen;
use cranelift_codegen::binemit::{self, CodeInfo};
use cranelift_codegen::ir;
use cranelift_codegen::ir::{Function, Value};
use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::print_errors::pretty_error;
use cranelift_reader::TestCommand;
use log::info;
use std::borrow::Cow;

struct TestRodata;

pub fn subtest(parsed: &TestCommand) -> SubtestResult<Box<dyn SubTest>> {
    assert_eq!(parsed.command, "rodata");
    if !parsed.options.is_empty() {
        Err(format!("No options allowed on {}", parsed))
    } else {
        Ok(Box::new(TestRodata))
    }
}

impl SubTest for TestRodata {
    fn name(&self) -> &'static str {
        "rodata"
    }

    fn is_mutating(&self) -> bool {
        true
    }

    fn needs_isa(&self) -> bool {
        true
    }

    fn run(&self, func: Cow<ir::Function>, context: &Context) -> SubtestResult<()> {
        let isa = context.isa.expect("rodata needs an ISA");
        let mut comp_ctx = cranelift_codegen::Context::for_function(func.into_owned());

        let CodeInfo { total_size, .. } = comp_ctx
            .compile(isa)
            .map_err(|e| pretty_error(&comp_ctx.func, context.isa, e))?;

        info!(
            "Generated {} bytes of code:\n{}",
            total_size,
            comp_ctx.func.display(isa)
        );

        // Verify that the returned code size matches the emitted bytes.
        let mut sink = RodataSink::default();
        binemit::emit_function(
            &comp_ctx.func,
            |func, inst, div, sink, isa| isa.emit_inst(func, inst, div, sink),
            &mut sink,
            isa,
        );

        // Run final code through filecheck.
        let text = format!("{:X?}", sink.rodata);
        info!("Found rodata: {}", text);
        run_filecheck(&text, context)
    }
}

/// Code sink that only captures emitted rodata
#[derive(Default)]
struct RodataSink {
    offset: usize,
    rodata: Vec<u8>,
    in_rodata: bool,
}

impl binemit::CodeSink for RodataSink {
    fn offset(&self) -> binemit::CodeOffset {
        self.offset as u32
    }

    fn put1(&mut self, byte: u8) {
        self.offset += 1;
        if self.in_rodata {
            self.rodata.push(byte);
        }
    }

    fn put2(&mut self, bytes: u16) {
        self.offset += 2;
        if self.in_rodata {
            self.rodata.extend_from_slice(&bytes.to_be_bytes());
        }
    }

    fn put4(&mut self, bytes: u32) {
        self.offset += 4;
        if self.in_rodata {
            self.rodata.extend_from_slice(&bytes.to_be_bytes());
        }
    }

    fn put8(&mut self, bytes: u64) {
        self.offset += 8;
        if self.in_rodata {
            self.rodata.extend_from_slice(&bytes.to_be_bytes());
        }
    }

    fn reloc_block(&mut self, _reloc: binemit::Reloc, _block_offset: binemit::CodeOffset) {}
    fn reloc_external(
        &mut self,
        _: ir::SourceLoc,
        _: binemit::Reloc,
        _: &ir::ExternalName,
        _: binemit::Addend,
    ) {
    }
    fn reloc_constant(&mut self, _: binemit::Reloc, _: ir::ConstantOffset) {}
    fn reloc_jt(&mut self, _reloc: binemit::Reloc, _jt: ir::JumpTable) {}
    fn trap(&mut self, _code: ir::TrapCode, _srcloc: ir::SourceLoc) {}
    fn begin_jumptables(&mut self) {
        assert!(!self.in_rodata, "Jump tables must be emitted before rodata");
    }
    fn begin_rodata(&mut self) {
        self.in_rodata = true;
    }
    fn end_codegen(&mut self) {
        assert!(
            self.in_rodata,
            "Expected rodata to be emitted before the end of codegen"
        );
    }
    fn add_stackmap(&mut self, _: &[Value], _: &Function, _: &dyn TargetIsa) {}
}
