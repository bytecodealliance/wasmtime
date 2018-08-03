//! A `Compilation` contains the compiled function bodies for a WebAssembly
//! module.

use cranelift_codegen::binemit;
use cranelift_codegen::ir;
use cranelift_codegen::ir::ExternalName;
use cranelift_codegen::isa;
use cranelift_codegen::Context;
use cranelift_wasm::{FuncTranslator, FunctionIndex};
use environ::{get_func_name, ModuleTranslation};
use module::Module;

/// An Instance of a WebAssemby module.
#[derive(Debug)]
pub struct Compilation<'module> {
    /// The module this `Compilation` is compiled from.
    pub module: &'module Module,

    /// Compiled machine code for the function bodies.
    pub functions: Vec<Vec<u8>>,
}

impl<'module> Compilation<'module> {
    /// Allocates the runtime data structures with the given flags.
    pub fn new(module: &'module Module, functions: Vec<Vec<u8>>) -> Self {
        Self { module, functions }
    }
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

/// Compile the module, producing a compilation result with associated
/// relocations.
pub fn compile_module<'data, 'module>(
    translation: &ModuleTranslation<'data, 'module>,
    isa: &isa::TargetIsa,
) -> Result<(Compilation<'module>, Relocations), String> {
    let mut functions = Vec::new();
    let mut relocations = Vec::new();
    for (i, input) in translation.lazy.function_body_inputs.iter().enumerate() {
        let func_index = i + translation.module.imported_funcs.len();
        let mut context = Context::new();
        context.func.name = get_func_name(func_index);
        context.func.signature =
            translation.module.signatures[translation.module.functions[func_index]].clone();

        let mut trans = FuncTranslator::new();
        trans
            .translate(input, &mut context.func, &mut translation.func_env())
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
    Ok((Compilation::new(translation.module, functions), relocations))
}
