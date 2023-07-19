//! Compilation support for the component model.

use crate::compiler::{Compiler, NativeRet};
use anyhow::Result;
use cranelift_codegen::ir::{self, InstBuilder, MemFlags};
use cranelift_codegen::isa::CallConv;
use cranelift_frontend::FunctionBuilder;
use std::any::Any;
use wasmtime_cranelift_shared::ALWAYS_TRAP_CODE;
use wasmtime_environ::component::{
    AllCallFunc, CanonicalOptions, Component, ComponentCompiler, ComponentTypes, FixedEncoding,
    LowerImport, RuntimeMemoryIndex, Transcode, Transcoder, TypeDef, VMComponentOffsets,
};
use wasmtime_environ::{PtrSize, WasmFuncType};

#[derive(Copy, Clone)]
enum Abi {
    Wasm,
    Native,
    Array,
}

impl Compiler {
    fn compile_lowered_trampoline_for_abi(
        &self,
        component: &Component,
        lowering: &LowerImport,
        types: &ComponentTypes,
        abi: Abi,
    ) -> Result<Box<dyn Any + Send>> {
        let wasm_func_ty = &types[lowering.canonical_abi];
        let isa = &*self.isa;
        let pointer_type = isa.pointer_type();
        let offsets = VMComponentOffsets::new(isa.pointer_bytes(), component);

        let mut compiler = self.function_compiler();

        let func = self.func(wasm_func_ty, abi);
        let (mut builder, block0) = compiler.builder(func);
        let args = builder.func.dfg.block_params(block0).to_vec();
        let vmctx = args[0];

        // More handling is necessary here if this changes
        assert!(matches!(
            NativeRet::classify(pointer_type, wasm_func_ty),
            NativeRet::Bare
        ));

        // Start off by spilling all the wasm arguments into a stack slot to be
        // passed to the host function.
        let (values_vec_ptr, values_vec_len) = match abi {
            Abi::Wasm | Abi::Native => {
                let (ptr, len) = self.allocate_stack_array_and_spill_args(
                    wasm_func_ty,
                    &mut builder,
                    &args[2..],
                );
                let len = builder.ins().iconst(pointer_type, i64::from(len));
                (ptr, len)
            }
            Abi::Array => {
                let params = builder.func.dfg.block_params(block0);
                (params[2], params[3])
            }
        };

        self.abi_preamble(&mut builder, &offsets, vmctx, abi);

        // Below this will incrementally build both the signature of the host
        // function we're calling as well as the list of arguments since the
        // list is somewhat long.
        let mut callee_args = Vec::new();
        let mut host_sig = ir::Signature::new(CallConv::triple_default(isa.triple()));

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

        // ty: TypeFuncIndex,
        let ty = match component.type_of_import(lowering.import, types) {
            TypeDef::ComponentFunc(func) => func,
            _ => unreachable!(),
        };
        host_sig.params.push(ir::AbiParam::new(ir::types::I32));
        callee_args.push(builder.ins().iconst(ir::types::I32, i64::from(ty.as_u32())));

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

        // realloc: *mut VMFuncRef
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
        callee_args.push(values_vec_ptr);

        // storage_len: usize
        host_sig.params.push(ir::AbiParam::new(pointer_type));
        callee_args.push(values_vec_len);

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

        match abi {
            Abi::Wasm | Abi::Native => {
                // After the host function has returned the results are loaded from
                // `values_vec_ptr` and then returned.
                let results = self.load_values_from_array(
                    wasm_func_ty.returns(),
                    &mut builder,
                    values_vec_ptr,
                    values_vec_len,
                );
                builder.ins().return_(&results);
            }
            Abi::Array => {
                builder.ins().return_(&[]);
            }
        }
        builder.finalize();

        Ok(Box::new(compiler.finish()?))
    }

    fn compile_always_trap_for_abi(
        &self,
        ty: &WasmFuncType,
        abi: Abi,
    ) -> Result<Box<dyn Any + Send>> {
        let mut compiler = self.function_compiler();
        let func = self.func(ty, abi);
        let (mut builder, _block0) = compiler.builder(func);
        builder.ins().trap(ir::TrapCode::User(ALWAYS_TRAP_CODE));
        builder.finalize();

        Ok(Box::new(compiler.finish()?))
    }

    fn compile_transcoder_for_abi(
        &self,
        component: &Component,
        transcoder: &Transcoder,
        types: &ComponentTypes,
        abi: Abi,
    ) -> Result<Box<dyn Any + Send>> {
        let ty = &types[transcoder.signature];
        let isa = &*self.isa;
        let offsets = VMComponentOffsets::new(isa.pointer_bytes(), component);
        let mut compiler = self.function_compiler();
        let func = self.func(ty, abi);
        let (mut builder, block0) = compiler.builder(func);

        match abi {
            Abi::Wasm => {
                self.translate_transcode(&mut builder, &offsets, transcoder, block0);
            }
            // Transcoders can only actually be called by Wasm, so let's assert
            // that here.
            Abi::Native | Abi::Array => {
                builder
                    .ins()
                    .trap(ir::TrapCode::User(crate::DEBUG_ASSERT_TRAP_CODE));
            }
        }

        builder.finalize();
        Ok(Box::new(compiler.finish()?))
    }

    fn func(&self, ty: &WasmFuncType, abi: Abi) -> ir::Function {
        let isa = &*self.isa;
        ir::Function::with_name_signature(
            ir::UserFuncName::user(0, 0),
            match abi {
                Abi::Wasm => crate::wasm_call_signature(isa, ty),
                Abi::Native => crate::native_call_signature(isa, ty),
                Abi::Array => crate::array_call_signature(isa),
            },
        )
    }

    fn compile_func_ref(
        &self,
        compile: impl Fn(Abi) -> Result<Box<dyn Any + Send>>,
    ) -> Result<AllCallFunc<Box<dyn Any + Send>>> {
        Ok(AllCallFunc {
            wasm_call: compile(Abi::Wasm)?,
            array_call: compile(Abi::Array)?,
            native_call: compile(Abi::Native)?,
        })
    }

    fn abi_preamble(
        &self,
        builder: &mut FunctionBuilder<'_>,
        offsets: &VMComponentOffsets<u8>,
        vmctx: ir::Value,
        abi: Abi,
    ) {
        let pointer_type = self.isa.pointer_type();
        // If we are crossing the Wasm-to-native boundary, we need to save the
        // exit FP and return address for stack walking purposes. However, we
        // always debug assert that our vmctx is a component context, regardless
        // whether we are actually crossing that boundary because it should
        // always hold.
        super::debug_assert_vmctx_kind(
            &*self.isa,
            builder,
            vmctx,
            wasmtime_environ::component::VMCOMPONENT_MAGIC,
        );
        if let Abi::Wasm = abi {
            let limits = builder.ins().load(
                pointer_type,
                MemFlags::trusted(),
                vmctx,
                i32::try_from(offsets.limits()).unwrap(),
            );
            super::save_last_wasm_exit_fp_and_pc(builder, pointer_type, &offsets.ptr, limits);
        }
    }
}

impl ComponentCompiler for Compiler {
    fn compile_lowered_trampoline(
        &self,
        component: &Component,
        lowering: &LowerImport,
        types: &ComponentTypes,
    ) -> Result<AllCallFunc<Box<dyn Any + Send>>> {
        self.compile_func_ref(|abi| {
            self.compile_lowered_trampoline_for_abi(component, lowering, types, abi)
        })
    }

    fn compile_always_trap(&self, ty: &WasmFuncType) -> Result<AllCallFunc<Box<dyn Any + Send>>> {
        self.compile_func_ref(|abi| self.compile_always_trap_for_abi(ty, abi))
    }

    fn compile_transcoder(
        &self,
        component: &Component,
        transcoder: &Transcoder,
        types: &ComponentTypes,
    ) -> Result<AllCallFunc<Box<dyn Any + Send>>> {
        self.compile_func_ref(|abi| {
            self.compile_transcoder_for_abi(component, transcoder, types, abi)
        })
    }
}

impl Compiler {
    fn translate_transcode(
        &self,
        builder: &mut FunctionBuilder<'_>,
        offsets: &VMComponentOffsets<u8>,
        transcoder: &Transcoder,
        block: ir::Block,
    ) {
        let pointer_type = self.isa.pointer_type();
        let vmctx = builder.func.dfg.block_params(block)[0];

        self.abi_preamble(builder, offsets, vmctx, Abi::Wasm);

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

        let uses_retptr = match transcoder.op {
            Transcode::Utf16ToUtf8
            | Transcode::Latin1ToUtf8
            | Transcode::Utf8ToLatin1
            | Transcode::Utf16ToLatin1 => true,
            _ => false,
        };

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
        if uses_retptr {
            let slot = builder.func.create_sized_stack_slot(ir::StackSlotData::new(
                ir::StackSlotKind::ExplicitSlot,
                pointer_type.bytes(),
            ));
            args.push(builder.ins().stack_addr(pointer_type, slot, 0));
        }
        let call = builder.ins().call_indirect(sig, transcode_libcall, &args);
        let mut results = builder.func.dfg.inst_results(call).to_vec();
        if uses_retptr {
            results.push(builder.ins().load(
                pointer_type,
                ir::MemFlags::trusted(),
                *args.last().unwrap(),
                0,
            ));
        }
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
mod host {
    use crate::compiler::Compiler;
    use cranelift_codegen::ir::{self, AbiParam};
    use cranelift_codegen::isa::CallConv;

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
                    let returns = vec![
                        $( AbiParam::new(host_transcode!(@ty pointer_type $result)) )?
                    ];
                    let sig = func.import_signature(ir::Signature {
                        params,
                        returns,
                        call_conv: CallConv::triple_default(compiler.isa.triple()),
                    });

                    (sig, offsets::$name)
                }
            )*
        };

        (@ty $ptr:ident size) => ($ptr);
        (@ty $ptr:ident ptr_u8) => ($ptr);
        (@ty $ptr:ident ptr_u16) => ($ptr);
        (@ty $ptr:ident ptr_size) => ($ptr);
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
