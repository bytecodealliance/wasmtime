use anyhow::Result;
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
