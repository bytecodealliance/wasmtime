use cranelift_codegen::ir::*;
use cranelift_codegen::isa::CallConv;
use cranelift_codegen::settings;
use cranelift_codegen::{Context, ir::types::I16};
use cranelift_entity::EntityRef;
use cranelift_frontend::*;
use cranelift_module::*;
use cranelift_object::*;

#[test]
fn error_on_incompatible_sig_in_declare_function() {
    let flag_builder = settings::builder();
    let isa_builder = cranelift_codegen::isa::lookup_by_name("x86_64-unknown-linux-gnu").unwrap();
    let isa = isa_builder
        .finish(settings::Flags::new(flag_builder))
        .unwrap();
    let mut module =
        ObjectModule::new(ObjectBuilder::new(isa, "foo", default_libcall_names()).unwrap());
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

fn define_simple_function(module: &mut ObjectModule) -> FuncId {
    let sig = Signature {
        params: vec![],
        returns: vec![],
        call_conv: CallConv::SystemV,
    };

    let func_id = module
        .declare_function("abc", Linkage::Local, &sig)
        .unwrap();

    let mut ctx = Context::new();
    ctx.func = Function::with_name_signature(UserFuncName::user(0, func_id.as_u32()), sig);
    let mut func_ctx = FunctionBuilderContext::new();
    {
        let mut bcx: FunctionBuilder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
        let block = bcx.create_block();
        bcx.switch_to_block(block);
        bcx.ins().return_(&[]);
    }

    module.define_function(func_id, &mut ctx).unwrap();

    func_id
}

#[test]
#[should_panic(expected = "Result::unwrap()` on an `Err` value: DuplicateDefinition(\"abc\")")]
fn panic_on_define_after_finalize() {
    let flag_builder = settings::builder();
    let isa_builder = cranelift_codegen::isa::lookup_by_name("x86_64-unknown-linux-gnu").unwrap();
    let isa = isa_builder
        .finish(settings::Flags::new(flag_builder))
        .unwrap();
    let mut module =
        ObjectModule::new(ObjectBuilder::new(isa, "foo", default_libcall_names()).unwrap());

    define_simple_function(&mut module);
    define_simple_function(&mut module);
}

#[test]
#[cfg_attr(not(debug_assertions), ignore = "checks a debug assertion")]
#[should_panic(expected = "function \"abc\" with linkage Local must be defined but is not")]
fn panic_on_declare_without_define() {
    let flag_builder = settings::builder();
    let isa_builder = cranelift_codegen::isa::lookup_by_name("x86_64-unknown-linux-gnu").unwrap();
    let isa = isa_builder
        .finish(settings::Flags::new(flag_builder))
        .unwrap();
    let mut module =
        ObjectModule::new(ObjectBuilder::new(isa, "foo", default_libcall_names()).unwrap());

    module
        .declare_function("abc", Linkage::Local, &Signature::new(CallConv::SystemV))
        .unwrap();

    module.finish();
}

#[test]
fn switch_error() {
    use cranelift_codegen::settings;

    let sig = Signature {
        params: vec![AbiParam::new(types::I32)],
        returns: vec![AbiParam::new(types::I32)],
        call_conv: CallConv::SystemV,
    };

    let mut func = Function::with_name_signature(UserFuncName::default(), sig);
    let mut func_ctx = FunctionBuilderContext::new();
    {
        let mut bcx: FunctionBuilder = FunctionBuilder::new(&mut func, &mut func_ctx);
        let start = bcx.create_block();
        let bb0 = bcx.create_block();
        let bb1 = bcx.create_block();
        let bb2 = bcx.create_block();
        let bb3 = bcx.create_block();
        println!("{start} {bb0} {bb1} {bb2} {bb3}");

        bcx.declare_var(types::I32);
        bcx.declare_var(types::I32);
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
                cranelift_codegen::print_errors::pretty_verifier_error(&func, None, err);
            panic!("pretty_error:\n{pretty_error}");
        }
    }
}

#[test]
fn libcall_function() {
    let flag_builder = settings::builder();
    let isa_builder = cranelift_codegen::isa::lookup_by_name("x86_64-unknown-linux-gnu").unwrap();
    let isa = isa_builder
        .finish(settings::Flags::new(flag_builder))
        .unwrap();
    let mut module =
        ObjectModule::new(ObjectBuilder::new(isa, "foo", default_libcall_names()).unwrap());

    let sig = Signature {
        params: vec![],
        returns: vec![],
        call_conv: CallConv::SystemV,
    };

    let func_id = module
        .declare_function("function", Linkage::Local, &sig)
        .unwrap();

    let mut ctx = Context::new();
    ctx.func = Function::with_name_signature(UserFuncName::user(0, func_id.as_u32()), sig);
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

    module.define_function(func_id, &mut ctx).unwrap();

    module.finish();
}

#[test]
#[should_panic(expected = "has a null byte, which is disallowed")]
fn reject_nul_byte_symbol_for_func() {
    let flag_builder = settings::builder();
    let isa_builder = cranelift_codegen::isa::lookup_by_name("x86_64-unknown-linux-gnu").unwrap();
    let isa = isa_builder
        .finish(settings::Flags::new(flag_builder))
        .unwrap();
    let mut module =
        ObjectModule::new(ObjectBuilder::new(isa, "foo", default_libcall_names()).unwrap());

    let sig = Signature {
        params: vec![],
        returns: vec![],
        call_conv: CallConv::SystemV,
    };

    let _ = module
        .declare_function("function\u{0}with\u{0}nul\u{0}bytes", Linkage::Local, &sig)
        .unwrap();
}

#[test]
#[should_panic(expected = "has a null byte, which is disallowed")]
fn reject_nul_byte_symbol_for_data() {
    let flag_builder = settings::builder();
    let isa_builder = cranelift_codegen::isa::lookup_by_name("x86_64-unknown-linux-gnu").unwrap();
    let isa = isa_builder
        .finish(settings::Flags::new(flag_builder))
        .unwrap();
    let mut module =
        ObjectModule::new(ObjectBuilder::new(isa, "foo", default_libcall_names()).unwrap());

    let _ = module
        .declare_data(
            "data\u{0}with\u{0}nul\u{0}bytes",
            Linkage::Local,
            /* writable = */ true,
            /* tls = */ false,
        )
        .unwrap();
}

#[test]
fn aarch64_colocated_data_symbol_reloc() {
    let flag_builder = settings::builder();
    let isa_builder = cranelift_codegen::isa::lookup_by_name("aarch64-unknown-linux-gnu").unwrap();
    let isa = isa_builder
        .finish(settings::Flags::new(flag_builder))
        .unwrap();
    let mut module =
        ObjectModule::new(ObjectBuilder::new(isa, "test", default_libcall_names()).unwrap());

    let data_id = module
        .declare_data("my_data", Linkage::Local, true, false)
        .unwrap();

    let mut data_desc = DataDescription::new();
    data_desc.define_zeroinit(64);
    module.define_data(data_id, &data_desc).unwrap();

    let sig = Signature {
        params: vec![],
        returns: vec![AbiParam::new(types::I64)],
        call_conv: CallConv::SystemV,
    };

    let func_id = module
        .declare_function("load_data_addr", Linkage::Local, &sig)
        .unwrap();

    let mut ctx = Context::new();
    ctx.func = Function::with_name_signature(UserFuncName::user(0, func_id.as_u32()), sig);
    let mut func_ctx = FunctionBuilderContext::new();
    {
        let mut bcx: FunctionBuilder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
        let block = bcx.create_block();
        bcx.switch_to_block(block);

        let gv = module.declare_data_in_func(data_id, &mut bcx.func);
        let ptr = module.target_config().pointer_type();
        let addr = bcx.ins().global_value(ptr, gv);
        bcx.ins().return_(&[addr]);

        bcx.seal_all_blocks();
        bcx.finalize();
    }

    module.define_function(func_id, &mut ctx).unwrap();

    let product = module.finish();
    product.emit().expect("emit object file");
}

// ---------- `.eh_frame` emission tests ----------

#[cfg(feature = "unwind")]
mod eh_frame {
    use super::*;
    use cranelift_codegen::settings::Configurable as _;
    use gimli::UnwindSection as _;
    use object::Object as _;
    use object::ObjectSection as _;

    /// Build an `ObjectModule` for `triple` with the unwind-info builder flag
    /// set to `unwind_info`. Frame pointers are forced on so cranelift emits
    /// a non-trivial prologue (and therefore unwind info) for every function.
    fn module_for(triple: &str, unwind_info: bool) -> ObjectModule {
        let mut flag_builder = settings::builder();
        flag_builder.set("preserve_frame_pointers", "true").unwrap();
        let isa = cranelift_codegen::isa::lookup_by_name(triple)
            .unwrap()
            .finish(settings::Flags::new(flag_builder))
            .unwrap();
        let mut builder = ObjectBuilder::new(isa, "test", default_libcall_names()).unwrap();
        builder.unwind_info(unwind_info);
        ObjectModule::new(builder)
    }

    /// Define a leaf function that just returns. With `preserve_frame_pointers`
    /// on, this is enough to make cranelift emit a System V FDE for it.
    fn define_leaf(module: &mut ObjectModule, name: &str) -> FuncId {
        let sig = Signature {
            params: vec![],
            returns: vec![],
            call_conv: CallConv::SystemV,
        };
        let func_id = module.declare_function(name, Linkage::Local, &sig).unwrap();
        let mut ctx = Context::new();
        ctx.func = Function::with_name_signature(UserFuncName::user(0, func_id.as_u32()), sig);
        let mut func_ctx = FunctionBuilderContext::new();
        {
            let mut bcx = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
            let block = bcx.create_block();
            bcx.switch_to_block(block);
            bcx.ins().return_(&[]);
            bcx.seal_all_blocks();
            bcx.finalize();
        }
        module.define_function(func_id, &mut ctx).unwrap();
        func_id
    }

    /// Iterate the entries in `.eh_frame` bytes, returning `(cie_count, fde_count)`.
    fn count_entries(data: &[u8]) -> (usize, usize) {
        let mut eh_frame = gimli::EhFrame::new(data, gimli::LittleEndian);
        eh_frame.set_address_size(8);
        let bases = gimli::BaseAddresses::default();
        let mut entries = eh_frame.entries(&bases);
        let (mut cies, mut fdes) = (0, 0);
        while let Some(entry) = entries.next().expect("walk eh_frame entries") {
            match entry {
                gimli::CieOrFde::Cie(_) => cies += 1,
                gimli::CieOrFde::Fde(_) => fdes += 1,
            }
        }
        (cies, fdes)
    }

    #[test]
    fn emits_eh_frame_for_x86_64_systemv_target() {
        let mut module = module_for("x86_64-unknown-linux-gnu", true);
        define_leaf(&mut module, "leaf");

        let bytes = module.finish().emit().expect("emit object file");
        let file = object::File::parse(&*bytes).expect("parse emitted object");
        let section = file
            .section_by_name(".eh_frame")
            .expect(".eh_frame section is present");
        let data = section.data().expect("read .eh_frame data");
        assert!(!data.is_empty(), ".eh_frame must not be empty");

        let (cies, fdes) = count_entries(data);
        assert_eq!(cies, 1, "expected exactly one CIE");
        assert_eq!(fdes, 1, "expected exactly one FDE");

        let relocations: Vec<_> = section.relocations().collect();
        assert!(
            !relocations.is_empty(),
            ".eh_frame must carry symbol relocations"
        );
    }

    #[test]
    fn emits_eh_frame_for_aarch64_target() {
        let mut module = module_for("aarch64-unknown-linux-gnu", true);
        define_leaf(&mut module, "leaf");

        let bytes = module.finish().emit().expect("emit object file");
        let file = object::File::parse(&*bytes).expect("parse emitted object");
        let data = file
            .section_by_name(".eh_frame")
            .expect(".eh_frame section is present")
            .data()
            .expect("read .eh_frame data");

        let (cies, fdes) = count_entries(data);
        assert_eq!(cies, 1, "expected exactly one CIE");
        assert_eq!(fdes, 1, "expected exactly one FDE");
    }

    #[test]
    fn unwind_info_disabled_by_default_emits_no_eh_frame() {
        let flag_builder = settings::builder();
        let isa = cranelift_codegen::isa::lookup_by_name("x86_64-unknown-linux-gnu")
            .unwrap()
            .finish(settings::Flags::new(flag_builder))
            .unwrap();
        let mut module =
            ObjectModule::new(ObjectBuilder::new(isa, "no_eh", default_libcall_names()).unwrap());
        define_simple_function(&mut module);

        let bytes = module.finish().emit().expect("emit object file");
        let file = object::File::parse(&*bytes).expect("parse emitted object");
        assert!(
            file.section_by_name(".eh_frame").is_none(),
            "no .eh_frame section should be emitted when unwind_info is off"
        );
    }

    #[test]
    fn cie_is_shared_across_multiple_functions() {
        let mut module = module_for("x86_64-unknown-linux-gnu", true);
        define_leaf(&mut module, "leaf_a");
        define_leaf(&mut module, "leaf_b");
        define_leaf(&mut module, "leaf_c");

        let bytes = module.finish().emit().expect("emit object file");
        let file = object::File::parse(&*bytes).expect("parse emitted object");
        let data = file
            .section_by_name(".eh_frame")
            .expect(".eh_frame section is present")
            .data()
            .expect("read .eh_frame data");

        let (cies, fdes) = count_entries(data);
        assert_eq!(cies, 1, "all functions must share a single CIE, got {cies}");
        assert_eq!(fdes, 3, "expected one FDE per function, got {fdes}");
    }

    #[test]
    fn fde_relocations_target_function_symbols() {
        use object::ObjectSymbol as _;

        let mut module = module_for("x86_64-unknown-linux-gnu", true);
        define_leaf(&mut module, "leaf");

        let bytes = module.finish().emit().expect("emit object file");
        let file = object::File::parse(&*bytes).expect("parse emitted object");
        let section = file
            .section_by_name(".eh_frame")
            .expect(".eh_frame section is present");

        let leaf_symbol_index = file
            .symbols()
            .find(|s| s.name() == Ok("leaf"))
            .expect("function symbol present in object")
            .index();

        let (_, reloc) = section
            .relocations()
            .next()
            .expect(".eh_frame must carry at least one relocation");
        let object::read::RelocationTarget::Symbol(target_index) = reloc.target() else {
            panic!(
                "expected symbol-targeted relocation, got {:?}",
                reloc.target()
            );
        };
        assert_eq!(
            target_index, leaf_symbol_index,
            "FDE relocation must target the function's text symbol"
        );
    }

    #[test]
    fn per_function_section_combines_with_unwind_info() {
        // With every function in its own subsection, the FDE relocations must
        // still resolve because they target function symbols rather than the
        // shared `.text` section. Just walking the entries should succeed and
        // produce one FDE per function.
        let mut flag_builder = settings::builder();
        flag_builder.set("preserve_frame_pointers", "true").unwrap();
        let isa = cranelift_codegen::isa::lookup_by_name("x86_64-unknown-linux-gnu")
            .unwrap()
            .finish(settings::Flags::new(flag_builder))
            .unwrap();
        let mut builder = ObjectBuilder::new(isa, "test", default_libcall_names()).unwrap();
        builder.unwind_info(true);
        builder.per_function_section(true);
        let mut module = ObjectModule::new(builder);
        define_leaf(&mut module, "leaf_a");
        define_leaf(&mut module, "leaf_b");

        let bytes = module.finish().emit().expect("emit object file");
        let file = object::File::parse(&*bytes).expect("parse emitted object");
        let data = file
            .section_by_name(".eh_frame")
            .expect(".eh_frame section is present")
            .data()
            .expect("read .eh_frame data");
        let (cies, fdes) = count_entries(data);
        assert_eq!(cies, 1);
        assert_eq!(fdes, 2);
    }

    #[test]
    fn no_eh_frame_emitted_for_windows_target() {
        // On Windows targets cranelift produces `UnwindInfo::WindowsX64`
        // rather than System V info, and the unwind builder ignores those
        // variants. With no FDE ever added, no `.eh_frame` section should
        // appear in the resulting object even though `unwind_info(true)` was
        // set on the builder.
        let mut module = module_for("x86_64-pc-windows-msvc", true);
        let sig = Signature {
            params: vec![],
            returns: vec![],
            call_conv: CallConv::WindowsFastcall,
        };
        let func_id = module
            .declare_function("leaf", Linkage::Local, &sig)
            .unwrap();
        let mut ctx = Context::new();
        ctx.func = Function::with_name_signature(UserFuncName::user(0, func_id.as_u32()), sig);
        let mut func_ctx = FunctionBuilderContext::new();
        {
            let mut bcx = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
            let block = bcx.create_block();
            bcx.switch_to_block(block);
            bcx.ins().return_(&[]);
            bcx.seal_all_blocks();
            bcx.finalize();
        }
        module.define_function(func_id, &mut ctx).unwrap();

        let bytes = module.finish().emit().expect("emit object file");
        let file = object::File::parse(&*bytes).expect("parse emitted object");
        assert!(
            file.section_by_name(".eh_frame").is_none(),
            "no .eh_frame should be emitted for Windows targets"
        );
    }
}
