//! Support for compiling with Cranelift.

use crate::address_map::{
    FunctionAddressMap, InstructionAddressMap, ModuleAddressMap, ValueLabelsRanges,
};
use crate::cache::{ModuleCacheData, ModuleCacheEntry};
use crate::compilation::{
    Compilation, CompileError, CompiledFunction, Relocation, RelocationTarget, Relocations,
    TrapInformation, Traps,
};
use crate::func_environ::{
    get_func_name, get_imported_memory32_grow_name, get_imported_memory32_size_name,
    get_memory32_grow_name, get_memory32_size_name, FuncEnvironment,
};
use crate::module::Module;
use crate::module_environ::FunctionBodyData;
use cranelift_codegen::binemit;
use cranelift_codegen::ir;
use cranelift_codegen::ir::ExternalName;
use cranelift_codegen::isa;
use cranelift_codegen::print_errors::pretty_error;
use cranelift_codegen::Context;
use cranelift_entity::PrimaryMap;
use cranelift_wasm::{DefinedFuncIndex, FuncIndex, FuncTranslator, ModuleTranslationState};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

/// Implementation of a relocation sink that just saves all the information for later
pub struct RelocSink {
    /// Current function index.
    func_index: FuncIndex,

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
        let reloc_target = if *name == get_memory32_grow_name() {
            RelocationTarget::Memory32Grow
        } else if *name == get_imported_memory32_grow_name() {
            RelocationTarget::ImportedMemory32Grow
        } else if *name == get_memory32_size_name() {
            RelocationTarget::Memory32Size
        } else if *name == get_imported_memory32_size_name() {
            RelocationTarget::ImportedMemory32Size
        } else if let ExternalName::User { namespace, index } = *name {
            debug_assert_eq!(namespace, 0);
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

    fn reloc_constant(
        &mut self,
        _code_offset: binemit::CodeOffset,
        _reloc: binemit::Reloc,
        _constant_offset: ir::ConstantOffset,
    ) {
        // Do nothing for now: cranelift emits constant data after the function code and also emits
        // function code with correct relative offsets to the constant data.
    }

    fn reloc_jt(&mut self, offset: binemit::CodeOffset, reloc: binemit::Reloc, jt: ir::JumpTable) {
        self.func_relocs.push(Relocation {
            reloc,
            reloc_target: RelocationTarget::JumpTable(self.func_index, jt),
            offset,
            addend: 0,
        });
    }
}

impl RelocSink {
    /// Return a new `RelocSink` instance.
    pub fn new(func_index: FuncIndex) -> Self {
        Self {
            func_index,
            func_relocs: Vec::new(),
        }
    }
}

struct TrapSink {
    pub traps: Vec<TrapInformation>,
}

impl TrapSink {
    fn new() -> Self {
        Self { traps: Vec::new() }
    }
}

impl binemit::TrapSink for TrapSink {
    fn trap(
        &mut self,
        code_offset: binemit::CodeOffset,
        source_loc: ir::SourceLoc,
        trap_code: ir::TrapCode,
    ) {
        self.traps.push(TrapInformation {
            code_offset,
            source_loc,
            trap_code,
        });
    }
}

fn get_function_address_map<'data>(
    context: &Context,
    data: &FunctionBodyData<'data>,
    body_len: usize,
    isa: &dyn isa::TargetIsa,
) -> FunctionAddressMap {
    let mut instructions = Vec::new();

    let func = &context.func;
    let mut ebbs = func.layout.ebbs().collect::<Vec<_>>();
    ebbs.sort_by_key(|ebb| func.offsets[*ebb]); // Ensure inst offsets always increase

    let encinfo = isa.encoding_info();
    for ebb in ebbs {
        for (offset, inst, size) in func.inst_offsets(ebb, &encinfo) {
            let srcloc = func.srclocs[inst];
            instructions.push(InstructionAddressMap {
                srcloc,
                code_offset: offset as usize,
                code_len: size as usize,
            });
        }
    }

    // Generate artificial srcloc for function start/end to identify boundary
    // within module. Similar to FuncTranslator::cur_srcloc(): it will wrap around
    // if byte code is larger than 4 GB.
    let start_srcloc = ir::SourceLoc::new(data.module_offset as u32);
    let end_srcloc = ir::SourceLoc::new((data.module_offset + data.data.len()) as u32);

    FunctionAddressMap {
        instructions,
        start_srcloc,
        end_srcloc,
        body_offset: 0,
        body_len,
    }
}

/// A compiler that compiles a WebAssembly module with Cranelift, translating the Wasm to Cranelift IR,
/// optimizing it and then translating to assembly.
pub struct Cranelift;

impl crate::compilation::Compiler for Cranelift {
    /// Compile the module using Cranelift, producing a compilation result with
    /// associated relocations.
    fn compile_module<'data, 'module>(
        module: &'module Module,
        module_translation: &ModuleTranslationState,
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
    > {
        let cache_entry = ModuleCacheEntry::new(
            module,
            &function_body_inputs,
            isa,
            "cranelift",
            generate_debug_info,
        );

        let data = match cache_entry.get_data() {
            Some(data) => data,
            None => {
                let mut functions = PrimaryMap::with_capacity(function_body_inputs.len());
                let mut relocations = PrimaryMap::with_capacity(function_body_inputs.len());
                let mut address_transforms = PrimaryMap::with_capacity(function_body_inputs.len());
                let mut value_ranges = PrimaryMap::with_capacity(function_body_inputs.len());
                let mut stack_slots = PrimaryMap::with_capacity(function_body_inputs.len());
                let mut traps = PrimaryMap::with_capacity(function_body_inputs.len());

                function_body_inputs
                    .into_iter()
                    .collect::<Vec<(DefinedFuncIndex, &FunctionBodyData<'data>)>>()
                    .par_iter()
                    .map_init(
                        || FuncTranslator::new(),
                        |func_translator, (i, input)| {
                            let func_index = module.func_index(*i);
                            let mut context = Context::new();
                            context.func.name = get_func_name(func_index);
                            context.func.signature =
                                module.signatures[module.functions[func_index]].clone();
                            if generate_debug_info {
                                context.func.collect_debug_info();
                            }

                            func_translator.translate(
                                module_translation,
                                input.data,
                                input.module_offset,
                                &mut context.func,
                                &mut FuncEnvironment::new(isa.frontend_config(), module),
                            )?;

                            let mut code_buf: Vec<u8> = Vec::new();
                            let mut unwind_info = Vec::new();
                            let mut reloc_sink = RelocSink::new(func_index);
                            let mut trap_sink = TrapSink::new();
                            let mut stackmap_sink = binemit::NullStackmapSink {};
                            context
                                .compile_and_emit(
                                    isa,
                                    &mut code_buf,
                                    &mut reloc_sink,
                                    &mut trap_sink,
                                    &mut stackmap_sink,
                                )
                                .map_err(|error| {
                                    CompileError::Codegen(pretty_error(
                                        &context.func,
                                        Some(isa),
                                        error,
                                    ))
                                })?;

                            context.emit_unwind_info(isa, &mut unwind_info);

                            let address_transform = if generate_debug_info {
                                let body_len = code_buf.len();
                                Some(get_function_address_map(&context, input, body_len, isa))
                            } else {
                                None
                            };

                            let ranges = if generate_debug_info {
                                let ranges =
                                    context.build_value_labels_ranges(isa).map_err(|error| {
                                        CompileError::Codegen(pretty_error(
                                            &context.func,
                                            Some(isa),
                                            error,
                                        ))
                                    })?;
                                Some(ranges)
                            } else {
                                None
                            };

                            Ok((
                                code_buf,
                                context.func.jt_offsets,
                                reloc_sink.func_relocs,
                                address_transform,
                                ranges,
                                context.func.stack_slots,
                                trap_sink.traps,
                                unwind_info,
                            ))
                        },
                    )
                    .collect::<Result<Vec<_>, CompileError>>()?
                    .into_iter()
                    .for_each(
                        |(
                            function,
                            func_jt_offsets,
                            relocs,
                            address_transform,
                            ranges,
                            sss,
                            function_traps,
                            unwind_info,
                        )| {
                            functions.push(CompiledFunction {
                                body: function,
                                jt_offsets: func_jt_offsets,
                                unwind_info,
                            });
                            relocations.push(relocs);
                            if let Some(address_transform) = address_transform {
                                address_transforms.push(address_transform);
                            }
                            value_ranges.push(ranges.unwrap_or_default());
                            stack_slots.push(sss);
                            traps.push(function_traps);
                        },
                    );

                // TODO: Reorganize where we create the Vec for the resolved imports.

                let data = ModuleCacheData::from_tuple((
                    Compilation::new(functions),
                    relocations,
                    address_transforms,
                    value_ranges,
                    stack_slots,
                    traps,
                ));
                cache_entry.update_data(&data);
                data
            }
        };

        Ok(data.to_tuple())
    }
}
