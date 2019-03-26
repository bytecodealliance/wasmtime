//! A `Compilation` contains the compiled function bodies for a WebAssembly
//! module.

use crate::module;
use crate::module_environ::FunctionBodyData;
use cranelift_codegen::{binemit, ir, isa, CodegenError};
use cranelift_entity::PrimaryMap;
use cranelift_wasm::{DefinedFuncIndex, FuncIndex, WasmError};
use std::ops::Range;
use std::vec::Vec;

type Functions = PrimaryMap<DefinedFuncIndex, Vec<u8>>;

/// The result of compiling a WebAssembly module's functions.
#[derive(Debug)]
pub struct Compilation {
    /// Compiled machine code for the function bodies.
    functions: Functions,
}

impl Compilation {
    /// Creates a compilation artifact from a contiguous function buffer and a set of ranges
    pub fn new(functions: Functions) -> Self {
        Self { functions }
    }

    /// Allocates the compilation result with the given function bodies.
    pub fn from_buffer(buffer: Vec<u8>, functions: impl IntoIterator<Item = Range<usize>>) -> Self {
        Self::new(
            functions
                .into_iter()
                .map(|range| buffer[range].to_vec())
                .collect(),
        )
    }

    /// Gets the bytes of a single function
    pub fn get(&self, func: DefinedFuncIndex) -> &[u8] {
        &self.functions[func]
    }

    /// Gets the number of functions defined.
    pub fn len(&self) -> usize {
        self.functions.len()
    }
}

impl<'a> IntoIterator for &'a Compilation {
    type IntoIter = Iter<'a>;
    type Item = <Self::IntoIter as Iterator>::Item;

    fn into_iter(self) -> Self::IntoIter {
        Iter {
            iterator: self.functions.iter(),
        }
    }
}

pub struct Iter<'a> {
    iterator: <&'a Functions as IntoIterator>::IntoIter,
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        self.iterator.next().map(|(_, b)| &b[..])
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

/// Single address point transform.
#[derive(Debug)]
pub struct InstructionAddressTransform {
    /// Original source location.
    pub srcloc: ir::SourceLoc,

    /// Generated instructions offset.
    pub code_offset: usize,

    /// Generated instructions length.
    pub code_len: usize,
}

/// Function and its instructions transforms.
#[derive(Debug)]
pub struct FunctionAddressTransform {
    /// Instructions transforms
    pub locations: Vec<InstructionAddressTransform>,

    /// Generated function body offset if applicable, otherwise 0.
    pub body_offset: usize,

    /// Generated function body length.
    pub body_len: usize,
}

/// Function AddressTransforms collection.
pub type AddressTransforms = PrimaryMap<DefinedFuncIndex, FunctionAddressTransform>;

/// An implementation of a compiler from parsed WebAssembly module to native code.
pub trait Compiler {
    /// Compile a parsed module with the given `TargetIsa`.
    fn compile_module<'data, 'module>(
        module: &'module module::Module,
        function_body_inputs: PrimaryMap<DefinedFuncIndex, FunctionBodyData<'data>>,
        isa: &dyn isa::TargetIsa,
        generate_debug_info: bool,
    ) -> Result<(Compilation, Relocations, AddressTransforms), CompileError>;
}
