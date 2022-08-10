//! Compilation support for the component model.

use crate::compiler::{Compiler, CompilerContext};
use crate::obj::ModuleTextBuilder;
use crate::CompiledFunction;
use anyhow::Result;
use cranelift_codegen::ir::{self, InstBuilder, MemFlags};
use cranelift_frontend::FunctionBuilder;
use object::write::Object;
use std::any::Any;
use std::ops::Range;
use wasmtime_environ::component::{
    AlwaysTrapInfo, CanonicalOptions, Component, ComponentCompiler, ComponentTypes, FixedEncoding,
    FunctionInfo, LowerImport, LoweredIndex, RuntimeAlwaysTrapIndex, RuntimeMemoryIndex,
    RuntimeTranscoderIndex, Transcode, Transcoder, VMComponentOffsets,
};
use wasmtime_environ::{PrimaryMap, PtrSize, SignatureIndex, Trampoline, TrapCode, WasmFuncType};

impl ComponentCompiler for Compiler {
    fn compile_lowered_trampoline(
        &self,
        component: &Component,
        lowering: &LowerImport,
        types: &ComponentTypes,
    ) -> Result<Box<dyn Any + Send>> {
        let ty = &types[lowering.canonical_abi];
        let isa = &*self.isa;
        let pointer_type = isa.pointer_type();
        let offsets = VMComponentOffsets::new(isa.pointer_bytes(), component);

        let CompilerContext {
            mut func_translator,
            codegen_context: mut context,
            mut incremental_cache_ctx,
        } = self.take_context();

        context.func = ir::Function::with_name_signature(
            ir::UserFuncName::user(0, 0),
            crate::indirect_signature(isa, ty),
        );

        let mut builder = FunctionBuilder::new(&mut context.func, func_translator.context());
        let block0 = builder.create_block();

        // Start off by spilling all the wasm arguments into a stack slot to be
        // passed to the host function.
        let (values_vec_ptr_val, values_vec_len) =
            self.wasm_to_host_spill_args(ty, &mut builder, block0);
        let vmctx = builder.func.dfg.block_params(block0)[0];

        // Save the exit FP and return address for stack walking purposes.
        self.save_last_wasm_fp_and_pc(&mut builder, &offsets, vmctx);

        // Below this will incrementally build both the signature of the host
        // function we're calling as well as the list of arguments since the
        // list is somewhat long.
        let mut callee_args = Vec::new();
        let mut host_sig = ir::Signature::new(crate::wasmtime_call_conv(isa));

        let CanonicalOptions {
            instance,
            memory,
            realloc,
            post_return,
            string_encoding,
        } = lowering.options;

        // vmctx: *mut VMComponentContext
        host_sig.params.push(ir::AbiParam::new(pointer_type));
        callee_args.push(vmctx);

        // data: *mut u8,
        host_sig.params.push(ir::AbiParam::new(pointer_type));
        callee_args.push(builder.ins().load(
            pointer_type,
            MemFlags::trusted(),
            vmctx,
            i32::try_from(offsets.lowering_data(lowering.index)).unwrap(),
        ));

        // flags: *mut VMGlobalDefinition
        host_sig.params.push(ir::AbiParam::new(pointer_type));
        callee_args.push(
            builder
                .ins()
                .iadd_imm(vmctx, i64::from(offsets.instance_flags(instance))),
        );

        // memory: *mut VMMemoryDefinition
        host_sig.params.push(ir::AbiParam::new(pointer_type));
        callee_args.push(match memory {
            Some(idx) => builder.ins().load(
                pointer_type,
                MemFlags::trusted(),
                vmctx,
                i32::try_from(offsets.runtime_memory(idx)).unwrap(),
            ),
            None => builder.ins().iconst(pointer_type, 0),
        });

        // realloc: *mut VMCallerCheckedAnyfunc
        host_sig.params.push(ir::AbiParam::new(pointer_type));
        callee_args.push(match realloc {
            Some(idx) => builder.ins().load(
                pointer_type,
                MemFlags::trusted(),
                vmctx,
                i32::try_from(offsets.runtime_realloc(idx)).unwrap(),
            ),
            None => builder.ins().iconst(pointer_type, 0),
        });

        // A post-return option is only valid on `canon.lift`'d functions so no
        // valid component should have this specified for a lowering which this
        // trampoline compiler is interested in.
        assert!(post_return.is_none());

        // string_encoding: StringEncoding
        host_sig.params.push(ir::AbiParam::new(ir::types::I8));
        callee_args.push(
            builder
                .ins()
                .iconst(ir::types::I8, i64::from(string_encoding as u8)),
        );

        // storage: *mut ValRaw
        host_sig.params.push(ir::AbiParam::new(pointer_type));
        callee_args.push(values_vec_ptr_val);

        // storage_len: usize
        host_sig.params.push(ir::AbiParam::new(pointer_type));
        callee_args.push(
            builder
                .ins()
                .iconst(pointer_type, i64::from(values_vec_len)),
        );

        // Load host function pointer from the vmcontext and then call that
        // indirect function pointer with the list of arguments.
        let host_fn = builder.ins().load(
            pointer_type,
            MemFlags::trusted(),
            vmctx,
            i32::try_from(offsets.lowering_callee(lowering.index)).unwrap(),
        );
        let host_sig = builder.import_signature(host_sig);
        builder.ins().call_indirect(host_sig, host_fn, &callee_args);

        // After the host function has returned the results are loaded from
        // `values_vec_ptr_val` and then returned.
        self.wasm_to_host_load_results(ty, &mut builder, values_vec_ptr_val);

        let func: CompiledFunction =
            self.finish_trampoline(&mut context, incremental_cache_ctx.as_mut(), isa)?;
        self.save_context(CompilerContext {
            func_translator,
            codegen_context: context,
            incremental_cache_ctx,
        });
        Ok(Box::new(func))
    }

    fn compile_always_trap(&self, ty: &WasmFuncType) -> Result<Box<dyn Any + Send>> {
        let isa = &*self.isa;
        let CompilerContext {
            mut func_translator,
            codegen_context: mut context,
            mut incremental_cache_ctx,
        } = self.take_context();
        context.func = ir::Function::with_name_signature(
            ir::UserFuncName::user(0, 0),
            crate::indirect_signature(isa, ty),
        );
        let mut builder = FunctionBuilder::new(&mut context.func, func_translator.context());
        let block0 = builder.create_block();
        builder.append_block_params_for_function_params(block0);
        builder.switch_to_block(block0);
        builder.seal_block(block0);
        builder
            .ins()
            .trap(ir::TrapCode::User(super::ALWAYS_TRAP_CODE));
        builder.finalize();

        let func: CompiledFunction =
            self.finish_trampoline(&mut context, incremental_cache_ctx.as_mut(), isa)?;
        self.save_context(CompilerContext {
            func_translator,
            codegen_context: context,
            incremental_cache_ctx,
        });
        Ok(Box::new(func))
    }

    fn compile_transcoder(
        &self,
        component: &Component,
        transcoder: &Transcoder,
        types: &ComponentTypes,
    ) -> Result<Box<dyn Any + Send>> {
        let ty = &types[transcoder.signature];
        let isa = &*self.isa;
        let offsets = VMComponentOffsets::new(isa.pointer_bytes(), component);

        let CompilerContext {
            mut func_translator,
            codegen_context: mut context,
            mut incremental_cache_ctx,
        } = self.take_context();

        context.func = ir::Function::with_name_signature(
            ir::UserFuncName::user(0, 0),
            crate::indirect_signature(isa, ty),
        );

        let mut builder = FunctionBuilder::new(&mut context.func, func_translator.context());
        let block0 = builder.create_block();
        builder.append_block_params_for_function_params(block0);
        builder.switch_to_block(block0);
        builder.seal_block(block0);

        self.translate_transcode(&mut builder, &offsets, transcoder, block0);

        let func: CompiledFunction =
            self.finish_trampoline(&mut context, incremental_cache_ctx.as_mut(), isa)?;
        self.save_context(CompilerContext {
            func_translator,
            codegen_context: context,
            incremental_cache_ctx,
        });
        Ok(Box::new(func))
    }

    fn emit_obj(
        &self,
        lowerings: PrimaryMap<LoweredIndex, Box<dyn Any + Send>>,
        always_trap: PrimaryMap<RuntimeAlwaysTrapIndex, Box<dyn Any + Send>>,
        transcoders: PrimaryMap<RuntimeTranscoderIndex, Box<dyn Any + Send>>,
        trampolines: Vec<(SignatureIndex, Box<dyn Any + Send>)>,
        obj: &mut Object<'static>,
    ) -> Result<(
        PrimaryMap<LoweredIndex, FunctionInfo>,
        PrimaryMap<RuntimeAlwaysTrapIndex, AlwaysTrapInfo>,
        PrimaryMap<RuntimeTranscoderIndex, FunctionInfo>,
        Vec<Trampoline>,
    )> {
        let module = Default::default();
        let mut text = ModuleTextBuilder::new(obj, &module, &*self.isa);

        let range2info = |range: Range<u64>| FunctionInfo {
            start: u32::try_from(range.start).unwrap(),
            length: u32::try_from(range.end - range.start).unwrap(),
        };
        let ret_lowerings = lowerings
            .iter()
            .map(|(i, lowering)| {
                let lowering = lowering.downcast_ref::<CompiledFunction>().unwrap();
                assert!(lowering.traps.is_empty());
                let range = text.named_func(
                    &format!("_wasm_component_lowering_trampoline{}", i.as_u32()),
                    &lowering,
                );
                range2info(range)
            })
            .collect();
        let ret_always_trap = always_trap
            .iter()
            .map(|(i, func)| {
                let func = func.downcast_ref::<CompiledFunction>().unwrap();
                assert_eq!(func.traps.len(), 1);
                assert_eq!(func.traps[0].trap_code, TrapCode::AlwaysTrapAdapter);
                let name = format!("_wasmtime_always_trap{}", i.as_u32());
                let range = text.named_func(&name, func);
                AlwaysTrapInfo {
                    info: range2info(range),
                    trap_offset: func.traps[0].code_offset,
                }
            })
            .collect();

        let ret_transcoders = transcoders
            .iter()
            .map(|(i, func)| {
                let func = func.downcast_ref::<CompiledFunction>().unwrap();
                let name = format!("_wasmtime_transcoder{}", i.as_u32());
                let range = text.named_func(&name, func);
                range2info(range)
            })
            .collect();

        let ret_trampolines = trampolines
            .iter()
            .map(|(i, func)| {
                let func = func.downcast_ref::<CompiledFunction>().unwrap();
                assert!(func.traps.is_empty());
                text.trampoline(*i, func)
            })
            .collect();

        text.finish()?;

        Ok((
            ret_lowerings,
            ret_always_trap,
            ret_transcoders,
            ret_trampolines,
        ))
    }
}

impl Compiler {
    fn save_last_wasm_fp_and_pc(
        &self,
        builder: &mut FunctionBuilder<'_>,
        offsets: &VMComponentOffsets<u8>,
        vmctx: ir::Value,
    ) {
        let pointer_type = self.isa.pointer_type();
        // First we need to get the `VMRuntimeLimits`.
        let limits = builder.ins().load(
            pointer_type,
            MemFlags::trusted(),
            vmctx,
            i32::try_from(offsets.limits()).unwrap(),
        );
        // Then save the exit Wasm FP to the limits. We dereference the current
        // FP to get the previous FP because the current FP is the trampoline's
        // FP, and we want the Wasm function's FP, which is the caller of this
        // trampoline.
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
            offsets.ptr.vmruntime_limits_last_wasm_exit_fp(),
        );
        // Finally save the Wasm return address to the limits.
        let wasm_pc = builder.ins().get_return_address(pointer_type);
        builder.ins().store(
            MemFlags::trusted(),
            wasm_pc,
            limits,
            offsets.ptr.vmruntime_limits_last_wasm_exit_pc(),
        );
    }

    fn translate_transcode(
        &self,
        builder: &mut FunctionBuilder<'_>,
        offsets: &VMComponentOffsets<u8>,
        transcoder: &Transcoder,
        block: ir::Block,
    ) {
        let pointer_type = self.isa.pointer_type();
        let vmctx = builder.func.dfg.block_params(block)[0];

        // Save the exit FP and return address for stack walking purposes. This
        // is used when an invalid encoding is encountered and a trap is raised.
        self.save_last_wasm_fp_and_pc(builder, &offsets, vmctx);

        // Determine the static signature of the host libcall for this transcode
        // operation and additionally calculate the static offset within the
        // transode libcalls array.
        let func = &mut builder.func;
        let (sig, offset) = match transcoder.op {
            Transcode::Copy(FixedEncoding::Utf8) => host::utf8_to_utf8(self, func),
            Transcode::Copy(FixedEncoding::Utf16) => host::utf16_to_utf16(self, func),
            Transcode::Copy(FixedEncoding::Latin1) => host::latin1_to_latin1(self, func),
            Transcode::Latin1ToUtf16 => host::latin1_to_utf16(self, func),
            Transcode::Latin1ToUtf8 => host::latin1_to_utf8(self, func),
            Transcode::Utf16ToCompactProbablyUtf16 => {
                host::utf16_to_compact_probably_utf16(self, func)
            }
            Transcode::Utf16ToCompactUtf16 => host::utf16_to_compact_utf16(self, func),
            Transcode::Utf16ToLatin1 => host::utf16_to_latin1(self, func),
            Transcode::Utf16ToUtf8 => host::utf16_to_utf8(self, func),
            Transcode::Utf8ToCompactUtf16 => host::utf8_to_compact_utf16(self, func),
            Transcode::Utf8ToLatin1 => host::utf8_to_latin1(self, func),
            Transcode::Utf8ToUtf16 => host::utf8_to_utf16(self, func),
        };

        // Load the host function pointer for this transcode which comes from a
        // function pointer within the VMComponentContext's libcall array.
        let transcode_libcalls_array = builder.ins().load(
            pointer_type,
            MemFlags::trusted(),
            vmctx,
            i32::try_from(offsets.transcode_libcalls()).unwrap(),
        );
        let transcode_libcall = builder.ins().load(
            pointer_type,
            MemFlags::trusted(),
            transcode_libcalls_array,
            i32::try_from(offset * u32::from(offsets.ptr.size())).unwrap(),
        );

        // Load the base pointers for the from/to linear memories.
        let from_base = self.load_runtime_memory_base(builder, vmctx, offsets, transcoder.from);
        let to_base = self.load_runtime_memory_base(builder, vmctx, offsets, transcoder.to);

        // Helper function to cast a core wasm input to a host pointer type
        // which will go into the host libcall.
        let cast_to_pointer = |builder: &mut FunctionBuilder<'_>, val: ir::Value, is64: bool| {
            let host64 = pointer_type == ir::types::I64;
            if is64 == host64 {
                val
            } else if !is64 {
                assert!(host64);
                builder.ins().uextend(pointer_type, val)
            } else {
                assert!(!host64);
                builder.ins().ireduce(pointer_type, val)
            }
        };

        // Helper function to cast an input parameter to the host pointer type.
        let len_param = |builder: &mut FunctionBuilder<'_>, param: usize, is64: bool| {
            let val = builder.func.dfg.block_params(block)[2 + param];
            cast_to_pointer(builder, val, is64)
        };

        // Helper function to interpret an input parameter as a pointer into
        // linear memory. This will cast the input parameter to the host integer
        // type and then add that value to the base.
        //
        // Note that bounds-checking happens in adapter modules, and this
        // trampoline is simply calling the host libcall.
        let ptr_param =
            |builder: &mut FunctionBuilder<'_>, param: usize, is64: bool, base: ir::Value| {
                let val = len_param(builder, param, is64);
                builder.ins().iadd(base, val)
            };

        let Transcoder { to64, from64, .. } = *transcoder;
        let mut args = Vec::new();

        // Most transcoders share roughly the same signature despite doing very
        // different things internally, so most libcalls are lumped together
        // here.
        match transcoder.op {
            Transcode::Copy(_)
            | Transcode::Latin1ToUtf16
            | Transcode::Utf16ToCompactProbablyUtf16
            | Transcode::Utf8ToLatin1
            | Transcode::Utf16ToLatin1
            | Transcode::Utf8ToUtf16 => {
                args.push(ptr_param(builder, 0, from64, from_base));
                args.push(len_param(builder, 1, from64));
                args.push(ptr_param(builder, 2, to64, to_base));
            }

            Transcode::Utf16ToUtf8 | Transcode::Latin1ToUtf8 => {
                args.push(ptr_param(builder, 0, from64, from_base));
                args.push(len_param(builder, 1, from64));
                args.push(ptr_param(builder, 2, to64, to_base));
                args.push(len_param(builder, 3, to64));
            }

            Transcode::Utf8ToCompactUtf16 | Transcode::Utf16ToCompactUtf16 => {
                args.push(ptr_param(builder, 0, from64, from_base));
                args.push(len_param(builder, 1, from64));
                args.push(ptr_param(builder, 2, to64, to_base));
                args.push(len_param(builder, 3, to64));
                args.push(len_param(builder, 4, to64));
            }
        };
        let call = builder.ins().call_indirect(sig, transcode_libcall, &args);
        let results = builder.func.dfg.inst_results(call).to_vec();
        let mut raw_results = Vec::new();

        // Helper to cast a host pointer integer type to the destination type.
        let cast_from_pointer = |builder: &mut FunctionBuilder<'_>, val: ir::Value, is64: bool| {
            let host64 = pointer_type == ir::types::I64;
            if is64 == host64 {
                val
            } else if !is64 {
                assert!(host64);
                builder.ins().ireduce(ir::types::I32, val)
            } else {
                assert!(!host64);
                builder.ins().uextend(ir::types::I64, val)
            }
        };

        // Like the arguments the results are fairly similar across libcalls, so
        // they're lumped into various buckets here.
        match transcoder.op {
            Transcode::Copy(_) | Transcode::Latin1ToUtf16 => {}

            Transcode::Utf8ToUtf16
            | Transcode::Utf16ToCompactProbablyUtf16
            | Transcode::Utf8ToCompactUtf16
            | Transcode::Utf16ToCompactUtf16 => {
                raw_results.push(cast_from_pointer(builder, results[0], to64));
            }

            Transcode::Latin1ToUtf8
            | Transcode::Utf16ToUtf8
            | Transcode::Utf8ToLatin1
            | Transcode::Utf16ToLatin1 => {
                raw_results.push(cast_from_pointer(builder, results[0], from64));
                raw_results.push(cast_from_pointer(builder, results[1], to64));
            }
        };

        builder.ins().return_(&raw_results);
        builder.finalize();
    }

    fn load_runtime_memory_base(
        &self,
        builder: &mut FunctionBuilder<'_>,
        vmctx: ir::Value,
        offsets: &VMComponentOffsets<u8>,
        mem: RuntimeMemoryIndex,
    ) -> ir::Value {
        let pointer_type = self.isa.pointer_type();
        let from_vmmemory_definition = builder.ins().load(
            pointer_type,
            MemFlags::trusted(),
            vmctx,
            i32::try_from(offsets.runtime_memory(mem)).unwrap(),
        );
        builder.ins().load(
            pointer_type,
            MemFlags::trusted(),
            from_vmmemory_definition,
            i32::from(offsets.ptr.vmmemory_definition_base()),
        )
    }
}

/// Module with macro-generated contents that will return the signature and
/// offset for each of the host transcoder functions.
///
/// Note that a macro is used here to keep this in sync with the actual
/// transcoder functions themselves which are also defined via a macro.
#[allow(unused_mut)]
mod host {
    use crate::compiler::Compiler;
    use cranelift_codegen::ir::{self, AbiParam};

    macro_rules! host_transcode {
        (
            $(
                $( #[$attr:meta] )*
                $name:ident( $( $pname:ident: $param:ident ),* ) $( -> $result:ident )?;
            )*
        ) => {
            $(
                pub(super) fn $name(compiler: &Compiler, func: &mut ir::Function) -> (ir::SigRef, u32) {
                    let pointer_type = compiler.isa.pointer_type();
                    let params = vec![
                        $( AbiParam::new(host_transcode!(@ty pointer_type $param)) ),*
                    ];
                    let mut returns = Vec::new();
                    $(host_transcode!(@push_return pointer_type params returns $result);)?
                    let sig = func.import_signature(ir::Signature {
                        params,
                        returns,
                        call_conv: crate::wasmtime_call_conv(&*compiler.isa),
                    });

                    (sig, offsets::$name)
                }
            )*
        };

        (@ty $ptr:ident size) => ($ptr);
        (@ty $ptr:ident ptr_u8) => ($ptr);
        (@ty $ptr:ident ptr_u16) => ($ptr);

        (@push_return $ptr:ident $params:ident $returns:ident size) => ($returns.push(AbiParam::new($ptr)););
        (@push_return $ptr:ident $params:ident $returns:ident size_pair) => ({
            $returns.push(AbiParam::new($ptr));
            $returns.push(AbiParam::new($ptr));
        });
    }

    wasmtime_environ::foreach_transcoder!(host_transcode);

    mod offsets {
        macro_rules! offsets {
            (
                $(
                    $( #[$attr:meta] )*
                    $name:ident($($t:tt)*) $( -> $result:ident )?;
                )*
            ) => {
                offsets!(@declare (0) $($name)*);
            };

            (@declare ($n:expr)) => ();
            (@declare ($n:expr) $name:ident $($rest:tt)*) => (
                pub static $name: u32 = $n;
                offsets!(@declare ($n + 1) $($rest)*);
            );
        }

        wasmtime_environ::foreach_transcoder!(offsets);
    }
}
