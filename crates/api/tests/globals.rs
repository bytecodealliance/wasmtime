use wasmtime::*;

#[test]
fn smoke() -> anyhow::Result<()> {
    let store = Store::default();
    let g = Global::new(
        &store,
        GlobalType::new(ValType::I32, Mutability::Const),
        0.into(),
    )?;
    assert_eq!(g.get().i32(), Some(0));
    assert!(g.set(0.into()).is_err());

    let g = Global::new(
        &store,
        GlobalType::new(ValType::I32, Mutability::Const),
        1i32.into(),
    )?;
    assert_eq!(g.get().i32(), Some(1));

    let g = Global::new(
        &store,
        GlobalType::new(ValType::I64, Mutability::Const),
        2i64.into(),
    )?;
    assert_eq!(g.get().i64(), Some(2));

    let g = Global::new(
        &store,
        GlobalType::new(ValType::F32, Mutability::Const),
        3.0f32.into(),
    )?;
    assert_eq!(g.get().f32(), Some(3.0));

    let g = Global::new(
        &store,
        GlobalType::new(ValType::F64, Mutability::Const),
        4.0f64.into(),
    )?;
    assert_eq!(g.get().f64(), Some(4.0));
    Ok(())
}

#[test]
fn mutability() -> anyhow::Result<()> {
    let store = Store::default();
    let g = Global::new(
        &store,
        GlobalType::new(ValType::I32, Mutability::Var),
        0.into(),
    )?;
    assert_eq!(g.get().i32(), Some(0));
    g.set(1.into())?;
    assert_eq!(g.get().i32(), Some(1));
    Ok(())
}

// Make sure that a global is still usable after its original instance is
// dropped. This is a bit of a weird test and really only fails depending on the
// implementation, but for now should hopefully be resilient enough to catch at
// least some cases of heap corruption.
#[test]
fn use_after_drop() -> anyhow::Result<()> {
    let store = Store::default();
    let module = Module::new(
        &store,
        r#"
            (module
                (global (export "foo") (mut i32) (i32.const 100)))
        "#,
    )?;
    let instance = Instance::new(&module, &[])?;
    let g = instance.exports()[0].global().unwrap().clone();
    assert_eq!(g.get().i32(), Some(100));
    g.set(101.into())?;
    drop(instance);
    assert_eq!(g.get().i32(), Some(101));
    Instance::new(&module, &[])?;
    assert_eq!(g.get().i32(), Some(101));
    drop(module);
    assert_eq!(g.get().i32(), Some(101));
    drop(store);
    assert_eq!(g.get().i32(), Some(101));

    // spray some heap values
    let mut x = Vec::new();
    for _ in 0..100 {
        x.push("xy".to_string());
    }
    drop(x);
    assert_eq!(g.get().i32(), Some(101));
    Ok(())
}
