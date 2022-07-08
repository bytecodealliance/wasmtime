//! Test command for running CLIF files and verifying their results
//!
//! The `run` test command compiles each function on the host machine and executes it

use crate::function_runner::SingleFunctionCompiler;
use crate::runtest_environment::{HeapMemory, RuntestEnvironment};
use crate::subtest::{Context, SubTest};
use cranelift_codegen::data_value::DataValue;
use cranelift_codegen::ir;
use cranelift_codegen::ir::Type;
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

        // Disable runtests with pinned reg enabled.
        // We've had some abi issues that the trampoline isn't quite ready for.
        if context.flags.enable_pinned_reg() {
            return Err(anyhow::anyhow!([
                "Cannot run runtests with pinned_reg enabled.",
                "See https://github.com/bytecodealliance/wasmtime/issues/4376 for more info"
            ]
            .join("\n")));
        }

        let test_env = RuntestEnvironment::parse(&context.details.comments[..])?;

        let mut compiler = SingleFunctionCompiler::with_host_isa(context.flags.clone())?;
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
                        test_env.validate_signature(&func)?;
                        let (_heaps, _ctx_struct, vmctx_ptr) =
                            build_vmctx_struct(&test_env, context.isa.unwrap().pointer_type());

                        let mut args = Vec::with_capacity(run_args.len());
                        if test_env.is_active() {
                            args.push(vmctx_ptr);
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

/// Build a VMContext struct with the layout described in docs/testing.md.
pub fn build_vmctx_struct(
    test_env: &RuntestEnvironment,
    ptr_ty: Type,
) -> (Vec<HeapMemory>, Vec<u64>, DataValue) {
    let heaps = test_env.allocate_memory();

    let context_struct: Vec<u64> = heaps
        .iter()
        .flat_map(|heap| [heap.as_ptr(), heap.as_ptr().wrapping_add(heap.len())])
        .map(|p| p as usize as u64)
        .collect();

    let ptr = context_struct.as_ptr() as usize as i128;
    let ptr_dv =
        DataValue::from_integer(ptr, ptr_ty).expect("Failed to cast pointer to native target size");

    // Return all these to make sure we don't deallocate the heaps too early
    (heaps, context_struct, ptr_dv)
}
