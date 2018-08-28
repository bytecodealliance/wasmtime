extern crate cranelift;
extern crate cranelift_module;
extern crate cranelift_simplejit;

use cranelift::prelude::*;
use cranelift_module::{Linkage, Module};
use cranelift_simplejit::{SimpleJITBackend, SimpleJITBuilder};
use std::mem;

fn main() {
    let mut module: Module<SimpleJITBackend> = Module::new(SimpleJITBuilder::new());
    let mut ctx = module.make_context();
    let mut func_ctx = FunctionBuilderContext::new();
    let sig = module.make_signature();

    let func_a = module.declare_function("a", Linkage::Local, &sig).unwrap();
    let func_b = module.declare_function("b", Linkage::Local, &sig).unwrap();

    ctx.func.name = ExternalName::user(0, func_a.index() as u32);
    {
        let mut bcx: FunctionBuilder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
        let ebb = bcx.create_ebb();

        bcx.switch_to_block(ebb);
        bcx.ins().return_(&[]);
        bcx.seal_all_blocks();
        bcx.finalize();
    }
    module.define_function(func_a, &mut ctx).unwrap();
    module.clear_context(&mut ctx);

    ctx.func.name = ExternalName::user(0, func_b.index() as u32);
    {
        let mut bcx: FunctionBuilder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
        let ebb = bcx.create_ebb();

        bcx.switch_to_block(ebb);
        let local_func = module.declare_func_in_func(func_a, &mut bcx.func);
        bcx.ins().call(local_func, &[]);
        bcx.ins().return_(&[]);
        bcx.seal_all_blocks();
        bcx.finalize();
    }
    module.define_function(func_b, &mut ctx).unwrap();
    module.clear_context(&mut ctx);

    // Perform linking.
    module.finalize_all();

    // Get a raw pointer to the generated code.
    let code_b = module.get_finalized_function(func_b);

    // Cast it to a rust function pointer type.
    let ptr_b = unsafe { mem::transmute::<_, fn()>(code_b) };

    // Call it!
    ptr_b();
}
