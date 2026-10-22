#![cfg(arc_try_new)]

use wasmtime::{Config, Engine, FuncType, Module, Result, ValType, WasmBacktraceDetails};
use wasmtime_fuzzing::oom::OomTest;

#[test]
fn module_deserialize() -> Result<()> {
    let module_bytes = {
        let mut config = Config::new();
        config.concurrency_support(false);
        config.wasm_backtrace_details(WasmBacktraceDetails::Enable);
        let engine = Engine::new(&config)?;
        let module = Module::new(
            &engine,
            r#"
                (module
                    (import "module" "func" (func (param i32) (result i32)))

                    (memory (export "memory") 1)
                    (data (i32.const 0) "a")
                    (data "a")

                    (table (export "table") 1 funcref)
                    (elem (i32.const 0) func 1)
                    (elem func 1)

                    (func $f (export "func") (param i32) (result i32)
                        (call 0 (local.get 0))
                    )

                    (@custom ".debug_info" "a")
                )
            "#,
        )?;
        module.serialize()?
    };

    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    config.wasm_backtrace_details(WasmBacktraceDetails::Enable);
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

/// Check that failing to register a module's types partway through leaves the
/// engine's type registry in a good state, by reusing the engine afterwards.
#[test]
fn module_deserialize_reuse_engine() -> Result<()> {
    let module_bytes = {
        let mut config = Config::new();
        config.concurrency_support(false);
        let engine = Engine::new(&config)?;
        Module::new(
            &engine,
            r#"
                (module
                    (rec
                        (type $a (func (param (ref null $b)) (result i32)))
                        (type $b (func (param (ref null $a))))
                    )
                    (type $c (func (param i32 i32) (result i64)))
                    (func (export "c") (type $c) (i64.const 0))
                    (func (export "b") (type $b))
                )
            "#,
        )?
        .serialize()?
    };

    OomTest::new()
        // See the note on `module_deserialize` above.
        .allow_alloc_after_oom(true)
        .test_with_reuse(
            || {
                let mut config = Config::new();
                config.enable_compiler(false);
                config.concurrency_support(false);
                Engine::new(&config)
            },
            |engine: &mut Engine| unsafe {
                let _ = Module::deserialize(engine, &module_bytes)?;
                Ok(())
            },
            |engine: Engine| unsafe {
                // Whether or not we hit an OOM above, the engine must remain
                // usable and its type registry must not have any of the
                // module's types left half-registered within it.
                let module = Module::deserialize(&engine, &module_bytes)?;

                // The module's types must be as expected.
                let c = module.get_export("c").unwrap().unwrap_func().clone();
                assert_eq!(c.params().len(), 2);
                assert_eq!(c.results().len(), 1);
                let c2 = FuncType::try_new(&engine, [ValType::I32, ValType::I32], [ValType::I64])?;
                assert!(FuncType::eq(&c, &c2), "{c:?} should be equal to {c2:?}");

                Ok(())
            },
        )
}
