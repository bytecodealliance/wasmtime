//! Support for compiling with Cranelift.

use crate::compilation::{Compilation, CompileError, Relocation, RelocationTarget, Relocations};
use crate::func_environ::{
    get_func_name, get_imported_memory32_grow_name, get_imported_memory32_size_name,
    get_memory32_grow_name, get_memory32_size_name, FuncEnvironment,
};
use crate::module::Module;
use cranelift_codegen::binemit;
use cranelift_codegen::ir;
use cranelift_codegen::ir::ExternalName;
use cranelift_codegen::isa;
use cranelift_codegen::Context;
use cranelift_entity::PrimaryMap;
use cranelift_wasm::{DefinedFuncIndex, FuncIndex, FuncTranslator};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use std::vec::Vec;

/// Implementation of a relocation sink that just saves all the information for later
struct RelocSink {
    /// Relocations recorded for the function.
    func_relocs: Vec<Relocation>,
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
        let reloc_target = if *name == get_memory32_grow_name() {
            RelocationTarget::Memory32Grow
        } else if *name == get_imported_memory32_grow_name() {
            RelocationTarget::ImportedMemory32Grow
        } else if *name == get_memory32_size_name() {
            RelocationTarget::Memory32Size
        } else if *name == get_imported_memory32_size_name() {
            RelocationTarget::ImportedMemory32Size
        } else if let ExternalName::User { namespace, index } = *name {
            debug_assert!(namespace == 0);
            RelocationTarget::UserFunc(FuncIndex::from_u32(index))
        } else if let ExternalName::LibCall(libcall) = *name {
            RelocationTarget::LibCall(libcall)
        } else {
            panic!("unrecognized external name")
        };
        self.func_relocs.push(Relocation {
            reloc,
            reloc_target,
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
    /// Return a new `RelocSink` instance.
    pub fn new() -> Self {
        Self {
            func_relocs: Vec::new(),
        }
    }
}

/// Compile the module using Cranelift, producing a compilation result with
/// associated relocations.
pub fn compile_module<'data, 'module>(
    module: &'module Module,
    function_body_inputs: PrimaryMap<DefinedFuncIndex, &'data [u8]>,
    isa: &dyn isa::TargetIsa,
) -> Result<(Compilation, Relocations), CompileError> {
    let mut functions = PrimaryMap::with_capacity(function_body_inputs.len());
    let mut relocations = PrimaryMap::with_capacity(function_body_inputs.len());

    function_body_inputs
        .into_iter()
        .collect::<Vec<(DefinedFuncIndex, &&'data [u8])>>()
        .par_iter()
        .map(|(i, input)| {
            let func_index = module.func_index(*i);
            let mut context = Context::new();
            context.func.name = get_func_name(func_index);
            context.func.signature = module.signatures[module.functions[func_index]].clone();

            let mut trans = FuncTranslator::new();
            trans
                .translate(
                    input,
                    &mut context.func,
                    &mut FuncEnvironment::new(isa.frontend_config(), module),
                )
                .map_err(CompileError::Wasm)?;

            let mut code_buf: Vec<u8> = Vec::new();
            let mut reloc_sink = RelocSink::new();
            let mut trap_sink = binemit::NullTrapSink {};
            context
                .compile_and_emit(isa, &mut code_buf, &mut reloc_sink, &mut trap_sink)
                .map_err(CompileError::Codegen)?;
            Ok((code_buf, reloc_sink.func_relocs))
        })
        .collect::<Result<Vec<_>, CompileError>>()?
        .into_iter()
        .for_each(|(function, relocs)| {
            functions.push(function);
            relocations.push(relocs);
        });

    // TODO: Reorganize where we create the Vec for the resolved imports.
    Ok((Compilation::new(functions), relocations))
}
