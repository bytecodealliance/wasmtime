use crate::builder::LinkOptions;
use crate::debug::ModuleMemoryOffset;
use crate::func_environ::{get_func_name, FuncEnvironment};
use crate::obj::ObjectBuilder;
use crate::{
    blank_sig, func_signature, indirect_signature, value_type, wasmtime_call_conv,
    CompiledFunction, FunctionAddressMap, Relocation, RelocationTarget,
};
use anyhow::{Context as _, Result};
use cranelift_codegen::ir::{self, ExternalName, InstBuilder, MemFlags};
use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::print_errors::pretty_error;
use cranelift_codegen::settings;
use cranelift_codegen::MachSrcLoc;
use cranelift_codegen::{binemit, Context};
use cranelift_entity::{EntityRef, PrimaryMap};
use cranelift_frontend::FunctionBuilder;
use cranelift_wasm::{
    DefinedFuncIndex, DefinedMemoryIndex, FuncIndex, FuncTranslator, MemoryIndex, SignatureIndex,
    WasmFuncType,
};
use object::write::Object;
use std::any::Any;
use std::cmp;
use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::mem;
use std::sync::Mutex;
use wasmtime_environ::{
    AddressMapSection, CompileError, FilePos, FlagValue, FunctionBodyData, FunctionInfo,
    InstructionAddressMap, Module, ModuleTranslation, StackMapInformation, Trampoline, TrapCode,
    TrapEncodingBuilder, TrapInformation, Tunables, TypeTables, VMOffsets,
};

/// A compiler that compiles a WebAssembly module with Compiler, translating
/// the Wasm to Compiler IR, optimizing it and then translating to assembly.
pub(crate) struct Compiler {
    translators: Mutex<Vec<FuncTranslator>>,
    isa: Box<dyn TargetIsa>,
    linkopts: LinkOptions,
}

impl Compiler {
    pub(crate) fn new(isa: Box<dyn TargetIsa>, linkopts: LinkOptions) -> Compiler {
        Compiler {
            translators: Default::default(),
            isa,
            linkopts,
        }
    }

    fn take_translator(&self) -> FuncTranslator {
        let candidate = self.translators.lock().unwrap().pop();
        candidate.unwrap_or_else(FuncTranslator::new)
    }

    fn save_translator(&self, translator: FuncTranslator) {
        self.translators.lock().unwrap().push(translator);
    }

    fn get_function_address_map(
        &self,
        context: &Context,
        data: &FunctionBodyData<'_>,
        body_len: u32,
    ) -> FunctionAddressMap {
        // Generate artificial srcloc for function start/end to identify boundary
        // within module.
        let data = data.body.get_binary_reader();
        let offset = data.original_position();
        let len = data.bytes_remaining();
        assert!((offset + len) <= u32::max_value() as usize);
        let start_srcloc = FilePos::new(offset as u32);
        let end_srcloc = FilePos::new((offset + len) as u32);

        // New-style backend: we have a `MachCompileResult` that will give us `MachSrcLoc` mapping
        // tuples.
        let instructions = collect_address_maps(
            body_len,
            context
                .mach_compile_result
                .as_ref()
                .unwrap()
                .buffer
                .get_srclocs_sorted()
                .into_iter()
                .map(|&MachSrcLoc { start, end, loc }| (loc, start, (end - start))),
        );

        FunctionAddressMap {
            instructions: instructions.into(),
            start_srcloc,
            end_srcloc,
            body_offset: 0,
            body_len,
        }
    }
}

impl wasmtime_environ::Compiler for Compiler {
    fn compile_function(
        &self,
        translation: &ModuleTranslation<'_>,
        func_index: DefinedFuncIndex,
        mut input: FunctionBodyData<'_>,
        tunables: &Tunables,
        types: &TypeTables,
    ) -> Result<Box<dyn Any + Send>, CompileError> {
        let isa = &*self.isa;
        let module = &translation.module;
        let func_index = module.func_index(func_index);
        let mut context = Context::new();
        context.func.name = get_func_name(func_index);
        context.func.signature = func_signature(isa, translation, types, func_index);
        if tunables.generate_native_debuginfo {
            context.func.collect_debug_info();
        }

        let mut func_env = FuncEnvironment::new(isa, translation, types, tunables);

        // We use these as constant offsets below in
        // `stack_limit_from_arguments`, so assert their values here. This
        // allows the closure below to get coerced to a function pointer, as
        // needed by `ir::Function`.
        //
        // Otherwise our stack limit is specially calculated from the vmctx
        // argument, where we need to load the `*const VMInterrupts`
        // pointer, and then from that pointer we need to load the stack
        // limit itself. Note that manual register allocation is needed here
        // too due to how late in the process this codegen happens.
        //
        // For more information about interrupts and stack checks, see the
        // top of this file.
        let vmctx = context
            .func
            .create_global_value(ir::GlobalValueData::VMContext);
        let interrupts_ptr = context.func.create_global_value(ir::GlobalValueData::Load {
            base: vmctx,
            offset: i32::try_from(func_env.offsets.vmctx_interrupts())
                .unwrap()
                .into(),
            global_type: isa.pointer_type(),
            readonly: true,
        });
        let stack_limit = context.func.create_global_value(ir::GlobalValueData::Load {
            base: interrupts_ptr,
            offset: i32::try_from(func_env.offsets.vminterrupts_stack_limit())
                .unwrap()
                .into(),
            global_type: isa.pointer_type(),
            readonly: false,
        });
        context.func.stack_limit = Some(stack_limit);
        let mut func_translator = self.take_translator();
        func_translator.translate_body(
            &mut input.validator,
            input.body.clone(),
            &mut context.func,
            &mut func_env,
        )?;
        self.save_translator(func_translator);

        let mut code_buf: Vec<u8> = Vec::new();
        let mut reloc_sink = RelocSink::new();
        let mut trap_sink = TrapSink::new();
        let mut stack_map_sink = StackMapSink::default();
        context
            .compile_and_emit(
                isa,
                &mut code_buf,
                &mut reloc_sink,
                &mut trap_sink,
                &mut stack_map_sink,
            )
            .map_err(|error| CompileError::Codegen(pretty_error(&context.func, error)))?;

        let unwind_info = context
            .create_unwind_info(isa)
            .map_err(|error| CompileError::Codegen(pretty_error(&context.func, error)))?;

        let address_transform =
            self.get_function_address_map(&context, &input, code_buf.len() as u32);

        let ranges = if tunables.generate_native_debuginfo {
            Some(
                context
                    .mach_compile_result
                    .as_ref()
                    .unwrap()
                    .value_labels_ranges
                    .clone(),
            )
        } else {
            None
        };

        let timing = cranelift_codegen::timing::take_current();
        log::debug!("{:?} translated in {:?}", func_index, timing.total());
        log::trace!("{:?} timing info\n{}", func_index, timing);

        let length = u32::try_from(code_buf.len()).unwrap();
        Ok(Box::new(CompiledFunction {
            body: code_buf,
            relocations: reloc_sink.func_relocs,
            value_labels_ranges: ranges.unwrap_or(Default::default()),
            stack_slots: context.func.stack_slots,
            unwind_info,
            traps: trap_sink.traps,
            info: FunctionInfo {
                start_srcloc: address_transform.start_srcloc,
                stack_maps: stack_map_sink.finish(),
                start: 0,
                length,
            },
            address_map: address_transform,
        }))
    }

    fn emit_obj(
        &self,
        translation: &ModuleTranslation,
        types: &TypeTables,
        funcs: PrimaryMap<DefinedFuncIndex, Box<dyn Any + Send>>,
        emit_dwarf: bool,
        obj: &mut Object,
    ) -> Result<(PrimaryMap<DefinedFuncIndex, FunctionInfo>, Vec<Trampoline>)> {
        let funcs: crate::CompiledFunctions = funcs
            .into_iter()
            .map(|(_i, f)| *f.downcast().unwrap())
            .collect();

        let mut builder = ObjectBuilder::new(obj, &translation.module, &*self.isa);
        if self.linkopts.force_jump_veneers {
            builder.text.force_veneers();
        }
        let mut addrs = AddressMapSection::default();
        let mut traps = TrapEncodingBuilder::default();
        let compiled_trampolines = translation
            .exported_signatures
            .iter()
            .map(|i| self.host_to_wasm_trampoline(&types.wasm_signatures[*i]))
            .collect::<Result<Vec<_>, _>>()?;

        let mut func_starts = Vec::with_capacity(funcs.len());
        for (i, func) in funcs.iter() {
            let range = builder.func(i, func);
            addrs.push(range.clone(), &func.address_map.instructions);
            traps.push(range.clone(), &func.traps);
            func_starts.push(range.start);
            if self.linkopts.padding_between_functions > 0 {
                builder
                    .text
                    .append(false, &vec![0; self.linkopts.padding_between_functions], 1);
            }
        }

        // Build trampolines for every signature that can be used by this module.
        let mut trampolines = Vec::with_capacity(translation.exported_signatures.len());
        for (i, func) in translation
            .exported_signatures
            .iter()
            .zip(&compiled_trampolines)
        {
            trampolines.push(builder.trampoline(*i, &func));
        }

        builder.unwind_info();

        if emit_dwarf && funcs.len() > 0 {
            let ofs = VMOffsets::new(
                self.isa
                    .triple()
                    .architecture
                    .pointer_width()
                    .unwrap()
                    .bytes(),
                &translation.module,
            );

            let memory_offset = if ofs.num_imported_memories > 0 {
                ModuleMemoryOffset::Imported(ofs.vmctx_vmmemory_import(MemoryIndex::new(0)))
            } else if ofs.num_defined_memories > 0 {
                ModuleMemoryOffset::Defined(
                    ofs.vmctx_vmmemory_definition_base(DefinedMemoryIndex::new(0)),
                )
            } else {
                ModuleMemoryOffset::None
            };
            let dwarf_sections = crate::debug::emit_dwarf(
                &*self.isa,
                &translation.debuginfo,
                &funcs,
                &memory_offset,
            )
            .with_context(|| "failed to emit DWARF debug information")?;
            builder.dwarf_sections(&dwarf_sections)?;
        }

        builder.finish()?;
        addrs.append_to(obj);
        traps.append_to(obj);

        Ok((
            funcs
                .into_iter()
                .zip(func_starts)
                .map(|((_, mut f), start)| {
                    f.info.start = start;
                    f.info
                })
                .collect(),
            trampolines,
        ))
    }

    fn emit_trampoline_obj(
        &self,
        ty: &WasmFuncType,
        host_fn: usize,
        obj: &mut Object,
    ) -> Result<(Trampoline, Trampoline)> {
        let host_to_wasm = self.host_to_wasm_trampoline(ty)?;
        let wasm_to_host = self.wasm_to_host_trampoline(ty, host_fn)?;
        let module = Module::new();
        let mut builder = ObjectBuilder::new(obj, &module, &*self.isa);
        let a = builder.trampoline(SignatureIndex::new(0), &host_to_wasm);
        let b = builder.trampoline(SignatureIndex::new(1), &wasm_to_host);
        builder.unwind_info();
        builder.finish()?;
        Ok((a, b))
    }

    fn triple(&self) -> &target_lexicon::Triple {
        self.isa.triple()
    }

    fn flags(&self) -> BTreeMap<String, FlagValue> {
        self.isa
            .flags()
            .iter()
            .map(|val| (val.name.to_string(), to_flag_value(&val)))
            .collect()
    }

    fn isa_flags(&self) -> BTreeMap<String, FlagValue> {
        self.isa
            .isa_flags()
            .iter()
            .map(|val| (val.name.to_string(), to_flag_value(val)))
            .collect()
    }
}

fn to_flag_value(v: &settings::Value) -> FlagValue {
    match v.kind() {
        settings::SettingKind::Enum => FlagValue::Enum(v.as_enum().unwrap().into()),
        settings::SettingKind::Num => FlagValue::Num(v.as_num().unwrap()),
        settings::SettingKind::Bool => FlagValue::Bool(v.as_bool().unwrap()),
        settings::SettingKind::Preset => unreachable!(),
    }
}

impl Compiler {
    fn host_to_wasm_trampoline(&self, ty: &WasmFuncType) -> Result<CompiledFunction, CompileError> {
        let isa = &*self.isa;
        let value_size = mem::size_of::<u128>();
        let pointer_type = isa.pointer_type();

        // The wasm signature we're calling in this trampoline has the actual
        // ABI of the function signature described by `ty`
        let wasm_signature = indirect_signature(isa, ty);

        // The host signature has the `VMTrampoline` signature where the ABI is
        // fixed.
        let mut host_signature = blank_sig(isa, wasmtime_call_conv(isa));
        host_signature.params.push(ir::AbiParam::new(pointer_type));
        host_signature.params.push(ir::AbiParam::new(pointer_type));

        let mut func_translator = self.take_translator();
        let mut context = Context::new();
        context.func = ir::Function::with_name_signature(ExternalName::user(0, 0), host_signature);

        // This trampoline will load all the parameters from the `values_vec`
        // that is passed in and then call the real function (also passed
        // indirectly) with the specified ABI.
        //
        // All the results are then stored into the same `values_vec`.
        let mut builder = FunctionBuilder::new(&mut context.func, func_translator.context());
        let block0 = builder.create_block();

        builder.append_block_params_for_function_params(block0);
        builder.switch_to_block(block0);
        builder.seal_block(block0);

        let (vmctx_ptr_val, caller_vmctx_ptr_val, callee_value, values_vec_ptr_val) = {
            let params = builder.func.dfg.block_params(block0);
            (params[0], params[1], params[2], params[3])
        };

        // Load the argument values out of `values_vec`.
        let mflags = ir::MemFlags::trusted();
        let callee_args = wasm_signature
            .params
            .iter()
            .enumerate()
            .map(|(i, r)| {
                match i {
                    0 => vmctx_ptr_val,
                    1 => caller_vmctx_ptr_val,
                    _ =>
                    // i - 2 because vmctx and caller vmctx aren't passed through `values_vec`.
                    {
                        builder.ins().load(
                            r.value_type,
                            mflags,
                            values_vec_ptr_val,
                            ((i - 2) * value_size) as i32,
                        )
                    }
                }
            })
            .collect::<Vec<_>>();

        // Call the indirect function pointer we were given
        let new_sig = builder.import_signature(wasm_signature);
        let call = builder
            .ins()
            .call_indirect(new_sig, callee_value, &callee_args);
        let results = builder.func.dfg.inst_results(call).to_vec();

        // Store the return values into `values_vec`.
        let mflags = ir::MemFlags::trusted();
        for (i, r) in results.iter().enumerate() {
            builder
                .ins()
                .store(mflags, *r, values_vec_ptr_val, (i * value_size) as i32);
        }
        builder.ins().return_(&[]);
        builder.finalize();

        let func = self.finish_trampoline(context, isa)?;
        self.save_translator(func_translator);
        Ok(func)
    }

    fn wasm_to_host_trampoline(
        &self,
        ty: &WasmFuncType,
        host_fn: usize,
    ) -> Result<CompiledFunction, CompileError> {
        let isa = &*self.isa;
        let pointer_type = isa.pointer_type();
        let wasm_signature = indirect_signature(isa, ty);
        // The host signature has an added parameter for the `values_vec` input
        // and output.
        let mut host_signature = blank_sig(isa, wasmtime_call_conv(isa));
        host_signature.params.push(ir::AbiParam::new(pointer_type));

        // Compute the size of the values vector. The vmctx and caller vmctx are passed separately.
        let value_size = mem::size_of::<u128>();
        let values_vec_len = (value_size * cmp::max(ty.params().len(), ty.returns().len())) as u32;

        let mut context = Context::new();
        context.func =
            ir::Function::with_name_signature(ir::ExternalName::user(0, 0), wasm_signature);

        let ss = context.func.create_stack_slot(ir::StackSlotData::new(
            ir::StackSlotKind::ExplicitSlot,
            values_vec_len,
        ));

        let mut func_translator = self.take_translator();
        let mut builder = FunctionBuilder::new(&mut context.func, func_translator.context());
        let block0 = builder.create_block();

        builder.append_block_params_for_function_params(block0);
        builder.switch_to_block(block0);
        builder.seal_block(block0);

        let values_vec_ptr_val = builder.ins().stack_addr(pointer_type, ss, 0);
        let mflags = MemFlags::trusted();
        for i in 0..ty.params().len() {
            let val = builder.func.dfg.block_params(block0)[i + 2];
            builder
                .ins()
                .store(mflags, val, values_vec_ptr_val, (i * value_size) as i32);
        }

        let block_params = builder.func.dfg.block_params(block0);
        let vmctx_ptr_val = block_params[0];
        let caller_vmctx_ptr_val = block_params[1];

        let callee_args = vec![vmctx_ptr_val, caller_vmctx_ptr_val, values_vec_ptr_val];

        let new_sig = builder.import_signature(host_signature);

        let callee_value = builder.ins().iconst(pointer_type, host_fn as i64);
        builder
            .ins()
            .call_indirect(new_sig, callee_value, &callee_args);

        let mflags = MemFlags::trusted();
        let mut results = Vec::new();
        for (i, r) in ty.returns().iter().enumerate() {
            let load = builder.ins().load(
                value_type(isa, *r),
                mflags,
                values_vec_ptr_val,
                (i * value_size) as i32,
            );
            results.push(load);
        }
        builder.ins().return_(&results);
        builder.finalize();

        let func = self.finish_trampoline(context, isa)?;
        self.save_translator(func_translator);
        Ok(func)
    }

    fn finish_trampoline(
        &self,
        mut context: Context,
        isa: &dyn TargetIsa,
    ) -> Result<CompiledFunction, CompileError> {
        let mut code_buf = Vec::new();
        let mut reloc_sink = TrampolineRelocSink::default();
        let mut trap_sink = binemit::NullTrapSink {};
        let mut stack_map_sink = binemit::NullStackMapSink {};
        context
            .compile_and_emit(
                isa,
                &mut code_buf,
                &mut reloc_sink,
                &mut trap_sink,
                &mut stack_map_sink,
            )
            .map_err(|error| CompileError::Codegen(pretty_error(&context.func, error)))?;

        let unwind_info = context
            .create_unwind_info(isa)
            .map_err(|error| CompileError::Codegen(pretty_error(&context.func, error)))?;

        Ok(CompiledFunction {
            body: code_buf,
            unwind_info,
            relocations: reloc_sink.relocs,
            stack_slots: Default::default(),
            value_labels_ranges: Default::default(),
            info: Default::default(),
            address_map: Default::default(),
            traps: Vec::new(),
        })
    }
}

// Collects an iterator of `InstructionAddressMap` into a `Vec` for insertion
// into a `FunctionAddressMap`. This will automatically coalesce adjacent
// instructions which map to the same original source position.
fn collect_address_maps(
    code_size: u32,
    iter: impl IntoIterator<Item = (ir::SourceLoc, u32, u32)>,
) -> Vec<InstructionAddressMap> {
    let mut iter = iter.into_iter();
    let (mut cur_loc, mut cur_offset, mut cur_len) = match iter.next() {
        Some(i) => i,
        None => return Vec::new(),
    };
    let mut ret = Vec::new();
    for (loc, offset, len) in iter {
        // If this instruction is adjacent to the previous and has the same
        // source location then we can "coalesce" it with the current
        // instruction.
        if cur_offset + cur_len == offset && loc == cur_loc {
            cur_len += len;
            continue;
        }

        // Push an entry for the previous source item.
        ret.push(InstructionAddressMap {
            srcloc: cvt(cur_loc),
            code_offset: cur_offset,
        });
        // And push a "dummy" entry if necessary to cover the span of ranges,
        // if any, between the previous source offset and this one.
        if cur_offset + cur_len != offset {
            ret.push(InstructionAddressMap {
                srcloc: FilePos::default(),
                code_offset: cur_offset + cur_len,
            });
        }
        // Update our current location to get extended later or pushed on at
        // the end.
        cur_loc = loc;
        cur_offset = offset;
        cur_len = len;
    }
    ret.push(InstructionAddressMap {
        srcloc: cvt(cur_loc),
        code_offset: cur_offset,
    });
    if cur_offset + cur_len != code_size {
        ret.push(InstructionAddressMap {
            srcloc: FilePos::default(),
            code_offset: cur_offset + cur_len,
        });
    }

    return ret;

    fn cvt(loc: ir::SourceLoc) -> FilePos {
        if loc.is_default() {
            FilePos::default()
        } else {
            FilePos::new(loc.bits())
        }
    }
}

/// Implementation of a relocation sink that just saves all the information for later
struct RelocSink {
    /// Relocations recorded for the function.
    func_relocs: Vec<Relocation>,
}

impl binemit::RelocSink for RelocSink {
    fn reloc_external(
        &mut self,
        offset: binemit::CodeOffset,
        _srcloc: ir::SourceLoc,
        reloc: binemit::Reloc,
        name: &ExternalName,
        addend: binemit::Addend,
    ) {
        let reloc_target = if let ExternalName::User { namespace, index } = *name {
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
}

impl RelocSink {
    /// Return a new `RelocSink` instance.
    fn new() -> Self {
        Self {
            func_relocs: Vec::new(),
        }
    }
}

/// Implementation of a trap sink that simply stores all trap info in-memory
#[derive(Default)]
struct TrapSink {
    /// The in-memory vector of trap info
    traps: Vec<TrapInformation>,
}

impl TrapSink {
    /// Create a new `TrapSink`
    fn new() -> Self {
        Self::default()
    }
}

impl binemit::TrapSink for TrapSink {
    fn trap(
        &mut self,
        code_offset: binemit::CodeOffset,
        _source_loc: ir::SourceLoc,
        trap_code: ir::TrapCode,
    ) {
        self.traps.push(TrapInformation {
            code_offset,
            trap_code: match trap_code {
                ir::TrapCode::StackOverflow => TrapCode::StackOverflow,
                ir::TrapCode::HeapOutOfBounds => TrapCode::HeapOutOfBounds,
                ir::TrapCode::HeapMisaligned => TrapCode::HeapMisaligned,
                ir::TrapCode::TableOutOfBounds => TrapCode::TableOutOfBounds,
                ir::TrapCode::IndirectCallToNull => TrapCode::IndirectCallToNull,
                ir::TrapCode::BadSignature => TrapCode::BadSignature,
                ir::TrapCode::IntegerOverflow => TrapCode::IntegerOverflow,
                ir::TrapCode::IntegerDivisionByZero => TrapCode::IntegerDivisionByZero,
                ir::TrapCode::BadConversionToInteger => TrapCode::BadConversionToInteger,
                ir::TrapCode::UnreachableCodeReached => TrapCode::UnreachableCodeReached,
                ir::TrapCode::Interrupt => TrapCode::Interrupt,

                // these should never be emitted by wasmtime-cranelift
                ir::TrapCode::User(_) => unreachable!(),
            },
        });
    }
}

#[derive(Default)]
struct StackMapSink {
    infos: Vec<StackMapInformation>,
}

impl binemit::StackMapSink for StackMapSink {
    fn add_stack_map(&mut self, code_offset: binemit::CodeOffset, stack_map: binemit::StackMap) {
        // This is converting from Cranelift's representation of a stack map to
        // Wasmtime's representation. They happen to align today but that may
        // not always be true in the future.
        let stack_map = wasmtime_environ::StackMap::new(
            stack_map.mapped_words(),
            stack_map.as_slice().iter().map(|a| a.0),
        );
        self.infos.push(StackMapInformation {
            code_offset,
            stack_map,
        });
    }
}

impl StackMapSink {
    fn finish(mut self) -> Vec<StackMapInformation> {
        self.infos.sort_unstable_by_key(|info| info.code_offset);
        self.infos
    }
}

/// We don't expect trampoline compilation to produce many relocations, so
/// this `RelocSink` just asserts that it doesn't recieve most of them, but
/// handles libcall ones.
#[derive(Default)]
struct TrampolineRelocSink {
    relocs: Vec<Relocation>,
}

impl binemit::RelocSink for TrampolineRelocSink {
    fn reloc_external(
        &mut self,
        offset: binemit::CodeOffset,
        _srcloc: ir::SourceLoc,
        reloc: binemit::Reloc,
        name: &ir::ExternalName,
        addend: binemit::Addend,
    ) {
        let reloc_target = if let ir::ExternalName::LibCall(libcall) = *name {
            RelocationTarget::LibCall(libcall)
        } else {
            panic!("unrecognized external name")
        };
        self.relocs.push(Relocation {
            reloc,
            reloc_target,
            offset,
            addend,
        });
    }
}
