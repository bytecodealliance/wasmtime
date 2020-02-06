//! A `Compilation` contains the compiled function bodies for a WebAssembly
//! module.

use crate::cache::ModuleCacheDataTupleType;
use crate::module;
use crate::module_environ::FunctionBodyData;
use crate::CacheConfig;
use cranelift_codegen::{binemit, ir, isa, Context};
use cranelift_entity::PrimaryMap;
use cranelift_wasm::{DefinedFuncIndex, FuncIndex, ModuleTranslationState, WasmError};
use serde::{Deserialize, Serialize};
use std::ops::Range;
use thiserror::Error;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FDERelocEntry(pub i64, pub usize, pub u8);

/// Relocation entry for unwind info.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CompiledFunctionUnwindInfoReloc {
    /// Entry offest in the code block.
    pub offset: u32,
    /// Entry addend relative to the code block.
    pub addend: u32,
}

/// Compiled function unwind information.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum CompiledFunctionUnwindInfo {
    /// No info.
    None,
    /// Windows UNWIND_INFO.
    Windows(Vec<u8>),
    /// Frame layout info.
    FrameLayout(Vec<u8>, usize, Vec<FDERelocEntry>),
}

impl CompiledFunctionUnwindInfo {
    /// Constructs unwind info object.
    pub fn new(isa: &dyn isa::TargetIsa, context: &Context) -> Self {
        use cranelift_codegen::binemit::{
            FrameUnwindKind, FrameUnwindOffset, FrameUnwindSink, Reloc,
        };
        use cranelift_codegen::isa::CallConv;

        struct Sink(Vec<u8>, usize, Vec<FDERelocEntry>);
        impl FrameUnwindSink for Sink {
            fn len(&self) -> FrameUnwindOffset {
                self.0.len()
            }
            fn bytes(&mut self, b: &[u8]) {
                self.0.extend_from_slice(b);
            }
            fn reserve(&mut self, len: usize) {
                self.0.reserve(len)
            }
            fn reloc(&mut self, r: Reloc, off: FrameUnwindOffset) {
                self.2.push(FDERelocEntry(
                    0,
                    off,
                    match r {
                        Reloc::Abs4 => 4,
                        Reloc::Abs8 => 8,
                        _ => {
                            panic!("unexpected reloc type");
                        }
                    },
                ))
            }
            fn set_entry_offset(&mut self, off: FrameUnwindOffset) {
                self.1 = off;
            }
        }

        let kind = match context.func.signature.call_conv {
            CallConv::SystemV | CallConv::Fast | CallConv::Cold => FrameUnwindKind::Libunwind,
            CallConv::WindowsFastcall => FrameUnwindKind::Fastcall,
            _ => {
                return CompiledFunctionUnwindInfo::None;
            }
        };

        let mut sink = Sink(Vec::new(), 0, Vec::new());
        context.emit_unwind_info(isa, kind, &mut sink);

        let Sink(data, offset, relocs) = sink;
        if data.is_empty() {
            return CompiledFunctionUnwindInfo::None;
        }

        match kind {
            FrameUnwindKind::Fastcall => CompiledFunctionUnwindInfo::Windows(data),
            FrameUnwindKind::Libunwind => {
                CompiledFunctionUnwindInfo::FrameLayout(data, offset, relocs)
            }
        }
    }

    /// Retuns true is no unwind info data.
    pub fn is_empty(&self) -> bool {
        match self {
            CompiledFunctionUnwindInfo::None => true,
            CompiledFunctionUnwindInfo::Windows(d) => d.is_empty(),
            CompiledFunctionUnwindInfo::FrameLayout(c, _, _) => c.is_empty(),
        }
    }

    /// Returns size of serilized unwind info.
    pub fn len(&self) -> usize {
        match self {
            CompiledFunctionUnwindInfo::None => 0,
            CompiledFunctionUnwindInfo::Windows(d) => d.len(),
            CompiledFunctionUnwindInfo::FrameLayout(c, _, _) => c.len(),
        }
    }

    /// Serializes data into byte array.
    pub fn serialize(&self, dest: &mut [u8], relocs: &mut Vec<CompiledFunctionUnwindInfoReloc>) {
        match self {
            CompiledFunctionUnwindInfo::None => (),
            CompiledFunctionUnwindInfo::Windows(d) => {
                dest.copy_from_slice(d);
            }
            CompiledFunctionUnwindInfo::FrameLayout(code, _fde_offset, r) => {
                dest.copy_from_slice(code);
                r.iter().for_each(move |r| {
                    assert_eq!(r.2, 8);
                    relocs.push(CompiledFunctionUnwindInfoReloc {
                        offset: r.1 as u32,
                        addend: r.0 as u32,
                    })
                });
            }
        }
    }
}

/// Compiled function: machine code body, jump table offsets, and unwind information.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CompiledFunction {
    /// The function body.
    pub body: Vec<u8>,

    /// The jump tables offsets (in the body).
    pub jt_offsets: ir::JumpTableOffsets,

    /// The unwind information.
    pub unwind_info: CompiledFunctionUnwindInfo,
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
        functions: impl IntoIterator<Item = (Range<usize>, ir::JumpTableOffsets, Range<usize>)>,
    ) -> Self {
        Self::new(
            functions
                .into_iter()
                .map(|(body_range, jt_offsets, unwind_range)| CompiledFunction {
                    body: buffer[body_range].to_vec(),
                    jt_offsets,
                    unwind_info: CompiledFunctionUnwindInfo::Windows(buffer[unwind_range].to_vec()),
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

/// An implementation of a compiler from parsed WebAssembly module to native code.
pub trait Compiler {
    /// Compile a parsed module with the given `TargetIsa`.
    fn compile_module<'data, 'module>(
        module: &'module module::Module,
        module_translation: &ModuleTranslationState,
        function_body_inputs: PrimaryMap<DefinedFuncIndex, FunctionBodyData<'data>>,
        isa: &dyn isa::TargetIsa,
        generate_debug_info: bool,
        cache_config: &CacheConfig,
    ) -> Result<ModuleCacheDataTupleType, CompileError>;
}
