//! A `Compilation` contains the compiled function bodies for a WebAssembly
//! module.

use crate::{FunctionAddressMap, FunctionBodyData, ModuleTranslation, Tunables, TypeTables};
use cranelift_codegen::{binemit, ir, isa, isa::unwind::UnwindInfo};
use cranelift_entity::PrimaryMap;
use cranelift_wasm::{DefinedFuncIndex, FuncIndex, WasmError, WasmFuncType};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[allow(missing_docs)]
pub type CompiledFunctions = PrimaryMap<DefinedFuncIndex, CompiledFunction>;

/// Compiled function: machine code body, jump table offsets, and unwind information.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
#[allow(missing_docs)]
pub struct CompiledFunction {
    /// The machine code for this function.
    pub body: Vec<u8>,

    /// The jump tables offsets (in the body).
    pub jt_offsets: ir::JumpTableOffsets,

    /// The unwind information.
    pub unwind_info: Option<UnwindInfo>,

    pub relocations: Vec<Relocation>,
    pub address_map: FunctionAddressMap,
    pub value_labels_ranges: cranelift_codegen::ValueLabelsRanges,
    pub stack_slots: ir::StackSlots,
    pub traps: Vec<TrapInformation>,
    pub stack_maps: Vec<StackMapInformation>,
}

/// A record of a relocation to perform.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Relocation {
    /// The relocation code.
    pub reloc: binemit::Reloc,
    /// Relocation target.
    pub reloc_target: RelocationTarget,
    /// The offset where to apply the relocation.
    pub offset: binemit::CodeOffset,
    /// The addend to add to the relocation value.
    pub addend: binemit::Addend,
}

/// Destination function. Can be either user function or some special one, like `memory.grow`.
#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
pub enum RelocationTarget {
    /// The user function index.
    UserFunc(FuncIndex),
    /// A compiler-generated libcall.
    LibCall(ir::LibCall),
    /// Jump table index.
    JumpTable(FuncIndex, ir::JumpTable),
}

/// Information about trap.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct TrapInformation {
    /// The offset of the trapping instruction in native code. It is relative to the beginning of the function.
    pub code_offset: binemit::CodeOffset,
    /// Code of the trap.
    pub trap_code: ir::TrapCode,
}

/// The offset within a function of a GC safepoint, and its associated stack
/// map.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct StackMapInformation {
    /// The offset of the GC safepoint within the function's native code. It is
    /// relative to the beginning of the function.
    pub code_offset: binemit::CodeOffset,

    /// The stack map for identifying live GC refs at the GC safepoint.
    pub stack_map: binemit::StackMap,
}

/// An error while compiling WebAssembly to machine code.
#[derive(Error, Debug)]
pub enum CompileError {
    /// A wasm translation error occured.
    #[error("WebAssembly translation error")]
    Wasm(#[from] WasmError),

    /// A compilation error occured.
    #[error("Compilation error: {0}")]
    Codegen(String),

    /// A compilation error occured.
    #[error("Debug info is not supported with this configuration")]
    DebugInfoNotSupported,
}

/// An implementation of a compiler from parsed WebAssembly module to native
/// code.
pub trait Compiler: Send + Sync {
    /// Compile a function with the given `TargetIsa`.
    fn compile_function(
        &self,
        translation: &ModuleTranslation<'_>,
        index: DefinedFuncIndex,
        data: FunctionBodyData<'_>,
        isa: &dyn isa::TargetIsa,
        tunables: &Tunables,
        types: &TypeTables,
    ) -> Result<CompiledFunction, CompileError>;

    /// Creates a trampoline which the host can use to enter wasm. The
    /// trampoline has type `VMTrampoline` and will call a function of type `ty`
    /// specified.
    fn host_to_wasm_trampoline(
        &self,
        isa: &dyn isa::TargetIsa,
        ty: &WasmFuncType,
    ) -> Result<CompiledFunction, CompileError>;

    /// Creates a trampoline suitable for a wasm module to import.
    ///
    /// The trampoline has the type specified by `ty` and will call the function
    /// `host_fn` which has type `VMTrampoline`.
    fn wasm_to_host_trampoline(
        &self,
        isa: &dyn isa::TargetIsa,
        ty: &WasmFuncType,
        host_fn: usize,
    ) -> Result<CompiledFunction, CompileError>;
}
