//! Support for compiling with Lightbeam.

use crate::compilation::{Compilation, CompileError, CompileResult, Compiler};
use crate::func_environ::FuncEnvironment;
use crate::CacheConfig;
use crate::ModuleTranslation;
// TODO: Put this in `compilation`
use crate::address_map::{ModuleAddressMap, ValueLabelsRanges};
use crate::cranelift::{RelocSink, TrapSink};
use cranelift_codegen::isa;
use cranelift_entity::{PrimaryMap, SecondaryMap};
use lightbeam::{CodeGenSession, NullOffsetSink, Sinks};

/// A compiler that compiles a WebAssembly module with Lightbeam, directly translating the Wasm file.
pub struct Lightbeam;

impl Compiler for Lightbeam {
    /// Compile the module using Lightbeam, producing a compilation result with
    /// associated relocations.
    fn compile_module(
        translation: &ModuleTranslation,
        isa: &dyn isa::TargetIsa,
    ) -> Result<CompileResult, CompileError> {
        if translation.tunables.debug_info {
            return Err(CompileError::DebugInfoNotSupported);
        }

        let env = FuncEnvironment::new(
            isa.frontend_config(),
            &translation.module.local,
            &translation.tunables,
        );
        let mut relocations = PrimaryMap::with_capacity(translation.function_body_inputs.len());
        let mut traps = PrimaryMap::with_capacity(translation.function_body_inputs.len());
        let stack_maps = PrimaryMap::with_capacity(translation.function_body_inputs.len());

        let mut codegen_session: CodeGenSession<_> = CodeGenSession::new(
            translation.function_body_inputs.len() as u32,
            &env,
            lightbeam::microwasm::I32,
        );

        for (i, function_body) in &translation.function_body_inputs {
            let func_index = translation.module.local.func_index(i);

            let mut reloc_sink = RelocSink::new(func_index);
            let mut trap_sink = TrapSink::new();
            lightbeam::translate_function(
                &mut codegen_session,
                Sinks {
                    relocs: &mut reloc_sink,
                    traps: &mut trap_sink,
                    offsets: &mut NullOffsetSink,
                },
                i.as_u32(),
                wasmparser::FunctionBody::new(0, function_body.data),
            )
            .map_err(|e| CompileError::Codegen(format!("Failed to translate function: {}", e)))?;

            relocations.push(reloc_sink.func_relocs);
            traps.push(trap_sink.traps);
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
            .map(|r| (r, SecondaryMap::new()));

        Ok((
            Compilation::from_buffer(code_section.buffer().to_vec(), code_section_ranges_and_jt),
            relocations,
            ModuleAddressMap::new(),
            ValueLabelsRanges::new(),
            PrimaryMap::new(),
            traps,
            stack_maps,
        ))
    }
}
