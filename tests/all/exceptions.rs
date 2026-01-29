use wasmtime::*;
use wasmtime_test_macros::wasmtime_test;

#[wasmtime_test(wasm_features(exceptions))]
#[cfg_attr(miri, ignore)]
fn basic_throw(config: &mut Config) -> Result<()> {
    let engine = Engine::new(config)?;
    let mut store = Store::new(&engine, ());

    let module = Module::new(
        &engine,
        r#"
        (module
          (tag $e0 (param i32 i64))

          (func $throw (param i32 i64)
                (throw $e0 (local.get 0) (local.get 1)))

          (func $catch (export "catch") (param i32 i64) (result i32 i64)

                (block $b (result i32 i64)
                       (try_table (result i32 i64)
                                  (catch $e0 $b)
                                  (call $throw (local.get 0) (local.get 1))
                                  (i32.const 42)
                                  (i64.const 100)))))
          "#,
    )?;

    let instance = Instance::new(&mut store, &module, &[])?;
    let func = instance.get_func(&mut store, "catch").unwrap();
    let mut results = [Val::I32(0), Val::I64(0)];
    func.call(&mut store, &[Val::I32(1), Val::I64(2)], &mut results[..])?;
    assert!(matches!(results[0], Val::I32(1)));
    assert!(matches!(results[1], Val::I64(2)));

    Ok(())
}

#[wasmtime_test(wasm_features(exceptions))]
#[cfg_attr(miri, ignore)]
fn dynamic_tags(config: &mut Config) -> Result<()> {
    let engine = Engine::new(config)?;
    let mut store = Store::new(&engine, ());

    let module = Module::new(
        &engine,
        r#"
        (module
          (import "test" "e0" (tag $e0 (param i32 i64)))
          (import "test" "e1" (tag $e1 (param i32 i64)))

          (func $throw_e1 (param i32 i64)
                (throw $e1 (local.get 0) (local.get 1)))

          (func $catch (export "catch") (param i32 i64) (result i32 i64 i32)
                (block $b1 (result i32 i64)
                 (block $b0 (result i32 i64)
                        (try_table (result i32 i64)
                                   (catch $e0 $b0)
                                   (catch $e1 $b1)
                                   (call $throw_e1 (local.get 0) (local.get 1))
                                   (unreachable)))
                 (i32.const 0)
                 (return))
                (i32.const 1)
                (return)))
          "#,
    )?;

    let functy = FuncType::new(&engine, [ValType::I32, ValType::I64], []);
    let tagty = TagType::new(functy);
    let tag0 = Tag::new(&mut store, &tagty)?;
    let tag1 = Tag::new(&mut store, &tagty)?;

    // Instantiate with two different tags -- second catch-clause
    // should match (on $e1).
    let instance1 = Instance::new(&mut store, &module, &[Extern::Tag(tag0), Extern::Tag(tag1)])?;
    let func1 = instance1.get_func(&mut store, "catch").unwrap();
    let mut results = [Val::I32(0), Val::I64(0), Val::I32(0)];
    func1.call(&mut store, &[Val::I32(1), Val::I64(2)], &mut results[..])?;
    assert!(matches!(results[0], Val::I32(1)));
    assert!(matches!(results[1], Val::I64(2)));
    assert!(matches!(results[2], Val::I32(1)));

    // Instantiate with two imports of the same tag -- now first
    // catch-clause should match (on $e0, since $e0 is an alias to
    // $e1).
    let instance2 = Instance::new(&mut store, &module, &[Extern::Tag(tag0), Extern::Tag(tag0)])?;
    let func2 = instance2.get_func(&mut store, "catch").unwrap();
    let mut results = [Val::I32(0), Val::I64(0), Val::I32(0)];
    func2.call(&mut store, &[Val::I32(1), Val::I64(2)], &mut results[..])?;
    assert!(matches!(results[0], Val::I32(1)));
    assert!(matches!(results[1], Val::I64(2)));
    assert!(matches!(results[2], Val::I32(0)));

    Ok(())
}

#[wasmtime_test(wasm_features(exceptions))]
#[cfg_attr(miri, ignore)]
fn exception_escape_to_host(config: &mut Config) -> Result<()> {
    let engine = Engine::new(config)?;
    let mut store = Store::new(&engine, ());

    let module = Module::new(
        &engine,
        r#"
        (module
          (import "test" "e0" (tag $e0 (param i32)))

          (func $throw (export "throw")
                (throw $e0 (i32.const 42))))
          "#,
    )?;

    let functy = FuncType::new(&engine, [ValType::I32], []);
    let tagty = TagType::new(functy);
    let tag = Tag::new(&mut store, &tagty)?;
    let instance = Instance::new(&mut store, &module, &[Extern::Tag(tag)])?;
    let func = instance.get_func(&mut store, "throw").unwrap();
    let mut results = [];
    let result = func.call(&mut store, &[], &mut results[..]);
    assert!(result.is_err());
    assert!(result.unwrap_err().is::<ThrownException>());
    let exn = store.take_pending_exception().unwrap();
    let exntag = exn.tag(&mut store)?;
    assert!(Tag::eq(&exntag, &tag, &store));

    Ok(())
}

#[wasmtime_test(wasm_features(exceptions))]
#[cfg_attr(miri, ignore)]
fn exception_from_host(config: &mut Config) -> Result<()> {
    let engine = Engine::new(config)?;
    let mut store = Store::new(&engine, ());

    let module = Module::new(
        &engine,
        r#"
        (module
          (import "test" "e0" (tag $e0 (param i32)))
          (import "test" "f" (func $f (param i32)))

          (func $catch (export "catch") (result i32)
                (block $b (result i32)
                  (try_table (result i32) (catch $e0 $b)
                   i32.const 42
                   call $f
                   i32.const 0))))
          "#,
    )?;

    let functy = FuncType::new(&engine, [ValType::I32], []);
    let tagty = TagType::new(functy.clone());
    let exnty = ExnType::from_tag_type(&tagty).unwrap();
    let exnpre = ExnRefPre::new(&mut store, exnty);
    let tag = Tag::new(&mut store, &tagty)?;
    let extfunc = Func::new(&mut store, functy, move |mut caller, args, _rets| {
        let exn = ExnRef::new(
            &mut caller,
            &exnpre,
            &tag,
            &[Val::I32(args[0].unwrap_i32())],
        )
        .unwrap();
        caller.as_context_mut().throw(exn)?;
        Ok(())
    });
    let instance = Instance::new(
        &mut store,
        &module,
        &[Extern::Tag(tag), Extern::Func(extfunc)],
    )?;
    let func = instance.get_func(&mut store, "catch").unwrap();
    let mut results = [Val::null_any_ref()];
    func.call(&mut store, &[], &mut results[..])?;
    assert_eq!(results[0].unwrap_i32(), 42);

    Ok(())
}

#[wasmtime_test(wasm_features(exceptions))]
fn exception_across_no_wasm(config: &mut Config) -> Result<()> {
    let engine = Engine::new(config)?;
    let mut store = Store::new(&engine, ());

    let functy = FuncType::new(&engine, [ValType::I32], []);
    let tagty = TagType::new(functy.clone());
    let exnty = ExnType::from_tag_type(&tagty).unwrap();
    let exnpre = ExnRefPre::new(&mut store, exnty);
    let tag = Tag::new(&mut store, &tagty)?;
    let extfunc = Func::new(&mut store, functy, move |mut caller, args, _rets| {
        let exn = ExnRef::new(
            &mut caller,
            &exnpre,
            &tag,
            &[Val::I32(args[0].unwrap_i32())],
        )
        .unwrap();
        caller.as_context_mut().throw(exn)?;
        Ok(())
    });
    let mut results = [];
    let result = extfunc.call(&mut store, &[Val::I32(42)], &mut results[..]);
    assert!(result.is_err() && result.unwrap_err().downcast::<ThrownException>().is_ok());
    let exn = store.take_pending_exception().unwrap();
    let exntag = exn.tag(&mut store)?;
    assert!(Tag::eq(&exntag, &tag, &store));
    assert_eq!(exn.field(&mut store, 0)?.unwrap_i32(), 42);

    Ok(())
}

#[wasmtime_test(wasm_features(gc, exceptions))]
fn gc_with_exnref_global(config: &mut Config) -> Result<()> {
    let engine = Engine::new(config)?;
    let mut store = Store::new(&engine, ());

    let module = Module::new(
        &engine,
        r#"
        (module
          (global (export "g") (mut exnref) (ref.null exn)))
          "#,
    )?;

    let instance = Instance::new(&mut store, &module, &[])?;

    let functy = FuncType::new(&engine, [], []);
    let tagty = TagType::new(functy.clone());
    let exnty = ExnType::from_tag_type(&tagty).unwrap();
    let exnpre = ExnRefPre::new(&mut store, exnty);
    let tag = Tag::new(&mut store, &tagty)?;
    let exn = ExnRef::new(&mut store, &exnpre, &tag, &[])?;

    let global = instance.get_global(&mut store, "g").unwrap();
    global.set(&mut store, Val::ExnRef(Some(exn)))?;

    store.gc(None)?;

    Ok(())
}
