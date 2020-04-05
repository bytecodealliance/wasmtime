//! Test command for running CLIF files and verifying their results
//!
//! The `run` test command compiles each function on the host machine and executes it

use crate::function_runner::FunctionRunner;
use crate::subtest::{Context, SubTest, SubtestResult};
use cranelift_codegen;
use cranelift_codegen::ir;
use cranelift_reader::parse_run_command;
use cranelift_reader::TestCommand;
use log::trace;
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
                let trimmed_comment = comment.text.trim_start_matches(|c| c == ' ' || c == ';');
                let command = parse_run_command(trimmed_comment, &func.signature)
                    .map_err(|e| format!("{}", e))?;
                trace!("Parsed run command: {}", command);
                // TODO in following changes we will use the parsed command to alter FunctionRunner's behavior.

                let runner =
                    FunctionRunner::with_host_isa(func.clone().into_owned(), context.flags.clone());
                runner.run()?
            }
        }
        Ok(())
    }
}
