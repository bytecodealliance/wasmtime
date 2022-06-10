//! Compilation support for the component model.

use crate::compiler::{Compiler, CompilerContext};
use crate::obj::ModuleTextBuilder;
use crate::CompiledFunction;
use anyhow::Result;
use cranelift_codegen::ir::{self, InstBuilder, MemFlags};
use cranelift_frontend::FunctionBuilder;
use object::write::Object;
use std::any::Any;
use wasmtime_environ::component::{
    CanonicalOptions, Component, ComponentCompiler, ComponentTypes, LowerImport, LoweredIndex,
    TrampolineInfo, VMComponentOffsets,
};
use wasmtime_environ::PrimaryMap;

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
        } = self.take_context();

        context.func = ir::Function::with_name_signature(
            ir::ExternalName::user(0, 0),
            crate::indirect_signature(isa, ty),
        );

        let mut builder = FunctionBuilder::new(&mut context.func, func_translator.context());
        let block0 = builder.create_block();

        // Start off by spilling all the wasm arguments into a stack slot to be
        // passed to the host function.
        let (values_vec_ptr_val, values_vec_len) =
            self.wasm_to_host_spill_args(ty, &mut builder, block0);
        let vmctx = builder.func.dfg.block_params(block0)[0];

        // Below this will incrementally build both the signature of the host
        // function we're calling as well as the list of arguments since the
        // list is somewhat long.
        let mut callee_args = Vec::new();
        let mut host_sig = ir::Signature::new(crate::wasmtime_call_conv(isa));

        let CanonicalOptions {
            memory,
            realloc,
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

        let func: CompiledFunction = self.finish_trampoline(&mut context, isa)?;
        self.save_context(CompilerContext {
            func_translator,
            codegen_context: context,
        });
        Ok(Box::new(func))
    }

    fn emit_obj(
        &self,
        trampolines: PrimaryMap<LoweredIndex, Box<dyn Any + Send>>,
        obj: &mut Object<'static>,
    ) -> Result<PrimaryMap<LoweredIndex, TrampolineInfo>> {
        let trampolines: PrimaryMap<LoweredIndex, CompiledFunction> = trampolines
            .into_iter()
            .map(|(_, f)| *f.downcast().unwrap())
            .collect();
        let module = Default::default();
        let mut text = ModuleTextBuilder::new(obj, &module, &*self.isa);
        let mut ret = PrimaryMap::new();
        for (idx, trampoline) in trampolines.iter() {
            let (_symbol, range) = text.append_func(
                false,
                format!("_wasm_component_host_trampoline{}", idx.as_u32()).into_bytes(),
                &trampoline,
            );

            let i = ret.push(TrampolineInfo {
                start: u32::try_from(range.start).unwrap(),
                length: u32::try_from(range.end - range.start).unwrap(),
            });
            assert_eq!(i, idx);
        }

        text.finish()?;

        Ok(ret)
    }
}
