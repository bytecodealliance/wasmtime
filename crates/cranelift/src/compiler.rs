use crate::debug::{DwarfSectionRelocTarget, ModuleMemoryOffset};
use crate::func_environ::FuncEnvironment;
use crate::obj::ModuleTextBuilder;
use crate::{
    blank_sig, func_signature, indirect_signature, value_type, wasmtime_call_conv,
    CompiledFunction, FunctionAddressMap, Relocation, RelocationTarget,
};
use anyhow::{Context as _, Result};
use cranelift_codegen::ir::{
    self, ExternalName, Function, InstBuilder, MemFlags, UserExternalName, UserFuncName, Value,
};
use cranelift_codegen::isa::{OwnedTargetIsa, TargetIsa};
use cranelift_codegen::print_errors::pretty_error;
use cranelift_codegen::Context;
use cranelift_codegen::{settings, MachReloc, MachTrap};
use cranelift_codegen::{CompiledCode, MachSrcLoc, MachStackMap};
use cranelift_entity::{EntityRef, PrimaryMap};
use cranelift_frontend::FunctionBuilder;
use cranelift_wasm::{
    DefinedFuncIndex, FuncIndex, FuncTranslator, MemoryIndex, OwnedMemoryIndex, WasmFuncType,
};
use object::write::{Object, StandardSegment, SymbolId};
use object::{RelocationEncoding, RelocationKind, SectionKind};
use std::any::Any;
use std::cmp;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::mem;
use std::sync::{Arc, Mutex};
use wasmparser::{FuncValidatorAllocations, FunctionBody};
use wasmtime_cranelift_shared::LinkOptions;
use wasmtime_environ::{
    AddressMapSection, CacheStore, CompileError, FilePos, FlagValue, FunctionBodyData, FunctionLoc,
    InstructionAddressMap, ModuleTranslation, ModuleTypes, PtrSize, StackMapInformation, Trap,
    TrapEncodingBuilder, TrapInformation, Tunables, VMOffsets, WasmFunctionInfo,
};

#[cfg(feature = "component-model")]
mod component;

struct IncrementalCacheContext {
    #[cfg(feature = "incremental-cache")]
    cache_store: Arc<dyn CacheStore>,
    num_hits: usize,
    num_cached: usize,
}

struct CompilerContext {
    func_translator: FuncTranslator,
    codegen_context: Context,
    incremental_cache_ctx: Option<IncrementalCacheContext>,
    validator_allocations: FuncValidatorAllocations,
}

impl Default for CompilerContext {
    fn default() -> Self {
        Self {
            func_translator: FuncTranslator::new(),
            codegen_context: Context::new(),
            incremental_cache_ctx: None,
            validator_allocations: Default::default(),
        }
    }
}

/// A compiler that compiles a WebAssembly module with Compiler, translating
/// the Wasm to Compiler IR, optimizing it and then translating to assembly.
pub(crate) struct Compiler {
    contexts: Mutex<Vec<CompilerContext>>,
    isa: OwnedTargetIsa,
    linkopts: LinkOptions,
    cache_store: Option<Arc<dyn CacheStore>>,
}

impl Drop for Compiler {
    fn drop(&mut self) {
        if self.cache_store.is_none() {
            return;
        }

        let mut num_hits = 0;
        let mut num_cached = 0;
        for ctx in self.contexts.lock().unwrap().iter() {
            if let Some(ref cache_ctx) = ctx.incremental_cache_ctx {
                num_hits += cache_ctx.num_hits;
                num_cached += cache_ctx.num_cached;
            }
        }

        let total = num_hits + num_cached;
        if num_hits + num_cached > 0 {
            log::trace!(
                "Incremental compilation cache stats: {}/{} = {}% (hits/lookup)\ncached: {}",
                num_hits,
                total,
                (num_hits as f32) / (total as f32) * 100.0,
                num_cached
            );
        }
    }
}

impl Compiler {
    pub(crate) fn new(
        isa: OwnedTargetIsa,
        cache_store: Option<Arc<dyn CacheStore>>,
        linkopts: LinkOptions,
    ) -> Compiler {
        Compiler {
            contexts: Default::default(),
            isa,
            linkopts,
            cache_store,
        }
    }

    pub fn isa(&self) -> &dyn TargetIsa {
        &*self.isa
    }

    fn take_context(&self) -> CompilerContext {
        let candidate = self.contexts.lock().unwrap().pop();
        candidate
            .map(|mut ctx| {
                ctx.codegen_context.clear();
                ctx
            })
            .unwrap_or_else(|| CompilerContext {
                #[cfg(feature = "incremental-cache")]
                incremental_cache_ctx: self.cache_store.as_ref().map(|cache_store| {
                    IncrementalCacheContext {
                        cache_store: cache_store.clone(),
                        num_hits: 0,
                        num_cached: 0,
                    }
                }),
                ..Default::default()
            })
    }

    fn save_context(&self, ctx: CompilerContext) {
        self.contexts.lock().unwrap().push(ctx);
    }

    fn get_function_address_map(
        compiled_code: &CompiledCode,
        body: &FunctionBody<'_>,
        body_len: u32,
        tunables: &Tunables,
    ) -> FunctionAddressMap {
        // Generate artificial srcloc for function start/end to identify boundary
        // within module.
        let data = body.get_binary_reader();
        let offset = data.original_position();
        let len = data.bytes_remaining();
        assert!((offset + len) <= u32::max_value() as usize);
        let start_srcloc = FilePos::new(offset as u32);
        let end_srcloc = FilePos::new((offset + len) as u32);

        // New-style backend: we have a `CompiledCode` that will give us `MachSrcLoc` mapping
        // tuples.
        let instructions = if tunables.generate_address_map {
            collect_address_maps(
                body_len,
                compiled_code
                    .buffer
                    .get_srclocs_sorted()
                    .into_iter()
                    .map(|&MachSrcLoc { start, end, loc }| (loc, start, (end - start))),
            )
        } else {
            Vec::new()
        };

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
        input: FunctionBodyData<'_>,
        tunables: &Tunables,
        types: &ModuleTypes,
    ) -> Result<(WasmFunctionInfo, Box<dyn Any + Send>), CompileError> {
        let isa = &*self.isa;
        let module = &translation.module;
        let func_index = module.func_index(func_index);

        let CompilerContext {
            mut func_translator,
            codegen_context: mut context,
            incremental_cache_ctx: mut cache_ctx,
            validator_allocations,
        } = self.take_context();

        context.func.signature = func_signature(isa, translation, types, func_index);
        context.func.name = UserFuncName::User(UserExternalName {
            namespace: 0,
            index: func_index.as_u32(),
        });

        if tunables.generate_native_debuginfo {
            context.func.collect_debug_info();
        }

        let mut func_env = FuncEnvironment::new(isa, translation, types, tunables);

        // The `stack_limit` global value below is the implementation of stack
        // overflow checks in Wasmtime.
        //
        // The Wasm spec defines that stack overflows will raise a trap, and
        // there's also an added constraint where as an embedder you frequently
        // are running host-provided code called from wasm. WebAssembly and
        // native code currently share the same call stack, so Wasmtime needs to
        // make sure that host-provided code will have enough call-stack
        // available to it.
        //
        // The way that stack overflow is handled here is by adding a prologue
        // check to all functions for how much native stack is remaining. The
        // `VMContext` pointer is the first argument to all functions, and the
        // first field of this structure is `*const VMRuntimeLimits` and the
        // first field of that is the stack limit. Note that the stack limit in
        // this case means "if the stack pointer goes below this, trap". Each
        // function which consumes stack space or isn't a leaf function starts
        // off by loading the stack limit, checking it against the stack
        // pointer, and optionally traps.
        //
        // This manual check allows the embedder to give wasm a relatively
        // precise amount of stack allocation. Using this scheme we reserve a
        // chunk of stack for wasm code relative from where wasm code was
        // called. This ensures that native code called by wasm should have
        // native stack space to run, and the numbers of stack spaces here
        // should all be configurable for various embeddings.
        //
        // Note that this check is independent of each thread's stack guard page
        // here. If the stack guard page is reached that's still considered an
        // abort for the whole program since the runtime limits configured by
        // the embedder should cause wasm to trap before it reaches that
        // (ensuring the host has enough space as well for its functionality).
        let vmctx = context
            .func
            .create_global_value(ir::GlobalValueData::VMContext);
        let interrupts_ptr = context.func.create_global_value(ir::GlobalValueData::Load {
            base: vmctx,
            offset: i32::try_from(func_env.offsets.vmctx_runtime_limits())
                .unwrap()
                .into(),
            global_type: isa.pointer_type(),
            readonly: true,
        });
        let stack_limit = context.func.create_global_value(ir::GlobalValueData::Load {
            base: interrupts_ptr,
            offset: i32::try_from(func_env.offsets.ptr.vmruntime_limits_stack_limit())
                .unwrap()
                .into(),
            global_type: isa.pointer_type(),
            readonly: false,
        });
        context.func.stack_limit = Some(stack_limit);
        let FunctionBodyData { validator, body } = input;
        let mut validator = validator.into_validator(validator_allocations);
        func_translator.translate_body(
            &mut validator,
            body.clone(),
            &mut context.func,
            &mut func_env,
        )?;

        let (_, code_buf) = compile_maybe_cached(&mut context, isa, cache_ctx.as_mut())?;
        // compile_maybe_cached returns the compiled_code but that borrow has the same lifetime as
        // the mutable borrow of `context`, so the borrow checker prohibits other borrows from
        // `context` while it's alive. Borrow it again to make the borrow checker happy.
        let compiled_code = context.compiled_code().unwrap();
        let alignment = compiled_code.alignment;

        let func_relocs = compiled_code
            .buffer
            .relocs()
            .into_iter()
            .map(|item| mach_reloc_to_reloc(&context.func, item))
            .collect();

        let traps = compiled_code
            .buffer
            .traps()
            .into_iter()
            .map(mach_trap_to_trap)
            .collect();

        let stack_maps = mach_stack_maps_to_stack_maps(compiled_code.buffer.stack_maps());

        let unwind_info = if isa.flags().unwind_info() {
            compiled_code
                .create_unwind_info(isa)
                .map_err(|error| CompileError::Codegen(pretty_error(&context.func, error)))?
        } else {
            None
        };

        let length = u32::try_from(code_buf.len()).unwrap();

        let address_transform =
            Self::get_function_address_map(compiled_code, &body, length, tunables);

        let ranges = if tunables.generate_native_debuginfo {
            Some(compiled_code.value_labels_ranges.clone())
        } else {
            None
        };

        let timing = cranelift_codegen::timing::take_current();
        log::debug!("{:?} translated in {:?}", func_index, timing.total());
        log::trace!("{:?} timing info\n{}", func_index, timing);

        let sized_stack_slots = std::mem::take(&mut context.func.sized_stack_slots);

        self.save_context(CompilerContext {
            func_translator,
            codegen_context: context,
            incremental_cache_ctx: cache_ctx,
            validator_allocations: validator.into_allocations(),
        });

        Ok((
            WasmFunctionInfo {
                start_srcloc: address_transform.start_srcloc,
                stack_maps: stack_maps.into(),
            },
            Box::new(CompiledFunction {
                body: code_buf,
                relocations: func_relocs,
                value_labels_ranges: ranges.unwrap_or(Default::default()),
                sized_stack_slots,
                unwind_info,
                traps,
                alignment,
                address_map: address_transform,
            }),
        ))
    }

    fn compile_host_to_wasm_trampoline(
        &self,
        ty: &WasmFuncType,
    ) -> Result<Box<dyn Any + Send>, CompileError> {
        self.host_to_wasm_trampoline(ty)
            .map(|x| Box::new(x) as Box<_>)
    }

    fn append_code(
        &self,
        obj: &mut Object<'static>,
        funcs: &[(String, Box<dyn Any + Send>)],
        tunables: &Tunables,
        resolve_reloc: &dyn Fn(usize, FuncIndex) -> usize,
    ) -> Result<Vec<(SymbolId, FunctionLoc)>> {
        let mut builder = ModuleTextBuilder::new(obj, self, funcs.len());
        if self.linkopts.force_jump_veneers {
            builder.force_veneers();
        }
        let mut addrs = AddressMapSection::default();
        let mut traps = TrapEncodingBuilder::default();

        let mut ret = Vec::with_capacity(funcs.len());
        for (i, (sym, func)) in funcs.iter().enumerate() {
            let func = func.downcast_ref().unwrap();
            let (sym, range) = builder.append_func(&sym, func, |idx| resolve_reloc(i, idx));
            if tunables.generate_address_map {
                addrs.push(range.clone(), &func.address_map.instructions);
            }
            traps.push(range.clone(), &func.traps);
            builder.append_padding(self.linkopts.padding_between_functions);
            let info = FunctionLoc {
                start: u32::try_from(range.start).unwrap(),
                length: u32::try_from(range.end - range.start).unwrap(),
            };
            ret.push((sym, info));
        }

        builder.finish();

        if tunables.generate_address_map {
            addrs.append_to(obj);
        }
        traps.append_to(obj);

        Ok(ret)
    }

    fn emit_trampoline_obj(
        &self,
        ty: &WasmFuncType,
        host_fn: usize,
        obj: &mut Object<'static>,
    ) -> Result<(FunctionLoc, FunctionLoc)> {
        let host_to_wasm = self.host_to_wasm_trampoline(ty)?;
        let wasm_to_host = self.wasm_to_host_trampoline(ty, host_fn)?;
        let mut builder = ModuleTextBuilder::new(obj, self, 2);
        let (_, a) = builder.append_func("host_to_wasm", &host_to_wasm, |_| unreachable!());
        let (_, b) = builder.append_func("wasm_to_host", &wasm_to_host, |_| unreachable!());
        let a = FunctionLoc {
            start: u32::try_from(a.start).unwrap(),
            length: u32::try_from(a.end - a.start).unwrap(),
        };
        let b = FunctionLoc {
            start: u32::try_from(b.start).unwrap(),
            length: u32::try_from(b.end - b.start).unwrap(),
        };
        builder.finish();
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

    fn is_branch_protection_enabled(&self) -> bool {
        self.isa.is_branch_protection_enabled()
    }

    #[cfg(feature = "component-model")]
    fn component_compiler(&self) -> &dyn wasmtime_environ::component::ComponentCompiler {
        self
    }

    fn append_dwarf(
        &self,
        obj: &mut Object<'_>,
        translation: &ModuleTranslation<'_>,
        funcs: &PrimaryMap<DefinedFuncIndex, (SymbolId, &(dyn Any + Send))>,
    ) -> Result<()> {
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
            // The addition of shared memory makes the following assumption,
            // "owned memory index = 0", possibly false. If the first memory
            // is a shared memory, the base pointer will not be stored in
            // the `owned_memories` array. The following code should
            // eventually be fixed to not only handle shared memories but
            // also multiple memories.
            assert_eq!(
                ofs.num_defined_memories, ofs.num_owned_memories,
                "the memory base pointer may be incorrect due to sharing memory"
            );
            ModuleMemoryOffset::Defined(
                ofs.vmctx_vmmemory_definition_base(OwnedMemoryIndex::new(0)),
            )
        } else {
            ModuleMemoryOffset::None
        };
        let compiled_funcs = funcs
            .iter()
            .map(|(_, (_, func))| func.downcast_ref().unwrap())
            .collect();
        let dwarf_sections = crate::debug::emit_dwarf(
            &*self.isa,
            &translation.debuginfo,
            &compiled_funcs,
            &memory_offset,
        )
        .with_context(|| "failed to emit DWARF debug information")?;

        let (debug_bodies, debug_relocs): (Vec<_>, Vec<_>) = dwarf_sections
            .iter()
            .map(|s| ((s.name, &s.body), (s.name, &s.relocs)))
            .unzip();
        let mut dwarf_sections_ids = HashMap::new();
        for (name, body) in debug_bodies {
            let segment = obj.segment_name(StandardSegment::Debug).to_vec();
            let section_id = obj.add_section(segment, name.as_bytes().to_vec(), SectionKind::Debug);
            dwarf_sections_ids.insert(name, section_id);
            obj.append_section_data(section_id, &body, 1);
        }

        // Write all debug data relocations.
        for (name, relocs) in debug_relocs {
            let section_id = *dwarf_sections_ids.get(name).unwrap();
            for reloc in relocs {
                let target_symbol = match reloc.target {
                    DwarfSectionRelocTarget::Func(index) => funcs[DefinedFuncIndex::new(index)].0,
                    DwarfSectionRelocTarget::Section(name) => {
                        obj.section_symbol(dwarf_sections_ids[name])
                    }
                };
                obj.add_relocation(
                    section_id,
                    object::write::Relocation {
                        offset: u64::from(reloc.offset),
                        size: reloc.size << 3,
                        kind: RelocationKind::Absolute,
                        encoding: RelocationEncoding::Generic,
                        symbol: target_symbol,
                        addend: i64::from(reloc.addend),
                    },
                )?;
            }
        }

        Ok(())
    }
}

#[cfg(feature = "incremental-cache")]
mod incremental_cache {
    use super::*;

    struct CraneliftCacheStore(Arc<dyn CacheStore>);

    impl cranelift_codegen::incremental_cache::CacheKvStore for CraneliftCacheStore {
        fn get(&self, key: &[u8]) -> Option<std::borrow::Cow<[u8]>> {
            self.0.get(key)
        }
        fn insert(&mut self, key: &[u8], val: Vec<u8>) {
            self.0.insert(key, val);
        }
    }

    pub(super) fn compile_maybe_cached<'a>(
        context: &'a mut Context,
        isa: &dyn TargetIsa,
        cache_ctx: Option<&mut IncrementalCacheContext>,
    ) -> Result<(&'a CompiledCode, Vec<u8>), CompileError> {
        let cache_ctx = match cache_ctx {
            Some(ctx) => ctx,
            None => return compile_uncached(context, isa),
        };

        let mut cache_store = CraneliftCacheStore(cache_ctx.cache_store.clone());
        let (compiled_code, from_cache) = context
            .compile_with_cache(isa, &mut cache_store)
            .map_err(|error| CompileError::Codegen(pretty_error(&error.func, error.inner)))?;

        if from_cache {
            cache_ctx.num_hits += 1;
        } else {
            cache_ctx.num_cached += 1;
        }

        Ok((compiled_code, compiled_code.code_buffer().to_vec()))
    }
}

#[cfg(feature = "incremental-cache")]
use incremental_cache::*;

#[cfg(not(feature = "incremental-cache"))]
fn compile_maybe_cached<'a>(
    context: &'a mut Context,
    isa: &dyn TargetIsa,
    _cache_ctx: Option<&mut IncrementalCacheContext>,
) -> Result<(&'a CompiledCode, Vec<u8>), CompileError> {
    compile_uncached(context, isa)
}

fn compile_uncached<'a>(
    context: &'a mut Context,
    isa: &dyn TargetIsa,
) -> Result<(&'a CompiledCode, Vec<u8>), CompileError> {
    let mut code_buf = Vec::new();
    let compiled_code = context
        .compile_and_emit(isa, &mut code_buf)
        .map_err(|error| CompileError::Codegen(pretty_error(&error.func, error.inner)))?;
    Ok((compiled_code, code_buf))
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

        let CompilerContext {
            mut func_translator,
            codegen_context: mut context,
            incremental_cache_ctx: mut cache_ctx,
            validator_allocations,
        } = self.take_context();

        // The name doesn't matter here.
        context.func = ir::Function::with_name_signature(UserFuncName::default(), host_signature);

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
        let mut mflags = ir::MemFlags::trusted();
        mflags.set_endianness(ir::Endianness::Little);
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
        for (i, r) in results.iter().enumerate() {
            builder
                .ins()
                .store(mflags, *r, values_vec_ptr_val, (i * value_size) as i32);
        }
        builder.ins().return_(&[]);
        builder.finalize();

        let func = self.finish_trampoline(&mut context, cache_ctx.as_mut(), isa)?;
        self.save_context(CompilerContext {
            func_translator,
            codegen_context: context,
            incremental_cache_ctx: cache_ctx,
            validator_allocations,
        });
        Ok(func)
    }

    /// Creates a trampoline for WebAssembly calling into the host where all the
    /// arguments are spilled to the stack and results are loaded from the
    /// stack.
    ///
    /// This style of trampoline is currently only used with the
    /// `Func::new`-style created functions in the Wasmtime embedding API. The
    /// generated trampoline has a function signature appropriate to the `ty`
    /// specified (e.g. a System-V ABI) and will call a `host_fn` that has a
    /// type signature of:
    ///
    /// ```ignore
    /// extern "C" fn(*mut VMContext, *mut VMContext, *mut ValRaw, usize)
    /// ```
    ///
    /// where the first two arguments are forwarded from the trampoline
    /// generated here itself, and the second two arguments are a pointer/length
    /// into stack-space of this trampoline with storage for both the arguments
    /// to the function and the results.
    ///
    /// Note that `host_fn` is an immediate which is an actual function pointer
    /// in this process. As such this compiled trampoline is not suitable for
    /// serialization.
    fn wasm_to_host_trampoline(
        &self,
        ty: &WasmFuncType,
        host_fn: usize,
    ) -> Result<CompiledFunction, CompileError> {
        let isa = &*self.isa;
        let pointer_type = isa.pointer_type();
        let wasm_signature = indirect_signature(isa, ty);
        let mut host_signature = blank_sig(isa, wasmtime_call_conv(isa));
        // The host signature has an added parameter for the `values_vec`
        // input/output buffer in addition to the size of the buffer, in units
        // of `ValRaw`.
        host_signature.params.push(ir::AbiParam::new(pointer_type));
        host_signature.params.push(ir::AbiParam::new(pointer_type));

        let CompilerContext {
            mut func_translator,
            codegen_context: mut context,
            incremental_cache_ctx: mut cache_ctx,
            validator_allocations,
        } = self.take_context();

        // The name doesn't matter here.
        context.func = ir::Function::with_name_signature(Default::default(), wasm_signature);

        let mut builder = FunctionBuilder::new(&mut context.func, func_translator.context());
        let block0 = builder.create_block();

        let (values_vec_ptr_val, values_vec_len) =
            self.wasm_to_host_spill_args(ty, &mut builder, block0);

        let block_params = builder.func.dfg.block_params(block0);
        let callee_args = [
            block_params[0],
            block_params[1],
            values_vec_ptr_val,
            builder
                .ins()
                .iconst(pointer_type, i64::from(values_vec_len)),
        ];

        let new_sig = builder.import_signature(host_signature);
        let callee_value = builder.ins().iconst(pointer_type, host_fn as i64);
        builder
            .ins()
            .call_indirect(new_sig, callee_value, &callee_args);

        self.wasm_to_host_load_results(ty, builder, values_vec_ptr_val);

        let func = self.finish_trampoline(&mut context, cache_ctx.as_mut(), isa)?;
        self.save_context(CompilerContext {
            func_translator,
            codegen_context: context,
            incremental_cache_ctx: cache_ctx,
            validator_allocations,
        });
        Ok(func)
    }

    /// Used for spilling arguments in wasm-to-host trampolines into the stack
    /// of the function of `builder` specified.
    ///
    /// The `block0` is the entry block of the function and `ty` is the wasm
    /// signature of the trampoline generated. This function will allocate a
    /// stack slot suitable for storing both the arguments and return values of
    /// the function, and then the arguments will all be stored in this block.
    ///
    /// The stack slot pointer is returned in addition to the size, in units of
    /// `ValRaw`, of the stack slot.
    fn wasm_to_host_spill_args(
        &self,
        ty: &WasmFuncType,
        builder: &mut FunctionBuilder,
        block0: ir::Block,
    ) -> (Value, u32) {
        let isa = &*self.isa;
        let pointer_type = isa.pointer_type();

        // Compute the size of the values vector.
        let value_size = mem::size_of::<u128>();
        let values_vec_len = cmp::max(ty.params().len(), ty.returns().len());
        let values_vec_byte_size = u32::try_from(value_size * values_vec_len).unwrap();
        let values_vec_len = u32::try_from(values_vec_len).unwrap();

        let ss = builder.func.create_sized_stack_slot(ir::StackSlotData::new(
            ir::StackSlotKind::ExplicitSlot,
            values_vec_byte_size,
        ));

        builder.append_block_params_for_function_params(block0);
        builder.switch_to_block(block0);
        builder.seal_block(block0);

        // Note that loads and stores are unconditionally done in the
        // little-endian format rather than the host's native-endianness,
        // despite this load/store being unrelated to execution in wasm itself.
        // For more details on this see the `ValRaw` type in the
        // `wasmtime-runtime` crate.
        let mut mflags = MemFlags::trusted();
        mflags.set_endianness(ir::Endianness::Little);

        let values_vec_ptr_val = builder.ins().stack_addr(pointer_type, ss, 0);
        for i in 0..ty.params().len() {
            let val = builder.func.dfg.block_params(block0)[i + 2];
            builder
                .ins()
                .store(mflags, val, values_vec_ptr_val, (i * value_size) as i32);
        }
        (values_vec_ptr_val, values_vec_len)
    }

    /// Use for loading the results of a host call from a trampoline's stack
    /// space.
    ///
    /// This is intended to be used with the stack space allocated by
    /// `wasm_to_host_spill_args` above. This is called after the function call
    /// is made which will load results from the stack space and then return
    /// them with the appropriate ABI (e.g. System-V).
    fn wasm_to_host_load_results(
        &self,
        ty: &WasmFuncType,
        mut builder: FunctionBuilder,
        values_vec_ptr_val: Value,
    ) {
        let isa = &*self.isa;
        let value_size = mem::size_of::<u128>();

        // Note that this is little-endian like `wasm_to_host_spill_args` above,
        // see notes there for more information.
        let mut mflags = MemFlags::trusted();
        mflags.set_endianness(ir::Endianness::Little);

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
    }

    fn finish_trampoline(
        &self,
        context: &mut Context,
        cache_ctx: Option<&mut IncrementalCacheContext>,
        isa: &dyn TargetIsa,
    ) -> Result<CompiledFunction, CompileError> {
        let (compiled_code, code_buf) = compile_maybe_cached(context, isa, cache_ctx)?;

        // Processing relocations isn't the hardest thing in the world here but
        // no trampoline should currently generate a relocation, so assert that
        // they're all empty and if this ever trips in the future then handling
        // will need to be added here to ensure they make their way into the
        // `CompiledFunction` below.
        assert!(compiled_code.buffer.relocs().is_empty());

        let traps = compiled_code
            .buffer
            .traps()
            .into_iter()
            .map(mach_trap_to_trap)
            .collect();
        let alignment = compiled_code.alignment;

        let unwind_info = if isa.flags().unwind_info() {
            compiled_code
                .create_unwind_info(isa)
                .map_err(|error| CompileError::Codegen(pretty_error(&context.func, error)))?
        } else {
            None
        };

        Ok(CompiledFunction {
            body: code_buf,
            unwind_info,
            relocations: Default::default(),
            sized_stack_slots: Default::default(),
            value_labels_ranges: Default::default(),
            address_map: Default::default(),
            traps,
            alignment,
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

fn mach_reloc_to_reloc(func: &Function, reloc: &MachReloc) -> Relocation {
    let &MachReloc {
        offset,
        kind,
        ref name,
        addend,
    } = reloc;
    let reloc_target = if let ExternalName::User(user_func_ref) = *name {
        let UserExternalName { namespace, index } = func.params.user_named_funcs()[user_func_ref];
        debug_assert_eq!(namespace, 0);
        RelocationTarget::UserFunc(FuncIndex::from_u32(index))
    } else if let ExternalName::LibCall(libcall) = *name {
        RelocationTarget::LibCall(libcall)
    } else {
        panic!("unrecognized external name")
    };
    Relocation {
        reloc: kind,
        reloc_target,
        offset,
        addend,
    }
}

const ALWAYS_TRAP_CODE: u16 = 100;

fn mach_trap_to_trap(trap: &MachTrap) -> TrapInformation {
    let &MachTrap { offset, code } = trap;
    TrapInformation {
        code_offset: offset,
        trap_code: match code {
            ir::TrapCode::StackOverflow => Trap::StackOverflow,
            ir::TrapCode::HeapOutOfBounds => Trap::MemoryOutOfBounds,
            ir::TrapCode::HeapMisaligned => Trap::HeapMisaligned,
            ir::TrapCode::TableOutOfBounds => Trap::TableOutOfBounds,
            ir::TrapCode::IndirectCallToNull => Trap::IndirectCallToNull,
            ir::TrapCode::BadSignature => Trap::BadSignature,
            ir::TrapCode::IntegerOverflow => Trap::IntegerOverflow,
            ir::TrapCode::IntegerDivisionByZero => Trap::IntegerDivisionByZero,
            ir::TrapCode::BadConversionToInteger => Trap::BadConversionToInteger,
            ir::TrapCode::UnreachableCodeReached => Trap::UnreachableCodeReached,
            ir::TrapCode::Interrupt => Trap::Interrupt,
            ir::TrapCode::User(ALWAYS_TRAP_CODE) => Trap::AlwaysTrapAdapter,

            // these should never be emitted by wasmtime-cranelift
            ir::TrapCode::User(_) => unreachable!(),
        },
    }
}

fn mach_stack_maps_to_stack_maps(mach_stack_maps: &[MachStackMap]) -> Vec<StackMapInformation> {
    // This is converting from Cranelift's representation of a stack map to
    // Wasmtime's representation. They happen to align today but that may
    // not always be true in the future.
    let mut stack_maps = Vec::new();
    for &MachStackMap {
        offset_end,
        ref stack_map,
        ..
    } in mach_stack_maps
    {
        let stack_map = wasmtime_environ::StackMap::new(
            stack_map.mapped_words(),
            stack_map.as_slice().iter().map(|a| a.0),
        );
        stack_maps.push(StackMapInformation {
            code_offset: offset_end,
            stack_map,
        });
    }
    stack_maps.sort_unstable_by_key(|info| info.code_offset);
    stack_maps
}
