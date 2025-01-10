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

    // mismatched initializer
    let ty = TableType::new(RefType::FUNCREF, 0, Some(1));
    assert!(Table::new(&mut store, ty.clone(), Ref::Extern(None)).is_err());

    // get out of bounds
    let ty = TableType::new(RefType::FUNCREF, 0, Some(1));
    let t = Table::new(&mut store, ty.clone(), Ref::Func(None)).unwrap();
    assert!(t.get(&mut store, 0).is_none());
    assert!(t.get(&mut store, u64::from(u32::MAX)).is_none());
    assert!(t.get(&mut store, u64::MAX).is_none());

    // set out of bounds or wrong type
    let ty = TableType::new(RefType::FUNCREF, 1, Some(1));
    let t = Table::new(&mut store, ty.clone(), Ref::Func(None)).unwrap();
    assert!(t.set(&mut store, 0, Ref::Extern(None)).is_err());
    assert!(t.set(&mut store, 0, Ref::Func(None)).is_ok());
    assert!(t.set(&mut store, 1, Ref::Func(None)).is_err());

    // grow beyond max
    let ty = TableType::new(RefType::FUNCREF, 1, Some(1));
    let t = Table::new(&mut store, ty.clone(), Ref::Func(None)).unwrap();
    assert!(t.grow(&mut store, 0, Ref::Func(None)).is_ok());
    assert!(t.grow(&mut store, 1, Ref::Func(None)).is_err());
    assert_eq!(t.size(&store), 1);

    // grow wrong type
    let ty = TableType::new(RefType::FUNCREF, 1, Some(2));
    let t = Table::new(&mut store, ty.clone(), Ref::Func(None)).unwrap();
    assert!(t.grow(&mut store, 1, Ref::Extern(None)).is_err());
    assert_eq!(t.size(&store), 1);
}

#[test]
#[cfg_attr(miri, ignore)]
fn cross_store() -> anyhow::Result<()> {
    let mut cfg = Config::new();
    cfg.wasm_reference_types(true);
    let engine = Engine::new(&cfg)?;
    let mut store1 = Store::new(&engine, ());
    let mut store2 = Store::new(&engine, ());

    eprintln!("============ Cross-store instantiation ==============");

    let func = Func::wrap(&mut store2, || {});
    let ty = GlobalType::new(ValType::I32, Mutability::Const);
    let global = Global::new(&mut store2, ty, Val::I32(0))?;
    let ty = MemoryType::new(1, None);
    let memory = Memory::new(&mut store2, ty)?;
    let ty = TableType::new(RefType::FUNCREF, 1, None);
    let table = Table::new(&mut store2, ty, Ref::Func(None))?;

    let need_func = Module::new(&engine, r#"(module (import "" "" (func)))"#)?;
    assert!(Instance::new(&mut store1, &need_func, &[func.into()]).is_err());

    let need_global = Module::new(&engine, r#"(module (import "" "" (global i32)))"#)?;
    assert!(Instance::new(&mut store1, &need_global, &[global.into()]).is_err());

    let need_table = Module::new(&engine, r#"(module (import "" "" (table 1 funcref)))"#)?;
    assert!(Instance::new(&mut store1, &need_table, &[table.into()]).is_err());

    let need_memory = Module::new(&engine, r#"(module (import "" "" (memory 1)))"#)?;
    assert!(Instance::new(&mut store1, &need_memory, &[memory.into()]).is_err());

    eprintln!("============ Cross-store globals ==============");

    let store1val = Val::FuncRef(Some(Func::wrap(&mut store1, || {})));
    let store1ref = store1val.ref_().unwrap();
    let store2val = Val::FuncRef(Some(Func::wrap(&mut store2, || {})));
    let store2ref = store2val.ref_().unwrap();

    let ty = GlobalType::new(ValType::FUNCREF, Mutability::Var);
    assert!(Global::new(&mut store2, ty.clone(), store1val).is_err());
    if let Ok(g) = Global::new(&mut store2, ty.clone(), store2val) {
        assert!(g.set(&mut store2, store1val).is_err());
    }

    eprintln!("============ Cross-store tables ==============");

    let ty = TableType::new(RefType::FUNCREF, 1, None);
    assert!(Table::new(&mut store2, ty.clone(), store1ref.clone()).is_err());
    let t1 = Table::new(&mut store2, ty.clone(), store2ref.clone())?;
    assert!(t1.set(&mut store2, 0, store1ref.clone()).is_err());
    assert!(t1.grow(&mut store2, 0, store1ref.clone()).is_err());
    assert!(t1.fill(&mut store2, 0, store1ref.clone(), 1).is_err());

    eprintln!("============ Cross-store funcs ==============");

    let module = Module::new(&engine, r#"(module (func (export "f") (param funcref)))"#)?;
    let s1_inst = Instance::new(&mut store1, &module, &[])?;
    let s2_inst = Instance::new(&mut store2, &module, &[])?;
    let s1_f = s1_inst.get_func(&mut store1, "f").unwrap();
    let s2_f = s2_inst.get_func(&mut store2, "f").unwrap();

    assert!(
        s1_f.call(&mut store1, &[Val::FuncRef(None)], &mut [])
            .is_ok()
    );
    assert!(
        s2_f.call(&mut store2, &[Val::FuncRef(None)], &mut [])
            .is_ok()
    );
    assert!(
        s1_f.call(&mut store1, &[Some(s1_f).into()], &mut [])
            .is_ok()
    );
    assert!(
        s1_f.call(&mut store1, &[Some(s2_f).into()], &mut [])
            .is_err()
    );
    assert!(
        s2_f.call(&mut store2, &[Some(s1_f).into()], &mut [])
            .is_err()
    );
    assert!(
        s2_f.call(&mut store2, &[Some(s2_f).into()], &mut [])
            .is_ok()
    );

    let s1_f_t = s1_f.typed::<Option<Func>, ()>(&store1)?;
    let s2_f_t = s2_f.typed::<Option<Func>, ()>(&store2)?;

    assert!(s1_f_t.call(&mut store1, None).is_ok());
    assert!(s2_f_t.call(&mut store2, None).is_ok());
    assert!(s1_f_t.call(&mut store1, Some(s1_f)).is_ok());
    assert!(s1_f_t.call(&mut store1, Some(s2_f)).is_err());
    assert!(s2_f_t.call(&mut store2, Some(s1_f)).is_err());
    assert!(s2_f_t.call(&mut store2, Some(s2_f)).is_ok());

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
        GlobalType::new(ValType::EXTERNREF, Mutability::Var),
        Val::ExternRef(None),
    )?;
    assert!(global.get(&mut store).unwrap_externref().is_none());

    let hello = ExternRef::new(&mut store, "hello".to_string())?;
    global.set(&mut store, hello.into())?;
    let r = global.get(&mut store).unwrap_externref().cloned().unwrap();
    assert!(
        r.data(&store)?
            .expect("should have host data")
            .is::<String>()
    );
    assert_eq!(
        r.data(&store)?
            .expect("should have host data")
            .downcast_ref::<String>()
            .unwrap(),
        "hello"
    );

    // Initialize with a non-null externref.

    let externref = ExternRef::new(&mut store, 42_i32)?;
    let global = Global::new(
        &mut store,
        GlobalType::new(ValType::EXTERNREF, Mutability::Const),
        externref.into(),
    )?;
    let r = global.get(&mut store).unwrap_externref().cloned().unwrap();
    assert!(r.data(&store)?.expect("should have host data").is::<i32>());
    assert_eq!(
        r.data(&store)?
            .expect("should have host data")
            .downcast_ref::<i32>()
            .copied()
            .unwrap(),
        42
    );

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
        GlobalType::new(ValType::FUNCREF, Mutability::Var),
        Val::FuncRef(None),
    )?;
    assert!(global.get(&mut store).unwrap_funcref().is_none());

    global.set(&mut store, Val::FuncRef(Some(f)))?;
    let f2 = global.get(&mut store).unwrap_funcref().cloned().unwrap();
    assert!(FuncType::eq(&f.ty(&store), &f2.ty(&store)));

    // Initialize with a non-null funcref.

    let global = Global::new(
        &mut store,
        GlobalType::new(ValType::FUNCREF, Mutability::Var),
        Val::FuncRef(Some(f)),
    )?;
    let f2 = global.get(&mut store).unwrap_funcref().cloned().unwrap();
    assert!(FuncType::eq(&f.ty(&store), &f2.ty(&store)));

    Ok(())
}

#[test]
fn create_get_set_funcref_tables_via_api() -> anyhow::Result<()> {
    let mut cfg = Config::new();
    cfg.wasm_reference_types(true);
    let engine = Engine::new(&cfg)?;
    let mut store = Store::new(&engine, ());

    let table_ty = TableType::new(RefType::FUNCREF, 10, None);
    let init = Ref::Func(Some(Func::wrap(&mut store, || {})));
    let table = Table::new(&mut store, table_ty, init)?;

    assert!(table.get(&mut store, 5).unwrap().unwrap_func().is_some());
    table.set(&mut store, 5, Ref::Func(None))?;
    assert!(table.get(&mut store, 5).unwrap().unwrap_func().is_none());

    Ok(())
}

#[test]
fn fill_funcref_tables_via_api() -> anyhow::Result<()> {
    let mut cfg = Config::new();
    cfg.wasm_reference_types(true);
    let engine = Engine::new(&cfg)?;
    let mut store = Store::new(&engine, ());

    let table_ty = TableType::new(RefType::FUNCREF, 10, None);
    let table = Table::new(&mut store, table_ty, Ref::Func(None))?;

    for i in 0..10 {
        assert!(table.get(&mut store, i).unwrap().unwrap_func().is_none());
    }

    let fill = Ref::Func(Some(Func::wrap(&mut store, || {})));
    table.fill(&mut store, 2, fill, 4)?;

    for i in (0..2).chain(7..10) {
        assert!(table.get(&mut store, i).unwrap().unwrap_func().is_none());
    }
    for i in 2..6 {
        assert!(table.get(&mut store, i).unwrap().unwrap_func().is_some());
    }

    Ok(())
}

#[test]
fn grow_funcref_tables_via_api() -> anyhow::Result<()> {
    let mut cfg = Config::new();
    cfg.wasm_reference_types(true);
    let engine = Engine::new(&cfg)?;
    let mut store = Store::new(&engine, ());

    let table_ty = TableType::new(RefType::FUNCREF, 10, None);
    let table = Table::new(&mut store, table_ty, Ref::Func(None))?;

    assert_eq!(table.size(&store), 10);
    table.grow(&mut store, 3, Ref::Func(None))?;
    assert_eq!(table.size(&store), 13);

    Ok(())
}

#[test]
fn create_get_set_externref_tables_via_api() -> anyhow::Result<()> {
    let mut cfg = Config::new();
    cfg.wasm_reference_types(true);
    let engine = Engine::new(&cfg)?;
    let mut store = Store::new(&engine, ());

    let table_ty = TableType::new(RefType::EXTERNREF, 10, None);
    let init = ExternRef::new(&mut store, 42_usize)?;
    let table = Table::new(&mut store, table_ty, init.into())?;

    assert_eq!(
        *table
            .get(&mut store, 5)
            .unwrap()
            .unwrap_extern()
            .unwrap()
            .data(&store)?
            .expect("should have host data")
            .downcast_ref::<usize>()
            .unwrap(),
        42
    );
    table.set(&mut store, 5, Ref::Extern(None))?;
    assert!(table.get(&mut store, 5).unwrap().unwrap_extern().is_none());

    Ok(())
}

#[test]
fn fill_externref_tables_via_api() -> anyhow::Result<()> {
    let mut cfg = Config::new();
    cfg.wasm_reference_types(true);
    let engine = Engine::new(&cfg)?;
    let mut store = Store::new(&engine, ());

    let table_ty = TableType::new(RefType::EXTERNREF, 10, None);
    let table = Table::new(&mut store, table_ty, Ref::Extern(None))?;

    for i in 0..10 {
        assert!(table.get(&mut store, i).unwrap().unwrap_extern().is_none());
    }

    let val = ExternRef::new(&mut store, 42_usize)?;
    table.fill(&mut store, 2, val.into(), 4)?;

    for i in (0..2).chain(7..10) {
        assert!(table.get(&mut store, i).unwrap().unwrap_extern().is_none());
    }
    for i in 2..6 {
        assert_eq!(
            *table
                .get(&mut store, i)
                .unwrap()
                .unwrap_extern()
                .unwrap()
                .data(&store)?
                .expect("should have host data")
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

    let table_ty = TableType::new(RefType::EXTERNREF, 10, None);
    let table = Table::new(&mut store, table_ty, Ref::Extern(None))?;

    assert_eq!(table.size(&store), 10);
    table.grow(&mut store, 3, Ref::Extern(None))?;
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
    let res = mem.write(&mut store, usize::MAX, &buffer);
    assert!(res.is_err());
}

// Returns (a, b, c) pairs of function type and function such that
//
//     a <: b <: c
//
// The functions will panic if actually called.
fn dummy_funcs_and_subtypes(
    store: &mut Store<()>,
) -> (FuncType, Func, FuncType, Func, FuncType, Func) {
    let engine = store.engine().clone();

    let c_ty = FuncType::with_finality_and_supertype(&engine, Finality::NonFinal, None, [], [
        ValType::FUNCREF,
    ])
    .unwrap();
    let c = Func::new(
        &mut *store,
        c_ty.clone(),
        |_caller, _args, _results| unreachable!(),
    );

    let b_ty =
        FuncType::with_finality_and_supertype(&engine, Finality::NonFinal, Some(&c_ty), [], [
            ValType::Ref(RefType::new(
                true,
                HeapType::ConcreteFunc(FuncType::new(&engine, None, None)),
            )),
        ])
        .unwrap();
    let b = Func::new(
        &mut *store,
        b_ty.clone(),
        |_caller, _args, _results| unreachable!(),
    );

    let a_ty =
        FuncType::with_finality_and_supertype(&engine, Finality::NonFinal, Some(&b_ty), [], [
            ValType::NULLFUNCREF,
        ])
        .unwrap();
    let a = Func::new(
        &mut *store,
        a_ty.clone(),
        |_caller, _args, _results| unreachable!(),
    );

    assert!(a_ty.matches(&a_ty));
    assert!(a_ty.matches(&b_ty));
    assert!(a_ty.matches(&c_ty));
    assert!(!b_ty.matches(&a_ty));
    assert!(b_ty.matches(&b_ty));
    assert!(b_ty.matches(&c_ty));
    assert!(!c_ty.matches(&a_ty));
    assert!(!c_ty.matches(&b_ty));
    assert!(c_ty.matches(&c_ty));

    assert!(a.matches_ty(&store, &a_ty));
    assert!(a.matches_ty(&store, &b_ty));
    assert!(a.matches_ty(&store, &c_ty));
    assert!(!b.matches_ty(&store, &a_ty));
    assert!(b.matches_ty(&store, &b_ty));
    assert!(b.matches_ty(&store, &c_ty));
    assert!(!c.matches_ty(&store, &a_ty));
    assert!(!c.matches_ty(&store, &b_ty));
    assert!(c.matches_ty(&store, &c_ty));

    (a_ty, a, b_ty, b, c_ty, c)
}

#[test]
#[cfg_attr(miri, ignore)]
fn new_global_func_subtyping() {
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());

    let (a_ty, a, b_ty, b, c_ty, c) = dummy_funcs_and_subtypes(&mut store);

    for (global_ty, a_expected, b_expected, c_expected) in [
        // a <: a, b </: a, c </: a
        (a_ty, true, false, false),
        // a <: b, b <: b, c </: a
        (b_ty, true, true, false),
        // a <: c, b <: c, c <: c
        (c_ty, true, true, true),
    ] {
        for (val, expected) in [(a, a_expected), (b, b_expected), (c, c_expected)] {
            for mutability in [Mutability::Var, Mutability::Const] {
                match Global::new(
                    &mut store,
                    GlobalType::new(
                        RefType::new(true, global_ty.clone().into()).into(),
                        mutability,
                    ),
                    val.into(),
                ) {
                    Ok(_) if expected => {}
                    Ok(_) => panic!("should have got type mismatch, but didn't"),
                    Err(e) if !expected => assert!(e.to_string().contains("type mismatch")),
                    Err(e) => panic!("should have created global, but got error: {e:?}"),
                }
            }
        }
    }
}

#[test]
#[cfg_attr(miri, ignore)]
fn global_set_func_subtyping() {
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());

    let (a_ty, a, b_ty, b, c_ty, c) = dummy_funcs_and_subtypes(&mut store);

    for (global_ty, a_expected, b_expected, c_expected) in [
        // a <: a, b </: a, c </: a
        (a_ty, true, false, false),
        // a <: b, b <: b, c </: a
        (b_ty, true, true, false),
        // a <: c, b <: c, c <: c
        (c_ty, true, true, true),
    ] {
        let global = Global::new(
            &mut store,
            GlobalType::new(
                RefType::new(true, global_ty.clone().into()).into(),
                Mutability::Var,
            ),
            Val::null_func_ref(),
        )
        .unwrap();

        for (val, expected) in [(a, a_expected), (b, b_expected), (c, c_expected)] {
            match global.set(&mut store, val.into()) {
                Ok(_) if expected => {}
                Ok(_) => panic!("should have got type mismatch, but didn't"),
                Err(e) if !expected => assert!(e.to_string().contains("type mismatch")),
                Err(e) => panic!("should have set global, but got error: {e:?}"),
            }
        }
    }
}

#[test]
#[cfg_attr(miri, ignore)]
fn new_table_func_subtyping() {
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());

    let (a_ty, a, b_ty, b, c_ty, c) = dummy_funcs_and_subtypes(&mut store);

    for (table_ty, a_expected, b_expected, c_expected) in [
        // a <: a, b </: a, c </: a
        (a_ty, true, false, false),
        // a <: b, b <: b, c </: a
        (b_ty, true, true, false),
        // a <: c, b <: c, c <: c
        (c_ty, true, true, true),
    ] {
        for (val, expected) in [(a, a_expected), (b, b_expected), (c, c_expected)] {
            match Table::new(
                &mut store,
                TableType::new(RefType::new(true, table_ty.clone().into()), 0, None),
                val.into(),
            ) {
                Ok(_) if expected => {}
                Ok(_) => panic!("should have got type mismatch, but didn't"),
                Err(e) if !expected => assert!(e.to_string().contains("type mismatch")),
                Err(e) => panic!("should have created table, but got error: {e:?}"),
            }
        }
    }
}

#[test]
#[cfg_attr(miri, ignore)]
fn table_set_func_subtyping() {
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());

    let (a_ty, a, b_ty, b, c_ty, c) = dummy_funcs_and_subtypes(&mut store);

    for (table_ty, a_expected, b_expected, c_expected) in [
        // a <: a, b </: a, c </: a
        (a_ty, true, false, false),
        // a <: b, b <: b, c </: a
        (b_ty, true, true, false),
        // a <: c, b <: c, c <: c
        (c_ty, true, true, true),
    ] {
        let table = Table::new(
            &mut store,
            TableType::new(RefType::new(true, table_ty.clone().into()), 3, None),
            Ref::Func(None),
        )
        .unwrap();

        let mut i = 0;
        for (val, expected) in [(a, a_expected), (b, b_expected), (c, c_expected)] {
            assert!(table.get(&mut store, i).expect("in bounds").is_null());

            match table.set(&mut store, i, val.into()) {
                Ok(_) if expected => {}
                Ok(_) => panic!("should have got type mismatch, but didn't"),
                Err(e) if !expected => assert!(e.to_string().contains("type mismatch")),
                Err(e) => panic!("should have set table element, but got error: {e:?}"),
            }

            if expected {
                assert!(table.get(&mut store, i).expect("in bounds").is_non_null());
            }

            i += 1;
        }
    }
}

#[test]
#[cfg_attr(miri, ignore)]
fn table_grow_func_subtyping() {
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());

    let (a_ty, a, b_ty, b, c_ty, c) = dummy_funcs_and_subtypes(&mut store);

    for (table_ty, a_expected, b_expected, c_expected) in [
        // a <: a, b </: a, c </: a
        (a_ty, true, false, false),
        // a <: b, b <: b, c </: a
        (b_ty, true, true, false),
        // a <: c, b <: c, c <: c
        (c_ty, true, true, true),
    ] {
        let table = Table::new(
            &mut store,
            TableType::new(RefType::new(true, table_ty.clone().into()), 3, None),
            Ref::Func(None),
        )
        .unwrap();

        for (val, expected) in [(a, a_expected), (b, b_expected), (c, c_expected)] {
            let orig_size = table.size(&store);

            match table.grow(&mut store, 10, val.into()) {
                Ok(_) if expected => {}
                Ok(_) => panic!("should have got type mismatch, but didn't"),
                Err(e) if !expected => assert!(e.to_string().contains("type mismatch")),
                Err(e) => panic!("should have done table grow, but got error: {e:?}"),
            }

            if expected {
                let new_size = table.size(&store);
                assert_eq!(new_size, orig_size + 10);
                for i in orig_size..new_size {
                    assert!(table.get(&mut store, i).expect("in bounds").is_non_null());
                }
            }
        }
    }
}

#[test]
#[cfg_attr(miri, ignore)]
fn table_fill_func_subtyping() {
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());

    let (a_ty, a, b_ty, b, c_ty, c) = dummy_funcs_and_subtypes(&mut store);

    for (table_ty, a_expected, b_expected, c_expected) in [
        // a <: a, b </: a, c </: a
        (a_ty, true, false, false),
        // a <: b, b <: b, c </: a
        (b_ty, true, true, false),
        // a <: c, b <: c, c <: c
        (c_ty, true, true, true),
    ] {
        for (val, expected) in [(a, a_expected), (b, b_expected), (c, c_expected)] {
            let table = Table::new(
                &mut store,
                TableType::new(RefType::new(true, table_ty.clone().into()), 10, None),
                Ref::Func(None),
            )
            .unwrap();

            match table.fill(&mut store, 3, val.into(), 4) {
                Ok(_) if expected => {}
                Ok(_) => panic!("should have got type mismatch, but didn't"),
                Err(e) if !expected => assert!(e.to_string().contains("type mismatch")),
                Err(e) => panic!("should have done table fill, but got error: {e:?}"),
            }

            if expected {
                for i in 3..7 {
                    assert!(table.get(&mut store, i).expect("in bounds").is_non_null());
                }
            }
        }
    }
}

#[test]
#[cfg_attr(miri, ignore)]
fn table_copy_func_subtyping() {
    let _ = env_logger::try_init();

    let engine = Engine::default();
    let mut store = Store::new(&engine, ());

    let (a_ty, a, b_ty, b, c_ty, c) = dummy_funcs_and_subtypes(&mut store);

    for (dst_ty, a_expected, b_expected, c_expected) in [
        // a <: a, b </: a, c </: a
        (a_ty.clone(), true, false, false),
        // a <: b, b <: b, c </: a
        (b_ty.clone(), true, true, false),
        // a <: c, b <: c, c <: c
        (c_ty.clone(), true, true, true),
    ] {
        let dest_table = Table::new(
            &mut store,
            TableType::new(RefType::new(true, dst_ty.clone().into()), 10, None),
            Ref::Func(None),
        )
        .unwrap();

        for (val, src_ty, expected) in [
            (a, a_ty.clone(), a_expected),
            (b, b_ty.clone(), b_expected),
            (c, c_ty.clone(), c_expected),
        ] {
            dest_table.fill(&mut store, 0, Ref::Func(None), 10).unwrap();

            let src_table = Table::new(
                &mut store,
                TableType::new(RefType::new(true, src_ty.into()), 10, None),
                val.into(),
            )
            .unwrap();

            match Table::copy(&mut store, &dest_table, 2, &src_table, 3, 5) {
                Ok(_) if expected => {}
                Ok(_) => panic!("should have got type mismatch, but didn't"),
                Err(e) if !expected => assert!(e.to_string().contains("type mismatch")),
                Err(e) => panic!("should have done table copy, but got error: {e:?}"),
            }

            if expected {
                for i in 2..7 {
                    assert!(
                        dest_table
                            .get(&mut store, i)
                            .expect("in bounds")
                            .is_non_null()
                    );
                }
            }
        }
    }
}
