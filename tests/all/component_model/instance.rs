use wasmtime::Result;
use wasmtime::component::*;
use wasmtime::{Config, Engine, Module, Store};

#[test]
#[cfg_attr(miri, ignore)]
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

    assert!(
        instance
            .get_export(&mut store, None, "not an instance")
            .is_none()
    );
    let i = instance.get_export_index(&mut store, None, "r").unwrap();
    assert!(instance.get_export(&mut store, Some(&i), "x").is_none());
    instance.get_export(&mut store, None, "i").unwrap();
    let i2 = instance.get_export_index(&mut store, None, "r2").unwrap();
    let m = instance
        .get_export_index(&mut store, Some(&i2), "m")
        .unwrap();
    assert!(instance.get_func(&mut store, &m).is_none());
    assert!(instance.get_module(&mut store, &m).is_some());

    let i = instance.get_export_index(&mut store, None, "i").unwrap();
    let i = instance
        .get_export_index(&mut store, Some(&i), "i")
        .unwrap();
    let m = instance
        .get_export_index(&mut store, Some(&i), "m")
        .unwrap();
    instance.get_module(&mut store, &m).unwrap();

    Ok(())
}

#[test]
fn export_old_get_new() -> Result<()> {
    let engine = super::engine();
    let component = r#"
        (component
            (core module $m)
            (export "a:b/m@1.0.0" (core module $m))

            (instance $i (export "m" (core module $m)))
            (export "a:b/i@1.0.0" (instance $i))
        )
    "#;

    let component = Component::new(&engine, component)?;
    component.get_export(None, "a:b/m@1.0.1").unwrap();
    let i = component.get_export_index(None, "a:b/i@1.0.1").unwrap();
    component.get_export(Some(&i), "m").unwrap();

    let mut store = Store::new(&engine, ());
    let linker = Linker::new(&engine);
    let instance = linker.instantiate(&mut store, &component)?;

    instance.get_module(&mut store, "a:b/m@1.0.1").unwrap();
    instance
        .get_export(&mut store, None, "a:b/m@1.0.1")
        .unwrap();

    let i = instance
        .get_export_index(&mut store, None, "a:b/i@1.0.1")
        .unwrap();
    instance.get_export(&mut store, Some(&i), "m").unwrap();

    Ok(())
}

#[test]
fn export_new_get_old() -> Result<()> {
    let engine = super::engine();
    let component = r#"
        (component
            (core module $m)
            (export "a:b/m@1.0.1" (core module $m))

            (instance $i (export "m" (core module $m)))
            (export "a:b/i@1.0.1" (instance $i))
        )
    "#;

    let component = Component::new(&engine, component)?;
    component.get_export(None, "a:b/m@1.0.0").unwrap();
    let i = component.get_export_index(None, "a:b/i@1.0.0").unwrap();
    component.get_export(Some(&i), "m").unwrap();

    let mut store = Store::new(&engine, ());
    let linker = Linker::new(&engine);
    let instance = linker.instantiate(&mut store, &component)?;

    instance.get_module(&mut store, "a:b/m@1.0.0").unwrap();
    instance
        .get_export(&mut store, None, "a:b/m@1.0.0")
        .unwrap();

    let i = instance
        .get_export_index(&mut store, None, "a:b/i@1.0.0")
        .unwrap();
    instance.get_export(&mut store, Some(&i), "m").unwrap();

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn export_keeps_highest_on_semver_track() -> Result<()> {
    let mut config = Config::new();
    config.wasm_component_model_canonical_names(true);
    let engine = Engine::new(&config)?;
    let component = r#"
        (component
            (core module $m1)
            (core module $m2 (import "" "" (func)))
            (instance $i1 (export "m" (core module $m1)))
            (instance $i2 (export "m" (core module $m2)))
            (export "a:b/i@1.0.1" (instance $i1))
            (export "a:b/i@1" (versionsuffix ".0.3") (instance $i2))
            (export "a:b/i@1.0.2" (instance $i1))

            (instance $o
                (export "a:b/n@0.2.3" (instance $i2))
                (export "a:b/n@0.2" (versionsuffix ".1") (instance $i1))
            )
            (export "o" (instance $o))
        )
    "#;

    fn assert_m2(module: &Module) {
        assert_eq!(module.imports().len(), 1);
    }

    let component = Component::new(&engine, component)?;

    // Only the highest version on a semver track is exported.
    let names = component
        .component_type()
        .exports(&engine)
        .map(|(name, _)| name.to_string())
        .collect::<Vec<_>>();
    assert_eq!(names, ["a:b/i@1.0.3", "o"]);

    let mut store = Store::new(&engine, ());
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;

    for name in [
        "a:b/i@1.0.0",
        "a:b/i@1.0.1",
        "a:b/i@1.0.2",
        "a:b/i@1.0.3",
        "a:b/i@1.0.4",
        "a:b/i@1",
    ] {
        println!("test {name}");
        let i = component.get_export_index(None, name).unwrap();
        let m = component.get_export_index(Some(&i), "m").unwrap();
        assert_m2(&instance.get_module(&mut store, &m).unwrap());

        let i = instance.get_export_index(&mut store, None, name).unwrap();
        let m = instance
            .get_export_index(&mut store, Some(&i), "m")
            .unwrap();
        assert_m2(&instance.get_module(&mut store, &m).unwrap());
    }

    // The same applies to the exports of an exported instance.
    let o = component.get_export_index(None, "o").unwrap();
    for name in ["a:b/n@0.2.1", "a:b/n@0.2.3", "a:b/n@0.2"] {
        println!("test {name}");
        let i = component.get_export_index(Some(&o), name).unwrap();
        let m = component.get_export_index(Some(&i), "m").unwrap();
        assert_m2(&instance.get_module(&mut store, &m).unwrap());
    }

    // Neither canonical nor a full version.
    assert!(component.get_export_index(None, "a:b/i@1.0").is_none());

    Ok(())
}

#[test]
fn export_canonical_duplicate_full_name() -> Result<()> {
    let mut config = Config::new();
    config.wasm_component_model_canonical_names(true);
    let engine = Engine::new(&config)?;
    let component = r#"
        (component
            (instance $i)
            (export "a:b/i@1" (versionsuffix ".0.1") (instance $i))
            (export "a:b/i@1.0.1" (instance $i))
        )
    "#;

    let err = Component::new(&engine, component).unwrap_err();
    let err = format!("{err:?}");
    assert!(
        err.contains("root export `a:b/i@1.0.1` is exported twice"),
        "{err}"
    );

    Ok(())
}
