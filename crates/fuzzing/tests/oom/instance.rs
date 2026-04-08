#![cfg(arc_try_new)]

use wasmtime::{Config, Engine, Func, Instance, Linker, Module, Result, Store};
use wasmtime_fuzzing::oom::OomTest;

#[test]
fn call_exported_func() -> Result<()> {
    let module_bytes = {
        let mut config = Config::new();
        config.concurrency_support(false);
        let engine = Engine::new(&config)?;
        let module = Module::new(
            &engine,
            r#"
                (module
                    (func (export "add") (param i32 i32) (result i32)
                        (i32.add (local.get 0) (local.get 1))
                    )
                )
            "#,
        )?;
        module.serialize()?
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
        let add = instance.get_typed_func::<(i32, i32), i32>(&mut store, "add")?;
        let result = add.call(&mut store, (1, 2))?;
        assert_eq!(result, 3);
        Ok(())
    })
}

#[test]
fn instance_new() -> Result<()> {
    let module_bytes = {
        let mut config = Config::new();
        config.concurrency_support(false);
        let engine = Engine::new(&config)?;
        Module::new(
            &engine,
            "(module (import \"\" \"f\" (func)) (func (export \"g\") (call 0)))",
        )?
        .serialize()?
    };
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let module = unsafe { Module::deserialize(&engine, &module_bytes)? };

    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let func = Func::try_wrap(&mut store, || {})?;
        let _instance = Instance::new(&mut store, &module, &[func.into()])?;
        Ok(())
    })
}

#[tokio::test]
async fn instance_new_async() -> Result<()> {
    let module_bytes = {
        let mut config = Config::new();
        config.concurrency_support(false);
        let engine = Engine::new(&config)?;
        Module::new(
            &engine,
            "(module (import \"\" \"f\" (func)) (func (export \"g\") (call 0)))",
        )?
        .serialize()?
    };
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let module = unsafe { Module::deserialize(&engine, &module_bytes)? };

    OomTest::new()
        .test_async(|| async {
            let mut store = Store::try_new(&engine, ())?;
            let func = Func::try_wrap(&mut store, || {})?;
            let _instance = Instance::new_async(&mut store, &module, &[func.into()]).await?;
            Ok(())
        })
        .await
}

#[test]
fn instance_get_export() -> Result<()> {
    let module_bytes = {
        let mut config = Config::new();
        config.concurrency_support(false);
        let engine = Engine::new(&config)?;
        Module::new(&engine, "(module (func (export \"f\")))")?.serialize()?
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
        let _export = instance.get_export(&mut store, "f");
        Ok(())
    })
}

#[test]
fn instance_exports() -> Result<()> {
    let module_bytes = {
        let mut config = Config::new();
        config.concurrency_support(false);
        let engine = Engine::new(&config)?;
        Module::new(
            &engine,
            "(module (func (export \"f\")) (memory (export \"m\") 1) (global (export \"g\") i32 (i32.const 0)))",
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
        let count = instance.exports(&mut store).count();
        assert_eq!(count, 3);
        Ok(())
    })
}

#[test]
fn instance_get_func() -> Result<()> {
    let module_bytes = {
        let mut config = Config::new();
        config.concurrency_support(false);
        let engine = Engine::new(&config)?;
        Module::new(
            &engine,
            "(module (func (export \"f\") (param i32) (result i32) (local.get 0)))",
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
        let f = instance.get_func(&mut store, "f");
        assert!(f.is_some());
        Ok(())
    })
}

#[test]
fn instance_get_typed_func() -> Result<()> {
    let module_bytes = {
        let mut config = Config::new();
        config.concurrency_support(false);
        let engine = Engine::new(&config)?;
        Module::new(
            &engine,
            "(module (func (export \"f\") (param i32) (result i32) (local.get 0)))",
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
        let _f = instance.get_typed_func::<i32, i32>(&mut store, "f")?;
        Ok(())
    })
}

#[test]
fn instance_get_table() -> Result<()> {
    let module_bytes = {
        let mut config = Config::new();
        config.concurrency_support(false);
        let engine = Engine::new(&config)?;
        Module::new(&engine, "(module (table (export \"t\") 1 funcref))")?.serialize()?
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
        let t = instance.get_table(&mut store, "t");
        assert!(t.is_some());
        Ok(())
    })
}

#[test]
fn instance_get_memory() -> Result<()> {
    let module_bytes = {
        let mut config = Config::new();
        config.concurrency_support(false);
        let engine = Engine::new(&config)?;
        Module::new(&engine, "(module (memory (export \"m\") 1))")?.serialize()?
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
        let m = instance.get_memory(&mut store, "m");
        assert!(m.is_some());
        Ok(())
    })
}

#[test]
fn instance_get_global() -> Result<()> {
    let module_bytes = {
        let mut config = Config::new();
        config.concurrency_support(false);
        let engine = Engine::new(&config)?;
        Module::new(
            &engine,
            "(module (global (export \"g\") i32 (i32.const 42)))",
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
        let g = instance.get_global(&mut store, "g");
        assert!(g.is_some());
        Ok(())
    })
}
