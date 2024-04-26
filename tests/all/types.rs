use wasmtime::*;

#[test]
fn basic_array_types() -> Result<()> {
    let engine = Engine::default();
    for mutability in [Mutability::Const, Mutability::Var] {
        for storage_ty in [
            StorageType::I8,
            StorageType::I16,
            StorageType::ValType(ValType::I32),
            StorageType::ValType(RefType::new(true, FuncType::new(&engine, [], []).into()).into()),
        ] {
            let field_ty = FieldType::new(mutability, storage_ty.clone());
            assert_eq!(field_ty.mutability(), mutability);
            assert!(StorageType::eq(field_ty.element_type(), &storage_ty));

            let array_ty = ArrayType::new(&engine, field_ty.clone());
            assert!(Engine::same(array_ty.engine(), &engine));
            assert!(FieldType::eq(&array_ty.field_type(), &field_ty));
            assert_eq!(array_ty.mutability(), mutability);
            assert!(StorageType::eq(&array_ty.element_type(), &storage_ty));
        }
    }
    Ok(())
}

#[test]
fn empty_struct_type() -> Result<()> {
    let engine = Engine::default();
    let struct_ty = StructType::new(&engine, [])?;
    assert_eq!(struct_ty.fields().len(), 0);
    assert!(struct_ty.field(0).is_none());
    Ok(())
}

#[test]
fn basic_struct_types() -> Result<()> {
    let engine = Engine::default();

    let field_types = || {
        [Mutability::Const, Mutability::Var]
            .into_iter()
            .flat_map(|mutability| {
                [
                    StorageType::I8,
                    StorageType::I16,
                    StorageType::ValType(ValType::I32),
                    StorageType::ValType(
                        RefType::new(true, FuncType::new(&engine, [], []).into()).into(),
                    ),
                ]
                .into_iter()
                .map(move |storage_ty| FieldType::new(mutability, storage_ty))
            })
    };

    let struct_ty = StructType::new(&engine, field_types())?;

    assert_eq!(struct_ty.fields().len(), field_types().count());
    for ((i, expected), actual) in field_types().enumerate().zip(struct_ty.fields()) {
        assert!(FieldType::eq(&expected, &actual));
        assert!(FieldType::eq(&expected, &struct_ty.field(i).unwrap()));
    }
    assert!(struct_ty.field(struct_ty.fields().len()).is_none());

    Ok(())
}

#[test]
fn struct_type_matches() -> Result<()> {
    let engine = Engine::default();

    let field = |heap_ty| {
        FieldType::new(
            Mutability::Var,
            StorageType::ValType(RefType::new(true, heap_ty).into()),
        )
    };

    let super_ty = StructType::new(&engine, [field(HeapType::Func)])?;

    // Depth.
    let sub_ty = StructType::new(&engine, [field(HeapType::NoFunc)])?;
    assert!(sub_ty.matches(&super_ty));

    // Width.
    let sub_ty = StructType::new(&engine, [field(HeapType::Func), field(HeapType::Extern)])?;
    assert!(sub_ty.matches(&super_ty));

    // Depth and width.
    let sub_ty = StructType::new(&engine, [field(HeapType::NoFunc), field(HeapType::Extern)])?;
    assert!(sub_ty.matches(&super_ty));

    // Not depth.
    let not_sub_ty = StructType::new(&engine, [field(HeapType::Extern)])?;
    assert!(!not_sub_ty.matches(&super_ty));

    // Not width.
    let not_sub_ty = StructType::new(&engine, [])?;
    assert!(!not_sub_ty.matches(&super_ty));

    Ok(())
}
