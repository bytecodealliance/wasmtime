use crate::subtest::{run_filecheck, Context, SubTest};
use cranelift_codegen::ir::Function;
use cranelift_reader::TestCommand;
use std::borrow::Cow;

struct TestSafepoint;

pub fn subtest(parsed: &TestCommand) -> anyhow::Result<Box<dyn SubTest>> {
    assert_eq!(parsed.command, "safepoint");
    if !parsed.options.is_empty() {
        anyhow::bail!("No options allowed on {}", parsed);
    }
    Ok(Box::new(TestSafepoint))
}

impl SubTest for TestSafepoint {
    fn name(&self) -> &'static str {
        "safepoint"
    }

    fn run(&self, func: Cow<Function>, context: &Context) -> anyhow::Result<()> {
        let mut comp_ctx = cranelift_codegen::Context::for_function(func.into_owned());

        let isa = context.isa.expect("register allocator needs an ISA");
        comp_ctx.compute_cfg();
        comp_ctx
            .legalize(isa)
            .map_err(|e| crate::pretty_anyhow_error(&comp_ctx.func, e))?;
        comp_ctx.compute_domtree();

        let text = comp_ctx.func.display().to_string();
        run_filecheck(&text, context)
    }
}
