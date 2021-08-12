use wasmtime::*;

#[test]
fn get_none() {
    let mut store = Store::<()>::default();
    let ty = TableType::new(ValType::FuncRef, 1, None);
    let table = Table::new(&mut store, ty, Val::FuncRef(None)).unwrap();
    match table.get(&mut store, 0) {
        Some(Val::FuncRef(None)) => {}
        _ => panic!(),
    }
    assert!(table.get(&mut store, 1).is_none());
}

#[test]
fn fill_wrong() {
    let mut store = Store::<()>::default();
    let ty = TableType::new(ValType::FuncRef, 1, None);
    let table = Table::new(&mut store, ty, Val::FuncRef(None)).unwrap();
    assert_eq!(
        table
            .fill(&mut store, 0, Val::ExternRef(None), 1)
            .map_err(|e| e.to_string())
            .unwrap_err(),
        "value does not match table element type"
    );

    let ty = TableType::new(ValType::ExternRef, 1, None);
    let table = Table::new(&mut store, ty, Val::ExternRef(None)).unwrap();
    assert_eq!(
        table
            .fill(&mut store, 0, Val::FuncRef(None), 1)
            .map_err(|e| e.to_string())
            .unwrap_err(),
        "value does not match table element type"
    );
}

#[test]
fn copy_wrong() {
    let mut store = Store::<()>::default();
    let ty = TableType::new(ValType::FuncRef, 1, None);
    let table1 = Table::new(&mut store, ty, Val::FuncRef(None)).unwrap();
    let ty = TableType::new(ValType::ExternRef, 1, None);
    let table2 = Table::new(&mut store, ty, Val::ExternRef(None)).unwrap();
    assert_eq!(
        Table::copy(&mut store, &table1, 0, &table2, 0, 1)
            .map_err(|e| e.to_string())
            .unwrap_err(),
        "tables do not have the same element type"
    );
}
