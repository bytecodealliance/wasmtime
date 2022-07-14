//! Test command for running CLIF files and verifying their results
//!
//! The `run` test command compiles each function on the host machine and executes it

use crate::function_runner::SingleFunctionCompiler;
use crate::runtest_environment::{HeapMemory, RuntestEnvironment};
use crate::subtest::{Context, SubTest};
use cranelift_codegen::data_value::DataValue;
use cranelift_codegen::ir::Type;
use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::settings::Configurable;
use cranelift_codegen::{ir, settings};
use cranelift_reader::parse_run_command;
use cranelift_reader::TestCommand;
use log::trace;
use std::borrow::Cow;

struct TestRun;

pub fn subtest(parsed: &TestCommand) -> anyhow::Result<Box<dyn SubTest>> {
    assert_eq!(parsed.command, "run");
    if !parsed.options.is_empty() {
        anyhow::bail!("No options allowed on {}", parsed);
    }
    Ok(Box::new(TestRun))
}

/// Builds a [TargetIsa] for the current host.
///
/// ISA Flags can be overridden by passing [Value]'s via `isa_flags`.
fn build_host_isa(
    infer_native_flags: bool,
    flags: settings::Flags,
    isa_flags: Vec<settings::Value>,
) -> Box<dyn TargetIsa> {
    let mut builder = cranelift_native::builder_with_options(infer_native_flags)
        .expect("Unable to build a TargetIsa for the current host");

    for value in isa_flags {
        builder.set(value.name, &value.value_string()).unwrap();
    }

    builder.finish(flags).unwrap()
}

/// Checks if the host's ISA is compatible with the one requested by the test.
fn is_isa_compatible(
    context: &Context,
    host: &dyn TargetIsa,
    requested: &dyn TargetIsa,
) -> Result<(), String> {
    // If this test requests to run on a completely different
    // architecture than the host platform then we skip it entirely,
    // since we won't be able to natively execute machine code.
    let host_arch = host.triple().architecture;
    let requested_arch = requested.triple().architecture;
    if host_arch != requested_arch {
        return Err(format!(
            "skipped {}: host can't run {:?} programs",
            context.file_path, requested_arch
        ));
    }

    // We need to check that the requested ISA does not have any flags that
    // we can't natively support on the host.
    let requested_flags = requested.isa_flags();
    for req_value in requested_flags {
        if let Some(requested) = req_value.as_bool() {
            let available_in_host = host
                .isa_flags()
                .iter()
                .find(|val| val.name == req_value.name)
                .and_then(|val| val.as_bool())
                .unwrap_or(false);

            if requested && !available_in_host {
                return Err(format!(
                    "skipped {}: host does not support ISA flag {}",
                    context.file_path, req_value.name
                ));
            }
        } else {
            unimplemented!("ISA flag {} of kind {:?}", req_value.name, req_value.kind());
        }
    }

    Ok(())
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
        // Disable runtests with pinned reg enabled.
        // We've had some abi issues that the trampoline isn't quite ready for.
        if context.flags.enable_pinned_reg() {
            return Err(anyhow::anyhow!([
                "Cannot run runtests with pinned_reg enabled.",
                "See https://github.com/bytecodealliance/wasmtime/issues/4376 for more info"
            ]
            .join("\n")));
        }

        let host_isa = build_host_isa(true, context.flags.clone(), vec![]);
        let requested_isa = context.isa.unwrap();
        if let Err(e) = is_isa_compatible(context, host_isa.as_ref(), requested_isa) {
            println!("{}", e);
            return Ok(());
        }

        // We can't use the requested ISA directly since it does not contain info
        // about the operating system / calling convention / etc..
        //
        // Copy the requested ISA flags into the host ISA and use that.
        let isa = build_host_isa(false, context.flags.clone(), requested_isa.isa_flags());

        let test_env = RuntestEnvironment::parse(&context.details.comments[..])?;

        let mut compiler = SingleFunctionCompiler::new(isa);
        for comment in context.details.comments.iter() {
            if let Some(command) = parse_run_command(comment.text, &func.signature)? {
                trace!("Parsed run command: {}", command);

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
