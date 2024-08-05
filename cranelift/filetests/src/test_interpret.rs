//! Test command for interpreting CLIF files and verifying their results
//!
//! The `interpret` test command interprets each function on the host machine
//! using [RunCommand](cranelift_reader::RunCommand)s.

use crate::runone::FileUpdate;
use crate::subtest::SubTest;
use anyhow::Context;
use cranelift_codegen::data_value::DataValue;
use cranelift_codegen::ir;
use cranelift_codegen::ir::{Function, LibCall};
use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::settings::Flags;
use cranelift_interpreter::environment::FunctionStore;
use cranelift_interpreter::interpreter::{Interpreter, InterpreterState, LibCallValues};
use cranelift_interpreter::step::ControlFlow;
use cranelift_reader::{parse_run_command, Details, TestCommand, TestFile};
use log::{info, trace};
use smallvec::smallvec;
use std::borrow::Cow;

struct TestInterpret;

pub fn subtest(parsed: &TestCommand) -> anyhow::Result<Box<dyn SubTest>> {
    assert_eq!(parsed.command, "interpret");
    if !parsed.options.is_empty() {
        anyhow::bail!("No options allowed on {}", parsed);
    }
    Ok(Box::new(TestInterpret))
}

impl SubTest for TestInterpret {
    fn name(&self) -> &'static str {
        "interpret"
    }

    fn is_mutating(&self) -> bool {
        false
    }

    fn needs_isa(&self) -> bool {
        false
    }

    /// Runs the entire subtest for a given target, invokes [Self::run] for running
    /// individual tests.
    fn run_target<'a>(
        &self,
        testfile: &TestFile,
        _: &mut FileUpdate,
        _: &'a str,
        _: &'a Flags,
        _: Option<&'a dyn TargetIsa>,
    ) -> anyhow::Result<()> {
        // We can build the FunctionStore once and reuse it
        let mut func_store = FunctionStore::default();
        for (func, _) in &testfile.functions {
            func_store.add(func.name.to_string(), &func);
        }

        for (func, details) in &testfile.functions {
            info!("Test: {}({}) interpreter", self.name(), func.name);

            run_test(&func_store, func, details).context(self.name())?;
        }

        Ok(())
    }

    fn run(
        &self,
        _func: Cow<ir::Function>,
        _context: &crate::subtest::Context,
    ) -> anyhow::Result<()> {
        unreachable!()
    }
}

fn run_test(func_store: &FunctionStore, func: &Function, details: &Details) -> anyhow::Result<()> {
    for comment in details.comments.iter() {
        if let Some(command) = parse_run_command(comment.text, &func.signature)? {
            trace!("Parsed run command: {}", command);

            command
                .run(|func_name, run_args| {
                    // Rebuild the interpreter state on every run to ensure that we don't accidentally depend on
                    // some leftover state
                    let state = InterpreterState::default()
                        .with_function_store(func_store.clone())
                        .with_libcall_handler(|libcall: LibCall, args: LibCallValues| {
                            use LibCall::*;
                            Ok(smallvec![match (libcall, &args[..]) {
                                (CeilF32, [DataValue::F32(a)]) => DataValue::F32(a.ceil()),
                                (CeilF64, [DataValue::F64(a)]) => DataValue::F64(a.ceil()),
                                (FloorF32, [DataValue::F32(a)]) => DataValue::F32(a.floor()),
                                (FloorF64, [DataValue::F64(a)]) => DataValue::F64(a.floor()),
                                (TruncF32, [DataValue::F32(a)]) => DataValue::F32(a.trunc()),
                                (TruncF64, [DataValue::F64(a)]) => DataValue::F64(a.trunc()),
                                _ => unreachable!(),
                            }])
                        });

                    let mut args = Vec::with_capacity(run_args.len());
                    args.extend_from_slice(run_args);

                    // Because we have stored function names with a leading %, we need to re-add it.
                    let func_name = &format!("%{func_name}");
                    match Interpreter::new(state).call_by_name(func_name, &args) {
                        Ok(ControlFlow::Return(results)) => Ok(results.to_vec()),
                        Ok(e) => {
                            panic!("Unexpected returned control flow: {e:?}")
                        }
                        Err(t) => Err(format!("unexpected trap: {t:?}")),
                    }
                })
                .map_err(|e| anyhow::anyhow!("{}", e))?;
        }
    }
    Ok(())
}
