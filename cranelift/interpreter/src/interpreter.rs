//! Cranelift IR interpreter.
//!
//! This module partially contains the logic for interpreting Cranelift IR.

use crate::environment::{FuncIndex, FunctionStore};
use crate::frame::Frame;
use crate::instruction::DfgInstructionContext;
use crate::state::{MemoryError, State};
use crate::step::{step, ControlFlow, StepError};
use crate::value::ValueError;
use cranelift_codegen::data_value::DataValue;
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::{Block, FuncRef, Function, Type, Value as ValueRef};
use log::trace;
use std::collections::HashSet;
use std::fmt::Debug;
use thiserror::Error;

/// The Cranelift interpreter; this contains some high-level functions to control the interpreter's
/// flow. The interpreter state is defined separately (see [InterpreterState]) as the execution
/// semantics for each Cranelift instruction (see [step]).
pub struct Interpreter<'a> {
    state: InterpreterState<'a>,
    fuel: Option<u64>,
}

impl<'a> Interpreter<'a> {
    pub fn new(state: InterpreterState<'a>) -> Self {
        Self { state, fuel: None }
    }

    /// The `fuel` mechanism sets a number of instructions that
    /// the interpreter can execute before stopping. If this
    /// value is `None` (the default), no limit is imposed.
    pub fn with_fuel(self, fuel: Option<u64>) -> Self {
        Self { fuel, ..self }
    }

    /// Call a function by name; this is a helpful proxy for [Interpreter::call_by_index].
    pub fn call_by_name(
        &mut self,
        func_name: &str,
        arguments: &[DataValue],
    ) -> Result<ControlFlow<'a, DataValue>, InterpreterError> {
        let index = self
            .state
            .functions
            .index_of(func_name)
            .ok_or_else(|| InterpreterError::UnknownFunctionName(func_name.to_string()))?;
        self.call_by_index(index, arguments)
    }

    /// Call a function by its index in the [FunctionStore]; this is a proxy for
    /// `Interpreter::call`.
    pub fn call_by_index(
        &mut self,
        index: FuncIndex,
        arguments: &[DataValue],
    ) -> Result<ControlFlow<'a, DataValue>, InterpreterError> {
        match self.state.functions.get_by_index(index) {
            None => Err(InterpreterError::UnknownFunctionIndex(index)),
            Some(func) => self.call(func, arguments),
        }
    }

    /// Interpret a call to a [Function] given its [DataValue] arguments.
    fn call(
        &mut self,
        function: &'a Function,
        arguments: &[DataValue],
    ) -> Result<ControlFlow<'a, DataValue>, InterpreterError> {
        trace!("Call: {}({:?})", function.name, arguments);
        let first_block = function
            .layout
            .blocks()
            .next()
            .expect("to have a first block");
        let parameters = function.dfg.block_params(first_block);
        self.state.push_frame(function);
        self.state
            .current_frame_mut()
            .set_all(parameters, arguments.to_vec());
        self.block(first_block)
    }

    /// Interpret a [Block] in a [Function]. This drives the interpretation over sequences of
    /// instructions, which may continue in other blocks, until the function returns.
    fn block(&mut self, block: Block) -> Result<ControlFlow<'a, DataValue>, InterpreterError> {
        trace!("Block: {}", block);
        let function = self.state.current_frame_mut().function;
        let layout = &function.layout;
        let mut maybe_inst = layout.first_inst(block);
        while let Some(inst) = maybe_inst {
            if self.consume_fuel() == FuelResult::Stop {
                return Err(InterpreterError::FuelExhausted);
            }

            let inst_context = DfgInstructionContext::new(inst, &function.dfg);
            match step(&mut self.state, inst_context)? {
                ControlFlow::Assign(values) => {
                    self.state
                        .current_frame_mut()
                        .set_all(function.dfg.inst_results(inst), values.to_vec());
                    maybe_inst = layout.next_inst(inst)
                }
                ControlFlow::Continue => maybe_inst = layout.next_inst(inst),
                ControlFlow::ContinueAt(block, block_arguments) => {
                    trace!("Block: {}", block);
                    self.state
                        .current_frame_mut()
                        .set_all(function.dfg.block_params(block), block_arguments.to_vec());
                    maybe_inst = layout.first_inst(block)
                }
                ControlFlow::Call(called_function, arguments) => {
                    let returned_arguments =
                        self.call(called_function, &arguments)?.unwrap_return();
                    self.state
                        .current_frame_mut()
                        .set_all(function.dfg.inst_results(inst), returned_arguments);
                    maybe_inst = layout.next_inst(inst)
                }
                ControlFlow::Return(returned_values) => {
                    self.state.pop_frame();
                    return Ok(ControlFlow::Return(returned_values));
                }
                ControlFlow::Trap(trap) => return Ok(ControlFlow::Trap(trap)),
            }
        }
        Err(InterpreterError::Unreachable)
    }

    fn consume_fuel(&mut self) -> FuelResult {
        match self.fuel {
            Some(0) => FuelResult::Stop,
            Some(ref mut n) => {
                *n -= 1;
                FuelResult::Continue
            }

            // We do not have fuel enabled, so unconditionally continue
            None => FuelResult::Continue,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
/// The result of consuming fuel. Signals if the caller should stop or continue.
pub enum FuelResult {
    /// We still have `fuel` available and should continue execution.
    Continue,
    /// The available `fuel` has been exhausted, we should stop now.
    Stop,
}

/// The ways interpretation can fail.
#[derive(Error, Debug)]
pub enum InterpreterError {
    #[error("failed to interpret instruction")]
    StepError(#[from] StepError),
    #[error("reached an unreachable statement")]
    Unreachable,
    #[error("unknown function index (has it been added to the function store?): {0}")]
    UnknownFunctionIndex(FuncIndex),
    #[error("unknown function with name (has it been added to the function store?): {0}")]
    UnknownFunctionName(String),
    #[error("value error")]
    ValueError(#[from] ValueError),
    #[error("fuel exhausted")]
    FuelExhausted,
}

/// Maintains the [Interpreter]'s state, implementing the [State] trait.
pub struct InterpreterState<'a> {
    pub functions: FunctionStore<'a>,
    pub frame_stack: Vec<Frame<'a>>,
    pub heap: Vec<u8>,
    pub iflags: HashSet<IntCC>,
    pub fflags: HashSet<FloatCC>,
}

impl Default for InterpreterState<'_> {
    fn default() -> Self {
        Self {
            functions: FunctionStore::default(),
            frame_stack: vec![],
            heap: vec![0; 1024],
            iflags: HashSet::new(),
            fflags: HashSet::new(),
        }
    }
}

impl<'a> InterpreterState<'a> {
    pub fn with_function_store(self, functions: FunctionStore<'a>) -> Self {
        Self { functions, ..self }
    }

    fn current_frame_mut(&mut self) -> &mut Frame<'a> {
        let num_frames = self.frame_stack.len();
        match num_frames {
            0 => panic!("unable to retrieve the current frame because no frames were pushed"),
            _ => &mut self.frame_stack[num_frames - 1],
        }
    }

    fn current_frame(&self) -> &Frame<'a> {
        let num_frames = self.frame_stack.len();
        match num_frames {
            0 => panic!("unable to retrieve the current frame because no frames were pushed"),
            _ => &self.frame_stack[num_frames - 1],
        }
    }
}

impl<'a> State<'a, DataValue> for InterpreterState<'a> {
    fn get_function(&self, func_ref: FuncRef) -> Option<&'a Function> {
        self.functions
            .get_from_func_ref(func_ref, self.frame_stack.last().unwrap().function)
    }
    fn get_current_function(&self) -> &'a Function {
        self.current_frame().function
    }

    fn push_frame(&mut self, function: &'a Function) {
        self.frame_stack.push(Frame::new(function));
    }
    fn pop_frame(&mut self) {
        self.frame_stack.pop();
    }

    fn get_value(&self, name: ValueRef) -> Option<DataValue> {
        Some(self.current_frame().get(name).clone()) // TODO avoid clone?
    }

    fn set_value(&mut self, name: ValueRef, value: DataValue) -> Option<DataValue> {
        self.current_frame_mut().set(name, value)
    }

    fn has_iflag(&self, flag: IntCC) -> bool {
        self.iflags.contains(&flag)
    }

    fn has_fflag(&self, flag: FloatCC) -> bool {
        self.fflags.contains(&flag)
    }

    fn set_iflag(&mut self, flag: IntCC) {
        self.iflags.insert(flag);
    }

    fn set_fflag(&mut self, flag: FloatCC) {
        self.fflags.insert(flag);
    }

    fn clear_flags(&mut self) {
        self.iflags.clear();
        self.fflags.clear()
    }

    fn load_heap(&self, offset: usize, ty: Type) -> Result<DataValue, MemoryError> {
        if offset + 16 < self.heap.len() {
            let pointer = self.heap[offset..offset + 16].as_ptr() as *const _ as *const u128;
            Ok(unsafe { DataValue::read_value_from(pointer, ty) })
        } else {
            Err(MemoryError::InsufficientMemory(offset, self.heap.len()))
        }
    }

    fn store_heap(&mut self, offset: usize, v: DataValue) -> Result<(), MemoryError> {
        if offset + 16 < self.heap.len() {
            let pointer = self.heap[offset..offset + 16].as_mut_ptr() as *mut _ as *mut u128;
            Ok(unsafe { v.write_value_to(pointer) })
        } else {
            Err(MemoryError::InsufficientMemory(offset, self.heap.len()))
        }
    }

    fn load_stack(&self, _offset: usize, _ty: Type) -> Result<DataValue, MemoryError> {
        unimplemented!()
    }

    fn store_stack(&mut self, _offset: usize, _v: DataValue) -> Result<(), MemoryError> {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::step::CraneliftTrap;
    use cranelift_codegen::ir::immediates::Ieee32;
    use cranelift_codegen::ir::TrapCode;
    use cranelift_reader::parse_functions;

    // Most interpreter tests should use the more ergonomic `test interpret` filetest but this
    // unit test serves as a sanity check that the interpreter still works without all of the
    // filetest infrastructure.
    #[test]
    fn sanity() {
        let code = "function %test() -> b1 {
        block0:
            v0 = iconst.i32 1
            v1 = iadd_imm v0, 1
            v2 = irsub_imm v1, 44  ; 44 - 2 == 42 (see irsub_imm's semantics)
            v3 = icmp_imm eq v2, 42
            return v3
        }";

        let func = parse_functions(code).unwrap().into_iter().next().unwrap();
        let mut env = FunctionStore::default();
        env.add(func.name.to_string(), &func);
        let state = InterpreterState::default().with_function_store(env);
        let result = Interpreter::new(state)
            .call_by_name("%test", &[])
            .unwrap()
            .unwrap_return();

        assert_eq!(result, vec![DataValue::B(true)])
    }

    // We don't have a way to check for traps with the current filetest infrastructure
    #[test]
    fn udiv_by_zero_traps() {
        let code = "function %test() -> i32 {
        block0:
            v0 = iconst.i32 1
            v1 = udiv_imm.i32 v0, 0
            return v1
        }";

        let func = parse_functions(code).unwrap().into_iter().next().unwrap();
        let mut env = FunctionStore::default();
        env.add(func.name.to_string(), &func);
        let state = InterpreterState::default().with_function_store(env);
        let result = Interpreter::new(state).call_by_name("%test", &[]).unwrap();

        match result {
            ControlFlow::Trap(CraneliftTrap::User(TrapCode::IntegerDivisionByZero)) => {}
            _ => panic!("Unexpected ControlFlow: {:?}", result),
        }
    }

    // This test verifies that functions can refer to each other using the function store. A double indirection is
    // required, which is tricky to get right: a referenced function is a FuncRef when called but a FuncIndex inside the
    // function store. This test would preferably be a CLIF filetest but the filetest infrastructure only looks at a
    // single function at a time--we need more than one function in the store for this test.
    #[test]
    fn function_references() {
        let code = "
        function %child(i32) -> i32 {
        block0(v0: i32):
            v1 = iadd_imm v0, -1
            return v1
        }

        function %parent(i32) -> i32 {
            fn42 = %child(i32) -> i32
        block0(v0: i32):
            v1 = iadd_imm v0, 1
            v2 = call fn42(v1)
            return v2
        }";

        let mut env = FunctionStore::default();
        let funcs = parse_functions(code).unwrap().to_vec();
        funcs.iter().for_each(|f| env.add(f.name.to_string(), f));

        let state = InterpreterState::default().with_function_store(env);
        let result = Interpreter::new(state)
            .call_by_name("%parent", &[DataValue::I32(0)])
            .unwrap()
            .unwrap_return();

        assert_eq!(result, vec![DataValue::I32(0)])
    }

    #[test]
    fn state_heap_roundtrip() -> Result<(), MemoryError> {
        let mut state = InterpreterState::default();
        let mut roundtrip = |dv: DataValue| {
            state.store_heap(0, dv.clone())?;
            assert_eq!(dv, state.load_heap(0, dv.ty())?);
            Ok(())
        };

        roundtrip(DataValue::B(true))?;
        roundtrip(DataValue::I64(42))?;
        roundtrip(DataValue::F32(Ieee32::from(0.42)))
    }

    #[test]
    fn state_flags() {
        let mut state = InterpreterState::default();
        let flag = IntCC::Overflow;
        assert!(!state.has_iflag(flag));
        state.set_iflag(flag);
        assert!(state.has_iflag(flag));
        state.clear_flags();
        assert!(!state.has_iflag(flag));
    }

    #[test]
    fn fuel() {
        let code = "function %test() -> b1 {
        block0:
            v0 = iconst.i32 1
            v1 = iadd_imm v0, 1
            return v1
        }";

        let func = parse_functions(code).unwrap().into_iter().next().unwrap();
        let mut env = FunctionStore::default();
        env.add(func.name.to_string(), &func);

        // The default interpreter should not enable the fuel mechanism
        let state = InterpreterState::default().with_function_store(env.clone());
        let result = Interpreter::new(state)
            .call_by_name("%test", &[])
            .unwrap()
            .unwrap_return();
        assert_eq!(result, vec![DataValue::I32(2)]);

        // With 2 fuel, we should execute the iconst and iadd, but not the return thus giving a
        // fuel exhausted error
        let state = InterpreterState::default().with_function_store(env.clone());
        let result = Interpreter::new(state)
            .with_fuel(Some(2))
            .call_by_name("%test", &[]);
        match result {
            Err(InterpreterError::FuelExhausted) => {}
            _ => panic!("Expected Err(FuelExhausted), but got {:?}", result),
        }

        // With 3 fuel, we should be able to execute the return instruction, and complete the test
        let state = InterpreterState::default().with_function_store(env.clone());
        let result = Interpreter::new(state)
            .with_fuel(Some(3))
            .call_by_name("%test", &[])
            .unwrap()
            .unwrap_return();
        assert_eq!(result, vec![DataValue::I32(2)]);
    }
}
