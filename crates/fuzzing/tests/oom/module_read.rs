#![cfg(arc_try_new)]

use wasmtime::{Config, Engine, Module, Result};
use wasmtime_fuzzing::oom::OomTest;

#[test]
fn module_name() -> Result<()> {
    let module_bytes = {
        let mut config = Config::new();
        config.concurrency_support(false);
        let engine = Engine::new(&config)?;
        Module::new(&engine, r#"(module $my_module (func (export "f")))"#)?.serialize()?
    };
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let module = unsafe { Module::deserialize(&engine, &module_bytes)? };

    OomTest::new().test(|| {
        let _name = module.name();
        Ok(())
    })
}

#[test]
fn module_imports() -> Result<()> {
    let module_bytes = {
        let mut config = Config::new();
        config.concurrency_support(false);
        let engine = Engine::new(&config)?;
        Module::new(
            &engine,
            r#"(module (import "mod" "func" (func)) (func (export "f")))"#,
        )?
        .serialize()?
    };
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let module = unsafe { Module::deserialize(&engine, &module_bytes)? };

    OomTest::new().test(|| {
        let count = module.imports().count();
        assert_eq!(count, 1);
        Ok(())
    })
}

#[test]
fn module_exports() -> Result<()> {
    let module_bytes = {
        let mut config = Config::new();
        config.concurrency_support(false);
        let engine = Engine::new(&config)?;
        Module::new(
            &engine,
            r#"(module (func (export "f")) (memory (export "m") 1))"#,
        )?
        .serialize()?
    };
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let module = unsafe { Module::deserialize(&engine, &module_bytes)? };

    OomTest::new().test(|| {
        let count = module.exports().count();
        assert_eq!(count, 2);
        Ok(())
    })
}

#[test]
fn module_get_export() -> Result<()> {
    let module_bytes = {
        let mut config = Config::new();
        config.concurrency_support(false);
        let engine = Engine::new(&config)?;
        Module::new(&engine, r#"(module (func (export "f")))"#)?.serialize()?
    };
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let module = unsafe { Module::deserialize(&engine, &module_bytes)? };

    OomTest::new().test(|| {
        let export = module.get_export("f");
        assert!(export.is_some());
        let missing = module.get_export("nonexistent");
        assert!(missing.is_none());
        Ok(())
    })
}

#[test]
fn module_engine() -> Result<()> {
    let module_bytes = {
        let mut config = Config::new();
        config.concurrency_support(false);
        let engine = Engine::new(&config)?;
        Module::new(&engine, "(module)")?.serialize()?
    };
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let module = unsafe { Module::deserialize(&engine, &module_bytes)? };

    OomTest::new().test(|| {
        let _engine = module.engine();
        Ok(())
    })
}
