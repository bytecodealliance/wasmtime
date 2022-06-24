use super::TypedFuncExt;
use anyhow::Result;
use std::fmt::Write;
use wasmtime::component::{Component, ComponentType, Lift, Linker, Lower};
use wasmtime::Store;

fn make_echo_component(type_definition: &str, type_size: u32) -> String {
    if type_size <= 4 {
        format!(
            r#"
            (component
                (core module $m
                    (func (export "echo") (param i32) (result i32)
                        local.get 0
                    )

                    (memory (export "memory") 1)
                )

                (core instance $i (instantiate $m))

                {}

                (func (export "echo") (param $Foo) (result $Foo)
                    (canon lift (core func $i "echo") (memory $i "memory"))
                )
            )"#,
            type_definition
        )
    } else {
        let mut params = String::new();
        let mut store = String::new();

        for index in 0..(type_size / 4) {
            params.push_str(" i32");
            write!(
                &mut store,
                "(i32.store offset={} (local.get $base) (local.get {}))",
                index * 4,
                index,
            )
            .unwrap();
        }

        format!(
            r#"
            (component
                (core module $m
                    (func (export "echo") (param{}) (result i32)
                        (local $base i32)
                        (local.set $base (i32.const 0))
                        {}
                        local.get $base
                    )

                    (memory (export "memory") 1)
                )

                (core instance $i (instantiate $m))

                {}

                (func (export "echo") (param $Foo) (result $Foo)
                    (canon lift (core func $i "echo") (memory $i "memory"))
                )
            )"#,
            params, store, type_definition
        )
    }
}

#[test]
fn record_derive() -> Result<()> {
    #[derive(ComponentType, Lift, Lower, PartialEq, Eq, Debug, Copy, Clone)]
    #[component(record)]
    struct Foo {
        #[component(name = "foo-bar-baz")]
        a: i32,
        b: u32,
    }

    let engine = super::engine();
    let mut store = Store::new(&engine, ());
    let input = Foo { a: -42, b: 73 };

    // Happy path: component type matches field count, names, and types

    let component = Component::new(
        &engine,
        make_echo_component(
            r#"(type $Foo (record (field "foo-bar-baz" s32) (field "b" u32)))"#,
            8,
        ),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;

    let output = instance
        .get_typed_func::<(Foo,), Foo, _>(&mut store, "echo")?
        .call_and_post_return(&mut store, (input,))?;

    assert_eq!(input, output);

    // Sad path: field count mismatch (too few)

    let component = Component::new(
        &engine,
        make_echo_component(r#"(type $Foo (record (field "foo-bar-baz" s32)))"#, 4),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;

    assert!(instance
        .get_typed_func::<(Foo,), Foo, _>(&mut store, "echo")
        .is_err());

    // Sad path: field count mismatch (too many)

    let component = Component::new(
        &engine,
        make_echo_component(
            r#"(type $Foo (record (field "foo-bar-baz" s32) (field "b" u32) (field "c" u32)))"#,
            12,
        ),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;

    assert!(instance
        .get_typed_func::<(Foo,), Foo, _>(&mut store, "echo")
        .is_err());

    // Sad path: field name mismatch

    let component = Component::new(
        &engine,
        make_echo_component(r#"(type $Foo (record (field "a" s32) (field "b" u32)))"#, 8),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;

    assert!(instance
        .get_typed_func::<(Foo,), Foo, _>(&mut store, "echo")
        .is_err());

    // Sad path: field type mismatch

    let component = Component::new(
        &engine,
        make_echo_component(
            r#"(type $Foo (record (field "foo-bar-baz" s32) (field "b" s32)))"#,
            8,
        ),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;

    assert!(instance
        .get_typed_func::<(Foo,), Foo, _>(&mut store, "echo")
        .is_err());

    Ok(())
}
