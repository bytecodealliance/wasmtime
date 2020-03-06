//! Test command for testing the DCE pass.
//!
//! The `dce` test command runs each function through the DCE pass after ensuring
//! that all instructions are legal for the target.
//!
//! The resulting function is sent to `filecheck`.

use crate::subtest::{run_filecheck, Context, SubTest, SubtestResult};
use cranelift_codegen;
use cranelift_codegen::ir::Function;
use cranelift_codegen::print_errors::pretty_error;
use cranelift_reader::TestCommand;
use std::borrow::Cow;

struct TestDCE;

pub fn subtest(parsed: &TestCommand) -> SubtestResult<Box<dyn SubTest>> {
    assert_eq!(parsed.command, "dce");
    if !parsed.options.is_empty() {
        Err(format!("No options allowed on {}", parsed))
    } else {
        Ok(Box::new(TestDCE))
    }
}

impl SubTest for TestDCE {
    fn name(&self) -> &'static str {
        "dce"
    }

    fn is_mutating(&self) -> bool {
        true
    }

    fn run(&self, func: Cow<Function>, context: &Context) -> SubtestResult<()> {
        let mut comp_ctx = cranelift_codegen::Context::for_function(func.into_owned());

        comp_ctx.flowgraph();
        comp_ctx.compute_loop_analysis();
        comp_ctx
            .dce(context.flags_or_isa())
            .map_err(|e| pretty_error(&comp_ctx.func, context.isa, Into::into(e)))?;

        let text = comp_ctx.func.display(context.isa).to_string();
        run_filecheck(&text, context)
    }
}
