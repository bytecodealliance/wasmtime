//! A `Compilation` contains the compiled function bodies for a WebAssembly
//! module.

use crate::address_map::{ModuleAddressMap, ValueLabelsRanges};
use crate::ModuleTranslation;
use cranelift_codegen::{binemit, ir, isa, isa::unwind::UnwindInfo};
use cranelift_entity::PrimaryMap;
use cranelift_wasm::{DefinedFuncIndex, FuncIndex, WasmError};
use serde::{Deserialize, Serialize};
use std::ops::Range;
use thiserror::Error;

/// Compiled function: machine code body, jump table offsets, and unwind information.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CompiledFunction {
    /// The function body.
    pub body: Vec<u8>,

    /// The jump tables offsets (in the body).
    pub jt_offsets: ir::JumpTableOffsets,

    /// The unwind information.
    pub unwind_info: Option<UnwindInfo>,
}

type Functions = PrimaryMap<DefinedFuncIndex, CompiledFunction>;

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
                .map(|(body_range, jt_offsets)| CompiledFunction {
                    body: buffer[body_range].to_vec(),
                    jt_offsets,
                    unwind_info: None, // not implemented for lightbeam currently
                })
                .collect(),
        )
    }

    /// Gets the bytes of a single function
    pub fn get(&self, func: DefinedFuncIndex) -> &CompiledFunction {
        &self.functions[func]
    }

    /// Gets the number of functions defined.
    pub fn len(&self) -> usize {
        self.functions.len()
    }

    /// Returns whether there are no functions defined.
    pub fn is_empty(&self) -> bool {
        self.functions.is_empty()
    }

    /// Returns unwind info for all defined functions.
    pub fn unwind_info(&self) -> PrimaryMap<DefinedFuncIndex, &Option<UnwindInfo>> {
        self.functions
            .iter()
            .map(|(_, func)| &func.unwind_info)
            .collect::<PrimaryMap<DefinedFuncIndex, _>>()
    }

    /// Gets functions jump table offsets.
    pub fn get_jt_offsets(&self) -> PrimaryMap<DefinedFuncIndex, ir::JumpTableOffsets> {
        self.functions
            .iter()
            .map(|(_, func)| func.jt_offsets.clone())
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
    type Item = &'a CompiledFunction;

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
    /// Jump table index.
    JumpTable(FuncIndex, ir::JumpTable),
}

/// Relocations to apply to function bodies.
pub type Relocations = PrimaryMap<DefinedFuncIndex, Vec<Relocation>>;

/// Information about trap.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
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

/// The offset within a function of a GC safepoint, and its associated stack
/// map.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct StackMapInformation {
    /// The offset of the GC safepoint within the function's native code. It is
    /// relative to the beginning of the function.
    pub code_offset: binemit::CodeOffset,

    /// The stack map for identifying live GC refs at the GC safepoint.
    pub stack_map: binemit::Stackmap,
}

/// Information about GC safepoints and their associated stack maps within each
/// function.
pub type StackMaps = PrimaryMap<DefinedFuncIndex, Vec<StackMapInformation>>;

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

/// A type alias for the result of `Compiler::compile_module`
pub type CompileResult = (
    Compilation,
    Relocations,
    ModuleAddressMap,
    ValueLabelsRanges,
    PrimaryMap<DefinedFuncIndex, ir::StackSlots>,
    Traps,
    StackMaps,
);

/// An implementation of a compiler from parsed WebAssembly module to native code.
pub trait Compiler {
    /// Compile a parsed module with the given `TargetIsa`.
    fn compile_module(
        translation: &ModuleTranslation,
        isa: &dyn isa::TargetIsa,
    ) -> Result<CompileResult, CompileError>;
}
