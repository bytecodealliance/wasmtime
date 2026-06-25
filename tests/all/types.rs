use wasmtime::*;

fn field(heap_ty: HeapType) -> FieldType {
    FieldType::new(
        Mutability::Var,
        StorageType::ValType(RefType::new(true, heap_ty).into()),
    )
}

fn imm_field(heap_ty: HeapType) -> FieldType {
    FieldType::new(
        Mutability::Const,
        StorageType::ValType(RefType::new(true, heap_ty).into()),
    )
}

fn valty(heap_ty: HeapType) -> ValType {
    ValType::Ref(RefType::new(true, heap_ty))
}

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

    let super_ty = StructType::with_finality_and_supertype(
        &engine,
        Finality::NonFinal,
        None,
        [imm_field(HeapType::Func)],
    )?;

    // Depth.
    let sub_ty = StructType::with_finality_and_supertype(
        &engine,
        Finality::Final,
        Some(&super_ty),
        [imm_field(HeapType::NoFunc)],
    )?;
    assert!(sub_ty.matches(&super_ty));
    let not_sub_ty = StructType::new(&engine, [imm_field(HeapType::NoFunc)])?;
    assert!(!not_sub_ty.matches(&super_ty));

    // Width.
    let sub_ty = StructType::with_finality_and_supertype(
        &engine,
        Finality::Final,
        Some(&super_ty),
        [imm_field(HeapType::Func), imm_field(HeapType::Extern)],
    )?;
    assert!(sub_ty.matches(&super_ty));
    let not_sub_ty = StructType::new(
        &engine,
        [imm_field(HeapType::Func), imm_field(HeapType::Extern)],
    )?;
    assert!(!not_sub_ty.matches(&super_ty));

    // Depth and width.
    let sub_ty = StructType::with_finality_and_supertype(
        &engine,
        Finality::Final,
        Some(&super_ty),
        [imm_field(HeapType::NoFunc), imm_field(HeapType::Extern)],
    )?;
    assert!(sub_ty.matches(&super_ty));
    let not_sub_ty = StructType::new(
        &engine,
        [imm_field(HeapType::NoFunc), imm_field(HeapType::Extern)],
    )?;
    assert!(!not_sub_ty.matches(&super_ty));

    // Unrelated structs.
    let not_sub_ty = StructType::new(&engine, [imm_field(HeapType::Extern)])?;
    assert!(!not_sub_ty.matches(&super_ty));
    let not_sub_ty = StructType::new(&engine, [field(HeapType::Extern)])?;
    assert!(!not_sub_ty.matches(&super_ty));
    let not_sub_ty = StructType::new(&engine, [])?;
    assert!(!not_sub_ty.matches(&super_ty));

    Ok(())
}

#[test]
fn struct_subtyping_fields_must_match() -> Result<()> {
    let engine = Engine::default();

    let a = StructType::with_finality_and_supertype(
        &engine,
        Finality::NonFinal,
        None,
        [imm_field(HeapType::Any)],
    )?;

    for (msg, expected, fields) in [
        ("Missing field", false, vec![]),
        (
            "Non-matching field",
            false,
            vec![imm_field(HeapType::Extern)],
        ),
        ("Wrong mutability field", false, vec![field(HeapType::Any)]),
        ("Exact match is okay", true, vec![imm_field(HeapType::Any)]),
        (
            "Subtype of the field is okay",
            true,
            vec![imm_field(HeapType::Eq)],
        ),
        (
            "Extra fields are okay",
            true,
            vec![imm_field(HeapType::Any), imm_field(HeapType::Extern)],
        ),
    ] {
        let actual =
            StructType::with_finality_and_supertype(&engine, Finality::NonFinal, Some(&a), fields)
                .is_ok();
        assert_eq!(
            expected, actual,
            "expected valid? {expected}; actually valid? {actual}; {msg}"
        );
    }

    Ok(())
}

#[test]
fn struct_subtyping_supertype_and_finality() -> Result<()> {
    let engine = Engine::default();

    for (expected, finality) in [(true, Finality::NonFinal), (false, Finality::Final)] {
        let a = StructType::with_finality_and_supertype(&engine, finality, None, [])?;
        let actual =
            StructType::with_finality_and_supertype(&engine, Finality::Final, Some(&a), []).is_ok();
        assert_eq!(expected, actual);
    }

    Ok(())
}

#[test]
fn struct_subtyping() -> Result<()> {
    let engine = Engine::default();

    // These types produce the following trees:
    //
    //                base               g
    //               /    \             /
    //              a      b           h
    //             / \                /
    //            c   d              i
    //           /
    //          e
    //         /
    //        f
    let base = StructType::with_finality_and_supertype(&engine, Finality::NonFinal, None, [])?;
    let a = StructType::with_finality_and_supertype(&engine, Finality::NonFinal, Some(&base), [])?;
    let b = StructType::with_finality_and_supertype(
        &engine,
        Finality::NonFinal,
        Some(&base),
        // Have to add a field so that `b` doesn't dedupe to `a`.
        [field(HeapType::Any)],
    )?;
    let c = StructType::with_finality_and_supertype(&engine, Finality::NonFinal, Some(&a), [])?;
    let d = StructType::with_finality_and_supertype(
        &engine,
        Finality::NonFinal,
        Some(&a),
        // Have to add a field so that `d` doesn't dedupe to `c`.
        [field(HeapType::Any)],
    )?;
    let e = StructType::with_finality_and_supertype(&engine, Finality::NonFinal, Some(&c), [])?;
    let f = StructType::with_finality_and_supertype(&engine, Finality::NonFinal, Some(&e), [])?;
    let g = StructType::with_finality_and_supertype(
        &engine,
        Finality::NonFinal,
        None,
        // Have to add a field so that `g` doesn't dedupe to `base`.
        [field(HeapType::Any)],
    )?;
    let h = StructType::with_finality_and_supertype(
        &engine,
        Finality::NonFinal,
        Some(&g),
        [field(HeapType::Any)],
    )?;
    let i = StructType::with_finality_and_supertype(
        &engine,
        Finality::NonFinal,
        Some(&h),
        [field(HeapType::Any)],
    )?;

    for (expected, sub_name, sub, sup_name, sup) in [
        // Identity, at root.
        (true, "base", &base, "base", &base),
        // Identity, in middle.
        (true, "c", &c, "c", &c),
        // Identity, at leaf.
        (true, "f", &f, "f", &f),
        // Direct, at root.
        (true, "a", &a, "base", &base),
        // Direct, in middle.
        (true, "c", &c, "a", &a),
        // Direct, at leaf.
        (true, "f", &f, "e", &e),
        // Transitive, at root.
        (true, "c", &c, "base", &base),
        // Transitive, in middle.
        (true, "e", &e, "a", &a),
        // Transitive, at leaf.
        (true, "f", &f, "c", &c),
        // Unrelated roots.
        (false, "base", &base, "g", &g),
        (false, "g", &g, "base", &base),
        // Unrelated siblings.
        (false, "a", &a, "b", &b),
        (false, "b", &b, "a", &a),
        (false, "c", &c, "d", &d),
        (false, "d", &d, "c", &c),
        // Unrelated root and middle.
        (false, "base", &base, "h", &h),
        (false, "h", &h, "base", &base),
        // Unrelated root and leaf.
        (false, "base", &base, "i", &i),
        (false, "i", &i, "base", &base),
        // Unrelated middles.
        (false, "a", &a, "h", &h),
        (false, "h", &h, "a", &a),
        // Unrelated middle and leaf.
        (false, "a", &a, "i", &i),
        (false, "i", &i, "a", &a),
    ] {
        eprintln!("expect that `{sub_name} <: {sup_name}` is `{expected}`");
        let sub = HeapType::ConcreteStruct(sub.clone());
        let sup = HeapType::ConcreteStruct(sup.clone());
        let actual = sub.matches(&sup);
        assert_eq!(expected, actual);
    }

    Ok(())
}

#[test]
fn array_subtyping_field_must_match() -> Result<()> {
    let engine = Engine::default();

    let a = ArrayType::with_finality_and_supertype(
        &engine,
        Finality::NonFinal,
        None,
        imm_field(HeapType::Any),
    )?;

    for (expected, field) in [
        // Non-matching field.
        (false, imm_field(HeapType::Extern)),
        // Wrong mutability field.
        (false, field(HeapType::Any)),
        // Exact match is okay.
        (true, imm_field(HeapType::Any)),
        // Subtype of the field is okay.
        (true, imm_field(HeapType::Eq)),
    ] {
        let actual =
            ArrayType::with_finality_and_supertype(&engine, Finality::NonFinal, Some(&a), field)
                .is_ok();
        assert_eq!(expected, actual);
    }

    Ok(())
}

#[test]
fn array_subtyping_supertype_and_finality() -> Result<()> {
    let engine = Engine::default();

    for (expected, finality) in [(true, Finality::NonFinal), (false, Finality::Final)] {
        let superty =
            ArrayType::with_finality_and_supertype(&engine, finality, None, field(HeapType::Any))?;
        let actual = ArrayType::with_finality_and_supertype(
            &engine,
            Finality::Final,
            Some(&superty),
            field(HeapType::Any),
        )
        .is_ok();
        assert_eq!(expected, actual);
    }

    Ok(())
}

#[test]
fn array_subtyping() -> Result<()> {
    let engine = Engine::default();

    // These types produce the following trees:
    //
    //                base               g
    //               /    \             /
    //              a      b           h
    //             / \                /
    //            c   d              i
    //           /
    //          e
    //         /
    //        f
    let base = ArrayType::with_finality_and_supertype(
        &engine,
        Finality::NonFinal,
        None,
        imm_field(HeapType::Any),
    )?;
    let a = ArrayType::with_finality_and_supertype(
        &engine,
        Finality::NonFinal,
        Some(&base),
        imm_field(HeapType::Any),
    )?;
    let b = ArrayType::with_finality_and_supertype(
        &engine,
        Finality::NonFinal,
        Some(&base),
        imm_field(HeapType::Eq),
    )?;
    let c = ArrayType::with_finality_and_supertype(
        &engine,
        Finality::NonFinal,
        Some(&a),
        imm_field(HeapType::Any),
    )?;
    let d = ArrayType::with_finality_and_supertype(
        &engine,
        Finality::NonFinal,
        Some(&a),
        imm_field(HeapType::Eq),
    )?;
    let e = ArrayType::with_finality_and_supertype(
        &engine,
        Finality::NonFinal,
        Some(&c),
        imm_field(HeapType::Any),
    )?;
    let f = ArrayType::with_finality_and_supertype(
        &engine,
        Finality::NonFinal,
        Some(&e),
        imm_field(HeapType::Any),
    )?;
    let g = ArrayType::with_finality_and_supertype(
        &engine,
        Finality::NonFinal,
        None,
        imm_field(HeapType::Eq),
    )?;
    let h = ArrayType::with_finality_and_supertype(
        &engine,
        Finality::NonFinal,
        Some(&g),
        imm_field(HeapType::Eq),
    )?;
    let i = ArrayType::with_finality_and_supertype(
        &engine,
        Finality::NonFinal,
        Some(&h),
        imm_field(HeapType::Eq),
    )?;

    for (expected, sub_name, sub, sup_name, sup) in [
        // Identity, at root.
        (true, "base", &base, "base", &base),
        // Identity, in middle.
        (true, "c", &c, "c", &c),
        // Identity, at leaf.
        (true, "f", &f, "f", &f),
        // Direct, at root.
        (true, "a", &a, "base", &base),
        // Direct, in middle.
        (true, "c", &c, "a", &a),
        // Direct, at leaf.
        (true, "f", &f, "e", &e),
        // Transitive, at root.
        (true, "c", &c, "base", &base),
        // Transitive, in middle.
        (true, "e", &e, "a", &a),
        // Transitive, at leaf.
        (true, "f", &f, "c", &c),
        // Unrelated roots.
        (false, "base", &base, "g", &g),
        (false, "g", &g, "base", &base),
        // Unrelated siblings.
        (false, "a", &a, "b", &b),
        (false, "b", &b, "a", &a),
        (false, "c", &c, "d", &d),
        (false, "d", &d, "c", &c),
        // Unrelated root and middle.
        (false, "base", &base, "h", &h),
        (false, "h", &h, "base", &base),
        // Unrelated root and leaf.
        (false, "base", &base, "i", &i),
        (false, "i", &i, "base", &base),
        // Unrelated middles.
        (false, "a", &a, "h", &h),
        (false, "h", &h, "a", &a),
        // Unrelated middle and leaf.
        (false, "a", &a, "i", &i),
        (false, "i", &i, "a", &a),
    ] {
        eprintln!("expect that `{sub_name} <: {sup_name}` is `{expected}`");
        let sub = HeapType::ConcreteArray(sub.clone());
        let sup = HeapType::ConcreteArray(sup.clone());
        let actual = sub.matches(&sup);
        assert_eq!(expected, actual);
    }

    Ok(())
}

#[test]
fn func_subtyping_field_must_match() -> Result<()> {
    let engine = Engine::default();

    let superty = FuncType::with_finality_and_supertype(
        &engine,
        Finality::NonFinal,
        None,
        [valty(HeapType::Struct)],
        [valty(HeapType::Any)],
    )?;

    for (expected, param, ret) in [
        // Non-matching param type.
        (false, valty(HeapType::Extern), valty(HeapType::Any)),
        // Non-matching return type.
        (false, valty(HeapType::Struct), valty(HeapType::Extern)),
        // Exact match is okay.
        (true, valty(HeapType::Struct), valty(HeapType::Any)),
        // Subtype of the return type is okay.
        (true, valty(HeapType::Struct), valty(HeapType::Eq)),
        // Supertype of the param type is okay.
        (true, valty(HeapType::Eq), valty(HeapType::Any)),
    ] {
        let actual = FuncType::with_finality_and_supertype(
            &engine,
            Finality::NonFinal,
            Some(&superty),
            [param],
            [ret],
        )
        .is_ok();
        assert_eq!(expected, actual);
    }

    Ok(())
}

#[test]
fn func_subtyping_supertype_and_finality() -> Result<()> {
    let engine = Engine::default();

    for (expected, finality) in [(true, Finality::NonFinal), (false, Finality::Final)] {
        let superty = FuncType::with_finality_and_supertype(
            &engine,
            finality,
            None,
            [],
            [valty(HeapType::Any)],
        )?;
        let actual = FuncType::with_finality_and_supertype(
            &engine,
            Finality::Final,
            Some(&superty),
            [],
            [valty(HeapType::Any)],
        )
        .is_ok();
        assert_eq!(expected, actual);
    }

    Ok(())
}

#[test]
fn func_subtyping() -> Result<()> {
    let engine = Engine::default();

    // These types produce the following trees:
    //
    //                base               g
    //               /    \             /
    //              a      b           h
    //             / \                /
    //            c   d              i
    //           /
    //          e
    //         /
    //        f
    let base = FuncType::with_finality_and_supertype(
        &engine,
        Finality::NonFinal,
        None,
        [],
        [valty(HeapType::Any)],
    )?;
    let a = FuncType::with_finality_and_supertype(
        &engine,
        Finality::NonFinal,
        Some(&base),
        [],
        [valty(HeapType::Any)],
    )?;
    let b = FuncType::with_finality_and_supertype(
        &engine,
        Finality::NonFinal,
        Some(&base),
        [],
        [valty(HeapType::Eq)],
    )?;
    let c = FuncType::with_finality_and_supertype(
        &engine,
        Finality::NonFinal,
        Some(&a),
        [],
        [valty(HeapType::Any)],
    )?;
    let d = FuncType::with_finality_and_supertype(
        &engine,
        Finality::NonFinal,
        Some(&a),
        [],
        [valty(HeapType::Eq)],
    )?;
    let e = FuncType::with_finality_and_supertype(
        &engine,
        Finality::NonFinal,
        Some(&c),
        [],
        [valty(HeapType::Any)],
    )?;
    let f = FuncType::with_finality_and_supertype(
        &engine,
        Finality::NonFinal,
        Some(&e),
        [],
        [valty(HeapType::Any)],
    )?;
    let g = FuncType::with_finality_and_supertype(
        &engine,
        Finality::NonFinal,
        None,
        [],
        [valty(HeapType::Eq)],
    )?;
    let h = FuncType::with_finality_and_supertype(
        &engine,
        Finality::NonFinal,
        Some(&g),
        [],
        [valty(HeapType::Eq)],
    )?;
    let i = FuncType::with_finality_and_supertype(
        &engine,
        Finality::NonFinal,
        Some(&h),
        [],
        [valty(HeapType::Eq)],
    )?;

    for (expected, sub_name, sub, sup_name, sup) in [
        // Identity, at root.
        (true, "base", &base, "base", &base),
        // Identity, in middle.
        (true, "c", &c, "c", &c),
        // Identity, at leaf.
        (true, "f", &f, "f", &f),
        // Direct, at root.
        (true, "a", &a, "base", &base),
        // Direct, in middle.
        (true, "c", &c, "a", &a),
        // Direct, at leaf.
        (true, "f", &f, "e", &e),
        // Transitive, at root.
        (true, "c", &c, "base", &base),
        // Transitive, in middle.
        (true, "e", &e, "a", &a),
        // Transitive, at leaf.
        (true, "f", &f, "c", &c),
        // Unrelated roots.
        (false, "base", &base, "g", &g),
        (false, "g", &g, "base", &base),
        // Unrelated siblings.
        (false, "a", &a, "b", &b),
        (false, "b", &b, "a", &a),
        (false, "c", &c, "d", &d),
        (false, "d", &d, "c", &c),
        // Unrelated root and middle.
        (false, "base", &base, "h", &h),
        (false, "h", &h, "base", &base),
        // Unrelated root and leaf.
        (false, "base", &base, "i", &i),
        (false, "i", &i, "base", &base),
        // Unrelated middles.
        (false, "a", &a, "h", &h),
        (false, "h", &h, "a", &a),
        // Unrelated middle and leaf.
        (false, "a", &a, "i", &i),
        (false, "i", &i, "a", &a),
    ] {
        eprintln!("expect that `{sub_name} <: {sup_name}` is `{expected}`");
        let sub = HeapType::ConcreteFunc(sub.clone());
        let sup = HeapType::ConcreteFunc(sup.clone());
        let actual = sub.matches(&sup);
        assert_eq!(expected, actual);
    }

    Ok(())
}

#[test]
fn heap_type_matches_noexn() -> Result<()> {
    // Test that NoExn only matches exception-related types.
    // NoExn should NOT match unrelated heap types like Func, Extern, Any, etc.

    // NoExn should match exception hierarchy types
    assert!(HeapType::NoExn.matches(&HeapType::Exn));
    assert!(HeapType::NoExn.matches(&HeapType::NoExn));

    // NoExn should NOT match types outside the exception hierarchy
    assert!(!HeapType::NoExn.matches(&HeapType::Func));
    assert!(!HeapType::NoExn.matches(&HeapType::NoFunc));
    assert!(!HeapType::NoExn.matches(&HeapType::Extern));
    assert!(!HeapType::NoExn.matches(&HeapType::NoExtern));
    assert!(!HeapType::NoExn.matches(&HeapType::Any));
    assert!(!HeapType::NoExn.matches(&HeapType::Eq));
    assert!(!HeapType::NoExn.matches(&HeapType::I31));
    assert!(!HeapType::NoExn.matches(&HeapType::Struct));
    assert!(!HeapType::NoExn.matches(&HeapType::Array));
    assert!(!HeapType::NoExn.matches(&HeapType::None));
    assert!(!HeapType::NoExn.matches(&HeapType::Cont));
    assert!(!HeapType::NoExn.matches(&HeapType::NoCont));

    Ok(())
}

/// Extract the concrete struct type referenced by a `(ref ... <struct>)` field.
fn field_concrete_struct(field: &FieldType) -> StructType {
    match field.element_type() {
        StorageType::ValType(ValType::Ref(r)) => match r.heap_type() {
            HeapType::ConcreteStruct(s) => s.clone(),
            other => panic!("expected concrete struct heap type, got {other:?}"),
        },
        other => panic!("expected a ref-typed field, got {other:?}"),
    }
}

#[test]
fn rec_group_self_reference() -> Result<()> {
    let engine = Engine::default();

    let mut builder = RecGroupBuilder::new(&engine);
    let node = builder.declare();
    // forward_ref_field(mutability, is_nullable, target)
    builder
        .define_struct(node)
        .forward_ref_field(Mutability::Const, true, node);
    let group = builder.build()?;

    assert_eq!(group.len(), 1);
    let node = group.get_struct(node).unwrap();
    assert_eq!(node.fields().len(), 1);

    let field = node.field(0).unwrap();
    assert!(StructType::eq(&field_concrete_struct(&field), &node));
    Ok(())
}

#[test]
fn rec_group_mutual_recursion() -> Result<()> {
    let engine = Engine::default();

    let mut builder = RecGroupBuilder::new(&engine);
    let s1 = builder.declare();
    let s2 = builder.declare();
    builder
        .define_struct(s1)
        .forward_ref_field(Mutability::Var, true, s2);
    builder
        .define_struct(s2)
        .forward_ref_field(Mutability::Const, false, s1);
    let group = builder.build()?;

    assert_eq!(group.len(), 2);
    let s1 = group.get_struct(s1).unwrap();
    let s2 = group.get_struct(s2).unwrap();

    let f1 = s1.field(0).unwrap();
    match f1.element_type() {
        StorageType::ValType(ValType::Ref(r)) => assert!(r.is_nullable()),
        other => panic!("unexpected {other:?}"),
    }
    assert!(StructType::eq(&field_concrete_struct(&f1), &s2));

    let f2 = s2.field(0).unwrap();
    match f2.element_type() {
        StorageType::ValType(ValType::Ref(r)) => assert!(!r.is_nullable()),
        other => panic!("unexpected {other:?}"),
    }
    assert!(StructType::eq(&field_concrete_struct(&f2), &s1));
    Ok(())
}

#[test]
fn rec_group_mixed_concrete_and_local() -> Result<()> {
    let engine = Engine::default();

    // An already-registered, one-off struct type.
    let existing = StructType::new(
        &engine,
        [FieldType::new(
            Mutability::Const,
            StorageType::ValType(ValType::I32),
        )],
    )?;

    let mut builder = RecGroupBuilder::new(&engine);
    let node = builder.declare();
    builder
        .define_struct(node)
        // forward ref to the sibling
        .forward_ref_field(Mutability::Var, true, node)
        // ref to an already-registered type
        .field(FieldType::new(
            Mutability::Const,
            StorageType::ValType(
                RefType::new(false, HeapType::ConcreteStruct(existing.clone())).into(),
            ),
        ))
        // a plain scalar
        .field(FieldType::new(
            Mutability::Var,
            StorageType::ValType(ValType::I64),
        ));
    let group = builder.build()?;
    let node = group.get_struct(node).unwrap();

    assert_eq!(node.fields().len(), 3);
    assert!(StructType::eq(
        &field_concrete_struct(&node.field(0).unwrap()),
        &node
    ));
    assert!(StructType::eq(
        &field_concrete_struct(&node.field(1).unwrap()),
        &existing
    ));
    assert!(node.field(2).unwrap().element_type().is_val_type());
    Ok(())
}

#[test]
fn rec_group_dedup() -> Result<()> {
    let engine = Engine::default();

    let build = || -> Result<StructType> {
        let mut builder = RecGroupBuilder::new(&engine);
        let s1 = builder.declare();
        let s2 = builder.declare();
        builder
            .define_struct(s1)
            .forward_ref_field(Mutability::Const, true, s2);
        builder
            .define_struct(s2)
            .forward_ref_field(Mutability::Const, false, s1);
        Ok(builder.build()?.get_struct(s1).unwrap())
    };

    let a = build()?;
    let b = build()?;
    // Two identical rec groups are hash-consed to the same registered types.
    assert!(StructType::eq(&a, &b));
    Ok(())
}

#[test]
fn rec_group_array_and_func_recursive() -> Result<()> {
    let engine = Engine::default();

    let mut builder = RecGroupBuilder::new(&engine);
    let node = builder.declare();
    let arr = builder.declare();
    let func = builder.declare();

    builder
        .define_struct(node)
        .forward_ref_field(Mutability::Var, true, arr)
        .forward_ref_field(Mutability::Var, true, func);
    builder
        .define_array(arr)
        .forward_ref_element(Mutability::Var, true, node);
    builder
        .define_func(func)
        .forward_ref_param(true, node)
        .forward_ref_result(true, node);

    let group = builder.build()?;
    assert_eq!(group.len(), 3);

    let arr = group.get_array(arr).unwrap();
    let func = group.get_func(func).unwrap();
    assert_eq!(func.params().len(), 1);
    assert_eq!(func.results().len(), 1);
    assert!(arr.field_type().element_type().is_val_type());
    Ok(())
}

#[test]
fn rec_group_declared_but_undefined_errors() {
    let engine = Engine::default();
    let mut builder = RecGroupBuilder::new(&engine);
    let _s = builder.declare();
    let err = builder.build().unwrap_err();
    assert!(
        err.to_string().contains("declared but never defined"),
        "unexpected error: {err}"
    );
}

#[test]
fn rec_group_empty_errors() {
    let engine = Engine::default();
    let builder = RecGroupBuilder::new(&engine);
    assert!(builder.build().is_err());
}

#[test]
fn rec_group_array_missing_element_errors() {
    let engine = Engine::default();
    let mut builder = RecGroupBuilder::new(&engine);
    let a = builder.declare();
    builder.define_array(a);
    let err = builder.build().unwrap_err();
    assert!(
        err.to_string().contains("element type was never set"),
        "unexpected error: {err}"
    );
}

#[test]
fn rec_group_types_iter() -> Result<()> {
    let engine = Engine::default();
    let mut builder = RecGroupBuilder::new(&engine);
    let s = builder.declare();
    let a = builder.declare();
    builder.define_struct(s).field(FieldType::new(
        Mutability::Const,
        StorageType::ValType(ValType::I32),
    ));
    builder
        .define_array(a)
        .element(FieldType::new(Mutability::Var, StorageType::I8));
    let group = builder.build()?;

    let kinds: Vec<_> = group
        .types()
        .map(|t| match t {
            CompositeType::Struct(_) => "struct",
            CompositeType::Array(_) => "array",
            CompositeType::Func(_) => "func",
        })
        .collect();
    assert_eq!(kinds, ["struct", "array"]);
    Ok(())
}

#[test]
fn rec_group_subtyping_via_forward_supertype() -> Result<()> {
    let engine = Engine::default();

    let mut builder = RecGroupBuilder::new(&engine);
    let base = builder.declare();
    let derived = builder.declare();
    builder
        .define_struct(base)
        .finality(Finality::NonFinal)
        .field(FieldType::new(
            Mutability::Const,
            StorageType::ValType(ValType::I32),
        ));
    builder
        .define_struct(derived)
        .forward_supertype(base)
        // width subtype: keeps base's field, adds another
        .field(FieldType::new(
            Mutability::Const,
            StorageType::ValType(ValType::I32),
        ))
        .field(FieldType::new(
            Mutability::Const,
            StorageType::ValType(ValType::I64),
        ));
    let group = builder.build()?;

    let base = group.get_struct(base).unwrap();
    let derived = group.get_struct(derived).unwrap();
    assert_eq!(base.finality(), Finality::NonFinal);
    assert!(derived.matches(&base));
    assert!(StructType::eq(&derived.supertype().unwrap(), &base));
    Ok(())
}

#[test]
fn rec_group_subtyping_via_known_supertype() -> Result<()> {
    let engine = Engine::default();

    let base = StructType::with_finality_and_supertype(
        &engine,
        Finality::NonFinal,
        None,
        [FieldType::new(
            Mutability::Const,
            StorageType::ValType(ValType::I32),
        )],
    )?;

    let mut builder = RecGroupBuilder::new(&engine);
    let derived = builder.declare();
    builder
        .define_struct(derived)
        .supertype(base.clone())
        .field(FieldType::new(
            Mutability::Const,
            StorageType::ValType(ValType::I32),
        ));
    let group = builder.build()?;

    assert!(group.get_struct(derived).unwrap().matches(&base));
    Ok(())
}

#[test]
fn rec_group_final_supertype_errors() {
    let engine = Engine::default();
    let mut builder = RecGroupBuilder::new(&engine);
    let base = builder.declare();
    let derived = builder.declare();
    // base is final (default)
    builder.define_struct(base).field(FieldType::new(
        Mutability::Const,
        StorageType::ValType(ValType::I32),
    ));
    builder
        .define_struct(derived)
        .forward_supertype(base)
        .field(FieldType::new(
            Mutability::Const,
            StorageType::ValType(ValType::I32),
        ));
    let err = builder.build().unwrap_err();
    assert!(
        err.to_string().contains("final supertype"),
        "unexpected error: {err}"
    );
}

#[test]
fn rec_group_supertype_mismatch_errors() {
    let engine = Engine::default();
    let mut builder = RecGroupBuilder::new(&engine);
    let base = builder.declare();
    let derived = builder.declare();
    builder
        .define_struct(base)
        .finality(Finality::NonFinal)
        // mutable field: subtype must match it exactly
        .field(FieldType::new(
            Mutability::Var,
            StorageType::ValType(ValType::I32),
        ));
    builder
        .define_struct(derived)
        .forward_supertype(base)
        .field(
            // i64 != i32 -> mismatch
            FieldType::new(Mutability::Var, StorageType::ValType(ValType::I64)),
        );
    let err = builder.build().unwrap_err();
    assert!(
        err.to_string().contains("must match their supertype"),
        "unexpected error: {err}"
    );
}

#[test]
fn rec_group_wrong_kind_supertype_errors() {
    let engine = Engine::default();
    let mut builder = RecGroupBuilder::new(&engine);
    let base = builder.declare();
    let derived = builder.declare();
    // base is an array, but we use it as a struct's supertype
    builder
        .define_array(base)
        .finality(Finality::NonFinal)
        .element(FieldType::new(Mutability::Const, StorageType::I8));
    builder
        .define_struct(derived)
        .forward_supertype(base)
        .field(FieldType::new(
            Mutability::Const,
            StorageType::ValType(ValType::I32),
        ));
    let err = builder.build().unwrap_err();
    assert!(
        err.to_string().contains("supertype must be a struct"),
        "unexpected error: {err}"
    );
}

#[test]
fn rec_group_nonfinal_root_without_supertype() -> Result<()> {
    let engine = Engine::default();
    let mut builder = RecGroupBuilder::new(&engine);
    let root = builder.declare();
    builder
        .define_struct(root)
        .finality(Finality::NonFinal)
        .field(FieldType::new(
            Mutability::Const,
            StorageType::ValType(ValType::I32),
        ));
    let group = builder.build()?;
    assert_eq!(
        group.get_struct(root).unwrap().finality(),
        Finality::NonFinal
    );
    Ok(())
}
