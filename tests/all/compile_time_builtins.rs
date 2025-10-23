use super::*;
use std::path::Path;
use wasmtime::*;

#[test]
#[cfg_attr(miri, ignore)]
fn smoke() -> Result<()> {
    let engine = Engine::default();
    let linker = component::Linker::new(&engine);

    let component = unsafe {
        CodeBuilder::new(&engine)
            .expose_unsafe_intrinsics("unsafe-intrinsics")
            .compile_time_builtins_binary_or_text(
                "host-api",
                r#"
                    (component
                        (import "unsafe-intrinsics"
                            (instance $intrinsics
                                (export "store-data-address" (func (result u64)))
                                (export "u8-native-load" (func (param "pointer" u64) (result u8)))
                            )
                        )

                        (core func $store-data-address' (canon lower (func $intrinsics "store-data-address")))
                        (core func $u8-native-load' (canon lower (func $intrinsics "u8-native-load")))

                        (core module $m
                            (import "" "store-data-address" (func $store-data-address (result i64)))
                            (import "" "u8-native-load" (func $u8-native-load (param i64) (result i32)))
                            (func (export "get") (result i32)
                                (call $u8-native-load (call $store-data-address))
                            )
                        )

                        (core instance $i
                            (instantiate $m
                                (with "" (instance (export "store-data-address" (func $store-data-address'))
                                                   (export "u8-native-load" (func $u8-native-load'))))
                            )
                        )

                        (func (export "get") (result u8)
                            (canon lift (core func $i "get"))
                        )
                    )
                "#.as_bytes(),
                Some(Path::new("host-api.wat")),
            )?
            .wasm_binary_or_text(
                r#"
                    (component
                        (import "host-api"
                            (instance $host-api
                                (export "get" (func (result u8)))
                            )
                        )

                        (core func $get' (canon lower (func $host-api "get")))

                        (core module $m
                            (import "" "get" (func $get (result i32)))
                            (func (export "double-get") (result i32)
                                (i32.add (call $get) (call $get))
                            )
                        )

                        (core instance $i
                            (instantiate $m (with "" (instance (export "get" (func $get')))))
                        )

                        (func (export "double-get") (result u8)
                            (canon lift (core func $i "double-get"))
                        )
                    )
                "#.as_bytes(),
                Some(Path::new("main.wat")),
            )?
            .compile_component()?
    };

    let mut store = Store::new(&engine, 42_u8);
    let instance = linker.instantiate(&mut store, &component)?;

    let (result,) = instance
        .get_typed_func::<(), (u8,)>(&mut store, "double-get")?
        .call(&mut store, ())?;
    assert_eq!(result, 84);

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn unused_compile_time_builtins() -> Result<()> {
    let engine = Engine::default();
    let linker = component::Linker::new(&engine);

    let component = unsafe {
        CodeBuilder::new(&engine)
            .expose_unsafe_intrinsics("unsafe-intrinsics")
            .compile_time_builtins_binary_or_text(
                "host-api",
                "(component)".as_bytes(),
                Some(Path::new("host-api.wat")),
            )?
            .wasm_binary_or_text(
                r#"
                    (component
                        (core module $m
                            (func (export "foo") (result i32)
                                (i32.const 42)
                            )
                        )

                        (core instance $i (instantiate $m))

                        (func (export "foo") (result u8)
                            (canon lift (core func $i "foo"))
                        )
                    )
                "#
                .as_bytes(),
                Some(Path::new("main.wat")),
            )?
            .compile_component()?
    };

    let mut store = Store::new(&engine, ());
    let instance = linker.instantiate(&mut store, &component)?;

    let (result,) = instance
        .get_typed_func::<(), (u8,)>(&mut store, "foo")?
        .call(&mut store, ())?;
    assert_eq!(result, 42);

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn multiple_compile_time_builtins() -> Result<()> {
    let engine = Engine::default();
    let linker = component::Linker::new(&engine);

    let component = unsafe {
        CodeBuilder::new(&engine)
            .expose_unsafe_intrinsics("unsafe-intrinsics")
            .compile_time_builtins_binary_or_text(
                "host-api1",
                "(component)".as_bytes(),
                Some(Path::new("host-api1.wat")),
            )?
            .compile_time_builtins_binary_or_text(
                "host-api2",
                "(component)".as_bytes(),
                Some(Path::new("host-api2.wat")),
            )?
            .compile_time_builtins_binary_or_text(
                "host-api3",
                "(component)".as_bytes(),
                Some(Path::new("host-api3.wat")),
            )?
            .wasm_binary_or_text(
                r#"
                    (component
                        (import "host-api2" (instance))
                        (import "host-api3" (instance))

                        (core module $m
                            (func (export "foo") (result i32)
                                (i32.const 42)
                            )
                        )

                        (core instance $i (instantiate $m))

                        (func (export "foo") (result u8)
                            (canon lift (core func $i "foo"))
                        )
                    )
                "#
                .as_bytes(),
                Some(Path::new("main.wat")),
            )?
            .compile_component()?
    };

    let mut store = Store::new(&engine, ());
    let instance = linker.instantiate(&mut store, &component)?;

    let (result,) = instance
        .get_typed_func::<(), (u8,)>(&mut store, "foo")?
        .call(&mut store, ())?;
    assert_eq!(result, 42);

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn main_wasm_cannot_use_intrinsics() -> Result<()> {
    let engine = Engine::default();

    let err = unsafe {
        CodeBuilder::new(&engine)
            .expose_unsafe_intrinsics("unsafe-intrinsics")
            .compile_time_builtins_binary_or_text(
                "host-api",
                "(component)".as_bytes(),
                Some(Path::new("host-api.wat")),
            )?
            .wasm_binary_or_text(
                r#"
                    (component
                        (import "unsafe-intrinsics" (instance $intrinsics))
                    )
                "#
                .as_bytes(),
                Some(Path::new("main.wat")),
            )?
            .compile_component()
            .map(|_| ())
            .unwrap_err()
    };

    err.assert_contains("main Wasm cannot import the unsafe intrinsics");
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn import_erased() -> Result<()> {
    let engine = Engine::default();

    let component = unsafe {
        CodeBuilder::new(&engine)
            .expose_unsafe_intrinsics("unsafe-intrinsics")
            .compile_time_builtins_binary_or_text(
                "compile-time-api",
                "(component)".as_bytes(),
                Some(Path::new("compile-time-api.wat")),
            )?
            .wasm_binary_or_text(
                r#"
                    (component
                        (import "compile-time-api" (instance))
                        (import "link-time-api" (instance))
                    )
                "#
                .as_bytes(),
                Some(Path::new("main.wat")),
            )?
            .compile_component()?
    };

    let component_type = component.component_type();
    let imports = component_type
        .imports(&engine)
        .map(|(name, _ty)| name)
        .collect::<Vec<_>>();
    assert_eq!(imports, ["link-time-api"]);

    Ok(())
}
