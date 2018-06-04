//! Test command for testing the postopt pass.
//!
//! The resulting function is sent to `filecheck`.

use cretonne_codegen;
use cretonne_codegen::ir::Function;
use cretonne_codegen::print_errors::pretty_error;
use cretonne_reader::TestCommand;
use std::borrow::Cow;
use subtest::{run_filecheck, Context, SubTest, SubtestResult};

struct TestPostopt;

pub fn subtest(parsed: &TestCommand) -> SubtestResult<Box<SubTest>> {
    assert_eq!(parsed.command, "postopt");
    if !parsed.options.is_empty() {
        Err(format!("No options allowed on {}", parsed))
    } else {
        Ok(Box::new(TestPostopt))
    }
}

impl SubTest for TestPostopt {
    fn name(&self) -> &'static str {
        "postopt"
    }

    fn is_mutating(&self) -> bool {
        true
    }

    fn run(&self, func: Cow<Function>, context: &Context) -> SubtestResult<()> {
        let mut comp_ctx = cretonne_codegen::Context::for_function(func.into_owned());
        let isa = context.isa.expect("postopt needs an ISA");

        comp_ctx.flowgraph();
        comp_ctx
            .postopt(isa)
            .map_err(|e| pretty_error(&comp_ctx.func, context.isa, Into::into(e)))?;

        let text = comp_ctx.func.to_string();
        run_filecheck(&text, context)
    }
}
