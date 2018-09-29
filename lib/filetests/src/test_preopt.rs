//! Test command for testing the preopt pass.
//!
//! The resulting function is sent to `filecheck`.

use cranelift_codegen;
use cranelift_codegen::ir::Function;
use cranelift_codegen::print_errors::pretty_error;
use cranelift_reader::TestCommand;
use std::borrow::Cow;
use subtest::{run_filecheck, Context, SubTest, SubtestResult};

struct TestPreopt;

pub fn subtest(parsed: &TestCommand) -> SubtestResult<Box<SubTest>> {
    assert_eq!(parsed.command, "preopt");
    if !parsed.options.is_empty() {
        Err(format!("No options allowed on {}", parsed))
    } else {
        Ok(Box::new(TestPreopt))
    }
}

impl SubTest for TestPreopt {
    fn name(&self) -> &'static str {
        "preopt"
    }

    fn is_mutating(&self) -> bool {
        true
    }

    fn run(&self, func: Cow<Function>, context: &Context) -> SubtestResult<()> {
        let mut comp_ctx = cranelift_codegen::Context::for_function(func.into_owned());
        let isa = context.isa.expect("preopt needs an ISA");

        comp_ctx.flowgraph();
        comp_ctx
            .preopt(isa)
            .map_err(|e| pretty_error(&comp_ctx.func, context.isa, Into::into(e)))?;

        let text = &comp_ctx.func.display(isa).to_string();
        run_filecheck(&text, context)
    }
}
