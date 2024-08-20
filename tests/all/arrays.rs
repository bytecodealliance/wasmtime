use wasmtime::*;

fn gc_store() -> Result<Store<()>> {
    let _ = env_logger::try_init();

    let mut config = Config::new();
    config.wasm_function_references(true);
    config.wasm_gc(true);

    let engine = Engine::new(&config)?;
    Ok(Store::new(&engine, ()))
}

#[test]
fn array_new_empty() -> Result<()> {
    let mut store = gc_store()?;
    let array_ty = ArrayType::new(
        store.engine(),
        FieldType::new(Mutability::Const, StorageType::I8),
    );
    let pre = ArrayRefPre::new(&mut store, array_ty);
    let array = ArrayRef::new(&mut store, &pre, &Val::I32(0), 0)?;
    assert_eq!(array.len(&store)?, 0);
    Ok(())
}

#[test]
fn array_new_with_elems() -> Result<()> {
    let mut store = gc_store()?;
    let array_ty = ArrayType::new(
        store.engine(),
        FieldType::new(Mutability::Const, ValType::I32.into()),
    );
    let pre = ArrayRefPre::new(&mut store, array_ty);
    let array = ArrayRef::new(&mut store, &pre, &Val::I32(99), 3)?;
    assert_eq!(array.len(&store)?, 3);
    for i in 0..3 {
        assert_eq!(array.get(&mut store, i)?.unwrap_i32(), 99);
    }
    Ok(())
}

#[test]
fn array_new_unrooted_initial_elem() -> Result<()> {
    let mut store = gc_store()?;
    let array_ty = ArrayType::new(
        store.engine(),
        FieldType::new(Mutability::Var, StorageType::ValType(ValType::ANYREF)),
    );
    let pre = ArrayRefPre::new(&mut store, array_ty);

    // Passing an unrooted `anyref` to `StructRef::new` results in an error.
    let anyref = {
        let mut scope = RootScope::new(&mut store);
        AnyRef::from_i31(&mut scope, I31::new_i32(1234).unwrap())
    };

    assert!(ArrayRef::new(&mut store, &pre, &anyref.into(), 3).is_err());
    Ok(())
}

#[test]
#[should_panic = "wrong store"]
fn array_new_cross_store_initial_elem() {
    let mut store1 = gc_store().unwrap();
    let mut store2 = gc_store().unwrap();

    let array_ty = ArrayType::new(
        store1.engine(),
        FieldType::new(Mutability::Var, StorageType::ValType(ValType::ANYREF)),
    );
    let pre = ArrayRefPre::new(&mut store1, array_ty);

    // Passing an `anyref` from a different store to `ArrayRef::new` results in
    // a panic.
    let anyref = AnyRef::from_i31(&mut store2, I31::new_i32(1234).unwrap());
    ArrayRef::new(&mut store1, &pre, &anyref.into(), 3).unwrap();
}

#[test]
#[should_panic = "wrong store"]
fn array_new_cross_store_pre() {
    let mut store1 = gc_store().unwrap();
    let mut store2 = gc_store().unwrap();

    let array_ty = ArrayType::new(
        store1.engine(),
        FieldType::new(Mutability::Var, StorageType::ValType(ValType::ANYREF)),
    );
    let pre = ArrayRefPre::new(&mut store2, array_ty);

    ArrayRef::new(&mut store1, &pre, &Val::I32(0), 3).unwrap();
}

#[test]
fn array_new_fixed_empty() -> Result<()> {
    let mut store = gc_store()?;
    let array_ty = ArrayType::new(
        store.engine(),
        FieldType::new(Mutability::Const, ValType::I32.into()),
    );
    let pre = ArrayRefPre::new(&mut store, array_ty);
    let array = ArrayRef::new_fixed(&mut store, &pre, &[])?;
    assert_eq!(array.len(&store)?, 0);
    Ok(())
}

#[test]
fn array_new_fixed_with_elems() -> Result<()> {
    let mut store = gc_store()?;
    let array_ty = ArrayType::new(
        store.engine(),
        FieldType::new(Mutability::Const, ValType::I32.into()),
    );
    let pre = ArrayRefPre::new(&mut store, array_ty);
    let array = ArrayRef::new_fixed(
        &mut store,
        &pre,
        &[Val::I32(11), Val::I32(22), Val::I32(33)],
    )?;
    assert_eq!(array.len(&store)?, 3);
    for i in 0..3 {
        assert_eq!(array.get(&mut store, i)?.unwrap_i32(), (i as i32 + 1) * 11);
    }
    Ok(())
}

#[test]
fn array_new_fixed_unrooted_initial_elem() -> Result<()> {
    let mut store = gc_store()?;
    let array_ty = ArrayType::new(
        store.engine(),
        FieldType::new(Mutability::Var, StorageType::ValType(ValType::ANYREF)),
    );
    let pre = ArrayRefPre::new(&mut store, array_ty);

    // Passing an unrooted `anyref` to `StructRef::new` results in an error.
    let anyref = {
        let mut scope = RootScope::new(&mut store);
        AnyRef::from_i31(&mut scope, I31::new_i32(1234).unwrap())
    };

    assert!(ArrayRef::new_fixed(&mut store, &pre, &[anyref.into()]).is_err());
    Ok(())
}

#[test]
#[should_panic = "wrong store"]
fn array_new_fixed_cross_store_initial_elem() {
    let mut store1 = gc_store().unwrap();
    let mut store2 = gc_store().unwrap();

    let array_ty = ArrayType::new(
        store1.engine(),
        FieldType::new(Mutability::Var, StorageType::ValType(ValType::ANYREF)),
    );
    let pre = ArrayRefPre::new(&mut store1, array_ty);

    // Passing an `anyref` from a different store to `ArrayRef::new_fixed`
    // results in a panic.
    let anyref = AnyRef::from_i31(&mut store2, I31::new_i32(1234).unwrap());
    ArrayRef::new_fixed(&mut store1, &pre, &[anyref.into()]).unwrap();
}

#[test]
#[should_panic = "wrong store"]
fn array_new_fixed_cross_store_pre() {
    let mut store1 = gc_store().unwrap();
    let mut store2 = gc_store().unwrap();

    let array_ty = ArrayType::new(
        store1.engine(),
        FieldType::new(Mutability::Var, StorageType::ValType(ValType::ANYREF)),
    );
    let pre = ArrayRefPre::new(&mut store2, array_ty);

    ArrayRef::new_fixed(&mut store1, &pre, &[Val::I32(0)]).unwrap();
}

#[test]
fn anyref_as_array() -> Result<()> {
    let mut store = gc_store()?;

    let array_ty = ArrayType::new(
        store.engine(),
        FieldType::new(Mutability::Const, StorageType::ValType(ValType::I32)),
    );
    let pre = ArrayRefPre::new(&mut store, array_ty);
    let a0 = ArrayRef::new_fixed(&mut store, &pre, &[])?;

    let anyref: Rooted<AnyRef> = a0.into();
    assert!(anyref.is_array(&store)?);
    let a1 = anyref.as_array(&store)?.unwrap();
    assert!(Rooted::ref_eq(&store, &a0, &a1)?);

    let anyref = AnyRef::from_i31(&mut store, I31::new_i32(42).unwrap());
    assert!(!anyref.is_array(&store)?);
    assert!(anyref.as_array(&store)?.is_none());

    Ok(())
}

#[test]
fn array_len_empty() -> Result<()> {
    let mut store = gc_store()?;

    let array_ty = ArrayType::new(
        store.engine(),
        FieldType::new(Mutability::Const, ValType::I32.into()),
    );
    let pre = ArrayRefPre::new(&mut store, array_ty);

    let array = ArrayRef::new_fixed(&mut store, &pre, &[])?;
    assert_eq!(array.len(&store)?, 0);

    Ok(())
}

#[test]
fn array_len_non_empty() -> Result<()> {
    let mut store = gc_store()?;

    let array_ty = ArrayType::new(
        store.engine(),
        FieldType::new(Mutability::Const, ValType::I32.into()),
    );
    let pre = ArrayRefPre::new(&mut store, array_ty);

    let array = ArrayRef::new_fixed(
        &mut store,
        &pre,
        &[Val::I32(11), Val::I32(22), Val::I32(33)],
    )?;
    assert_eq!(array.len(&store)?, 3);

    Ok(())
}

#[test]
fn array_get_in_bounds() -> Result<()> {
    let mut store = gc_store()?;

    let array_ty = ArrayType::new(
        store.engine(),
        FieldType::new(Mutability::Const, ValType::I32.into()),
    );
    let pre = ArrayRefPre::new(&mut store, array_ty);

    let array = ArrayRef::new_fixed(
        &mut store,
        &pre,
        &[Val::I32(11), Val::I32(22), Val::I32(33)],
    )?;

    assert_eq!(array.get(&mut store, 0)?.unwrap_i32(), 11);
    assert_eq!(array.get(&mut store, 1)?.unwrap_i32(), 22);
    assert_eq!(array.get(&mut store, 2)?.unwrap_i32(), 33);

    Ok(())
}

#[test]
fn array_get_out_of_bounds() -> Result<()> {
    let mut store = gc_store()?;

    let array_ty = ArrayType::new(
        store.engine(),
        FieldType::new(Mutability::Const, ValType::I32.into()),
    );
    let pre = ArrayRefPre::new(&mut store, array_ty);

    let array = ArrayRef::new_fixed(
        &mut store,
        &pre,
        &[Val::I32(11), Val::I32(22), Val::I32(33)],
    )?;

    assert!(array.get(&mut store, 3).is_err());
    assert!(array.get(&mut store, 4).is_err());

    Ok(())
}

#[test]
fn array_get_on_unrooted() -> Result<()> {
    let mut store = gc_store()?;

    let array_ty = ArrayType::new(
        store.engine(),
        FieldType::new(Mutability::Const, ValType::I32.into()),
    );
    let pre = ArrayRefPre::new(&mut store, array_ty);

    let array = {
        let mut scope = RootScope::new(&mut store);
        ArrayRef::new_fixed(&mut scope, &pre, &[Val::I32(11)])?
    };

    assert!(array.get(&mut store, 0).is_err());
    Ok(())
}

#[test]
#[should_panic = "wrong store"]
fn array_get_wrong_store() {
    let mut store1 = gc_store().unwrap();
    let mut store2 = gc_store().unwrap();

    let array_ty = ArrayType::new(
        store1.engine(),
        FieldType::new(Mutability::Const, ValType::I32.into()),
    );
    let pre = ArrayRefPre::new(&mut store1, array_ty);

    let array = ArrayRef::new_fixed(&mut store1, &pre, &[Val::I32(11)]).unwrap();

    // Should panic.
    let _ = array.get(&mut store2, 0);
}

#[test]
fn array_set_in_bounds() -> Result<()> {
    let mut store = gc_store()?;

    let array_ty = ArrayType::new(
        store.engine(),
        FieldType::new(Mutability::Var, ValType::I32.into()),
    );
    let pre = ArrayRefPre::new(&mut store, array_ty);

    let array = ArrayRef::new_fixed(&mut store, &pre, &[Val::I32(11)])?;

    assert_eq!(array.get(&mut store, 0)?.unwrap_i32(), 11);
    array.set(&mut store, 0, Val::I32(22))?;
    assert_eq!(array.get(&mut store, 0)?.unwrap_i32(), 22);

    Ok(())
}

#[test]
fn array_set_out_of_bounds() -> Result<()> {
    let mut store = gc_store()?;

    let array_ty = ArrayType::new(
        store.engine(),
        FieldType::new(Mutability::Var, ValType::I32.into()),
    );
    let pre = ArrayRefPre::new(&mut store, array_ty);

    let array = ArrayRef::new_fixed(&mut store, &pre, &[Val::I32(11)])?;

    assert!(array.set(&mut store, 1, Val::I32(22)).is_err());
    assert!(array.set(&mut store, 2, Val::I32(33)).is_err());

    Ok(())
}

#[test]
fn array_set_on_unrooted() -> Result<()> {
    let mut store = gc_store()?;

    let array_ty = ArrayType::new(
        store.engine(),
        FieldType::new(Mutability::Var, ValType::I32.into()),
    );
    let pre = ArrayRefPre::new(&mut store, array_ty);

    let array = {
        let mut scope = RootScope::new(&mut store);
        ArrayRef::new_fixed(&mut scope, &pre, &[Val::I32(11)])?
    };

    assert!(array.set(&mut store, 0, Val::I32(22)).is_err());
    Ok(())
}

#[test]
fn array_set_given_unrooted() -> Result<()> {
    let mut store = gc_store()?;

    let array_ty = ArrayType::new(
        store.engine(),
        FieldType::new(Mutability::Var, ValType::ANYREF.into()),
    );
    let pre = ArrayRefPre::new(&mut store, array_ty);

    let array = ArrayRef::new_fixed(&mut store, &pre, &[Val::AnyRef(None)])?;

    let anyref = {
        let mut scope = RootScope::new(&mut store);
        AnyRef::from_i31(&mut scope, I31::new_i32(1234).unwrap())
    };

    assert!(array.set(&mut store, 0, anyref.into()).is_err());
    Ok(())
}

#[test]
#[should_panic = "wrong store"]
fn array_set_cross_store_value() {
    let mut store1 = gc_store().unwrap();
    let mut store2 = gc_store().unwrap();

    let array_ty = ArrayType::new(
        store1.engine(),
        FieldType::new(Mutability::Var, ValType::ANYREF.into()),
    );
    let pre = ArrayRefPre::new(&mut store1, array_ty);

    let array = ArrayRef::new_fixed(&mut store1, &pre, &[Val::AnyRef(None)]).unwrap();

    let anyref = AnyRef::from_i31(&mut store2, I31::new_i32(1234).unwrap());

    // Should panic.
    let _ = array.set(&mut store1, 0, anyref.into());
}

#[test]
fn array_set_immutable_elems() -> Result<()> {
    let mut store = gc_store()?;

    let array_ty = ArrayType::new(
        store.engine(),
        FieldType::new(Mutability::Const, ValType::I32.into()),
    );
    let pre = ArrayRefPre::new(&mut store, array_ty);

    let array = ArrayRef::new_fixed(
        &mut store,
        &pre,
        &[Val::I32(11), Val::I32(22), Val::I32(33)],
    )?;

    assert!(array.set(&mut store, 0, Val::I32(22)).is_err());
    assert!(array.set(&mut store, 1, Val::I32(33)).is_err());
    assert!(array.set(&mut store, 2, Val::I32(44)).is_err());

    Ok(())
}

#[test]
fn array_set_wrong_field_type() -> Result<()> {
    let mut store = gc_store()?;

    let array_ty = ArrayType::new(
        store.engine(),
        FieldType::new(Mutability::Var, ValType::I32.into()),
    );
    let pre = ArrayRefPre::new(&mut store, array_ty);

    let array = ArrayRef::new_fixed(&mut store, &pre, &[Val::I32(11)])?;

    assert!(array.set(&mut store, 0, Val::I64(22)).is_err());
    Ok(())
}

#[test]
fn array_ty() -> Result<()> {
    let mut store = gc_store()?;
    let array_ty = ArrayType::new(
        store.engine(),
        FieldType::new(Mutability::Const, ValType::I32.into()),
    );
    let pre = ArrayRefPre::new(&mut store, array_ty.clone());
    let a = ArrayRef::new_fixed(&mut store, &pre, &[Val::I32(11)])?;
    assert!(ArrayType::eq(&array_ty, &a.ty(&store)?));
    Ok(())
}

#[test]
fn array_ty_unrooted() -> Result<()> {
    let mut store = gc_store()?;
    let array_ty = ArrayType::new(
        store.engine(),
        FieldType::new(Mutability::Const, ValType::I32.into()),
    );
    let pre = ArrayRefPre::new(&mut store, array_ty);
    let a = {
        let mut scope = RootScope::new(&mut store);
        ArrayRef::new_fixed(&mut scope, &pre, &[Val::I32(11)])?
    };
    assert!(a.ty(&store).is_err());
    Ok(())
}

#[test]
fn array_elems_empty() -> Result<()> {
    let mut store = gc_store()?;
    let array_ty = ArrayType::new(
        store.engine(),
        FieldType::new(Mutability::Const, ValType::I32.into()),
    );
    let pre = ArrayRefPre::new(&mut store, array_ty);
    let array = ArrayRef::new_fixed(&mut store, &pre, &[])?;
    let mut elems = array.elems(&mut store)?;
    assert_eq!(elems.len(), 0);
    assert!(elems.next().is_none());
    Ok(())
}

#[test]
fn array_elems_non_empty() -> Result<()> {
    let mut store = gc_store()?;
    let array_ty = ArrayType::new(
        store.engine(),
        FieldType::new(Mutability::Const, ValType::I32.into()),
    );
    let pre = ArrayRefPre::new(&mut store, array_ty);
    let array = ArrayRef::new_fixed(
        &mut store,
        &pre,
        &[Val::I32(11), Val::I32(22), Val::I32(33)],
    )?;
    let mut elems = array.elems(&mut store)?;
    assert_eq!(elems.len(), 3);
    assert_eq!(elems.next().unwrap().unwrap_i32(), 11);
    assert_eq!(elems.next().unwrap().unwrap_i32(), 22);
    assert_eq!(elems.next().unwrap().unwrap_i32(), 33);
    assert!(elems.next().is_none());
    Ok(())
}

#[test]
fn array_elems_unrooted() -> Result<()> {
    let mut store = gc_store()?;
    let array_ty = ArrayType::new(
        store.engine(),
        FieldType::new(Mutability::Const, ValType::I32.into()),
    );
    let pre = ArrayRefPre::new(&mut store, array_ty);
    let array = {
        let mut scope = RootScope::new(&mut store);
        ArrayRef::new_fixed(&mut scope, &pre, &[Val::I32(11)])?
    };
    assert!(array.elems(&mut store).is_err());
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn passing_arrays_through_wasm_with_untyped_calls() -> Result<()> {
    let mut store = gc_store()?;

    let module = Module::new(
        store.engine(),
        r#"
            (module
                (type (array i8))
                (import "" "" (func $f (param (ref 0)) (result (ref 0))))
                (func (export "run") (param (ref 0)) (result (ref 0))
                    (call $f (local.get 0))
                )
            )
        "#,
    )?;

    let array_ty = ArrayType::new(
        store.engine(),
        FieldType::new(Mutability::Const, StorageType::I8),
    );

    let ref_ty = RefType::new(false, HeapType::ConcreteArray(array_ty.clone()));
    let func_ty = FuncType::new(store.engine(), [ref_ty.clone().into()], [ref_ty.into()]);

    let func = Func::new(&mut store, func_ty, |mut caller, args, results| {
        let a = args[0].unwrap_any_ref().unwrap();
        let a = a.unwrap_array(&mut caller)?;
        assert_eq!(a.len(&mut caller)?, 1);
        assert_eq!(a.get(&mut caller, 0)?.unwrap_i32(), 42);
        results[0] = args[0];
        Ok(())
    });

    let instance = Instance::new(&mut store, &module, &[func.into()])?;
    let run = instance.get_func(&mut store, "run").unwrap();

    let pre = ArrayRefPre::new(&mut store, array_ty.clone());
    let a = ArrayRef::new_fixed(&mut store, &pre, &[Val::I32(42)])?;

    let mut results = vec![Val::null_any_ref()];
    run.call(&mut store, &[a.into()], &mut results)?;

    let a2 = results[0].unwrap_any_ref().unwrap();
    let a2 = a2.unwrap_array(&mut store)?;
    assert_eq!(a2.len(&mut store)?, 1);
    assert_eq!(a2.get(&mut store, 0)?.unwrap_i32(), 42);
    assert!(Rooted::ref_eq(&store, &a, &a2)?);

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn passing_arrays_through_wasm_with_typed_calls() -> Result<()> {
    let mut store = gc_store()?;

    let module = Module::new(
        store.engine(),
        r#"
            (module
                (type (array i8))
                (import "" "" (func $f (param (ref array)) (result (ref array))))
                (func (export "run") (param (ref 0)) (result (ref array))
                    (call $f (local.get 0))
                )
            )
        "#,
    )?;

    let array_ty = ArrayType::new(
        store.engine(),
        FieldType::new(Mutability::Const, StorageType::I8),
    );

    let func = Func::wrap(
        &mut store,
        |mut caller: Caller<()>, a: Rooted<ArrayRef>| -> Result<Rooted<ArrayRef>> {
            assert_eq!(a.len(&caller)?, 1);
            assert_eq!(a.get(&mut caller, 0)?.unwrap_i32(), 42);
            Ok(a)
        },
    );

    let instance = Instance::new(&mut store, &module, &[func.into()])?;
    let run = instance.get_typed_func::<Rooted<ArrayRef>, Rooted<ArrayRef>>(&mut store, "run")?;

    let pre = ArrayRefPre::new(&mut store, array_ty.clone());
    let a = ArrayRef::new_fixed(&mut store, &pre, &[Val::I32(42)])?;

    let a2 = run.call(&mut store, a)?;

    assert_eq!(a2.len(&store)?, 1);
    assert_eq!(a2.get(&mut store, 0)?.unwrap_i32(), 42);
    assert!(Rooted::ref_eq(&store, &a, &a2)?);

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn host_sets_array_global() -> Result<()> {
    let mut store = gc_store()?;

    let module = Module::new(
        store.engine(),
        r#"
            (module
                (type (array i8))
                (global $g (export "g") (mut (ref null 0)) (ref.null 0))
                (func (export "f") (result (ref null 0))
                    global.get $g
                )
            )
        "#,
    )?;

    let instance = Instance::new(&mut store, &module, &[])?;
    let g = instance.get_global(&mut store, "g").unwrap();

    let array_ty = ArrayType::new(
        store.engine(),
        FieldType::new(Mutability::Const, StorageType::I8),
    );
    let pre = ArrayRefPre::new(&mut store, array_ty.clone());
    let a0 = ArrayRef::new_fixed(&mut store, &pre, &[Val::I32(42)])?;
    g.set(&mut store, a0.into())?;

    // Get the global from the host.
    let val = g.get(&mut store);
    let anyref = val.unwrap_anyref().expect("non-null");
    let a1 = anyref.unwrap_array(&store)?;
    assert_eq!(a1.len(&store)?, 1);
    assert_eq!(a1.get(&mut store, 0)?.unwrap_i32(), 42);
    assert!(Rooted::ref_eq(&store, &a0, &a1)?);

    // Get the global from the guest.
    let f = instance.get_typed_func::<(), Option<Rooted<ArrayRef>>>(&mut store, "f")?;
    let a2 = f.call(&mut store, ())?.expect("non-null");
    assert_eq!(a2.len(&store)?, 1);
    assert_eq!(a2.get(&mut store, 0)?.unwrap_i32(), 42);
    assert!(Rooted::ref_eq(&store, &a0, &a2)?);

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn wasm_sets_array_global() -> Result<()> {
    let mut store = gc_store()?;

    let module = Module::new(
        store.engine(),
        r#"
            (module
                (type (array i8))
                (global $g (export "g") (mut (ref null 0)) (ref.null 0))
                (func (export "get") (result (ref null 0))
                    global.get $g
                )
                (func (export "set") (param (ref null 0))
                    local.get 0
                    global.set $g
                )
            )
        "#,
    )?;

    let array_ty = ArrayType::new(
        store.engine(),
        FieldType::new(Mutability::Const, StorageType::I8),
    );
    let pre = ArrayRefPre::new(&mut store, array_ty.clone());
    let a0 = ArrayRef::new_fixed(&mut store, &pre, &[Val::I32(42)])?;

    let instance = Instance::new(&mut store, &module, &[])?;
    let set = instance.get_func(&mut store, "set").unwrap();
    set.call(&mut store, &[a0.into()], &mut [])?;

    // Get the global from the host.
    let g = instance.get_global(&mut store, "g").unwrap();
    let val = g.get(&mut store);
    let anyref = val.unwrap_anyref().expect("non-null");
    let a1 = anyref.unwrap_array(&store)?;
    assert_eq!(a1.len(&store)?, 1);
    assert_eq!(a1.get(&mut store, 0)?.unwrap_i32(), 42);
    assert!(Rooted::ref_eq(&store, &a0, &a1)?);

    // Get the global from the guest.
    let f = instance.get_typed_func::<(), Option<Rooted<ArrayRef>>>(&mut store, "get")?;
    let a2 = f.call(&mut store, ())?.expect("non-null");
    assert_eq!(a2.len(&store)?, 1);
    assert_eq!(a2.get(&mut store, 0)?.unwrap_i32(), 42);
    assert!(Rooted::ref_eq(&store, &a0, &a2)?);

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn host_sets_array_in_table() -> Result<()> {
    let mut store = gc_store()?;

    let module = Module::new(
        store.engine(),
        r#"
            (module
                (type (array i8))
                (table $t (export "t") 1 1 (ref null 0) (ref.null 0))
                (func (export "f") (result (ref null 0))
                    i32.const 0
                    table.get $t
                )
            )
        "#,
    )?;

    let instance = Instance::new(&mut store, &module, &[])?;
    let t = instance.get_table(&mut store, "t").unwrap();

    let array_ty = ArrayType::new(
        store.engine(),
        FieldType::new(Mutability::Const, StorageType::I8),
    );
    let pre = ArrayRefPre::new(&mut store, array_ty.clone());
    let a0 = ArrayRef::new_fixed(&mut store, &pre, &[Val::I32(42)])?;
    t.set(&mut store, 0, a0.into())?;

    // Get the global from the host.
    let val = t.get(&mut store, 0).expect("in bounds");
    let anyref = val.unwrap_any().expect("non-null");
    let a1 = anyref.unwrap_array(&store)?;
    assert_eq!(a1.len(&store)?, 1);
    assert_eq!(a1.get(&mut store, 0)?.unwrap_i32(), 42);
    assert!(Rooted::ref_eq(&store, &a0, &a1)?);

    // Get the global from the guest.
    let f = instance.get_typed_func::<(), Option<Rooted<ArrayRef>>>(&mut store, "f")?;
    let a2 = f.call(&mut store, ())?.expect("non-null");
    assert_eq!(a2.len(&store)?, 1);
    assert_eq!(a2.get(&mut store, 0)?.unwrap_i32(), 42);
    assert!(Rooted::ref_eq(&store, &a0, &a2)?);

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn wasm_sets_array_in_table() -> Result<()> {
    let mut store = gc_store()?;

    let module = Module::new(
        store.engine(),
        r#"
            (module
                (type (array i8))
                (table $t (export "t") 1 1 (ref null 0) (ref.null 0))
                (func (export "get") (result (ref null 0))
                    i32.const 0
                    table.get $t
                )
                (func (export "set") (param (ref null 0))
                    i32.const 0
                    local.get 0
                    table.set $t
                )
            )
        "#,
    )?;

    let array_ty = ArrayType::new(
        store.engine(),
        FieldType::new(Mutability::Const, StorageType::I8),
    );
    let pre = ArrayRefPre::new(&mut store, array_ty.clone());
    let a0 = ArrayRef::new_fixed(&mut store, &pre, &[Val::I32(42)])?;

    let instance = Instance::new(&mut store, &module, &[])?;
    let set = instance.get_func(&mut store, "set").unwrap();
    set.call(&mut store, &[a0.into()], &mut [])?;

    // Get the global from the host.
    let t = instance.get_table(&mut store, "t").unwrap();
    let val = t.get(&mut store, 0).expect("in bounds");
    let anyref = val.unwrap_any().expect("non-null");
    let a1 = anyref.unwrap_array(&store)?;
    assert_eq!(a1.len(&mut store)?, 1);
    assert_eq!(a1.get(&mut store, 0)?.unwrap_i32(), 42);
    assert!(Rooted::ref_eq(&store, &a0, &a1)?);

    // Get the global from the guest.
    let f = instance.get_typed_func::<(), Option<Rooted<ArrayRef>>>(&mut store, "get")?;
    let a2 = f.call(&mut store, ())?.expect("non-null");
    assert_eq!(a2.len(&mut store)?, 1);
    assert_eq!(a2.get(&mut store, 0)?.unwrap_i32(), 42);
    assert!(Rooted::ref_eq(&store, &a0, &a2)?);

    Ok(())
}

#[test]
fn instantiate_with_array_global() -> Result<()> {
    let mut store = gc_store()?;

    let module = Module::new(
        store.engine(),
        r#"
            (module
                (type (array i8))
                (import "" "" (global (ref null 0)))
                (export "g" (global 0))
            )
        "#,
    )?;

    let array_ty = ArrayType::new(
        store.engine(),
        FieldType::new(Mutability::Const, StorageType::I8),
    );
    let global_ty = GlobalType::new(
        ValType::Ref(RefType::new(
            true,
            HeapType::ConcreteArray(array_ty.clone()),
        )),
        Mutability::Const,
    );

    // Instantiate with a null-ref global.
    let g = Global::new(&mut store, global_ty.clone(), Val::AnyRef(None))?;
    let instance = Instance::new(&mut store, &module, &[g.into()])?;
    let g = instance.get_global(&mut store, "g").expect("export exists");
    let val = g.get(&mut store);
    assert!(val.unwrap_anyref().is_none());

    // Instantiate with a non-null-ref global.
    let pre = ArrayRefPre::new(&mut store, array_ty);
    let a0 = ArrayRef::new_fixed(&mut store, &pre, &[Val::I32(42)])?;
    let g = Global::new(&mut store, global_ty, a0.into())?;
    let instance = Instance::new(&mut store, &module, &[g.into()])?;
    let g = instance.get_global(&mut store, "g").expect("export exists");
    let val = g.get(&mut store);
    let anyref = val.unwrap_anyref().expect("non-null");
    let a1 = anyref.unwrap_array(&store)?;
    assert_eq!(a1.len(&mut store)?, 1);
    assert_eq!(a1.get(&mut store, 0)?.unwrap_i32(), 42);
    assert!(Rooted::ref_eq(&store, &a0, &a1)?);

    Ok(())
}
