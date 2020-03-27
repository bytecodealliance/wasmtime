//! Support for compiling with Lightbeam.

use crate::cache::ModuleCacheDataTupleType;
use crate::compilation::{Compilation, CompileError};
use crate::func_environ::FuncEnvironment;
use crate::module::Module;
use crate::module_environ::FunctionBodyData;
use crate::CacheConfig;
// TODO: Put this in `compilation`
use crate::address_map::{
    FunctionAddressMap, InstructionAddressMap, ModuleAddressMap, ValueLabelsRanges,
};
use crate::cranelift::{RelocSink, TrapSink};
use cranelift_codegen::{ir, isa};
use cranelift_entity::{PrimaryMap, SecondaryMap};
use cranelift_wasm::{DefinedFuncIndex, ModuleTranslationState};
use lightbeam::{CodeGenSession, OffsetSink, Sinks};
use std::{convert::TryFrom, mem};

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

        struct WasmtimeOffsetSink {
            offset: usize,
            last: Option<(ir::SourceLoc, usize)>,
            address_map: FunctionAddressMap,
        }

        impl WasmtimeOffsetSink {
            fn new(offset: usize, start_srcloc: ir::SourceLoc, end_srcloc: ir::SourceLoc) -> Self {
                WasmtimeOffsetSink {
                    offset,
                    last: None,
                    address_map: FunctionAddressMap {
                        instructions: vec![],
                        start_srcloc,
                        end_srcloc,
                        body_offset: 0,
                        body_len: 0,
                    },
                }
            }

            fn finalize(mut self, body_len: usize) -> FunctionAddressMap {
                if let Some((srcloc, code_offset)) = self.last {
                    self.address_map.instructions.push(InstructionAddressMap {
                        srcloc,
                        code_offset,
                        code_len: body_len
                            .checked_sub(code_offset)
                            .expect("Code offset exceeds size of body"),
                    });
                }

                self.address_map.body_len = body_len
                    .checked_sub(self.address_map.body_offset)
                    .expect("Code offset exceeds size of body");

                self.address_map
            }
        }

        impl OffsetSink for WasmtimeOffsetSink {
            fn offset(
                &mut self,
                offset_in_wasm_function: ir::SourceLoc,
                offset_in_compiled_function: usize,
            ) {
                if self.last.as_ref().map(|(s, _)| s) == Some(&offset_in_wasm_function) {
                    return;
                }

                let offset_in_compiled_function = offset_in_compiled_function
                    .checked_sub(self.offset)
                    .expect("Code offset exceeds size of body");

                let last = mem::replace(
                    &mut self.last,
                    Some((offset_in_wasm_function, offset_in_compiled_function)),
                );

                if let Some((srcloc, code_offset)) = last {
                    self.address_map.instructions.push(InstructionAddressMap {
                        srcloc,
                        code_offset,
                        code_len: offset_in_compiled_function
                            .checked_sub(code_offset)
                            .expect("Code offset exceeds size of body"),
                    })
                }
            }
        }

        let env = FuncEnvironment::new(isa.frontend_config(), &module.local);
        let mut relocations = PrimaryMap::with_capacity(function_body_inputs.len());
        let mut traps = PrimaryMap::with_capacity(function_body_inputs.len());
        let mut module_addresses = ModuleAddressMap::with_capacity(function_body_inputs.len());

        let mut codegen_session: CodeGenSession<_> = CodeGenSession::new(
            function_body_inputs.len() as u32,
            &env,
            lightbeam::microwasm::I32,
        );

        for (i, function_body) in &function_body_inputs {
            let func_index = module.local.func_index(i);

            let start_offset = codegen_session.offset();

            let mut reloc_sink = RelocSink::new(func_index);
            let mut trap_sink = TrapSink::new();
            let mut offset_sink = WasmtimeOffsetSink::new(
                start_offset,
                ir::SourceLoc::new(
                    u32::try_from(function_body.module_offset)
                        .expect("Size of module exceeded u32"),
                ),
                ir::SourceLoc::new(
                    u32::try_from(function_body.module_offset + function_body.data.len())
                        .expect("Size of module exceeded u32"),
                ),
            );

            lightbeam::translate_function(
                &mut codegen_session,
                Sinks {
                    relocs: &mut reloc_sink,
                    traps: &mut trap_sink,
                    offsets: &mut offset_sink,
                },
                i.as_u32(),
                wasmparser::FunctionBody::new(function_body.module_offset, function_body.data),
            )
            .map_err(|e| CompileError::Codegen(format!("Failed to translate function: {}", e)))?;

            relocations.push(reloc_sink.func_relocs);
            traps.push(trap_sink.traps);
            module_addresses.push(offset_sink.finalize(codegen_session.offset() - start_offset));
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
            Compilation::from_buffer(code_section.buffer(), code_section_ranges_and_jt),
            relocations,
            module_addresses,
            ValueLabelsRanges::new(),
            PrimaryMap::new(),
            traps,
            PrimaryMap::new(),
        ))
    }
}
