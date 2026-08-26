use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
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
fn try_table_fallthrough_with_multi_value_results(config: &mut Config) -> Result<()> {
    let engine = Engine::new(config)?;
    let mut store = Store::new(&engine, ());

    let module = Module::new(
        &engine,
        r#"
        (module
          (tag $e (param i32))
          (func $callee (result i32) (i32.const 9))
          (func (export "f") (result i32)
            (block $h (result i32)
              (try_table (result i32 i32 i32 i32 i32 i32) (catch $e $h)
                (i32.const 1)
                (i32.const 2)
                (i32.const 3)
                (i32.const 4)
                (i32.const 5)
                (call $callee))
              drop drop drop drop drop)))
        "#,
    )?;

    let instance = Instance::new(&mut store, &module, &[])?;
    let f = instance.get_typed_func::<(), i32>(&mut store, "f")?;
    assert_eq!(f.call(&mut store, ())?, 1);
    Ok(())
}

#[wasmtime_test(wasm_features(exceptions))]
#[cfg_attr(miri, ignore)]
fn try_table_exception_with_multi_value_payload(config: &mut Config) -> Result<()> {
    let engine = Engine::new(config)?;
    let mut store = Store::new(&engine, ());

    let module = Module::new(
        &engine,
        r#"
        (module
          (tag $e (param i32 i32 i32 i32 i32 i32))

          (func $throw (result i32 i32 i32 i32 i32 i32)
            (throw $e
              (i32.const 1)
              (i32.const 2)
              (i32.const 3)
              (i32.const 4)
              (i32.const 5)
              (i32.const 6)))

          (func (export "f") (result i32)
            (block $handler (result i32 i32 i32 i32 i32 i32)
              (try_table
                (result i32 i32 i32 i32 i32 i32)
                (catch $e $handler)
                (call $throw)))
            drop drop drop drop drop))
        "#,
    )?;

    let instance = Instance::new(&mut store, &module, &[])?;
    let f = instance.get_typed_func::<(), i32>(&mut store, "f")?;
    assert_eq!(f.call(&mut store, ())?, 1);
    Ok(())
}

#[wasmtime_test(wasm_features(exceptions))]
#[cfg_attr(miri, ignore)]
fn try_table_branch_and_fallthrough_with_multi_value_results(config: &mut Config) -> Result<()> {
    let engine = Engine::new(config)?;
    let mut store = Store::new(&engine, ());

    let module = Module::new(
        &engine,
        r#"
        (module
          (tag $e (param i32))

          (func (export "f") (param $branch i32) (result i32)
            (block $handler (result i32)
              (try_table
                (result i32 i32 i32 i32 i32 i32)
                (catch $e $handler)
                (i32.const 1)
                (i32.const 2)
                (i32.const 3)
                (i32.const 4)
                (i32.const 5)
                (i32.const 6)
                (local.get $branch)
                br_if 0)
              drop drop drop drop drop)))
        "#,
    )?;

    let instance = Instance::new(&mut store, &module, &[])?;
    let f = instance.get_typed_func::<i32, i32>(&mut store, "f")?;
    assert_eq!(f.call(&mut store, 0)?, 1);
    assert_eq!(f.call(&mut store, 1)?, 1);
    Ok(())
}

#[wasmtime_test(wasm_features(exceptions))]
#[cfg_attr(miri, ignore)]
fn try_table_unreachable_fallthrough_with_multi_value_results(config: &mut Config) -> Result<()> {
    let engine = Engine::new(config)?;
    let mut store = Store::new(&engine, ());

    let module = Module::new(
        &engine,
        r#"
        (module
          (tag $e (param i32))
          (func $callee)

          (func (export "f") (result i32)
            (block $handler (result i32)
              (try_table
                (result i32 i32 i32 i32 i32 i32)
                (catch $e $handler)
                (call $callee)
                (i32.const 1)
                (i32.const 2)
                (i32.const 3)
                (i32.const 4)
                (i32.const 5)
                (i32.const 6)
                br 0)
              drop drop drop drop drop)))
        "#,
    )?;

    let instance = Instance::new(&mut store, &module, &[])?;
    let f = instance.get_typed_func::<(), i32>(&mut store, "f")?;
    assert_eq!(f.call(&mut store, ())?, 1);
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
fn nested_handler_scopes(config: &mut Config) -> Result<()> {
    let engine = Engine::new(config)?;
    let mut store = Store::new(&engine, ());

    let module = Module::new(
        &engine,
        r#"
        (module
          (tag $outer)
          (tag $inner)

          (func $throw_outer
            (throw $outer))

          (func $throw_inner
            (throw $inner))

          ;; While both handlers are active, the inner tag does not match and
          ;; lookup continues to the outer handler.
          (func (export "nested") (result i32)
            (block $outer_handler
              (try_table (catch $outer $outer_handler)
                (block $inner_handler
                  (try_table (catch $inner $inner_handler)
                    (call $throw_outer)))))
            (i32.const 1))

          ;; After the inner try_table ends, its handler is no longer active.
          ;; The outer catch_all handles the throw instead.
          (func (export "after") (result i32)
            (block $stale_inner_handler
              (block $outer_handler
                (try_table (catch_all $outer_handler)
                  (try_table (catch $inner $stale_inner_handler)
                    (nop))
                  (call $throw_inner)))
              (return (i32.const 1)))
            (i32.const 2)))
        "#,
    )?;

    let instance = Instance::new(&mut store, &module, &[])?;
    let nested = instance.get_typed_func::<(), i32>(&mut store, "nested")?;
    let after = instance.get_typed_func::<(), i32>(&mut store, "after")?;
    assert_eq!(nested.call(&mut store, ())?, 1);
    assert_eq!(after.call(&mut store, ())?, 1);

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
fn defined_exception_escape_to_host(config: &mut Config) -> Result<()> {
    let engine = Engine::new(config)?;
    let mut store = Store::new(&engine, ());

    let module = Module::new(
        &engine,
        r#"
        (module
          (tag $e (export "e") (param i64))
          (func (export "throw")
                (throw $e (i64.const 84))))
          "#,
    )?;

    let instance = Instance::new(&mut store, &module, &[])?;
    let tag = instance.get_tag(&mut store, "e").unwrap();
    let func = instance.get_func(&mut store, "throw").unwrap();
    let result = func.call(&mut store, &[], &mut []);
    assert!(result.unwrap_err().is::<ThrownException>());
    let exn = store.take_pending_exception().unwrap();
    let exntag = exn.tag(&mut store)?;
    assert!(Tag::eq(&exntag, &tag, &store));
    assert_eq!(exn.field(&mut store, 0)?.unwrap_i64(), 84);

    Ok(())
}

#[wasmtime_test(wasm_features(exceptions, reference_types))]
#[cfg_attr(miri, ignore)]
fn funcref_exception_payload_escape_to_host(config: &mut Config) -> Result<()> {
    let engine = Engine::new(config)?;
    let mut store = Store::new(&engine, ());

    let module = Module::new(
        &engine,
        r#"
        (module
          (tag $e (param funcref))
          (func (export "throw") (param funcref)
                (throw $e (local.get 0))))
          "#,
    )?;

    let instance = Instance::new(&mut store, &module, &[])?;
    let func = instance.get_func(&mut store, "throw").unwrap();
    let expected = Func::wrap(&mut store, || 126_i32);
    let result = func.call(&mut store, &[Val::FuncRef(Some(expected))], &mut []);
    assert!(result.unwrap_err().is::<ThrownException>());
    let exn = store.take_pending_exception().unwrap();
    let payload = exn.field(&mut store, 0)?;
    let payload = payload.unwrap_funcref().unwrap();
    let payload = payload.typed::<(), i32>(&store)?;
    assert_eq!(payload.call(&mut store, ())?, 126);

    Ok(())
}

#[wasmtime_test(collectors(All), wasm_features(exceptions, reference_types))]
#[cfg_attr(miri, ignore)]
fn caught_funcref_payload(config: &mut Config) -> Result<()> {
    let engine = Engine::new(config)?;
    let mut store = Store::new(&engine, ());

    let module = Module::new(
        &engine,
        r#"
        (module
          (tag $e (param funcref i32))

          (func $throw (param funcref)
            (throw $e (local.get 0) (i32.const 42)))

          (func (export "catch") (param funcref) (result funcref i32)
            (block $handler (result funcref i32)
              (try_table (result funcref i32) (catch $e $handler)
                (call $throw (local.get 0))
                (ref.null func)
                (i32.const 0)))))
        "#,
    )?;

    let instance = Instance::new(&mut store, &module, &[])?;
    let catch = instance.get_func(&mut store, "catch").unwrap();
    let expected = Func::wrap(&mut store, || 126_i32);
    let mut results = [Val::null_func_ref(), Val::I32(0)];
    catch.call(&mut store, &[Val::FuncRef(Some(expected))], &mut results)?;

    let actual = results[0].unwrap_funcref().unwrap();
    let actual = actual.typed::<(), i32>(&store)?;
    assert_eq!(actual.call(&mut store, ())?, 126);
    assert_eq!(results[1].unwrap_i32(), 42);

    Ok(())
}

#[wasmtime_test(
    collectors(Copying, DeferredReferenceCounting),
    wasm_features(exceptions, reference_types)
)]
#[cfg_attr(miri, ignore)]
fn thrown_externref_payload_survives_gc(config: &mut Config) -> Result<()> {
    let engine = Engine::new(config)?;
    let mut store = Store::new(&engine, ());

    let module = Module::new(
        &engine,
        r#"
        (module
          (tag $e (param externref))
          (func (export "throw") (param externref)
            (throw $e (local.get 0))))
        "#,
    )?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let func = instance.get_func(&mut store, "throw").unwrap();
    let dropped = Arc::new(AtomicBool::new(false));

    {
        let mut scope = RootScope::new(&mut store);
        let payload = ExternRef::new(&mut scope, SetFlagOnDrop(dropped.clone()))?;
        let result = func.call(&mut scope, &[Val::ExternRef(Some(payload))], &mut []);
        assert!(result.unwrap_err().is::<ThrownException>());
    }

    store.gc(None)?;
    assert!(!dropped.load(Relaxed));

    let exn = store.take_pending_exception().unwrap();
    let payload = exn
        .field(&mut store, 0)?
        .unwrap_externref()
        .copied()
        .unwrap();
    assert!(payload.data(&store)?.is_some());

    Ok(())
}

#[wasmtime_test(collectors(All), wasm_features(exceptions, reference_types))]
#[cfg_attr(miri, ignore)]
fn caught_externref_payload_survives_gc(config: &mut Config) -> Result<()> {
    let engine = Engine::new(config)?;
    let mut store = Store::new(&engine, ());

    let module = Module::new(
        &engine,
        r#"
        (module
          (tag $e (param externref i32))

          (func $throw (param externref)
            (throw $e (local.get 0) (i32.const 42)))

          (func (export "catch") (param externref) (result externref i32)
            (block $handler (result externref i32)
              (try_table (result externref i32) (catch $e $handler)
                (call $throw (local.get 0))
                (ref.null extern)
                (i32.const 0)))))
        "#,
    )?;

    let instance = Instance::new(&mut store, &module, &[])?;
    let catch = instance
        .get_typed_func::<Option<Rooted<ExternRef>>, (Option<Rooted<ExternRef>>, i32)>(
            &mut store, "catch",
        )?;
    let dropped = Arc::new(AtomicBool::new(false));

    let caught = {
        let mut scope = RootScope::new(&mut store);
        let payload = ExternRef::new(&mut scope, SetFlagOnDrop(dropped.clone()))?;
        let (caught, value) = catch.call(&mut scope, Some(payload))?;
        assert_eq!(value, 42);
        caught.unwrap().to_owned_rooted(&mut scope)?
    };

    store.gc(None)?;
    assert!(!dropped.load(Relaxed));
    assert!(caught.data(&store)?.is_some());

    Ok(())
}

#[wasmtime_test(collectors(Null), wasm_features(exceptions))]
#[cfg_attr(miri, ignore)]
fn throw_with_null_collector(config: &mut Config) -> Result<()> {
    let engine = Engine::new(config)?;
    let mut store = Store::new(&engine, ());

    let module = Module::new(
        &engine,
        r#"
        (module
          (tag $e (param i32))
          (func (export "throw")
            (throw $e (i32.const 42))))
        "#,
    )?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let func = instance.get_func(&mut store, "throw").unwrap();
    let result = func.call(&mut store, &[], &mut []);
    assert!(result.unwrap_err().is::<ThrownException>());

    let exn = store.take_pending_exception().unwrap();
    assert_eq!(exn.field(&mut store, 0)?.unwrap_i32(), 42);

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
#[cfg_attr(miri, ignore)]
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

#[wasmtime_test(wasm_features(exceptions))]
#[cfg_attr(miri, ignore)]
fn thrown_exception_without_throwing(config: &mut Config) -> Result<()> {
    let engine = Engine::new(config)?;
    let mut store = Store::new(&engine, ());

    let module = Module::new(
        &engine,
        r#"
        (module
            (import "" "" (func))

            (func (export "run") call 0)
        )
        "#,
    )?;

    let func = Func::wrap(&mut store, || -> Result<()> { Err(ThrownException.into()) });
    let instance = Instance::new(&mut store, &module, &[func.into()])?;
    let func = instance.get_func(&mut store, "run").unwrap();
    let err = func.call(&mut store, &[], &mut []).unwrap_err();
    assert!(err.is::<ThrownException>());

    Ok(())
}

#[wasmtime_test(wasm_features(exceptions))]
#[cfg_attr(miri, ignore)]
fn wasm_exceptions_have_backtraces(config: &mut Config) -> Result<()> {
    let engine = Engine::new(config)?;
    let mut store = Store::new(&engine, ());

    let module = Module::new(
        &engine,
        r#"
        (module
            (tag $t0)

            (func (export "run") throw $t0)
        )
        "#,
    )?;

    let instance = Instance::new(&mut store, &module, &[])?;
    let func = instance.get_func(&mut store, "run").unwrap();
    let err = func.call(&mut store, &[], &mut []).unwrap_err();
    assert!(err.is::<ThrownException>());
    assert!(err.is::<WasmBacktrace>());

    Ok(())
}

#[wasmtime_test(collectors(DeferredReferenceCounting), wasm_features(exceptions))]
#[cfg_attr(miri, ignore)]
fn store_pending_exnref_is_cloned(config: &mut Config) -> wasmtime::Result<()> {
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());

    let module = Module::new(
        &engine,
        r#"
        (module
          (import "h" "t1" (tag $t1 (param i32)))
          (import "h" "throw_t1" (func $throw_t1))
          (func (export "run") (result i32)
            (block $h (result i32)
              (try_table (result i32) (catch $t1 $h)
                call $throw_t1
                unreachable
              )
            )
          )
        )
        "#,
    )?;

    let functy = FuncType::new(&engine, [ValType::I32], []);
    let tagty = TagType::new(functy);
    let t1 = Tag::new(&mut store, &tagty)?;
    let exnty = ExnType::from_tag_type(&tagty)?;
    let exnpre_for_t1 = ExnRefPre::new(&mut store, exnty);

    let throw_t1 = Func::wrap(
        &mut store,
        move |mut caller: Caller<'_, ()>| -> Result<()> {
            let err = {
                let mut scope = RootScope::new(&mut caller);
                let exn = ExnRef::new(&mut scope, &exnpre_for_t1, &t1, &[Val::I32(0x1111_1111)])?;
                scope.as_context_mut().throw::<()>(exn)
            };
            caller.as_context_mut().gc(None)?;
            err
        },
    );

    let instance = Instance::new(
        &mut store,
        &module,
        &[Extern::Tag(t1), Extern::Func(throw_t1)],
    )?;
    let run = instance.get_typed_func::<(), i32>(&mut store, "run")?;
    let result = run.call(&mut store, ())?;
    assert_eq!(result, 0x1111_1111);
    Ok(())
}

#[wasmtime_test(collectors(All), wasm_features(exceptions, reference_types))]
#[cfg_attr(miri, ignore)]
fn store_pending_exnref_is_exposed(config: &mut Config) -> wasmtime::Result<()> {
    let engine = Engine::new(config)?;
    let mut store = Store::new(&engine, ());

    let module = Module::new(
        &engine,
        r#"
        (module
          (import "h" "t1" (tag $t1 (param i32)))
          (import "h" "throw_t1" (func $throw_t1))
          (import "" "gc" (func $gc))

          (func (export "run") (result i32 (ref exn))
            (block $h (result i32 (ref exn))
              (try_table (result i32) (catch_ref $t1 $h)
                call $throw_t1
                unreachable
              )
              unreachable
            )
            call $gc
          )

          (func (export "run_all") (result (ref exn))
            (block $h (result (ref exn))
              (try_table (catch_all_ref $h)
                call $throw_t1
                unreachable
              )
              unreachable
            )
            call $gc
          )
        )
        "#,
    )?;

    let functy = FuncType::new(&engine, [ValType::I32], []);
    let tagty = TagType::new(functy);
    let t1 = Tag::new(&mut store, &tagty)?;
    let exnty = ExnType::from_tag_type(&tagty)?;
    let exnpre_for_t1 = ExnRefPre::new(&mut store, exnty);

    let throw_t1 = Func::wrap(
        &mut store,
        move |mut caller: Caller<'_, ()>| -> Result<()> {
            let err = {
                let mut scope = RootScope::new(&mut caller);
                let exn = ExnRef::new(&mut scope, &exnpre_for_t1, &t1, &[Val::I32(0x1111_1111)])?;
                scope.as_context_mut().throw::<()>(exn)
            };
            caller.as_context_mut().gc(None)?;
            err
        },
    );
    let gc = Func::wrap(
        &mut store,
        move |mut caller: Caller<'_, ()>| -> Result<()> {
            caller.gc(None)?;
            Ok(())
        },
    );

    let instance = Instance::new(
        &mut store,
        &module,
        &[t1.into(), throw_t1.into(), gc.into()],
    )?;
    let run = instance.get_typed_func::<(), (i32, Rooted<ExnRef>)>(&mut store, "run")?;
    let (result, exnref) = run.call(&mut store, ())?;
    assert_eq!(result, 0x1111_1111);

    store.gc(None)?;

    assert_eq!(exnref.field(&mut store, 0)?.unwrap_i32(), 0x1111_1111);

    let run_all = instance.get_typed_func::<(), Rooted<ExnRef>>(&mut store, "run_all")?;
    let exnref = run_all.call(&mut store, ())?;

    store.gc(None)?;

    assert_eq!(exnref.field(&mut store, 0)?.unwrap_i32(), 0x1111_1111);
    Ok(())
}

#[wasmtime_test(
    collectors(DeferredReferenceCounting),
    wasm_features(exceptions, reference_types)
)]
#[cfg_attr(miri, ignore)]
fn catch_ref_preserves_externref_payload(config: &mut Config) -> wasmtime::Result<()> {
    let engine = Engine::new(config)?;
    let mut store = Store::new(&engine, ());

    let module = Module::new(
        &engine,
        r#"
        (module
          (import "" "gc" (func $gc))
          (tag $e (param externref i32))

          (func $throw (param externref)
            (throw $e (local.get 0) (i32.const 42)))

          (func (export "catch") (param externref) (result externref i32 (ref exn))
            (block $handler (result externref i32 (ref exn))
              (try_table (catch_ref $e $handler)
                (call $throw (local.get 0)))
              unreachable)
            call $gc)
        )
        "#,
    )?;

    let gc = Func::wrap(&mut store, |mut caller: Caller<'_, ()>| -> Result<()> {
        caller.gc(None)?;
        Ok(())
    });
    let instance = Instance::new(&mut store, &module, &[gc.into()])?;
    let catch = instance
        .get_typed_func::<
            Option<Rooted<ExternRef>>,
            (Option<Rooted<ExternRef>>, i32, Rooted<ExnRef>),
        >(
            &mut store, "catch",
        )?;

    let payload = ExternRef::new(&mut store, 0xDECAFu32)?;
    let (caught, value, exnref) = catch.call(&mut store, Some(payload))?;
    assert_eq!(value, 42);
    let caught = caught.expect("catch_ref should return the payload");
    let caught_data = caught
        .data(&store)?
        .and_then(|data| data.downcast_ref::<u32>().copied());
    assert_eq!(caught_data, Some(0xDECAF));

    let field = exnref.field(&mut store, 0)?;
    let field = field
        .unwrap_externref()
        .expect("the exception payload should not be null");
    let field_data = field
        .data(&store)?
        .and_then(|data| data.downcast_ref::<u32>().copied());
    assert_eq!(field_data, Some(0xDECAF));
    assert_eq!(exnref.field(&mut store, 1)?.unwrap_i32(), 42);
    Ok(())
}

struct SetFlagOnDrop(Arc<AtomicBool>);

impl Drop for SetFlagOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Relaxed);
    }
}

#[wasmtime_test(collectors(DeferredReferenceCounting), wasm_features(exceptions))]
fn store_pending_exnref_has_write_barrier(config: &mut Config) -> wasmtime::Result<()> {
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());

    let functy = FuncType::new(&engine, [ValType::EXTERNREF], []);
    let tagty = TagType::new(functy);
    let tag = Tag::new(&mut store, &tagty)?;
    let exnty = ExnType::from_tag_type(&tagty)?;
    let exnpre = ExnRefPre::new(&mut store, exnty);

    let dropped = Arc::new(AtomicBool::new(false));

    eprintln!("a1");

    {
        let mut scope = RootScope::new(&mut store);
        let r = ExternRef::new(&mut scope, SetFlagOnDrop(dropped.clone()))?;
        let exn1 = ExnRef::new(&mut scope, &exnpre, &tag, &[Val::ExternRef(Some(r))])?;
        let _ = scope.as_context_mut().throw::<()>(exn1);
    }
    eprintln!("a2");

    store.gc(None)?;
    eprintln!("a5");
    assert!(!dropped.load(Relaxed));

    {
        let mut scope = RootScope::new(&mut store);
        let exn2 = ExnRef::new(&mut scope, &exnpre, &tag, &[Val::ExternRef(None)])?;
        let _ = scope.as_context_mut().throw::<()>(exn2);
    }
    eprintln!("a3");

    store.gc(None)?;
    eprintln!("a4");
    assert!(dropped.load(Relaxed));

    Ok(())
}

#[wasmtime_test(wasm_features(exceptions, reference_types))]
#[cfg_attr(miri, ignore)]
fn exnref_local_defaults_to_null(config: &mut Config) -> Result<()> {
    let engine = Engine::new(config)?;
    let module = Module::new(
        &engine,
        r#"
        (module
          (func (export "run") (result i32)
            (local exnref)
            local.get 0
            ref.is_null))
        "#,
    )?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;
    let run = instance.get_typed_func::<(), i32>(&mut store, "run")?;
    assert_eq!(run.call(&mut store, ())?, 1);
    Ok(())
}
