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
#[cfg_attr(miri, ignore)]
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
    let func = instance.get_typed_func::<(), (u32,)>(&mut store, "a")?;
    assert_eq!(func.call(&mut store, ())?, (103,));

    Ok(())
}

#[test]
fn deserialize_from_serialized() -> Result<()> {
    let engine = super::engine();
    let buffer1 = Component::new(&engine, "(component (core module))")?.serialize()?;
    let buffer2 = unsafe { Component::deserialize(&engine, &buffer1)?.serialize()? };
    assert!(buffer1 == buffer2);
    Ok(())
}

// This specifically tests the current behavior that it's an error, but this can
// be made to work if necessary in the future. Currently the implementation of
// `serialize` is not conducive to easily implementing this feature and
// otherwise it's not seen as too important to implement.
#[test]
fn cannot_serialize_exported_module() -> Result<()> {
    let engine = super::engine();
    let component = Component::new(
        &engine,
        r#"(component
            (core module $m)
            (export "a" (core module $m))
        )"#,
    )?;
    let mut store = Store::new(&engine, ());
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let module = instance.get_module(&mut store, "a").unwrap();
    assert!(module.serialize().is_err());
    Ok(())
}

#[test]
fn usable_exported_modules() -> Result<()> {
    let engine = super::engine();
    let component = Component::new(
        &engine,
        r#"(component
            (core module $m)
            (core module $m1 (export "a")
                (import "" "" (func (param i32)))
            )
        )"#,
    )?;
    let mut store = Store::new(&engine, ());
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let module = instance.get_module(&mut store, "a").unwrap();
    let mut core_linker = wasmtime::Linker::new(&engine);
    core_linker.func_wrap("", "", |_: u32| {})?;
    core_linker.instantiate(&mut store, &module)?;
    Ok(())
}
