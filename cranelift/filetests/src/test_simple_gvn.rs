//! Test command for testing the simple GVN pass.
//!
//! The `simple-gvn` test command runs each function through the simple GVN pass after ensuring
//! that all instructions are legal for the target.
//!
//! The resulting function is sent to `filecheck`.

use crate::subtest::{run_filecheck, Context, SubTest};
use cranelift_codegen;
use cranelift_codegen::ir::Function;
use cranelift_reader::TestCommand;
use std::borrow::Cow;

struct TestSimpleGVN;

pub fn subtest(parsed: &TestCommand) -> anyhow::Result<Box<dyn SubTest>> {
    assert_eq!(parsed.command, "simple-gvn");
    if !parsed.options.is_empty() {
        anyhow::bail!("No options allowed on {}", parsed);
    }
    Ok(Box::new(TestSimpleGVN))
}

impl SubTest for TestSimpleGVN {
    fn name(&self) -> &'static str {
        "simple-gvn"
    }

    fn is_mutating(&self) -> bool {
        true
    }

    fn run(&self, func: Cow<Function>, context: &Context) -> anyhow::Result<()> {
        let mut comp_ctx = cranelift_codegen::Context::for_function(func.into_owned());

        comp_ctx.flowgraph();
        comp_ctx
            .simple_gvn(context.flags_or_isa())
            .map_err(|e| crate::pretty_anyhow_error(&comp_ctx.func, Into::into(e)))?;

        let text = comp_ctx.func.display().to_string();
        run_filecheck(&text, context)
    }
}
