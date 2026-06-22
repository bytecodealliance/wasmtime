use crate::ErrorExt;
use wasmtime::*;

#[test]
#[cfg_attr(miri, ignore)]
fn wasm_export_tags() -> Result<()> {
    let source = r#"
            (module
                (tag (export "t1") (param i32))
                (tag (export "t2") (param i32))
                (tag (export "t3") (param i64))
            )
        "#;
    let _ = env_logger::try_init();
    let mut config = Config::new();
    config.wasm_exceptions(true);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());
    let module = Module::new(&engine, source)?;

    let instance = Instance::new(&mut store, &module, &[])?;
    let t1 = instance.get_tag(&mut store, "t1");
    assert!(t1.is_some());
    let t1 = t1.unwrap();

    let t2 = instance.get_tag(&mut store, "t2");
    assert!(t2.is_some());
    let t2 = t2.unwrap();

    let t1_ty = t1.ty(&store);
    let t2_ty = t2.ty(&store);
    assert!(Tag::eq(&t1, &t1, &store));
    assert!(!Tag::eq(&t1, &t2, &store));
    assert!(FuncType::eq(t1_ty.ty(), t2_ty.ty()));

    let t3 = instance.get_tag(&mut store, "t3");
    assert!(t3.is_some());
    let t3 = t3.unwrap();
    let t3_ty = t3.ty(&store);
    assert!(Tag::eq(&t3, &t3, &store));
    assert!(!Tag::eq(&t3, &t1, &store));
    assert!(!Tag::eq(&t3, &t2, &store));
    assert!(!FuncType::eq(t1_ty.ty(), t3_ty.ty()));

    return Ok(());
}

#[test]
#[cfg_attr(miri, ignore)]
fn wasm_import_tags() -> Result<()> {
    let m1_src = r#"
            (module
                (tag (export "t1") (param i32))
            )
        "#;
    let m2_src = r#"
            (module
                (tag (export "t1_2") (import "" "") (param i32))
                (tag (export "t1_22") (import "" "") (param i32))
                (tag (export "t2") (param i32))
            )
        "#;
    let _ = env_logger::try_init();
    let mut config = Config::new();
    config.wasm_exceptions(true);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());
    let m1 = Module::new(&engine, m1_src)?;
    let m2 = Module::new(&engine, m2_src)?;

    let m1_instance = Instance::new(&mut store, &m1, &[])?;
    let t1 = m1_instance.get_tag(&mut store, "t1").unwrap();
    let m2_instance = Instance::new(&mut store, &m2, &[t1.into(), t1.into()])?;
    let t1_2 = m2_instance.get_tag(&mut store, "t1_2").unwrap();
    assert!(Tag::eq(&t1, &t1_2, &store));
    let t1_22 = m2_instance.get_tag(&mut store, "t1_22").unwrap();
    assert!(Tag::eq(&t1, &t1_22, &store));
    assert!(Tag::eq(&t1_2, &t1_22, &store));

    return Ok(());
}

// Tests that `cont.new` with a function type whose arity exceeds the
// continuation stack size produces an error rather than writing past the
// stack allocation.
#[test]
#[cfg_attr(miri, ignore)]
fn stack_switching_cont_new_high_arity_rejected() -> Result<()> {
    let mut config = Config::new();
    config.wasm_stack_switching(true);
    config.wasm_exceptions(true);
    config.wasm_function_references(true);

    // Use a small continuation stack so we can overflow it with fewer
    // than 1000 params (the wasmparser limit for function params).
    // With async_stack_size = 8192:
    //   VMContinuationStack::new rounds to page size (8192), adds a
    //   guard page, so total mmap = 12288 but usable = 8192.
    //   800 params * 16 bytes + 64 byte header = 12864 > 8192.
    config.async_stack_size(8192);
    config.max_wasm_stack(4096);

    let Ok(engine) = Engine::new(&config) else {
        // Stack switching is not supported on all platforms; skip gracefully.
        assert!(!(cfg!(target_arch = "x86_64") && cfg!(unix)));
        return Ok(());
    };

    // Build a WAT module with a high-arity function type.
    // 800 params stays under wasmparser's MAX_WASM_FUNCTION_PARAMS (1000)
    // but exceeds the 8192-byte usable stack space.
    let n_params = 800;
    let params: String = (0..n_params).map(|_| " i32").collect();
    let wat = format!(
        r#"(module
            (type $ft (func (param{params})))
            (type $ct (cont $ft))
            (func $target (type $ft))
            (elem declare func $target)
            (func (export "run")
                (drop (cont.new $ct (ref.func $target)))
            )
        )"#
    );

    let module = Module::new(&engine, &wat)?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;
    let run = instance.get_typed_func::<(), ()>(&mut store, "run")?;

    let err = run.call(&mut store, ()).unwrap_err();
    err.assert_contains("exceeds");

    return Ok(());
}

// Regression test for #13703: with async_stack_size=8192 and 600 params, the
// control data (600 * 16 + 64 = 9664 bytes) fits within the total mmap
// allocation (12288 = 8192 + 4096 guard) but exceeds the usable stack space
// (8192). Before the fix, the bounds check compared against self.len (which
// includes the guard page), so this case passed the check and then wrote
// into the guard page, causing a segfault.
#[test]
#[cfg_attr(miri, ignore)]
fn stack_switching_cont_new_guard_page_arity_rejected() -> Result<()> {
    let mut config = Config::new();
    config.wasm_stack_switching(true);
    config.wasm_exceptions(true);
    config.wasm_function_references(true);
    config.async_stack_size(8192);
    config.max_wasm_stack(4096);

    let Ok(engine) = Engine::new(&config) else {
        // Stack switching is not supported on all platforms; skip gracefully.
        assert!(!(cfg!(target_arch = "x86_64") && cfg!(unix)));
        return Ok(());
    };

    // 600 params: control data = 600 * 16 + 64 = 9664 bytes.
    // This exceeds the 8192-byte usable stack but fits within the
    // 12288-byte total allocation (including guard page).
    let n_params = 600;
    let params: String = (0..n_params).map(|_| " i32").collect();
    let wat = format!(
        r#"(module
            (type $ft (func (param{params})))
            (type $ct (cont $ft))
            (func $target (type $ft))
            (elem declare func $target)
            (func (export "run")
                (drop (cont.new $ct (ref.func $target)))
            )
        )"#
    );

    let module = Module::new(&engine, &wat)?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;
    let run = instance.get_typed_func::<(), ()>(&mut store, "run")?;

    let err = run.call(&mut store, ()).unwrap_err();
    err.assert_contains("exceeds");

    return Ok(());
}

// Tests that enabling inlining with stack switching, for now, returns an error.
// If the support in Cranelift is fixed to the point that this is fine to
// enable, then delete this test and the check in `config.rs` as well.
#[test]
fn stack_switching_disallows_inlining() -> Result<()> {
    let mut config = Config::new();
    config.wasm_stack_switching(true);
    config.compiler_inlining(wasmtime::Inlining::Yes);
    assert!(Engine::new(&config).is_err());
    return Ok(());
}

#[test]
#[cfg_attr(miri, ignore)]
fn issue_13474_create_tag_without_gc_runtime_configured() -> Result<()> {
    let mut config = Config::new();
    config.strategy(Strategy::Winch);
    // Ignore targets that don't have support for Winch just yet
    let Ok(engine) = Engine::new(&config) else {
        return Ok(());
    };
    let mut store = Store::new(&engine, ());
    let fty = FuncType::new(&engine, [], []);
    let tty1 = TagType::new(fty.clone());
    let result = Tag::new(&mut store, &tty1.clone());
    result
        .unwrap_err()
        .assert_contains("cannot define `ExnType`s without a GC runtime enabled");
    Ok(())
}
