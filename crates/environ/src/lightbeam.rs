//! Support for compiling with Lightbeam.

use crate::cache::ModuleCacheDataTupleType;
use crate::compilation::{Compilation, CompileError, Traps};
use crate::func_environ::FuncEnvironment;
use crate::module::Module;
use crate::module_environ::FunctionBodyData;
// TODO: Put this in `compilation`
use crate::address_map::{ModuleAddressMap, ValueLabelsRanges};
use crate::cranelift::RelocSink;
use crate::CacheConfig;
use cranelift_codegen::isa;
use cranelift_entity::{PrimaryMap, SecondaryMap};
use cranelift_wasm::{DefinedFuncIndex, ModuleTranslationState};

/// A compiler that compiles a WebAssembly module with Lightbeam, directly translating the Wasm file.
pub struct Lightbeam;

impl crate::compilation::Compiler for Lightbeam {
    /// Compile the module using Lightbeam, producing a compilation result with
    /// associated relocations.
    fn compile_module<'data, 'module>(
        module: &'module Module,
        _module_translation: &ModuleTranslationState,
        function_body_inputs: PrimaryMap<DefinedFuncIndex, FunctionBodyData<'data>>,
        isa: &dyn isa::TargetIsa,
        // TODO
        generate_debug_info: bool,
        _cache_config: &CacheConfig,
    ) -> Result<ModuleCacheDataTupleType, CompileError> {
        if generate_debug_info {
            return Err(CompileError::DebugInfoNotSupported);
        }

        let env = FuncEnvironment::new(isa.frontend_config(), &module.local);
        let mut relocations = PrimaryMap::new();
        let mut codegen_session: lightbeam::CodeGenSession<_> =
            lightbeam::CodeGenSession::new(function_body_inputs.len() as u32, &env);

        for (i, function_body) in &function_body_inputs {
            let func_index = module.local.func_index(i);
            let mut reloc_sink = RelocSink::new(func_index);

            lightbeam::translate_function(
                &mut codegen_session,
                &mut reloc_sink,
                i.as_u32(),
                &wasmparser::FunctionBody::new(0, function_body.data),
            )
            .map_err(|e| CompileError::Codegen(format!("Failed to translate function: {}", e)))?;
            relocations.push(reloc_sink.func_relocs);
        }

        let code_section = codegen_session
            .into_translated_code_section()
            .map_err(|e| CompileError::Codegen(format!("Failed to generate output code: {}", e)))?;

        // TODO pass jump table offsets to Compilation::from_buffer() when they
        // are implemented in lightbeam -- using empty set of offsets for now.
        // TODO: pass an empty range for the unwind information until lightbeam emits it
        let code_section_ranges_and_jt = code_section
            .funcs()
            .into_iter()
            .map(|r| (r, SecondaryMap::new(), 0..0));

        Ok((
            Compilation::from_buffer(code_section.buffer().to_vec(), code_section_ranges_and_jt),
            relocations,
            ModuleAddressMap::new(),
            ValueLabelsRanges::new(),
            PrimaryMap::new(),
            Traps::new(),
        ))
    }
}
