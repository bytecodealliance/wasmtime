//! Support for compiling with Lightbeam.

use crate::compilation::{CompileError, CompiledFunction, Compiler};
use crate::cranelift::{RelocSink, TrapSink};
use crate::func_environ::FuncEnvironment;
use crate::{FunctionBodyData, ModuleTranslation};
use cranelift_codegen::isa;
use cranelift_wasm::DefinedFuncIndex;
use lightbeam::{CodeGenSession, NullOffsetSink, Sinks};

/// A compiler that compiles a WebAssembly module with Lightbeam, directly translating the Wasm file.
pub struct Lightbeam;

impl Compiler for Lightbeam {
    fn compile_function(
        &self,
        translation: &ModuleTranslation,
        i: DefinedFuncIndex,
        function_body: &FunctionBodyData<'_>,
        isa: &dyn isa::TargetIsa,
    ) -> Result<CompiledFunction, CompileError> {
        if translation.tunables.debug_info {
            return Err(CompileError::DebugInfoNotSupported);
        }
        let func_index = translation.module.local.func_index(i);

        let env = FuncEnvironment::new(
            isa.frontend_config(),
            &translation.module.local,
            &translation.tunables,
        );
        let mut codegen_session: CodeGenSession<_> = CodeGenSession::new(
            translation.function_body_inputs.len() as u32,
            &env,
            lightbeam::microwasm::I32,
        );

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

        let code_section = codegen_session
            .into_translated_code_section()
            .map_err(|e| CompileError::Codegen(format!("Failed to generate output code: {}", e)))?;

        Ok(CompiledFunction {
            // TODO: try to remove copy here (?)
            body: code_section.buffer().to_vec(),
            traps: trap_sink.traps,
            relocations: reloc_sink.func_relocs,

            // not implemented for lightbeam currently
            unwind_info: None,
            stack_maps: Default::default(),
            stack_slots: Default::default(),
            value_labels_ranges: Default::default(),
            address_map: Default::default(),
            jt_offsets: Default::default(),
        })
    }
}
