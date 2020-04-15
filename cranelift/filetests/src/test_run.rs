//! Test command for running CLIF files and verifying their results
//!
//! The `run` test command compiles each function on the host machine and executes it

use crate::function_runner::FunctionRunner;
use crate::subtest::{Context, SubTest, SubtestResult};
use cranelift_codegen::ir;
use cranelift_reader::parse_run_command;
use cranelift_reader::TestCommand;
use log::trace;
use std::borrow::Cow;
use target_lexicon::Architecture;

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
        true
    }

    fn run(&self, func: Cow<ir::Function>, context: &Context) -> SubtestResult<()> {
        for comment in context.details.comments.iter() {
            if comment.text.contains("run") {
                let trimmed_comment = comment.text.trim_start_matches(|c| c == ' ' || c == ';');
                let command = parse_run_command(trimmed_comment, &func.signature)
                    .map_err(|e| format!("{}", e))?;
                trace!("Parsed run command: {}", command);

                // If this test requests to run on a completely different
                // architecture than the host platform then we skip it entirely,
                // since we won't be able to natively execute machine code.
                let requested_arch = context.isa.unwrap().triple().architecture;
                if requested_arch != Architecture::host() {
                    return Ok(());
                }

                // TODO in following changes we will use the parsed command to alter FunctionRunner's behavior.
                //
                // Note that here we're also explicitly ignoring `context.isa`,
                // regardless of what's requested. We want to use the native
                // host ISA no matter what here, so the ISA listed in the file
                // is only used as a filter to not run into situations like
                // running x86_64 code on aarch64 platforms.
                let runner =
                    FunctionRunner::with_host_isa(func.clone().into_owned(), context.flags.clone());
                runner.run()?
            }
        }
        Ok(())
    }
}
