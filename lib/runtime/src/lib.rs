//! Standalone runtime for WebAssembly using Cranelift. Provides functions to translate
//! `get_global`, `set_global`, `current_memory`, `grow_memory`, `call_indirect` that hardcode in
//! the translation the base addresses of regions of memory that will hold the globals, tables and
//! linear memories.

#![deny(missing_docs)]

extern crate cranelift_codegen;
extern crate cranelift_wasm;
extern crate target_lexicon;

mod compilation;
mod environ;
mod instance;
mod module;

pub use compilation::Compilation;
pub use environ::ModuleEnvironment;
pub use instance::Instance;
pub use module::Module;

use cranelift_codegen::binemit;
use cranelift_codegen::ir;
use cranelift_codegen::ir::ExternalName;
use cranelift_codegen::isa;
use cranelift_wasm::{FuncTranslator, FunctionIndex, GlobalIndex, MemoryIndex, TableIndex};
use environ::{FuncEnvironment, LazyContents};

/// Compute a `ir::ExternalName` for a given wasm function index.
pub fn get_func_name(func_index: FunctionIndex) -> cranelift_codegen::ir::ExternalName {
    debug_assert!(func_index as u32 as FunctionIndex == func_index);
    ir::ExternalName::user(0, func_index as u32)
}

/// An entity to export.
pub enum Export {
    /// Function export.
    Function(FunctionIndex),
    /// Table export.
    Table(TableIndex),
    /// Memory export.
    Memory(MemoryIndex),
    /// Global export.
    Global(GlobalIndex),
}

/// Implementation of a relocation sink that just saves all the information for later
pub struct RelocSink {
    /// Relocations recorded for the function.
    pub func_relocs: Vec<Relocation>,
}

impl binemit::RelocSink for RelocSink {
    fn reloc_ebb(
        &mut self,
        _offset: binemit::CodeOffset,
        _reloc: binemit::Reloc,
        _ebb_offset: binemit::CodeOffset,
    ) {
        // This should use the `offsets` field of `ir::Function`.
        panic!("ebb headers not yet implemented");
    }
    fn reloc_external(
        &mut self,
        offset: binemit::CodeOffset,
        reloc: binemit::Reloc,
        name: &ExternalName,
        addend: binemit::Addend,
    ) {
        // FIXME: Handle grow_memory/current_memory.
        let func_index = if let ExternalName::User { namespace, index } = *name {
            debug_assert!(namespace == 0);
            index
        } else {
            panic!("unrecognized external name")
        } as usize;
        self.func_relocs.push(Relocation {
            reloc,
            func_index,
            offset,
            addend,
        });
    }
    fn reloc_jt(
        &mut self,
        _offset: binemit::CodeOffset,
        _reloc: binemit::Reloc,
        _jt: ir::JumpTable,
    ) {
        panic!("jump tables not yet implemented");
    }
}

impl RelocSink {
    fn new() -> RelocSink {
        RelocSink {
            func_relocs: Vec::new(),
        }
    }
}

/// A record of a relocation to perform.
#[derive(Debug)]
pub struct Relocation {
    /// The relocation code.
    pub reloc: binemit::Reloc,
    /// The function index.
    pub func_index: FunctionIndex,
    /// The offset where to apply the relocation.
    pub offset: binemit::CodeOffset,
    /// The addend to add to the relocation value.
    pub addend: binemit::Addend,
}

/// Relocations to apply to function bodies.
pub type Relocations = Vec<Vec<Relocation>>;

/// The result of translating via `ModuleEnvironment`.
pub struct ModuleTranslation<'data, 'module> {
    /// Compilation setting flags.
    pub isa: &'module isa::TargetIsa,

    /// Module information.
    pub module: &'module Module,

    /// Pointers into the raw data buffer.
    pub lazy: LazyContents<'data>,
}

/// Convenience functions for the user to be called after execution for debug purposes.
impl<'data, 'module> ModuleTranslation<'data, 'module> {
    fn func_env(&self) -> FuncEnvironment {
        FuncEnvironment::new(self.isa, &self.module)
    }

    /// Compile the module, producing a compilation result with associated
    /// relocations.
    pub fn compile(
        &self,
        isa: &isa::TargetIsa,
    ) -> Result<(Compilation<'module>, Relocations), String> {
        let mut functions = Vec::new();
        let mut relocations = Vec::new();
        for (i, input) in self.lazy.function_body_inputs.iter().enumerate() {
            let func_index = i + self.module.imported_funcs.len();
            let mut context = cranelift_codegen::Context::new();
            context.func.name = get_func_name(func_index);
            context.func.signature =
                self.module.signatures[self.module.functions[func_index]].clone();

            let mut trans = FuncTranslator::new();
            trans
                .translate(input, &mut context.func, &mut self.func_env())
                .map_err(|e| e.to_string())?;

            let mut code_buf: Vec<u8> = Vec::new();
            let mut reloc_sink = RelocSink::new();
            let mut trap_sink = binemit::NullTrapSink {};
            context
                .compile_and_emit(isa, &mut code_buf, &mut reloc_sink, &mut trap_sink)
                .map_err(|e| e.to_string())?;
            functions.push(code_buf);
            relocations.push(reloc_sink.func_relocs);
        }
        Ok((Compilation::new(self.module, functions), relocations))
    }
}
