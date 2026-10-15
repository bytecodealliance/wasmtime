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
