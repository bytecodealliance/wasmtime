use super::gc_and_exceptions_store;
use wasmtime::*;

#[test]
fn basic_throw() -> Result<()> {
    let mut store = gc_and_exceptions_store()?;
    let engine = store.engine();

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

#[test]
fn dynamic_tags() -> Result<()> {
    let mut store = gc_and_exceptions_store()?;
    let engine = store.engine();

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

#[test]
fn exception_escape_to_host() -> Result<()> {
    let mut store = gc_and_exceptions_store()?;
    let engine = store.engine();

    let module = Module::new(
        &engine,
        r#"
        (module
          (tag $e0)

          (func $throw (export "throw")
                (throw $e0)))
          "#,
    )?;

    let instance = Instance::new(&mut store, &module, &[])?;
    let func = instance.get_func(&mut store, "throw").unwrap();
    let mut results = [];
    let result = func.call(&mut store, &[], &mut results[..]);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().downcast::<Trap>().unwrap(),
        Trap::ExceptionToHost
    );

    Ok(())
}
