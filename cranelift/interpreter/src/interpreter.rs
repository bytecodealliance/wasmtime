//! Cranelift IR interpreter.
//!
//! This module contains the logic for interpreting Cranelift instructions.

use crate::environment::Environment;
use crate::frame::Frame;
use crate::interpreter::Trap::InvalidType;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{
    Block, FuncRef, Function, Inst, InstructionData, InstructionData::*, Opcode, Opcode::*, Type,
    Value as ValueRef, ValueList,
};
use cranelift_reader::{DataValue, DataValueCastFailure};
use log::trace;
use std::ops::{Add, Div, Mul, Sub};
use thiserror::Error;

/// The valid control flow states.
pub enum ControlFlow {
    Continue,
    ContinueAt(Block, Vec<ValueRef>),
    Return(Vec<DataValue>),
}

impl ControlFlow {
    /// For convenience, we can unwrap the [ControlFlow] state assuming that it is a
    /// [ControlFlow::Return], panicking otherwise.
    pub fn unwrap_return(self) -> Vec<DataValue> {
        if let ControlFlow::Return(values) = self {
            values
        } else {
            panic!("expected the control flow to be in the return state")
        }
    }
}

/// The ways interpretation can fail.
#[derive(Error, Debug)]
pub enum Trap {
    #[error("unknown trap")]
    Unknown,
    #[error("invalid type for {1}: expected {0}")]
    InvalidType(String, ValueRef),
    #[error("invalid cast")]
    InvalidCast(#[from] DataValueCastFailure),
    #[error("the instruction is not implemented (perhaps for the given types): {0}")]
    Unsupported(Inst),
    #[error("reached an unreachable statement")]
    Unreachable,
    #[error("invalid control flow: {0}")]
    InvalidControlFlow(String),
    #[error("invalid function reference: {0}")]
    InvalidFunctionReference(FuncRef),
    #[error("invalid function name: {0}")]
    InvalidFunctionName(String),
}

/// The Cranelift interpreter; it contains immutable elements such as the function environment and
/// implements the Cranelift IR semantics.
#[derive(Default)]
pub struct Interpreter {
    pub env: Environment,
}

/// Helper for more concise matching.
macro_rules! binary_op {
    ( $op:path[$arg1:ident, $arg2:ident]; [ $( $data_value_ty:ident ),* ]; $inst:ident ) => {
        match ($arg1, $arg2) {
            $( (DataValue::$data_value_ty(a), DataValue::$data_value_ty(b)) => { Ok(DataValue::$data_value_ty($op(a, b))) } )*
            _ => Err(Trap::Unsupported($inst)),
        }
    };
}

impl Interpreter {
    /// Construct a new [Interpreter] using the given [Environment].
    pub fn new(env: Environment) -> Self {
        Self { env }
    }

    /// Call a function by name; this is a helpful proxy for [Interpreter::call_by_index].
    pub fn call_by_name(
        &self,
        func_name: &str,
        arguments: &[DataValue],
    ) -> Result<ControlFlow, Trap> {
        let func_ref = self
            .env
            .index_of(func_name)
            .ok_or_else(|| Trap::InvalidFunctionName(func_name.to_string()))?;
        self.call_by_index(func_ref, arguments)
    }

    /// Call a function by its index in the [Environment]; this is a proxy for [Interpreter::call].
    pub fn call_by_index(
        &self,
        func_ref: FuncRef,
        arguments: &[DataValue],
    ) -> Result<ControlFlow, Trap> {
        match self.env.get_by_func_ref(func_ref) {
            None => Err(Trap::InvalidFunctionReference(func_ref)),
            Some(func) => self.call(func, arguments),
        }
    }

    /// Interpret a call to a [Function] given its [DataValue] arguments.
    fn call(&self, function: &Function, arguments: &[DataValue]) -> Result<ControlFlow, Trap> {
        trace!("Call: {}({:?})", function.name, arguments);
        let first_block = function
            .layout
            .blocks()
            .next()
            .expect("to have a first block");
        let parameters = function.dfg.block_params(first_block);
        let mut frame = Frame::new(function);
        frame.set_all(parameters, arguments.to_vec());
        self.block(&mut frame, first_block)
    }

    /// Interpret a [Block] in a [Function]. This drives the interpretation over sequences of
    /// instructions, which may continue in other blocks, until the function returns.
    fn block(&self, frame: &mut Frame, block: Block) -> Result<ControlFlow, Trap> {
        trace!("Block: {}", block);
        let layout = &frame.function.layout;
        let mut maybe_inst = layout.first_inst(block);
        while let Some(inst) = maybe_inst {
            match self.inst(frame, inst)? {
                ControlFlow::Continue => maybe_inst = layout.next_inst(inst),
                ControlFlow::ContinueAt(block, old_names) => {
                    trace!("Block: {}", block);
                    let new_names = frame.function.dfg.block_params(block);
                    frame.rename(&old_names, new_names);
                    maybe_inst = layout.first_inst(block)
                }
                ControlFlow::Return(rs) => return Ok(ControlFlow::Return(rs)),
            }
        }
        Err(Trap::Unreachable)
    }

    /// Interpret a single [instruction](Inst). This contains a `match`-based dispatch to the
    /// implementations.
    fn inst(&self, frame: &mut Frame, inst: Inst) -> Result<ControlFlow, Trap> {
        use ControlFlow::{Continue, ContinueAt};
        trace!("Inst: {}", &frame.function.dfg.display_inst(inst, None));

        let data = &frame.function.dfg[inst];
        match data {
            Binary { opcode, args } => {
                let arg1 = frame.get(&args[0]);
                let arg2 = frame.get(&args[1]);
                let result = match opcode {
                    Iadd => binary_op!(Add::add[arg1, arg2]; [I8, I16, I32, I64]; inst),
                    Isub => binary_op!(Sub::sub[arg1, arg2]; [I8, I16, I32, I64]; inst),
                    Imul => binary_op!(Mul::mul[arg1, arg2]; [I8, I16, I32, I64]; inst),
                    Fadd => binary_op!(Add::add[arg1, arg2]; [F32, F64]; inst),
                    Fsub => binary_op!(Sub::sub[arg1, arg2]; [F32, F64]; inst),
                    Fmul => binary_op!(Mul::mul[arg1, arg2]; [F32, F64]; inst),
                    Fdiv => binary_op!(Div::div[arg1, arg2]; [F32, F64]; inst),
                    _ => unimplemented!("interpreter does not support opcode yet: {}", opcode),
                }?;
                frame.set(first_result(frame.function, inst), result);
                Ok(Continue)
            }

            BinaryImm { opcode, arg, imm } => {
                let imm = DataValue::from_integer(*imm, type_of(*arg, frame.function))?;
                let arg = frame.get(&arg);
                let result = match opcode {
                    IaddImm => binary_op!(Add::add[arg, imm]; [I8, I16, I32, I64]; inst),
                    IrsubImm => binary_op!(Sub::sub[imm, arg]; [I8, I16, I32, I64]; inst),
                    ImulImm => binary_op!(Mul::mul[arg, imm]; [I8, I16, I32, I64]; inst),
                    _ => unimplemented!("interpreter does not support opcode yet: {}", opcode),
                }?;
                frame.set(first_result(frame.function, inst), result);
                Ok(Continue)
            }

            Branch {
                opcode,
                args,
                destination,
            } => match opcode {
                Brnz => {
                    let mut args = value_refs(frame.function, args);
                    let first = args.remove(0);
                    match frame.get(&first) {
                        DataValue::B(false)
                        | DataValue::I8(0)
                        | DataValue::I16(0)
                        | DataValue::I32(0)
                        | DataValue::I64(0) => Ok(Continue),
                        DataValue::B(true)
                        | DataValue::I8(_)
                        | DataValue::I16(_)
                        | DataValue::I32(_)
                        | DataValue::I64(_) => Ok(ContinueAt(*destination, args)),
                        _ => Err(Trap::InvalidType("boolean or integer".to_string(), args[0])),
                    }
                }
                _ => unimplemented!("interpreter does not support opcode yet: {}", opcode),
            },
            InstructionData::Call { args, func_ref, .. } => {
                // Find the function to call.
                let func_name = function_name_of_func_ref(*func_ref, frame.function);

                // Call function.
                let args = frame.get_all(args.as_slice(&frame.function.dfg.value_lists));
                let result = self.call_by_name(&func_name, &args)?;

                // Save results.
                if let ControlFlow::Return(returned_values) = result {
                    let ssa_values = frame.function.dfg.inst_results(inst);
                    assert_eq!(
                        ssa_values.len(),
                        returned_values.len(),
                        "expected result length ({}) to match SSA values length ({}): {}",
                        returned_values.len(),
                        ssa_values.len(),
                        frame.function.dfg.display_inst(inst, None)
                    );
                    frame.set_all(ssa_values, returned_values);
                    Ok(Continue)
                } else {
                    Err(Trap::InvalidControlFlow(format!(
                        "did not return from: {}",
                        frame.function.dfg.display_inst(inst, None)
                    )))
                }
            }
            InstructionData::Jump {
                opcode,
                destination,
                args,
            } => match opcode {
                Opcode::Fallthrough => {
                    Ok(ContinueAt(*destination, value_refs(frame.function, args)))
                }
                Opcode::Jump => Ok(ContinueAt(*destination, value_refs(frame.function, args))),
                _ => unimplemented!("interpreter does not support opcode yet: {}", opcode),
            },
            IntCompareImm {
                opcode,
                arg,
                cond,
                imm,
            } => match opcode {
                IcmpImm => {
                    let arg_value = match *frame.get(arg) {
                        DataValue::I8(i) => Ok(i as i64),
                        DataValue::I16(i) => Ok(i as i64),
                        DataValue::I32(i) => Ok(i as i64),
                        DataValue::I64(i) => Ok(i),
                        _ => Err(InvalidType("integer".to_string(), *arg)),
                    }?;
                    let imm_value = (*imm).into();
                    let result = match cond {
                        IntCC::UnsignedLessThanOrEqual => arg_value <= imm_value,
                        IntCC::Equal => arg_value == imm_value,
                        _ => unimplemented!(
                            "interpreter does not support condition code yet: {}",
                            cond
                        ),
                    };
                    let res = first_result(frame.function, inst);
                    frame.set(res, DataValue::B(result));
                    Ok(Continue)
                }
                _ => unimplemented!("interpreter does not support opcode yet: {}", opcode),
            },
            MultiAry { opcode, args } => match opcode {
                Return => {
                    let rs: Vec<DataValue> = args
                        .as_slice(&frame.function.dfg.value_lists)
                        .iter()
                        .map(|r| frame.get(r).clone())
                        .collect();
                    Ok(ControlFlow::Return(rs))
                }
                _ => unimplemented!("interpreter does not support opcode yet: {}", opcode),
            },
            NullAry { opcode } => match opcode {
                Nop => Ok(Continue),
                _ => unimplemented!("interpreter does not support opcode yet: {}", opcode),
            },
            UnaryImm { opcode, imm } => match opcode {
                Iconst => {
                    let res = first_result(frame.function, inst);
                    let imm_value = DataValue::from_integer(*imm, type_of(res, frame.function))?;
                    frame.set(res, imm_value);
                    Ok(Continue)
                }
                _ => unimplemented!("interpreter does not support opcode yet: {}", opcode),
            },
            UnaryBool { opcode, imm } => match opcode {
                Bconst => {
                    let res = first_result(frame.function, inst);
                    frame.set(res, DataValue::B(*imm));
                    Ok(Continue)
                }
                _ => unimplemented!("interpreter does not support opcode yet: {}", opcode),
            },

            _ => unimplemented!("interpreter does not support instruction yet: {:?}", data),
        }
    }
}

/// Return the first result of an instruction.
///
/// This helper cushions the interpreter from changes to the [Function] API.
#[inline]
fn first_result(function: &Function, inst: Inst) -> ValueRef {
    function.dfg.first_result(inst)
}

/// Return a list of IR values as a vector.
///
/// This helper cushions the interpreter from changes to the [Function] API.
#[inline]
fn value_refs(function: &Function, args: &ValueList) -> Vec<ValueRef> {
    args.as_slice(&function.dfg.value_lists).to_vec()
}

/// Return the (external) function name of `func_ref` in a local `function`. Note that this may
/// be truncated.
///
/// This helper cushions the interpreter from changes to the [Function] API.
#[inline]
fn function_name_of_func_ref(func_ref: FuncRef, function: &Function) -> String {
    function
        .dfg
        .ext_funcs
        .get(func_ref)
        .expect("function to exist")
        .name
        .to_string()
}

/// Helper for calculating the type of an IR value. TODO move to Frame?
#[inline]
fn type_of(value: ValueRef, function: &Function) -> Type {
    function.dfg.value_type(value)
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let mut env = Environment::default();
        env.add(func.name.to_string(), func);
        let interpreter = Interpreter::new(env);
        let result = interpreter
            .call_by_name("%test", &[])
            .unwrap()
            .unwrap_return();

        assert_eq!(result, vec![DataValue::B(true)])
    }
}
