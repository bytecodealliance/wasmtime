use crate::debug::DwarfSectionRelocTarget;
use crate::func_environ::FuncEnvironment;
use crate::DEBUG_ASSERT_TRAP_CODE;
use crate::{array_call_signature, CompiledFunction, ModuleTextBuilder};
use crate::{builder::LinkOptions, wasm_call_signature, BuiltinFunctionSignatures};
use anyhow::{Context as _, Result};
use cranelift_codegen::binemit::CodeOffset;
use cranelift_codegen::bitset::CompoundBitSet;
use cranelift_codegen::ir::{self, InstBuilder, MemFlags, UserExternalName, UserFuncName, Value};
use cranelift_codegen::isa::{
    unwind::{UnwindInfo, UnwindInfoKind},
    OwnedTargetIsa, TargetIsa,
};
use cranelift_codegen::print_errors::pretty_error;
use cranelift_codegen::{CompiledCode, Context};
use cranelift_entity::PrimaryMap;
use cranelift_frontend::FunctionBuilder;
use cranelift_wasm::{DefinedFuncIndex, FuncTranslator, WasmFuncType, WasmValType};
use object::write::{Object, StandardSegment, SymbolId};
use object::{RelocationEncoding, RelocationFlags, RelocationKind, SectionKind};
use std::any::Any;
use std::cmp;
use std::collections::HashMap;
use std::mem;
use std::path;
use std::sync::{Arc, Mutex};
use wasmparser::{FuncValidatorAllocations, FunctionBody};
use wasmtime_environ::{
    AddressMapSection, BuiltinFunctionIndex, CacheStore, CompileError, FlagValue, FunctionBodyData,
    FunctionLoc, ModuleTranslation, ModuleTypesBuilder, PtrSize, RelocationTarget,
    StackMapInformation, StaticModuleIndex, TrapEncodingBuilder, Tunables, VMOffsets,
    WasmFunctionInfo,
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
pub struct Compiler {
    tunables: Tunables,
    contexts: Mutex<Vec<CompilerContext>>,
    isa: OwnedTargetIsa,
    linkopts: LinkOptions,
    cache_store: Option<Arc<dyn CacheStore>>,
    clif_dir: Option<path::PathBuf>,
    wmemcheck: bool,
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
    pub fn new(
        tunables: Tunables,
        isa: OwnedTargetIsa,
        cache_store: Option<Arc<dyn CacheStore>>,
        linkopts: LinkOptions,
        clif_dir: Option<path::PathBuf>,
        wmemcheck: bool,
    ) -> Compiler {
        Compiler {
            contexts: Default::default(),
            tunables,
            isa,
            linkopts,
            cache_store,
            clif_dir,
            wmemcheck,
        }
    }
}

impl wasmtime_environ::Compiler for Compiler {
    fn compile_function(
        &self,
        translation: &ModuleTranslation<'_>,
        func_index: DefinedFuncIndex,
        input: FunctionBodyData<'_>,
        types: &ModuleTypesBuilder,
    ) -> Result<(WasmFunctionInfo, Box<dyn Any + Send>), CompileError> {
        let isa = &*self.isa;
        let module = &translation.module;
        let func_index = module.func_index(func_index);
        let sig = translation.module.functions[func_index].signature;
        let wasm_func_ty = types[sig].unwrap_func();

        let mut compiler = self.function_compiler();

        let context = &mut compiler.cx.codegen_context;
        context.func.signature = wasm_call_signature(isa, wasm_func_ty, &self.tunables);
        context.func.name = UserFuncName::User(UserExternalName {
            namespace: crate::NS_WASM_FUNC,
            index: func_index.as_u32(),
        });

        if self.tunables.generate_native_debuginfo {
            context.func.collect_debug_info();
        }

        let mut func_env = FuncEnvironment::new(
            isa,
            translation,
            types,
            &self.tunables,
            self.wmemcheck,
            wasm_func_ty,
        );

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
            offset: i32::from(func_env.offsets.ptr.vmctx_runtime_limits()).into(),
            global_type: isa.pointer_type(),
            flags: MemFlags::trusted().with_readonly(),
        });
        let stack_limit = context.func.create_global_value(ir::GlobalValueData::Load {
            base: interrupts_ptr,
            offset: i32::try_from(func_env.offsets.ptr.vmruntime_limits_stack_limit())
                .unwrap()
                .into(),
            global_type: isa.pointer_type(),
            flags: MemFlags::trusted(),
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

        let (info, func) = compiler.finish_with_info(Some((&body, &self.tunables)))?;

        let timing = cranelift_codegen::timing::take_current();
        log::debug!("{:?} translated in {:?}", func_index, timing.total());
        log::trace!("{:?} timing info\n{}", func_index, timing);

        Ok((info, Box::new(func)))
    }

    fn compile_array_to_wasm_trampoline(
        &self,
        translation: &ModuleTranslation<'_>,
        types: &ModuleTypesBuilder,
        def_func_index: DefinedFuncIndex,
    ) -> Result<Box<dyn Any + Send>, CompileError> {
        let func_index = translation.module.func_index(def_func_index);
        let sig = translation.module.functions[func_index].signature;
        let wasm_func_ty = types[sig].unwrap_func();

        let isa = &*self.isa;
        let pointer_type = isa.pointer_type();
        let wasm_call_sig = wasm_call_signature(isa, wasm_func_ty, &self.tunables);
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
        let vm_runtime_limits_offset = offsets.ptr.vmctx_runtime_limits();
        save_last_wasm_entry_sp(
            &mut builder,
            pointer_type,
            &offsets.ptr,
            vm_runtime_limits_offset.into(),
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

    fn compile_wasm_to_array_trampoline(
        &self,
        wasm_func_ty: &WasmFuncType,
    ) -> Result<Box<dyn Any + Send>, CompileError> {
        let isa = &*self.isa;
        let pointer_type = isa.pointer_type();
        let wasm_call_sig = wasm_call_signature(isa, wasm_func_ty, &self.tunables);
        let array_call_sig = array_call_signature(isa);

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
        let ptr = isa.pointer_bytes();
        let limits = builder.ins().load(
            pointer_type,
            MemFlags::trusted(),
            caller_vmctx,
            i32::try_from(ptr.vmcontext_runtime_limits()).unwrap(),
        );
        save_last_wasm_exit_fp_and_pc(&mut builder, pointer_type, &ptr, limits);

        // Spill all wasm arguments to the stack in `ValRaw` slots.
        let (args_base, args_len) =
            self.allocate_stack_array_and_spill_args(wasm_func_ty, &mut builder, &args[2..]);
        let args_len = builder.ins().iconst(pointer_type, i64::from(args_len));

        // Load the actual callee out of the
        // `VMArrayCallHostFuncContext::host_func`.
        let ptr_size = isa.pointer_bytes();
        let callee = builder.ins().load(
            pointer_type,
            MemFlags::trusted(),
            callee_vmctx,
            ptr_size.vmarray_call_host_func_context_func_ref() + ptr_size.vm_func_ref_array_call(),
        );

        // Do an indirect call to the callee.
        let callee_signature = builder.func.import_signature(array_call_sig);
        builder.ins().call_indirect(
            callee_signature,
            callee,
            &[callee_vmctx, caller_vmctx, args_base, args_len],
        );

        let results =
            self.load_values_from_array(wasm_func_ty.returns(), &mut builder, args_base, args_len);
        builder.ins().return_(&results);
        builder.finalize();

        Ok(Box::new(compiler.finish()?))
    }

    fn append_code(
        &self,
        obj: &mut Object<'static>,
        funcs: &[(String, Box<dyn Any + Send>)],
        resolve_reloc: &dyn Fn(usize, RelocationTarget) -> usize,
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
            let (sym, range) = builder.append_func(&sym, func, |idx| resolve_reloc(i, idx));
            if self.tunables.generate_address_map {
                let addr = func.address_map();
                addrs.push(range.clone(), &addr.instructions);
            }
            traps.push(range.clone(), &func.traps().collect::<Vec<_>>());
            builder.append_padding(self.linkopts.padding_between_functions);
            let info = FunctionLoc {
                start: u32::try_from(range.start).unwrap(),
                length: u32::try_from(range.end - range.start).unwrap(),
            };
            ret.push((sym, info));
        }

        builder.finish();

        if self.tunables.generate_address_map {
            addrs.append_to(obj);
        }
        traps.append_to(obj);

        Ok(ret)
    }

    fn triple(&self) -> &target_lexicon::Triple {
        self.isa.triple()
    }

    fn flags(&self) -> Vec<(&'static str, FlagValue<'static>)> {
        crate::clif_flags_to_wasmtime(self.isa.flags().iter())
    }

    fn isa_flags(&self) -> Vec<(&'static str, FlagValue<'static>)> {
        crate::clif_flags_to_wasmtime(self.isa.isa_flags())
    }

    fn is_branch_protection_enabled(&self) -> bool {
        self.isa.is_branch_protection_enabled()
    }

    #[cfg(feature = "component-model")]
    fn component_compiler(&self) -> &dyn wasmtime_environ::component::ComponentCompiler {
        self
    }

    fn append_dwarf<'a>(
        &self,
        obj: &mut Object<'_>,
        translations: &'a PrimaryMap<StaticModuleIndex, ModuleTranslation<'a>>,
        get_func: &'a dyn Fn(
            StaticModuleIndex,
            DefinedFuncIndex,
        ) -> (SymbolId, &'a (dyn Any + Send)),
        dwarf_package_bytes: Option<&'a [u8]>,
        tunables: &'a Tunables,
    ) -> Result<()> {
        let get_func = move |m, f| {
            let (sym, any) = get_func(m, f);
            (
                sym,
                any.downcast_ref::<CompiledFunction>().unwrap().metadata(),
            )
        };
        let mut compilation = crate::debug::Compilation::new(
            &*self.isa,
            translations,
            &get_func,
            dwarf_package_bytes,
            tunables,
        );
        let dwarf_sections = crate::debug::emit_dwarf(&*self.isa, &mut compilation)
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
                    DwarfSectionRelocTarget::Func(id) => compilation.symbol_id(id),
                    DwarfSectionRelocTarget::Section(name) => {
                        obj.section_symbol(dwarf_sections_ids[name])
                    }
                };
                obj.add_relocation(
                    section_id,
                    object::write::Relocation {
                        offset: u64::from(reloc.offset),
                        symbol: target_symbol,
                        addend: i64::from(reloc.addend),
                        flags: RelocationFlags::Generic {
                            size: reloc.size << 3,
                            kind: RelocationKind::Absolute,
                            encoding: RelocationEncoding::Generic,
                        },
                    },
                )?;
            }
        }

        Ok(())
    }

    fn create_systemv_cie(&self) -> Option<gimli::write::CommonInformationEntry> {
        self.isa.create_systemv_cie()
    }

    fn compile_wasm_to_builtin(
        &self,
        index: BuiltinFunctionIndex,
    ) -> Result<Box<dyn Any + Send>, CompileError> {
        let isa = &*self.isa;
        let ptr_size = isa.pointer_bytes();
        let pointer_type = isa.pointer_type();
        let sig = BuiltinFunctionSignatures::new(isa).signature(index);

        let mut compiler = self.function_compiler();
        let func = ir::Function::with_name_signature(Default::default(), sig.clone());
        let (mut builder, block0) = compiler.builder(func);
        let vmctx = builder.block_params(block0)[0];

        // Debug-assert that this is the right kind of vmctx, and then
        // additionally perform the "routine of the exit trampoline" of saving
        // fp/pc/etc.
        debug_assert_vmctx_kind(isa, &mut builder, vmctx, wasmtime_environ::VMCONTEXT_MAGIC);
        let limits = builder.ins().load(
            pointer_type,
            MemFlags::trusted(),
            vmctx,
            ptr_size.vmcontext_runtime_limits(),
        );
        save_last_wasm_exit_fp_and_pc(&mut builder, pointer_type, &ptr_size, limits);

        // Now it's time to delegate to the actual builtin. Builtins are stored
        // in an array in all `VMContext`s. First load the base pointer of the
        // array and then load the entry of the array that corresponds to this
        // builtin.
        let mem_flags = ir::MemFlags::trusted().with_readonly();
        let array_addr = builder.ins().load(
            pointer_type,
            mem_flags,
            vmctx,
            i32::try_from(ptr_size.vmcontext_builtin_functions()).unwrap(),
        );
        let body_offset = i32::try_from(index.index() * pointer_type.bytes()).unwrap();
        let func_addr = builder
            .ins()
            .load(pointer_type, mem_flags, array_addr, body_offset);

        // Forward all our own arguments to the libcall itself, and then return
        // all the same results as the libcall.
        let block_params = builder.block_params(block0).to_vec();
        let sig = builder.func.import_signature(sig);
        let call = builder.ins().call_indirect(sig, func_addr, &block_params);
        let results = builder.func.dfg.inst_results(call).to_vec();
        builder.ins().return_(&results);
        builder.finalize();

        Ok(Box::new(compiler.finish()?))
    }

    fn compiled_function_relocation_targets<'a>(
        &'a self,
        func: &'a dyn Any,
    ) -> Box<dyn Iterator<Item = RelocationTarget> + 'a> {
        let func = func.downcast_ref::<CompiledFunction>().unwrap();
        Box::new(func.relocations().map(|r| r.reloc_target))
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
        args: &[ir::Value],
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
            4,
        ));
        let values_vec_ptr = builder.ins().stack_addr(pointer_type, slot, 0);

        {
            let values_vec_len = builder
                .ins()
                .iconst(ir::types::I32, i64::try_from(values_vec_len).unwrap());
            self.store_values_to_array(builder, ty.params(), args, values_vec_ptr, values_vec_len);
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
        types: &[WasmValType],
        values: &[Value],
        values_vec_ptr: Value,
        values_vec_capacity: Value,
    ) {
        debug_assert_eq!(types.len(), values.len());
        debug_assert_enough_capacity_for_length(builder, types.len(), values_vec_capacity);

        // Note that loads and stores are unconditionally done in the
        // little-endian format rather than the host's native-endianness,
        // despite this load/store being unrelated to execution in wasm itself.
        // For more details on this see the `ValRaw` type in
        // `wasmtime::runtime::vm`.
        let flags = ir::MemFlags::new()
            .with_notrap()
            .with_endianness(ir::Endianness::Little);

        let value_size = mem::size_of::<u128>();
        for (i, val) in values.iter().copied().enumerate() {
            crate::unbarriered_store_type_at_offset(
                &mut builder.cursor(),
                flags,
                values_vec_ptr,
                i32::try_from(i * value_size).unwrap(),
                val,
            );
        }
    }

    /// Used for loading the values of an array-call host function's value
    /// array.
    ///
    /// This can be used to load arguments out of the array if the trampoline we
    /// are building exposes the array calling convention, or it can be used to
    /// load results out of the array if the trampoline we are building calls a
    /// function that uses the array calling convention.
    fn load_values_from_array(
        &self,
        types: &[WasmValType],
        builder: &mut FunctionBuilder,
        values_vec_ptr: Value,
        values_vec_capacity: Value,
    ) -> Vec<ir::Value> {
        let isa = &*self.isa;
        let value_size = mem::size_of::<u128>();

        debug_assert_enough_capacity_for_length(builder, types.len(), values_vec_capacity);

        // Note that this is little-endian like `store_values_to_array` above,
        // see notes there for more information.
        let flags = MemFlags::new()
            .with_notrap()
            .with_endianness(ir::Endianness::Little);

        let mut results = Vec::new();
        for (i, ty) in types.iter().enumerate() {
            results.push(crate::unbarriered_load_type_at_offset(
                isa,
                &mut builder.cursor(),
                *ty,
                flags,
                values_vec_ptr,
                i32::try_from(i * value_size).unwrap(),
            ));
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
        let (_, _code_buf) =
            compile_maybe_cached(context, isa, self.cx.incremental_cache_ctx.as_mut())?;
        let mut compiled_code = context.take_compiled_code().unwrap();

        // Give wasm functions, user defined code, a "preferred" alignment
        // instead of the minimum alignment as this can help perf in niche
        // situations.
        let preferred_alignment = if body_and_tunables.is_some() {
            self.compiler.isa.function_alignment().preferred
        } else {
            1
        };

        let alignment = compiled_code.buffer.alignment.max(preferred_alignment);
        let mut compiled_function = CompiledFunction::new(
            compiled_code.buffer.clone(),
            context.func.params.user_named_funcs().clone(),
            alignment,
        );

        if let Some((body, tunables)) = body_and_tunables {
            let data = body.get_binary_reader();
            let offset = data.original_position();
            let len = data.bytes_remaining();
            compiled_function.set_address_map(
                offset as u32,
                len as u32,
                tunables.generate_address_map,
            );
        }

        if isa.flags().unwind_info() {
            let unwind = compiled_code
                .create_unwind_info(isa)
                .map_err(|error| CompileError::Codegen(pretty_error(&context.func, error)))?;

            if let Some(unwind_info) = unwind {
                compiled_function.set_unwind_info(unwind_info);
            }
        }

        if body_and_tunables
            .map(|(_, t)| t.generate_native_debuginfo)
            .unwrap_or(false)
        {
            compiled_function.set_value_labels_ranges(compiled_code.value_labels_ranges.clone());

            // DWARF debugging needs the CFA-based unwind information even on Windows.
            if !matches!(
                compiled_function.metadata().unwind_info,
                Some(UnwindInfo::SystemV(_))
            ) {
                let cfa_unwind = compiled_code
                    .create_unwind_info_of_kind(isa, UnwindInfoKind::SystemV)
                    .map_err(|error| CompileError::Codegen(pretty_error(&context.func, error)))?;

                if let Some(UnwindInfo::SystemV(cfa_unwind_info)) = cfa_unwind {
                    compiled_function.set_cfa_unwind_info(cfa_unwind_info);
                }
            }
        }

        let stack_maps =
            clif_to_env_stack_maps(compiled_code.buffer.take_user_stack_maps().into_iter());
        compiled_function
            .set_sized_stack_slots(std::mem::take(&mut context.func.sized_stack_slots));
        self.compiler.contexts.lock().unwrap().push(self.cx);

        Ok((
            WasmFunctionInfo {
                start_srcloc: compiled_function.metadata().address_map.start_srcloc,
                stack_maps: stack_maps.into(),
            },
            compiled_function,
        ))
    }
}

/// Convert from Cranelift's representation of a stack map to Wasmtime's
/// compiler-agnostic representation.
fn clif_to_env_stack_maps(
    clif_stack_maps: impl ExactSizeIterator<Item = (CodeOffset, u32, ir::UserStackMap)>,
) -> Vec<StackMapInformation> {
    let mut stack_maps = Vec::with_capacity(clif_stack_maps.len());
    for (code_offset, mapped_bytes, stack_map) in clif_stack_maps {
        let mut bitset = CompoundBitSet::new();
        for (ty, offset) in stack_map.entries() {
            assert_eq!(ty, ir::types::I32);
            bitset.insert(usize::try_from(offset).unwrap());
        }
        if bitset.is_empty() {
            continue;
        }
        let stack_map = wasmtime_environ::StackMap::new(mapped_bytes, bitset);
        stack_maps.push(StackMapInformation {
            code_offset,
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
            namespace: crate::NS_WASM_FUNC,
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
        // `crates/wasmtime/src/runtime/vm/traphandlers/backtrace.rs`.
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
