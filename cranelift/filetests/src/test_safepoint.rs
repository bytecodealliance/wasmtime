use crate::subtest::{run_filecheck, Context, SubTest, SubtestResult};
use cranelift_codegen::ir::Function;
use cranelift_codegen::print_errors::pretty_error;
use cranelift_reader::TestCommand;
use std::borrow::Cow;

struct TestSafepoint;

pub fn subtest(parsed: &TestCommand) -> SubtestResult<Box<dyn SubTest>> {
    assert_eq!(parsed.command, "safepoint");
    if !parsed.options.is_empty() {
        Err(format!("No options allowed on {}", parsed))
    } else {
        Ok(Box::new(TestSafepoint))
    }
}

impl SubTest for TestSafepoint {
    fn name(&self) -> &'static str {
        "safepoint"
    }

    fn run(&self, func: Cow<Function>, context: &Context) -> SubtestResult<()> {
        let mut comp_ctx = cranelift_codegen::Context::for_function(func.into_owned());

        let isa = context.isa.expect("register allocator needs an ISA");
        comp_ctx.compute_cfg();
        comp_ctx
            .legalize(isa)
            .map_err(|e| pretty_error(&comp_ctx.func, context.isa, e))?;
        comp_ctx.compute_domtree();
        comp_ctx
            .regalloc(isa)
            .map_err(|e| pretty_error(&comp_ctx.func, context.isa, e))?;

        let text = comp_ctx.func.display(context.isa).to_string();
        run_filecheck(&text, context)
    }
}
