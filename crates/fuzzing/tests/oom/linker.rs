#![cfg(arc_try_new)]

use wasmtime::{Config, Engine, Func, FuncType, Linker, Module, Result, Store};
use wasmtime_fuzzing::oom::OomTest;

#[test]
fn linker_new() -> Result<()> {
    OomTest::new().test(|| {
        let mut config = Config::new();
        config.enable_compiler(false);
        let engine = Engine::new(&config)?;
        let _linker = Linker::<()>::new(&engine);
        Ok(())
    })
}

#[test]
fn linker_func_wrap() -> Result<()> {
    OomTest::new().test(|| {
        let mut config = Config::new();
        config.enable_compiler(false);
        let engine = Engine::new(&config)?;
        let mut linker = Linker::<()>::new(&engine);
        linker.func_wrap("module", "func", |x: i32| x * 2)?;
        Ok(())
    })
}

#[test]
fn linker_instantiate_pre() -> Result<()> {
    let module_bytes = {
        let mut config = Config::new();
        config.concurrency_support(false);
        let engine = Engine::new(&config)?;
        let module = Module::new(
            &engine,
            r#"
                (module
                    (import "module" "func" (func (param i32) (result i32)))

                    (memory (export "memory") 1)
                    (data (i32.const 0) "a")

                    (table (export "table") 1 funcref)
                    (elem (i32.const 0) func 1)

                    (func (export "func") (param i32) (result i32)
                        (call 0 (local.get 0))
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

    let mut linker = Linker::<()>::new(&engine);
    linker.func_wrap("module", "func", |x: i32| x * 2)?;

    let module = unsafe { Module::deserialize(&engine, &module_bytes)? };

    OomTest::new().test(|| {
        let _ = linker.instantiate_pre(&module)?;
        Ok(())
    })
}

#[test]
fn linker_define() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let mut linker = Linker::<()>::new(&engine);
        let func = Func::try_wrap(&mut store, || {})?;
        linker.define(&store, "mod", "func", func)?;
        Ok(())
    })
}

#[test]
fn linker_func_new() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new().test(|| {
        let mut linker = Linker::<()>::new(&engine);
        linker.func_new(
            "mod",
            "func",
            FuncType::try_new(&engine, [], [])?,
            |_caller, _params, _results| Ok(()),
        )?;
        Ok(())
    })
}

#[test]
fn linker_instance() -> Result<()> {
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
    let pre_linker = Linker::<()>::new(&engine);
    let instance_pre = pre_linker.instantiate_pre(&module)?;

    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let instance = instance_pre.instantiate(&mut store)?;
        let mut linker = Linker::<()>::new(&engine);
        linker.instance(&mut store, "inst", instance)?;
        Ok(())
    })
}

#[test]
fn linker_get() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    let mut linker = Linker::<()>::new(&engine);
    linker.func_wrap("mod", "func", || {})?;

    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let ext = linker.try_get(&mut store, "mod", "func")?;
        assert!(ext.is_some());
        Ok(())
    })
}

#[test]
fn linker_get_by_import() -> Result<()> {
    let module_bytes = {
        let mut config = Config::new();
        config.concurrency_support(false);
        let engine = Engine::new(&config)?;
        Module::new(&engine, r#"(module (import "mod" "func" (func)))"#)?.serialize()?
    };
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let module = unsafe { Module::deserialize(&engine, &module_bytes)? };

    let mut linker = Linker::<()>::new(&engine);
    linker.func_wrap("mod", "func", || {})?;

    let import = module.imports().next().unwrap();

    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let ext = linker.try_get_by_import(&mut store, &import)?;
        assert!(ext.is_some());
        Ok(())
    })
}

#[test]
fn linker_get_default() -> Result<()> {
    let module_bytes = {
        let mut config = Config::new();
        config.concurrency_support(false);
        let engine = Engine::new(&config)?;
        Module::new(&engine, r#"(module (func (export "_start")))"#)?.serialize()?
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
        let mut linker = Linker::<()>::new(&engine);
        linker.instance(&mut store, "inst", instance)?;
        let _default = linker.get_default(&mut store, "inst")?;
        Ok(())
    })
}

#[test]
fn linker_define_name() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let mut linker = Linker::<()>::new(&engine);
        let func = Func::try_wrap(&mut store, || {})?;
        linker.define_name(&store, "func", func)?;
        Ok(())
    })
}

// Note: linker_define_unknown_imports_as_traps and
// linker_define_unknown_imports_as_default_values are not tested under OOM
// because UnknownImportError::new uses infallible String allocations
// (to_string) that cannot be made fallible without changing the public API.

#[test]
fn linker_alias() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new().test(|| {
        let mut linker = Linker::<()>::new(&engine);
        linker.func_wrap("mod", "func", || {})?;
        linker.alias("mod", "func", "mod2", "func2")?;
        Ok(())
    })
}

#[test]
fn linker_alias_module() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new().test(|| {
        let mut linker = Linker::<()>::new(&engine);
        linker.func_wrap("mod", "func1", || {})?;
        linker.func_wrap("mod", "func2", || {})?;
        linker.alias_module("mod", "mod2")?;
        Ok(())
    })
}
