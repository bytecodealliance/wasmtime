pub(crate) mod arch;
pub(crate) mod frame;
pub(crate) mod gc;

use std::collections::HashMap;
use std::mem;

use cranelift::prelude::*;
use cranelift_codegen::ir::BlockArg;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module};

use crate::frame::*;

/// Intermediate metadata entry for a single function.
#[derive(Debug, Clone)]
struct FunctionMetadata {
    pub total_size: usize,
    pub stack_locations: FunctionStackMap,
}

fn main() {
    let mut settings = settings::builder();
    settings.set("preserve_frame_pointers", "true").unwrap();

    let flags = settings::Flags::new(settings);
    let isa = cranelift_native::builder()
        .unwrap()
        .finish(flags.clone())
        .unwrap();

    let mut builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
    builder.symbol("gc_alloc", gc::allocate_object as *const u8);
    builder.symbol("gc_collect", gc::trigger_collection as *const u8);

    let mut module = JITModule::new(builder);

    // `gc_alloc` is meant to be used whenever a runtime-managed allocation
    // is needed. For unmanaged allocations, used `malloc` or similar function.
    let allocation_func = {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));

        let func = module
            .declare_function("gc_alloc", Linkage::Import, &sig)
            .unwrap();

        func
    };

    // `gc_collect` is used to manually collect dead objects to reclaim
    // memory. You'd likely want to insert this before all call expressions,
    // with some condition before collection actually happens.
    //
    // For example, only run collection every 500ms or once a certain
    // amount of memory is in use.
    let collection_func = {
        let sig = module.make_signature();
        let func = module
            .declare_function("gc_collect", Linkage::Import, &sig)
            .unwrap();

        func
    };

    let mut ctx = module.make_context();
    let mut func_ctx = FunctionBuilderContext::new();

    let mut function_metadata = HashMap::new();

    // The main function is not meant to have any practical application,
    // expect show an example implementation of a tracing garbage collector.
    //
    // The function is something akin to the following Rust code:
    // ```rs
    // struct Object {
    //     pub value: i32,
    // }
    //
    // fn main() -> i32 {
    //     let a = Object { value: 8 };
    //
    //     let mut counter = 10;
    //     loop {
    //         let b = Object { value: 0 };
    //
    //         counter -= 1;
    //
    //         if counter == 0 {
    //             break;
    //         }
    //     }
    //
    //     gc_collect();
    //
    //     a.value
    // }
    // ```
    //
    // After the loop has finished, `gc_collect()` will cause all the objects
    // allocated within the loop to be deallocated, while the single allocation
    // outside the loop will remain allocated.
    let main_func = {
        let mut sig = module.make_signature();
        sig.returns.push(AbiParam::new(types::I32));

        let func = module
            .declare_function("main", Linkage::Export, &sig)
            .unwrap();

        ctx.func.signature = sig.clone();

        let mut bcx = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
        let entry_block = bcx.create_block();
        bcx.append_block_params_for_function_params(entry_block);

        let loop_body = bcx.create_block();
        bcx.append_block_param(loop_body, types::I32);
        bcx.append_block_param(loop_body, types::I64);

        let loop_exit = bcx.create_block();
        bcx.append_block_param(loop_exit, types::I64);

        bcx.switch_to_block(entry_block);
        {
            // Allocate 8 bytes for the object, in which we store an integer.
            let alloc_size = bcx.ins().iconst(types::I64, 4);
            let allocation_func_ref = module.declare_func_in_func(allocation_func, bcx.func);

            let call_inst = bcx.ins().call(allocation_func_ref, &[alloc_size]);
            let alloc_ptr = bcx.inst_results(call_inst)[0];
            bcx.declare_value_needs_stack_map(alloc_ptr);

            let field_value = bcx.ins().iconst(types::I32, 8);
            bcx.ins().store(MemFlags::new(), field_value, alloc_ptr, 0);

            let counter_value = bcx.ins().iconst(types::I32, 10);

            bcx.ins().jump(
                loop_body,
                vec![&BlockArg::Value(counter_value), &BlockArg::Value(alloc_ptr)],
            );
        }

        bcx.switch_to_block(loop_body);
        {
            let parent_obj = bcx.block_params(loop_body)[1];
            bcx.declare_value_needs_stack_map(parent_obj); // required since this is a new block.

            let alloc_size = bcx.ins().iconst(types::I64, 4);
            let allocation_func_ref = module.declare_func_in_func(allocation_func, bcx.func);

            let call_inst = bcx.ins().call(allocation_func_ref, &[alloc_size]);
            let alloc_ptr = bcx.inst_results(call_inst)[0];
            bcx.declare_value_needs_stack_map(alloc_ptr);

            let current_count = bcx.block_params(loop_body)[0];
            let next_count = bcx.ins().iadd_imm(current_count, -1);

            let cmp_val = bcx
                .ins()
                .icmp_imm(IntCC::SignedGreaterThan, current_count, 0);

            bcx.ins().brif(
                cmp_val,
                loop_body,
                vec![&BlockArg::Value(next_count), &BlockArg::Value(parent_obj)],
                loop_exit,
                vec![&BlockArg::Value(parent_obj)],
            );
        }

        bcx.switch_to_block(loop_exit);
        {
            let parent_obj = bcx.block_params(loop_exit)[0];
            bcx.declare_value_needs_stack_map(parent_obj); // required since this is a new block.

            let collection_func_ref = module.declare_func_in_func(collection_func, bcx.func);
            bcx.ins().call(collection_func_ref, &[]);

            let field_value = bcx.ins().load(types::I32, MemFlags::new(), parent_obj, 0);
            bcx.ins().return_(&[field_value]);
        }

        bcx.seal_all_blocks();
        bcx.finalize();

        module.define_function(func, &mut ctx).unwrap();

        let compiled_code = ctx.compiled_code().unwrap();
        let code_len = compiled_code.buffer.total_size() as usize;

        // We change the format of the stack maps, since we don't actually
        // need the type of each entry in the stack map.
        let mut stack_locations = Vec::new();
        for (offset, length, map) in compiled_code.buffer.user_stack_maps() {
            let refs = map
                .entries()
                .map(|(_, offset)| offset as usize)
                .collect::<Vec<_>>();

            stack_locations.push((*offset as usize, *length as usize, refs));
        }

        // This is an intermediate map for mapping functions to their matching stack locations,
        // since we can't get them after clearing the context.
        function_metadata.insert(
            "main",
            FunctionMetadata {
                total_size: code_len,
                stack_locations,
            },
        );

        module.clear_context(&mut ctx);

        func
    };

    module.finalize_definitions().unwrap();

    let mut func_stack_maps = Vec::new();

    // In an implementation with dynamic codegen, this would need to be executed
    // once per compiled function. Since we only have a single function, we just
    // act like it's a loop.
    {
        let metadata = function_metadata.remove("main").unwrap();
        let start = FunctionPtr::new(module.get_finalized_function(main_func));
        let end = FunctionPtr::new(unsafe { start.ptr().byte_add(metadata.total_size) });

        func_stack_maps.push(CompiledFunctionMetadata {
            start,
            end,
            stack_locations: metadata.stack_locations,
        });
    }

    // Declare the stack maps globally, so we can use them when iterating
    // through the stack frames.
    declare_stack_maps(func_stack_maps);

    let main_addr = module.get_finalized_function(main_func);
    let main_ptr = unsafe { mem::transmute::<_, extern "C" fn() -> i32>(main_addr) };
    let ret_code = main_ptr();

    std::process::exit(ret_code);
}
