//! Test command for testing the alias analysis pass.
//!
//! The `alias-analysis` test command runs each function through GVN
//! and then alias analysis after ensuring that all instructions are
//! legal for the target.
//!
//! The resulting function is sent to `filecheck`.

use crate::subtest::{Context, SubTest, run_filecheck};
use cranelift_codegen::ir::Function;
use cranelift_reader::TestCommand;
use std::borrow::Cow;

struct TestAliasAnalysis;

pub fn subtest(parsed: &TestCommand) -> anyhow::Result<Box<dyn SubTest>> {
    assert_eq!(parsed.command, "alias-analysis");
    if !parsed.options.is_empty() {
        anyhow::bail!("No options allowed on {}", parsed);
    }
    Ok(Box::new(TestAliasAnalysis))
}

impl SubTest for TestAliasAnalysis {
    fn name(&self) -> &'static str {
        "alias-analysis"
    }

    fn is_mutating(&self) -> bool {
        true
    }

    fn run(&self, func: Cow<Function>, context: &Context) -> anyhow::Result<()> {
        let mut comp_ctx = cranelift_codegen::Context::for_function(func.into_owned());

        comp_ctx.flowgraph();
        comp_ctx
            .replace_redundant_loads()
            .map_err(|e| crate::pretty_anyhow_error(&comp_ctx.func, Into::into(e)))?;

        let text = comp_ctx.func.display().to_string();
        run_filecheck(&text, context)
    }
}
