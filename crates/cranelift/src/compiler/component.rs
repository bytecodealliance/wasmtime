//! Compilation support for the component model.

use crate::{compiler::Compiler, TRAP_ALWAYS, TRAP_CANNOT_ENTER, TRAP_INTERNAL_ASSERT};
use anyhow::Result;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{self, InstBuilder, MemFlags, Value};
use cranelift_codegen::isa::{CallConv, TargetIsa};
use cranelift_frontend::FunctionBuilder;
use std::any::Any;
use wasmtime_environ::component::*;
use wasmtime_environ::fact::SYNC_ENTER_FIXED_PARAMS;
use wasmtime_environ::{
    HostCall, ModuleInternedTypeIndex, PtrSize, TrapSentinel, Tunables, WasmFuncType, WasmValType,
};

struct TrampolineCompiler<'a> {
    compiler: &'a Compiler,
    isa: &'a (dyn TargetIsa + 'static),
    builder: FunctionBuilder<'a>,
    component: &'a Component,
    types: &'a ComponentTypesBuilder,
    offsets: VMComponentOffsets<u8>,
    abi: Abi,
    block0: ir::Block,
    signature: ModuleInternedTypeIndex,
    tunables: &'a Tunables,
}

#[derive(Debug, Copy, Clone)]
enum Abi {
    Wasm,
    Array,
}

type GetLibcallFn =
    fn(&dyn TargetIsa, &mut ir::Function) -> (ir::SigRef, ComponentBuiltinFunctionIndex);

impl<'a> TrampolineCompiler<'a> {
    fn new(
        compiler: &'a Compiler,
        func_compiler: &'a mut super::FunctionCompiler<'_>,
        component: &'a Component,
        types: &'a ComponentTypesBuilder,
        index: TrampolineIndex,
        abi: Abi,
        tunables: &'a Tunables,
    ) -> TrampolineCompiler<'a> {
        let isa = &*compiler.isa;
        let signature = component.trampolines[index];
        let ty = types[signature].unwrap_func();
        let func = ir::Function::with_name_signature(
            ir::UserFuncName::user(0, 0),
            match abi {
                Abi::Wasm => crate::wasm_call_signature(isa, ty, &compiler.tunables),
                Abi::Array => crate::array_call_signature(isa),
            },
        );
        let (builder, block0) = func_compiler.builder(func);
        TrampolineCompiler {
            compiler,
            isa,
            builder,
            component,
            types,
            offsets: VMComponentOffsets::new(isa.pointer_bytes(), component),
            abi,
            block0,
            signature,
            tunables,
        }
    }

    fn translate(&mut self, trampoline: &Trampoline) {
        match trampoline {
            Trampoline::Transcoder {
                op,
                from,
                from64,
                to,
                to64,
            } => {
                match self.abi {
                    Abi::Wasm => {
                        self.translate_transcode(*op, *from, *from64, *to, *to64);
                    }
                    // Transcoders can only actually be called by Wasm, so let's assert
                    // that here.
                    Abi::Array => {
                        self.builder.ins().trap(TRAP_INTERNAL_ASSERT);
                    }
                }
            }
            Trampoline::LowerImport {
                index,
                options,
                lower_ty,
            } => {
                self.translate_lower_import(*index, options, *lower_ty);
            }
            Trampoline::AlwaysTrap => {
                self.translate_always_trap();
            }
            Trampoline::ResourceNew(ty) => self.translate_resource_new(*ty),
            Trampoline::ResourceRep(ty) => self.translate_resource_rep(*ty),
            Trampoline::ResourceDrop(ty) => self.translate_resource_drop(*ty),
            Trampoline::BackpressureSet { instance } => {
                self.translate_backpressure_set_call(*instance)
            }
            Trampoline::TaskReturn { results, options } => {
                self.translate_task_return_call(*results, options)
            }
            Trampoline::WaitableSetNew { instance } => self.translate_waitable_set_new(*instance),
            Trampoline::WaitableSetWait {
                instance,
                async_,
                memory,
            } => self.translate_task_wait_or_poll_call(
                *instance,
                *async_,
                *memory,
                host::waitable_set_wait,
            ),
            Trampoline::WaitableSetPoll {
                instance,
                async_,
                memory,
            } => self.translate_task_wait_or_poll_call(
                *instance,
                *async_,
                *memory,
                host::waitable_set_poll,
            ),
            Trampoline::WaitableSetDrop { instance } => self.translate_waitable_set_drop(*instance),
            Trampoline::WaitableJoin { instance } => self.translate_waitable_join(*instance),
            Trampoline::Yield { async_ } => self.translate_yield_call(*async_),
            Trampoline::SubtaskDrop { instance } => self.translate_subtask_drop_call(*instance),
            Trampoline::StreamNew { ty } => self.translate_future_or_stream_call(
                &[ty.as_u32()],
                None,
                host::stream_new,
                TrapSentinel::NegativeOne,
            ),
            Trampoline::StreamRead { ty, options } => {
                let tys = &[ty.as_u32()];
                if let Some(info) = self.flat_stream_element_info(*ty).cloned() {
                    self.translate_flat_stream_call(tys, options, host::flat_stream_read, &info)
                } else {
                    self.translate_future_or_stream_call(
                        tys,
                        Some(options),
                        host::stream_read,
                        TrapSentinel::NegativeOne,
                    )
                }
            }
            Trampoline::StreamWrite { ty, options } => {
                let tys = &[ty.as_u32()];
                if let Some(info) = self.flat_stream_element_info(*ty).cloned() {
                    self.translate_flat_stream_call(tys, options, host::flat_stream_write, &info)
                } else {
                    self.translate_future_or_stream_call(
                        tys,
                        Some(options),
                        host::stream_write,
                        TrapSentinel::NegativeOne,
                    )
                }
            }
            Trampoline::StreamCancelRead { ty, async_ } => {
                self.translate_cancel_call(ty.as_u32(), *async_, host::stream_cancel_read)
            }
            Trampoline::StreamCancelWrite { ty, async_ } => {
                self.translate_cancel_call(ty.as_u32(), *async_, host::stream_cancel_write)
            }
            Trampoline::StreamCloseReadable { ty } => self.translate_future_or_stream_call(
                &[ty.as_u32()],
                None,
                host::stream_close_readable,
                TrapSentinel::Falsy,
            ),
            Trampoline::StreamCloseWritable { ty } => self.translate_future_or_stream_call(
                &[ty.as_u32()],
                None,
                host::stream_close_writable,
                TrapSentinel::Falsy,
            ),
            Trampoline::FutureNew { ty } => self.translate_future_or_stream_call(
                &[ty.as_u32()],
                None,
                host::future_new,
                TrapSentinel::NegativeOne,
            ),
            Trampoline::FutureRead { ty, options } => self.translate_future_or_stream_call(
                &[ty.as_u32()],
                Some(&options),
                host::future_read,
                TrapSentinel::NegativeOne,
            ),
            Trampoline::FutureWrite { ty, options } => self.translate_future_or_stream_call(
                &[ty.as_u32()],
                Some(options),
                host::future_write,
                TrapSentinel::NegativeOne,
            ),
            Trampoline::FutureCancelRead { ty, async_ } => {
                self.translate_cancel_call(ty.as_u32(), *async_, host::future_cancel_read)
            }
            Trampoline::FutureCancelWrite { ty, async_ } => {
                self.translate_cancel_call(ty.as_u32(), *async_, host::future_cancel_write)
            }
            Trampoline::FutureCloseReadable { ty } => self.translate_future_or_stream_call(
                &[ty.as_u32()],
                None,
                host::future_close_readable,
                TrapSentinel::Falsy,
            ),
            Trampoline::FutureCloseWritable { ty } => self.translate_future_or_stream_call(
                &[ty.as_u32()],
                None,
                host::future_close_writable,
                TrapSentinel::Falsy,
            ),
            Trampoline::ErrorContextNew { ty, options } => self.translate_error_context_call(
                *ty,
                options,
                host::error_context_new,
                TrapSentinel::NegativeOne,
            ),
            Trampoline::ErrorContextDebugMessage { ty, options } => self
                .translate_error_context_call(
                    *ty,
                    options,
                    host::error_context_debug_message,
                    TrapSentinel::Falsy,
                ),
            Trampoline::ErrorContextDrop { ty } => self.translate_error_context_drop_call(*ty),
            Trampoline::ResourceTransferOwn => {
                self.translate_host_libcall(host::resource_transfer_own, |me, rets| {
                    rets[0] = me.raise_if_negative_one_and_truncate(rets[0]);
                })
            }
            Trampoline::ResourceTransferBorrow => {
                self.translate_host_libcall(host::resource_transfer_borrow, |me, rets| {
                    rets[0] = me.raise_if_negative_one_and_truncate(rets[0]);
                })
            }
            Trampoline::ResourceEnterCall => {
                self.translate_host_libcall(host::resource_enter_call, |_, _| {})
            }
            Trampoline::ResourceExitCall => {
                self.translate_host_libcall(host::resource_exit_call, |me, rets| {
                    me.raise_if_host_trapped(rets.pop().unwrap());
                })
            }
            Trampoline::SyncEnterCall => self.translate_sync_enter(),
            Trampoline::SyncExitCall { callback } => self.translate_sync_exit(*callback),
            Trampoline::AsyncEnterCall => {
                self.translate_async_enter_or_exit(host::async_enter, None, TrapSentinel::Falsy)
            }
            Trampoline::AsyncExitCall {
                callback,
                post_return,
            } => self.translate_async_enter_or_exit(
                host::async_exit,
                Some((*callback, *post_return)),
                TrapSentinel::NegativeOne,
            ),
            Trampoline::FutureTransfer => {
                self.translate_host_libcall(host::future_transfer, |me, rets| {
                    rets[0] = me.raise_if_negative_one_and_truncate(rets[0]);
                })
            }
            Trampoline::StreamTransfer => {
                self.translate_host_libcall(host::stream_transfer, |me, rets| {
                    rets[0] = me.raise_if_negative_one_and_truncate(rets[0]);
                })
            }
            Trampoline::ErrorContextTransfer => {
                self.translate_host_libcall(host::error_context_transfer, |me, rets| {
                    rets[0] = me.raise_if_negative_one_and_truncate(rets[0]);
                })
            }
        }
    }

    /// Determine whether the specified type can be optimized as a stream
    /// payload by lifting and lowering with a simple `memcpy`.
    ///
    /// Any type containing only "flat", primitive data (i.e. no pointers or
    /// handles) should qualify for this optimization, but it's also okay to
    /// conservatively return `None` here; the fallback slow path will always
    /// work -- it just won't be as efficient.
    fn flat_stream_element_info(&self, ty: TypeStreamTableIndex) -> Option<&CanonicalAbiInfo> {
        let payload = self.types[self.types[ty].ty].payload;
        match payload {
            None => Some(&CanonicalAbiInfo::ZERO),
            Some(
                payload @ (InterfaceType::Bool
                | InterfaceType::S8
                | InterfaceType::U8
                | InterfaceType::S16
                | InterfaceType::U16
                | InterfaceType::S32
                | InterfaceType::U32
                | InterfaceType::S64
                | InterfaceType::U64
                | InterfaceType::Float32
                | InterfaceType::Float64
                | InterfaceType::Char),
            ) => Some(self.types.canonical_abi(&payload)),
            // TODO: Recursively check for other "flat" types (i.e. those without pointers or handles),
            // e.g. `record`s, `variant`s, etc. which contain only flat types.
            _ => None,
        }
    }

    fn store_wasm_arguments(&mut self, args: &[Value]) -> (Value, Value) {
        let pointer_type = self.isa.pointer_type();
        let wasm_func_ty = &self.types[self.signature].unwrap_func();

        // Start off by spilling all the wasm arguments into a stack slot to be
        // passed to the host function.
        match self.abi {
            Abi::Wasm => {
                let (ptr, len) = self.compiler.allocate_stack_array_and_spill_args(
                    wasm_func_ty,
                    &mut self.builder,
                    args,
                );
                let len = self.builder.ins().iconst(pointer_type, i64::from(len));
                (ptr, len)
            }
            Abi::Array => {
                let params = self.builder.func.dfg.block_params(self.block0);
                (params[2], params[3])
            }
        }
    }

    fn translate_intrinsic_libcall(
        &mut self,
        vmctx: ir::Value,
        get_libcall: GetLibcallFn,
        args: &[ir::Value],
        sentinel: TrapSentinel,
    ) {
        match self.abi {
            Abi::Wasm => {}

            Abi::Array => {
                // TODO: A guest could hypothetically export the same intrinsic
                // it imported, allowing the host to call it directly.  We need
                // to support that here (except for `sync-enter`, `sync-exit`,
                // `async-enter`, and `async-exit`, which are only ever called
                // from FACT-generated Wasm code and never exported).
                //
                // https://github.com/bytecodealliance/wasmtime/issues/10143
                self.builder.ins().trap(TRAP_INTERNAL_ASSERT);
                return;
            }
        }

        let call = self.call_libcall(vmctx, get_libcall, args);

        let result = self.builder.func.dfg.inst_results(call)[0];
        let result_ty = self.builder.func.dfg.value_type(result);
        let expected = &self.builder.func.signature.returns;
        match sentinel {
            TrapSentinel::NegativeOne => {
                assert_eq!(expected.len(), 1);
                let result = match (result_ty, expected[0].value_type) {
                    (ir::types::I64, ir::types::I32) => {
                        self.raise_if_negative_one_and_truncate(result)
                    }
                    (ir::types::I64, ir::types::I64) => self.raise_if_negative_one(result),
                    other => panic!("unsupported NegativeOne combo {other:?}"),
                };
                self.abi_store_results(&[result]);
            }
            TrapSentinel::Falsy => {
                assert_eq!(expected.len(), 0);
                self.raise_if_host_trapped(result);
                self.builder.ins().return_(&[]);
            }
            _ => todo!("support additional return types if/when necessary"),
        }
    }

    fn translate_task_return_call(&mut self, results: TypeTupleIndex, options: &CanonicalOptions) {
        // FIXME(#10338) shouldn't ignore options here.
        let _ = options;
        let args = self.builder.func.dfg.block_params(self.block0).to_vec();
        let vmctx = args[0];

        let (values_vec_ptr, values_vec_len) = self.store_wasm_arguments(&args[2..]);

        let ty = self
            .builder
            .ins()
            .iconst(ir::types::I32, i64::from(results.as_u32()));

        self.translate_intrinsic_libcall(
            vmctx,
            host::task_return,
            &[vmctx, ty, values_vec_ptr, values_vec_len],
            TrapSentinel::Falsy,
        );
    }

    fn translate_waitable_set_new(&mut self, instance: RuntimeComponentInstanceIndex) {
        let args = self.builder.func.dfg.block_params(self.block0).to_vec();
        let vmctx = args[0];

        let instance = self
            .builder
            .ins()
            .iconst(ir::types::I32, i64::from(instance.as_u32()));

        self.translate_intrinsic_libcall(
            vmctx,
            host::waitable_set_new,
            &[vmctx, instance],
            TrapSentinel::NegativeOne,
        );
    }

    fn translate_waitable_set_drop(&mut self, instance: RuntimeComponentInstanceIndex) {
        let args = self.builder.func.dfg.block_params(self.block0).to_vec();
        let vmctx = args[0];
        let set = args[2];

        let instance = self
            .builder
            .ins()
            .iconst(ir::types::I32, i64::from(instance.as_u32()));

        self.translate_intrinsic_libcall(
            vmctx,
            host::waitable_set_drop,
            &[vmctx, instance, set],
            TrapSentinel::Falsy,
        );
    }

    fn translate_waitable_join(&mut self, instance: RuntimeComponentInstanceIndex) {
        let args = self.builder.func.dfg.block_params(self.block0).to_vec();
        let vmctx = args[0];
        let set = args[2];
        let waitable = args[3];

        let instance = self
            .builder
            .ins()
            .iconst(ir::types::I32, i64::from(instance.as_u32()));

        self.translate_intrinsic_libcall(
            vmctx,
            host::waitable_join,
            &[vmctx, instance, set, waitable],
            TrapSentinel::Falsy,
        );
    }

    fn translate_sync_enter(&mut self) {
        match self.abi {
            Abi::Wasm => {}

            Abi::Array => {
                // This code can only be called from (FACT-generated) Wasm, so
                // we don't need to support the array ABI.
                self.builder.ins().trap(TRAP_INTERNAL_ASSERT);
                return;
            }
        }

        let args = self.builder.func.dfg.block_params(self.block0).to_vec();
        let vmctx = args[0];

        let pointer_type = self.isa.pointer_type();
        let wasm_func_ty = &self.types[self.signature].unwrap_func();

        let param_offset = SYNC_ENTER_FIXED_PARAMS.len();
        let spill_offset = param_offset + 2;

        let (values_vec_ptr, len) = self.compiler.allocate_stack_array_and_spill_args(
            &WasmFuncType::new(
                wasm_func_ty
                    .params()
                    .iter()
                    .skip(param_offset)
                    .copied()
                    .collect(),
                Box::new([]),
            ),
            &mut self.builder,
            &args[spill_offset..],
        );
        let values_vec_len = self.builder.ins().iconst(pointer_type, i64::from(len));

        let mut callee_args = vec![vmctx];

        // remaining non-Wasm parameters
        callee_args.extend(args[2..spill_offset].iter().copied());

        callee_args.push(values_vec_ptr);
        callee_args.push(values_vec_len);

        self.translate_intrinsic_libcall(
            vmctx,
            host::sync_enter,
            &callee_args,
            TrapSentinel::Falsy,
        );
    }

    fn translate_sync_exit(&mut self, callback: Option<RuntimeCallbackIndex>) {
        match self.abi {
            Abi::Wasm => {}

            Abi::Array => {
                // This code can only be called from (FACT-generated) Wasm, so
                // we don't need to support the array ABI.
                self.builder.ins().trap(TRAP_INTERNAL_ASSERT);
                return;
            }
        }

        let pointer_type = self.isa.pointer_type();
        let args = self.builder.func.dfg.block_params(self.block0).to_vec();
        let vmctx = args[0];
        let wasm_func_ty = &self.types[self.signature].unwrap_func();

        let mut callee_args = vec![vmctx, self.load_callback(vmctx, callback)];

        // remaining non-Wasm parameters
        callee_args.extend(args[2..].iter().copied());

        let (values_vec_ptr, len) = self.compiler.allocate_stack_array_and_spill_args(
            &WasmFuncType::new(
                Box::new([]),
                wasm_func_ty.returns().iter().copied().collect(),
            ),
            &mut self.builder,
            &[],
        );
        let values_vec_len = self.builder.ins().iconst(pointer_type, i64::from(len));

        callee_args.push(values_vec_ptr);
        callee_args.push(values_vec_len);

        let call = self.call_libcall(vmctx, host::sync_exit, &callee_args);

        let succeeded = self.builder.func.dfg.inst_results(call)[0];
        self.raise_if_host_trapped(succeeded);
        // After the host function has returned the results are loaded from
        // `values_vec_ptr` and then returned.
        let results = self.compiler.load_values_from_array(
            wasm_func_ty.returns(),
            &mut self.builder,
            values_vec_ptr,
            values_vec_len,
        );
        self.builder.ins().return_(&results);
    }

    fn translate_async_enter_or_exit(
        &mut self,
        get_libcall: GetLibcallFn,
        callback_and_post_return: Option<(
            Option<RuntimeCallbackIndex>,
            Option<RuntimePostReturnIndex>,
        )>,
        sentinel: TrapSentinel,
    ) {
        match self.abi {
            Abi::Wasm => {}

            Abi::Array => {
                // This code can only be called from (FACT-generated) Wasm, so
                // we don't need to support the array ABI.
                self.builder.ins().trap(TRAP_INTERNAL_ASSERT);
                return;
            }
        }

        let args = self.builder.func.dfg.block_params(self.block0).to_vec();
        let vmctx = args[0];

        let mut callee_args = vec![vmctx];

        if let Some((callback, post_return)) = callback_and_post_return {
            // callback: *mut VMFuncRef
            callee_args.push(self.load_callback(vmctx, callback));
            // post_return: *mut VMFuncRef
            callee_args.push(self.load_post_return(vmctx, post_return));
        }

        // remaining parameters
        callee_args.extend(args[2..].iter().copied());

        self.translate_intrinsic_libcall(vmctx, get_libcall, &callee_args, sentinel);
    }

    fn translate_backpressure_set_call(&mut self, caller_instance: RuntimeComponentInstanceIndex) {
        let args = self.builder.func.dfg.block_params(self.block0).to_vec();
        let vmctx = args[0];

        let mut callee_args = vec![
            vmctx,
            self.builder
                .ins()
                .iconst(ir::types::I32, i64::from(caller_instance.as_u32())),
        ];

        callee_args.extend(args[2..].iter().copied());

        self.translate_intrinsic_libcall(
            vmctx,
            host::backpressure_set,
            &callee_args,
            TrapSentinel::Falsy,
        );
    }

    fn translate_task_wait_or_poll_call(
        &mut self,
        caller_instance: RuntimeComponentInstanceIndex,
        async_: bool,
        memory: RuntimeMemoryIndex,
        get_libcall: GetLibcallFn,
    ) {
        let args = self.builder.func.dfg.block_params(self.block0).to_vec();
        let vmctx = args[0];

        let mut callee_args = vec![
            vmctx,
            self.builder
                .ins()
                .iconst(ir::types::I32, i64::from(caller_instance.as_u32())),
            self.builder
                .ins()
                .iconst(ir::types::I8, if async_ { 1 } else { 0 }),
            self.load_memory(vmctx, memory),
        ];

        callee_args.extend(args[2..].iter().copied());

        self.translate_intrinsic_libcall(
            vmctx,
            get_libcall,
            &callee_args,
            TrapSentinel::NegativeOne,
        );
    }

    fn translate_yield_call(&mut self, async_: bool) {
        let args = self.builder.func.dfg.block_params(self.block0).to_vec();
        let vmctx = args[0];

        let callee_args = [
            vmctx,
            self.builder
                .ins()
                .iconst(ir::types::I8, if async_ { 1 } else { 0 }),
        ];

        self.translate_intrinsic_libcall(vmctx, host::yield_, &callee_args, TrapSentinel::Falsy);
    }

    fn translate_subtask_drop_call(&mut self, caller_instance: RuntimeComponentInstanceIndex) {
        let args = self.builder.func.dfg.block_params(self.block0).to_vec();
        let vmctx = args[0];

        let mut callee_args = vec![
            vmctx,
            self.builder
                .ins()
                .iconst(ir::types::I32, i64::from(caller_instance.as_u32())),
        ];

        callee_args.extend(args[2..].iter().copied());

        self.translate_intrinsic_libcall(
            vmctx,
            host::subtask_drop,
            &callee_args,
            TrapSentinel::Falsy,
        );
    }

    fn translate_lower_import(
        &mut self,
        index: LoweredIndex,
        options: &CanonicalOptions,
        lower_ty: TypeFuncIndex,
    ) {
        let pointer_type = self.isa.pointer_type();
        let args = self.builder.func.dfg.block_params(self.block0).to_vec();
        let vmctx = args[0];
        let wasm_func_ty = self.types[self.signature].unwrap_func();

        let (values_vec_ptr, values_vec_len) = self.store_wasm_arguments(&args[2..]);

        // Below this will incrementally build both the signature of the host
        // function we're calling as well as the list of arguments since the
        // list is somewhat long.
        let mut callee_args = Vec::new();
        let mut host_sig = ir::Signature::new(CallConv::triple_default(self.isa.triple()));

        let CanonicalOptions {
            instance,
            memory,
            realloc,
            callback,
            post_return,
            string_encoding,
            async_,
        } = *options;

        assert!(callback.is_none());

        // vmctx: *mut VMComponentContext
        host_sig.params.push(ir::AbiParam::new(pointer_type));
        callee_args.push(vmctx);

        // data: *mut u8,
        host_sig.params.push(ir::AbiParam::new(pointer_type));
        callee_args.push(self.builder.ins().load(
            pointer_type,
            MemFlags::trusted(),
            vmctx,
            i32::try_from(self.offsets.lowering_data(index)).unwrap(),
        ));

        // ty: TypeFuncIndex,
        host_sig.params.push(ir::AbiParam::new(ir::types::I32));
        callee_args.push(
            self.builder
                .ins()
                .iconst(ir::types::I32, i64::from(lower_ty.as_u32())),
        );

        // caller_instance: RuntimeComponentInstanceIndex
        host_sig.params.push(ir::AbiParam::new(ir::types::I32));
        callee_args.push(
            self.builder
                .ins()
                .iconst(ir::types::I32, i64::from(instance.as_u32())),
        );

        // flags: *mut VMGlobalDefinition
        host_sig.params.push(ir::AbiParam::new(pointer_type));
        callee_args.push(
            self.builder
                .ins()
                .iadd_imm(vmctx, i64::from(self.offsets.instance_flags(instance))),
        );

        // memory: *mut VMMemoryDefinition
        host_sig.params.push(ir::AbiParam::new(pointer_type));
        callee_args.push(match memory {
            Some(idx) => self.load_memory(vmctx, idx),
            None => self.builder.ins().iconst(pointer_type, 0),
        });

        // realloc: *mut VMFuncRef
        host_sig.params.push(ir::AbiParam::new(pointer_type));
        callee_args.push(self.load_realloc(vmctx, realloc));

        // A post-return option is only valid on `canon.lift`'d functions so no
        // valid component should have this specified for a lowering which this
        // trampoline compiler is interested in.
        assert!(post_return.is_none());

        // string_encoding: StringEncoding
        host_sig.params.push(ir::AbiParam::new(ir::types::I8));
        callee_args.push(self.string_encoding(string_encoding));

        // async_: bool
        host_sig.params.push(ir::AbiParam::new(ir::types::I8));
        callee_args.push(
            self.builder
                .ins()
                .iconst(ir::types::I8, if async_ { 1 } else { 0 }),
        );

        // storage: *mut ValRaw
        host_sig.params.push(ir::AbiParam::new(pointer_type));
        callee_args.push(values_vec_ptr);

        // storage_len: usize
        host_sig.params.push(ir::AbiParam::new(pointer_type));
        callee_args.push(values_vec_len);

        // return value is a bool whether a trap was raised or not
        host_sig.returns.push(ir::AbiParam::new(ir::types::I8));

        // Load host function pointer from the vmcontext and then call that
        // indirect function pointer with the list of arguments.
        let host_fn = self.builder.ins().load(
            pointer_type,
            MemFlags::trusted(),
            vmctx,
            i32::try_from(self.offsets.lowering_callee(index)).unwrap(),
        );
        let host_sig = self.builder.import_signature(host_sig);
        let call = self.compiler.call_indirect_host(
            &mut self.builder,
            HostCall::ComponentLowerImport,
            host_sig,
            host_fn,
            &callee_args,
        );
        let succeeded = self.builder.func.dfg.inst_results(call)[0];

        match self.abi {
            Abi::Wasm => {
                self.raise_if_host_trapped(succeeded);
                // After the host function has returned the results are loaded from
                // `values_vec_ptr` and then returned.
                let results = self.compiler.load_values_from_array(
                    wasm_func_ty.returns(),
                    &mut self.builder,
                    values_vec_ptr,
                    values_vec_len,
                );
                self.builder.ins().return_(&results);
            }
            Abi::Array => {
                self.builder.ins().return_(&[succeeded]);
            }
        }
    }

    fn translate_always_trap(&mut self) {
        if self.tunables.signals_based_traps {
            self.builder.ins().trap(TRAP_ALWAYS);
            return;
        }

        let args = self.abi_load_params();
        let vmctx = args[0];

        let (host_sig, index) = host::trap(self.isa, &mut self.builder.func);
        let host_fn = self.load_libcall(vmctx, index);

        let code = self.builder.ins().iconst(
            ir::types::I8,
            i64::from(wasmtime_environ::Trap::AlwaysTrapAdapter as u8),
        );
        self.compiler.call_indirect_host(
            &mut self.builder,
            index,
            host_sig,
            host_fn,
            &[vmctx, code],
        );
        let succeeded = self.builder.ins().iconst(ir::types::I8, 0);
        self.raise_if_host_trapped(succeeded);
        // debug trap in case execution actually falls through, but this
        // shouldn't ever get hit at runtime.
        self.builder.ins().trap(TRAP_INTERNAL_ASSERT);
    }

    fn translate_resource_new(&mut self, resource: TypeResourceTableIndex) {
        let args = self.abi_load_params();
        let vmctx = args[0];

        // The arguments this shim passes along to the libcall are:
        //
        //   * the vmctx
        //   * a constant value for this `ResourceNew` intrinsic
        //   * the wasm argument to wrap
        let mut host_args = Vec::new();
        host_args.push(vmctx);
        host_args.push(
            self.builder
                .ins()
                .iconst(ir::types::I32, i64::from(resource.as_u32())),
        );
        host_args.push(args[2]);

        // Currently this only support resources represented by `i32`
        assert_eq!(
            self.types[self.signature].unwrap_func().params()[0],
            WasmValType::I32
        );
        let call = self.call_libcall(vmctx, host::resource_new32, &host_args);
        let result = self.builder.func.dfg.inst_results(call)[0];
        let result = self.raise_if_negative_one_and_truncate(result);
        self.abi_store_results(&[result]);
    }

    fn translate_resource_rep(&mut self, resource: TypeResourceTableIndex) {
        let args = self.abi_load_params();
        let vmctx = args[0];

        // The arguments this shim passes along to the libcall are:
        //
        //   * the vmctx
        //   * a constant value for this `ResourceRep` intrinsic
        //   * the wasm argument to unwrap
        let mut host_args = Vec::new();
        host_args.push(vmctx);
        host_args.push(
            self.builder
                .ins()
                .iconst(ir::types::I32, i64::from(resource.as_u32())),
        );
        host_args.push(args[2]);

        // Currently this only support resources represented by `i32`
        assert_eq!(
            self.types[self.signature].unwrap_func().returns()[0],
            WasmValType::I32
        );
        let call = self.call_libcall(vmctx, host::resource_rep32, &host_args);
        let result = self.builder.func.dfg.inst_results(call)[0];
        let result = self.raise_if_negative_one_and_truncate(result);
        self.abi_store_results(&[result]);
    }

    fn translate_resource_drop(&mut self, resource: TypeResourceTableIndex) {
        let args = self.abi_load_params();
        let vmctx = args[0];
        let caller_vmctx = args[1];
        let pointer_type = self.isa.pointer_type();

        // The arguments this shim passes along to the libcall are:
        //
        //   * the vmctx
        //   * a constant value for this `ResourceDrop` intrinsic
        //   * the wasm handle index to drop
        let mut host_args = Vec::new();
        host_args.push(vmctx);
        host_args.push(
            self.builder
                .ins()
                .iconst(ir::types::I32, i64::from(resource.as_u32())),
        );
        host_args.push(args[2]);

        let call = self.call_libcall(vmctx, host::resource_drop, &host_args);
        let should_run_destructor = self.builder.func.dfg.inst_results(call)[0];

        // Immediately raise a trap if requested by the host
        let minus_one = self.builder.ins().iconst(ir::types::I64, -1);
        let succeeded = self
            .builder
            .ins()
            .icmp(IntCC::NotEqual, should_run_destructor, minus_one);
        self.raise_if_host_trapped(succeeded);

        let resource_ty = self.types[resource].ty;
        let resource_def = self
            .component
            .defined_resource_index(resource_ty)
            .map(|idx| {
                self.component
                    .initializers
                    .iter()
                    .filter_map(|i| match i {
                        GlobalInitializer::Resource(r) if r.index == idx => Some(r),
                        _ => None,
                    })
                    .next()
                    .unwrap()
            });
        let has_destructor = match resource_def {
            Some(def) => def.dtor.is_some(),
            None => true,
        };
        // Synthesize the following:
        //
        //      ...
        //      brif should_run_destructor, run_destructor_block, return_block
        //
        //    run_destructor_block:
        //      ;; test may_enter, but only if the component instances
        //      ;; differ
        //      flags = load.i32 vmctx+$offset
        //      masked = band flags, $FLAG_MAY_ENTER
        //      trapz masked, CANNOT_ENTER_CODE
        //
        //      ;; ============================================================
        //      ;; this is conditionally emitted based on whether the resource
        //      ;; has a destructor or not, and can be statically omitted
        //      ;; because that information is known at compile time here.
        //      rep = ushr.i64 rep, 1
        //      rep = ireduce.i32 rep
        //      dtor = load.ptr vmctx+$offset
        //      func_addr = load.ptr dtor+$offset
        //      callee_vmctx = load.ptr dtor+$offset
        //      call_indirect func_addr, callee_vmctx, vmctx, rep
        //      ;; ============================================================
        //
        //      jump return_block
        //
        //    return_block:
        //      return
        //
        // This will decode `should_run_destructor` and run the destructor
        // funcref if one is specified for this resource. Note that not all
        // resources have destructors, hence the null check.
        self.builder.ensure_inserted_block();
        let current_block = self.builder.current_block().unwrap();
        let run_destructor_block = self.builder.create_block();
        self.builder
            .insert_block_after(run_destructor_block, current_block);
        let return_block = self.builder.create_block();
        self.builder
            .insert_block_after(return_block, run_destructor_block);

        self.builder.ins().brif(
            should_run_destructor,
            run_destructor_block,
            &[],
            return_block,
            &[],
        );

        let trusted = ir::MemFlags::trusted().with_readonly();

        self.builder.switch_to_block(run_destructor_block);

        // If this is a defined resource within the component itself then a
        // check needs to be emitted for the `may_enter` flag. Note though
        // that this check can be elided if the resource table resides in
        // the same component instance that defined the resource as the
        // component is calling itself.
        if let Some(def) = resource_def {
            if self.types[resource].instance != def.instance {
                let flags = self.builder.ins().load(
                    ir::types::I32,
                    trusted,
                    vmctx,
                    i32::try_from(self.offsets.instance_flags(def.instance)).unwrap(),
                );
                let masked = self
                    .builder
                    .ins()
                    .band_imm(flags, i64::from(FLAG_MAY_ENTER));
                self.builder.ins().trapz(masked, TRAP_CANNOT_ENTER);
            }
        }

        // Conditionally emit destructor-execution code based on whether we
        // statically know that a destructor exists or not.
        if has_destructor {
            let rep = self.builder.ins().ushr_imm(should_run_destructor, 1);
            let rep = self.builder.ins().ireduce(ir::types::I32, rep);
            let index = self.types[resource].ty;
            // NB: despite the vmcontext storing nullable funcrefs for function
            // pointers we know this is statically never null due to the
            // `has_destructor` check above.
            let dtor_func_ref = self.builder.ins().load(
                pointer_type,
                trusted,
                vmctx,
                i32::try_from(self.offsets.resource_destructor(index)).unwrap(),
            );
            if cfg!(debug_assertions) {
                self.builder
                    .ins()
                    .trapz(dtor_func_ref, TRAP_INTERNAL_ASSERT);
            }
            let func_addr = self.builder.ins().load(
                pointer_type,
                trusted,
                dtor_func_ref,
                i32::from(self.offsets.ptr.vm_func_ref_wasm_call()),
            );
            let callee_vmctx = self.builder.ins().load(
                pointer_type,
                trusted,
                dtor_func_ref,
                i32::from(self.offsets.ptr.vm_func_ref_vmctx()),
            );

            let sig = crate::wasm_call_signature(
                self.isa,
                &self.types[self.signature].unwrap_func(),
                &self.compiler.tunables,
            );
            let sig_ref = self.builder.import_signature(sig);

            // NB: note that the "caller" vmctx here is the caller of this
            // intrinsic itself, not the `VMComponentContext`. This effectively
            // takes ourselves out of the chain here but that's ok since the
            // caller is only used for store/limits and that same info is
            // stored, but elsewhere, in the component context.
            self.builder.ins().call_indirect(
                sig_ref,
                func_addr,
                &[callee_vmctx, caller_vmctx, rep],
            );
        }
        self.builder.ins().jump(return_block, &[]);
        self.builder.seal_block(run_destructor_block);

        self.builder.switch_to_block(return_block);
        self.builder.seal_block(return_block);
        self.abi_store_results(&[]);
    }

    /// Invokes a host libcall and returns the result.
    ///
    /// Only intended for simple trampolines and effectively acts as a bridge
    /// from the wasm abi to host.
    fn translate_host_libcall(
        &mut self,
        get_libcall: GetLibcallFn,
        handle_results: fn(&mut Self, &mut Vec<ir::Value>),
    ) {
        match self.abi {
            Abi::Wasm => {}

            // These trampolines can only actually be called by Wasm, so
            // let's assert that here.
            Abi::Array => {
                self.builder.ins().trap(TRAP_INTERNAL_ASSERT);
                return;
            }
        }

        let args = self.builder.func.dfg.block_params(self.block0).to_vec();
        let vmctx = args[0];
        let mut host_args = vec![vmctx];
        host_args.extend(args[2..].iter().copied());

        let call = self.call_libcall(vmctx, get_libcall, &host_args);
        let mut results = self.builder.func.dfg.inst_results(call).to_vec();
        handle_results(self, &mut results);
        self.builder.ins().return_(&results);
    }

    fn translate_cancel_call(&mut self, ty: u32, async_: bool, get_libcall: GetLibcallFn) {
        let args = self.builder.func.dfg.block_params(self.block0).to_vec();
        let vmctx = args[0];
        let mut callee_args = vec![
            vmctx,
            self.builder.ins().iconst(ir::types::I32, i64::from(ty)),
            self.builder
                .ins()
                .iconst(ir::types::I8, if async_ { 1 } else { 0 }),
        ];

        callee_args.extend(args[2..].iter().copied());

        self.translate_intrinsic_libcall(
            vmctx,
            get_libcall,
            &callee_args,
            TrapSentinel::NegativeOne,
        );
    }

    fn load_memory(&mut self, vmctx: ir::Value, memory: RuntimeMemoryIndex) -> ir::Value {
        self.builder.ins().load(
            self.isa.pointer_type(),
            MemFlags::trusted(),
            vmctx,
            i32::try_from(self.offsets.runtime_memory(memory)).unwrap(),
        )
    }

    fn load_realloc(
        &mut self,
        vmctx: ir::Value,
        realloc: Option<RuntimeReallocIndex>,
    ) -> ir::Value {
        let pointer_type = self.isa.pointer_type();
        match realloc {
            Some(idx) => self.builder.ins().load(
                pointer_type,
                MemFlags::trusted(),
                vmctx,
                i32::try_from(self.offsets.runtime_realloc(idx)).unwrap(),
            ),
            None => self.builder.ins().iconst(pointer_type, 0),
        }
    }

    fn load_callback(
        &mut self,
        vmctx: ir::Value,
        callback: Option<RuntimeCallbackIndex>,
    ) -> ir::Value {
        let pointer_type = self.isa.pointer_type();
        match callback {
            Some(idx) => self.builder.ins().load(
                pointer_type,
                MemFlags::trusted(),
                vmctx,
                i32::try_from(self.offsets.runtime_callback(idx)).unwrap(),
            ),
            None => self.builder.ins().iconst(pointer_type, 0),
        }
    }

    fn load_post_return(
        &mut self,
        vmctx: ir::Value,
        post_return: Option<RuntimePostReturnIndex>,
    ) -> ir::Value {
        let pointer_type = self.isa.pointer_type();
        match post_return {
            Some(idx) => self.builder.ins().load(
                pointer_type,
                MemFlags::trusted(),
                vmctx,
                i32::try_from(self.offsets.runtime_post_return(idx)).unwrap(),
            ),
            None => self.builder.ins().iconst(pointer_type, 0),
        }
    }

    fn string_encoding(&mut self, string_encoding: StringEncoding) -> ir::Value {
        self.builder
            .ins()
            .iconst(ir::types::I8, i64::from(string_encoding as u8))
    }

    fn translate_future_or_stream_call(
        &mut self,
        tys: &[u32],
        options: Option<&CanonicalOptions>,
        get_libcall: GetLibcallFn,
        sentinel: TrapSentinel,
    ) {
        let args = self.builder.func.dfg.block_params(self.block0).to_vec();
        let vmctx = args[0];
        let mut callee_args = vec![vmctx];

        if let Some(options) = options {
            // memory: *mut VMMemoryDefinition
            callee_args.push(self.load_memory(vmctx, options.memory.unwrap()));
            // realloc: *mut VMFuncRef
            callee_args.push(self.load_realloc(vmctx, options.realloc));
            // string_encoding: StringEncoding
            callee_args.push(self.string_encoding(options.string_encoding))
        }

        for ty in tys {
            callee_args.push(self.builder.ins().iconst(ir::types::I32, i64::from(*ty)));
        }

        callee_args.extend(args[2..].iter().copied());

        self.translate_intrinsic_libcall(vmctx, get_libcall, &callee_args, sentinel);
    }

    fn translate_flat_stream_call(
        &mut self,
        tys: &[u32],
        options: &CanonicalOptions,
        get_libcall: GetLibcallFn,
        info: &CanonicalAbiInfo,
    ) {
        let args = self.builder.func.dfg.block_params(self.block0).to_vec();
        let vmctx = args[0];
        let mut callee_args = vec![
            vmctx,
            self.load_memory(vmctx, options.memory.unwrap()),
            self.load_realloc(vmctx, options.realloc),
        ];
        for ty in tys {
            callee_args.push(self.builder.ins().iconst(ir::types::I32, i64::from(*ty)));
        }

        callee_args.extend([
            self.builder
                .ins()
                .iconst(ir::types::I32, i64::from(info.size32)),
            self.builder
                .ins()
                .iconst(ir::types::I32, i64::from(info.align32)),
        ]);

        callee_args.extend(args[2..].iter().copied());

        self.translate_intrinsic_libcall(
            vmctx,
            get_libcall,
            &callee_args,
            TrapSentinel::NegativeOne,
        );
    }

    fn translate_error_context_call(
        &mut self,
        ty: TypeComponentLocalErrorContextTableIndex,
        options: &CanonicalOptions,
        get_libcall: GetLibcallFn,
        sentinel: TrapSentinel,
    ) {
        let args = self.builder.func.dfg.block_params(self.block0).to_vec();
        let vmctx = args[0];
        let mut callee_args = vec![
            vmctx,
            self.load_memory(vmctx, options.memory.unwrap()),
            self.load_realloc(vmctx, options.realloc),
            self.string_encoding(options.string_encoding),
            self.builder
                .ins()
                .iconst(ir::types::I32, i64::from(ty.as_u32())),
        ];

        callee_args.extend(args[2..].iter().copied());

        self.translate_intrinsic_libcall(vmctx, get_libcall, &callee_args, sentinel);
    }

    fn translate_error_context_drop_call(&mut self, ty: TypeComponentLocalErrorContextTableIndex) {
        let args = self.builder.func.dfg.block_params(self.block0).to_vec();
        let vmctx = args[0];
        let mut callee_args = vec![
            vmctx,
            self.builder
                .ins()
                .iconst(ir::types::I32, i64::from(ty.as_u32())),
        ];

        callee_args.extend(args[2..].iter().copied());

        self.translate_intrinsic_libcall(
            vmctx,
            host::error_context_drop,
            &callee_args,
            TrapSentinel::Falsy,
        );
    }

    /// Loads a host function pointer for a libcall stored at the `offset`
    /// provided in the libcalls array.
    ///
    /// The offset is calculated in the `host` module below.
    fn load_libcall(
        &mut self,
        vmctx: ir::Value,
        index: ComponentBuiltinFunctionIndex,
    ) -> ir::Value {
        let pointer_type = self.isa.pointer_type();
        // First load the pointer to the builtins structure which is static
        // per-process.
        let builtins_array = self.builder.ins().load(
            pointer_type,
            MemFlags::trusted().with_readonly(),
            vmctx,
            i32::try_from(self.offsets.builtins()).unwrap(),
        );
        // Next load the function pointer at `offset` and return that.
        self.builder.ins().load(
            pointer_type,
            MemFlags::trusted().with_readonly(),
            builtins_array,
            i32::try_from(index.index() * u32::from(self.offsets.ptr.size())).unwrap(),
        )
    }

    fn abi_load_params(&mut self) -> Vec<ir::Value> {
        let mut block0_params = self.builder.func.dfg.block_params(self.block0).to_vec();
        match self.abi {
            // Wasm and native ABIs pass parameters as normal function
            // parameters.
            Abi::Wasm => block0_params,

            // The array ABI passes a pointer/length as the 3rd/4th arguments
            // and those are used to load the actual wasm parameters.
            Abi::Array => {
                let results = self.compiler.load_values_from_array(
                    self.types[self.signature].unwrap_func().params(),
                    &mut self.builder,
                    block0_params[2],
                    block0_params[3],
                );
                block0_params.truncate(2);
                block0_params.extend(results);
                block0_params
            }
        }
    }

    fn abi_store_results(&mut self, results: &[ir::Value]) {
        match self.abi {
            // Wasm/native ABIs return values as usual.
            Abi::Wasm => {
                self.builder.ins().return_(results);
            }

            // The array ABI stores all results in the pointer/length passed
            // as arguments to this function, which contractually are required
            // to have enough space for the results.
            Abi::Array => {
                let block0_params = self.builder.func.dfg.block_params(self.block0);
                let (ptr, len) = (block0_params[2], block0_params[3]);
                self.compiler.store_values_to_array(
                    &mut self.builder,
                    self.types[self.signature].unwrap_func().returns(),
                    results,
                    ptr,
                    len,
                );
                let true_value = self.builder.ins().iconst(ir::types::I8, 1);
                self.builder.ins().return_(&[true_value]);
            }
        }
    }

    fn raise_if_host_trapped(&mut self, succeeded: ir::Value) {
        let caller_vmctx = self.builder.func.dfg.block_params(self.block0)[1];
        self.compiler
            .raise_if_host_trapped(&mut self.builder, caller_vmctx, succeeded);
    }

    fn raise_if_transcode_trapped(&mut self, amount_copied: ir::Value) {
        let pointer_type = self.isa.pointer_type();
        let minus_one = self.builder.ins().iconst(pointer_type, -1);
        let succeeded = self
            .builder
            .ins()
            .icmp(IntCC::NotEqual, amount_copied, minus_one);
        self.raise_if_host_trapped(succeeded);
    }

    fn raise_if_negative_one_and_truncate(&mut self, ret: ir::Value) -> ir::Value {
        let ret = self.raise_if_negative_one(ret);
        self.builder.ins().ireduce(ir::types::I32, ret)
    }

    fn raise_if_negative_one(&mut self, ret: ir::Value) -> ir::Value {
        let minus_one = self.builder.ins().iconst(ir::types::I64, -1);
        let succeeded = self.builder.ins().icmp(IntCC::NotEqual, ret, minus_one);
        self.raise_if_host_trapped(succeeded);
        ret
    }

    fn call_libcall(
        &mut self,
        vmctx: ir::Value,
        get_libcall: GetLibcallFn,
        args: &[ir::Value],
    ) -> ir::Inst {
        let (host_sig, index) = get_libcall(self.isa, &mut self.builder.func);
        let host_fn = self.load_libcall(vmctx, index);
        self.compiler
            .call_indirect_host(&mut self.builder, index, host_sig, host_fn, args)
    }
}

impl ComponentCompiler for Compiler {
    fn compile_trampoline(
        &self,
        component: &ComponentTranslation,
        types: &ComponentTypesBuilder,
        index: TrampolineIndex,
        tunables: &Tunables,
    ) -> Result<AllCallFunc<Box<dyn Any + Send>>> {
        let compile = |abi: Abi| -> Result<_> {
            let mut compiler = self.function_compiler();
            let mut c = TrampolineCompiler::new(
                self,
                &mut compiler,
                &component.component,
                types,
                index,
                abi,
                tunables,
            );

            // If we are crossing the Wasm-to-native boundary, we need to save the
            // exit FP and return address for stack walking purposes. However, we
            // always debug assert that our vmctx is a component context, regardless
            // whether we are actually crossing that boundary because it should
            // always hold.
            let vmctx = c.builder.block_params(c.block0)[0];
            let pointer_type = self.isa.pointer_type();
            super::debug_assert_vmctx_kind(
                &*self.isa,
                &mut c.builder,
                vmctx,
                wasmtime_environ::component::VMCOMPONENT_MAGIC,
            );
            if let Abi::Wasm = abi {
                let vm_store_context = c.builder.ins().load(
                    pointer_type,
                    MemFlags::trusted(),
                    vmctx,
                    i32::try_from(c.offsets.vm_store_context()).unwrap(),
                );
                super::save_last_wasm_exit_fp_and_pc(
                    &mut c.builder,
                    pointer_type,
                    &c.offsets.ptr,
                    vm_store_context,
                );
            }

            c.translate(&component.trampolines[index]);
            c.builder.finalize();

            Ok(Box::new(compiler.finish(&format!(
                "component_trampoline_{}_{abi:?}",
                index.as_u32(),
            ))?))
        };
        Ok(AllCallFunc {
            wasm_call: compile(Abi::Wasm)?,
            array_call: compile(Abi::Array)?,
        })
    }
}

impl TrampolineCompiler<'_> {
    fn translate_transcode(
        &mut self,
        op: Transcode,
        from: RuntimeMemoryIndex,
        from64: bool,
        to: RuntimeMemoryIndex,
        to64: bool,
    ) {
        let pointer_type = self.isa.pointer_type();
        let vmctx = self.builder.func.dfg.block_params(self.block0)[0];

        // Determine the static signature of the host libcall for this transcode
        // operation and additionally calculate the static offset within the
        // transode libcalls array.
        let get_libcall = match op {
            Transcode::Copy(FixedEncoding::Utf8) => host::utf8_to_utf8,
            Transcode::Copy(FixedEncoding::Utf16) => host::utf16_to_utf16,
            Transcode::Copy(FixedEncoding::Latin1) => host::latin1_to_latin1,
            Transcode::Latin1ToUtf16 => host::latin1_to_utf16,
            Transcode::Latin1ToUtf8 => host::latin1_to_utf8,
            Transcode::Utf16ToCompactProbablyUtf16 => host::utf16_to_compact_probably_utf16,
            Transcode::Utf16ToCompactUtf16 => host::utf16_to_compact_utf16,
            Transcode::Utf16ToLatin1 => host::utf16_to_latin1,
            Transcode::Utf16ToUtf8 => host::utf16_to_utf8,
            Transcode::Utf8ToCompactUtf16 => host::utf8_to_compact_utf16,
            Transcode::Utf8ToLatin1 => host::utf8_to_latin1,
            Transcode::Utf8ToUtf16 => host::utf8_to_utf16,
        };

        // Load the base pointers for the from/to linear memories.
        let from_base = self.load_runtime_memory_base(vmctx, from);
        let to_base = self.load_runtime_memory_base(vmctx, to);

        let mut args = Vec::new();

        let uses_retptr = match op {
            Transcode::Utf16ToUtf8
            | Transcode::Latin1ToUtf8
            | Transcode::Utf8ToLatin1
            | Transcode::Utf16ToLatin1 => true,
            _ => false,
        };

        // Most transcoders share roughly the same signature despite doing very
        // different things internally, so most libcalls are lumped together
        // here.
        match op {
            Transcode::Copy(_)
            | Transcode::Latin1ToUtf16
            | Transcode::Utf16ToCompactProbablyUtf16
            | Transcode::Utf8ToLatin1
            | Transcode::Utf16ToLatin1
            | Transcode::Utf8ToUtf16 => {
                args.push(self.ptr_param(0, from64, from_base));
                args.push(self.len_param(1, from64));
                args.push(self.ptr_param(2, to64, to_base));
            }

            Transcode::Utf16ToUtf8 | Transcode::Latin1ToUtf8 => {
                args.push(self.ptr_param(0, from64, from_base));
                args.push(self.len_param(1, from64));
                args.push(self.ptr_param(2, to64, to_base));
                args.push(self.len_param(3, to64));
            }

            Transcode::Utf8ToCompactUtf16 | Transcode::Utf16ToCompactUtf16 => {
                args.push(self.ptr_param(0, from64, from_base));
                args.push(self.len_param(1, from64));
                args.push(self.ptr_param(2, to64, to_base));
                args.push(self.len_param(3, to64));
                args.push(self.len_param(4, to64));
            }
        };
        if uses_retptr {
            let slot = self
                .builder
                .func
                .create_sized_stack_slot(ir::StackSlotData::new(
                    ir::StackSlotKind::ExplicitSlot,
                    pointer_type.bytes(),
                    0,
                ));
            args.push(self.builder.ins().stack_addr(pointer_type, slot, 0));
        }
        let call = self.call_libcall(vmctx, get_libcall, &args);
        let mut results = self.builder.func.dfg.inst_results(call).to_vec();
        if uses_retptr {
            results.push(self.builder.ins().load(
                pointer_type,
                ir::MemFlags::trusted(),
                *args.last().unwrap(),
                0,
            ));
        }
        let mut raw_results = Vec::new();

        // Like the arguments the results are fairly similar across libcalls, so
        // they're lumped into various buckets here.
        match op {
            Transcode::Copy(_) | Transcode::Latin1ToUtf16 => {
                self.raise_if_host_trapped(results[0]);
            }

            Transcode::Utf8ToUtf16
            | Transcode::Utf16ToCompactProbablyUtf16
            | Transcode::Utf8ToCompactUtf16
            | Transcode::Utf16ToCompactUtf16 => {
                self.raise_if_transcode_trapped(results[0]);
                raw_results.push(self.cast_from_pointer(results[0], to64));
            }

            Transcode::Latin1ToUtf8
            | Transcode::Utf16ToUtf8
            | Transcode::Utf8ToLatin1
            | Transcode::Utf16ToLatin1 => {
                self.raise_if_transcode_trapped(results[0]);
                raw_results.push(self.cast_from_pointer(results[0], from64));
                raw_results.push(self.cast_from_pointer(results[1], to64));
            }
        };

        self.builder.ins().return_(&raw_results);
    }

    // Helper function to cast an input parameter to the host pointer type.
    fn len_param(&mut self, param: usize, is64: bool) -> ir::Value {
        let val = self.builder.func.dfg.block_params(self.block0)[2 + param];
        self.cast_to_pointer(val, is64)
    }

    // Helper function to interpret an input parameter as a pointer into
    // linear memory. This will cast the input parameter to the host integer
    // type and then add that value to the base.
    //
    // Note that bounds-checking happens in adapter modules, and this
    // trampoline is simply calling the host libcall.
    fn ptr_param(&mut self, param: usize, is64: bool, base: ir::Value) -> ir::Value {
        let val = self.len_param(param, is64);
        self.builder.ins().iadd(base, val)
    }

    // Helper function to cast a core wasm input to a host pointer type
    // which will go into the host libcall.
    fn cast_to_pointer(&mut self, val: ir::Value, is64: bool) -> ir::Value {
        let pointer_type = self.isa.pointer_type();
        let host64 = pointer_type == ir::types::I64;
        if is64 == host64 {
            val
        } else if !is64 {
            assert!(host64);
            self.builder.ins().uextend(pointer_type, val)
        } else {
            assert!(!host64);
            self.builder.ins().ireduce(pointer_type, val)
        }
    }

    // Helper to cast a host pointer integer type to the destination type.
    fn cast_from_pointer(&mut self, val: ir::Value, is64: bool) -> ir::Value {
        let host64 = self.isa.pointer_type() == ir::types::I64;
        if is64 == host64 {
            val
        } else if !is64 {
            assert!(host64);
            self.builder.ins().ireduce(ir::types::I32, val)
        } else {
            assert!(!host64);
            self.builder.ins().uextend(ir::types::I64, val)
        }
    }

    fn load_runtime_memory_base(&mut self, vmctx: ir::Value, mem: RuntimeMemoryIndex) -> ir::Value {
        let pointer_type = self.isa.pointer_type();
        let from_vmmemory_definition = self.load_memory(vmctx, mem);
        self.builder.ins().load(
            pointer_type,
            MemFlags::trusted(),
            from_vmmemory_definition,
            i32::from(self.offsets.ptr.vmmemory_definition_base()),
        )
    }
}

/// Module with macro-generated contents that will return the signature and
/// offset for each of the host transcoder functions.
///
/// Note that a macro is used here to keep this in sync with the actual
/// transcoder functions themselves which are also defined via a macro.
mod host {
    use cranelift_codegen::ir::{self, AbiParam};
    use cranelift_codegen::isa::{CallConv, TargetIsa};
    use wasmtime_environ::component::ComponentBuiltinFunctionIndex;

    macro_rules! define {
        (
            $(
                $( #[$attr:meta] )*
                $name:ident( $( $pname:ident: $param:ident ),* ) $( -> $result:ident )?;
            )*
        ) => {
            $(
                pub(super) fn $name(isa: &dyn TargetIsa, func: &mut ir::Function) -> (ir::SigRef, ComponentBuiltinFunctionIndex) {
                    let pointer_type = isa.pointer_type();
                    let params = vec![
                        $( AbiParam::new(define!(@ty pointer_type $param)) ),*
                    ];
                    let returns = vec![
                        $( AbiParam::new(define!(@ty pointer_type $result)) )?
                    ];
                    let sig = func.import_signature(ir::Signature {
                        params,
                        returns,
                        call_conv: CallConv::triple_default(isa.triple()),
                    });

                    (sig, ComponentBuiltinFunctionIndex::$name())
                }
            )*
        };

        (@ty $ptr:ident size) => ($ptr);
        (@ty $ptr:ident ptr_u8) => ($ptr);
        (@ty $ptr:ident ptr_u16) => ($ptr);
        (@ty $ptr:ident ptr_size) => ($ptr);
        (@ty $ptr:ident bool) => (ir::types::I8);
        (@ty $ptr:ident u8) => (ir::types::I8);
        (@ty $ptr:ident u32) => (ir::types::I32);
        (@ty $ptr:ident u64) => (ir::types::I64);
        (@ty $ptr:ident vmctx) => ($ptr);
    }

    wasmtime_environ::foreach_builtin_component_function!(define);
}
