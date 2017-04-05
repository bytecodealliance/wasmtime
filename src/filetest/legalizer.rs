//! Test command for checking the IL legalizer.
//!
//! The `test legalizer` test command runs each function through `legalize_function()` and sends
//! the result to filecheck.

use std::borrow::Cow;
use cretonne::{self, write_function};
use cretonne::ir::Function;
use cton_reader::TestCommand;
use filetest::subtest::{SubTest, Context, Result, run_filecheck};
use utils::pretty_verifier_error;

struct TestLegalizer;

pub fn subtest(parsed: &TestCommand) -> Result<Box<SubTest>> {
    assert_eq!(parsed.command, "legalizer");
    if !parsed.options.is_empty() {
        Err(format!("No options allowed on {}", parsed))
    } else {
        Ok(Box::new(TestLegalizer))
    }
}

impl SubTest for TestLegalizer {
    fn name(&self) -> Cow<str> {
        Cow::from("legalizer")
    }

    fn is_mutating(&self) -> bool {
        true
    }

    fn needs_isa(&self) -> bool {
        true
    }

    fn run(&self, func: Cow<Function>, context: &Context) -> Result<()> {
        let mut comp_ctx = cretonne::Context::new();
        comp_ctx.func = func.into_owned();
        let isa = context.isa.expect("legalizer needs an ISA");

        comp_ctx.flowgraph();
        comp_ctx.legalize(isa);
        comp_ctx.verify(isa)
            .map_err(|e| pretty_verifier_error(&comp_ctx.func, e))?;

        let mut text = String::new();
        write_function(&mut text, &comp_ctx.func, Some(isa)).map_err(|e| e.to_string())?;
        run_filecheck(&text, context)
    }
}
