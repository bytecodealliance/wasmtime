//! Test command for testing the preopt pass.
//!
//! The resulting function is sent to `filecheck`.

use crate::subtest::{run_filecheck, Context, SubTest};
use cranelift_codegen;
use cranelift_codegen::ir::Function;
use cranelift_reader::TestCommand;
use std::borrow::Cow;

struct TestSimplePreopt;

pub fn subtest(parsed: &TestCommand) -> anyhow::Result<Box<dyn SubTest>> {
    assert_eq!(parsed.command, "simple_preopt");
    if !parsed.options.is_empty() {
        anyhow::bail!("No options allowed on {}", parsed);
    }
    Ok(Box::new(TestSimplePreopt))
}

impl SubTest for TestSimplePreopt {
    fn name(&self) -> &'static str {
        "simple_preopt"
    }

    fn needs_isa(&self) -> bool {
        true
    }

    fn is_mutating(&self) -> bool {
        true
    }

    fn run(&self, func: Cow<Function>, context: &Context) -> anyhow::Result<()> {
        let mut comp_ctx = cranelift_codegen::Context::for_function(func.into_owned());
        let isa = context.isa.expect("preopt needs an ISA");

        comp_ctx.compute_cfg();
        comp_ctx
            .preopt(isa)
            .map_err(|e| crate::pretty_anyhow_error(&comp_ctx.func, e))?;
        let text = &comp_ctx.func.display().to_string();
        log::debug!("After simple_preopt:\n{}", text);
        run_filecheck(&text, context)
    }
}
