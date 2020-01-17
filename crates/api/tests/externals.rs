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
