#![cfg(arc_try_new)]

use wasmtime::{Config, Engine, Func, FuncType, Module, Result, Store};
use wasmtime_fuzzing::oom::OomTest;

#[test]
fn caller_get_export() -> Result<()> {
    let module_bytes = {
        let mut config = Config::new();
        config.concurrency_support(false);
        let engine = Engine::new(&config)?;
        Module::new(
            &engine,
            r#"(module
                (import "" "f" (func))
                (memory (export "m") 1)
                (func (export "run") (call 0))
            )"#,
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
        let host_func = Func::try_new(
            &mut store,
            FuncType::try_new(&engine, [], [])?,
            |mut caller, _params, _results| {
                let mem = caller.get_export("m");
                assert!(mem.is_some());
                Ok(())
            },
        )?;
        let instance = wasmtime::Instance::new(&mut store, &module, &[host_func.into()])?;
        let run = instance.get_typed_func::<(), ()>(&mut store, "run")?;
        run.call(&mut store, ())?;
        Ok(())
    })
}

#[test]
fn caller_data() -> Result<()> {
    let module_bytes = {
        let mut config = Config::new();
        config.concurrency_support(false);
        let engine = Engine::new(&config)?;
        Module::new(
            &engine,
            r#"(module
                (import "" "f" (func))
                (func (export "run") (call 0))
            )"#,
        )?
        .serialize()?
    };
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let module = unsafe { Module::deserialize(&engine, &module_bytes)? };

    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, 42u32)?;
        let host_func = Func::try_new(
            &mut store,
            FuncType::try_new(&engine, [], [])?,
            |caller, _params, _results| {
                assert_eq!(*caller.data(), 42u32);
                Ok(())
            },
        )?;
        let instance = wasmtime::Instance::new(&mut store, &module, &[host_func.into()])?;
        let run = instance.get_typed_func::<(), ()>(&mut store, "run")?;
        run.call(&mut store, ())?;
        Ok(())
    })
}

#[test]
fn caller_engine() -> Result<()> {
    let module_bytes = {
        let mut config = Config::new();
        config.concurrency_support(false);
        let engine = Engine::new(&config)?;
        Module::new(
            &engine,
            r#"(module
                (import "" "f" (func))
                (func (export "run") (call 0))
            )"#,
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
        let host_func = Func::try_new(
            &mut store,
            FuncType::try_new(&engine, [], [])?,
            |caller, _params, _results| {
                let _engine = caller.engine();
                Ok(())
            },
        )?;
        let instance = wasmtime::Instance::new(&mut store, &module, &[host_func.into()])?;
        let run = instance.get_typed_func::<(), ()>(&mut store, "run")?;
        run.call(&mut store, ())?;
        Ok(())
    })
}
