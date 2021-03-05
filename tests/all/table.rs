use wasmtime::*;

#[test]
fn get_none() {
    let store = Store::default();
    let ty = TableType::new(ValType::FuncRef, Limits::new(1, None));
    let table = Table::new(&store, ty, Val::FuncRef(None)).unwrap();
    match table.get(0) {
        Some(Val::FuncRef(None)) => {}
        _ => panic!(),
    }
    assert!(table.get(1).is_none());
}

#[test]
fn fill_wrong() {
    let store = Store::default();
    let ty = TableType::new(ValType::FuncRef, Limits::new(1, None));
    let table = Table::new(&store, ty, Val::FuncRef(None)).unwrap();
    assert_eq!(
        table
            .fill(0, Val::ExternRef(None), 1)
            .map_err(|e| e.to_string())
            .unwrap_err(),
        "mismatched element fill type"
    );

    let ty = TableType::new(ValType::ExternRef, Limits::new(1, None));
    let table = Table::new(&store, ty, Val::ExternRef(None)).unwrap();
    assert_eq!(
        table
            .fill(0, Val::FuncRef(None), 1)
            .map_err(|e| e.to_string())
            .unwrap_err(),
        "mismatched element fill type"
    );
}

#[test]
fn copy_wrong() {
    let store = Store::default();
    let ty = TableType::new(ValType::FuncRef, Limits::new(1, None));
    let table1 = Table::new(&store, ty, Val::FuncRef(None)).unwrap();
    let ty = TableType::new(ValType::ExternRef, Limits::new(1, None));
    let table2 = Table::new(&store, ty, Val::ExternRef(None)).unwrap();
    assert_eq!(
        Table::copy(&table1, 0, &table2, 0, 1)
            .map_err(|e| e.to_string())
            .unwrap_err(),
        "tables do not have the same element type"
    );
}
