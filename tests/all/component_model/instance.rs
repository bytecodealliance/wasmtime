use anyhow::Result;
use wasmtime::component::*;
use wasmtime::{Module, Store};

#[test]
fn instance_exports() -> Result<()> {
    let engine = super::engine();
    let component = r#"
        (component
            (import "a" (instance $i))
            (import "b" (instance $i2 (export "m" (core module))))

            (alias export $i2 "m" (core module $m))

            (component $c
                (component $c
                    (export "m" (core module $m))
                )
                (instance $c (instantiate $c))
                (export "i" (instance $c))
            )
            (instance $c (instantiate $c))
            (export "i" (instance $c))
            (export "r" (instance $i))
            (export "r2" (instance $i2))
        )
    "#;
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);
    linker.instance("a")?;
    linker
        .instance("b")?
        .module("m", &Module::new(&engine, "(module)")?)?;
    let instance = linker.instantiate(&mut store, &component)?;

    assert!(instance
        .get_export(&mut store, None, "not an instance")
        .is_none());
    let i = instance.get_export(&mut store, None, "r").unwrap();
    assert!(instance.get_export(&mut store, Some(&i), "x").is_none());
    instance.get_export(&mut store, None, "i").unwrap();
    let i2 = instance.get_export(&mut store, None, "r2").unwrap();
    let m = instance.get_export(&mut store, Some(&i2), "m").unwrap();
    assert!(instance.get_func(&mut store, &m).is_none());
    assert!(instance.get_module(&mut store, &m).is_some());

    let i = instance.get_export(&mut store, None, "i").unwrap();
    let i = instance.get_export(&mut store, Some(&i), "i").unwrap();
    let m = instance.get_export(&mut store, Some(&i), "m").unwrap();
    instance.get_module(&mut store, &m).unwrap();

    Ok(())
}
