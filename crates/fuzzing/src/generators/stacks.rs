//! Generate a Wasm program that keeps track of its current stack frames.
//!
//! We can then compare the stack trace we observe in Wasmtime to what the Wasm
//! program believes its stack should be. Any discrepancies between the two
//! points to a bug in either this test case generator or Wasmtime's stack
//! walker.

use std::mem;

use arbitrary::{Arbitrary, Result, Unstructured};
use wasm_encoder::{Instruction, ValType};

const MAX_FUNCS: u32 = 20;
const MAX_OPS: usize = 1_000;
const MAX_PARAMS: usize = 10;

/// Generate a Wasm module that keeps track of its current call stack, to
/// compare to the host.
#[derive(Debug)]
pub struct Stacks {
    funcs: Vec<Function>,
    inputs: Vec<u8>,
}

#[derive(Debug, Default)]
struct Function {
    ops: Vec<Op>,
    params: usize,
    results: usize,
}

#[derive(Debug, Clone, Copy)]
enum Op {
    CheckStackInHost,
    Call(u32),
    CallThroughHost(u32),
    ReturnCall(u32),
}

impl<'a> Arbitrary<'a> for Stacks {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        let funcs = Self::arbitrary_funcs(u)?;
        let n = u.len().min(200);
        let inputs = u.bytes(n)?.to_vec();
        Ok(Stacks { funcs, inputs })
    }
}

impl Stacks {
    fn arbitrary_funcs(u: &mut Unstructured) -> Result<Vec<Function>> {
        // Generate a list of functions first with a number of parameters and
        // results. Bodies are generated afterwards.
        let nfuncs = u.int_in_range(1..=MAX_FUNCS)?;
        let mut funcs = (0..nfuncs)
            .map(|_| {
                Ok(Function {
                    ops: Vec::new(), // generated later
                    params: u.int_in_range(0..=MAX_PARAMS)?,
                    results: u.int_in_range(0..=MAX_PARAMS)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let mut funcs_by_result = vec![Vec::new(); MAX_PARAMS + 1];
        for (i, func) in funcs.iter().enumerate() {
            funcs_by_result[func.results].push(i as u32);
        }

        // Fill in each function body with various instructions/operations now
        // that the set of functions is known.
        for f in funcs.iter_mut() {
            let funcs_with_same_results = &funcs_by_result[f.results];
            for _ in 0..u.arbitrary_len::<usize>()?.min(MAX_OPS) {
                let op = match u.int_in_range(0..=3)? {
                    0 => Op::CheckStackInHost,
                    1 => Op::Call(u.int_in_range(0..=nfuncs - 1)?),
                    2 => Op::CallThroughHost(u.int_in_range(0..=nfuncs - 1)?),
                    // This only works if the target function has the same
                    // number of results, so choose from a different set here.
                    3 => Op::ReturnCall(*u.choose(funcs_with_same_results)?),
                    _ => unreachable!(),
                };
                f.ops.push(op);
                // once a `return_call` has been generated there's no need to
                // generate any more instructions, so fall through to below.
                if let Some(Op::ReturnCall(_)) = f.ops.last() {
                    break;
                }
            }
        }

        Ok(funcs)
    }

    /// Get the input values to run the Wasm module with.
    pub fn inputs(&self) -> &[u8] {
        &self.inputs
    }

    /// Get this test case's Wasm module.
    ///
    /// The Wasm module has the following imports:
    ///
    /// * `host.check_stack: [] -> []`: The host can check the Wasm's
    ///   understanding of its own stack against the host's understanding of the
    ///   Wasm stack to find discrepancy bugs.
    ///
    /// * `host.call_func: [funcref] -> []`: The host should call the given
    ///   `funcref`, creating a call stack with multiple sequences of contiguous
    ///   Wasm frames on the stack like `[..., wasm, host, wasm]`.
    ///
    /// The Wasm module has the following exports:
    ///
    /// * `run: [i32] -> []`: This function should be called with each of the
    ///   input values to run this generated test case.
    ///
    /// * `get_stack: [] -> [i32 i32]`: Get the pointer and length of the `u32`
    ///   array of this Wasm's understanding of its stack. This is useful for
    ///   checking whether the host's view of the stack at a trap matches the
    ///   Wasm program's understanding.
    pub fn wasm(&self) -> Vec<u8> {
        let mut module = wasm_encoder::Module::new();

        let mut types = wasm_encoder::TypeSection::new();

        let run_type = types.len();
        types
            .ty()
            .function(vec![wasm_encoder::ValType::I32], vec![]);

        let get_stack_type = types.len();
        types.ty().function(
            vec![],
            vec![wasm_encoder::ValType::I32, wasm_encoder::ValType::I32],
        );

        let call_func_type = types.len();
        types
            .ty()
            .function(vec![wasm_encoder::ValType::FUNCREF], vec![]);

        let check_stack_type = types.len();
        types.ty().function(vec![], vec![]);

        let func_types_start = types.len();
        for func in self.funcs.iter() {
            types.ty().function(
                vec![ValType::I32; func.params],
                vec![ValType::I32; func.results],
            );
        }

        section(&mut module, types);

        let mut imports = wasm_encoder::ImportSection::new();
        let check_stack_func = 0;
        imports.import(
            "host",
            "check_stack",
            wasm_encoder::EntityType::Function(check_stack_type),
        );
        let call_func_func = 1;
        imports.import(
            "host",
            "call_func",
            wasm_encoder::EntityType::Function(call_func_type),
        );
        let num_imported_funcs = 2;
        section(&mut module, imports);

        let mut funcs = wasm_encoder::FunctionSection::new();
        for (i, _) in self.funcs.iter().enumerate() {
            funcs.function(func_types_start + (i as u32));
        }
        let run_func = funcs.len() + num_imported_funcs;
        funcs.function(run_type);
        let get_stack_func = funcs.len() + num_imported_funcs;
        funcs.function(get_stack_type);
        section(&mut module, funcs);

        let mut mems = wasm_encoder::MemorySection::new();
        let memory = mems.len();
        mems.memory(wasm_encoder::MemoryType {
            minimum: 1,
            maximum: Some(1),
            memory64: false,
            shared: false,
            page_size_log2: None,
        });
        section(&mut module, mems);

        let mut globals = wasm_encoder::GlobalSection::new();
        let fuel_global = globals.len();
        globals.global(
            wasm_encoder::GlobalType {
                val_type: wasm_encoder::ValType::I32,
                mutable: true,
                shared: false,
            },
            &wasm_encoder::ConstExpr::i32_const(0),
        );
        let stack_len_global = globals.len();
        globals.global(
            wasm_encoder::GlobalType {
                val_type: wasm_encoder::ValType::I32,
                mutable: true,
                shared: false,
            },
            &wasm_encoder::ConstExpr::i32_const(0),
        );
        section(&mut module, globals);

        let mut exports = wasm_encoder::ExportSection::new();
        exports.export("run", wasm_encoder::ExportKind::Func, run_func);
        exports.export("get_stack", wasm_encoder::ExportKind::Func, get_stack_func);
        exports.export("memory", wasm_encoder::ExportKind::Memory, memory);
        exports.export("fuel", wasm_encoder::ExportKind::Global, fuel_global);
        section(&mut module, exports);

        let mut elems = wasm_encoder::ElementSection::new();
        elems.declared(wasm_encoder::Elements::Functions(
            (0..num_imported_funcs + u32::try_from(self.funcs.len()).unwrap())
                .collect::<Vec<_>>()
                .into(),
        ));
        section(&mut module, elems);

        let check_fuel = |body: &mut wasm_encoder::Function| {
            // Trap if we are out of fuel.
            body.instruction(&Instruction::GlobalGet(fuel_global))
                .instruction(&Instruction::I32Eqz)
                .instruction(&Instruction::If(wasm_encoder::BlockType::Empty))
                .instruction(&Instruction::Unreachable)
                .instruction(&Instruction::End);

            // Decrement fuel.
            body.instruction(&Instruction::GlobalGet(fuel_global))
                .instruction(&Instruction::I32Const(1))
                .instruction(&Instruction::I32Sub)
                .instruction(&Instruction::GlobalSet(fuel_global));
        };

        let push_func_to_stack = |body: &mut wasm_encoder::Function, func: u32| {
            // Add this function to our internal stack.
            //
            // Note that we know our `stack_len_global` can't go beyond memory
            // bounds because we limit fuel to at most `u8::MAX` and each stack
            // entry is an `i32` and `u8::MAX * size_of(i32)` still fits in one
            // Wasm page.
            body.instruction(&Instruction::GlobalGet(stack_len_global))
                .instruction(&Instruction::I32Const(func as i32))
                .instruction(&Instruction::I32Store(wasm_encoder::MemArg {
                    offset: 0,
                    align: 0,
                    memory_index: memory,
                }))
                .instruction(&Instruction::GlobalGet(stack_len_global))
                .instruction(&Instruction::I32Const(mem::size_of::<i32>() as i32))
                .instruction(&Instruction::I32Add)
                .instruction(&Instruction::GlobalSet(stack_len_global));
        };

        let pop_func_from_stack = |body: &mut wasm_encoder::Function| {
            // Remove this function from our internal stack.
            body.instruction(&Instruction::GlobalGet(stack_len_global))
                .instruction(&Instruction::I32Const(mem::size_of::<i32>() as i32))
                .instruction(&Instruction::I32Sub)
                .instruction(&Instruction::GlobalSet(stack_len_global));
        };

        let push_params = |body: &mut wasm_encoder::Function, func: u32| {
            let func = &self.funcs[func as usize];
            for _ in 0..func.params {
                body.instruction(&Instruction::I32Const(0));
            }
        };
        let pop_results = |body: &mut wasm_encoder::Function, func: u32| {
            let func = &self.funcs[func as usize];
            for _ in 0..func.results {
                body.instruction(&Instruction::Drop);
            }
        };
        let push_results = |body: &mut wasm_encoder::Function, func: u32| {
            let func = &self.funcs[func as usize];
            for _ in 0..func.results {
                body.instruction(&Instruction::I32Const(0));
            }
        };

        let mut code = wasm_encoder::CodeSection::new();
        for (func_index, func) in self.funcs.iter().enumerate() {
            let mut body = wasm_encoder::Function::new(vec![]);

            push_func_to_stack(
                &mut body,
                num_imported_funcs + u32::try_from(func_index).unwrap(),
            );
            check_fuel(&mut body);

            let mut check_fuel_and_pop_at_end = true;

            // Perform our specified operations.
            for op in &func.ops {
                assert!(check_fuel_and_pop_at_end);
                match op {
                    Op::CheckStackInHost => {
                        body.instruction(&Instruction::Call(check_stack_func));
                    }
                    Op::Call(f) => {
                        push_params(&mut body, *f);
                        body.instruction(&Instruction::Call(f + num_imported_funcs));
                        pop_results(&mut body, *f);
                    }
                    Op::CallThroughHost(f) => {
                        body.instruction(&Instruction::RefFunc(f + num_imported_funcs))
                            .instruction(&Instruction::Call(call_func_func));
                    }

                    // For a `return_call` preemptively check fuel to possibly
                    // trap and then pop our function from the in-wasm managed
                    // stack. After that execute the `return_call` itself.
                    Op::ReturnCall(f) => {
                        push_params(&mut body, *f);
                        check_fuel(&mut body);
                        pop_func_from_stack(&mut body);
                        check_fuel_and_pop_at_end = false;
                        body.instruction(&Instruction::ReturnCall(f + num_imported_funcs));
                    }
                }
            }

            // Potentially trap at the end of our function as well, so that we
            // exercise the scenario where the Wasm-to-host trampoline
            // initialized `last_wasm_exit_sp` et al when calling out to a host
            // function, but then we returned back to Wasm and then trapped
            // while `last_wasm_exit_sp` et al are still initialized from that
            // previous host call.
            if check_fuel_and_pop_at_end {
                check_fuel(&mut body);
                pop_func_from_stack(&mut body);
                push_results(&mut body, func_index as u32);
            }

            function(&mut code, body);
        }

        let mut run_body = wasm_encoder::Function::new(vec![]);

        // Reset the bump pointer for the internal stack (this allows us to
        // reuse an instance in the oracle, rather than re-instantiate).
        run_body
            .instruction(&Instruction::I32Const(0))
            .instruction(&Instruction::GlobalSet(stack_len_global));

        // Initialize the fuel global.
        run_body
            .instruction(&Instruction::LocalGet(0))
            .instruction(&Instruction::GlobalSet(fuel_global));

        push_func_to_stack(&mut run_body, run_func);

        // Make sure to check for out-of-fuel in the `run` function as well, so
        // that we also capture stack traces with only one frame, not just `run`
        // followed by the first locally-defined function and then zero or more
        // extra frames.
        check_fuel(&mut run_body);

        // Call the first locally defined function.
        push_params(&mut run_body, 0);
        run_body.instruction(&Instruction::Call(num_imported_funcs));
        pop_results(&mut run_body, 0);

        check_fuel(&mut run_body);
        pop_func_from_stack(&mut run_body);

        function(&mut code, run_body);

        let mut get_stack_body = wasm_encoder::Function::new(vec![]);
        get_stack_body
            .instruction(&Instruction::I32Const(0))
            .instruction(&Instruction::GlobalGet(stack_len_global));
        function(&mut code, get_stack_body);

        section(&mut module, code);

        return module.finish();

        // Helper that defines a section in the module and takes ownership of it
        // so that it is dropped and its memory reclaimed after adding it to the
        // module.
        fn section(module: &mut wasm_encoder::Module, section: impl wasm_encoder::Section) {
            module.section(&section);
        }

        // Helper that defines a function body in the code section and takes
        // ownership of it so that it is dropped and its memory reclaimed after
        // adding it to the module.
        fn function(code: &mut wasm_encoder::CodeSection, mut func: wasm_encoder::Function) {
            func.instruction(&Instruction::End);
            code.function(&func);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::prelude::*;
    use wasmparser::Validator;

    #[test]
    fn stacks_generates_valid_wasm_modules() {
        let mut rng = SmallRng::seed_from_u64(0);
        let mut buf = vec![0; 2048];
        for _ in 0..1024 {
            rng.fill_bytes(&mut buf);
            let u = Unstructured::new(&buf);
            if let Ok(stacks) = Stacks::arbitrary_take_rest(u) {
                let wasm = stacks.wasm();
                validate(&wasm);
            }
        }
    }

    fn validate(wasm: &[u8]) {
        let mut validator = Validator::new();
        let err = match validator.validate_all(wasm) {
            Ok(_) => return,
            Err(e) => e,
        };
        drop(std::fs::write("test.wasm", wasm));
        if let Ok(text) = wasmprinter::print_bytes(wasm) {
            drop(std::fs::write("test.wat", &text));
        }
        panic!("wasm failed to validate: {err}");
    }
}
