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
    assert_eq!(g.get(&mut store).v128(), Some(V128::from(0)));
    g.set(&mut store, 1u128.into())?;
    assert_eq!(g.get(&mut store).v128(), Some(V128::from(1)));
    Ok(())
}

#[test]
fn i31ref_global_new() -> Result<()> {
    let mut config = Config::new();
    config.wasm_function_references(true);
    config.wasm_gc(true);

    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());

    for mutability in [Mutability::Const, Mutability::Var] {
        for val in [
            Some(AnyRef::from_i31(&mut store, I31::new_u32(42).unwrap())),
            None,
        ] {
            Global::new(
                &mut store,
                GlobalType::new(ValType::I31REF, mutability),
                val.into(),
            )?;
        }
    }
    Ok(())
}

#[test]
fn i31ref_global_get() -> Result<()> {
    let mut config = Config::new();
    config.wasm_function_references(true);
    config.wasm_gc(true);

    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());

    for mutability in [Mutability::Const, Mutability::Var] {
        for val in [
            Some(AnyRef::from_i31(&mut store, I31::new_u32(42).unwrap())),
            None,
        ] {
            let val = Val::from(val);
            let global = Global::new(
                &mut store,
                GlobalType::new(ValType::I31REF, mutability),
                val,
            )?;

            let got = global.get(&mut store);

            let val = val
                .anyref()
                .and_then(|a| a.and_then(|a| a.as_i31(&store).unwrap()));
            let got = got
                .anyref()
                .and_then(|a| a.and_then(|a| a.as_i31(&store).unwrap()));

            assert_eq!(val, got);
        }
    }
    Ok(())
}

#[test]
fn i31ref_global_set() -> Result<()> {
    let mut config = Config::new();
    config.wasm_function_references(true);
    config.wasm_gc(true);

    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());

    for init in [
        Some(AnyRef::from_i31(&mut store, I31::new_u32(42).unwrap())),
        None,
    ] {
        for new_val in [
            Some(AnyRef::from_i31(&mut store, I31::new_u32(36).unwrap())),
            None,
        ] {
            let global = Global::new(
                &mut store,
                GlobalType::new(ValType::I31REF, Mutability::Var),
                init.into(),
            )?;

            let new_val = Val::from(new_val);
            global.set(&mut store, new_val)?;
            let got = global.get(&mut store);

            let new_val = new_val
                .anyref()
                .and_then(|a| a.and_then(|a| a.as_i31(&store).unwrap()));
            let got = got
                .anyref()
                .and_then(|a| a.and_then(|a| a.as_i31(&store).unwrap()));

            assert_eq!(new_val, got);
        }
    }
    Ok(())
}

#[test]
fn i31ref_global_ty() -> Result<()> {
    let mut config = Config::new();
    config.wasm_function_references(true);
    config.wasm_gc(true);

    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());

    for mutability in [Mutability::Const, Mutability::Var] {
        for val in [
            Some(AnyRef::from_i31(&mut store, I31::new_u32(42).unwrap())),
            None,
        ] {
            let expected_ty = GlobalType::new(ValType::I31REF, mutability);
            let global = Global::new(&mut store, expected_ty.clone(), val.into())?;
            let actual_ty = global.ty(&store);
            assert_eq!(expected_ty.mutability(), actual_ty.mutability());
            assert!(ValType::eq(expected_ty.content(), actual_ty.content()));
        }
    }
    Ok(())
}

#[test]
fn i31ref_as_anyref_global_new() -> Result<()> {
    let mut config = Config::new();
    config.wasm_function_references(true);
    config.wasm_gc(true);

    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());

    for mutability in [Mutability::Const, Mutability::Var] {
        for val in [
            Some(AnyRef::from_i31(&mut store, I31::new_u32(42).unwrap())),
            None,
        ] {
            Global::new(
                &mut store,
                GlobalType::new(ValType::ANYREF, mutability),
                val.into(),
            )?;
        }
    }
    Ok(())
}

#[test]
fn i31ref_as_anyref_global_get() -> Result<()> {
    let mut config = Config::new();
    config.wasm_function_references(true);
    config.wasm_gc(true);

    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());

    for mutability in [Mutability::Const, Mutability::Var] {
        for val in [
            Some(AnyRef::from_i31(&mut store, I31::new_u32(42).unwrap())),
            None,
        ] {
            let val = Val::from(val);
            let global = Global::new(
                &mut store,
                GlobalType::new(ValType::ANYREF, mutability),
                val,
            )?;

            let got = global.get(&mut store);

            let val = val
                .anyref()
                .and_then(|a| a.and_then(|a| a.as_i31(&store).unwrap()));
            let got = got
                .anyref()
                .and_then(|a| a.and_then(|a| a.as_i31(&store).unwrap()));

            assert_eq!(val, got);
        }
    }
    Ok(())
}

#[test]
fn i31ref_as_anyref_global_set() -> Result<()> {
    let mut config = Config::new();
    config.wasm_function_references(true);
    config.wasm_gc(true);

    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());

    for init in [
        Some(AnyRef::from_i31(&mut store, I31::new_u32(42).unwrap())),
        None,
    ] {
        for new_val in [
            Some(AnyRef::from_i31(&mut store, I31::new_u32(36).unwrap())),
            None,
        ] {
            let global = Global::new(
                &mut store,
                GlobalType::new(ValType::ANYREF, Mutability::Var),
                init.into(),
            )?;

            let new_val = Val::from(new_val);
            global.set(&mut store, new_val)?;
            let got = global.get(&mut store);

            let new_val = new_val
                .anyref()
                .and_then(|a| a.and_then(|a| a.as_i31(&store).unwrap()));
            let got = got
                .anyref()
                .and_then(|a| a.and_then(|a| a.as_i31(&store).unwrap()));

            assert_eq!(new_val, got);
        }
    }
    Ok(())
}

#[test]
fn i31ref_as_anyref_global_ty() -> Result<()> {
    let mut config = Config::new();
    config.wasm_function_references(true);
    config.wasm_gc(true);

    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());

    for mutability in [Mutability::Const, Mutability::Var] {
        for val in [
            Some(AnyRef::from_i31(&mut store, I31::new_u32(42).unwrap())),
            None,
        ] {
            let expected_ty = GlobalType::new(ValType::ANYREF, mutability);
            let global = Global::new(&mut store, expected_ty.clone(), val.into())?;
            let actual_ty = global.ty(&store);
            assert_eq!(expected_ty.mutability(), actual_ty.mutability());
            assert!(ValType::eq(expected_ty.content(), actual_ty.content()));
        }
    }
    Ok(())
}
