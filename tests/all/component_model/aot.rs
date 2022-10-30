use anyhow::Result;
use wasmtime::component::{Component, Linker};
use wasmtime::{Module, Store};

#[test]
fn module_component_mismatch() -> Result<()> {
    let engine = super::engine();
    let module = Module::new(&engine, "(module)")?.serialize()?;
    let component = Component::new(&engine, "(component)")?.serialize()?;

    unsafe {
        assert!(Module::deserialize(&engine, &component).is_err());
        assert!(Component::deserialize(&engine, &module).is_err());
    }

    Ok(())
}

#[test]
fn bare_bones() -> Result<()> {
    let engine = super::engine();
    let component = Component::new(&engine, "(component)")?.serialize()?;
    assert_eq!(component, engine.precompile_component(b"(component)")?);

    let component = unsafe { Component::deserialize(&engine, &component)? };
    let mut store = Store::new(&engine, ());
    Linker::new(&engine).instantiate(&mut store, &component)?;

    Ok(())
}

#[test]
fn mildly_more_interesting() -> Result<()> {
    let engine = super::engine();
    let component = Component::new(
        &engine,
        r#"
            (component
                (core module $a
                    (func (export "a") (result i32)
                        i32.const 100)
                )
                (core instance $a (instantiate $a))

                (core module $b
                    (import "a" "a" (func $import (result i32)))
                    (func (export "a") (result i32)
                        call $import
                        i32.const 3
                        i32.add)
                )
                (core instance $b (instantiate $b (with "a" (instance $a))))

                (func (export "a") (result u32)
                    (canon lift (core func $b "a"))
                )
            )
        "#,
    )?
    .serialize()?;

    let component = unsafe { Component::deserialize(&engine, &component)? };
    let mut store = Store::new(&engine, ());
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let func = instance.get_typed_func::<(), (u32,), _>(&mut store, "a")?;
    assert_eq!(func.call(&mut store, ())?, (103,));

    Ok(())
}
