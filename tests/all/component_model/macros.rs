use super::TypedFuncExt;
use anyhow::Result;
use component_macro_test::add_variants;
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

    // Happy path: component type matches field count, names, and types

    let component = Component::new(
        &engine,
        make_echo_component(
            r#"(type $Foo (record (field "foo-bar-baz" s32) (field "b" u32)))"#,
            8,
        ),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;

    let input = Foo { a: -42, b: 73 };
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

    // Happy path redux, with generics this time

    #[derive(ComponentType, Lift, Lower, PartialEq, Eq, Debug, Copy, Clone)]
    #[component(record)]
    struct Generic<A, B> {
        #[component(name = "foo-bar-baz")]
        a: A,
        b: B,
    }

    let input = Generic {
        a: -43_i32,
        b: 74_u32,
    };

    let component = Component::new(
        &engine,
        make_echo_component(
            r#"(type $Foo (record (field "foo-bar-baz" s32) (field "b" u32)))"#,
            8,
        ),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;

    let output = instance
        .get_typed_func::<(Generic<i32, u32>,), Generic<i32, u32>, _>(&mut store, "echo")?
        .call_and_post_return(&mut store, (input,))?;

    assert_eq!(input, output);

    Ok(())
}

#[test]
fn union_derive() -> Result<()> {
    #[derive(ComponentType, Lift, Lower, PartialEq, Debug, Copy, Clone)]
    #[component(union)]
    enum Foo {
        A(i32),
        B(u32),
        C(i32),
    }

    let engine = super::engine();
    let mut store = Store::new(&engine, ());

    // Happy path: component type matches case count and types

    let component = Component::new(
        &engine,
        make_echo_component(r#"(type $Foo (union s32 u32 s32))"#, 8),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let func = instance.get_typed_func::<(Foo,), Foo, _>(&mut store, "echo")?;

    for &input in &[Foo::A(-42), Foo::B(73), Foo::C(314159265)] {
        let output = func.call_and_post_return(&mut store, (input,))?;

        assert_eq!(input, output);
    }

    // Sad path: case count mismatch (too few)

    let component = Component::new(
        &engine,
        make_echo_component(r#"(type $Foo (union s32 u32))"#, 8),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;

    assert!(instance
        .get_typed_func::<(Foo,), Foo, _>(&mut store, "echo")
        .is_err());

    // Sad path: case count mismatch (too many)

    let component = Component::new(
        &engine,
        make_echo_component(r#"(type $Foo (union s32 u32 s32 s32))"#, 8),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;

    assert!(instance
        .get_typed_func::<(Foo,), Foo, _>(&mut store, "echo")
        .is_err());

    assert!(instance
        .get_typed_func::<(Foo,), Foo, _>(&mut store, "echo")
        .is_err());

    // Sad path: case type mismatch

    let component = Component::new(
        &engine,
        make_echo_component(r#"(type $Foo (union s32 s32 s32))"#, 8),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;

    assert!(instance
        .get_typed_func::<(Foo,), Foo, _>(&mut store, "echo")
        .is_err());

    // Happy path redux, with generics this time

    #[derive(ComponentType, Lift, Lower, PartialEq, Debug, Copy, Clone)]
    #[component(union)]
    enum Generic<A, B, C> {
        A(A),
        B(B),
        C(C),
    }

    let component = Component::new(
        &engine,
        make_echo_component(r#"(type $Foo (union s32 u32 s32))"#, 8),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let func = instance.get_typed_func::<(Generic<i32, u32, i32>,), Generic<i32, u32, i32>, _>(
        &mut store, "echo",
    )?;

    for &input in &[
        Generic::<i32, u32, i32>::A(-42),
        Generic::B(73),
        Generic::C(314159265),
    ] {
        let output = func.call_and_post_return(&mut store, (input,))?;

        assert_eq!(input, output);
    }

    Ok(())
}

#[test]
fn variant_derive() -> Result<()> {
    #[derive(ComponentType, Lift, Lower, PartialEq, Eq, Debug, Copy, Clone)]
    #[component(variant)]
    enum Foo {
        #[component(name = "foo-bar-baz")]
        A(i32),
        B(u32),
        C,
    }

    let engine = super::engine();
    let mut store = Store::new(&engine, ());

    // Happy path: component type matches case count, names, and types

    let component = Component::new(
        &engine,
        make_echo_component(
            r#"(type $Foo (variant (case "foo-bar-baz" s32) (case "B" u32) (case "C" unit)))"#,
            8,
        ),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let func = instance.get_typed_func::<(Foo,), Foo, _>(&mut store, "echo")?;

    for &input in &[Foo::A(-42), Foo::B(73), Foo::C] {
        let output = func.call_and_post_return(&mut store, (input,))?;

        assert_eq!(input, output);
    }

    // Sad path: case count mismatch (too few)

    let component = Component::new(
        &engine,
        make_echo_component(
            r#"(type $Foo (variant (case "foo-bar-baz" s32) (case "B" u32)))"#,
            8,
        ),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;

    assert!(instance
        .get_typed_func::<(Foo,), Foo, _>(&mut store, "echo")
        .is_err());

    // Sad path: case count mismatch (too many)

    let component = Component::new(
        &engine,
        make_echo_component(
            r#"(type $Foo (variant (case "foo-bar-baz" s32) (case "B" u32) (case "C" unit) (case "D" u32)))"#,
            8,
        ),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;

    assert!(instance
        .get_typed_func::<(Foo,), Foo, _>(&mut store, "echo")
        .is_err());

    // Sad path: case name mismatch

    let component = Component::new(
        &engine,
        make_echo_component(
            r#"(type $Foo (variant (case "A" s32) (case "B" u32) (case "C" unit)))"#,
            8,
        ),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;

    assert!(instance
        .get_typed_func::<(Foo,), Foo, _>(&mut store, "echo")
        .is_err());

    // Sad path: case type mismatch

    let component = Component::new(
        &engine,
        make_echo_component(
            r#"(type $Foo (variant (case "foo-bar-baz" s32) (case "B" s32) (case "C" unit)))"#,
            8,
        ),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;

    assert!(instance
        .get_typed_func::<(Foo,), Foo, _>(&mut store, "echo")
        .is_err());

    // Happy path redux, with generics this time

    #[derive(ComponentType, Lift, Lower, PartialEq, Eq, Debug, Copy, Clone)]
    #[component(variant)]
    enum Generic<A, B> {
        #[component(name = "foo-bar-baz")]
        A(A),
        B(B),
        C,
    }

    let component = Component::new(
        &engine,
        make_echo_component(
            r#"(type $Foo (variant (case "foo-bar-baz" s32) (case "B" u32) (case "C" unit)))"#,
            8,
        ),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let func = instance
        .get_typed_func::<(Generic<i32, u32>,), Generic<i32, u32>, _>(&mut store, "echo")?;

    for &input in &[Generic::<i32, u32>::A(-42), Generic::B(73), Generic::C] {
        let output = func.call_and_post_return(&mut store, (input,))?;

        assert_eq!(input, output);
    }

    Ok(())
}

#[test]
fn enum_derive() -> Result<()> {
    #[derive(ComponentType, Lift, Lower, PartialEq, Eq, Debug, Copy, Clone)]
    #[component(enum)]
    enum Foo {
        #[component(name = "foo-bar-baz")]
        A,
        B,
        C,
    }

    let engine = super::engine();
    let mut store = Store::new(&engine, ());

    // Happy path: component type matches case count and names

    let component = Component::new(
        &engine,
        make_echo_component(r#"(type $Foo (enum "foo-bar-baz" "B" "C"))"#, 4),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let func = instance.get_typed_func::<(Foo,), Foo, _>(&mut store, "echo")?;

    for &input in &[Foo::A, Foo::B, Foo::C] {
        let output = func.call_and_post_return(&mut store, (input,))?;

        assert_eq!(input, output);
    }

    // Sad path: case count mismatch (too few)

    let component = Component::new(
        &engine,
        make_echo_component(r#"(type $Foo (enum "foo-bar-baz" "B"))"#, 4),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;

    assert!(instance
        .get_typed_func::<(Foo,), Foo, _>(&mut store, "echo")
        .is_err());

    // Sad path: case count mismatch (too many)

    let component = Component::new(
        &engine,
        make_echo_component(r#"(type $Foo (enum "foo-bar-baz" "B" "C" "D"))"#, 4),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;

    assert!(instance
        .get_typed_func::<(Foo,), Foo, _>(&mut store, "echo")
        .is_err());

    // Sad path: case name mismatch

    let component = Component::new(
        &engine,
        make_echo_component(r#"(type $Foo (enum "A" "B" "C"))"#, 4),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;

    assert!(instance
        .get_typed_func::<(Foo,), Foo, _>(&mut store, "echo")
        .is_err());

    #[add_variants(257)]
    #[derive(ComponentType, Lift, Lower, PartialEq, Eq, Debug, Copy, Clone)]
    #[component(enum)]
    enum Many {}

    let component = Component::new(
        &engine,
        make_echo_component(
            &format!(
                r#"(type $Foo (enum {}))"#,
                (0..257)
                    .map(|index| format!(r#""V{}""#, index))
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            4,
        ),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let func = instance.get_typed_func::<(Many,), Many, _>(&mut store, "echo")?;

    for &input in &[Many::V0, Many::V1, Many::V254, Many::V255, Many::V256] {
        let output = func.call_and_post_return(&mut store, (input,))?;

        assert_eq!(input, output);
    }

    // TODO: The following case takes forever (i.e. I gave up after 30 minutes) to compile; we'll need to profile
    // the compiler to find out why, which may point the way to a more efficient option.  On the other hand, this
    // may not be worth spending time on.  Enums with over 2^16 variants are rare enough.

    // #[add_variants(65537)]
    // #[derive(ComponentType, Lift, Lower, PartialEq, Eq, Debug, Copy, Clone)]
    // #[component(enum)]
    // enum ManyMore {}

    Ok(())
}
