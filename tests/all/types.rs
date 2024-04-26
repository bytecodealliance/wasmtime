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
