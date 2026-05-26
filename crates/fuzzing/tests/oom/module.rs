#![cfg(arc_try_new)]

use wasmtime::{Config, Engine, Module, Result};
use wasmtime_fuzzing::oom::OomTest;

#[test]
fn module_deserialize() -> Result<()> {
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

    OomTest::new()
        // NB: We use `postcard` to deserialize module metadata, and it will
        // return a `postcard::Error::SerdeDeCustom` when we generate an
        // `OutOfMemory` error during deserialization. That is then converted
        // into a `wasmtime::Error`, and in the process we will attempt to box
        // that into an `Error` trait object. There is no good way to avoid all
        // this, so just allow allocation attempts after OOM here.
        .allow_alloc_after_oom(true)
        .test(|| unsafe {
            let _ = Module::deserialize(&engine, &module_bytes)?;
            Ok(())
        })
}

#[test]
fn module_deserialize_multiple_rec_groups() -> Result<()> {
    let module_bytes = {
        let mut config = Config::new();
        config.concurrency_support(false);
        let engine = Engine::new(&config)?;
        Module::new(
            &engine,
            r#"
                (module
                    (type (func (param i64 i64 i64 i64 i64 i64 i64 i64
                                       i64 i64 i64 i64 i64 i64 i64 i64
                                       i64 i64 i64 i64) (result i64)))
                    (type (func (param i64 i64 i64 i64 i64 i64 i64) (result i32)))
                    (global (mut i32) i32.const 1000)
                )
            "#,
        )?
        .serialize()?
    };

    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new().allow_alloc_after_oom(true).test(|| unsafe {
        let _ = Module::deserialize(&engine, &module_bytes)?;
        Ok(())
    })
}
