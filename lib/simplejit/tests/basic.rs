extern crate cranelift_codegen;
extern crate cranelift_entity;
extern crate cranelift_frontend;
extern crate cranelift_module;
extern crate cranelift_simplejit;

use cranelift_codegen::ir::*;
use cranelift_codegen::settings::*;
use cranelift_codegen::Context;
use cranelift_entity::EntityRef;
use cranelift_frontend::*;
use cranelift_module::*;
use cranelift_simplejit::*;

#[test]
fn error_on_incompatible_sig_in_declare_function() {
    let mut module: Module<SimpleJITBackend> = Module::new(SimpleJITBuilder::new());
    let mut sig = Signature {
        params: vec![AbiParam::new(types::I64)],
        returns: vec![],
        call_conv: CallConv::SystemV,
    };
    module
        .declare_function("abc", Linkage::Local, &sig)
        .unwrap();
    sig.params[0] = AbiParam::new(types::I32);
    module
        .declare_function("abc", Linkage::Local, &sig)
        .err()
        .unwrap(); // Make sure this is an error
}

fn define_simple_function(module: &mut Module<SimpleJITBackend>) -> FuncId {
    let sig = Signature {
        params: vec![],
        returns: vec![],
        call_conv: CallConv::SystemV,
    };

    let func_id = module
        .declare_function("abc", Linkage::Local, &sig)
        .unwrap();

    let mut ctx = Context::new();
    ctx.func = Function::with_name_signature(ExternalName::user(0, func_id.index() as u32), sig);
    let mut func_ctx = FunctionBuilderContext::new();
    {
        let mut bcx: FunctionBuilder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
        let ebb = bcx.create_ebb();
        bcx.switch_to_block(ebb);
        bcx.ins().return_(&[]);
    }

    module.define_function(func_id, &mut ctx).unwrap();

    func_id
}

#[test]
fn double_finalize() {
    let mut module: Module<SimpleJITBackend> = Module::new(SimpleJITBuilder::new());

    define_simple_function(&mut module);
    module.finalize_definitions();

    // Calling `finalize_definitions` a second time without any new definitions
    // should have no effect.
    module.finalize_definitions();
}

#[test]
#[should_panic(expected = "Result::unwrap()` on an `Err` value: DuplicateDefinition(\"abc\")")]
fn panic_on_define_after_finalize() {
    let mut module: Module<SimpleJITBackend> = Module::new(SimpleJITBuilder::new());

    define_simple_function(&mut module);
    module.finalize_definitions();
    define_simple_function(&mut module);
}
