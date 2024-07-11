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
fn struct_new_empty() -> Result<()> {
    let mut store = gc_store()?;
    let struct_ty = StructType::new(store.engine(), [])?;
    let pre = StructRefPre::new(&mut store, struct_ty);
    StructRef::new(store, &pre, &[])?;
    Ok(())
}

#[test]
fn struct_new_with_fields() -> Result<()> {
    let mut store = gc_store()?;
    let struct_ty = StructType::new(
        store.engine(),
        [
            FieldType::new(Mutability::Const, StorageType::I8),
            FieldType::new(Mutability::Const, StorageType::ValType(ValType::I32)),
            FieldType::new(Mutability::Var, StorageType::ValType(ValType::ANYREF)),
        ],
    )?;
    let pre = StructRefPre::new(&mut store, struct_ty);
    StructRef::new(
        store,
        &pre,
        &[Val::I32(1), Val::I32(2), Val::null_any_ref()],
    )?;
    Ok(())
}

#[test]
fn struct_new_unrooted_field() -> Result<()> {
    let mut store = gc_store()?;
    let struct_ty = StructType::new(
        store.engine(),
        [FieldType::new(
            Mutability::Var,
            StorageType::ValType(ValType::ANYREF),
        )],
    )?;
    // Passing an unrooted `anyref` to `StructRef::new` results in an error.
    let anyref = {
        let mut scope = RootScope::new(&mut store);
        AnyRef::from_i31(&mut scope, I31::new_i32(1234).unwrap())
    };
    assert!(anyref.is_i31(&store).is_err());
    let pre = StructRefPre::new(&mut store, struct_ty);
    assert!(StructRef::new(store, &pre, &[anyref.into()]).is_err());
    Ok(())
}

#[test]
#[should_panic = "wrong store"]
fn struct_new_cross_store_field() {
    let mut store = gc_store().unwrap();
    let struct_ty = StructType::new(
        store.engine(),
        [FieldType::new(
            Mutability::Var,
            StorageType::ValType(ValType::ANYREF),
        )],
    )
    .unwrap();

    let mut other_store = gc_store().unwrap();
    let anyref = AnyRef::from_i31(&mut other_store, I31::new_i32(1234).unwrap());

    let pre = StructRefPre::new(&mut store, struct_ty);

    // This should panic.
    let _ = StructRef::new(store, &pre, &[anyref.into()]);
}

#[test]
#[should_panic = "wrong store"]
fn struct_new_cross_store_pre() {
    let mut store = gc_store().unwrap();
    let struct_ty = StructType::new(store.engine(), []).unwrap();

    let mut other_store = gc_store().unwrap();
    let pre = StructRefPre::new(&mut other_store, struct_ty);

    // This should panic.
    let _ = StructRef::new(&mut store, &pre, &[]);
}

#[test]
fn anyref_as_struct() -> Result<()> {
    let mut store = gc_store()?;

    let struct_ty = StructType::new(
        store.engine(),
        [FieldType::new(Mutability::Const, StorageType::I8)],
    )?;
    let pre = StructRefPre::new(&mut store, struct_ty.clone());
    let s0 = StructRef::new(&mut store, &pre, &[Val::I32(42)])?;

    let anyref: Rooted<AnyRef> = s0.into();
    assert!(anyref.is_struct(&store)?);
    let s1 = anyref.as_struct(&store)?.unwrap();
    assert_eq!(s1.field(&mut store, 0)?.unwrap_i32(), 42);
    assert!(Rooted::ref_eq(&store, &s0, &s1)?);

    let anyref: Rooted<AnyRef> = AnyRef::from_i31(&mut store, I31::new_i32(42).unwrap());
    assert!(!anyref.is_struct(&store)?);
    assert!(anyref.as_struct(&store)?.is_none());

    Ok(())
}

#[test]
fn struct_field_simple() -> Result<()> {
    let mut store = gc_store()?;
    let struct_ty = StructType::new(
        store.engine(),
        [FieldType::new(
            Mutability::Var,
            StorageType::ValType(ValType::I32),
        )],
    )?;
    let pre = StructRefPre::new(&mut store, struct_ty);
    let s = StructRef::new(&mut store, &pre, &[Val::I32(1234)])?;
    let val = s.field(&mut store, 0)?;
    assert_eq!(val.unwrap_i32(), 1234);
    Ok(())
}

#[test]
fn struct_field_out_of_bounds() -> Result<()> {
    let mut store = gc_store()?;
    let struct_ty = StructType::new(
        store.engine(),
        [FieldType::new(
            Mutability::Var,
            StorageType::ValType(ValType::I32),
        )],
    )?;
    let pre = StructRefPre::new(&mut store, struct_ty);
    let s = StructRef::new(&mut store, &pre, &[Val::I32(1234)])?;
    assert!(s.field(&mut store, 1).is_err());
    Ok(())
}

#[test]
fn struct_field_on_unrooted() -> Result<()> {
    let mut store = gc_store()?;
    let struct_ty = StructType::new(
        store.engine(),
        [FieldType::new(
            Mutability::Var,
            StorageType::ValType(ValType::I32),
        )],
    )?;
    let pre = StructRefPre::new(&mut store, struct_ty);
    let s = {
        let mut scope = RootScope::new(&mut store);
        StructRef::new(&mut scope, &pre, &[Val::I32(1234)])?
    };
    // The root scope ended and unrooted `s`.
    assert!(s.field(&mut store, 0).is_err());
    Ok(())
}

#[test]
fn struct_set_field_simple() -> Result<()> {
    let mut store = gc_store()?;
    let struct_ty = StructType::new(
        store.engine(),
        [FieldType::new(
            Mutability::Var,
            StorageType::ValType(ValType::I32),
        )],
    )?;
    let pre = StructRefPre::new(&mut store, struct_ty);
    let s = StructRef::new(&mut store, &pre, &[Val::I32(1234)])?;
    s.set_field(&mut store, 0, Val::I32(5678))?;
    let val = s.field(&mut store, 0)?;
    assert_eq!(val.unwrap_i32(), 5678);
    Ok(())
}

#[test]
fn struct_set_field_out_of_bounds() -> Result<()> {
    let mut store = gc_store()?;
    let struct_ty = StructType::new(
        store.engine(),
        [FieldType::new(
            Mutability::Var,
            StorageType::ValType(ValType::I32),
        )],
    )?;
    let pre = StructRefPre::new(&mut store, struct_ty);
    let s = StructRef::new(&mut store, &pre, &[Val::I32(1234)])?;
    assert!(s.set_field(&mut store, 1, Val::I32(1)).is_err());
    Ok(())
}

#[test]
fn struct_set_field_on_unrooted() -> Result<()> {
    let mut store = gc_store()?;
    let struct_ty = StructType::new(
        store.engine(),
        [FieldType::new(
            Mutability::Var,
            StorageType::ValType(ValType::I32),
        )],
    )?;
    let pre = StructRefPre::new(&mut store, struct_ty);
    let s = {
        let mut scope = RootScope::new(&mut store);
        StructRef::new(&mut scope, &pre, &[Val::I32(1234)])?
    };
    // The root scope ended and unrooted `s`.
    assert!(s.set_field(&mut store, 0, Val::I32(1)).is_err());
    Ok(())
}

#[test]
fn struct_set_field_with_unrooted() -> Result<()> {
    let mut store = gc_store()?;
    let struct_ty = StructType::new(
        store.engine(),
        [FieldType::new(
            Mutability::Var,
            StorageType::ValType(ValType::ANYREF),
        )],
    )?;
    let pre = StructRefPre::new(&mut store, struct_ty);
    let s = StructRef::new(&mut store, &pre, &[Val::null_any_ref()])?;
    let anyref = {
        let mut scope = RootScope::new(&mut store);
        AnyRef::from_i31(&mut scope, I31::wrapping_i32(42))
    };
    // The root scope ended and `anyref` is unrooted.
    assert!(s.set_field(&mut store, 0, anyref.into()).is_err());
    Ok(())
}

#[test]
fn struct_set_field_cross_store_value() -> Result<()> {
    let mut store = gc_store()?;
    let struct_ty = StructType::new(
        store.engine(),
        [FieldType::new(
            Mutability::Var,
            StorageType::ValType(ValType::EXTERNREF),
        )],
    )?;
    let pre = StructRefPre::new(&mut store, struct_ty);
    let s = StructRef::new(&mut store, &pre, &[Val::null_extern_ref()])?;

    let mut other_store = gc_store()?;
    let externref = ExternRef::new(&mut other_store, "blah")?;

    assert!(s.set_field(&mut store, 0, externref.into()).is_err());
    Ok(())
}

#[test]
fn struct_set_field_immutable() -> Result<()> {
    let mut store = gc_store()?;
    let struct_ty = StructType::new(
        store.engine(),
        [FieldType::new(
            Mutability::Const,
            StorageType::ValType(ValType::I32),
        )],
    )?;
    let pre = StructRefPre::new(&mut store, struct_ty);
    let s = StructRef::new(&mut store, &pre, &[Val::I32(1234)])?;
    assert!(s.set_field(&mut store, 0, Val::I32(5678)).is_err());
    Ok(())
}

#[test]
fn struct_set_field_wrong_type() -> Result<()> {
    let mut store = gc_store()?;
    let struct_ty = StructType::new(
        store.engine(),
        [FieldType::new(
            Mutability::Const,
            StorageType::ValType(ValType::I32),
        )],
    )?;
    let pre = StructRefPre::new(&mut store, struct_ty);
    let s = StructRef::new(&mut store, &pre, &[Val::I32(1234)])?;
    assert!(s.set_field(&mut store, 0, Val::I64(5678)).is_err());
    Ok(())
}

#[test]
fn struct_ty() -> Result<()> {
    let mut store = gc_store()?;
    let struct_ty = StructType::new(store.engine(), [])?;
    let pre = StructRefPre::new(&mut store, struct_ty.clone());
    let s = StructRef::new(&mut store, &pre, &[])?;
    assert!(StructType::eq(&struct_ty, &s.ty(&store)?));
    Ok(())
}

#[test]
fn struct_ty_unrooted() -> Result<()> {
    let mut store = gc_store()?;
    let struct_ty = StructType::new(store.engine(), [])?;
    let pre = StructRefPre::new(&mut store, struct_ty);
    let s = {
        let mut scope = RootScope::new(&mut store);
        StructRef::new(&mut scope, &pre, &[])?
    };
    // The root scope ended and `s` is unrooted.
    assert!(s.ty(&mut store).is_err());
    Ok(())
}

#[test]
fn struct_fields_empty() -> Result<()> {
    let mut store = gc_store()?;
    let struct_ty = StructType::new(store.engine(), [])?;
    let pre = StructRefPre::new(&mut store, struct_ty.clone());
    let s = StructRef::new(&mut store, &pre, &[])?;
    let fields = s.fields(&mut store)?;
    assert_eq!(fields.len(), 0);
    assert!(fields.collect::<Vec<_>>().is_empty());
    Ok(())
}

#[test]
fn struct_fields_non_empty() -> Result<()> {
    let mut store = gc_store()?;
    let struct_ty = StructType::new(
        store.engine(),
        [
            FieldType::new(Mutability::Const, StorageType::I8),
            FieldType::new(Mutability::Var, StorageType::ValType(ValType::ANYREF)),
        ],
    )?;
    let pre = StructRefPre::new(&mut store, struct_ty.clone());
    let s = StructRef::new(&mut store, &pre, &[Val::I32(36), Val::null_any_ref()])?;
    let mut fields = s.fields(&mut store)?;
    assert_eq!(fields.len(), 2);
    assert_eq!(fields.next().unwrap().unwrap_i32(), 36);
    assert!(fields.next().unwrap().unwrap_any_ref().is_none());
    assert!(fields.next().is_none());
    Ok(())
}

#[test]
fn struct_fields_unrooted() -> Result<()> {
    let mut store = gc_store()?;
    let struct_ty = StructType::new(store.engine(), [])?;
    let pre = StructRefPre::new(&mut store, struct_ty);
    let s = {
        let mut scope = RootScope::new(&mut store);
        StructRef::new(&mut scope, &pre, &[])?
    };
    // The root scope ended and `s` is unrooted.
    assert!(s.fields(&mut store).is_err());
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn passing_structs_through_wasm_with_untyped_calls() -> Result<()> {
    let mut store = gc_store()?;

    let module = Module::new(
        store.engine(),
        r#"
            (module
                (type (struct (field i8)))
                (import "" "" (func $f (param (ref 0)) (result (ref 0))))
                (func (export "run") (param (ref 0)) (result (ref 0))
                    (call $f (local.get 0))
                )
            )
        "#,
    )?;

    let struct_ty = StructType::new(
        store.engine(),
        [FieldType::new(Mutability::Const, StorageType::I8)],
    )?;

    let ref_ty = RefType::new(false, HeapType::ConcreteStruct(struct_ty.clone()));
    let func_ty = FuncType::new(store.engine(), [ref_ty.clone().into()], [ref_ty.into()]);

    let func = Func::new(&mut store, func_ty, |mut caller, args, results| {
        let s = args[0].unwrap_any_ref().unwrap();
        let s = s.unwrap_struct(&mut caller)?;
        assert_eq!(s.field(&mut caller, 0)?.unwrap_i32(), 42);
        results[0] = args[0].clone();
        Ok(())
    });

    let instance = Instance::new(&mut store, &module, &[func.into()])?;
    let run = instance.get_func(&mut store, "run").unwrap();

    let pre = StructRefPre::new(&mut store, struct_ty.clone());
    let s = StructRef::new(&mut store, &pre, &[Val::I32(42)])?;

    let mut results = vec![Val::null_any_ref()];
    run.call(&mut store, &[s.into()], &mut results)?;

    let t = results[0].unwrap_any_ref().unwrap();
    let t = t.unwrap_struct(&mut store)?;
    assert_eq!(t.field(&mut store, 0)?.unwrap_i32(), 42);
    assert!(Rooted::ref_eq(&store, &s, &t)?);

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn passing_structs_through_wasm_with_typed_calls() -> Result<()> {
    let mut store = gc_store()?;

    let module = Module::new(
        store.engine(),
        r#"
            (module
                (type (struct (field i8)))
                (import "" "" (func $f (param (ref struct)) (result (ref struct))))
                (func (export "run") (param (ref 0)) (result (ref struct))
                    (call $f (local.get 0))
                )
            )
        "#,
    )?;

    let struct_ty = StructType::new(
        store.engine(),
        [FieldType::new(Mutability::Const, StorageType::I8)],
    )?;

    let func = Func::wrap(
        &mut store,
        |mut caller: Caller<()>, s: Rooted<StructRef>| -> Result<Rooted<StructRef>> {
            assert_eq!(s.field(&mut caller, 0)?.unwrap_i32(), 42);
            Ok(s)
        },
    );

    let instance = Instance::new(&mut store, &module, &[func.into()])?;
    let run = instance.get_typed_func::<Rooted<StructRef>, Rooted<StructRef>>(&mut store, "run")?;

    let pre = StructRefPre::new(&mut store, struct_ty.clone());
    let s = StructRef::new(&mut store, &pre, &[Val::I32(42)])?;

    let t = run.call(&mut store, s)?;

    assert_eq!(t.field(&mut store, 0)?.unwrap_i32(), 42);
    assert!(Rooted::ref_eq(&store, &s, &t)?);

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn host_sets_struct_global() -> Result<()> {
    let mut store = gc_store()?;

    let module = Module::new(
        store.engine(),
        r#"
            (module
                (type (struct (field i8)))
                (global $g (export "g") (mut (ref null 0)) (ref.null 0))
                (func (export "f") (result (ref null 0))
                    global.get $g
                )
            )
        "#,
    )?;

    let instance = Instance::new(&mut store, &module, &[])?;
    let g = instance.get_global(&mut store, "g").unwrap();

    let struct_ty = StructType::new(
        store.engine(),
        [FieldType::new(Mutability::Const, StorageType::I8)],
    )?;
    let pre = StructRefPre::new(&mut store, struct_ty.clone());
    let s0 = StructRef::new(&mut store, &pre, &[Val::I32(42)])?;
    g.set(&mut store, s0.into())?;

    // Get the global from the host.
    let val = g.get(&mut store);
    let anyref = val.unwrap_anyref().expect("non-null");
    let s1 = anyref.unwrap_struct(&store)?;
    assert_eq!(s1.field(&mut store, 0)?.unwrap_i32(), 42);
    assert!(Rooted::ref_eq(&store, &s0, &s1)?);

    // Get the global from the guest.
    let f = instance.get_typed_func::<(), Option<Rooted<StructRef>>>(&mut store, "f")?;
    let s2 = f.call(&mut store, ())?.expect("non-null");
    assert_eq!(s2.field(&mut store, 0)?.unwrap_i32(), 42);
    assert!(Rooted::ref_eq(&store, &s0, &s2)?);

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn wasm_sets_struct_global() -> Result<()> {
    let mut store = gc_store()?;

    let module = Module::new(
        store.engine(),
        r#"
            (module
                (type (struct (field i8)))
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

    let struct_ty = StructType::new(
        store.engine(),
        [FieldType::new(Mutability::Const, StorageType::I8)],
    )?;
    let pre = StructRefPre::new(&mut store, struct_ty.clone());
    let s0 = StructRef::new(&mut store, &pre, &[Val::I32(42)])?;

    let instance = Instance::new(&mut store, &module, &[])?;
    let set = instance.get_func(&mut store, "set").unwrap();
    set.call(&mut store, &[s0.into()], &mut [])?;

    // Get the global from the host.
    let g = instance.get_global(&mut store, "g").unwrap();
    let val = g.get(&mut store);
    let anyref = val.unwrap_anyref().expect("non-null");
    let s1 = anyref.unwrap_struct(&store)?;
    assert_eq!(s1.field(&mut store, 0)?.unwrap_i32(), 42);
    assert!(Rooted::ref_eq(&store, &s0, &s1)?);

    // Get the global from the guest.
    let f = instance.get_typed_func::<(), Option<Rooted<StructRef>>>(&mut store, "get")?;
    let s2 = f.call(&mut store, ())?.expect("non-null");
    assert_eq!(s2.field(&mut store, 0)?.unwrap_i32(), 42);
    assert!(Rooted::ref_eq(&store, &s0, &s2)?);

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn host_sets_struct_in_table() -> Result<()> {
    let mut store = gc_store()?;

    let module = Module::new(
        store.engine(),
        r#"
            (module
                (type (struct (field i8)))
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

    let struct_ty = StructType::new(
        store.engine(),
        [FieldType::new(Mutability::Const, StorageType::I8)],
    )?;
    let pre = StructRefPre::new(&mut store, struct_ty.clone());
    let s0 = StructRef::new(&mut store, &pre, &[Val::I32(42)])?;
    t.set(&mut store, 0, s0.into())?;

    // Get the global from the host.
    let val = t.get(&mut store, 0).expect("in bounds");
    let anyref = val.unwrap_any().expect("non-null");
    let s1 = anyref.unwrap_struct(&store)?;
    assert_eq!(s1.field(&mut store, 0)?.unwrap_i32(), 42);
    assert!(Rooted::ref_eq(&store, &s0, &s1)?);

    // Get the global from the guest.
    let f = instance.get_typed_func::<(), Option<Rooted<StructRef>>>(&mut store, "f")?;
    let s2 = f.call(&mut store, ())?.expect("non-null");
    assert_eq!(s2.field(&mut store, 0)?.unwrap_i32(), 42);
    assert!(Rooted::ref_eq(&store, &s0, &s2)?);

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn wasm_sets_struct_in_table() -> Result<()> {
    let mut store = gc_store()?;

    let module = Module::new(
        store.engine(),
        r#"
            (module
                (type (struct (field i8)))
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

    let struct_ty = StructType::new(
        store.engine(),
        [FieldType::new(Mutability::Const, StorageType::I8)],
    )?;
    let pre = StructRefPre::new(&mut store, struct_ty.clone());
    let s0 = StructRef::new(&mut store, &pre, &[Val::I32(42)])?;

    let instance = Instance::new(&mut store, &module, &[])?;
    let set = instance.get_func(&mut store, "set").unwrap();
    set.call(&mut store, &[s0.into()], &mut [])?;

    // Get the global from the host.
    let t = instance.get_table(&mut store, "t").unwrap();
    let val = t.get(&mut store, 0).expect("in bounds");
    let anyref = val.unwrap_any().expect("non-null");
    let s1 = anyref.unwrap_struct(&store)?;
    assert_eq!(s1.field(&mut store, 0)?.unwrap_i32(), 42);
    assert!(Rooted::ref_eq(&store, &s0, &s1)?);

    // Get the global from the guest.
    let f = instance.get_typed_func::<(), Option<Rooted<StructRef>>>(&mut store, "get")?;
    let s2 = f.call(&mut store, ())?.expect("non-null");
    assert_eq!(s2.field(&mut store, 0)?.unwrap_i32(), 42);
    assert!(Rooted::ref_eq(&store, &s0, &s2)?);

    Ok(())
}

#[test]
fn instantiate_with_struct_global() -> Result<()> {
    let mut store = gc_store()?;

    let module = Module::new(
        store.engine(),
        r#"
            (module
                (type (struct (field i8)))
                (import "" "" (global (ref null 0)))
                (export "g" (global 0))
            )
        "#,
    )?;

    let struct_ty = StructType::new(
        store.engine(),
        [FieldType::new(Mutability::Const, StorageType::I8)],
    )?;
    let global_ty = GlobalType::new(
        ValType::Ref(RefType::new(
            true,
            HeapType::ConcreteStruct(struct_ty.clone()),
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
    let pre = StructRefPre::new(&mut store, struct_ty);
    let s0 = StructRef::new(&mut store, &pre, &[Val::I32(42)])?;
    let g = Global::new(&mut store, global_ty, s0.into())?;
    let instance = Instance::new(&mut store, &module, &[g.into()])?;
    let g = instance.get_global(&mut store, "g").expect("export exists");
    let val = g.get(&mut store);
    let anyref = val.unwrap_anyref().expect("non-null");
    let s1 = anyref.unwrap_struct(&store)?;
    assert_eq!(s1.field(&mut store, 0)?.unwrap_i32(), 42);
    assert!(Rooted::ref_eq(&store, &s0, &s1)?);

    Ok(())
}
