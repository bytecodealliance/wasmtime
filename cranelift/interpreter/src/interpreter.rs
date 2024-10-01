//! Cranelift IR interpreter.
//!
//! This module partially contains the logic for interpreting Cranelift IR.

use crate::address::{Address, AddressFunctionEntry, AddressRegion, AddressSize};
use crate::environment::{FuncIndex, FunctionStore};
use crate::frame::Frame;
use crate::instruction::DfgInstructionContext;
use crate::state::{InterpreterFunctionRef, MemoryError, State};
use crate::step::{step, ControlFlow, CraneliftTrap, StepError};
use crate::value::{DataValueExt, ValueError};
use cranelift_codegen::data_value::DataValue;
use cranelift_codegen::ir::{
    ArgumentPurpose, Block, Endianness, ExternalName, FuncRef, Function, GlobalValue,
    GlobalValueData, LibCall, MemFlags, StackSlot, Type,
};
use log::trace;
use smallvec::SmallVec;
use std::fmt::Debug;
use std::iter;
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
    ) -> Result<ControlFlow<'a>, InterpreterError> {
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
    ) -> Result<ControlFlow<'a>, InterpreterError> {
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
    ) -> Result<ControlFlow<'a>, InterpreterError> {
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
    fn block(&mut self, block: Block) -> Result<ControlFlow<'a>, InterpreterError> {
        trace!("Block: {}", block);
        let function = self.state.current_frame_mut().function();
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
                    match self.call(called_function, &arguments)? {
                        ControlFlow::Return(rets) => {
                            self.state
                                .current_frame_mut()
                                .set_all(function.dfg.inst_results(inst), rets.to_vec());
                            maybe_inst = layout.next_inst(inst)
                        }
                        ControlFlow::Trap(trap) => return Ok(ControlFlow::Trap(trap)),
                        cf => {
                            panic!("invalid control flow after call: {cf:?}")
                        }
                    }
                }
                ControlFlow::ReturnCall(callee, args) => {
                    self.state.pop_frame();

                    return match self.call(callee, &args)? {
                        ControlFlow::Return(rets) => Ok(ControlFlow::Return(rets)),
                        ControlFlow::Trap(trap) => Ok(ControlFlow::Trap(trap)),
                        cf => {
                            panic!("invalid control flow after return_call: {cf:?}")
                        }
                    };
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

pub type LibCallValues = SmallVec<[DataValue; 1]>;
pub type LibCallHandler = fn(LibCall, LibCallValues) -> Result<LibCallValues, CraneliftTrap>;

/// Maintains the [Interpreter]'s state, implementing the [State] trait.
pub struct InterpreterState<'a> {
    pub functions: FunctionStore<'a>,
    pub libcall_handler: LibCallHandler,
    pub frame_stack: Vec<Frame<'a>>,
    /// Number of bytes from the bottom of the stack where the current frame's stack space is
    pub frame_offset: usize,
    pub stack: Vec<u8>,
    pub pinned_reg: DataValue,
    pub native_endianness: Endianness,
}

impl Default for InterpreterState<'_> {
    fn default() -> Self {
        let native_endianness = if cfg!(target_endian = "little") {
            Endianness::Little
        } else {
            Endianness::Big
        };
        Self {
            functions: FunctionStore::default(),
            libcall_handler: |_, _| Err(CraneliftTrap::UnreachableCodeReached),
            frame_stack: vec![],
            frame_offset: 0,
            stack: Vec::with_capacity(1024),
            pinned_reg: DataValue::I64(0),
            native_endianness,
        }
    }
}

impl<'a> InterpreterState<'a> {
    pub fn with_function_store(self, functions: FunctionStore<'a>) -> Self {
        Self { functions, ..self }
    }

    /// Registers a libcall handler
    pub fn with_libcall_handler(mut self, handler: LibCallHandler) -> Self {
        self.libcall_handler = handler;
        self
    }
}

impl<'a> State<'a> for InterpreterState<'a> {
    fn get_function(&self, func_ref: FuncRef) -> Option<&'a Function> {
        self.functions
            .get_from_func_ref(func_ref, self.frame_stack.last().unwrap().function())
    }
    fn get_current_function(&self) -> &'a Function {
        self.current_frame().function()
    }

    fn get_libcall_handler(&self) -> LibCallHandler {
        self.libcall_handler
    }

    fn push_frame(&mut self, function: &'a Function) {
        if let Some(frame) = self.frame_stack.iter().last() {
            self.frame_offset += frame.function().fixed_stack_size() as usize;
        }

        // Grow the stack by the space necessary for this frame
        self.stack
            .extend(iter::repeat(0).take(function.fixed_stack_size() as usize));

        self.frame_stack.push(Frame::new(function));
    }
    fn pop_frame(&mut self) {
        if let Some(frame) = self.frame_stack.pop() {
            // Shorten the stack after exiting the frame
            self.stack
                .truncate(self.stack.len() - frame.function().fixed_stack_size() as usize);

            // Reset frame_offset to the start of this function
            if let Some(frame) = self.frame_stack.iter().last() {
                self.frame_offset -= frame.function().fixed_stack_size() as usize;
            }
        }
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

    fn stack_address(
        &self,
        size: AddressSize,
        slot: StackSlot,
        offset: u64,
    ) -> Result<Address, MemoryError> {
        let stack_slots = &self.get_current_function().sized_stack_slots;
        let stack_slot = &stack_slots[slot];

        // offset must be `0 <= Offset < sizeof(SS)`
        if offset >= stack_slot.size as u64 {
            return Err(MemoryError::InvalidOffset {
                offset,
                max: stack_slot.size as u64,
            });
        }

        // Calculate the offset from the current frame to the requested stack slot
        let slot_offset: u64 = stack_slots
            .keys()
            .filter(|k| k < &slot)
            .map(|k| stack_slots[k].size as u64)
            .sum();

        let final_offset = self.frame_offset as u64 + slot_offset + offset;
        Address::from_parts(size, AddressRegion::Stack, 0, final_offset)
    }

    fn checked_load(
        &self,
        addr: Address,
        ty: Type,
        mem_flags: MemFlags,
    ) -> Result<DataValue, MemoryError> {
        let load_size = ty.bytes() as usize;
        let addr_start = addr.offset as usize;
        let addr_end = addr_start + load_size;

        let src = match addr.region {
            AddressRegion::Stack => {
                if addr_end > self.stack.len() {
                    return Err(MemoryError::OutOfBoundsLoad {
                        addr,
                        load_size,
                        mem_flags,
                    });
                }

                &self.stack[addr_start..addr_end]
            }
            _ => unimplemented!(),
        };

        // Aligned flag is set and address is not aligned for the given type
        if mem_flags.aligned() && addr_start % load_size != 0 {
            return Err(MemoryError::MisalignedLoad { addr, load_size });
        }

        Ok(match mem_flags.endianness(self.native_endianness) {
            Endianness::Big => DataValue::read_from_slice_be(src, ty),
            Endianness::Little => DataValue::read_from_slice_le(src, ty),
        })
    }

    fn checked_store(
        &mut self,
        addr: Address,
        v: DataValue,
        mem_flags: MemFlags,
    ) -> Result<(), MemoryError> {
        let store_size = v.ty().bytes() as usize;
        let addr_start = addr.offset as usize;
        let addr_end = addr_start + store_size;

        let dst = match addr.region {
            AddressRegion::Stack => {
                if addr_end > self.stack.len() {
                    return Err(MemoryError::OutOfBoundsStore {
                        addr,
                        store_size,
                        mem_flags,
                    });
                }

                &mut self.stack[addr_start..addr_end]
            }
            _ => unimplemented!(),
        };

        // Aligned flag is set and address is not aligned for the given type
        if mem_flags.aligned() && addr_start % store_size != 0 {
            return Err(MemoryError::MisalignedStore { addr, store_size });
        }

        Ok(match mem_flags.endianness(self.native_endianness) {
            Endianness::Big => v.write_to_slice_be(dst),
            Endianness::Little => v.write_to_slice_le(dst),
        })
    }

    fn function_address(
        &self,
        size: AddressSize,
        name: &ExternalName,
    ) -> Result<Address, MemoryError> {
        let curr_func = self.get_current_function();
        let (entry, index) = match name {
            ExternalName::User(username) => {
                let ext_name = &curr_func.params.user_named_funcs()[*username];

                // TODO: This is not optimal since we are looking up by string name
                let index = self.functions.index_of(&ext_name.to_string()).unwrap();

                (AddressFunctionEntry::UserFunction, index.as_u32())
            }

            ExternalName::TestCase(testname) => {
                // TODO: This is not optimal since we are looking up by string name
                let index = self.functions.index_of(&testname.to_string()).unwrap();

                (AddressFunctionEntry::UserFunction, index.as_u32())
            }
            ExternalName::LibCall(libcall) => {
                // We don't properly have a "libcall" store, but we can use `LibCall::all()`
                // and index into that.
                let index = LibCall::all_libcalls()
                    .iter()
                    .position(|lc| lc == libcall)
                    .unwrap();

                (AddressFunctionEntry::LibCall, index as u32)
            }
            _ => unimplemented!("function_address: {:?}", name),
        };

        Address::from_parts(size, AddressRegion::Function, entry as u64, index as u64)
    }

    fn get_function_from_address(&self, address: Address) -> Option<InterpreterFunctionRef<'a>> {
        let index = address.offset as u32;
        if address.region != AddressRegion::Function {
            return None;
        }

        match AddressFunctionEntry::from(address.entry) {
            AddressFunctionEntry::UserFunction => self
                .functions
                .get_by_index(FuncIndex::from_u32(index))
                .map(InterpreterFunctionRef::from),

            AddressFunctionEntry::LibCall => LibCall::all_libcalls()
                .get(index as usize)
                .copied()
                .map(InterpreterFunctionRef::from),
        }
    }

    /// Non-Recursively resolves a global value until its address is found
    fn resolve_global_value(&self, gv: GlobalValue) -> Result<DataValue, MemoryError> {
        // Resolving a Global Value is a "pointer" chasing operation that lends itself to
        // using a recursive solution. However, resolving this in a recursive manner
        // is a bad idea because its very easy to add a bunch of global values and
        // blow up the call stack.
        //
        // Adding to the challenges of this, is that the operations possible with GlobalValues
        // mean that we cannot use a simple loop to resolve each global value, we must keep
        // a pending list of operations.

        // These are the possible actions that we can perform
        #[derive(Debug)]
        enum ResolveAction {
            Resolve(GlobalValue),
            /// Perform an add on the current address
            Add(DataValue),
            /// Load From the current address and replace it with the loaded value
            Load {
                /// Offset added to the base pointer before doing the load.
                offset: i32,

                /// Type of the loaded value.
                global_type: Type,
            },
        }

        let func = self.get_current_function();

        // We start with a sentinel value that will fail if we try to load / add to it
        // without resolving the base GV First.
        let mut current_val = DataValue::I8(0);
        let mut action_stack = vec![ResolveAction::Resolve(gv)];

        loop {
            match action_stack.pop() {
                Some(ResolveAction::Resolve(gv)) => match func.global_values[gv] {
                    GlobalValueData::VMContext => {
                        // Fetch the VMContext value from the values of the first block in the function
                        let index = func
                            .signature
                            .params
                            .iter()
                            .enumerate()
                            .find(|(_, p)| p.purpose == ArgumentPurpose::VMContext)
                            .map(|(i, _)| i)
                            // This should be validated by the verifier
                            .expect("No VMCtx argument was found, but one is referenced");

                        let first_block =
                            func.layout.blocks().next().expect("to have a first block");
                        let vmctx_value = func.dfg.block_params(first_block)[index];
                        current_val = self.current_frame().get(vmctx_value).clone();
                    }
                    GlobalValueData::Load {
                        base,
                        offset,
                        global_type,
                        ..
                    } => {
                        action_stack.push(ResolveAction::Load {
                            offset: offset.into(),
                            global_type,
                        });
                        action_stack.push(ResolveAction::Resolve(base));
                    }
                    GlobalValueData::IAddImm {
                        base,
                        offset,
                        global_type,
                    } => {
                        let offset: i64 = offset.into();
                        let dv = DataValue::int(offset as i128, global_type)
                            .map_err(|_| MemoryError::InvalidAddressType(global_type))?;
                        action_stack.push(ResolveAction::Add(dv));
                        action_stack.push(ResolveAction::Resolve(base));
                    }
                    GlobalValueData::Symbol { .. } => unimplemented!(),
                    GlobalValueData::DynScaleTargetConst { .. } => unimplemented!(),
                },
                Some(ResolveAction::Add(dv)) => {
                    current_val = current_val
                        .add(dv.clone())
                        .map_err(|_| MemoryError::InvalidAddress(dv))?;
                }
                Some(ResolveAction::Load {
                    offset,
                    global_type,
                }) => {
                    let mut addr = Address::try_from(current_val)?;
                    let mem_flags = MemFlags::trusted();
                    // We can forego bounds checking here since its performed in `checked_load`
                    addr.offset += offset as u64;
                    current_val = self.checked_load(addr, global_type, mem_flags)?;
                }

                // We are done resolving this, return the current value
                None => return Ok(current_val),
            }
        }
    }

    fn get_pinned_reg(&self) -> DataValue {
        self.pinned_reg.clone()
    }

    fn set_pinned_reg(&mut self, v: DataValue) {
        self.pinned_reg = v;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::step::CraneliftTrap;
    use cranelift_codegen::ir::immediates::Ieee32;
    use cranelift_codegen::ir::TrapCode;
    use cranelift_reader::parse_functions;
    use smallvec::smallvec;

    // Most interpreter tests should use the more ergonomic `test interpret` filetest but this
    // unit test serves as a sanity check that the interpreter still works without all of the
    // filetest infrastructure.
    #[test]
    fn sanity() {
        let code = "function %test() -> i8 {
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
        let result = Interpreter::new(state).call_by_name("%test", &[]).unwrap();

        assert_eq!(result, ControlFlow::Return(smallvec![DataValue::I8(1)]));
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
        let trap = Interpreter::new(state).call_by_name("%test", &[]).unwrap();

        assert_eq!(
            trap,
            ControlFlow::Trap(CraneliftTrap::User(TrapCode::INTEGER_DIVISION_BY_ZERO))
        );
    }

    #[test]
    fn sdiv_min_by_neg_one_traps_with_overflow() {
        let code = "function %test() -> i8 {
        block0:
            v0 = iconst.i32 -2147483648
            v1 = sdiv_imm.i32 v0, -1
            return v1
        }";

        let func = parse_functions(code).unwrap().into_iter().next().unwrap();
        let mut env = FunctionStore::default();
        env.add(func.name.to_string(), &func);
        let state = InterpreterState::default().with_function_store(env);
        let result = Interpreter::new(state).call_by_name("%test", &[]).unwrap();

        match result {
            ControlFlow::Trap(CraneliftTrap::User(TrapCode::INTEGER_OVERFLOW)) => {}
            _ => panic!("Unexpected ControlFlow: {result:?}"),
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
            .unwrap();

        assert_eq!(result, ControlFlow::Return(smallvec![DataValue::I32(0)]));
    }

    #[test]
    fn fuel() {
        let code = "function %test() -> i8 {
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
        let result = Interpreter::new(state).call_by_name("%test", &[]).unwrap();

        assert_eq!(result, ControlFlow::Return(smallvec![DataValue::I32(2)]));

        // With 2 fuel, we should execute the iconst and iadd, but not the return thus giving a
        // fuel exhausted error
        let state = InterpreterState::default().with_function_store(env.clone());
        let result = Interpreter::new(state)
            .with_fuel(Some(2))
            .call_by_name("%test", &[]);
        match result {
            Err(InterpreterError::FuelExhausted) => {}
            _ => panic!("Expected Err(FuelExhausted), but got {result:?}"),
        }

        // With 3 fuel, we should be able to execute the return instruction, and complete the test
        let state = InterpreterState::default().with_function_store(env.clone());
        let result = Interpreter::new(state)
            .with_fuel(Some(3))
            .call_by_name("%test", &[])
            .unwrap();

        assert_eq!(result, ControlFlow::Return(smallvec![DataValue::I32(2)]));
    }

    // Verifies that writing to the stack on a called function does not overwrite the parents
    // stack slots.
    #[test]
    fn stack_slots_multi_functions() {
        let code = "
        function %callee(i64, i64) -> i64 {
            ss0 = explicit_slot 8
            ss1 = explicit_slot 8

        block0(v0: i64, v1: i64):
            stack_store.i64 v0, ss0
            stack_store.i64 v1, ss1
            v2 = stack_load.i64 ss0
            v3 = stack_load.i64 ss1
            v4 = iadd.i64 v2, v3
            return v4
        }

        function %caller(i64, i64, i64, i64) -> i64 {
            fn0 = %callee(i64, i64) -> i64
            ss0 = explicit_slot 8
            ss1 = explicit_slot 8

        block0(v0: i64, v1: i64, v2: i64, v3: i64):
            stack_store.i64 v0, ss0
            stack_store.i64 v1, ss1

            v4 = call fn0(v2, v3)

            v5 = stack_load.i64 ss0
            v6 = stack_load.i64 ss1

            v7 = iadd.i64 v4, v5
            v8 = iadd.i64 v7, v6

            return v8
        }";

        let mut env = FunctionStore::default();
        let funcs = parse_functions(code).unwrap().to_vec();
        funcs.iter().for_each(|f| env.add(f.name.to_string(), f));

        let state = InterpreterState::default().with_function_store(env);
        let result = Interpreter::new(state)
            .call_by_name(
                "%caller",
                &[
                    DataValue::I64(3),
                    DataValue::I64(5),
                    DataValue::I64(7),
                    DataValue::I64(11),
                ],
            )
            .unwrap();

        assert_eq!(result, ControlFlow::Return(smallvec![DataValue::I64(26)]))
    }

    #[test]
    fn out_of_slot_write_traps() {
        let code = "
        function %stack_write() {
            ss0 = explicit_slot 8

        block0:
            v0 = iconst.i64 10
            stack_store.i64 v0, ss0+8
            return
        }";

        let func = parse_functions(code).unwrap().into_iter().next().unwrap();
        let mut env = FunctionStore::default();
        env.add(func.name.to_string(), &func);
        let state = InterpreterState::default().with_function_store(env);
        let trap = Interpreter::new(state)
            .call_by_name("%stack_write", &[])
            .unwrap();

        assert_eq!(
            trap,
            ControlFlow::Trap(CraneliftTrap::User(TrapCode::HEAP_OUT_OF_BOUNDS))
        );
    }

    #[test]
    fn partial_out_of_slot_write_traps() {
        let code = "
        function %stack_write() {
            ss0 = explicit_slot 8

        block0:
            v0 = iconst.i64 10
            stack_store.i64 v0, ss0+4
            return
        }";

        let func = parse_functions(code).unwrap().into_iter().next().unwrap();
        let mut env = FunctionStore::default();
        env.add(func.name.to_string(), &func);
        let state = InterpreterState::default().with_function_store(env);
        let trap = Interpreter::new(state)
            .call_by_name("%stack_write", &[])
            .unwrap();

        assert_eq!(
            trap,
            ControlFlow::Trap(CraneliftTrap::User(TrapCode::HEAP_OUT_OF_BOUNDS))
        );
    }

    #[test]
    fn out_of_slot_read_traps() {
        let code = "
        function %stack_load() {
            ss0 = explicit_slot 8

        block0:
            v0 = stack_load.i64 ss0+8
            return
        }";

        let func = parse_functions(code).unwrap().into_iter().next().unwrap();
        let mut env = FunctionStore::default();
        env.add(func.name.to_string(), &func);
        let state = InterpreterState::default().with_function_store(env);
        let trap = Interpreter::new(state)
            .call_by_name("%stack_load", &[])
            .unwrap();

        assert_eq!(
            trap,
            ControlFlow::Trap(CraneliftTrap::User(TrapCode::HEAP_OUT_OF_BOUNDS))
        );
    }

    #[test]
    fn partial_out_of_slot_read_traps() {
        let code = "
        function %stack_load() {
            ss0 = explicit_slot 8

        block0:
            v0 = stack_load.i64 ss0+4
            return
        }";

        let func = parse_functions(code).unwrap().into_iter().next().unwrap();
        let mut env = FunctionStore::default();
        env.add(func.name.to_string(), &func);
        let state = InterpreterState::default().with_function_store(env);
        let trap = Interpreter::new(state)
            .call_by_name("%stack_load", &[])
            .unwrap();

        assert_eq!(
            trap,
            ControlFlow::Trap(CraneliftTrap::User(TrapCode::HEAP_OUT_OF_BOUNDS))
        );
    }

    #[test]
    fn partial_out_of_slot_read_by_addr_traps() {
        let code = "
        function %stack_load() {
            ss0 = explicit_slot 8

        block0:
            v0 = stack_addr.i64 ss0
            v1 = iconst.i64 4
            v2 = iadd.i64 v0, v1
            v3 = load.i64 v2
            return
        }";

        let func = parse_functions(code).unwrap().into_iter().next().unwrap();
        let mut env = FunctionStore::default();
        env.add(func.name.to_string(), &func);
        let state = InterpreterState::default().with_function_store(env);
        let trap = Interpreter::new(state)
            .call_by_name("%stack_load", &[])
            .unwrap();

        assert_eq!(
            trap,
            ControlFlow::Trap(CraneliftTrap::User(TrapCode::HEAP_OUT_OF_BOUNDS))
        );
    }

    #[test]
    fn partial_out_of_slot_write_by_addr_traps() {
        let code = "
        function %stack_store() {
            ss0 = explicit_slot 8

        block0:
            v0 = stack_addr.i64 ss0
            v1 = iconst.i64 4
            v2 = iadd.i64 v0, v1
            store.i64 v1, v2
            return
        }";

        let func = parse_functions(code).unwrap().into_iter().next().unwrap();
        let mut env = FunctionStore::default();
        env.add(func.name.to_string(), &func);
        let state = InterpreterState::default().with_function_store(env);
        let trap = Interpreter::new(state)
            .call_by_name("%stack_store", &[])
            .unwrap();

        assert_eq!(
            trap,
            ControlFlow::Trap(CraneliftTrap::User(TrapCode::HEAP_OUT_OF_BOUNDS))
        );
    }

    #[test]
    fn srem_trap() {
        let code = "function %test() -> i64 {
        block0:
            v0 = iconst.i64 0x8000_0000_0000_0000
            v1 = iconst.i64 -1
            v2 = srem.i64 v0, v1
            return v2
        }";

        let func = parse_functions(code).unwrap().into_iter().next().unwrap();
        let mut env = FunctionStore::default();
        env.add(func.name.to_string(), &func);
        let state = InterpreterState::default().with_function_store(env);
        let trap = Interpreter::new(state).call_by_name("%test", &[]).unwrap();

        assert_eq!(
            trap,
            ControlFlow::Trap(CraneliftTrap::User(TrapCode::INTEGER_OVERFLOW))
        );
    }

    #[test]
    fn libcall() {
        let code = "function %test() -> i64 {
            fn0 = colocated %CeilF32 (f32) -> f32 fast
        block0:
            v1 = f32const 0x0.5
            v2 = call fn0(v1)
            return v2
        }";

        let func = parse_functions(code).unwrap().into_iter().next().unwrap();
        let mut env = FunctionStore::default();
        env.add(func.name.to_string(), &func);
        let state = InterpreterState::default()
            .with_function_store(env)
            .with_libcall_handler(|libcall, args| {
                Ok(smallvec![match (libcall, &args[..]) {
                    (LibCall::CeilF32, [DataValue::F32(a)]) => DataValue::F32(a.ceil()),
                    _ => panic!("Unexpected args"),
                }])
            });

        let result = Interpreter::new(state).call_by_name("%test", &[]).unwrap();

        assert_eq!(
            result,
            ControlFlow::Return(smallvec![DataValue::F32(Ieee32::with_float(1.0))])
        )
    }

    #[test]
    fn misaligned_store_traps() {
        let code = "
        function %test() {
            ss0 = explicit_slot 16

        block0:
            v0 = stack_addr.i64 ss0
            v1 = iconst.i64 1
            store.i64 aligned v1, v0+2
            return
        }";

        let func = parse_functions(code).unwrap().into_iter().next().unwrap();
        let mut env = FunctionStore::default();
        env.add(func.name.to_string(), &func);
        let state = InterpreterState::default().with_function_store(env);
        let trap = Interpreter::new(state).call_by_name("%test", &[]).unwrap();

        assert_eq!(trap, ControlFlow::Trap(CraneliftTrap::HeapMisaligned));
    }

    #[test]
    fn misaligned_load_traps() {
        let code = "
        function %test() {
            ss0 = explicit_slot 16

        block0:
            v0 = stack_addr.i64 ss0
            v1 = iconst.i64 1
            store.i64 aligned v1, v0
            v2 = load.i64 aligned v0+2
            return
        }";

        let func = parse_functions(code).unwrap().into_iter().next().unwrap();
        let mut env = FunctionStore::default();
        env.add(func.name.to_string(), &func);
        let state = InterpreterState::default().with_function_store(env);
        let trap = Interpreter::new(state).call_by_name("%test", &[]).unwrap();

        assert_eq!(trap, ControlFlow::Trap(CraneliftTrap::HeapMisaligned));
    }

    // When a trap occurs in a function called by another function, the trap was not being propagated
    // correctly. Instead the interpterer panicked with a invalid control flow state.
    // See this issue for more details: https://github.com/bytecodealliance/wasmtime/issues/6155
    #[test]
    fn trap_across_call_propagates_correctly() {
        let code = "
        function %u2() -> f32 system_v {
            ss0 = explicit_slot 69
            ss1 = explicit_slot 69
            ss2 = explicit_slot 69

        block0:
            v0 = f32const -0x1.434342p-60
            v1 = stack_addr.i64 ss2+24
            store notrap aligned v0, v1
            return v0
        }

        function %u1() -> f32 system_v {
            sig0 = () -> f32 system_v
            fn0 = colocated %u2 sig0

        block0:
            v57 = call fn0()
            return v57
        }";

        let mut env = FunctionStore::default();

        let funcs = parse_functions(code).unwrap();
        for func in &funcs {
            env.add(func.name.to_string(), func);
        }

        let state = InterpreterState::default().with_function_store(env);
        let trap = Interpreter::new(state).call_by_name("%u1", &[]).unwrap();

        // Ensure that the correct trap was propagated.
        assert_eq!(trap, ControlFlow::Trap(CraneliftTrap::HeapMisaligned));
    }
}
