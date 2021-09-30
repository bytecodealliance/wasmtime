use crate::subtest::{run_filecheck, Context, SubTest};
use cranelift_codegen::binemit::{self, Addend, CodeOffset, CodeSink, Reloc, StackMap};
use cranelift_codegen::ir::*;
use cranelift_codegen::isa::TargetIsa;
use cranelift_reader::TestCommand;
use std::borrow::Cow;
use std::fmt::Write;

struct TestStackMaps;

pub fn subtest(parsed: &TestCommand) -> anyhow::Result<Box<dyn SubTest>> {
    assert_eq!(parsed.command, "stack_maps");
    if !parsed.options.is_empty() {
        anyhow::bail!("No options allowed on {}", parsed);
    }
    Ok(Box::new(TestStackMaps))
}

impl SubTest for TestStackMaps {
    fn name(&self) -> &'static str {
        "stack_maps"
    }

    fn run(&self, func: Cow<Function>, context: &Context) -> anyhow::Result<()> {
        let mut comp_ctx = cranelift_codegen::Context::for_function(func.into_owned());

        comp_ctx
            .compile(context.isa.expect("`test stack_maps` requires an isa"))
            .map_err(|e| crate::pretty_anyhow_error(&comp_ctx.func, context.isa, e))?;

        let mut sink = TestStackMapsSink::default();
        // TODO remove entirely? seems a bit meaningless now
        binemit::emit_function(
            &comp_ctx.func,
            |func, inst, sink, isa| {
                if func.dfg[inst].opcode() == Opcode::Safepoint {
                    writeln!(&mut sink.text, "{}", func.dfg.display_inst(inst, isa)).unwrap();
                }
            },
            &mut sink,
            context.isa.expect("`test stack_maps` requires an isa"),
        );

        let mut text = comp_ctx.func.display(context.isa).to_string();
        text.push('\n');
        text.push_str("Stack maps:\n");
        text.push('\n');
        text.push_str(&sink.text);

        run_filecheck(&text, context)
    }
}

#[derive(Default)]
struct TestStackMapsSink {
    offset: u32,
    text: String,
}

impl CodeSink for TestStackMapsSink {
    fn offset(&self) -> CodeOffset {
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

    fn reloc_external(&mut self, _: SourceLoc, _: Reloc, _: &ExternalName, _: Addend) {}
    fn reloc_constant(&mut self, _: Reloc, _: ConstantOffset) {}
    fn reloc_jt(&mut self, _: Reloc, _: JumpTable) {}
    fn trap(&mut self, _: TrapCode, _: SourceLoc) {}
    fn begin_jumptables(&mut self) {}
    fn begin_rodata(&mut self) {}
    fn end_codegen(&mut self) {}

    fn add_stack_map(&mut self, val_list: &[Value], func: &Function, isa: &dyn TargetIsa) {
        let map = StackMap::from_values(&val_list, func, isa);

        writeln!(&mut self.text, "  - mapped words: {}", map.mapped_words()).unwrap();
        write!(&mut self.text, "  - live: [").unwrap();

        let mut needs_comma_space = false;
        for i in 0..(map.mapped_words() as usize) {
            if map.get_bit(i) {
                if needs_comma_space {
                    write!(&mut self.text, ", ").unwrap();
                }
                needs_comma_space = true;

                write!(&mut self.text, "{}", i).unwrap();
            }
        }

        writeln!(&mut self.text, "]").unwrap();
    }
}
