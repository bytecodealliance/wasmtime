//! Test command for interpreting CLIF files and verifying their results
//!
//! The `interpret` test command interprets each function on the host machine
//! using [RunCommand](cranelift_reader::RunCommand)s.

use crate::runtest_environment::RuntestEnvironment;
use crate::subtest::{Context, SubTest};
use cranelift_codegen::data_value::DataValue;
use cranelift_codegen::ir::types::I64;
use cranelift_codegen::{self, ir};
use cranelift_interpreter::environment::FunctionStore;
use cranelift_interpreter::interpreter::{HeapInit, Interpreter, InterpreterState};
use cranelift_interpreter::step::ControlFlow;
use cranelift_reader::{parse_run_command, TestCommand};
use log::trace;
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

    fn run(&self, func: Cow<ir::Function>, context: &Context) -> anyhow::Result<()> {
        let test_env = RuntestEnvironment::parse(&context.details.comments[..])?;
        for comment in context.details.comments.iter() {
            if let Some(command) = parse_run_command(comment.text, &func.signature)? {
                trace!("Parsed run command: {}", command);

                let mut env = FunctionStore::default();
                env.add(func.name.to_string(), &func);

                command
                    .run(|func_name, run_args| {
                        test_env.validate_signature(&func)?;

                        let mut state = InterpreterState::default().with_function_store(env);

                        let mut args = Vec::with_capacity(run_args.len());
                        if test_env.is_active() {
                            let vmctx_addr = register_heaps(&mut state, &test_env);
                            args.push(vmctx_addr);
                        }
                        args.extend_from_slice(run_args);

                        // Because we have stored function names with a leading %, we need to re-add it.
                        let func_name = &format!("%{}", func_name);
                        match Interpreter::new(state).call_by_name(func_name, &args) {
                            Ok(ControlFlow::Return(results)) => Ok(results.to_vec()),
                            Ok(_) => {
                                panic!("Unexpected returned control flow--this is likely a bug.")
                            }
                            Err(t) => Err(format!("unexpected trap: {:?}", t)),
                        }
                    })
                    .map_err(|e| anyhow::anyhow!("{}", e))?;
            }
        }
        Ok(())
    }
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
