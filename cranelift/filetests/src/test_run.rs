//! Test command for running CLIF files and verifying their results
//!
//! The `run` test command compiles each function on the host machine and executes it

use crate::function_runner::SingleFunctionCompiler;
use crate::subtest::{Context, SubTest, SubtestResult};
use cranelift_codegen::ir;
use cranelift_codegen::isa::TargetIsa;
use cranelift_native::default_host_isa;
use cranelift_reader::{parse_run_command, TestCommand};
use log::trace;
use std::borrow::Cow;

struct TestRun {
    default_host_isa: Box<dyn TargetIsa>,
}

pub fn subtest(parsed: &TestCommand) -> SubtestResult<Box<dyn SubTest>> {
    assert_eq!(parsed.command, "run");
    if !parsed.options.is_empty() {
        Err(format!("No options allowed on {}", parsed))
    } else {
        let default_host_isa = default_host_isa()?;
        Ok(Box::new(TestRun { default_host_isa }))
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
        let requested_isa = context.isa.unwrap();

        // If this test requests to run on a completely different architecture than the host
        // platform or if it requests flags that are unavailable, then we skip it entirely,
        // since we won't be able to natively execute machine code.
        if !self.default_host_isa.is_compatible_with(requested_isa) {
            println!(
                "skipped {}: host and target ISAs do not match: host = {}, target = {} (could also be due to flags, not shown)",
                context.file_path,
                self.default_host_isa.triple(),
                requested_isa.triple(),
            );
            return Ok(());
        }

        let mut compiler = SingleFunctionCompiler::new(requested_isa);
        for comment in context.details.comments.iter() {
            if let Some(command) =
                parse_run_command(comment.text, &func.signature).map_err(|e| e.to_string())?
            {
                trace!("Parsed run command: {}", command);

                // Note that here we're also explicitly ignoring `context.isa`,
                // regardless of what's requested. We want to use the native
                // host ISA no matter what here, so the ISA listed in the file
                // is only used as a filter to not run into situations like
                // running x86_64 code on aarch64 platforms.
                let compiled_fn = compiler
                    .compile(func.clone().into_owned())
                    .map_err(|e| format!("{:?}", e))?;
                command.run(|_, args| Ok(compiled_fn.call(args)))?;
            }
        }
        Ok(())
    }
}
