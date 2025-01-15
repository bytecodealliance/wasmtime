//! These tests are intended to exercise various relocation-based logic of
//! Wasmtime, especially the "jump veneer" insertion in the object-file-assembly
//! for when platform-specific relative call instructors can't always reach
//! their destination within the platform-specific limits.
//!
//! Note that the limits of AArch64 are primarily what's being stressed here
//! where the jump target for a call is 26-bits. On x86_64 the jump target is
//! 32-bits, and right now object files aren't supported larger than 4gb anyway
//! so we would need a lot of other support necessary to exercise that.

#![cfg(not(miri))]

use wasmtime::*;

const MB: usize = 1 << 20;
const PADDING: usize = if cfg!(target_pointer_width = "32") {
    1 * MB
} else {
    128 * MB
};

fn store_with_padding(padding: usize) -> Result<Store<()>> {
    let mut config = Config::new();
    // This is an internal debug-only setting specifically recognized for
    // basically just this set of tests.
    unsafe {
        config.cranelift_flag_set(
            "wasmtime_linkopt_padding_between_functions",
            &padding.to_string(),
        );
    }
    let engine = Engine::new(&config)?;
    Ok(Store::new(&engine, ()))
}

#[test]
fn forward_call_works() -> Result<()> {
    let mut store = store_with_padding(PADDING)?;
    let module = Module::new(
        store.engine(),
        r#"
            (module
                (func (export "foo") (result i32)
                    call 1)
                (func (result i32)
                    i32.const 4)
            )
        "#,
    )?;

    let i = Instance::new(&mut store, &module, &[])?;
    let foo = i.get_typed_func::<(), i32>(&mut store, "foo")?;
    assert_eq!(foo.call(&mut store, ())?, 4);
    Ok(())
}

#[test]
fn backwards_call_works() -> Result<()> {
    let mut store = store_with_padding(PADDING)?;
    let module = Module::new(
        store.engine(),
        r#"
            (module
                (func (result i32)
                    i32.const 4)
                (func (export "foo") (result i32)
                    call 0)
            )
        "#,
    )?;

    let i = Instance::new(&mut store, &module, &[])?;
    let foo = i.get_typed_func::<(), i32>(&mut store, "foo")?;
    assert_eq!(foo.call(&mut store, ())?, 4);
    Ok(())
}

#[test]
fn mixed() -> Result<()> {
    test_many_call_module(store_with_padding(MB)?)
}

#[test]
fn mixed_forced() -> Result<()> {
    let mut config = Config::new();
    unsafe {
        config.cranelift_flag_set("wasmtime_linkopt_force_jump_veneer", "true");
    }
    let engine = Engine::new(&config)?;
    test_many_call_module(Store::new(&engine, ()))
}

fn test_many_call_module(mut store: Store<()>) -> Result<()> {
    const N: i32 = 200;

    let mut wat = String::new();
    wat.push_str("(module\n");
    wat.push_str("(func $first (result i32) (i32.const 1))\n");
    for i in 0..N {
        wat.push_str(&format!("(func (export \"{i}\") (result i32 i32)\n"));
        wat.push_str("call $first\n");
        wat.push_str(&format!("i32.const {i}\n"));
        wat.push_str("i32.add\n");
        wat.push_str("call $last\n");
        wat.push_str(&format!("i32.const {i}\n"));
        wat.push_str("i32.add)\n");
    }
    wat.push_str("(func $last (result i32) (i32.const 2))\n");
    wat.push_str(")\n");

    let module = Module::new(store.engine(), &wat)?;

    let instance = Instance::new(&mut store, &module, &[])?;

    for i in 0..N {
        let name = i.to_string();
        let func = instance.get_typed_func::<(), (i32, i32)>(&mut store, &name)?;
        let (a, b) = func.call(&mut store, ())?;
        assert_eq!(a, i + 1);
        assert_eq!(b, i + 2);
    }
    Ok(())
}
