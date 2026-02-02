#![cfg(not(miri))]

use wasmtime::Result;
use wasmtime::component::*;
use wasmtime::{Store, StoreContextMut, Trap};

#[test]
fn invoke_post_return() -> Result<()> {
    let component = r#"
        (component
            (import "f" (func $f))

            (core func $f_lower
                (canon lower (func $f))
            )
            (core module $m
                (import "" "" (func $f))

                (func (export "thunk"))

                (func $post_return
                    call $f)
                (export "post-return" (func $post_return))
            )
            (core instance $i (instantiate $m
                (with "" (instance
                    (export "" (func $f_lower))
                ))
            ))
            (func (export "thunk")
                (canon lift
                    (core func $i "thunk")
                    (post-return (func $i "post-return"))
                )
            )
        )
    "#;

    let engine = super::engine();
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);
    linker
        .root()
        .func_wrap("f", |_: StoreContextMut<'_, ()>, _: ()| -> Result<()> {
            unreachable!()
        })?;

    let instance = linker.instantiate(&mut store, &component)?;
    let thunk = instance.get_typed_func::<(), ()>(&mut store, "thunk")?;

    let result = thunk.call(&mut store, ());
    assert!(matches!(
        result.unwrap_err().downcast::<Trap>(),
        Ok(Trap::CannotLeaveComponent)
    ));

    Ok(())
}

#[test]
fn post_return_all_types() -> Result<()> {
    let component = r#"
        (component
            (core module $m
                (func (export "i32") (result i32)
                    i32.const 1)
                (func (export "i64") (result i64)
                    i64.const 2)
                (func (export "f32") (result f32)
                    f32.const 3)
                (func (export "f64") (result f64)
                    f64.const 4)

                (func (export "post-i32") (param i32)
                    local.get 0
                    i32.const 1
                    i32.ne
                    if unreachable end)
                (func (export "post-i64") (param i64)
                    local.get 0
                    i64.const 2
                    i64.ne
                    if unreachable end)
                (func (export "post-f32") (param f32)
                    local.get 0
                    f32.const 3
                    f32.ne
                    if unreachable end)
                (func (export "post-f64") (param f64)
                    local.get 0
                    f64.const 4
                    f64.ne
                    if unreachable end)
            )
            (core instance $i (instantiate $m))
            (func (export "i32") (result u32)
                (canon lift (core func $i "i32") (post-return (func $i "post-i32")))
            )
            (func (export "i64") (result u64)
                (canon lift (core func $i "i64") (post-return (func $i "post-i64")))
            )
            (func (export "f32") (result float32)
                (canon lift (core func $i "f32") (post-return (func $i "post-f32")))
            )
            (func (export "f64") (result float64)
                (canon lift (core func $i "f64") (post-return (func $i "post-f64")))
            )
        )
    "#;

    let engine = super::engine();
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, false);
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let i32 = instance.get_typed_func::<(), (u32,)>(&mut store, "i32")?;
    let i64 = instance.get_typed_func::<(), (u64,)>(&mut store, "i64")?;
    let f32 = instance.get_typed_func::<(), (f32,)>(&mut store, "f32")?;
    let f64 = instance.get_typed_func::<(), (f64,)>(&mut store, "f64")?;

    assert_eq!(i32.call(&mut store, ())?, (1,));

    assert_eq!(i64.call(&mut store, ())?, (2,));

    assert_eq!(f32.call(&mut store, ())?, (3.,));

    assert_eq!(f64.call(&mut store, ())?, (4.,));

    Ok(())
}

#[test]
fn post_return_string() -> Result<()> {
    let component = r#"
        (component
            (core module $m
                (memory (export "memory") 1)
                (func (export "get") (result i32)
                    (i32.store offset=0 (i32.const 8) (i32.const 100))
                    (i32.store offset=4 (i32.const 8) (i32.const 11))
                    i32.const 8
                )

                (func (export "post") (param i32)
                    local.get 0
                    i32.const 8
                    i32.ne
                    if unreachable end)

                (data (i32.const 100) "hello world")
            )
            (core instance $i (instantiate $m))
            (func (export "get") (result string)
                (canon lift
                    (core func $i "get")
                    (post-return (func $i "post"))
                    (memory $i "memory")
                )
            )
        )
    "#;

    let engine = super::engine();
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, false);
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let get = instance.get_typed_func::<(), (WasmStr,)>(&mut store, "get")?;
    let s = get.call(&mut store, ())?.0;
    assert_eq!(s.to_str(&store)?, "hello world");

    Ok(())
}

#[test]
fn trap_in_post_return_poisons_instance() -> Result<()> {
    let component = r#"
        (component
            (core module $m
                (func (export "f"))
                (func (export "post") unreachable)
            )
            (core instance $i (instantiate $m))
            (func (export "f")
                (canon lift
                    (core func $i "f")
                    (post-return (func $i "post"))
                )
            )
        )
    "#;

    let engine = super::engine();
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, ());
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let f = instance.get_typed_func::<(), ()>(&mut store, "f")?;
    let trap = f.call(&mut store, ()).unwrap_err().downcast::<Trap>()?;
    assert_eq!(trap, Trap::UnreachableCodeReached);
    let err = f.call(&mut store, ()).unwrap_err();
    assert_eq!(
        err.downcast_ref(),
        Some(&Trap::CannotEnterComponent),
        "{err}",
    );

    Ok(())
}
