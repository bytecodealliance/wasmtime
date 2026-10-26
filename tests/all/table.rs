use wasmtime::*;

#[test]
fn get_none() {
    let mut store = Store::<()>::default();
    let ty = TableType::new(RefType::FUNCREF, 1, None).unwrap();
    let table = Table::new(&mut store, ty, Ref::Func(None)).unwrap();
    match table.get(&mut store, 0) {
        Some(Ref::Func(None)) => {}
        _ => panic!(),
    }
    assert!(table.get(&mut store, 1).is_none());
}

#[test]
fn min_greater_than_max() -> Result<()> {
    assert!(TableType::new(RefType::FUNCREF, 10, Some(5)).is_err());
    assert!(TableType::new64(RefType::FUNCREF, 10, Some(5)).is_err());

    let mut store = Store::<()>::default();
    let ty = TableType::new(RefType::FUNCREF, 5, Some(5))?;
    Table::new(&mut store, ty, Ref::Func(None))?;
    Ok(())
}

#[test]
fn fill_wrong() {
    let mut store = Store::<()>::default();
    let ty = TableType::new(RefType::FUNCREF, 1, None).unwrap();
    let table = Table::new(&mut store, ty, Ref::Func(None)).unwrap();
    assert_eq!(
        table
            .fill(&mut store, 0, Ref::Extern(None), 1)
            .map_err(|e| e.to_string())
            .unwrap_err(),
        "type mismatch: value does not match table element type"
    );

    let ty = TableType::new(RefType::EXTERNREF, 1, None).unwrap();
    let table = Table::new(&mut store, ty, Ref::Extern(None)).unwrap();
    assert_eq!(
        table
            .fill(&mut store, 0, Ref::Func(None), 1)
            .map_err(|e| e.to_string())
            .unwrap_err(),
        "type mismatch: value does not match table element type"
    );
}

#[test]
fn copy_wrong() {
    let mut store = Store::<()>::default();
    let ty = TableType::new(RefType::FUNCREF, 1, None).unwrap();
    let table1 = Table::new(&mut store, ty, Ref::Func(None)).unwrap();
    let ty = TableType::new(RefType::EXTERNREF, 1, None).unwrap();
    let table2 = Table::new(&mut store, ty, Ref::Extern(None)).unwrap();
    assert_eq!(
        Table::copy(&mut store, &table1, 0, &table2, 0, 1)
            .map_err(|e| e.to_string())
            .unwrap_err(),
        "type mismatch: source table's element type does not match destination table's element type"
    );
}

#[test]
#[cfg_attr(miri, ignore)]
fn null_elem_segment_works_with_imported_table() -> Result<()> {
    let mut store = Store::<()>::default();
    let ty = TableType::new(RefType::FUNCREF, 1, None)?;
    let table = Table::new(&mut store, ty, Ref::Func(None))?;
    let module = Module::new(
        store.engine(),
        r#"
(module
  (import "" "" (table (;0;) 1 funcref))
  (func
    i32.const 0
    table.get 0
    drop
  )
  (start 0)
  (elem (;0;) (i32.const 0) funcref (ref.null func))
)
"#,
    )?;
    Instance::new(&mut store, &module, &[table.into()])?;
    Ok(())
}

#[test]
fn i31ref_table_new() -> Result<()> {
    let mut config = Config::new();
    config.wasm_function_references(true);
    config.wasm_gc(true);

    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());

    for (elem_ty, inits) in [
        (
            RefType::I31REF,
            vec![
                Ref::Any(None),
                AnyRef::from_i31(&mut store, I31::default()).into(),
            ],
        ),
        (
            RefType::new(false, HeapType::I31),
            vec![AnyRef::from_i31(&mut store, I31::default()).into()],
        ),
    ] {
        let table_ty = TableType::new(elem_ty, 10, None)?;
        for init in inits {
            Table::new(&mut store, table_ty.clone(), init)?;
        }
    }

    Ok(())
}

#[test]
fn i31ref_table_get() -> Result<()> {
    let mut config = Config::new();
    config.wasm_function_references(true);
    config.wasm_gc(true);

    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());

    for (elem_ty, inits) in [
        (
            RefType::I31REF,
            vec![
                Ref::Any(None),
                AnyRef::from_i31(&mut store, I31::default()).into(),
            ],
        ),
        (
            RefType::new(false, HeapType::I31),
            vec![AnyRef::from_i31(&mut store, I31::default()).into()],
        ),
    ] {
        let table_ty = TableType::new(elem_ty, 10, None)?;
        for init in inits {
            let table = Table::new(&mut store, table_ty.clone(), init.clone())?;
            for i in 0..10 {
                let val = table.get(&mut store, i).unwrap();
                assert_eq!(init.is_null(), val.is_null());
                assert_eq!(
                    init.as_any()
                        .expect("is anyref")
                        .map(|a| a.as_i31(&store).expect("is in scope")),
                    val.as_any()
                        .expect("is anyref")
                        .map(|a| a.as_i31(&store).expect("is in scope"))
                )
            }
        }
    }

    Ok(())
}

#[test]
fn i31ref_table_set() -> Result<()> {
    let mut config = Config::new();
    config.wasm_function_references(true);
    config.wasm_gc(true);

    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());

    for (elem_ty, inits, vals) in [
        (
            RefType::I31REF,
            vec![
                Ref::Any(None),
                AnyRef::from_i31(&mut store, I31::default()).into(),
            ],
            vec![
                Ref::Any(None),
                AnyRef::from_i31(&mut store, I31::wrapping_u32(42)).into(),
            ],
        ),
        (
            RefType::new(false, HeapType::I31),
            vec![AnyRef::from_i31(&mut store, I31::default()).into()],
            vec![AnyRef::from_i31(&mut store, I31::wrapping_u32(42)).into()],
        ),
    ] {
        let table_ty = TableType::new(elem_ty, 10, None)?;
        for init in inits {
            for expected in vals.clone() {
                let table = Table::new(&mut store, table_ty.clone(), init.clone())?;
                for i in 0..10 {
                    table.set(&mut store, i, expected.clone())?;
                    let actual = table.get(&mut store, i).unwrap();
                    assert_eq!(expected.is_null(), actual.is_null());
                    assert_eq!(
                        expected
                            .as_any()
                            .expect("is anyref")
                            .map(|a| a.as_i31(&store).expect("is in scope")),
                        actual
                            .as_any()
                            .expect("is anyref")
                            .map(|a| a.as_i31(&store).expect("is in scope"))
                    )
                }
            }
        }
    }

    Ok(())
}

#[test]
fn i31ref_table_grow() -> Result<()> {
    let mut config = Config::new();
    config.wasm_function_references(true);
    config.wasm_gc(true);

    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());

    for (elem_ty, init) in [
        (RefType::I31REF, Ref::Any(None)),
        (
            RefType::new(false, HeapType::I31),
            AnyRef::from_i31(&mut store, I31::default()).into(),
        ),
    ] {
        let table_ty = TableType::new(elem_ty, 10, None)?;
        let table = Table::new(&mut store, table_ty, init)?;
        assert_eq!(table.size(&store), 10);
        for i in 10..20 {
            assert!(table.get(&mut store, i).is_none());
        }
        let expected = I31::wrapping_u32(42);
        let grow_val = AnyRef::from_i31(&mut store, expected);
        table.grow(&mut store, 10, grow_val.into())?;
        for i in 10..20 {
            let actual = table.get(&mut store, i).unwrap();
            assert_eq!(
                actual
                    .as_any()
                    .expect("is anyref")
                    .expect("is non null")
                    .as_i31(&store)
                    .expect("is in scope")
                    .expect("is i31"),
                expected,
            );
        }
        assert!(table.get(&mut store, 20).is_none());
    }

    Ok(())
}

#[test]
fn i31ref_table_fill() -> Result<()> {
    let mut config = Config::new();
    config.wasm_function_references(true);
    config.wasm_gc(true);

    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());

    let table_ty = TableType::new(RefType::I31REF, 10, None)?;
    let table = Table::new(&mut store, table_ty, Ref::Any(None))?;

    let expected = I31::wrapping_u32(42);
    let fill_val = AnyRef::from_i31(&mut store, expected);
    let dst = 3;
    let len = 4;
    table.fill(&mut store, dst, fill_val.into(), len)?;

    for i in 0..dst {
        let actual = table.get(&mut store, i).unwrap();
        assert!(actual.as_any().expect("is anyref").is_none());
    }
    for i in dst..dst + len {
        let actual = table.get(&mut store, i).unwrap();
        assert_eq!(
            actual
                .as_any()
                .expect("is anyref")
                .expect("is non null")
                .as_i31(&store)
                .expect("is in scope")
                .expect("is i31"),
            expected,
        );
    }
    for i in dst + len..10 {
        let actual = table.get(&mut store, i).unwrap();
        assert!(actual.as_any().expect("is anyref").is_none());
    }

    Ok(())
}

#[test]
fn i31ref_table_copy() -> Result<()> {
    let mut config = Config::new();
    config.wasm_function_references(true);
    config.wasm_gc(true);

    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());

    let table_ty = TableType::new(RefType::I31REF, 10, None)?;
    let dst_table = Table::new(&mut store, table_ty.clone(), Ref::Any(None))?;

    let expected = I31::wrapping_u32(42);
    let init_val = AnyRef::from_i31(&mut store, expected);
    let src_table = Table::new(&mut store, table_ty, init_val.into())?;

    let dst_index = 1;
    let src_index = 2;
    let len = 3;
    Table::copy(
        &mut store, &dst_table, dst_index, &src_table, src_index, len,
    )?;

    for i in 0..dst_index {
        let actual = dst_table.get(&mut store, i).unwrap();
        assert!(actual.as_any().expect("is anyref").is_none());
    }
    for i in dst_index..dst_index + len {
        let actual = dst_table.get(&mut store, i).unwrap();
        assert_eq!(
            actual
                .as_any()
                .expect("is anyref")
                .expect("is non null")
                .as_i31(&store)
                .expect("is in scope")
                .expect("is i31"),
            expected,
        );
    }
    for i in dst_index + len..10 {
        let actual = dst_table.get(&mut store, i).unwrap();
        assert!(actual.as_any().expect("is anyref").is_none());
    }

    Ok(())
}

#[test]
fn host_table_keep_type_registration() -> Result<()> {
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());

    let ty = FuncType::new(&engine, [], []);

    let t = Table::new(
        &mut store,
        TableType::new(RefType::new(true, HeapType::ConcreteFunc(ty)), 1, None)?,
        Ref::Func(None),
    )?;

    {
        let _ty2 = FuncType::new(&engine, [ValType::I32], [ValType::I32]);
        let ty = t.ty(&store);
        let fty = ty.element().heap_type().unwrap_concrete_func();
        assert!(fty.params().len() == 0);
        assert!(fty.results().len() == 0);
    }

    let ty = t.ty(&store);
    let fty = ty.element().heap_type().unwrap_concrete_func();
    assert!(fty.params().len() == 0);
    assert!(fty.results().len() == 0);

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn gc_store_with_table_initializers() -> Result<()> {
    let mut config = Config::new();
    config.wasm_gc(true);
    config.wasm_function_references(true);
    let engine = Engine::new(&config)?;

    let test = |wat: &str| -> Result<()> {
        let module = Module::new(&engine, wat)?;
        Instance::new(&mut Store::new(&engine, ()), &module, &[])?;
        Ok(())
    };

    test("(module (table 1 i31ref))")?;
    test("(module (table 1 i31ref (ref.i31 (i32.const 1))))")?;
    test("(module (table 1 i31ref (ref.null i31)))")?;

    Ok(())
}

#[test]
#[cfg(all(feature = "cranelift", not(miri)))]
fn imported_function_segments_preserve_order_and_source_lifetimes() -> Result<()> {
    let mut initialization_fuel = None;
    for lazy in [false, true] {
        let mut config = Config::new();
        config.table_lazy_init(lazy).consume_fuel(true);
        let engine = Engine::new(&config)?;
        let mut store = Store::new(&engine, ());
        store.set_fuel(100_000)?;
        let table = Table::new(
            &mut store,
            TableType::new(RefType::FUNCREF, 6, None)?,
            Ref::Func(None),
        )?;
        let mut instantiate = |offset: u32, value: i32| -> Result<()> {
            let module = Module::new(
                &engine,
                format!(
                    r#"(module
                (import "" "" (table 6 funcref))
                (global $value i32 (i32.const {value}))
                (func $first (result i32) global.get $value)
                (func $second (result i32) global.get $value i32.const 1 i32.add)
                (elem (i32.const {offset}) func $first $first $second)
                (elem (i32.const {}) func $second))"#,
                    offset + 1
                ),
            )?;
            // SAFETY: these bytes were just serialized by this Engine. This also
            // exercises retained active-segment metadata after deserialization.
            let module = unsafe { Module::deserialize(&engine, module.serialize()?)? };
            Instance::new(&mut store, &module, &[table.into()])?;
            Ok(())
        };
        instantiate(0, 10)?;
        instantiate(2, 20)?;
        let remaining_fuel = store.get_fuel()?;
        if let Some(expected) = initialization_fuel {
            assert_eq!(remaining_fuel, expected);
        }
        initialization_fuel = Some(remaining_fuel);
        // The caller holds no Instance handle for either contributing module.
        table.set(&mut store, 1, Ref::Func(None))?;
        Table::copy(&mut store, &table, 2, &table, 0, 3)?;
        for (index, expected) in [Some(10), None, Some(10), None, Some(20), None]
            .into_iter()
            .enumerate()
        {
            match (table.get(&mut store, index as u64), expected) {
                (Some(Ref::Func(Some(f))), Some(value)) => {
                    assert_eq!(f.typed::<(), i32>(&store)?.call(&mut store, ())?, value)
                }
                (Some(Ref::Func(None)), None) => {}
                other => panic!("unexpected table element {index}: {other:?}"),
            }
        }
        let failed = Module::new(
            &engine,
            r#"(module
            (import "" "" (table 6 funcref))
            (func $f (result i32) i32.const 99)
            (elem (i32.const 5) func $f)
            (func $start unreachable) (start $start))"#,
        )?;
        assert_eq!(
            Instance::new(&mut store, &failed, &[table.into()])
                .unwrap_err()
                .downcast_ref::<Trap>(),
            Some(&Trap::UnreachableCodeReached)
        );
        let Some(Ref::Func(Some(f))) = table.get(&mut store, 5) else {
            panic!("missing failed-start contribution")
        };
        assert_eq!(f.typed::<(), i32>(&store)?.call(&mut store, ())?, 99);
        let out_of_bounds = Module::new(
            &engine,
            r#"(module
            (import "" "" (table 6 funcref))
            (func $f (result i32) i32.const 101)
            (elem (i32.const 0) func $f)
            (elem (i32.const 6) func $f))"#,
        )?;
        assert_eq!(
            Instance::new(&mut store, &out_of_bounds, &[table.into()])
                .unwrap_err()
                .downcast_ref::<Trap>(),
            Some(&Trap::TableOutOfBounds)
        );
        let Some(Ref::Func(Some(f))) = table.get(&mut store, 0) else {
            panic!("missing earlier segment")
        };
        assert_eq!(f.typed::<(), i32>(&store)?.call(&mut store, ())?, 101);
    }
    Ok(())
}

#[test]
#[cfg(all(feature = "cranelift", not(miri)))]
fn imported_function_segments_bound_history_and_preserve_explicit_writes() -> Result<()> {
    let engine = Engine::default();
    let module = Module::new(
        &engine,
        r#"(module
        (import "" "table" (table 2 funcref))
        (import "" "value" (global $value i32))
        (func $f (result i32) global.get $value)
        (elem (i32.const 0) func $f))"#,
    )?;
    let mut store = Store::new(&engine, ());
    let table = Table::new(
        &mut store,
        TableType::new(RefType::FUNCREF, 2, None)?,
        Ref::Func(None),
    )?;
    // Cross the bounded deferred-history limit while old sources remain alive.
    for value in 0..132 {
        let global = Global::new(
            &mut store,
            GlobalType::new(ValType::I32, Mutability::Const),
            Val::I32(value),
        )?;
        Instance::new(&mut store, &module, &[table.into(), global.into()])?;
    }
    let Some(Ref::Func(Some(f))) = table.get(&mut store, 0) else {
        panic!("missing function")
    };
    assert_eq!(f.typed::<(), i32>(&store)?.call(&mut store, ())?, 131);
    table.set(&mut store, 0, Ref::Func(None))?;
    assert!(matches!(table.get(&mut store, 0), Some(Ref::Func(None))));
    Ok(())
}

#[test]
#[cfg(all(feature = "cranelift", feature = "pooling-allocator", not(miri)))]
fn imported_function_segments_do_not_survive_pooled_table_reuse() -> Result<()> {
    let mut pooling = PoolingAllocationConfig::default();
    pooling
        .total_core_instances(2)
        .total_tables(2)
        .max_tables_per_module(1)
        .table_elements(6);
    let mut config = Config::new();
    config.allocation_strategy(InstanceAllocationStrategy::Pooling(pooling));
    let engine = Engine::new(&config)?;
    let owner = Module::new(&engine, "(module (table (export \"table\") 6 funcref))")?;
    let source = Module::new(
        &engine,
        r#"(module
        (import "" "" (table 6 funcref))
        (func $f (result i32) i32.const 42)
        (elem (i32.const 0) func $f))"#,
    )?;
    for _ in 0..3 {
        let mut store = Store::new(&engine, ());
        let owner = Instance::new(&mut store, &owner, &[])?;
        let table = owner.get_table(&mut store, "table").unwrap();
        assert!(matches!(table.get(&mut store, 0), Some(Ref::Func(None))));
        Instance::new(&mut store, &source, &[table.into()])?;
        // Drop the Store without materializing the deferred function.
    }
    Ok(())
}
