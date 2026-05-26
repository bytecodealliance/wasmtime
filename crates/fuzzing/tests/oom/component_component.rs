#![cfg(arc_try_new)]

use wasmtime::component::Component;
use wasmtime::{Config, Engine, Result};
use wasmtime_fuzzing::oom::OomTest;

#[test]
fn component_serialize() -> Result<()> {
    let component_bytes = {
        let mut config = Config::new();
        config.concurrency_support(false);
        let engine = Engine::new(&config)?;
        Component::new(
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
        )?
        .serialize()?
    };
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let component = unsafe { Component::deserialize(&engine, &component_bytes)? };

    // Error propagation via anyhow allocates after OOM.
    OomTest::new().allow_alloc_after_oom(true).test(|| {
        let _bytes = component.serialize()?;
        Ok(())
    })
}
