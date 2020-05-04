use wasmtime::*;

#[test]
fn get_none() {
    let store = Store::default();
    let ty = TableType::new(ValType::FuncRef, Limits::new(1, None));
    let table = Table::new(&store, ty, Val::AnyRef(AnyRef::Null)).unwrap();
    match table.get(0) {
        Some(Val::AnyRef(AnyRef::Null)) => {}
        _ => panic!(),
    }
    assert!(table.get(1).is_none());
}
