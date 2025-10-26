//! Tests related to the unsafe Wasmtime intrinsics we can give to components
//! via `CodeBuilder::expose_unsafe_intrinsics`.

use super::*;
use std::{cell::UnsafeCell, path::Path, sync::Arc};
use wasmtime::*;

#[test]
#[cfg_attr(miri, ignore)]
fn native_loads_and_stores() -> Result<()> {
    let engine = Engine::default();

    for (comp_ty, core_ty) in [
        ("u8", "i32"),
        ("u16", "i32"),
        ("u32", "i32"),
        ("u64", "i64"),
    ] {
        let path = format!("intrinsics::native_loads_and_stores::{comp_ty}");
        let path = Path::new(&path);
        let wat = format!(
            r#"
                (component
                    (import "host" (instance $host (export "get-pointer" (func (result u64)))))
                    (import "unsafe-intrinsics"
                        (instance $intrinsics
                            (export "{comp_ty}-native-load" (func (param "pointer" u64) (result {comp_ty})))
                            (export "{comp_ty}-native-store" (func (param "pointer" u64) (param "value" {comp_ty})))
                        )
                    )

                    (core func $get-pointer' (canon lower (func $host "get-pointer")))
                    (core func $load' (canon lower (func $intrinsics "{comp_ty}-native-load")))
                    (core func $store' (canon lower (func $intrinsics "{comp_ty}-native-store")))

                    (core module $m
                        (import "" "get-pointer" (func $get-pointer (result i64)))
                        (import "" "load" (func $load (param i64) (result {core_ty})))
                        (import "" "store" (func $store (param i64 {core_ty})))
                        (func (export "run")
                            (local $pointer i64)

                            ;; Get a native pointer from the host.
                            (local.set $pointer (call $get-pointer))

                            ;; Assert that loading from the pointer results in `42`.
                            (if ({core_ty}.ne (call $load (local.get $pointer))
                                              ({core_ty}.const 42))
                                (then (unreachable)))

                            ;; Store `-1` through the pointer.
                            (call $store (local.get $pointer) ({core_ty}.const -1))
                        )
                    )

                    (core instance $i
                        (instantiate $m
                            (with "" (instance (export "get-pointer" (func $get-pointer'))
                                               (export "load" (func $load'))
                                               (export "store" (func $store'))))
                        )
                    )

                    (func (export "run") (canon lift (core func $i "run")))
                )
            "#
        );
        let mut code_builder = CodeBuilder::new(&engine);
        code_builder.wasm_binary_or_text(wat.as_bytes(), Some(path))?;
        unsafe {
            code_builder.expose_unsafe_intrinsics("unsafe-intrinsics");
        }
        let component = code_builder.compile_component()?;

        // Data that will be manipulated directly by Wasm via intrinsics.
        let mut data = Arc::new(UnsafeCell::new(0x6666666666666666_u64));

        // Write `42` into `data` at the appropriate place for this type.
        let ptr = data.get();
        unsafe {
            match comp_ty {
                "u8" => ptr.cast::<u8>().write(42),
                "u16" => ptr.cast::<u16>().write(42),
                "u32" => ptr.cast::<u32>().write(42),
                "u64" => ptr.cast::<u64>().write(42),
                _ => unreachable!(),
            }
        }

        // Create a linker and define `host::get-pointer` to return a pointer to
        // `data`.
        let mut linker = component::Linker::new(&engine);
        linker.instance("host")?.func_new("get-pointer", {
            let ptr = ptr as usize;
            let ptr = u64::try_from(ptr).unwrap();
            move |_cx, _, _args, results| {
                results[0] = component::Val::U64(ptr);
                Ok(())
            }
        })?;

        // Instantiate the component and call its `run` function.
        let mut store = Store::new(&engine, ());
        let instance = linker.instantiate(&mut store, &component)?;
        let run = instance.get_typed_func::<(), ()>(&mut store, "run")?;
        run.call(&mut store, ())?;

        // The `run` function should have written `-1` into its view of `data`
        // and should not have modified any other part of it.
        drop(linker);
        let actual = Arc::get_mut(&mut data).unwrap().get_mut().to_ne_bytes();
        let expected = match comp_ty {
            "u8" => [0xFF, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66],
            "u16" => [0xFF, 0xFF, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66],
            "u32" => [0xFF, 0xFF, 0xFF, 0xFF, 0x66, 0x66, 0x66, 0x66],
            "u64" => [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
            _ => unreachable!(),
        };
        assert_eq!(
            expected,
            actual,
            "expected != actual\n\
             \texpected: {}\n\
             \t  actual: {}",
            expected
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>()
                .join(" "),
            actual
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>()
                .join(" "),
        );
    }
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn cannot_enable_unsafe_intrinsics_for_core_module() -> Result<()> {
    let engine = Engine::default();
    let mut code_builder = CodeBuilder::new(&engine);
    code_builder.wasm_binary_or_text("(module)".as_bytes(), None)?;
    unsafe {
        code_builder.expose_unsafe_intrinsics("unsafe-intrinsics");
    }
    let err = code_builder.compile_module().unwrap_err();
    let err = format!("{err:?}");
    assert!(
        err.contains("`CodeBuilder::expose_unsafe_intrinsics` can only be used with components"),
        "unexpected error: {err}"
    );
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn ref_func_of_intrinsic() -> Result<()> {
    let mut config = Config::new();
    config.wasm_function_references(true);
    config.wasm_gc(true);
    let engine = Engine::new(&config)?;

    let path = format!("intrinsics::ref_func_of_intrinsic");
    let path = Path::new(&path);

    let wat = r#"
        (component
            (import "unsafe-intrinsics"
                (instance $intrinsics
                    (export "u32-native-load" (func (param "pointer" u64) (result u32)))
                )
            )

            (core func $load' (canon lower (func $intrinsics "u32-native-load")))

            (core module $m
                (type $ty (func (param i64) (result i32)))
                (import "" "load" (func $load (type $ty)))
                (elem declare func $load)
                (func (export "run") (param $pointer i64) (result i32)
                    (local $f (ref null func))
                    (local.set $f (ref.func $load))
                    (call_ref $ty (local.get $pointer) (ref.cast (ref $ty) (local.get $f)))
                )
            )

            (core instance $i
                (instantiate $m
                    (with "" (instance (export "load" (func $load'))))
                )
            )

            (func (export "run") (param "pointer" u64) (result u32)
              (canon lift (core func $i "run"))
            )
        )
    "#;

    let mut code_builder = CodeBuilder::new(&engine);
    code_builder.wasm_binary_or_text(wat.as_bytes(), Some(path))?;
    unsafe {
        code_builder.expose_unsafe_intrinsics("unsafe-intrinsics");
    }
    let component = code_builder.compile_component()?;

    // Instantiate the component and call its `run` function.
    let linker = component::Linker::new(&engine);
    let mut store = Store::new(&engine, ());
    let instance = linker.instantiate(&mut store, &component)?;
    let run = instance.get_typed_func::<(u64,), (u32,)>(&mut store, "run")?;

    let data = 0x12345678_u32;
    let (result,) = run.call(&mut store, (&data as *const _ as usize as u64,))?;
    assert_eq!(result, data);

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn table_element_segment_with_intrinsic() -> Result<()> {
    let engine = Engine::default();

    let path = format!("intrinsics::ref_func_of_intrinsic");
    let path = Path::new(&path);

    let wat = r#"
        (component
            (import "unsafe-intrinsics"
                (instance $intrinsics
                    (export "u32-native-load" (func (param "pointer" u64) (result u32)))
                )
            )

            (core func $load' (canon lower (func $intrinsics "u32-native-load")))

            (core module $m
                (type $ty (func (param i64) (result i32)))
                (import "" "load" (func $load (type $ty)))
                (table $t 1 1 funcref)
                (elem (table $t) (i32.const 0) func $load)
                (func (export "run") (param $pointer i64) (result i32)
                    (call_indirect (type $ty) (local.get $pointer) (i32.const 0))
                )
            )

            (core instance $i
                (instantiate $m
                    (with "" (instance (export "load" (func $load'))))
                )
            )

            (func (export "run") (param "pointer" u64) (result u32)
              (canon lift (core func $i "run"))
            )
        )
    "#;

    let mut code_builder = CodeBuilder::new(&engine);
    code_builder.wasm_binary_or_text(wat.as_bytes(), Some(path))?;
    unsafe {
        code_builder.expose_unsafe_intrinsics("unsafe-intrinsics");
    }
    let component = code_builder.compile_component()?;

    // Instantiate the component and call its `run` function.
    let linker = component::Linker::new(&engine);
    let mut store = Store::new(&engine, ());
    let instance = linker.instantiate(&mut store, &component)?;
    let run = instance.get_typed_func::<(u64,), (u32,)>(&mut store, "run")?;

    let data = 0x12345678_u32;
    let (result,) = run.call(&mut store, (&data as *const _ as usize as u64,))?;
    assert_eq!(result, data);

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn intrinsics_not_listed_in_imports() -> Result<()> {
    let engine = Engine::default();

    let path = format!("intrinsics::ref_func_of_intrinsic");
    let path = Path::new(&path);

    let wat = r#"
        (component
            (import "unsafe-intrinsics"
                (instance $intrinsics
                    (export "u32-native-load" (func (param "pointer" u64) (result u32)))
                )
            )
        )
    "#;

    let mut code_builder = CodeBuilder::new(&engine);
    code_builder.wasm_binary_or_text(wat.as_bytes(), Some(path))?;
    unsafe {
        code_builder.expose_unsafe_intrinsics("unsafe-intrinsics");
    }
    let component = code_builder.compile_component()?;

    let component_type = component.component_type();
    let imports = component_type.imports(&engine);
    assert_eq!(imports.count(), 0);

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn unknown_intrinsic_function() -> Result<()> {
    let engine = Engine::default();

    let path = format!("intrinsics::ref_func_of_intrinsic");
    let path = Path::new(&path);

    let wat = r#"
        (component
            (import "unsafe-intrinsics"
                (instance $intrinsics
                    (export "unknown" (func (param "pointer" u64) (result u32)))
                )
            )
        )
    "#;

    let mut code_builder = CodeBuilder::new(&engine);
    code_builder.wasm_binary_or_text(wat.as_bytes(), Some(path))?;
    unsafe {
        code_builder.expose_unsafe_intrinsics("unsafe-intrinsics");
    }
    let err = code_builder.compile_component().map(|_| ()).unwrap_err();
    let err = format!("{err:?}");
    assert!(
        err.contains("invalid unsafe intrinsic: \"unknown\""),
        "unexpected error: {err}"
    );

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn bad_extra_param() -> Result<()> {
    let engine = Engine::default();

    let path = format!("intrinsics::ref_func_of_intrinsic");
    let path = Path::new(&path);

    let wat = r#"
        (component
            (import "unsafe-intrinsics"
                (instance $intrinsics
                    (export "u8-native-load" (func (param "pointer" u64) (param "extra" u64) (result u8)))
                )
            )
        )
    "#;

    let mut code_builder = CodeBuilder::new(&engine);
    code_builder.wasm_binary_or_text(wat.as_bytes(), Some(path))?;
    unsafe {
        code_builder.expose_unsafe_intrinsics("unsafe-intrinsics");
    }
    let err = code_builder.compile_component().map(|_| ()).unwrap_err();
    let err = format!("{err:?}");
    assert!(
        err.contains(
            "bad unsafe intrinsics import at `unsafe-intrinsics`: function `u8-native-load` \
             must have 1 parameters, found 2"
        ),
        "unexpected error: {err}"
    );

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn bad_missing_param() -> Result<()> {
    let engine = Engine::default();

    let path = format!("intrinsics::ref_func_of_intrinsic");
    let path = Path::new(&path);

    let wat = r#"
        (component
            (import "unsafe-intrinsics"
                (instance $intrinsics
                    (export "u8-native-load" (func (result u8)))
                )
            )
        )
    "#;

    let mut code_builder = CodeBuilder::new(&engine);
    code_builder.wasm_binary_or_text(wat.as_bytes(), Some(path))?;
    unsafe {
        code_builder.expose_unsafe_intrinsics("unsafe-intrinsics");
    }
    let err = code_builder.compile_component().map(|_| ()).unwrap_err();
    let err = format!("{err:?}");
    assert!(
        err.contains(
            "bad unsafe intrinsics import at `unsafe-intrinsics`: function `u8-native-load` \
         must have 1 parameters, found 0"
        ),
        "unexpected error: {err}"
    );

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn bad_missing_return() -> Result<()> {
    let engine = Engine::default();

    let path = format!("intrinsics::ref_func_of_intrinsic");
    let path = Path::new(&path);

    let wat = r#"
        (component
            (import "unsafe-intrinsics"
                (instance $intrinsics
                    (export "u8-native-load" (func  (param "pointer" u64)))
                )
            )
        )
    "#;

    let mut code_builder = CodeBuilder::new(&engine);
    code_builder.wasm_binary_or_text(wat.as_bytes(), Some(path))?;
    unsafe {
        code_builder.expose_unsafe_intrinsics("unsafe-intrinsics");
    }
    let err = code_builder.compile_component().map(|_| ()).unwrap_err();
    let err = format!("{err:?}");
    assert!(
        err.contains(
            "bad unsafe intrinsics import at `unsafe-intrinsics`: function `u8-native-load` must \
             have 1 results, found 0"
        ),
        "unexpected error: {err}"
    );

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn bad_extra_return() -> Result<()> {
    let engine = Engine::default();

    let path = format!("intrinsics::ref_func_of_intrinsic");
    let path = Path::new(&path);

    let wat = r#"
        (component
            (import "unsafe-intrinsics"
                (instance $intrinsics
                    (export "u8-native-store" (func (param "pointer" u64) (param "value" u8) (result u32)))
                )
            )
        )
    "#;

    let mut code_builder = CodeBuilder::new(&engine);
    code_builder.wasm_binary_or_text(wat.as_bytes(), Some(path))?;
    unsafe {
        code_builder.expose_unsafe_intrinsics("unsafe-intrinsics");
    }
    let err = code_builder.compile_component().map(|_| ()).unwrap_err();
    let err = format!("{err:?}");
    assert!(
        err.contains(
            "bad unsafe intrinsics import at `unsafe-intrinsics`: function `u8-native-store` \
             must have 0 results, found 1"
        ),
        "unexpected error: {err}"
    );

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn bad_param_type() -> Result<()> {
    let engine = Engine::default();

    let path = format!("intrinsics::ref_func_of_intrinsic");
    let path = Path::new(&path);

    let wat = r#"
        (component
            (import "unsafe-intrinsics"
                (instance $intrinsics
                    (export "u8-native-store" (func (param "pointer" u64) (param "value" u16)))
                )
            )
        )
    "#;

    let mut code_builder = CodeBuilder::new(&engine);
    code_builder.wasm_binary_or_text(wat.as_bytes(), Some(path))?;
    unsafe {
        code_builder.expose_unsafe_intrinsics("unsafe-intrinsics");
    }
    let err = code_builder.compile_component().map(|_| ()).unwrap_err();
    let err = format!("{err:?}");
    assert!(
        err.contains(
            "bad unsafe intrinsics import at `unsafe-intrinsics`: parameters[1] for function \
             `u8-native-store` must be `U8`, found `Primitive(U16)`"
        ),
        "unexpected error: {err}"
    );

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn bad_result_type() -> Result<()> {
    let engine = Engine::default();

    let path = format!("intrinsics::ref_func_of_intrinsic");
    let path = Path::new(&path);

    let wat = r#"
        (component
            (import "unsafe-intrinsics"
                (instance $intrinsics
                    (export "u8-native-load" (func (param "pointer" u64) (result u16)))
                )
            )
        )
    "#;

    let mut code_builder = CodeBuilder::new(&engine);
    code_builder.wasm_binary_or_text(wat.as_bytes(), Some(path))?;
    unsafe {
        code_builder.expose_unsafe_intrinsics("unsafe-intrinsics");
    }
    let err = code_builder.compile_component().map(|_| ()).unwrap_err();
    let err = format!("{err:?}");
    assert!(
        err.contains(
            "bad unsafe intrinsics import at `unsafe-intrinsics`: results[0] for function \
             `u8-native-load` must be `U8`, found `Primitive(U16)`"
        ),
        "unexpected error: {err}"
    );

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn store_data_address() -> Result<()> {
    let engine = Engine::default();

    let wat = r#"
        (component
            (import "unsafe-intrinsics"
                (instance $intrinsics
                    (export "store-data-address" (func (result u64)))
                    (export "u8-native-load" (func (param "pointer" u64) (result u8)))
                    (export "u8-native-store" (func (param "pointer" u64) (param "value" u8)))
                )
            )

            (core func $store-data-address' (canon lower (func $intrinsics "store-data-address")))
            (core func $u8-native-load' (canon lower (func $intrinsics "u8-native-load")))
            (core func $u8-native-store' (canon lower (func $intrinsics "u8-native-store")))

            (core module $m
                (import "" "store-data-address" (func $store-data-address (result i64)))
                (import "" "u8-native-load" (func $load (param i64) (result i32)))
                (import "" "u8-native-store" (func $store (param i64 i32)))
                (func (export "run")
                    ;; Load the store data, it should be 42.
                    (if (i32.ne (call $load (call $store-data-address))
                                (i32.const 42))
                        (then (unreachable)))

                    ;; Store 36 into the store data.
                    (call $store (call $store-data-address) (i32.const 36))
                )
            )

            (core instance $i
                (instantiate $m
                    (with "" (instance (export "store-data-address" (func $store-data-address'))
                                       (export "u8-native-load" (func $u8-native-load'))
                                       (export "u8-native-store" (func $u8-native-store'))))
                )
            )

            (func (export "run")
              (canon lift (core func $i "run"))
            )
        )
    "#;

    let mut code_builder = CodeBuilder::new(&engine);
    code_builder.wasm_binary_or_text(wat.as_bytes(), None)?;
    unsafe {
        code_builder.expose_unsafe_intrinsics("unsafe-intrinsics");
    }
    let component = code_builder.compile_component()?;

    // Instantiate the component and call its `run` function.
    let linker = component::Linker::new(&engine);
    let mut store = Store::new(&engine, 42_u8);
    let instance = linker.instantiate(&mut store, &component)?;
    let run = instance.get_typed_func::<(), ()>(&mut store, "run")?;

    run.call(&mut store, ())?;
    assert_eq!(*store.data(), 36);

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn other_import_name() -> Result<()> {
    let engine = Engine::default();

    let wat = r#"
        (component
            (import "other-name"
                (instance
                    (export "u32-native-load" (func (param "pointer" u64) (result u32)))
                )
            )
        )
    "#;

    let mut code_builder = CodeBuilder::new(&engine);
    code_builder.wasm_binary_or_text(wat.as_bytes(), None)?;
    unsafe {
        code_builder.expose_unsafe_intrinsics("other-name");
    }
    code_builder.compile_component()?;

    Ok(())
}
