//! Support for performing actions with a wasm module from the outside.

use std::fmt;
use wasmtime_environ::ir;

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
