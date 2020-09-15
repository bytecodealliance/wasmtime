//! Test command for testing the Shrink pass.
//!
//! The `shrink` test command runs each function through the Shrink pass after ensuring
//! that all instructions are legal for the target.
//!
//! The resulting function is sent to `filecheck`.

use crate::subtest::{run_filecheck, Context, SubTest};
use cranelift_codegen;
use cranelift_codegen::ir::Function;
use cranelift_reader::TestCommand;
use std::borrow::Cow;

struct TestShrink;

pub fn subtest(parsed: &TestCommand) -> anyhow::Result<Box<dyn SubTest>> {
    assert_eq!(parsed.command, "shrink");
    if !parsed.options.is_empty() {
        anyhow::bail!("No options allowed on {}", parsed);
    }
    Ok(Box::new(TestShrink))
}

impl SubTest for TestShrink {
    fn name(&self) -> &'static str {
        "shrink"
    }

    fn is_mutating(&self) -> bool {
        true
    }

    fn run(&self, func: Cow<Function>, context: &Context) -> anyhow::Result<()> {
        let isa = context.isa.expect("shrink needs an ISA");
        let mut comp_ctx = cranelift_codegen::Context::for_function(func.into_owned());

        comp_ctx
            .shrink_instructions(isa)
            .map_err(|e| crate::pretty_anyhow_error(&comp_ctx.func, context.isa, Into::into(e)))?;

        let text = comp_ctx.func.display(isa).to_string();
        run_filecheck(&text, context)
    }
}
