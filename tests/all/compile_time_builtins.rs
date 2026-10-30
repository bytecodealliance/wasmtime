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

/// A compile-time builtin that exports `get`, returning `value`.
fn constant_builtin(value: u32) -> String {
    format!(
        r#"
            (component
                (core module $m
                    (func (export "get") (result i32)
                        (i32.const {value})
                    )
                )

                (core instance $i (instantiate $m))

                (func (export "get") (result u32)
                    (canon lift (core func $i "get"))
                )
            )
        "#
    )
}

/// A compile-time builtin that imports `lower` as an instance exporting `get`,
/// and exports `get`, returning `lower`'s `get` plus `offset`.
fn layered_builtin(lower: &str, offset: u32) -> String {
    format!(
        r#"
            (component
                (import "{lower}" (instance $lower (export "get" (func (result u32)))))
                (core func $lower-get (canon lower (func $lower "get")))

                (core module $m
                    (import "" "lower-get" (func $lower-get (result i32)))
                    (func (export "get") (result i32)
                        (i32.add (call $lower-get) (i32.const {offset}))
                    )
                )

                (core instance $i
                    (instantiate $m (with "" (instance (export "lower-get" (func $lower-get)))))
                )

                (func (export "get") (result u32)
                    (canon lift (core func $i "get"))
                )
            )
        "#
    )
}

/// A main Wasm that imports each of `names` as an instance exporting `get`,
/// and exports `sum`, the sum of every `get`.
fn summing_main(names: &[&str]) -> String {
    let mut imports = String::new();
    let mut lowers = String::new();
    let mut params = String::new();
    let mut calls = String::from("(i32.const 0)");
    let mut args = String::new();

    for (i, name) in names.iter().enumerate() {
        imports += &format!(
            r#"(import "{name}" (instance $import{i} (export "get" (func (result u32)))))"#
        );
        lowers += &format!(r#"(core func $get{i} (canon lower (func $import{i} "get")))"#);
        params += &format!(r#"(import "" "get{i}" (func $get{i} (result i32)))"#);
        calls = format!("(i32.add {calls} (call $get{i}))");
        args += &format!(r#"(export "get{i}" (func $get{i}))"#);
    }

    format!(
        r#"
            (component
                {imports}
                {lowers}

                (core module $m
                    {params}
                    (func (export "sum") (result i32)
                        {calls}
                    )
                )

                (core instance $i (instantiate $m (with "" (instance {args}))))

                (func (export "sum") (result u32)
                    (canon lift (core func $i "sum"))
                )
            )
        "#
    )
}

/// Instantiates `component` and returns the result of calling its `sum`.
fn call_sum(engine: &Engine, component: &component::Component) -> Result<u32> {
    let linker = component::Linker::new(engine);
    let mut store = Store::new(engine, ());
    let instance = linker.instantiate(&mut store, component)?;
    let (result,) = instance
        .get_typed_func::<(), (u32,)>(&mut store, "sum")?
        .call(&mut store, ())?;

    Ok(result)
}

#[test]
#[cfg_attr(miri, ignore)]
fn interface_name() -> Result<()> {
    let engine = Engine::default();

    let component = unsafe {
        CodeBuilder::new(&engine)
            .expose_unsafe_intrinsics("unsafe-intrinsics")
            .compile_time_builtins_binary_or_text(
                "my:pkg/if@1.0.0",
                constant_builtin(42).as_bytes(),
                Some(Path::new("if.wat")),
            )?
            .wasm_binary_or_text(
                summing_main(&["my:pkg/if@1.0.0"]).as_bytes(),
                Some(Path::new("main.wat")),
            )?
            .compile_component()?
    };

    assert_eq!(call_sum(&engine, &component)?, 42);
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn versions_of_one_interface() -> Result<()> {
    let engine = Engine::default();

    let component = unsafe {
        CodeBuilder::new(&engine)
            .expose_unsafe_intrinsics("unsafe-intrinsics")
            .compile_time_builtins_binary_or_text(
                "a:b/c@1.0.0",
                constant_builtin(1).as_bytes(),
                Some(Path::new("c-1.0.0.wat")),
            )?
            .compile_time_builtins_binary_or_text(
                "a:b/c@1.0.1",
                constant_builtin(10).as_bytes(),
                Some(Path::new("c-1.0.1.wat")),
            )?
            .wasm_binary_or_text(
                summing_main(&["a:b/c@1.0.0", "a:b/c@1.0.1"]).as_bytes(),
                Some(Path::new("main.wat")),
            )?
            .compile_component()?
    };

    assert_eq!(call_sum(&engine, &component)?, 11);
    Ok(())
}

/// A compile-time builtin that exports `get`, returning the `u8` store data,
/// read through the unsafe intrinsics.
const STORE_DATA_BUILTIN: &str = r#"
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

        (func (export "get") (result u32)
            (canon lift (core func $i "get"))
        )
    )
"#;

#[test]
#[cfg_attr(miri, ignore)]
fn builtins_share_intrinsics() -> Result<()> {
    // Both builtins' intrinsics imports merge into one.
    let engine = Engine::default();
    let linker = component::Linker::new(&engine);

    let component = unsafe {
        CodeBuilder::new(&engine)
            .expose_unsafe_intrinsics("unsafe-intrinsics")
            .compile_time_builtins_binary_or_text(
                "a:b/first",
                STORE_DATA_BUILTIN.as_bytes(),
                Some(Path::new("first.wat")),
            )?
            .compile_time_builtins_binary_or_text(
                "a:b/second",
                STORE_DATA_BUILTIN.as_bytes(),
                Some(Path::new("second.wat")),
            )?
            .wasm_binary_or_text(
                summing_main(&["a:b/first", "a:b/second"]).as_bytes(),
                Some(Path::new("main.wat")),
            )?
            .compile_component()?
    };

    let mut store = Store::new(&engine, 42_u8);
    let instance = linker.instantiate(&mut store, &component)?;

    let (result,) = instance
        .get_typed_func::<(), (u32,)>(&mut store, "sum")?
        .call(&mut store, ())?;

    assert_eq!(result, 84);

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn unused_compile_time_builtins_are_not_instantiated() -> Result<()> {
    // `a:b/unused` traps on instantiation, so this only works if it's left out.
    const TRAPS: &str = r#"
        (component
            (core module $m
                (func $trap unreachable)
                (start $trap)
            )
            (core instance (instantiate $m))
        )
    "#;

    let engine = Engine::default();

    let component = unsafe {
        CodeBuilder::new(&engine)
            .expose_unsafe_intrinsics("unsafe-intrinsics")
            .compile_time_builtins_binary_or_text(
                "a:b/used",
                constant_builtin(42).as_bytes(),
                Some(Path::new("used.wat")),
            )?
            .compile_time_builtins_binary_or_text(
                "a:b/unused",
                TRAPS.as_bytes(),
                Some(Path::new("unused.wat")),
            )?
            .wasm_binary_or_text(
                summing_main(&["a:b/used"]).as_bytes(),
                Some(Path::new("main.wat")),
            )?
            .compile_component()?
    };

    assert_eq!(call_sum(&engine, &component)?, 42);
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn invalid_compile_time_builtins_are_named_in_errors() -> Result<()> {
    // Exports a function that doesn't exist, so it fails validation.
    let engine = Engine::default();

    let err = unsafe {
        CodeBuilder::new(&engine)
            .expose_unsafe_intrinsics("unsafe-intrinsics")
            .compile_time_builtins_binary_or_text(
                "a:b/broken",
                r#"(component (export "get" (func 0)))"#.as_bytes(),
                Some(Path::new("broken.wat")),
            )?
            .wasm_binary_or_text(
                summing_main(&["a:b/broken"]).as_bytes(),
                Some(Path::new("main.wat")),
            )?
            .compile_component()
            .map(|_| ())
            .unwrap_err()
    };

    err.assert_contains("`a:b/broken`");
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn unused_compile_time_builtins_with_other_imports() -> Result<()> {
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

#[test]
#[cfg_attr(miri, ignore)]
fn builtins_import_builtins() -> Result<()> {
    // The main Wasm imports only `a:b/outer`, which imports `a:b/inner`.
    let engine = Engine::default();

    let component = unsafe {
        CodeBuilder::new(&engine)
            .expose_unsafe_intrinsics("unsafe-intrinsics")
            .compile_time_builtins_binary_or_text(
                "a:b/outer",
                layered_builtin("a:b/inner", 100).as_bytes(),
                Some(Path::new("outer.wat")),
            )?
            .compile_time_builtins_binary_or_text(
                "a:b/inner",
                constant_builtin(42).as_bytes(),
                Some(Path::new("inner.wat")),
            )?
            .wasm_binary_or_text(
                summing_main(&["a:b/outer"]).as_bytes(),
                Some(Path::new("main.wat")),
            )?
            .compile_component()?
    };

    let component_type = component.component_type();
    assert_eq!(component_type.imports(&engine).len(), 0);
    assert_eq!(call_sum(&engine, &component)?, 142);
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn builtins_are_shared_by_their_importers() -> Result<()> {
    // `get` counts its calls, so the sum is 1 + 2 only if both importers call
    // the same instance.
    const COUNTER: &str = r#"
        (component
            (core module $m
                (global $count (mut i32) (i32.const 0))
                (func (export "get") (result i32)
                    (global.set $count (i32.add (global.get $count) (i32.const 1)))
                    (global.get $count)
                )
            )

            (core instance $i (instantiate $m))

            (func (export "get") (result u32)
                (canon lift (core func $i "get"))
            )
        )
    "#;

    let engine = Engine::default();

    let component = unsafe {
        CodeBuilder::new(&engine)
            .expose_unsafe_intrinsics("unsafe-intrinsics")
            .compile_time_builtins_binary_or_text(
                "a:b/memory",
                COUNTER.as_bytes(),
                Some(Path::new("memory.wat")),
            )?
            .compile_time_builtins_binary_or_text(
                "a:b/buffers",
                layered_builtin("a:b/memory", 0).as_bytes(),
                Some(Path::new("buffers.wat")),
            )?
            .compile_time_builtins_binary_or_text(
                "a:b/strings",
                layered_builtin("a:b/memory", 0).as_bytes(),
                Some(Path::new("strings.wat")),
            )?
            .wasm_binary_or_text(
                summing_main(&["a:b/buffers", "a:b/strings"]).as_bytes(),
                Some(Path::new("main.wat")),
            )?
            .compile_component()?
    };

    assert_eq!(call_sum(&engine, &component)?, 3);
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn builtin_import_cycles_are_named_in_errors() -> Result<()> {
    let engine = Engine::default();

    let err = unsafe {
        CodeBuilder::new(&engine)
            .expose_unsafe_intrinsics("unsafe-intrinsics")
            .compile_time_builtins_binary_or_text(
                "a:b/first",
                layered_builtin("a:b/second", 0).as_bytes(),
                Some(Path::new("first.wat")),
            )?
            .compile_time_builtins_binary_or_text(
                "a:b/second",
                layered_builtin("a:b/first", 0).as_bytes(),
                Some(Path::new("second.wat")),
            )?
            .wasm_binary_or_text(
                summing_main(&["a:b/first"]).as_bytes(),
                Some(Path::new("main.wat")),
            )?
            .compile_component()
            .map(|_| ())
            .unwrap_err()
    };
    err.assert_contains("`a:b/first` -> `a:b/second` -> `a:b/first`");

    let err = unsafe {
        CodeBuilder::new(&engine)
            .expose_unsafe_intrinsics("unsafe-intrinsics")
            .compile_time_builtins_binary_or_text(
                "a:b/self",
                layered_builtin("a:b/self", 0).as_bytes(),
                Some(Path::new("self.wat")),
            )?
            .wasm_binary_or_text(
                summing_main(&["a:b/self"]).as_bytes(),
                Some(Path::new("main.wat")),
            )?
            .compile_component()
            .map(|_| ())
            .unwrap_err()
    };
    err.assert_contains("`a:b/self` -> `a:b/self`");

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn builtins_imported_by_builtins_get_intrinsics() -> Result<()> {
    // Only `a:b/outer` is imported by the main Wasm, but `a:b/inner` uses the
    // intrinsics.
    let engine = Engine::default();
    let linker = component::Linker::new(&engine);

    let component = unsafe {
        CodeBuilder::new(&engine)
            .expose_unsafe_intrinsics("unsafe-intrinsics")
            .compile_time_builtins_binary_or_text(
                "a:b/outer",
                layered_builtin("a:b/inner", 100).as_bytes(),
                Some(Path::new("outer.wat")),
            )?
            .compile_time_builtins_binary_or_text(
                "a:b/inner",
                STORE_DATA_BUILTIN.as_bytes(),
                Some(Path::new("inner.wat")),
            )?
            .wasm_binary_or_text(
                summing_main(&["a:b/outer"]).as_bytes(),
                Some(Path::new("main.wat")),
            )?
            .compile_component()?
    };

    let mut store = Store::new(&engine, 42_u8);
    let instance = linker.instantiate(&mut store, &component)?;

    let (result,) = instance
        .get_typed_func::<(), (u32,)>(&mut store, "sum")?
        .call(&mut store, ())?;

    assert_eq!(result, 142);

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn builtin_link_errors_name_the_importer() -> Result<()> {
    // `a:b/inner` doesn't export the `get` that `a:b/outer` imports.
    let engine = Engine::default();

    let err = unsafe {
        CodeBuilder::new(&engine)
            .expose_unsafe_intrinsics("unsafe-intrinsics")
            .compile_time_builtins_binary_or_text(
                "a:b/outer",
                layered_builtin("a:b/inner", 0).as_bytes(),
                Some(Path::new("outer.wat")),
            )?
            .compile_time_builtins_binary_or_text(
                "a:b/inner",
                "(component)".as_bytes(),
                Some(Path::new("inner.wat")),
            )?
            .wasm_binary_or_text(
                summing_main(&["a:b/outer"]).as_bytes(),
                Some(Path::new("main.wat")),
            )?
            .compile_component()
            .map(|_| ())
            .unwrap_err()
    };

    err.assert_contains("`a:b/inner`");
    err.assert_contains("`a:b/outer`");
    Ok(())
}
