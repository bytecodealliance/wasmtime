use cranelift::prelude::*;
use cranelift_codegen::binemit::{NullStackMapSink, NullTrapSink};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{default_libcall_names, Linkage, Module};
use std::mem;

fn main() {
    let mut flag_builder = settings::builder();
    flag_builder.set("use_colocated_libcalls", "false").unwrap();
    // FIXME set back to true once the x64 backend supports it.
    flag_builder.set("is_pic", "false").unwrap();
    let isa_builder = cranelift_native::builder().unwrap_or_else(|msg| {
        panic!("host machine is not supported: {}", msg);
    });
    let isa = isa_builder.finish(settings::Flags::new(flag_builder));
    let mut module = JITModule::new(JITBuilder::with_isa(isa, default_libcall_names()));

    let mut ctx = module.make_context();
    let mut func_ctx = FunctionBuilderContext::new();

    let mut sig_a = module.make_signature();
    sig_a.params.push(AbiParam::new(types::I32));
    sig_a.returns.push(AbiParam::new(types::I32));

    let mut sig_b = module.make_signature();
    sig_b.returns.push(AbiParam::new(types::I32));

    let func_a = module
        .declare_function("a", Linkage::Local, &sig_a)
        .unwrap();
    let func_b = module
        .declare_function("b", Linkage::Local, &sig_b)
        .unwrap();

    ctx.func.signature = sig_a;
    ctx.func.name = ExternalName::user(0, func_a.as_u32());
    {
        let mut bcx: FunctionBuilder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
        let block = bcx.create_block();

        bcx.switch_to_block(block);
        bcx.append_block_params_for_function_params(block);
        let param = bcx.block_params(block)[0];
        let cst = bcx.ins().iconst(types::I32, 37);
        let add = bcx.ins().iadd(cst, param);
        bcx.ins().return_(&[add]);
        bcx.seal_all_blocks();
        bcx.finalize();
    }
    let mut trap_sink = NullTrapSink {};
    let mut stack_map_sink = NullStackMapSink {};
    module
        .define_function(func_a, &mut ctx, &mut trap_sink, &mut stack_map_sink)
        .unwrap();
    module.clear_context(&mut ctx);

    ctx.func.signature = sig_b;
    ctx.func.name = ExternalName::user(0, func_b.as_u32());
    {
        let mut bcx: FunctionBuilder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
        let block = bcx.create_block();

        bcx.switch_to_block(block);
        let local_func = module.declare_func_in_func(func_a, &mut bcx.func);
        let arg = bcx.ins().iconst(types::I32, 5);
        let call = bcx.ins().call(local_func, &[arg]);
        let value = {
            let results = bcx.inst_results(call);
            assert_eq!(results.len(), 1);
            results[0].clone()
        };
        bcx.ins().return_(&[value]);
        bcx.seal_all_blocks();
        bcx.finalize();
    }
    module
        .define_function(func_b, &mut ctx, &mut trap_sink, &mut stack_map_sink)
        .unwrap();
    module.clear_context(&mut ctx);

    // Perform linking.
    module.finalize_definitions();

    // Get a raw pointer to the generated code.
    let code_b = module.get_finalized_function(func_b);

    // Cast it to a rust function pointer type.
    let ptr_b = unsafe { mem::transmute::<_, fn() -> u32>(code_b) };

    // Call it!
    let res = ptr_b();

    assert_eq!(res, 42);
}
