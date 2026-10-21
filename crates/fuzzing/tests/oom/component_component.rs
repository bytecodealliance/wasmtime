#![cfg(arc_try_new)]

use wasmtime::component::Component;
use wasmtime::{Config, Engine, Result};
use wasmtime_fuzzing::oom::OomTest;

#[test]
fn component_serialize() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let component = Component::new(
        &engine,
        r#"
            (component
                (core module $m
                    (func (export "id") (param i32) (result i32) (local.get 0))
                )
                (core instance $i (instantiate $m))
                (func (export "id") (param "x" s32) (result s32)
                    (canon lift (core func $i "id"))
                )
            )
        "#,
    )?;

    // Error propagation via anyhow allocates after OOM.
    OomTest::new().allow_alloc_after_oom(true).test(|| {
        let _bytes = component.serialize()?;
        Ok(())
    })
}

#[test]
fn component_deserialize() -> Result<()> {
    let component_bytes = {
        let mut config = Config::new();
        config.concurrency_support(false);
        let engine = Engine::new(&config)?;
        Component::new(
            &engine,
            r#"
                (component
                    (type $r (record (field "a" u32) (field "b" string)))
                    (export $r' "r" (type $r))
                    (type $e (enum "a" "b"))
                    (export $e' "e" (type $e))
                    (type $f (flags "a" "b"))
                    (export $f' "f" (type $f))
                    (type $v (variant (case "a") (case "b" (list $r'))))
                    (export $v' "v" (type $v))
                    (type $t (tuple (option $f') (result string (error $e'))))
                    (import "host" (func $host (param "x" string) (result u32)))

                    (core module $libc
                        (memory (export "memory") 1)
                        (func (export "realloc") (param i32 i32 i32 i32) (result i32)
                            unreachable)
                    )
                    (core instance $libc (instantiate $libc))
                    (core func $host (canon lower (func $host)
                        (memory (core memory $libc "memory")) (realloc (core func $libc "realloc"))))

                    (core module $m
                        (import "" "host" (func (param i32 i32) (result i32)))
                        (func (export "id") (param i32) (result i32) (local.get 0))
                        (func (export "tuple") (param i32 i32 i32 i32 i32) (result i32)
                            unreachable)
                    )
                    (core instance $i (instantiate $m
                        (with "" (instance (export "host" (func $host))))
                    ))
                    (func (export "id") (param "x" s32) (result s32)
                        (canon lift (core func $i "id")))
                    (func (export "tuple") (param "x" $t) (result $v')
                        (canon lift (core func $i "tuple")
                            (memory (core memory $libc "memory")) (realloc (core func $libc "realloc"))))
                )
            "#,
        )?
        .serialize()?
    };

    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new()
        // NB: see `module_deserialize` for why allocation after OOM is
        // allowed.
        .allow_alloc_after_oom(true)
        .test(|| unsafe {
            let _ = Component::deserialize(&engine, &component_bytes)?;
            Ok(())
        })
}
