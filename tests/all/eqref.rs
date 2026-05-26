use super::gc_store;
use wasmtime::*;

#[test]
fn eqref_from_i31() -> Result<()> {
    let mut config = Config::new();
    config.wasm_function_references(true);
    config.wasm_gc(true);

    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());

    let i31 = I31::wrapping_u32(31);

    // without EqRef::from_i31
    let any_ref = AnyRef::from_i31(&mut store, i31);
    let eq_ref1 = any_ref.unwrap_eqref(&mut store)?;

    // with EqRef::from_i31
    let eq_ref2 = EqRef::from_i31(&mut store, i31);

    // reference to same i31
    assert_eq!(Rooted::ref_eq(&store, &eq_ref1, &eq_ref2)?, true);

    Ok(())
}

#[test]
fn eqref_from_structref() -> Result<()> {
    let mut store = gc_store()?;
    let struct_ty = StructType::new(
        store.engine(),
        [FieldType::new(Mutability::Const, ValType::I32.into())],
    )?;
    let pre = StructRefPre::new(&mut store, struct_ty);
    let s = StructRef::new(&mut store, &pre, &[Val::I32(42)])?;

    // From<Rooted<StructRef>> for Rooted<EqRef>
    let eq: Rooted<EqRef> = s.into();
    assert!(eq.is_struct(&store)?);

    Ok(())
}

#[test]
fn eqref_owned_from_structref() -> Result<()> {
    let mut store = gc_store()?;
    let struct_ty = StructType::new(
        store.engine(),
        [FieldType::new(Mutability::Const, ValType::I32.into())],
    )?;
    let pre = StructRefPre::new(&mut store, struct_ty);
    let s = StructRef::new(&mut store, &pre, &[Val::I32(99)])?;
    let owned = s.to_owned_rooted(&mut store)?;

    // From<OwnedRooted<StructRef>> for OwnedRooted<EqRef>
    let eq: OwnedRooted<EqRef> = owned.into();
    let rooted = eq.to_rooted(&mut store);
    assert!(rooted.is_struct(&store)?);

    Ok(())
}

#[test]
fn eqref_from_arrayref() -> Result<()> {
    let mut store = gc_store()?;
    let array_ty = ArrayType::new(
        store.engine(),
        FieldType::new(Mutability::Const, ValType::I32.into()),
    );
    let pre = ArrayRefPre::new(&mut store, array_ty);
    let a = ArrayRef::new(&mut store, &pre, &Val::I32(0), 3)?;

    // From<Rooted<ArrayRef>> for Rooted<EqRef>
    let eq: Rooted<EqRef> = a.into();
    assert!(eq.is_array(&store)?);

    Ok(())
}

#[test]
fn eqref_owned_from_arrayref() -> Result<()> {
    let mut store = gc_store()?;
    let array_ty = ArrayType::new(
        store.engine(),
        FieldType::new(Mutability::Const, ValType::I32.into()),
    );
    let pre = ArrayRefPre::new(&mut store, array_ty);
    let a = ArrayRef::new(&mut store, &pre, &Val::I32(7), 2)?;
    let owned = a.to_owned_rooted(&mut store)?;

    // From<OwnedRooted<ArrayRef>> for OwnedRooted<EqRef>
    let eq: OwnedRooted<EqRef> = owned.into();
    let rooted = eq.to_rooted(&mut store);
    assert!(rooted.is_array(&store)?);

    Ok(())
}

#[test]
fn eqref_rooted_to_anyref() -> Result<()> {
    let mut store = gc_store()?;
    let eq = EqRef::from_i31(&mut store, I31::wrapping_u32(5));

    // Rooted<EqRef>::to_anyref
    let any: Rooted<AnyRef> = eq.to_anyref();
    assert!(any.is_i31(&store)?);

    Ok(())
}

#[test]
fn eqref_owned_to_anyref() -> Result<()> {
    let mut store = gc_store()?;
    let eq = EqRef::from_i31(&mut store, I31::wrapping_u32(5));
    let owned = eq.to_owned_rooted(&mut store)?;

    // OwnedRooted<EqRef>::to_anyref
    let any_owned: OwnedRooted<AnyRef> = owned.to_anyref();
    let any = any_owned.to_rooted(&mut store);
    assert!(any.is_i31(&store)?);

    Ok(())
}

#[test]
fn eqref_ty_and_matches_ty_struct() -> Result<()> {
    let mut store = gc_store()?;
    let struct_ty = StructType::new(
        store.engine(),
        [FieldType::new(Mutability::Const, ValType::I32.into())],
    )?;
    let pre = StructRefPre::new(&mut store, struct_ty);
    let s = StructRef::new(&mut store, &pre, &[Val::I32(1)])?;
    let eq: Rooted<EqRef> = s.into();

    let ty = eq.ty(&store)?;
    assert!(matches!(ty, HeapType::ConcreteStruct(_)));
    assert!(eq.matches_ty(&store, &HeapType::Eq)?);
    assert!(!eq.matches_ty(&store, &HeapType::I31)?);

    Ok(())
}

#[test]
fn eqref_ty_array() -> Result<()> {
    let mut store = gc_store()?;
    let array_ty = ArrayType::new(
        store.engine(),
        FieldType::new(Mutability::Const, ValType::I32.into()),
    );
    let pre = ArrayRefPre::new(&mut store, array_ty);
    let a = ArrayRef::new(&mut store, &pre, &Val::I32(0), 1)?;
    let eq: Rooted<EqRef> = a.into();

    let ty = eq.ty(&store)?;
    assert!(matches!(ty, HeapType::ConcreteArray(_)));

    Ok(())
}

#[test]
fn eqref_gc_is_i31() -> Result<()> {
    let mut store = gc_store()?;
    let eq = EqRef::from_i31(&mut store, I31::wrapping_u32(10));
    assert!(eq.is_i31(&store)?);
    let array_ty = ArrayType::new(
        store.engine(),
        FieldType::new(Mutability::Const, ValType::I32.into()),
    );
    let pre = ArrayRefPre::new(&mut store, array_ty);
    let a = ArrayRef::new(&mut store, &pre, &Val::I32(0), 1)?;
    let eq: Rooted<EqRef> = a.into();
    assert!(!eq.is_i31(&store)?);
    Ok(())
}

#[test]
fn eqref_gc_as_i31() -> Result<()> {
    let mut store = gc_store()?;
    let eq = EqRef::from_i31(&mut store, I31::wrapping_u32(42));
    assert_eq!(eq.as_i31(&store)?.unwrap().get_u32(), 42);
    let struct_ty = StructType::new(store.engine(), [])?;
    let pre = StructRefPre::new(&mut store, struct_ty);
    let s = StructRef::new(&mut store, &pre, &[])?;
    let eq: Rooted<EqRef> = s.into();
    assert!(eq.as_i31(&store)?.is_none());
    Ok(())
}

#[test]
fn eqref_unwrap_i31() -> Result<()> {
    let mut store = gc_store()?;
    let eq = EqRef::from_i31(&mut store, I31::wrapping_u32(7));
    let i31 = eq.unwrap_i31(&store)?;
    assert_eq!(i31.get_u32(), 7);
    Ok(())
}

#[test]
fn eqref_gc_is_struct() -> Result<()> {
    let mut store = gc_store()?;
    let struct_ty = StructType::new(store.engine(), [])?;
    let pre = StructRefPre::new(&mut store, struct_ty);
    let s = StructRef::new(&mut store, &pre, &[])?;
    let eq: Rooted<EqRef> = s.into();
    assert!(eq.is_struct(&store)?);
    let eq = EqRef::from_i31(&mut store, I31::wrapping_u32(0));
    assert!(!eq.is_struct(&store)?);
    Ok(())
}

#[test]
fn eqref_gc_as_struct() -> Result<()> {
    let mut store = gc_store()?;
    let struct_ty = StructType::new(store.engine(), [])?;
    let pre = StructRefPre::new(&mut store, struct_ty);
    let s = StructRef::new(&mut store, &pre, &[])?;
    let eq: Rooted<EqRef> = s.into();
    assert!(eq.as_struct(&store)?.is_some());
    let eq = EqRef::from_i31(&mut store, I31::wrapping_u32(0));
    assert!(eq.as_struct(&store)?.is_none());
    Ok(())
}

#[test]
fn eqref_unwrap_struct() -> Result<()> {
    let mut store = gc_store()?;
    let struct_ty = StructType::new(store.engine(), [])?;
    let pre = StructRefPre::new(&mut store, struct_ty);
    let s = StructRef::new(&mut store, &pre, &[])?;
    let eq: Rooted<EqRef> = s.into();
    let _s2 = eq.unwrap_struct(&store)?;
    Ok(())
}

#[test]
fn eqref_gc_is_array() -> Result<()> {
    let mut store = gc_store()?;
    let array_ty = ArrayType::new(
        store.engine(),
        FieldType::new(Mutability::Const, ValType::I32.into()),
    );
    let pre = ArrayRefPre::new(&mut store, array_ty);
    let a = ArrayRef::new(&mut store, &pre, &Val::I32(0), 1)?;
    let eq: Rooted<EqRef> = a.into();
    assert!(eq.is_array(&store)?);
    let eq = EqRef::from_i31(&mut store, I31::wrapping_u32(0));
    assert!(!eq.is_array(&store)?);
    Ok(())
}

#[test]
fn eqref_gc_as_array() -> Result<()> {
    let mut store = gc_store()?;
    let array_ty = ArrayType::new(
        store.engine(),
        FieldType::new(Mutability::Const, ValType::I32.into()),
    );
    let pre = ArrayRefPre::new(&mut store, array_ty);
    let a = ArrayRef::new(&mut store, &pre, &Val::I32(0), 2)?;
    let eq: Rooted<EqRef> = a.into();
    assert!(eq.as_array(&store)?.is_some());
    let eq = EqRef::from_i31(&mut store, I31::wrapping_u32(0));
    assert!(eq.as_array(&store)?.is_none());
    Ok(())
}

#[test]
fn eqref_unwrap_array() -> Result<()> {
    let mut store = gc_store()?;
    let array_ty = ArrayType::new(
        store.engine(),
        FieldType::new(Mutability::Const, ValType::I32.into()),
    );
    let pre = ArrayRefPre::new(&mut store, array_ty);
    let a = ArrayRef::new(&mut store, &pre, &Val::I32(5), 3)?;
    let eq: Rooted<EqRef> = a.into();
    let _a2 = eq.unwrap_array(&store)?;
    Ok(())
}

#[test]
fn eqref_matches_ty_eq() -> Result<()> {
    let mut store = gc_store()?;
    let eq = EqRef::from_i31(&mut store, I31::wrapping_u32(1));
    assert!(eq.matches_ty(&store, &HeapType::Eq)?);
    assert!(!eq.matches_ty(&store, &HeapType::Extern)?);
    Ok(())
}
