use wasmtime::*;

#[test]
fn smoke() -> anyhow::Result<()> {
    let mut store = Store::<()>::default();
    let g = Global::new(
        &mut store,
        GlobalType::new(ValType::I32, Mutability::Const),
        0.into(),
    )?;
    assert_eq!(g.get(&mut store).i32(), Some(0));
    assert!(g.set(&mut store, 0.into()).is_err());

    let g = Global::new(
        &mut store,
        GlobalType::new(ValType::I32, Mutability::Const),
        1i32.into(),
    )?;
    assert_eq!(g.get(&mut store).i32(), Some(1));

    let g = Global::new(
        &mut store,
        GlobalType::new(ValType::I64, Mutability::Const),
        2i64.into(),
    )?;
    assert_eq!(g.get(&mut store).i64(), Some(2));

    let g = Global::new(
        &mut store,
        GlobalType::new(ValType::F32, Mutability::Const),
        3.0f32.into(),
    )?;
    assert_eq!(g.get(&mut store).f32(), Some(3.0));

    let g = Global::new(
        &mut store,
        GlobalType::new(ValType::F64, Mutability::Const),
        4.0f64.into(),
    )?;
    assert_eq!(g.get(&mut store).f64(), Some(4.0));
    Ok(())
}

#[test]
fn mutability() -> anyhow::Result<()> {
    let mut store = Store::<()>::default();
    let g = Global::new(
        &mut store,
        GlobalType::new(ValType::I32, Mutability::Var),
        0.into(),
    )?;
    assert_eq!(g.get(&mut store).i32(), Some(0));
    g.set(&mut store, 1.into())?;
    assert_eq!(g.get(&mut store).i32(), Some(1));
    Ok(())
}

// Make sure that a global is still usable after its original instance is
// dropped. This is a bit of a weird test and really only fails depending on the
// implementation, but for now should hopefully be resilient enough to catch at
// least some cases of heap corruption.
#[test]
fn use_after_drop() -> anyhow::Result<()> {
    let mut store = Store::<()>::default();
    let module = Module::new(
        store.engine(),
        r#"
            (module
                (global (export "foo") (mut i32) (i32.const 100)))
        "#,
    )?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let g = instance.get_global(&mut store, "foo").unwrap();
    assert_eq!(g.get(&mut store).i32(), Some(100));
    g.set(&mut store, 101.into())?;
    drop(instance);
    assert_eq!(g.get(&mut store).i32(), Some(101));
    Instance::new(&mut store, &module, &[])?;
    assert_eq!(g.get(&mut store).i32(), Some(101));
    drop(module);
    assert_eq!(g.get(&mut store).i32(), Some(101));

    // spray some heap values
    let mut x = Vec::new();
    for _ in 0..100 {
        x.push("xy".to_string());
    }
    drop(x);
    assert_eq!(g.get(&mut store).i32(), Some(101));
    Ok(())
}

#[test]
fn v128() -> anyhow::Result<()> {
    let mut store = Store::<()>::default();
    let g = Global::new(
        &mut store,
        GlobalType::new(ValType::V128, Mutability::Var),
        0u128.into(),
    )?;
    assert_eq!(g.get(&mut store).v128(), Some(0));
    g.set(&mut store, 1u128.into())?;
    assert_eq!(g.get(&mut store).v128(), Some(1));
    Ok(())
}
