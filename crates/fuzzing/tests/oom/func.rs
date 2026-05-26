#![cfg(arc_try_new)]

use wasmtime::{Config, Engine, Func, FuncType, Linker, Module, Result, Store, Val, ValType};
use wasmtime_fuzzing::oom::OomTest;

#[test]
fn func_new() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let _func = Func::try_wrap(&mut store, |x: i32| x * 2)?;
        Ok(())
    })
}

#[test]
fn func_new_with_type() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let ty = FuncType::try_new(&engine, [ValType::I32], [ValType::I32])?;
        let _func = Func::try_new(&mut store, ty, |_caller, params, results| {
            results[0] = params[0].clone();
            Ok(())
        })?;
        Ok(())
    })
}

#[test]
fn func_call() -> Result<()> {
    let module_bytes = {
        let mut config = Config::new();
        config.concurrency_support(false);
        let engine = Engine::new(&config)?;
        Module::new(
            &engine,
            r#"(module (func (export "id") (param i32) (result i32) (local.get 0)))"#,
        )?
        .serialize()?
    };
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let module = unsafe { Module::deserialize(&engine, &module_bytes)? };
    let linker = Linker::<()>::new(&engine);
    let instance_pre = linker.instantiate_pre(&module)?;

    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let instance = instance_pre.instantiate(&mut store)?;
        let id = instance.get_func(&mut store, "id").unwrap();
        let mut results = [Val::I32(0)];
        id.call(&mut store, &[Val::I32(42)], &mut results)?;
        assert_eq!(results[0].unwrap_i32(), 42);
        Ok(())
    })
}

#[tokio::test]
async fn func_call_async() -> Result<()> {
    let module_bytes = {
        let mut config = Config::new();
        config.concurrency_support(false);
        let engine = Engine::new(&config)?;
        Module::new(
            &engine,
            r#"(module (func (export "id") (param i32) (result i32) (local.get 0)))"#,
        )?
        .serialize()?
    };
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let module = unsafe { Module::deserialize(&engine, &module_bytes)? };
    let linker = Linker::<()>::new(&engine);
    let instance_pre = linker.instantiate_pre(&module)?;

    OomTest::new()
        .test_async(|| async {
            let mut store = Store::try_new(&engine, ())?;
            let instance = instance_pre.instantiate_async(&mut store).await?;
            let id = instance.get_func(&mut store, "id").unwrap();
            let mut results = [Val::I32(0)];
            id.call_async(&mut store, &[Val::I32(42)], &mut results)
                .await?;
            assert_eq!(results[0].unwrap_i32(), 42);
            Ok(())
        })
        .await
}

#[tokio::test]
async fn typed_func_call_async() -> Result<()> {
    let module_bytes = {
        let mut config = Config::new();
        config.concurrency_support(false);
        let engine = Engine::new(&config)?;
        Module::new(
            &engine,
            r#"(module (func (export "id") (param i32) (result i32) (local.get 0)))"#,
        )?
        .serialize()?
    };
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let module = unsafe { Module::deserialize(&engine, &module_bytes)? };
    let linker = Linker::<()>::new(&engine);
    let instance_pre = linker.instantiate_pre(&module)?;

    OomTest::new()
        .test_async(|| async {
            let mut store = Store::try_new(&engine, ())?;
            let instance = instance_pre.instantiate_async(&mut store).await?;
            let id = instance.get_typed_func::<i32, i32>(&mut store, "id")?;
            let result = id.call_async(&mut store, 42).await?;
            assert_eq!(result, 42);
            Ok(())
        })
        .await
}

#[test]
fn func_typed() -> Result<()> {
    let module_bytes = {
        let mut config = Config::new();
        config.concurrency_support(false);
        let engine = Engine::new(&config)?;
        Module::new(
            &engine,
            r#"(module (func (export "id") (param i32) (result i32) (local.get 0)))"#,
        )?
        .serialize()?
    };
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let module = unsafe { Module::deserialize(&engine, &module_bytes)? };
    let linker = Linker::<()>::new(&engine);
    let instance_pre = linker.instantiate_pre(&module)?;

    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let instance = instance_pre.instantiate(&mut store)?;
        let id = instance.get_typed_func::<i32, i32>(&mut store, "id")?;
        let result = id.call(&mut store, 42)?;
        assert_eq!(result, 42);
        Ok(())
    })
}

#[test]
fn func_ty() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let func = Func::try_wrap(&mut store, |x: i32| x * 2)?;
        let ty = func.ty(&store);
        assert_eq!(ty.params().len(), 1);
        assert_eq!(ty.results().len(), 1);
        Ok(())
    })
}
