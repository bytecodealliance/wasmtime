//! A `Compilation` contains the compiled function bodies for a WebAssembly
//! module.

use crate::address_map::{ModuleAddressMap, ValueLabelsRanges};
use crate::module;
use crate::module_environ::FunctionBodyData;
use alloc::vec::Vec;
use cranelift_codegen::{binemit, ir, isa, CodegenError};
use cranelift_entity::PrimaryMap;
use cranelift_wasm::{DefinedFuncIndex, FuncIndex, WasmError};
use serde::{Deserialize, Serialize};
use std::ops::Range;

/// Compiled machine code: body and jump table offsets.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CodeAndJTOffsets {
    /// The function body.
    pub body: Vec<u8>,

    /// The jump tables offsets (in the body).
    pub jt_offsets: ir::JumpTableOffsets,
}

type Functions = PrimaryMap<DefinedFuncIndex, CodeAndJTOffsets>;

/// The result of compiling a WebAssembly module's functions.
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
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
    pub fn from_buffer(
        buffer: Vec<u8>,
        functions: impl IntoIterator<Item = (Range<usize>, ir::JumpTableOffsets)>,
    ) -> Self {
        Self::new(
            functions
                .into_iter()
                .map(|(range, jt_offsets)| CodeAndJTOffsets {
                    body: buffer[range].to_vec(),
                    jt_offsets,
                })
                .collect(),
        )
    }

    /// Gets the bytes of a single function
    pub fn get(&self, func: DefinedFuncIndex) -> &CodeAndJTOffsets {
        &self.functions[func]
    }

    /// Gets the number of functions defined.
    pub fn len(&self) -> usize {
        self.functions.len()
    }

    /// Gets functions jump table offsets.
    pub fn get_jt_offsets(&self) -> PrimaryMap<DefinedFuncIndex, ir::JumpTableOffsets> {
        self.functions
            .iter()
            .map(|(_, code_and_jt)| code_and_jt.jt_offsets.clone())
            .collect::<PrimaryMap<DefinedFuncIndex, _>>()
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
    type Item = &'a CodeAndJTOffsets;

    fn next(&mut self) -> Option<Self::Item> {
        self.iterator.next().map(|(_, b)| b)
    }
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
    /// Function for growing a locally-defined 32-bit memory by the specified amount of pages.
    Memory32Grow,
    /// Function for growing an imported 32-bit memory by the specified amount of pages.
    ImportedMemory32Grow,
    /// Function for query current size of a locally-defined 32-bit linear memory.
    Memory32Size,
    /// Function for query current size of an imported 32-bit linear memory.
    ImportedMemory32Size,
    /// Jump table index.
    JumpTable(FuncIndex, ir::JumpTable),
}

/// Relocations to apply to function bodies.
pub type Relocations = PrimaryMap<DefinedFuncIndex, Vec<Relocation>>;

/// Information about trap.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct TrapInformation {
    /// The offset of the trapping instruction in native code. It is relative to the beginning of the function.
    pub code_offset: binemit::CodeOffset,
    /// Location of trapping instruction in WebAssembly binary module.
    pub source_loc: ir::SourceLoc,
    /// Code of the trap.
    pub trap_code: ir::TrapCode,
}

/// Information about traps associated with the functions where the traps are placed.
pub type Traps = PrimaryMap<DefinedFuncIndex, Vec<TrapInformation>>;

/// An error while compiling WebAssembly to machine code.
#[derive(Fail, Debug)]
pub enum CompileError {
    /// A wasm translation error occured.
    #[fail(display = "WebAssembly translation error: {}", _0)]
    Wasm(WasmError),

    /// A compilation error occured.
    #[fail(display = "Compilation error: {}", _0)]
    Codegen(CodegenError),

    /// A compilation error occured.
    #[fail(display = "Debug info is not supported with this configuration")]
    DebugInfoNotSupported,
}

/// An implementation of a compiler from parsed WebAssembly module to native code.
pub trait Compiler {
    /// Compile a parsed module with the given `TargetIsa`.
    fn compile_module<'data, 'module>(
        module: &'module module::Module,
        function_body_inputs: PrimaryMap<DefinedFuncIndex, FunctionBodyData<'data>>,
        isa: &dyn isa::TargetIsa,
        generate_debug_info: bool,
    ) -> Result<
        (
            Compilation,
            Relocations,
            ModuleAddressMap,
            ValueLabelsRanges,
            PrimaryMap<DefinedFuncIndex, ir::StackSlots>,
            Traps,
        ),
        CompileError,
    >;
}
