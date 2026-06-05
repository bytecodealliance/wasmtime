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
