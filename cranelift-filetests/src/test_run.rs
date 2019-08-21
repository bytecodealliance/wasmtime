//! Test command for running CLIF files and verifying their results
//!
//! The `run` test command compiles each function on the host machine and executes it

use crate::function_runner::FunctionRunner;
use crate::subtest::{Context, SubTest, SubtestResult};
use cranelift_codegen;
use cranelift_codegen::ir;
use cranelift_reader::TestCommand;
use std::borrow::Cow;

struct TestRun;

pub fn subtest(parsed: &TestCommand) -> SubtestResult<Box<dyn SubTest>> {
    assert_eq!(parsed.command, "run");
    if !parsed.options.is_empty() {
        Err(format!("No options allowed on {}", parsed))
    } else {
        Ok(Box::new(TestRun))
    }
}

impl SubTest for TestRun {
    fn name(&self) -> &'static str {
        "run"
    }

    fn is_mutating(&self) -> bool {
        false
    }

    fn needs_isa(&self) -> bool {
        false
    }

    fn run(&self, func: Cow<ir::Function>, context: &Context) -> SubtestResult<()> {
        for comment in context.details.comments.iter() {
            if comment.text.contains("run") {
                let runner =
                    FunctionRunner::with_host_isa(func.clone().into_owned(), context.flags.clone());
                runner.run()?
            }
        }
        Ok(())
    }
}
