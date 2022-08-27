//! Test command for interpreting CLIF files and verifying their results
//!
//! The `interpret` test command interprets each function on the host machine
//! using [RunCommand](cranelift_reader::RunCommand)s.

use crate::runone::FileUpdate;
use crate::runtest_environment::RuntestEnvironment;
use crate::subtest::SubTest;
use anyhow::{anyhow, Context};
use cranelift_codegen::data_value::DataValue;
use cranelift_codegen::ir::types::I64;
use cranelift_codegen::ir::Function;
use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::settings::Flags;
use cranelift_codegen::{self, ir};
use cranelift_interpreter::environment::FunctionStore;
use cranelift_interpreter::interpreter::{HeapInit, Interpreter, InterpreterState};
use cranelift_interpreter::step::ControlFlow;
use cranelift_reader::{parse_run_command, Details, TestCommand, TestFile};
use log::{info, trace};
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

            let test_env = RuntestEnvironment::parse(&details.comments[..])?;
            test_env.validate_signature(&func).map_err(|s| anyhow!(s))?;

            run_test(&test_env, &func_store, func, details).context(self.name())?;
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

fn run_test(
    test_env: &RuntestEnvironment,
    func_store: &FunctionStore,
    func: &Function,
    details: &Details,
) -> anyhow::Result<()> {
    for comment in details.comments.iter() {
        if let Some(command) = parse_run_command(comment.text, &func.signature)? {
            trace!("Parsed run command: {}", command);

            command
                .run(|func_name, run_args| {
                    // Rebuild the interpreter state on every run to ensure that we don't accidentally depend on
                    // some leftover state
                    let mut state =
                        InterpreterState::default().with_function_store(func_store.clone());

                    let mut args = Vec::with_capacity(run_args.len());
                    if test_env.is_active() {
                        let vmctx_addr = register_heaps(&mut state, test_env);
                        args.push(vmctx_addr);
                    }
                    args.extend_from_slice(run_args);

                    // Because we have stored function names with a leading %, we need to re-add it.
                    let func_name = &format!("%{}", func_name);
                    match Interpreter::new(state).call_by_name(func_name, &args) {
                        Ok(ControlFlow::Return(results)) => Ok(results.to_vec()),
                        Ok(e) => {
                            panic!("Unexpected returned control flow: {:?}", e)
                        }
                        Err(t) => Err(format!("unexpected trap: {:?}", t)),
                    }
                })
                .map_err(|e| anyhow::anyhow!("{}", e))?;
        }
    }
    Ok(())
}

/// Build a VMContext struct with the layout described in docs/testing.md.
pub fn register_heaps<'a>(
    state: &mut InterpreterState<'a>,
    test_env: &RuntestEnvironment,
) -> DataValue {
    let mem = test_env.allocate_memory();
    let vmctx_struct = mem
        .into_iter()
        // This memory layout (a contiguous list of base + bound ptrs)
        // is enforced by the RuntestEnvironment when parsing the heap
        // directives. So we are safe to replicate that here.
        .flat_map(|mem| {
            let heap_len = mem.len() as u64;
            let heap = state.register_heap(HeapInit::FromBacking(mem));
            [
                state.get_heap_address(I64, heap, 0).unwrap(),
                state.get_heap_address(I64, heap, heap_len).unwrap(),
            ]
        })
        .map(|addr| {
            let mut mem = [0u8; 8];
            addr.write_to_slice(&mut mem[..]);
            mem
        })
        .flatten()
        .collect();

    let vmctx_heap = state.register_heap(HeapInit::FromBacking(vmctx_struct));
    state.get_heap_address(I64, vmctx_heap, 0).unwrap()
}
