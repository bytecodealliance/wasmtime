//! Test command for testing the simple GVN pass.
//!
//! The `simple-gvn` test command runs each function through the simple GVN pass after ensuring
//! that all instructions are legal for the target.
//!
//! The resulting function is sent to `filecheck`.

use cretonne::ir::Function;
use cretonne;
use cton_reader::TestCommand;
use filetest::subtest::{SubTest, Context, Result, run_filecheck};
use std::borrow::Cow;
use std::fmt::Write;
use utils::pretty_error;

struct TestSimpleGVN;

pub fn subtest(parsed: &TestCommand) -> Result<Box<SubTest>> {
    assert_eq!(parsed.command, "simple-gvn");
    if !parsed.options.is_empty() {
        Err(format!("No options allowed on {}", parsed))
    } else {
        Ok(Box::new(TestSimpleGVN))
    }
}

impl SubTest for TestSimpleGVN {
    fn name(&self) -> Cow<str> {
        Cow::from("simple-gvn")
    }

    fn is_mutating(&self) -> bool {
        true
    }

    fn run(&self, func: Cow<Function>, context: &Context) -> Result<()> {
        // Create a compilation context, and drop in the function.
        let mut comp_ctx = cretonne::Context::new();
        comp_ctx.func = func.into_owned();

        comp_ctx.flowgraph();
        comp_ctx.simple_gvn();
        comp_ctx.verify(context.flags_or_isa()).map_err(|e| {
            pretty_error(&comp_ctx.func, context.isa, Into::into(e))
        })?;

        let mut text = String::new();
        write!(&mut text, "{}", &comp_ctx.func).map_err(
            |e| e.to_string(),
        )?;
        run_filecheck(&text, context)
    }
}
