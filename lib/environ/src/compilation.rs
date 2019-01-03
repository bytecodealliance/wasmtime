//! A `Compilation` contains the compiled function bodies for a WebAssembly
//! module.

use cranelift_codegen::binemit;
use cranelift_codegen::ir;
use cranelift_codegen::CodegenError;
use cranelift_entity::PrimaryMap;
use cranelift_wasm::{DefinedFuncIndex, FuncIndex, WasmError};
use std::vec::Vec;

/// The result of compiling a WebAssembly module's functions.
#[derive(Debug)]
pub struct Compilation {
    /// Compiled machine code for the function bodies.
    pub functions: PrimaryMap<DefinedFuncIndex, Vec<u8>>,
}

impl Compilation {
    /// Allocates the compilation result with the given function bodies.
    pub fn new(functions: PrimaryMap<DefinedFuncIndex, Vec<u8>>) -> Self {
        Self { functions }
    }
}

/// A record of a relocation to perform.
#[derive(Debug, Clone)]
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
#[derive(Debug, Copy, Clone)]
pub enum RelocationTarget {
    /// The user function index.
    UserFunc(FuncIndex),
    /// A compiler-generated libcall.
    LibCall(ir::LibCall),
    /// Function for growing a locally-defined 32-bit memory by the specified amount of pages.
    Memory32Grow,
    /// Function for growing an imported 32-bit memory by the specified amount of pages.
    ImportedMemory32Grow,
    /// Function for query current size of a locally-defined 32-bit linear memory.
    Memory32Size,
    /// Function for query current size of an imported 32-bit linear memory.
    ImportedMemory32Size,
}

/// Relocations to apply to function bodies.
pub type Relocations = PrimaryMap<DefinedFuncIndex, Vec<Relocation>>;

/// An error while compiling WebAssembly to machine code.
#[derive(Fail, Debug)]
pub enum CompileError {
    /// A wasm translation error occured.
    #[fail(display = "WebAssembly translation error: {}", _0)]
    Wasm(WasmError),

    /// A compilation error occured.
    #[fail(display = "Compilation error: {}", _0)]
    Codegen(CodegenError),
}
