use cranelift_codegen::binemit::NullTrapSink;
use cranelift_codegen::ir::*;
use cranelift_codegen::isa::CallConv;
use cranelift_codegen::settings::{self, Configurable};
use cranelift_codegen::{ir::types::I16, Context};
use cranelift_entity::EntityRef;
use cranelift_frontend::*;
use cranelift_module::*;
use cranelift_simplejit::*;

#[test]
fn error_on_incompatible_sig_in_declare_function() {
    let mut flag_builder = settings::builder();
    flag_builder.set("use_colocated_libcalls", "false").unwrap();
    // FIXME set back to true once the x64 backend supports it.
    flag_builder.set("is_pic", "false").unwrap();
    let isa_builder = cranelift_native::builder().unwrap_or_else(|msg| {
        panic!("host machine is not supported: {}", msg);
    });
    let isa = isa_builder.finish(settings::Flags::new(flag_builder));
    let mut module: SimpleJITModule =
        SimpleJITModule::new(SimpleJITBuilder::with_isa(isa, default_libcall_names()));

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

fn define_simple_function(module: &mut SimpleJITModule) -> FuncId {
    let sig = Signature {
        params: vec![],
        returns: vec![],
        call_conv: CallConv::SystemV,
    };

    let func_id = module
        .declare_function("abc", Linkage::Local, &sig)
        .unwrap();

    let mut ctx = Context::new();
    ctx.func = Function::with_name_signature(ExternalName::user(0, func_id.as_u32()), sig);
    let mut func_ctx = FunctionBuilderContext::new();
    {
        let mut bcx: FunctionBuilder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
        let block = bcx.create_block();
        bcx.switch_to_block(block);
        bcx.ins().return_(&[]);
    }

    let mut trap_sink = NullTrapSink {};
    module
        .define_function(func_id, &mut ctx, &mut trap_sink)
        .unwrap();

    func_id
}

#[test]
#[should_panic(expected = "Result::unwrap()` on an `Err` value: DuplicateDefinition(\"abc\")")]
fn panic_on_define_after_finalize() {
    let mut flag_builder = settings::builder();
    flag_builder.set("use_colocated_libcalls", "false").unwrap();
    // FIXME set back to true once the x64 backend supports it.
    flag_builder.set("is_pic", "false").unwrap();
    let isa_builder = cranelift_native::builder().unwrap_or_else(|msg| {
        panic!("host machine is not supported: {}", msg);
    });
    let isa = isa_builder.finish(settings::Flags::new(flag_builder));
    let mut module: SimpleJITModule =
        SimpleJITModule::new(SimpleJITBuilder::with_isa(isa, default_libcall_names()));

    define_simple_function(&mut module);
    define_simple_function(&mut module);
}

#[test]
fn switch_error() {
    use cranelift_codegen::settings;

    let sig = Signature {
        params: vec![AbiParam::new(types::I32)],
        returns: vec![AbiParam::new(types::I32)],
        call_conv: CallConv::SystemV,
    };

    let mut func = Function::with_name_signature(ExternalName::user(0, 0), sig);

    let mut func_ctx = FunctionBuilderContext::new();
    {
        let mut bcx: FunctionBuilder = FunctionBuilder::new(&mut func, &mut func_ctx);
        let start = bcx.create_block();
        let bb0 = bcx.create_block();
        let bb1 = bcx.create_block();
        let bb2 = bcx.create_block();
        let bb3 = bcx.create_block();
        println!("{} {} {} {} {}", start, bb0, bb1, bb2, bb3);

        bcx.declare_var(Variable::new(0), types::I32);
        bcx.declare_var(Variable::new(1), types::I32);
        let in_val = bcx.append_block_param(start, types::I32);
        bcx.switch_to_block(start);
        bcx.def_var(Variable::new(0), in_val);
        bcx.ins().jump(bb0, &[]);

        bcx.switch_to_block(bb0);
        let discr = bcx.use_var(Variable::new(0));
        let mut switch = cranelift_frontend::Switch::new();
        for &(index, bb) in &[
            (9, bb1),
            (13, bb1),
            (10, bb1),
            (92, bb1),
            (39, bb1),
            (34, bb1),
        ] {
            switch.set_entry(index, bb);
        }
        switch.emit(&mut bcx, discr, bb2);

        bcx.switch_to_block(bb1);
        let v = bcx.use_var(Variable::new(0));
        bcx.def_var(Variable::new(1), v);
        bcx.ins().jump(bb3, &[]);

        bcx.switch_to_block(bb2);
        let v = bcx.use_var(Variable::new(0));
        bcx.def_var(Variable::new(1), v);
        bcx.ins().jump(bb3, &[]);

        bcx.switch_to_block(bb3);
        let r = bcx.use_var(Variable::new(1));
        bcx.ins().return_(&[r]);

        bcx.seal_all_blocks();
        bcx.finalize();
    }

    let flags = settings::Flags::new(settings::builder());
    match cranelift_codegen::verify_function(&func, &flags) {
        Ok(_) => {}
        Err(err) => {
            let pretty_error =
                cranelift_codegen::print_errors::pretty_verifier_error(&func, None, None, err);
            panic!("pretty_error:\n{}", pretty_error);
        }
    }
}

#[test]
fn libcall_function() {
    let mut flag_builder = settings::builder();
    flag_builder.set("use_colocated_libcalls", "false").unwrap();
    // FIXME set back to true once the x64 backend supports it.
    flag_builder.set("is_pic", "false").unwrap();
    let isa_builder = cranelift_native::builder().unwrap_or_else(|msg| {
        panic!("host machine is not supported: {}", msg);
    });
    let isa = isa_builder.finish(settings::Flags::new(flag_builder));
    let mut module: SimpleJITModule =
        SimpleJITModule::new(SimpleJITBuilder::with_isa(isa, default_libcall_names()));

    let sig = Signature {
        params: vec![],
        returns: vec![],
        call_conv: CallConv::SystemV,
    };

    let func_id = module
        .declare_function("function", Linkage::Local, &sig)
        .unwrap();

    let mut ctx = Context::new();
    ctx.func = Function::with_name_signature(ExternalName::user(0, func_id.as_u32()), sig);
    let mut func_ctx = FunctionBuilderContext::new();
    {
        let mut bcx: FunctionBuilder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
        let block = bcx.create_block();
        bcx.switch_to_block(block);

        let int = module.target_config().pointer_type();
        let zero = bcx.ins().iconst(I16, 0);
        let size = bcx.ins().iconst(int, 10);

        let mut signature = module.make_signature();
        signature.params.push(AbiParam::new(int));
        signature.returns.push(AbiParam::new(int));
        let callee = module
            .declare_function("malloc", Linkage::Import, &signature)
            .expect("declare malloc function");
        let local_callee = module.declare_func_in_func(callee, &mut bcx.func);
        let argument_exprs = vec![size];
        let call = bcx.ins().call(local_callee, &argument_exprs);
        let buffer = bcx.inst_results(call)[0];

        bcx.call_memset(module.target_config(), buffer, zero, size);

        bcx.ins().return_(&[]);
    }

    let mut trap_sink = NullTrapSink {};
    module
        .define_function(func_id, &mut ctx, &mut trap_sink)
        .unwrap();

    module.finalize_definitions();
}
