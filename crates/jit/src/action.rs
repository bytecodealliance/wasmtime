//! Support for performing actions with a wasm module from the outside.

use crate::compiler::Compiler;
use crate::instantiate::SetupError;
use cranelift_codegen::ir;
use std::cmp::max;
use std::{fmt, mem, ptr, slice};
use thiserror::Error;
use wasmtime_runtime::{wasmtime_call_trampoline, Export, InstanceHandle, VMInvokeArgument};

/// A runtime value.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum RuntimeValue {
    /// A runtime value with type i32.
    I32(i32),
    /// A runtime value with type i64.
    I64(i64),
    /// A runtime value with type f32.
    F32(u32),
    /// A runtime value with type f64.
    F64(u64),
    /// A runtime value with type v128
    V128([u8; 16]),
}

impl RuntimeValue {
    /// Return the type of this `RuntimeValue`.
    pub fn value_type(self) -> ir::Type {
        match self {
            Self::I32(_) => ir::types::I32,
            Self::I64(_) => ir::types::I64,
            Self::F32(_) => ir::types::F32,
            Self::F64(_) => ir::types::F64,
            Self::V128(_) => ir::types::I8X16,
        }
    }

    /// Assuming this `RuntimeValue` holds an `i32`, return that value.
    pub fn unwrap_i32(self) -> i32 {
        match self {
            Self::I32(x) => x,
            _ => panic!("unwrapping value of type {} as i32", self.value_type()),
        }
    }

    /// Assuming this `RuntimeValue` holds an `i64`, return that value.
    pub fn unwrap_i64(self) -> i64 {
        match self {
            Self::I64(x) => x,
            _ => panic!("unwrapping value of type {} as i64", self.value_type()),
        }
    }

    /// Assuming this `RuntimeValue` holds an `f32`, return that value.
    pub fn unwrap_f32(self) -> f32 {
        f32::from_bits(self.unwrap_f32_bits())
    }

    /// Assuming this `RuntimeValue` holds an `f32`, return the bits of that value as a `u32`.
    pub fn unwrap_f32_bits(self) -> u32 {
        match self {
            Self::F32(x) => x,
            _ => panic!("unwrapping value of type {} as f32", self.value_type()),
        }
    }

    /// Assuming this `RuntimeValue` holds an `f64`, return that value.
    pub fn unwrap_f64(self) -> f64 {
        f64::from_bits(self.unwrap_f64_bits())
    }

    /// Assuming this `RuntimeValue` holds an `f64`, return the bits of that value as a `u64`.
    pub fn unwrap_f64_bits(self) -> u64 {
        match self {
            Self::F64(x) => x,
            _ => panic!("unwrapping value of type {} as f64", self.value_type()),
        }
    }
}

impl fmt::Display for RuntimeValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::I32(x) => write!(f, "{}: i32", x),
            Self::I64(x) => write!(f, "{}: i64", x),
            Self::F32(x) => write!(f, "{}: f32", x),
            Self::F64(x) => write!(f, "{}: f64", x),
            Self::V128(x) => write!(f, "{:?}: v128", x.to_vec()),
        }
    }
}

/// The result of invoking a wasm function or reading a wasm global.
#[derive(Debug)]
pub enum ActionOutcome {
    /// The action returned normally. Its return values are provided.
    Returned {
        /// The return values.
        values: Vec<RuntimeValue>,
    },

    /// A trap occurred while the action was executing.
    Trapped {
        /// The trap message.
        message: String,
    },
}

/// An error detected while invoking a wasm function or reading a wasm global.
/// Note that at this level, traps are not reported errors, but are rather
/// returned through `ActionOutcome`.
#[derive(Error, Debug)]
pub enum ActionError {
    /// An internal implementation error occurred.
    #[error("Failed to setup a module")]
    Setup(#[from] SetupError),

    /// No field with the specified name was present.
    #[error("Unknown field: {0}")]
    Field(String),

    /// The field was present but was the wrong kind (eg. function, table, global, or memory).
    #[error("Kind error: {0}")]
    Kind(String),

    /// The field was present but was the wrong type (eg. i32, i64, f32, or f64).
    #[error("Type error: {0}")]
    Type(String),
}

/// Invoke a function through an `InstanceHandle` identified by an export name.
pub fn invoke(
    compiler: &mut Compiler,
    instance: &mut InstanceHandle,
    function_name: &str,
    args: &[RuntimeValue],
) -> Result<ActionOutcome, ActionError> {
    let (address, signature, callee_vmctx) = match instance.lookup(function_name) {
        Some(Export::Function {
            address,
            signature,
            vmctx,
        }) => (address, signature, vmctx),
        Some(_) => {
            return Err(ActionError::Kind(format!(
                "exported item \"{}\" is not a function",
                function_name
            )));
        }
        None => {
            return Err(ActionError::Field(format!(
                "no export named \"{}\"",
                function_name
            )));
        }
    };

    for (index, value) in args.iter().enumerate() {
        // Add one to account for the leading vmctx argument.
        assert_eq!(value.value_type(), signature.params[index + 1].value_type);
    }

    // TODO: Support values larger than v128. And pack the values into memory
    // instead of just using fixed-sized slots.
    // Subtract one becase we don't pass the vmctx argument in `values_vec`.
    let value_size = mem::size_of::<VMInvokeArgument>();
    let mut values_vec: Vec<VMInvokeArgument> =
        vec![VMInvokeArgument::new(); max(signature.params.len() - 1, signature.returns.len())];

    // Store the argument values into `values_vec`.
    for (index, arg) in args.iter().enumerate() {
        unsafe {
            let ptr = values_vec.as_mut_ptr().add(index);

            match arg {
                RuntimeValue::I32(x) => ptr::write(ptr as *mut i32, *x),
                RuntimeValue::I64(x) => ptr::write(ptr as *mut i64, *x),
                RuntimeValue::F32(x) => ptr::write(ptr as *mut u32, *x),
                RuntimeValue::F64(x) => ptr::write(ptr as *mut u64, *x),
                RuntimeValue::V128(x) => ptr::write(ptr as *mut [u8; 16], *x),
            }
        }
    }

    // Get the trampoline to call for this function.
    let exec_code_buf = compiler
        .get_trampoline(address, &signature, value_size)
        .map_err(ActionError::Setup)?;

    // Make all JIT code produced thus far executable.
    compiler.publish_compiled_code();

    // Call the trampoline.
    if let Err(message) = unsafe {
        wasmtime_call_trampoline(
            callee_vmctx,
            exec_code_buf,
            values_vec.as_mut_ptr() as *mut u8,
        )
    } {
        return Ok(ActionOutcome::Trapped { message });
    }

    // Load the return values out of `values_vec`.
    let values = signature
        .returns
        .iter()
        .enumerate()
        .map(|(index, abi_param)| unsafe {
            let ptr = values_vec.as_ptr().add(index);

            match abi_param.value_type {
                ir::types::I32 => RuntimeValue::I32(ptr::read(ptr as *const i32)),
                ir::types::I64 => RuntimeValue::I64(ptr::read(ptr as *const i64)),
                ir::types::F32 => RuntimeValue::F32(ptr::read(ptr as *const u32)),
                ir::types::F64 => RuntimeValue::F64(ptr::read(ptr as *const u64)),
                ir::types::I8X16 => RuntimeValue::V128(ptr::read(ptr as *const [u8; 16])),
                other => panic!("unsupported value type {:?}", other),
            }
        })
        .collect();

    Ok(ActionOutcome::Returned { values })
}

/// Returns a slice of the contents of allocated linear memory.
pub fn inspect_memory<'instance>(
    instance: &'instance InstanceHandle,
    memory_name: &str,
    start: usize,
    len: usize,
) -> Result<&'instance [u8], ActionError> {
    let definition = match unsafe { instance.lookup_immutable(memory_name) } {
        Some(Export::Memory {
            definition,
            memory: _memory,
            vmctx: _vmctx,
        }) => definition,
        Some(_) => {
            return Err(ActionError::Kind(format!(
                "exported item \"{}\" is not a linear memory",
                memory_name
            )));
        }
        None => {
            return Err(ActionError::Field(format!(
                "no export named \"{}\"",
                memory_name
            )));
        }
    };

    Ok(unsafe {
        let memory_def = &*definition;
        &slice::from_raw_parts(memory_def.base, memory_def.current_length)[start..start + len]
    })
}

/// Read a global in the given instance identified by an export name.
pub fn get(instance: &InstanceHandle, global_name: &str) -> Result<RuntimeValue, ActionError> {
    let (definition, global) = match unsafe { instance.lookup_immutable(global_name) } {
        Some(Export::Global {
            definition,
            vmctx: _,
            global,
        }) => (definition, global),
        Some(_) => {
            return Err(ActionError::Kind(format!(
                "exported item \"{}\" is not a global variable",
                global_name
            )));
        }
        None => {
            return Err(ActionError::Field(format!(
                "no export named \"{}\"",
                global_name
            )));
        }
    };

    unsafe {
        let global_def = &*definition;
        Ok(match global.ty {
            ir::types::I32 => RuntimeValue::I32(*global_def.as_i32()),
            ir::types::I64 => RuntimeValue::I64(*global_def.as_i64()),
            ir::types::F32 => RuntimeValue::F32(*global_def.as_f32_bits()),
            ir::types::F64 => RuntimeValue::F64(*global_def.as_f64_bits()),
            other => {
                return Err(ActionError::Type(format!(
                    "global with type {} not supported",
                    other
                )));
            }
        })
    }
}
