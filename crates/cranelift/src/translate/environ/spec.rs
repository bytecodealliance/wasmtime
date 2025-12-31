//! All the runtime support necessary for the wasm to cranelift translation is formalized by the
//! traits `FunctionEnvironment` and `ModuleEnvironment`.
//!
//! There are skeleton implementations of these traits in the `dummy` module, and complete
//! implementations in [Wasmtime].
//!
//! [Wasmtime]: https://github.com/bytecodealliance/wasmtime

use cranelift_codegen::ir;
use cranelift_codegen::ir::immediates::Offset32;
use cranelift_codegen::isa::TargetFrontendConfig;
use smallvec::SmallVec;
use wasmtime_environ::{ConstExpr, ConstOp, Tunables, TypeConvert, WasmHeapType};

/// The value of a WebAssembly global variable.
#[derive(Clone, Copy)]
pub enum GlobalVariable {
    /// The global is known to be a constant value.
    Constant {
        /// The global's known value.
        value: GlobalConstValue,
    },

    /// This is a variable in memory that should be referenced through a `GlobalValue`.
    Memory {
        /// The address of the global variable storage.
        gv: ir::GlobalValue,
        /// An offset to add to the address.
        offset: Offset32,
        /// The global variable's type.
        ty: ir::Type,
    },

    /// This is a global variable that needs to be handled by the environment.
    Custom,
}

/// A global's constant value, known at compile time.
#[derive(Clone, Copy)]
pub enum GlobalConstValue {
    I32(i32),
    I64(i64),
    F32(u32),
    F64(u64),
    V128(u128),
}

impl GlobalConstValue {
    /// Attempt to evaluate the given const-expr at compile time.
    pub fn const_eval(init: &ConstExpr) -> Option<GlobalConstValue> {
        // TODO: Actually maintain an evaluation stack and handle `i32.add`,
        // `i32.sub`, etc... const ops.
        match init.ops() {
            [ConstOp::I32Const(x)] => Some(Self::I32(*x)),
            [ConstOp::I64Const(x)] => Some(Self::I64(*x)),
            [ConstOp::F32Const(x)] => Some(Self::F32(*x)),
            [ConstOp::F64Const(x)] => Some(Self::F64(*x)),
            [ConstOp::V128Const(x)] => Some(Self::V128(*x)),
            _ => None,
        }
    }
}

/// Environment affecting the translation of a WebAssembly.
pub trait TargetEnvironment: TypeConvert {
    /// Get the information needed to produce Cranelift IR for the given target.
    fn target_config(&self) -> TargetFrontendConfig;

    /// Whether to enable Spectre mitigations for heap accesses.
    fn heap_access_spectre_mitigation(&self) -> bool;

    /// Whether to add proof-carrying-code facts to verify memory accesses.
    fn proof_carrying_code(&self) -> bool;

    /// Get the Cranelift reference type to use for the given Wasm reference
    /// type.
    ///
    /// Returns a pair of the CLIF reference type to use and a boolean that
    /// describes whether the value should be included in GC stack maps or not.
    fn reference_type(&self, ty: WasmHeapType) -> (ir::Type, bool);

    /// Returns the compilation knobs that are in effect.
    fn tunables(&self) -> &Tunables;
}

/// A smallvec that holds the IR values for a struct's fields.
pub type StructFieldsVec = SmallVec<[ir::Value; 4]>;
