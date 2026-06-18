use crate::subtest::{Context, SubTest, run_filecheck};
use cranelift_codegen::ir::Function;
use cranelift_reader::TestCommand;
use std::borrow::Cow;

struct TestSafepoint;

pub fn subtest(parsed: &TestCommand) -> anyhow::Result<Box<dyn SubTest>> {
    assert_eq!(parsed.command, "safepoint");
    if !parsed.options.is_empty() {
        anyhow::bail!("No options allowed on {parsed}");
    }
    Ok(Box::new(TestSafepoint))
}

impl SubTest for TestSafepoint {
    fn name(&self) -> &'static str {
        "safepoint"
    }

    fn run(&self, func: Cow<Function>, context: &Context) -> anyhow::Result<()> {
        let mut comp_ctx = cranelift_codegen::Context::for_function(func.into_owned());

        comp_ctx.compute_cfg();
        comp_ctx.compute_domtree();

        let text = comp_ctx.func.display().to_string();
        run_filecheck(&text, context)
    }
}
