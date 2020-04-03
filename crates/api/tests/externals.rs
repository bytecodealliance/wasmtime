use wasmtime::*;

#[test]
fn bad_globals() {
    let ty = GlobalType::new(ValType::I32, Mutability::Var);
    assert!(Global::new(&Store::default(), ty.clone(), Val::I64(0)).is_err());
    assert!(Global::new(&Store::default(), ty.clone(), Val::F32(0)).is_err());
    assert!(Global::new(&Store::default(), ty.clone(), Val::F64(0)).is_err());

    let ty = GlobalType::new(ValType::I32, Mutability::Const);
    let g = Global::new(&Store::default(), ty.clone(), Val::I32(0)).unwrap();
    assert!(g.set(Val::I32(1)).is_err());

    let ty = GlobalType::new(ValType::I32, Mutability::Var);
    let g = Global::new(&Store::default(), ty.clone(), Val::I32(0)).unwrap();
    assert!(g.set(Val::I64(0)).is_err());
}

#[test]
fn bad_tables() {
    // i32 not supported yet
    let ty = TableType::new(ValType::I32, Limits::new(0, Some(1)));
    assert!(Table::new(&Store::default(), ty.clone(), Val::I32(0)).is_err());

    // mismatched initializer
    let ty = TableType::new(ValType::FuncRef, Limits::new(0, Some(1)));
    assert!(Table::new(&Store::default(), ty.clone(), Val::I32(0)).is_err());

    // get out of bounds
    let ty = TableType::new(ValType::FuncRef, Limits::new(0, Some(1)));
    let t = Table::new(&Store::default(), ty.clone(), Val::AnyRef(AnyRef::Null)).unwrap();
    assert!(t.get(0).is_none());
    assert!(t.get(u32::max_value()).is_none());

    // set out of bounds or wrong type
    let ty = TableType::new(ValType::FuncRef, Limits::new(1, Some(1)));
    let t = Table::new(&Store::default(), ty.clone(), Val::AnyRef(AnyRef::Null)).unwrap();
    assert!(t.set(0, Val::I32(0)).is_err());
    assert!(t.set(0, Val::AnyRef(AnyRef::Null)).is_ok());
    assert!(t.set(1, Val::AnyRef(AnyRef::Null)).is_err());

    // grow beyond max
    let ty = TableType::new(ValType::FuncRef, Limits::new(1, Some(1)));
    let t = Table::new(&Store::default(), ty.clone(), Val::AnyRef(AnyRef::Null)).unwrap();
    assert!(t.grow(0, Val::AnyRef(AnyRef::Null)).is_ok());
    assert!(t.grow(1, Val::AnyRef(AnyRef::Null)).is_err());
    assert_eq!(t.size(), 1);

    // grow wrong type
    let ty = TableType::new(ValType::FuncRef, Limits::new(1, Some(2)));
    let t = Table::new(&Store::default(), ty.clone(), Val::AnyRef(AnyRef::Null)).unwrap();
    assert!(t.grow(1, Val::I32(0)).is_err());
    assert_eq!(t.size(), 1);
}

#[test]
fn cross_store() -> anyhow::Result<()> {
    let mut cfg = Config::new();
    cfg.wasm_reference_types(true);
    let store1 = Store::new(&Engine::new(&cfg));
    let store2 = Store::new(&Engine::new(&cfg));

    // ============ Cross-store instantiation ==============

    let func = Func::wrap(&store2, || {});
    let ty = GlobalType::new(ValType::I32, Mutability::Const);
    let global = Global::new(&store2, ty, Val::I32(0))?;
    let ty = MemoryType::new(Limits::new(1, None));
    let memory = Memory::new(&store2, ty);
    let ty = TableType::new(ValType::FuncRef, Limits::new(1, None));
    let table = Table::new(&store2, ty, Val::AnyRef(AnyRef::Null))?;

    let need_func = Module::new(&store1, r#"(module (import "" "" (func)))"#)?;
    assert!(Instance::new(&need_func, &[func.into()]).is_err());

    let need_global = Module::new(&store1, r#"(module (import "" "" (global i32)))"#)?;
    assert!(Instance::new(&need_global, &[global.into()]).is_err());

    let need_table = Module::new(&store1, r#"(module (import "" "" (table 1 funcref)))"#)?;
    assert!(Instance::new(&need_table, &[table.into()]).is_err());

    let need_memory = Module::new(&store1, r#"(module (import "" "" (memory 1)))"#)?;
    assert!(Instance::new(&need_memory, &[memory.into()]).is_err());

    // ============ Cross-store globals ==============

    let store1val = Val::FuncRef(Func::wrap(&store1, || {}));
    let store2val = Val::FuncRef(Func::wrap(&store2, || {}));

    let ty = GlobalType::new(ValType::FuncRef, Mutability::Var);
    assert!(Global::new(&store2, ty.clone(), store1val.clone()).is_err());
    if let Ok(g) = Global::new(&store2, ty.clone(), store2val.clone()) {
        assert!(g.set(store1val.clone()).is_err());
    }

    // ============ Cross-store tables ==============

    let ty = TableType::new(ValType::FuncRef, Limits::new(1, None));
    assert!(Table::new(&store2, ty.clone(), store1val.clone()).is_err());
    let t1 = Table::new(&store2, ty.clone(), store2val.clone())?;
    assert!(t1.set(0, store1val.clone()).is_err());
    assert!(t1.grow(0, store1val.clone()).is_err());
    let t2 = Table::new(&store1, ty.clone(), store1val.clone())?;
    assert!(Table::copy(&t1, 0, &t2, 0, 0).is_err());

    // ============ Cross-store funcs ==============

    // TODO: need to actually fill this out once we support anyref params/locals
    // let module = Module::new(&store1, r#"(module (func (export "a") (param funcref)))"#)?;

    Ok(())
}
