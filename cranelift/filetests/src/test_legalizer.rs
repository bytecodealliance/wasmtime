//! Test command for checking the IR legalizer.
//!
//! The `test legalizer` test command runs each function through `legalize_function()` and sends
//! the result to filecheck.

use crate::subtest::{run_filecheck, Context, SubTest};
use cranelift_codegen;
use cranelift_codegen::ir::Function;
use cranelift_reader::TestCommand;
use std::borrow::Cow;

struct TestLegalizer;

pub fn subtest(parsed: &TestCommand) -> anyhow::Result<Box<dyn SubTest>> {
    assert_eq!(parsed.command, "legalizer");
    if !parsed.options.is_empty() {
        anyhow::bail!("No options allowed on {}", parsed);
    }
    Ok(Box::new(TestLegalizer))
}

impl SubTest for TestLegalizer {
    fn name(&self) -> &'static str {
        "legalizer"
    }

    fn is_mutating(&self) -> bool {
        true
    }

    fn needs_isa(&self) -> bool {
        true
    }

    fn run(&self, func: Cow<Function>, context: &Context) -> anyhow::Result<()> {
        let mut comp_ctx = cranelift_codegen::Context::for_function(func.into_owned());
        let isa = context.isa.expect("legalizer needs an ISA");

        comp_ctx.compute_cfg();
        comp_ctx
            .legalize(isa)
            .map_err(|e| crate::pretty_anyhow_error(&comp_ctx.func, e))?;

        let text = comp_ctx.func.display().to_string();
        run_filecheck(&text, context)
    }
}
