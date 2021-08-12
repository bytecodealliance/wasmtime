use wasmtime::*;

#[test]
fn bad_globals() {
    let mut store = Store::<()>::default();
    let ty = GlobalType::new(ValType::I32, Mutability::Var);
    assert!(Global::new(&mut store, ty.clone(), Val::I64(0)).is_err());
    assert!(Global::new(&mut store, ty.clone(), Val::F32(0)).is_err());
    assert!(Global::new(&mut store, ty.clone(), Val::F64(0)).is_err());

    let ty = GlobalType::new(ValType::I32, Mutability::Const);
    let g = Global::new(&mut store, ty.clone(), Val::I32(0)).unwrap();
    assert!(g.set(&mut store, Val::I32(1)).is_err());

    let ty = GlobalType::new(ValType::I32, Mutability::Var);
    let g = Global::new(&mut store, ty.clone(), Val::I32(0)).unwrap();
    assert!(g.set(&mut store, Val::I64(0)).is_err());
}

#[test]
fn bad_tables() {
    let mut store = Store::<()>::default();

    // i32 not supported yet
    let ty = TableType::new(ValType::I32, 0, Some(1));
    assert!(Table::new(&mut store, ty.clone(), Val::I32(0)).is_err());

    // mismatched initializer
    let ty = TableType::new(ValType::FuncRef, 0, Some(1));
    assert!(Table::new(&mut store, ty.clone(), Val::I32(0)).is_err());

    // get out of bounds
    let ty = TableType::new(ValType::FuncRef, 0, Some(1));
    let t = Table::new(&mut store, ty.clone(), Val::FuncRef(None)).unwrap();
    assert!(t.get(&mut store, 0).is_none());
    assert!(t.get(&mut store, u32::max_value()).is_none());

    // set out of bounds or wrong type
    let ty = TableType::new(ValType::FuncRef, 1, Some(1));
    let t = Table::new(&mut store, ty.clone(), Val::FuncRef(None)).unwrap();
    assert!(t.set(&mut store, 0, Val::I32(0)).is_err());
    assert!(t.set(&mut store, 0, Val::FuncRef(None)).is_ok());
    assert!(t.set(&mut store, 1, Val::FuncRef(None)).is_err());

    // grow beyond max
    let ty = TableType::new(ValType::FuncRef, 1, Some(1));
    let t = Table::new(&mut store, ty.clone(), Val::FuncRef(None)).unwrap();
    assert!(t.grow(&mut store, 0, Val::FuncRef(None)).is_ok());
    assert!(t.grow(&mut store, 1, Val::FuncRef(None)).is_err());
    assert_eq!(t.size(&store), 1);

    // grow wrong type
    let ty = TableType::new(ValType::FuncRef, 1, Some(2));
    let t = Table::new(&mut store, ty.clone(), Val::FuncRef(None)).unwrap();
    assert!(t.grow(&mut store, 1, Val::I32(0)).is_err());
    assert_eq!(t.size(&store), 1);
}

#[test]
fn cross_store() -> anyhow::Result<()> {
    let mut cfg = Config::new();
    cfg.wasm_reference_types(true);
    let engine = Engine::new(&cfg)?;
    let mut store1 = Store::new(&engine, ());
    let mut store2 = Store::new(&engine, ());

    // ============ Cross-store instantiation ==============

    let func = Func::wrap(&mut store2, || {});
    let ty = GlobalType::new(ValType::I32, Mutability::Const);
    let global = Global::new(&mut store2, ty, Val::I32(0))?;
    let ty = MemoryType::new(1, None);
    let memory = Memory::new(&mut store2, ty)?;
    let ty = TableType::new(ValType::FuncRef, 1, None);
    let table = Table::new(&mut store2, ty, Val::FuncRef(None))?;

    let need_func = Module::new(&engine, r#"(module (import "" "" (func)))"#)?;
    assert!(Instance::new(&mut store1, &need_func, &[func.into()]).is_err());

    let need_global = Module::new(&engine, r#"(module (import "" "" (global i32)))"#)?;
    assert!(Instance::new(&mut store1, &need_global, &[global.into()]).is_err());

    let need_table = Module::new(&engine, r#"(module (import "" "" (table 1 funcref)))"#)?;
    assert!(Instance::new(&mut store1, &need_table, &[table.into()]).is_err());

    let need_memory = Module::new(&engine, r#"(module (import "" "" (memory 1)))"#)?;
    assert!(Instance::new(&mut store1, &need_memory, &[memory.into()]).is_err());

    // ============ Cross-store globals ==============

    let store1val = Val::FuncRef(Some(Func::wrap(&mut store1, || {})));
    let store2val = Val::FuncRef(Some(Func::wrap(&mut store2, || {})));

    let ty = GlobalType::new(ValType::FuncRef, Mutability::Var);
    assert!(Global::new(&mut store2, ty.clone(), store1val.clone()).is_err());
    if let Ok(g) = Global::new(&mut store2, ty.clone(), store2val.clone()) {
        assert!(g.set(&mut store2, store1val.clone()).is_err());
    }

    // ============ Cross-store tables ==============

    let ty = TableType::new(ValType::FuncRef, 1, None);
    assert!(Table::new(&mut store2, ty.clone(), store1val.clone()).is_err());
    let t1 = Table::new(&mut store2, ty.clone(), store2val.clone())?;
    assert!(t1.set(&mut store2, 0, store1val.clone()).is_err());
    assert!(t1.grow(&mut store2, 0, store1val.clone()).is_err());
    assert!(t1.fill(&mut store2, 0, store1val.clone(), 1).is_err());

    // ============ Cross-store funcs ==============

    let module = Module::new(&engine, r#"(module (func (export "f") (param funcref)))"#)?;
    let s1_inst = Instance::new(&mut store1, &module, &[])?;
    let s2_inst = Instance::new(&mut store2, &module, &[])?;
    let s1_f = s1_inst.get_func(&mut store1, "f").unwrap();
    let s2_f = s2_inst.get_func(&mut store2, "f").unwrap();

    assert!(s1_f.call(&mut store1, &[Val::FuncRef(None)]).is_ok());
    assert!(s2_f.call(&mut store2, &[Val::FuncRef(None)]).is_ok());
    assert!(s1_f.call(&mut store1, &[Some(s1_f.clone()).into()]).is_ok());
    assert!(s1_f
        .call(&mut store1, &[Some(s2_f.clone()).into()])
        .is_err());
    assert!(s2_f
        .call(&mut store2, &[Some(s1_f.clone()).into()])
        .is_err());
    assert!(s2_f.call(&mut store2, &[Some(s2_f.clone()).into()]).is_ok());

    let s1_f_t = s1_f.typed::<Option<Func>, (), _>(&store1)?;
    let s2_f_t = s2_f.typed::<Option<Func>, (), _>(&store2)?;

    assert!(s1_f_t.call(&mut store1, None).is_ok());
    assert!(s2_f_t.call(&mut store2, None).is_ok());
    assert!(s1_f_t.call(&mut store1, Some(s1_f.clone())).is_ok());
    assert!(s1_f_t.call(&mut store1, Some(s2_f.clone())).is_err());
    assert!(s2_f_t.call(&mut store2, Some(s1_f.clone())).is_err());
    assert!(s2_f_t.call(&mut store2, Some(s2_f.clone())).is_ok());

    Ok(())
}

#[test]
fn get_set_externref_globals_via_api() -> anyhow::Result<()> {
    let mut cfg = Config::new();
    cfg.wasm_reference_types(true);
    let engine = Engine::new(&cfg)?;
    let mut store = Store::new(&engine, ());

    // Initialize with a null externref.

    let global = Global::new(
        &mut store,
        GlobalType::new(ValType::ExternRef, Mutability::Var),
        Val::ExternRef(None),
    )?;
    assert!(global.get(&mut store).unwrap_externref().is_none());

    global.set(
        &mut store,
        Val::ExternRef(Some(ExternRef::new("hello".to_string()))),
    )?;
    let r = global.get(&mut store).unwrap_externref().unwrap();
    assert!(r.data().is::<String>());
    assert_eq!(r.data().downcast_ref::<String>().unwrap(), "hello");

    // Initialize with a non-null externref.

    let global = Global::new(
        &mut store,
        GlobalType::new(ValType::ExternRef, Mutability::Const),
        Val::ExternRef(Some(ExternRef::new(42_i32))),
    )?;
    let r = global.get(&mut store).unwrap_externref().unwrap();
    assert!(r.data().is::<i32>());
    assert_eq!(r.data().downcast_ref::<i32>().copied().unwrap(), 42);

    Ok(())
}

#[test]
fn get_set_funcref_globals_via_api() -> anyhow::Result<()> {
    let mut cfg = Config::new();
    cfg.wasm_reference_types(true);
    let engine = Engine::new(&cfg)?;
    let mut store = Store::new(&engine, ());

    let f = Func::wrap(&mut store, || {});

    // Initialize with a null funcref.

    let global = Global::new(
        &mut store,
        GlobalType::new(ValType::FuncRef, Mutability::Var),
        Val::FuncRef(None),
    )?;
    assert!(global.get(&mut store).unwrap_funcref().is_none());

    global.set(&mut store, Val::FuncRef(Some(f.clone())))?;
    let f2 = global.get(&mut store).unwrap_funcref().cloned().unwrap();
    assert_eq!(f.ty(&store), f2.ty(&store));

    // Initialize with a non-null funcref.

    let global = Global::new(
        &mut store,
        GlobalType::new(ValType::FuncRef, Mutability::Var),
        Val::FuncRef(Some(f.clone())),
    )?;
    let f2 = global.get(&mut store).unwrap_funcref().cloned().unwrap();
    assert_eq!(f.ty(&store), f2.ty(&store));

    Ok(())
}

#[test]
fn create_get_set_funcref_tables_via_api() -> anyhow::Result<()> {
    let mut cfg = Config::new();
    cfg.wasm_reference_types(true);
    let engine = Engine::new(&cfg)?;
    let mut store = Store::new(&engine, ());

    let table_ty = TableType::new(ValType::FuncRef, 10, None);
    let init = Val::FuncRef(Some(Func::wrap(&mut store, || {})));
    let table = Table::new(&mut store, table_ty, init)?;

    assert!(table.get(&mut store, 5).unwrap().unwrap_funcref().is_some());
    table.set(&mut store, 5, Val::FuncRef(None))?;
    assert!(table.get(&mut store, 5).unwrap().unwrap_funcref().is_none());

    Ok(())
}

#[test]
fn fill_funcref_tables_via_api() -> anyhow::Result<()> {
    let mut cfg = Config::new();
    cfg.wasm_reference_types(true);
    let engine = Engine::new(&cfg)?;
    let mut store = Store::new(&engine, ());

    let table_ty = TableType::new(ValType::FuncRef, 10, None);
    let table = Table::new(&mut store, table_ty, Val::FuncRef(None))?;

    for i in 0..10 {
        assert!(table.get(&mut store, i).unwrap().unwrap_funcref().is_none());
    }

    let fill = Val::FuncRef(Some(Func::wrap(&mut store, || {})));
    table.fill(&mut store, 2, fill, 4)?;

    for i in (0..2).chain(7..10) {
        assert!(table.get(&mut store, i).unwrap().unwrap_funcref().is_none());
    }
    for i in 2..6 {
        assert!(table.get(&mut store, i).unwrap().unwrap_funcref().is_some());
    }

    Ok(())
}

#[test]
fn grow_funcref_tables_via_api() -> anyhow::Result<()> {
    let mut cfg = Config::new();
    cfg.wasm_reference_types(true);
    let engine = Engine::new(&cfg)?;
    let mut store = Store::new(&engine, ());

    let table_ty = TableType::new(ValType::FuncRef, 10, None);
    let table = Table::new(&mut store, table_ty, Val::FuncRef(None))?;

    assert_eq!(table.size(&store), 10);
    table.grow(&mut store, 3, Val::FuncRef(None))?;
    assert_eq!(table.size(&store), 13);

    Ok(())
}

#[test]
fn create_get_set_externref_tables_via_api() -> anyhow::Result<()> {
    let mut cfg = Config::new();
    cfg.wasm_reference_types(true);
    let engine = Engine::new(&cfg)?;
    let mut store = Store::new(&engine, ());

    let table_ty = TableType::new(ValType::ExternRef, 10, None);
    let table = Table::new(
        &mut store,
        table_ty,
        Val::ExternRef(Some(ExternRef::new(42_usize))),
    )?;

    assert_eq!(
        *table
            .get(&mut store, 5)
            .unwrap()
            .unwrap_externref()
            .unwrap()
            .data()
            .downcast_ref::<usize>()
            .unwrap(),
        42
    );
    table.set(&mut store, 5, Val::ExternRef(None))?;
    assert!(table
        .get(&mut store, 5)
        .unwrap()
        .unwrap_externref()
        .is_none());

    Ok(())
}

#[test]
fn fill_externref_tables_via_api() -> anyhow::Result<()> {
    let mut cfg = Config::new();
    cfg.wasm_reference_types(true);
    let engine = Engine::new(&cfg)?;
    let mut store = Store::new(&engine, ());

    let table_ty = TableType::new(ValType::ExternRef, 10, None);
    let table = Table::new(&mut store, table_ty, Val::ExternRef(None))?;

    for i in 0..10 {
        assert!(table
            .get(&mut store, i)
            .unwrap()
            .unwrap_externref()
            .is_none());
    }

    table.fill(
        &mut store,
        2,
        Val::ExternRef(Some(ExternRef::new(42_usize))),
        4,
    )?;

    for i in (0..2).chain(7..10) {
        assert!(table
            .get(&mut store, i)
            .unwrap()
            .unwrap_externref()
            .is_none());
    }
    for i in 2..6 {
        assert_eq!(
            *table
                .get(&mut store, i)
                .unwrap()
                .unwrap_externref()
                .unwrap()
                .data()
                .downcast_ref::<usize>()
                .unwrap(),
            42
        );
    }

    Ok(())
}

#[test]
fn grow_externref_tables_via_api() -> anyhow::Result<()> {
    let mut cfg = Config::new();
    cfg.wasm_reference_types(true);
    let engine = Engine::new(&cfg)?;
    let mut store = Store::new(&engine, ());

    let table_ty = TableType::new(ValType::ExternRef, 10, None);
    let table = Table::new(&mut store, table_ty, Val::ExternRef(None))?;

    assert_eq!(table.size(&store), 10);
    table.grow(&mut store, 3, Val::ExternRef(None))?;
    assert_eq!(table.size(&store), 13);

    Ok(())
}

#[test]
fn read_write_memory_via_api() {
    let cfg = Config::new();
    let mut store = Store::new(&Engine::new(&cfg).unwrap(), ());
    let ty = MemoryType::new(1, None);
    let mem = Memory::new(&mut store, ty).unwrap();
    mem.grow(&mut store, 1).unwrap();

    let value = b"hello wasm";
    let size = mem.data_size(&store);
    mem.write(&mut store, size - value.len(), value).unwrap();

    let mut buffer = [0u8; 10];
    mem.read(&store, mem.data_size(&store) - buffer.len(), &mut buffer)
        .unwrap();
    assert_eq!(value, &buffer);

    // Error conditions.

    // Out of bounds write.

    let size = mem.data_size(&store);
    let res = mem.write(&mut store, size - value.len() + 1, value);
    assert!(res.is_err());
    assert_ne!(
        mem.data(&store)[mem.data_size(&store) - value.len() + 1],
        value[0],
        "no data is written",
    );

    // Out of bounds read.

    buffer[0] = 0x42;
    let res = mem.read(
        &store,
        mem.data_size(&store) - buffer.len() + 1,
        &mut buffer,
    );
    assert!(res.is_err());
    assert_eq!(buffer[0], 0x42, "no data is read");

    // Read offset overflow.
    let res = mem.read(&store, usize::MAX, &mut buffer);
    assert!(res.is_err());

    // Write offset overflow.
    let res = mem.write(&mut store, usize::MAX, &mut buffer);
    assert!(res.is_err());
}
