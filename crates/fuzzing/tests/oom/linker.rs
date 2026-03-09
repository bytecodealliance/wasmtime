#![cfg(arc_try_new)]

use wasmtime::{Config, Engine, Linker, Module, Result};
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
