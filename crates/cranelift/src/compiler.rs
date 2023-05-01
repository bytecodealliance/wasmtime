use crate::debug::{DwarfSectionRelocTarget, ModuleMemoryOffset};
use crate::func_environ::FuncEnvironment;
use crate::{array_call_signature, native_call_signature, DEBUG_ASSERT_TRAP_CODE};
use crate::{
    builder::LinkOptions, value_type, wasm_call_signature, CompiledFunction, FunctionAddressMap,
};
use anyhow::{Context as _, Result};
use cranelift_codegen::ir::{
    self, ExternalName, Function, InstBuilder, MemFlags, UserExternalName, UserFuncName, Value,
};
use cranelift_codegen::isa::{OwnedTargetIsa, TargetIsa};
use cranelift_codegen::print_errors::pretty_error;
use cranelift_codegen::Context;
use cranelift_codegen::{CompiledCode, MachSrcLoc, MachStackMap};
use cranelift_codegen::{MachReloc, MachTrap};
use cranelift_entity::{EntityRef, PrimaryMap};
use cranelift_frontend::FunctionBuilder;
use cranelift_wasm::{
    DefinedFuncIndex, FuncIndex, FuncTranslator, MemoryIndex, OwnedMemoryIndex, WasmFuncType,
    WasmType,
};
use object::write::{Object, StandardSegment, SymbolId};
use object::{RelocationEncoding, RelocationKind, SectionKind};
use std::any::Any;
use std::cmp;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::mem;
use std::path;
use std::sync::{Arc, Mutex};
use wasmparser::{FuncValidatorAllocations, FunctionBody};
use wasmtime_cranelift_shared::obj::ModuleTextBuilder;
use wasmtime_cranelift_shared::{Relocation, RelocationTarget};
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
    clif_dir: Option<path::PathBuf>,
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
        clif_dir: Option<path::PathBuf>,
    ) -> Compiler {
        Compiler {
            contexts: Default::default(),
            isa,
            linkopts,
            cache_store,
            clif_dir,
        }
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
        let sig = translation.module.functions[func_index].signature;
        let wasm_func_ty = &types[sig];

        let mut compiler = self.function_compiler();

        let context = &mut compiler.cx.codegen_context;
        context.func.signature = wasm_call_signature(isa, wasm_func_ty);
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
        let mut validator =
            validator.into_validator(mem::take(&mut compiler.cx.validator_allocations));
        compiler.cx.func_translator.translate_body(
            &mut validator,
            body.clone(),
            &mut context.func,
            &mut func_env,
        )?;

        if let Some(path) = &self.clif_dir {
            use std::io::Write;

            let mut path = path.to_path_buf();
            path.push(format!("wasm_func_{}", func_index.as_u32()));
            path.set_extension("clif");

            let mut output = std::fs::File::create(path).unwrap();
            write!(output, "{}", context.func.display()).unwrap();
        }

        let (info, func) = compiler.finish_with_info(Some((&body, tunables)))?;

        let timing = cranelift_codegen::timing::take_current();
        log::debug!("{:?} translated in {:?}", func_index, timing.total());
        log::trace!("{:?} timing info\n{}", func_index, timing);

        Ok((info, Box::new(func)))
    }

    fn compile_array_to_wasm_trampoline(
        &self,
        translation: &ModuleTranslation<'_>,
        types: &ModuleTypes,
        def_func_index: DefinedFuncIndex,
    ) -> Result<Box<dyn Any + Send>, CompileError> {
        let func_index = translation.module.func_index(def_func_index);
        let sig = translation.module.functions[func_index].signature;
        let wasm_func_ty = &types[sig];

        let isa = &*self.isa;
        let pointer_type = isa.pointer_type();
        let wasm_call_sig = wasm_call_signature(isa, wasm_func_ty);
        let array_call_sig = array_call_signature(isa);

        let mut compiler = self.function_compiler();
        let func = ir::Function::with_name_signature(Default::default(), array_call_sig);
        let (mut builder, block0) = compiler.builder(func);

        let (vmctx, caller_vmctx, values_vec_ptr, values_vec_len) = {
            let params = builder.func.dfg.block_params(block0);
            (params[0], params[1], params[2], params[3])
        };

        // First load the actual arguments out of the array.
        let mut args = self.load_values_from_array(
            wasm_func_ty.params(),
            &mut builder,
            values_vec_ptr,
            values_vec_len,
        );
        args.insert(0, caller_vmctx);
        args.insert(0, vmctx);

        // Just before we enter Wasm, save our stack pointer.
        //
        // Assert that we were really given a core Wasm vmctx, since that's
        // what we are assuming with our offsets below.
        debug_assert_vmctx_kind(isa, &mut builder, vmctx, wasmtime_environ::VMCONTEXT_MAGIC);
        let offsets = VMOffsets::new(isa.pointer_bytes(), &translation.module);
        let vm_runtime_limits_offset = offsets.vmctx_runtime_limits();
        save_last_wasm_entry_sp(
            &mut builder,
            pointer_type,
            &offsets.ptr,
            vm_runtime_limits_offset,
            vmctx,
        );

        // Then call the Wasm function with those arguments.
        let call = declare_and_call(&mut builder, wasm_call_sig, func_index.as_u32(), &args);
        let results = builder.func.dfg.inst_results(call).to_vec();

        // Then store the results back into the array.
        self.store_values_to_array(
            &mut builder,
            wasm_func_ty.returns(),
            &results,
            values_vec_ptr,
            values_vec_len,
        );

        builder.ins().return_(&[]);
        builder.finalize();

        Ok(Box::new(compiler.finish()?))
    }

    fn compile_native_to_wasm_trampoline(
        &self,
        translation: &ModuleTranslation<'_>,
        types: &ModuleTypes,
        def_func_index: DefinedFuncIndex,
    ) -> Result<Box<dyn Any + Send>, CompileError> {
        let func_index = translation.module.func_index(def_func_index);
        let sig = translation.module.functions[func_index].signature;
        let wasm_func_ty = &types[sig];

        let isa = &*self.isa;
        let pointer_type = isa.pointer_type();
        let func_index = translation.module.func_index(def_func_index);
        let wasm_call_sig = wasm_call_signature(isa, wasm_func_ty);
        let native_call_sig = native_call_signature(isa, wasm_func_ty);

        let mut compiler = self.function_compiler();
        let func = ir::Function::with_name_signature(Default::default(), native_call_sig);
        let (mut builder, block0) = compiler.builder(func);

        let args = builder.func.dfg.block_params(block0).to_vec();
        let vmctx = args[0];

        // Since we are entering Wasm, save our SP.
        //
        // Assert that we were really given a core Wasm vmctx, since that's
        // what we are assuming with our offsets below.
        debug_assert_vmctx_kind(isa, &mut builder, vmctx, wasmtime_environ::VMCONTEXT_MAGIC);
        let offsets = VMOffsets::new(isa.pointer_bytes(), &translation.module);
        let vm_runtime_limits_offset = offsets.vmctx_runtime_limits();
        save_last_wasm_entry_sp(
            &mut builder,
            pointer_type,
            &offsets.ptr,
            vm_runtime_limits_offset,
            vmctx,
        );

        // Then call into Wasm.
        let call = declare_and_call(&mut builder, wasm_call_sig, func_index.as_u32(), &args);

        // Forward the results along.
        let results = builder.func.dfg.inst_results(call).to_vec();
        builder.ins().return_(&results);
        builder.finalize();

        Ok(Box::new(compiler.finish()?))
    }

    fn compile_wasm_to_native_trampoline(
        &self,
        translation: &ModuleTranslation,
        wasm_func_ty: &WasmFuncType,
    ) -> Result<Box<dyn Any + Send>, CompileError> {
        let isa = &*self.isa;
        let pointer_type = isa.pointer_type();
        let wasm_call_sig = wasm_call_signature(isa, wasm_func_ty);
        let native_call_sig = native_call_signature(isa, wasm_func_ty);

        let mut compiler = self.function_compiler();
        let func = ir::Function::with_name_signature(Default::default(), wasm_call_sig);
        let (mut builder, block0) = compiler.builder(func);

        let args = builder.func.dfg.block_params(block0).to_vec();
        let callee_vmctx = args[0];
        let caller_vmctx = args[1];

        // We are exiting Wasm, so save our PC and FP.
        //
        // Assert that the caller vmctx really is a core Wasm vmctx, since
        // that's what we are assuming with our offsets below.
        debug_assert_vmctx_kind(
            isa,
            &mut builder,
            caller_vmctx,
            wasmtime_environ::VMCONTEXT_MAGIC,
        );
        let offsets = VMOffsets::new(isa.pointer_bytes(), &translation.module);
        let limits = builder.ins().load(
            pointer_type,
            MemFlags::trusted(),
            caller_vmctx,
            i32::try_from(offsets.vmctx_runtime_limits()).unwrap(),
        );
        save_last_wasm_exit_fp_and_pc(&mut builder, pointer_type, &offsets.ptr, limits);

        // Load the actual callee out of the
        // `VMNativeCallHostFuncContext::host_func`.
        let ptr_size = isa.pointer_bytes();
        let callee = builder.ins().load(
            pointer_type,
            MemFlags::trusted(),
            callee_vmctx,
            ptr_size.vmnative_call_host_func_context_func_ref()
                + ptr_size.vm_func_ref_native_call(),
        );

        // Do an indirect call to the callee.
        let callee_signature = builder.func.import_signature(native_call_sig);
        let call = builder.ins().call_indirect(callee_signature, callee, &args);

        // Forward the results back to the caller.
        let results = builder.func.dfg.inst_results(call).to_vec();
        builder.ins().return_(&results);
        builder.finalize();

        Ok(Box::new(compiler.finish()?))
    }

    fn append_code(
        &self,
        obj: &mut Object<'static>,
        funcs: &[(String, Box<dyn Any + Send>)],
        tunables: &Tunables,
        resolve_reloc: &dyn Fn(usize, FuncIndex) -> usize,
    ) -> Result<Vec<(SymbolId, FunctionLoc)>> {
        let mut builder =
            ModuleTextBuilder::new(obj, self, self.isa.text_section_builder(funcs.len()));
        if self.linkopts.force_jump_veneers {
            builder.force_veneers();
        }
        let mut addrs = AddressMapSection::default();
        let mut traps = TrapEncodingBuilder::default();

        let mut ret = Vec::with_capacity(funcs.len());
        for (i, (sym, func)) in funcs.iter().enumerate() {
            let func = func.downcast_ref::<CompiledFunction>().unwrap();
            let (sym, range) = builder.append_func(
                &sym,
                &func.body,
                func.alignment,
                func.unwind_info.as_ref(),
                &func.relocations,
                |idx| resolve_reloc(i, idx),
            );
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

    fn emit_trampolines_for_array_call_host_func(
        &self,
        ty: &WasmFuncType,
        host_fn: usize,
        obj: &mut Object<'static>,
    ) -> Result<(FunctionLoc, FunctionLoc)> {
        let wasm_to_array = self.wasm_to_array_trampoline(ty, host_fn)?;
        let native_to_array = self.native_to_array_trampoline(ty, host_fn)?;

        let mut builder = ModuleTextBuilder::new(obj, self, self.isa.text_section_builder(2));

        let (_, wasm_to_array) = builder.append_func(
            "wasm_to_array",
            &wasm_to_array.body,
            wasm_to_array.alignment,
            wasm_to_array.unwind_info.as_ref(),
            &wasm_to_array.relocations,
            |_| unreachable!(),
        );
        let (_, native_to_array) = builder.append_func(
            "native_to_array",
            &native_to_array.body,
            native_to_array.alignment,
            native_to_array.unwind_info.as_ref(),
            &native_to_array.relocations,
            |_| unreachable!(),
        );

        let wasm_to_array = FunctionLoc {
            start: u32::try_from(wasm_to_array.start).unwrap(),
            length: u32::try_from(wasm_to_array.end - wasm_to_array.start).unwrap(),
        };
        let native_to_array = FunctionLoc {
            start: u32::try_from(native_to_array.start).unwrap(),
            length: u32::try_from(native_to_array.end - native_to_array.start).unwrap(),
        };

        builder.finish();
        Ok((wasm_to_array, native_to_array))
    }

    fn triple(&self) -> &target_lexicon::Triple {
        self.isa.triple()
    }

    fn flags(&self) -> BTreeMap<String, FlagValue> {
        wasmtime_cranelift_shared::clif_flags_to_wasmtime(self.isa.flags().iter())
    }

    fn isa_flags(&self) -> BTreeMap<String, FlagValue> {
        wasmtime_cranelift_shared::clif_flags_to_wasmtime(self.isa.isa_flags())
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

    fn create_systemv_cie(&self) -> Option<gimli::write::CommonInformationEntry> {
        self.isa.create_systemv_cie()
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
            .compile_with_cache(isa, &mut cache_store, &mut Default::default())
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
        .compile_and_emit(isa, &mut code_buf, &mut Default::default())
        .map_err(|error| CompileError::Codegen(pretty_error(&error.func, error.inner)))?;
    Ok((compiled_code, code_buf))
}

impl Compiler {
    /// Creates a trampoline for calling a host function callee defined with the
    /// "array" calling convention from a native calling convention caller.
    ///
    /// This style of trampoline is used with `Func::new`-style callees and
    /// `TypedFunc::call`-style callers.
    ///
    /// Both callee and caller are on the host side, so there is no host/Wasm
    /// transition and associated entry/exit state to maintain.
    ///
    /// The `host_fn` is a function pointer in this process with the following
    /// signature:
    ///
    /// ```ignore
    /// unsafe extern "C" fn(*mut VMContext, *mut VMContext, *mut ValRaw, usize)
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
    fn native_to_array_trampoline(
        &self,
        ty: &WasmFuncType,
        host_fn: usize,
    ) -> Result<CompiledFunction, CompileError> {
        let isa = &*self.isa;
        let pointer_type = isa.pointer_type();
        let native_call_sig = native_call_signature(isa, ty);
        let array_call_sig = array_call_signature(isa);

        let mut compiler = self.function_compiler();
        let func = ir::Function::with_name_signature(Default::default(), native_call_sig);
        let (mut builder, block0) = compiler.builder(func);

        let (values_vec_ptr, values_vec_len) =
            self.allocate_stack_array_and_spill_args(ty, &mut builder, block0);
        let values_vec_len = builder
            .ins()
            .iconst(pointer_type, i64::from(values_vec_len));

        let block_params = builder.func.dfg.block_params(block0);
        let callee_args = [
            block_params[0],
            block_params[1],
            values_vec_ptr,
            values_vec_len,
        ];

        let new_sig = builder.import_signature(array_call_sig);
        let callee_value = builder.ins().iconst(pointer_type, host_fn as i64);
        builder
            .ins()
            .call_indirect(new_sig, callee_value, &callee_args);

        let results =
            self.load_values_from_array(ty.returns(), &mut builder, values_vec_ptr, values_vec_len);
        builder.ins().return_(&results);
        builder.finalize();

        compiler.finish()
    }

    /// Creates a trampoline for WebAssembly to call a host function defined
    /// with the "array" calling convention: where all the arguments are spilled
    /// to an array on the stack and results are loaded from the stack array.
    ///
    /// This style of trampoline is currently only used with the
    /// `Func::new`-style created functions in the Wasmtime embedding API. The
    /// generated trampoline has a function signature appropriate to the `ty`
    /// specified (e.g. a System-V ABI) and will call a `host_fn` that has a
    /// type signature of:
    ///
    /// ```ignore
    /// unsafe extern "C" fn(*mut VMContext, *mut VMContext, *mut ValRaw, usize)
    /// ```
    ///
    /// where the first two arguments are forwarded from the trampoline
    /// generated here itself, and the second two arguments are a pointer/length
    /// into stack-space of this trampoline with storage for both the arguments
    /// to the function and the results.
    ///
    /// Note that `host_fn` is an immediate which is an actual function pointer
    /// in this process, and `limits` is a pointer to `VMRuntimeLimits`. As such
    /// this compiled trampoline is not suitable for serialization, and only
    /// valid for a particular store.
    fn wasm_to_array_trampoline(
        &self,
        ty: &WasmFuncType,
        host_fn: usize,
    ) -> Result<CompiledFunction, CompileError> {
        let isa = &*self.isa;
        let pointer_type = isa.pointer_type();
        let wasm_call_sig = wasm_call_signature(isa, ty);
        let array_call_sig = array_call_signature(isa);

        let mut compiler = self.function_compiler();
        let func = ir::Function::with_name_signature(Default::default(), wasm_call_sig);
        let (mut builder, block0) = compiler.builder(func);
        let caller_vmctx = builder.func.dfg.block_params(block0)[1];

        // Assert that we were really given a core Wasm vmctx, since that's
        // what we are assuming with our offsets below.
        debug_assert_vmctx_kind(
            isa,
            &mut builder,
            caller_vmctx,
            wasmtime_environ::VMCONTEXT_MAGIC,
        );
        let ptr_size = isa.pointer_bytes();
        let limits = builder.ins().load(
            pointer_type,
            MemFlags::trusted(),
            caller_vmctx,
            ptr_size.vmcontext_runtime_limits(),
        );
        save_last_wasm_exit_fp_and_pc(&mut builder, pointer_type, &ptr_size, limits);

        let (values_vec_ptr, values_vec_len) =
            self.allocate_stack_array_and_spill_args(ty, &mut builder, block0);
        let values_vec_len = builder
            .ins()
            .iconst(pointer_type, i64::from(values_vec_len));

        let block_params = builder.func.dfg.block_params(block0);
        let callee_args = [
            block_params[0],
            block_params[1],
            values_vec_ptr,
            values_vec_len,
        ];

        let new_sig = builder.import_signature(array_call_sig);
        let callee_value = builder.ins().iconst(pointer_type, host_fn as i64);
        builder
            .ins()
            .call_indirect(new_sig, callee_value, &callee_args);

        let results =
            self.load_values_from_array(ty.returns(), &mut builder, values_vec_ptr, values_vec_len);
        builder.ins().return_(&results);
        builder.finalize();

        compiler.finish()
    }

    /// This function will allocate a stack slot suitable for storing both the
    /// arguments and return values of the function, and then the arguments will
    /// all be stored in this block.
    ///
    /// `block0` must be the entry block of the function and `ty` must be the
    /// Wasm function type of the trampoline.
    ///
    /// The stack slot pointer is returned in addition to the size, in units of
    /// `ValRaw`, of the stack slot.
    fn allocate_stack_array_and_spill_args(
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

        let slot = builder.func.create_sized_stack_slot(ir::StackSlotData::new(
            ir::StackSlotKind::ExplicitSlot,
            values_vec_byte_size,
        ));
        let values_vec_ptr = builder.ins().stack_addr(pointer_type, slot, 0);

        // NB: `2..` because the vmctx and caller vmctx don't go in the array.
        let args = builder.func.dfg.block_params(block0)[2..].to_vec();

        {
            let values_vec_len = builder
                .ins()
                .iconst(ir::types::I32, i64::try_from(values_vec_len).unwrap());
            self.store_values_to_array(builder, ty.params(), &args, values_vec_ptr, values_vec_len);
        }

        (values_vec_ptr, values_vec_len)
    }

    /// Store values to an array in the array calling convention.
    ///
    /// Used either to store arguments to the array when calling a function
    /// using the array calling convention, or used to store results to the
    /// array when implementing a function that exposes the array calling
    /// convention.
    fn store_values_to_array(
        &self,
        builder: &mut FunctionBuilder,
        types: &[WasmType],
        values: &[Value],
        values_vec_ptr: Value,
        values_vec_capacity: Value,
    ) {
        debug_assert_enough_capacity_for_length(builder, types.len(), values_vec_capacity);

        // Note that loads and stores are unconditionally done in the
        // little-endian format rather than the host's native-endianness,
        // despite this load/store being unrelated to execution in wasm itself.
        // For more details on this see the `ValRaw` type in the
        // `wasmtime-runtime` crate.
        let mut mflags = MemFlags::trusted();
        mflags.set_endianness(ir::Endianness::Little);

        let value_size = mem::size_of::<u128>();
        for (i, val) in values.iter().copied().enumerate() {
            builder
                .ins()
                .store(mflags, val, values_vec_ptr, (i * value_size) as i32);
        }
    }

    /// Used for loading the values of an array-call host function's value
    /// array.
    ///
    /// This can be used to load arguments out of the array if the trampoline we
    /// are building exposes the array calling convention, or it can be used to
    /// laod results out of the array if the trampoline we are building calls a
    /// function that uses the array calling convention.
    fn load_values_from_array(
        &self,
        types: &[WasmType],
        builder: &mut FunctionBuilder,
        values_vec_ptr: Value,
        values_vec_capacity: Value,
    ) -> Vec<ir::Value> {
        let isa = &*self.isa;
        let value_size = mem::size_of::<u128>();

        debug_assert_enough_capacity_for_length(builder, types.len(), values_vec_capacity);

        // Note that this is little-endian like `store_values_to_array` above,
        // see notes there for more information.
        let mut mflags = MemFlags::trusted();
        mflags.set_endianness(ir::Endianness::Little);

        let mut results = Vec::new();
        for (i, r) in types.iter().enumerate() {
            let load = builder.ins().load(
                value_type(isa, *r),
                mflags,
                values_vec_ptr,
                (i * value_size) as i32,
            );
            results.push(load);
        }
        results
    }

    fn function_compiler(&self) -> FunctionCompiler<'_> {
        let saved_context = self.contexts.lock().unwrap().pop();
        FunctionCompiler {
            compiler: self,
            cx: saved_context
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
                }),
        }
    }
}

struct FunctionCompiler<'a> {
    compiler: &'a Compiler,
    cx: CompilerContext,
}

impl FunctionCompiler<'_> {
    fn builder(&mut self, func: ir::Function) -> (FunctionBuilder<'_>, ir::Block) {
        self.cx.codegen_context.func = func;
        let mut builder = FunctionBuilder::new(
            &mut self.cx.codegen_context.func,
            self.cx.func_translator.context(),
        );

        let block0 = builder.create_block();
        builder.append_block_params_for_function_params(block0);
        builder.switch_to_block(block0);
        builder.seal_block(block0);
        (builder, block0)
    }

    fn finish(self) -> Result<CompiledFunction, CompileError> {
        let (info, func) = self.finish_with_info(None)?;
        assert!(info.stack_maps.is_empty());
        Ok(func)
    }

    fn finish_with_info(
        mut self,
        body_and_tunables: Option<(&FunctionBody<'_>, &Tunables)>,
    ) -> Result<(WasmFunctionInfo, CompiledFunction), CompileError> {
        let context = &mut self.cx.codegen_context;
        let isa = &*self.compiler.isa;
        let (_, code_buf) =
            compile_maybe_cached(context, isa, self.cx.incremental_cache_ctx.as_mut())?;
        let compiled_code = context.compiled_code().unwrap();

        let relocations = compiled_code
            .buffer
            .relocs()
            .into_iter()
            .map(|item| mach_reloc_to_reloc(&context.func, item))
            .collect();

        let traps = compiled_code
            .buffer
            .traps()
            .into_iter()
            .filter_map(mach_trap_to_trap)
            .collect();

        // Give wasm functions, user defined code, a "preferred" alignment
        // instead of the minimum alignment as this can help perf in niche
        // situations.
        let preferred_alignment = if body_and_tunables.is_some() {
            self.compiler.isa.function_alignment().preferred
        } else {
            1
        };
        let alignment = compiled_code.alignment.max(preferred_alignment);

        let address_map = match body_and_tunables {
            Some((body, tunables)) => Compiler::get_function_address_map(
                compiled_code,
                body,
                u32::try_from(code_buf.len()).unwrap(),
                tunables,
            ),
            None => Default::default(),
        };

        let value_labels_ranges = if body_and_tunables
            .map(|(_, t)| t.generate_native_debuginfo)
            .unwrap_or(false)
        {
            compiled_code.value_labels_ranges.clone()
        } else {
            Default::default()
        };

        let unwind_info = if isa.flags().unwind_info() {
            compiled_code
                .create_unwind_info(isa)
                .map_err(|error| CompileError::Codegen(pretty_error(&context.func, error)))?
        } else {
            None
        };
        let stack_maps = mach_stack_maps_to_stack_maps(compiled_code.buffer.stack_maps());
        let sized_stack_slots = std::mem::take(&mut context.func.sized_stack_slots);
        self.compiler.contexts.lock().unwrap().push(self.cx);

        Ok((
            WasmFunctionInfo {
                start_srcloc: address_map.start_srcloc,
                stack_maps: stack_maps.into(),
            },
            CompiledFunction {
                body: code_buf,
                unwind_info,
                relocations,
                sized_stack_slots,
                value_labels_ranges,
                address_map,
                traps,
                alignment,
            },
        ))
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

fn mach_trap_to_trap(trap: &MachTrap) -> Option<TrapInformation> {
    let &MachTrap { offset, code } = trap;
    Some(TrapInformation {
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

            // These do not get converted to wasmtime traps, since they
            // shouldn't ever be hit in theory. Instead of catching and handling
            // these, we let the signal crash the process.
            ir::TrapCode::User(DEBUG_ASSERT_TRAP_CODE) => return None,

            // these should never be emitted by wasmtime-cranelift
            ir::TrapCode::User(_) => unreachable!(),
        },
    })
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

fn declare_and_call(
    builder: &mut FunctionBuilder,
    signature: ir::Signature,
    func_index: u32,
    args: &[ir::Value],
) -> ir::Inst {
    let name = ir::ExternalName::User(builder.func.declare_imported_user_function(
        ir::UserExternalName {
            namespace: 0,
            index: func_index,
        },
    ));
    let signature = builder.func.import_signature(signature);
    let callee = builder.func.dfg.ext_funcs.push(ir::ExtFuncData {
        name,
        signature,
        colocated: true,
    });
    builder.ins().call(callee, &args)
}

fn debug_assert_enough_capacity_for_length(
    builder: &mut FunctionBuilder,
    length: usize,
    capacity: ir::Value,
) {
    if cfg!(debug_assertions) {
        let enough_capacity = builder.ins().icmp_imm(
            ir::condcodes::IntCC::UnsignedGreaterThanOrEqual,
            capacity,
            ir::immediates::Imm64::new(length.try_into().unwrap()),
        );
        builder
            .ins()
            .trapz(enough_capacity, ir::TrapCode::User(DEBUG_ASSERT_TRAP_CODE));
    }
}

fn debug_assert_vmctx_kind(
    isa: &dyn TargetIsa,
    builder: &mut FunctionBuilder,
    vmctx: ir::Value,
    expected_vmctx_magic: u32,
) {
    if cfg!(debug_assertions) {
        let magic = builder.ins().load(
            ir::types::I32,
            MemFlags::trusted().with_endianness(isa.endianness()),
            vmctx,
            0,
        );
        let is_expected_vmctx = builder.ins().icmp_imm(
            ir::condcodes::IntCC::Equal,
            magic,
            i64::from(expected_vmctx_magic),
        );
        builder.ins().trapz(
            is_expected_vmctx,
            ir::TrapCode::User(DEBUG_ASSERT_TRAP_CODE),
        );
    }
}

fn save_last_wasm_entry_sp(
    builder: &mut FunctionBuilder,
    pointer_type: ir::Type,
    ptr_size: &impl PtrSize,
    vm_runtime_limits_offset: u32,
    vmctx: Value,
) {
    // First we need to get the `VMRuntimeLimits`.
    let limits = builder.ins().load(
        pointer_type,
        MemFlags::trusted(),
        vmctx,
        i32::try_from(vm_runtime_limits_offset).unwrap(),
    );

    // Then store our current stack pointer into the appropriate slot.
    let sp = builder.ins().get_stack_pointer(pointer_type);
    builder.ins().store(
        MemFlags::trusted(),
        sp,
        limits,
        ptr_size.vmruntime_limits_last_wasm_entry_sp(),
    );
}

fn save_last_wasm_exit_fp_and_pc(
    builder: &mut FunctionBuilder,
    pointer_type: ir::Type,
    ptr: &impl PtrSize,
    limits: Value,
) {
    // Save the exit Wasm FP to the limits. We dereference the current FP to get
    // the previous FP because the current FP is the trampoline's FP, and we
    // want the Wasm function's FP, which is the caller of this trampoline.
    let trampoline_fp = builder.ins().get_frame_pointer(pointer_type);
    let wasm_fp = builder.ins().load(
        pointer_type,
        MemFlags::trusted(),
        trampoline_fp,
        // The FP always points to the next older FP for all supported
        // targets. See assertion in
        // `crates/runtime/src/traphandlers/backtrace.rs`.
        0,
    );
    builder.ins().store(
        MemFlags::trusted(),
        wasm_fp,
        limits,
        ptr.vmruntime_limits_last_wasm_exit_fp(),
    );
    // Finally save the Wasm return address to the limits.
    let wasm_pc = builder.ins().get_return_address(pointer_type);
    builder.ins().store(
        MemFlags::trusted(),
        wasm_pc,
        limits,
        ptr.vmruntime_limits_last_wasm_exit_pc(),
    );
}
