//! Support for performing actions with a wasm module from the outside.

use cranelift_codegen::ir;
use std::string::String;
use std::vec::Vec;

/// A runtime value.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Value {
    /// A runtime value with type i32.
    I32(i32),
    /// A runtime value with type i64.
    I64(i64),
    /// A runtime value with type f32.
    F32(u32),
    /// A runtime value with type f64.
    F64(u64),
}

impl Value {
    /// Return the type of this `Value`.
    pub fn value_type(self) -> ir::Type {
        match self {
            Value::I32(_) => ir::types::I32,
            Value::I64(_) => ir::types::I64,
            Value::F32(_) => ir::types::F32,
            Value::F64(_) => ir::types::F64,
        }
    }

    /// Assuming this `Value` holds an `i32`, return that value.
    pub fn unwrap_i32(self) -> i32 {
        match self {
            Value::I32(x) => x,
            _ => panic!("unwrapping value of type {} as i32", self.value_type()),
        }
    }

    /// Assuming this `Value` holds an `i64`, return that value.
    pub fn unwrap_i64(self) -> i64 {
        match self {
            Value::I64(x) => x,
            _ => panic!("unwrapping value of type {} as i64", self.value_type()),
        }
    }

    /// Assuming this `Value` holds an `f32`, return that value.
    pub fn unwrap_f32(self) -> u32 {
        match self {
            Value::F32(x) => x,
            _ => panic!("unwrapping value of type {} as f32", self.value_type()),
        }
    }

    /// Assuming this `Value` holds an `f64`, return that value.
    pub fn unwrap_f64(self) -> u64 {
        match self {
            Value::F64(x) => x,
            _ => panic!("unwrapping value of type {} as f64", self.value_type()),
        }
    }
}

/// The result of invoking a wasm function or reading a wasm global.
#[derive(Debug)]
pub enum ActionOutcome {
    /// The action returned normally. Its return values are provided.
    Returned {
        /// The return values.
        values: Vec<Value>,
    },
    /// A trap occurred while the action was executing.
    Trapped {
        /// The trap message.
        message: String,
    },
}
