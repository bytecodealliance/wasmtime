//! Test command for running CLIF files and verifying their results
//!
//! The `run` test command compiles each function on the host machine and executes it

use crate::function_runner::SingleFunctionCompiler;
use crate::runtest_environment::RuntestEnvironment;
use crate::subtest::{Context, SubTest};
use cranelift_codegen::ir;
use cranelift_codegen::ir::ArgumentPurpose;
use cranelift_reader::parse_run_command;
use cranelift_reader::TestCommand;
use log::trace;
use std::borrow::Cow;
use target_lexicon::Architecture;

struct TestRun;

pub fn subtest(parsed: &TestCommand) -> anyhow::Result<Box<dyn SubTest>> {
    assert_eq!(parsed.command, "run");
    if !parsed.options.is_empty() {
        anyhow::bail!("No options allowed on {}", parsed);
    }
    Ok(Box::new(TestRun))
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

    fn run(&self, func: Cow<ir::Function>, context: &Context) -> anyhow::Result<()> {
        // If this test requests to run on a completely different
        // architecture than the host platform then we skip it entirely,
        // since we won't be able to natively execute machine code.
        let requested_arch = context.isa.unwrap().triple().architecture;
        if requested_arch != Architecture::host() {
            println!(
                "skipped {}: host can't run {:?} programs",
                context.file_path, requested_arch
            );
            return Ok(());
        }

        let test_env = RuntestEnvironment::parse(&context.details.comments[..])?;

        let mut compiler = SingleFunctionCompiler::with_host_isa(context.flags.clone());
        for comment in context.details.comments.iter() {
            if let Some(command) = parse_run_command(comment.text, &func.signature)? {
                trace!("Parsed run command: {}", command);

                // Note that here we're also explicitly ignoring `context.isa`,
                // regardless of what's requested. We want to use the native
                // host ISA no matter what here, so the ISA listed in the file
                // is only used as a filter to not run into situations like
                // running x86_64 code on aarch64 platforms.
                let compiled_fn = compiler.compile(func.clone().into_owned())?;
                command
                    .run(|_, run_args| {
                        let runtime_struct = test_env.runtime_struct();

                        let first_arg_is_vmctx = func
                            .signature
                            .params
                            .first()
                            .map(|p| p.purpose == ArgumentPurpose::VMContext)
                            .unwrap_or(false);

                        if !first_arg_is_vmctx && test_env.is_active() {
                            return Err(concat!(
                                "This test requests a heap, but the first argument is not `i64 vmctx`.\n",
                                "See docs/testing.md for more info on using heap annotations."
                            ).to_string());
                        }

                        let mut args = Vec::with_capacity(run_args.len());
                        if test_env.is_active() {
                            args.push(runtime_struct.pointer(context.isa.unwrap().pointer_type()));
                        }
                        args.extend_from_slice(run_args);

                        Ok(compiled_fn.call(&args))
                    })
                    .map_err(|s| anyhow::anyhow!("{}", s))?;
            }
        }
        Ok(())
    }
}
