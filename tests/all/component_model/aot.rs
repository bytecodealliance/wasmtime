use wasmtime::Result;
use wasmtime::component::types::ComponentItem;
use wasmtime::component::{Component, Linker, Type};
use wasmtime::{Config, Engine, Module, Precompiled, Store};

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
#[cfg_attr(miri, ignore)]
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

#[test]
#[cfg_attr(miri, ignore)]
fn detect_precompiled() -> Result<()> {
    let engine = super::engine();
    let buffer = Component::new(&engine, "(component)")?.serialize()?;
    assert_eq!(Engine::detect_precompiled(&[]), None);
    assert_eq!(Engine::detect_precompiled(&buffer[..5]), None);
    assert_eq!(
        Engine::detect_precompiled(&buffer),
        Some(Precompiled::Component)
    );
    Ok(())
}

#[test]
fn reflect_resource_import() -> Result<()> {
    let engine = super::engine();
    let c = Component::new(
        &engine,
        r#"
        (component
            (import "x" (type $x (sub resource)))
            (import "y" (func (result (own $x))))
        )
        "#,
    )?;
    let ty = c.component_type();
    let mut imports = ty.imports(&engine);
    let (_, x) = imports.next().unwrap();
    let (_, y) = imports.next().unwrap();
    let x = match x.ty {
        ComponentItem::Resource(t) => t,
        _ => unreachable!(),
    };
    let y = match y.ty {
        ComponentItem::ComponentFunc(t) => t,
        _ => unreachable!(),
    };
    let result = y.results().next().unwrap();
    assert_eq!(result, Type::Own(x));

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn truncated_component_binaries_dont_panic() -> Result<()> {
    let engine = super::engine();

    let binary = wat::parse_str(
        r#"
        (component
            (import "a" (core module $m0
                (import "" "" (func))
            ))

            (core module $m1
                (func (export ""))
            )
            (core instance $i1 (instantiate (module $m1)))
            (func $f (canon lift (core func $i1 "f")))

            (component $c1
                (import "f" (func))
                (core module $m2
                    (func (export "g"))
                )
                (core instance $i2 (instantiate $m2))
                (func (export "g")
                    (canon lift (core func $i2 "g"))
                )
            )
            (instance $i3 (instantiate $c1 (with "f" (func $f))))
            (func (export "g") (alias export $i3 "g"))
        )
        "#,
    )?;

    // Check that if we feed each truncation of the component binary into
    // `Component::new` we don't get any panics.
    for i in 1..binary.len() - 1 {
        let _ = Component::from_binary(&engine, &binary[0..i]);
    }

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn implements_shows_up() -> Result<()> {
    let mut config = Config::new();
    config.wasm_component_model_implements(true);
    let engine = Engine::new(&config)?;
    let component = Component::new(
        &engine,
        r#"
            (component
                (import "a" (implements "a1:b1/c1") (instance $a))
                (export "b" (implements "a2:b2/c2") (instance $a))

                (import "v" (implements "a:b/c@1.2.0") (instance))
            )
        "#,
    )?;

    let ty = component.component_type();
    let mut imports = ty.imports(&engine);
    let (_, a) = imports.next().unwrap();
    assert_eq!(a.implements.as_deref(), Some("a1:b1/c1"));
    assert!(a.is_implements("a1:b1/c1"));
    assert!(!a.is_implements("a:b/c"));
    assert!(!a.is_implements("a1:b1/c1@1.0.0"));
    let mut exports = ty.exports(&engine);
    let (_, b) = exports.next().unwrap();
    assert_eq!(b.implements.as_deref(), Some("a2:b2/c2"));

    let (_, a) = imports.next().unwrap();
    assert_eq!(a.implements.as_deref(), Some("a:b/c@1.2.0"));
    assert!(!a.is_implements("a:b/c"));
    assert!(a.is_implements("a:b/c@1.2.0"));
    assert!(a.is_implements("a:b/c@1.3.0"));
    assert!(a.is_implements("a:b/c@1.0.0"));

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn issue_13540_resources_in_adapter_no_concurrency() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    engine.precompile_component(
        br#"
(component
  (component $A
    (type $t' (resource (rep i32)))
    (export $t "t" (type $t'))

    (core module $m (func (export "r") (param i32)))
    (core instance $i (instantiate $m))
    (func (export "r") (param "a" (borrow $t)) (canon lift (core func $i "r")))
  )
  (component $B
    (import "a" (instance $a
      (export "t" (type $t (sub resource)))
      (export "r" (func (param "a" (borrow $t))))
    ))

    (core func $r (canon lower (func $a "r")))
  )
  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "a" (instance $a))))
)
    "#,
    )?;
    Ok(())
}
