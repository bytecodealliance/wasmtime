use wasmtime::*;

#[test]
fn get_none() {
    let mut store = Store::<()>::default();
    let ty = TableType::new(RefType::FUNCREF, 1, None);
    let table = Table::new(&mut store, ty, Ref::Func(None)).unwrap();
    match table.get(&mut store, 0) {
        Some(Ref::Func(None)) => {}
        _ => panic!(),
    }
    assert!(table.get(&mut store, 1).is_none());
}

#[test]
fn fill_wrong() {
    let mut store = Store::<()>::default();
    let ty = TableType::new(RefType::FUNCREF, 1, None);
    let table = Table::new(&mut store, ty, Ref::Func(None)).unwrap();
    assert_eq!(
        table
            .fill(&mut store, 0, Ref::Extern(None), 1)
            .map_err(|e| e.to_string())
            .unwrap_err(),
        "type mismatch: value does not match table element type"
    );

    let ty = TableType::new(RefType::EXTERNREF, 1, None);
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
    let ty = TableType::new(RefType::FUNCREF, 1, None);
    let table1 = Table::new(&mut store, ty, Ref::Func(None)).unwrap();
    let ty = TableType::new(RefType::EXTERNREF, 1, None);
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
    let ty = TableType::new(RefType::FUNCREF, 1, None);
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
        let table_ty = TableType::new(elem_ty, 10, None);
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
        let table_ty = TableType::new(elem_ty, 10, None);
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
        let table_ty = TableType::new(elem_ty, 10, None);
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
        let table_ty = TableType::new(elem_ty, 10, None);
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

    let table_ty = TableType::new(RefType::I31REF, 10, None);
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

    let table_ty = TableType::new(RefType::I31REF, 10, None);
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
