use super::gc_store;
use wasmtime::*;

#[test]
fn tag_objects() -> Result<()> {
    let mut store = gc_store()?;
    let engine = store.engine();

    let func_ty = FuncType::new(&engine, [ValType::I32, ValType::I64], []);
    let tag_ty = TagType::new(func_ty);

    let tag = Tag::new(&mut store, &tag_ty).unwrap();

    assert!(tag.ty(&store).ty().matches(tag_ty.ty()));

    let tag2 = Tag::new(&mut store, &tag_ty).unwrap();

    assert!(!Tag::eq(&tag, &tag2, &store));

    Ok(())
}

#[test]
fn exn_types() -> Result<()> {
    let mut store = gc_store()?;
    let engine = store.engine();

    let func_ty = FuncType::new(&engine, [ValType::I32, ValType::I64], []);
    let tag_ty = TagType::new(func_ty);

    let tag = Tag::new(&mut store, &tag_ty).unwrap();

    assert!(tag.ty(&store).ty().matches(tag_ty.ty()));

    let tag2 = Tag::new(&mut store, &tag_ty).unwrap();

    assert!(!Tag::eq(&tag, &tag2, &store));

    let exntype = ExnType::from_tag_type(&tag_ty).unwrap();
    let exntype2 = ExnType::new(store.engine(), [ValType::I32, ValType::I64]).unwrap();

    assert!(exntype.matches(&exntype2));
    assert!(exntype.tag_type().ty().matches(&tag_ty.ty()));

    Ok(())
}

#[test]
fn exn_objects() -> Result<()> {
    let mut store = gc_store()?;
    let exntype = ExnType::new(store.engine(), [ValType::I32, ValType::I64]).unwrap();

    // Create a tag instance to associate with our exception objects.
    let tag = Tag::new(&mut store, &exntype.tag_type()).unwrap();

    // Create an allocator for the exn type.
    let allocator = ExnRefPre::new(&mut store, exntype);

    {
        let mut scope = RootScope::new(&mut store);

        for i in 0..10 {
            ExnRef::new(
                &mut scope,
                &allocator,
                &tag,
                &[Val::I32(i), Val::I64(i64::MAX)],
            )?;
        }

        let obj = ExnRef::new(
            &mut scope,
            &allocator,
            &tag,
            &[Val::I32(42), Val::I64(i64::MIN)],
        )?;

        assert_eq!(obj.fields(&mut scope)?.len(), 2);
        assert_eq!(obj.field(&mut scope, 0)?.unwrap_i32(), 42);
        assert_eq!(obj.field(&mut scope, 1)?.unwrap_i64(), i64::MIN);
        assert!(Tag::eq(&obj.tag(&mut scope)?, &tag, &scope));
    }

    Ok(())
}
