//! Support for performing actions with a wasm module from the outside.

use cranelift_codegen::ir;
use link::LinkError;
use std::fmt;
use std::string::String;
use std::vec::Vec;
use wasmtime_environ::CompileError;

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
}

impl RuntimeValue {
    /// Return the type of this `RuntimeValue`.
    pub fn value_type(self) -> ir::Type {
        match self {
            RuntimeValue::I32(_) => ir::types::I32,
            RuntimeValue::I64(_) => ir::types::I64,
            RuntimeValue::F32(_) => ir::types::F32,
            RuntimeValue::F64(_) => ir::types::F64,
        }
    }

    /// Assuming this `RuntimeValue` holds an `i32`, return that value.
    pub fn unwrap_i32(self) -> i32 {
        match self {
            RuntimeValue::I32(x) => x,
            _ => panic!("unwrapping value of type {} as i32", self.value_type()),
        }
    }

    /// Assuming this `RuntimeValue` holds an `i64`, return that value.
    pub fn unwrap_i64(self) -> i64 {
        match self {
            RuntimeValue::I64(x) => x,
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
            RuntimeValue::F32(x) => x,
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
            RuntimeValue::F64(x) => x,
            _ => panic!("unwrapping value of type {} as f64", self.value_type()),
        }
    }
}

impl fmt::Display for RuntimeValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RuntimeValue::I32(x) => write!(f, "{}: i32", x),
            RuntimeValue::I64(x) => write!(f, "{}: i64", x),
            RuntimeValue::F32(x) => write!(f, "{}: f32", x),
            RuntimeValue::F64(x) => write!(f, "{}: f64", x),
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
#[derive(Fail, Debug)]
pub enum ActionError {
    /// No field with the specified name was present.
    #[fail(display = "Unknown field: {}", _0)]
    Field(String),

    /// An index was out of bounds.
    #[fail(display = "Index out of bounds: {}", _0)]
    Index(u64),

    /// The field was present but was the wrong kind (eg. function, table, global, or memory).
    #[fail(display = "Kind error: {}", _0)]
    Kind(String),

    /// The field was present but was the wrong type (eg. i32, i64, f32, or f64).
    #[fail(display = "Type error: {}", _0)]
    Type(String),

    /// A wasm translation error occured.
    #[fail(display = "WebAssembly compilation error: {}", _0)]
    Compile(CompileError),

    /// Some runtime resource was unavailable or insufficient.
    #[fail(display = "Runtime resource error: {}", _0)]
    Resource(String),

    /// Link error.
    #[fail(display = "Link error: {}", _0)]
    Link(LinkError),

    /// Start function trapped.
    #[fail(display = "Start function trapped: {}", _0)]
    Start(String),
}
