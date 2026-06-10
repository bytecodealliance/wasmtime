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

/// Exercise intrinsics that are:
///
/// - lowered to core functions,
/// - re-exported directly from a core instance *without* being wrapped in any
///   core function,
/// - lifted back into component functions,
/// - and then called directly by the host.
///
/// This is a tricky case for intrinsics like `store-data-address` that read out
/// of a vmctx: in this path the intrinsic trampoline is reached without any
/// intervening core-Wasm caller, so we must not assume that the trampoline's
/// caller vmctx is a core-Wasm `VMContext`.
#[test]
#[cfg_attr(miri, ignore)]
fn directly_reexported_and_lifted_intrinsics() -> Result<()> {
    let engine = Engine::default();

    let wat = r#"
        (component
            (import "unsafe-intrinsics"
                (instance $intrinsics
                    (export "store-data-address" (func (result u64)))
                    (export "u64-native-load" (func (param "pointer" u64) (result u64)))
                )
            )

            ;; Lower the intrinsics to core functions.
            (core func $store-data-address' (canon lower (func $intrinsics "store-data-address")))
            (core func $u64-native-load' (canon lower (func $intrinsics "u64-native-load")))

            ;; A core module that imports the intrinsics and re-exports them
            ;; directly, without wrapping them in any core function of its own.
            (core module $m
                (import "" "store-data-address" (func $store-data-address (result i64)))
                (import "" "u64-native-load" (func $load (param i64) (result i64)))
                (export "store-data-address" (func $store-data-address))
                (export "u64-native-load" (func $load))
            )

            (core instance $i
                (instantiate $m
                    (with "" (instance (export "store-data-address" (func $store-data-address'))
                                       (export "u64-native-load" (func $u64-native-load'))))
                )
            )

            ;; Lift the re-exported core functions directly back into component
            ;; functions and export them.
            (func (export "store-data-address") (result u64)
                (canon lift (core func $i "store-data-address")))
            (func (export "u64-native-load") (param "pointer" u64) (result u64)
                (canon lift (core func $i "u64-native-load")))
        )
    "#;

    let mut code_builder = CodeBuilder::new(&engine);
    code_builder.wasm_binary_or_text(wat.as_bytes(), None)?;
    unsafe {
        code_builder.expose_unsafe_intrinsics("unsafe-intrinsics");
    }
    let component = code_builder.compile_component()?;

    let known = 0x1122_3344_5566_7788_u64;
    let linker = component::Linker::new(&engine);
    let mut store = Store::new(&engine, known);
    let instance = linker.instantiate(&mut store, &component)?;

    let store_data_address =
        instance.get_typed_func::<(), (u64,)>(&mut store, "store-data-address")?;
    let load = instance.get_typed_func::<(u64,), (u64,)>(&mut store, "u64-native-load")?;

    // `store-data-address` must return the address of the store's `T` data,
    // which is the same address that `Store::data` exposes.
    let (address,) = store_data_address.call(&mut store, ())?;
    let expected = core::ptr::from_ref(store.data()) as u64;
    assert_eq!(
        address, expected,
        "store-data-address returned the wrong pointer"
    );

    // And loading through that address (also via a directly-lifted intrinsic)
    // must observe the known store data.
    let (value,) = load.call(&mut store, (address,))?;
    assert_eq!(
        value, known,
        "u64-native-load through store-data-address read the wrong data"
    );

    Ok(())
}

/// A 16-byte buffer aligned to 8 bytes so that aligned native accesses of any
/// of our intrinsic widths (`u8`/`u16`/`u32`/`u64`) are well-defined.
#[repr(align(8))]
struct AlignedBuf([u8; 16]);

/// Get the native-endian byte encoding of `x` truncated to the given intrinsic
/// width.
fn ne_bytes(comp_ty: &str, x: u64) -> Vec<u8> {
    match comp_ty {
        "u8" => (x as u8).to_ne_bytes().to_vec(),
        "u16" => (x as u16).to_ne_bytes().to_vec(),
        "u32" => (x as u32).to_ne_bytes().to_vec(),
        "u64" => x.to_ne_bytes().to_vec(),
        _ => unreachable!(),
    }
}

/// Truncate `x` to the given intrinsic width, zero-extended back into a `u64`
/// (matching what a checked load of that width returns).
fn mask(comp_ty: &str, x: u64) -> u64 {
    match comp_ty {
        "u8" => x as u8 as u64,
        "u16" => x as u16 as u64,
        "u32" => x as u32 as u64,
        "u64" => x,
        _ => unreachable!(),
    }
}

fn val_to_u64(v: &component::Val) -> u64 {
    match v {
        component::Val::U8(x) => u64::from(*x),
        component::Val::U16(x) => u64::from(*x),
        component::Val::U32(x) => u64::from(*x),
        component::Val::U64(x) => *x,
        other => panic!("unexpected result value: {other:?}"),
    }
}

fn val_of_width(comp_ty: &str, x: u64) -> component::Val {
    match comp_ty {
        "u8" => component::Val::U8(x as u8),
        "u16" => component::Val::U16(x as u16),
        "u32" => component::Val::U32(x as u32),
        "u64" => component::Val::U64(x),
        _ => unreachable!(),
    }
}

/// Exercise the bounds-checked native load/store intrinsics, both the
/// non-trapping (in-bounds) and trapping (out-of-bounds and overflowing) cases,
/// for every access width.
///
/// The functional behavior must be identical regardless of whether Spectre
/// mitigations are enabled; `spectre` controls the
/// `enable_heap_access_spectre_mitigation` Cranelift setting so that we cover
/// both code paths. (The actual codegen difference is checked by the `disas`
/// filetests.)
///
/// `inlining` controls whether the intrinsics are inlined into their callers,
/// which is a separate code path in the compiler (the trampoline compiler
/// versus the inlined-intrinsic compiler); we cover both.
fn checked_native_loads_and_stores(spectre: bool, inlining: bool) -> Result<()> {
    let mut config = Config::new();
    config.compiler_inlining(if inlining {
        Inlining::Yes
    } else {
        Inlining::No
    });

    if spectre {
        unsafe {
            config.cranelift_flag_set("enable_heap_access_spectre_mitigation", "true");
        }
    } else {
        unsafe {
            config.cranelift_flag_set("enable_heap_access_spectre_mitigation", "false");
        }
    }

    let engine = match Engine::new(&config) {
        Ok(engine) => engine,

        // Some build configurations don't support signals-based traps, which
        // means that we cannot test checked intrinsics with Spectre mitigations
        // on them, as Spectre mitigations require signals-based traps.
        Err(e)
            if spectre
                && e.to_string().contains(
                    "when signals-based traps are disabled then spectre mitigations \
                     must also be disabled",
                ) =>
        {
            return Ok(());
        }

        Err(e) => return Err(e),
    };

    // A few distinct values, used for the initial buffer contents (`KNOWN*`)
    // and the values written by the store intrinsic (`STORED*`).
    const KNOWN: u64 = 0x1122_3344_5566_7788;
    const KNOWN2: u64 = 0x99aa_bbcc_ddee_ff00;
    const STORED: u64 = 0xa5b6_c7d8_e9fa_0b1c;
    const STORED2: u64 = 0x0102_0304_0506_0708;

    for (comp_ty, core_ty, size) in [
        ("u8", "i32", 1),
        ("u16", "i32", 2),
        ("u32", "i32", 4),
        ("u64", "i64", 8),
    ] {
        let wat = format!(
            r#"
                (component
                    ;; Import the unsafe intrinsics.
                    (import "unsafe-intrinsics"
                        (instance $intrinsics
                            (export "{comp_ty}-checked-native-load"
                                (func (param "base" u64) (param "offset" u64) (param "length" u64)
                                      (result {comp_ty})))
                            (export "{comp_ty}-checked-native-store"
                                (func (param "base" u64) (param "offset" u64) (param "length" u64)
                                      (param "value" {comp_ty})))
                        )
                    )

                    ;; Lower them to core functions.
                    (core func $load' (canon lower (func $intrinsics "{comp_ty}-checked-native-load")))
                    (core func $store' (canon lower (func $intrinsics "{comp_ty}-checked-native-store")))

                    ;; Define a core module that imports them and exports functions that wrap them.
                    (core module $m
                        (import "" "load" (func $load (param i64 i64 i64) (result {core_ty})))
                        (import "" "store" (func $store (param i64 i64 i64 {core_ty})))

                        (func (export "load") (param $base i64) (param $offset i64) (param $length i64)
                                              (result {core_ty})
                            (call $load (local.get $base)
                                        (local.get $offset)
                                        (local.get $length))
                        )

                        (func (export "store") (param $base i64) (param $offset i64) (param $length i64)
                                               (param $value {core_ty})
                            (call $store (local.get $base)
                                         (local.get $offset)
                                         (local.get $length)
                                         (local.get $value))
                        )
                    )

                    ;; Instantiate that core module.
                    (core instance $i
                        (instantiate $m
                            (with "" (instance (export "load" (func $load'))
                                               (export "store" (func $store'))))
                        )
                    )

                    ;; Export lifted versions of the core instance's wrapper functions.
                    (func (export "load") (param "base" u64)
                                          (param "offset" u64)
                                          (param "length" u64)
                                          (result {comp_ty})
                        (canon lift (core func $i "load"))
                    )
                    (func (export "store") (param "base" u64)
                                           (param "offset" u64)
                                           (param "length" u64)
                                           (param "value" {comp_ty})
                        (canon lift (core func $i "store"))
                    )
                )
            "#
        );

        let mut code_builder = CodeBuilder::new(&engine);
        code_builder.wasm_binary_or_text(wat.as_bytes(), None)?;
        unsafe {
            code_builder.expose_unsafe_intrinsics("unsafe-intrinsics");
        }
        let component = code_builder.compile_component()?;
        let linker = component::Linker::new(&engine);

        // Allocate a host buffer that Wasm will access directly via the
        // intrinsics. We tell the intrinsics its length so they can bounds
        // check accesses against it.
        let data = Arc::new(UnsafeCell::new(AlignedBuf([0; 16])));
        let base_ptr = data.get().cast::<u8>();
        let base = base_ptr as usize as u64;
        let length: u64 = 16;
        // The last offset at which a `size`-byte access is in bounds.
        let boundary: u64 = length - size;

        // Initialize known values at the start of the buffer and at the last
        // in-bounds slot.
        unsafe {
            let bytes = ne_bytes(comp_ty, KNOWN);
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), base_ptr, bytes.len());
            let bytes = ne_bytes(comp_ty, KNOWN2);
            std::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                base_ptr.add(usize::try_from(boundary).unwrap()),
                bytes.len(),
            );
        }

        // Each call uses a fresh store and instance because a trap leaves the
        // instance unusable.
        let call_load = |offset: u64, len: u64| -> Result<u64> {
            let mut store = Store::new(&engine, ());
            let instance = linker.instantiate(&mut store, &component)?;
            let func = instance.get_func(&mut store, "load").unwrap();
            let mut results = [component::Val::Bool(false)];
            func.call(
                &mut store,
                &[
                    component::Val::U64(base),
                    component::Val::U64(offset),
                    component::Val::U64(len),
                ],
                &mut results,
            )?;
            Ok(val_to_u64(&results[0]))
        };
        let call_store = |offset: u64, len: u64, value: u64| -> Result<()> {
            let mut store = Store::new(&engine, ());
            let instance = linker.instantiate(&mut store, &component)?;
            let func = instance.get_func(&mut store, "store").unwrap();
            func.call(
                &mut store,
                &[
                    component::Val::U64(base),
                    component::Val::U64(offset),
                    component::Val::U64(len),
                    val_of_width(comp_ty, value),
                ],
                &mut [],
            )?;
            Ok(())
        };
        let assert_oob = |result: Result<u64>, what: &str| {
            let err = result.err().unwrap_or_else(|| {
                panic!("expected a trap for {what} (spectre={spectre}, inlining={inlining}, ty={comp_ty})")
            });
            match err.downcast::<Trap>() {
                Ok(trap) => assert_eq!(
                    trap,
                    Trap::MemoryOutOfBounds,
                    "wrong trap for {what} (spectre={spectre}, inlining={inlining}, ty={comp_ty})"
                ),
                Err(e) => panic!(
                    "expected a Trap for {what} (spectre={spectre}, inlining={inlining}, ty={comp_ty}), got: {e:?}"
                ),
            }
        };

        // Non-trapping loads.

        // A load at offset 0 reads the value we wrote there.
        assert_eq!(call_load(0, length)?, mask(comp_ty, KNOWN));
        // A load at the very last in-bounds offset is allowed.
        assert_eq!(call_load(boundary, length)?, mask(comp_ty, KNOWN2));

        // Trapping loads.

        // One byte past the last in-bounds offset traps.
        assert_oob(call_load(boundary + 1, length), "load one past the end");
        // An offset equal to the length traps.
        assert_oob(call_load(length, length), "load at offset == length");
        // The `length` argument is what is checked, not the underlying
        // allocation: shrinking it makes otherwise-valid offsets trap.
        assert_oob(call_load(size, size), "load past a shortened length");
        // A huge offset whose `offset + size` does not overflow but exceeds the
        // length still traps (and must not wrap around to an in-bounds access).
        assert_oob(call_load(u64::MAX - 100, length), "load at a huge offset");
        // An offset whose `offset + size` overflows traps.
        assert_oob(
            call_load(u64::MAX, length),
            "load whose offset + size overflows",
        );

        // Non-trapping stores.

        // Store at offset 0 and at the last in-bounds offset, then read the
        // values back out of the host buffer.
        call_store(0, length, STORED)?;
        call_store(boundary, length, STORED2)?;
        let buf = unsafe { (*data.get()).0 };
        let size = usize::try_from(size).unwrap();
        let boundary = usize::try_from(boundary).unwrap();
        assert_eq!(&buf[..size], &ne_bytes(comp_ty, STORED)[..]);
        assert_eq!(&buf[boundary..], &ne_bytes(comp_ty, STORED2)[..]);

        // Trapping stores.

        // All of these must trap and leave the buffer unmodified.
        assert_oob(
            call_store(boundary as u64 + 1, length, !STORED).map(|()| 0),
            "store one past the end",
        );
        assert_oob(
            call_store(length, length, !STORED).map(|()| 0),
            "store at offset == length",
        );
        assert_oob(
            call_store(size as u64, size as u64, !STORED).map(|()| 0),
            "store past a shortened length",
        );
        assert_oob(
            call_store(u64::MAX, length, !STORED).map(|()| 0),
            "store whose offset + size overflows",
        );

        // Confirm the failed stores did not corrupt the buffer.
        let buf = unsafe { (*data.get()).0 };
        assert_eq!(&buf[..size], &ne_bytes(comp_ty, STORED)[..]);
        assert_eq!(&buf[boundary..], &ne_bytes(comp_ty, STORED2)[..]);
    }

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
// These tests require signals-based traps, and we can't always enable that on
// 32-bit architectures.
#[cfg(target_pointer_width = "64")]
fn checked_native_loads_and_stores_with_spectre_mitigations() -> Result<()> {
    checked_native_loads_and_stores(true, false)?;
    checked_native_loads_and_stores(true, true)?;
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn checked_native_loads_and_stores_without_spectre_mitigations() -> Result<()> {
    checked_native_loads_and_stores(false, false)?;
    checked_native_loads_and_stores(false, true)?;
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
