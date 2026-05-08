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

#[test]
fn host_exnref_has_trace_info_for_gc() -> Result<()> {
    for collector in [Collector::Copying, Collector::DeferredReferenceCounting] {
        println!("Using GC collector: {collector:?}");

        let mut config = Config::new();
        config.wasm_exceptions(true).wasm_gc(true);
        config.collector(collector);

        let engine = Engine::new(&config)?;

        // Create a store and allocate the GC store by instantiating a module
        let mut store = Store::new(&engine, ());
        let module = Module::new(
            &engine,
            r#"(module
              (global (export "g") (mut exnref) (ref.null exn))
              (table 1 anyref)
            )"#,
        )?;
        let instance = Instance::new(&mut store, &module, &[])?;
        let g = instance.get_global(&mut store, "g").unwrap();

        // Allocate a host exnref object in a nested scope and put it into the
        // module that was just allocated.
        let exn_ty = ExnType::new(&engine, [ValType::I32])?;
        let exnpre = ExnRefPre::new(&mut store, exn_ty.clone());
        let tag = Tag::new(&mut store, &exn_ty.tag_type())?;
        {
            let mut scope = RootScope::new(&mut store);
            let exn = ExnRef::new(&mut scope, &exnpre, &tag, &[Val::I32(43)])?;
            g.set(&mut scope, Val::ExnRef(Some(exn)))?;
        }

        // The exnref should stay alive and be traced properly...
        store.gc(None)?;

        assert!(matches!(g.get(&mut store), Val::ExnRef(Some(_))));
    }

    Ok(())
}
