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

    let mut exports = instance.exports(&mut store);
    assert!(exports.instance("not an instance").is_none());
    let mut i = exports.instance("r").unwrap();
    assert!(i.func("x").is_none());
    drop(i);
    exports.root().instance("i").unwrap();
    let mut i2 = exports.instance("r2").unwrap();
    assert!(i2.func("m").is_none());
    assert!(i2.module("m").is_some());
    drop(i2);

    exports
        .instance("i")
        .unwrap()
        .instance("i")
        .unwrap()
        .module("m")
        .unwrap();

    Ok(())
}
