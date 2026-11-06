#![cfg(not(miri))]

use super::make_echo_component;
use wasmtime::Result;
use wasmtime::component::{
    Component, ComponentType, FixedHostHeapUsage, HostHeapUsage, Lift, Linker, Lower,
};
use wasmtime::{Engine, Store};
use wasmtime_test_macros::{add_variants, flags_test};

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
        make_echo_component(r#"(record (field "foo-bar-baz" s32) (field "b" u32))"#, 8),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;

    let input = Foo { a: -42, b: 73 };
    let output = instance
        .get_typed_func::<(Foo,), (Foo,)>(&mut store, "echo")?
        .call(&mut store, (input,))?;

    assert_eq!((input,), output);

    // Sad path: field count mismatch (too few)

    let component = Component::new(
        &engine,
        make_echo_component(r#"(record (field "foo-bar-baz" s32))"#, 4),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;

    assert!(
        instance
            .get_typed_func::<(Foo,), (Foo,)>(&mut store, "echo")
            .is_err()
    );

    // Sad path: field count mismatch (too many)

    let component = Component::new(
        &engine,
        make_echo_component(
            r#"(record (field "foo-bar-baz" s32) (field "b" u32) (field "c" u32))"#,
            12,
        ),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;

    assert!(
        instance
            .get_typed_func::<(Foo,), (Foo,)>(&mut store, "echo")
            .is_err()
    );

    // Sad path: field name mismatch

    let component = Component::new(
        &engine,
        make_echo_component(r#"(record (field "a" s32) (field "b" u32))"#, 8),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;

    assert!(
        instance
            .get_typed_func::<(Foo,), (Foo,)>(&mut store, "echo")
            .is_err()
    );

    // Sad path: field type mismatch

    let component = Component::new(
        &engine,
        make_echo_component(r#"(record (field "foo-bar-baz" s32) (field "b" s32))"#, 8),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;

    assert!(
        instance
            .get_typed_func::<(Foo,), (Foo,)>(&mut store, "echo")
            .is_err()
    );

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
        make_echo_component(r#"(record (field "foo-bar-baz" s32) (field "b" u32))"#, 8),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;

    let output = instance
        .get_typed_func::<(Generic<i32, u32>,), (Generic<i32, u32>,)>(&mut store, "echo")?
        .call(&mut store, (input,))?;

    assert_eq!((input,), output);

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
            r#"(variant (case "foo-bar-baz" s32) (case "B" u32) (case "C"))"#,
            8,
        ),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let func = instance.get_typed_func::<(Foo,), (Foo,)>(&mut store, "echo")?;

    for &input in &[Foo::A(-42), Foo::B(73), Foo::C] {
        let output = func.call(&mut store, (input,))?;

        assert_eq!((input,), output);
    }

    // Sad path: case count mismatch (too few)

    let component = Component::new(
        &engine,
        make_echo_component(r#"(variant (case "foo-bar-baz" s32) (case "B" u32))"#, 8),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;

    assert!(
        instance
            .get_typed_func::<(Foo,), (Foo,)>(&mut store, "echo")
            .is_err()
    );

    // Sad path: case count mismatch (too many)

    let component = Component::new(
        &engine,
        make_echo_component(
            r#"(variant (case "foo-bar-baz" s32) (case "B" u32) (case "C") (case "D" u32))"#,
            8,
        ),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;

    assert!(
        instance
            .get_typed_func::<(Foo,), (Foo,)>(&mut store, "echo")
            .is_err()
    );

    // Sad path: case name mismatch

    let component = Component::new(
        &engine,
        make_echo_component(r#"(variant (case "A" s32) (case "B" u32) (case "C"))"#, 8),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;

    assert!(
        instance
            .get_typed_func::<(Foo,), (Foo,)>(&mut store, "echo")
            .is_err()
    );

    // Sad path: case type mismatch

    let component = Component::new(
        &engine,
        make_echo_component(
            r#"(variant (case "foo-bar-baz" s32) (case "B" s32) (case "C"))"#,
            8,
        ),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;

    assert!(
        instance
            .get_typed_func::<(Foo,), (Foo,)>(&mut store, "echo")
            .is_err()
    );

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
            r#"(variant (case "foo-bar-baz" s32) (case "B" u32) (case "C"))"#,
            8,
        ),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let func = instance
        .get_typed_func::<(Generic<i32, u32>,), (Generic<i32, u32>,)>(&mut store, "echo")?;

    for &input in &[Generic::<i32, u32>::A(-42), Generic::B(73), Generic::C] {
        let output = func.call(&mut store, (input,))?;

        assert_eq!((input,), output);
    }

    Ok(())
}

#[test]
fn enum_derive() -> Result<()> {
    #[derive(ComponentType, Lift, Lower, PartialEq, Eq, Debug, Copy, Clone)]
    #[component(enum)]
    #[repr(u8)]
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
        make_echo_component(r#"(enum "foo-bar-baz" "B" "C")"#, 4),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let func = instance.get_typed_func::<(Foo,), (Foo,)>(&mut store, "echo")?;

    for &input in &[Foo::A, Foo::B, Foo::C] {
        let output = func.call(&mut store, (input,))?;

        assert_eq!((input,), output);
    }

    // Sad path: case count mismatch (too few)

    let component = Component::new(
        &engine,
        make_echo_component(r#"(enum "foo-bar-baz" "B")"#, 4),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;

    assert!(
        instance
            .get_typed_func::<(Foo,), (Foo,)>(&mut store, "echo")
            .is_err()
    );

    // Sad path: case count mismatch (too many)

    let component = Component::new(
        &engine,
        make_echo_component(r#"(enum "foo-bar-baz" "B" "C" "D")"#, 4),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;

    assert!(
        instance
            .get_typed_func::<(Foo,), (Foo,)>(&mut store, "echo")
            .is_err()
    );

    // Sad path: case name mismatch

    let component = Component::new(&engine, make_echo_component(r#"(enum "A" "B" "C")"#, 4))?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;

    assert!(
        instance
            .get_typed_func::<(Foo,), (Foo,)>(&mut store, "echo")
            .is_err()
    );

    // Happy path redux, with large enums (i.e. more than 2^8 cases)

    #[add_variants(257)]
    #[derive(ComponentType, Lift, Lower, PartialEq, Eq, Debug, Copy, Clone)]
    #[component(enum)]
    #[repr(u16)]
    enum Many {}

    let component = Component::new(
        &engine,
        make_echo_component(
            &format!(
                "(enum {})",
                (0..257)
                    .map(|index| format!(r#""V{index}""#))
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            4,
        ),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let func = instance.get_typed_func::<(Many,), (Many,)>(&mut store, "echo")?;

    for &input in &[Many::V0, Many::V1, Many::V254, Many::V255, Many::V256] {
        let output = func.call(&mut store, (input,))?;

        assert_eq!((input,), output);
    }

    // TODO: The following case takes forever (i.e. I gave up after 30 minutes) to compile; we'll need to profile
    // the compiler to find out why, which may point the way to a more efficient option.  On the other hand, this
    // may not be worth spending time on.  Enums with over 2^16 variants are rare enough.

    // #[add_variants(65537)]
    // #[derive(ComponentType, Lift, Lower, PartialEq, Eq, Debug, Copy, Clone)]
    // #[component(enum)]
    // #[repr(u32)]
    // enum ManyMore {}

    Ok(())
}

#[test]
fn flags() -> Result<()> {
    let config = wasmtime_test_util::component::config();
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());

    // Simple 8-bit flags
    wasmtime::component::flags! {
        Foo {
            #[component(name = "foo-bar-baz")]
            const A;
            const B;
            const C;
        }
    }

    assert_eq!(Foo::default(), (Foo::A | Foo::B) & Foo::C);
    assert_eq!(Foo::B, (Foo::A | Foo::B) & Foo::B);
    assert_eq!(Foo::A, (Foo::A | Foo::B) & Foo::A);
    assert_eq!(Foo::A | Foo::B, Foo::A ^ Foo::B);
    assert_eq!(Foo::default(), Foo::A ^ Foo::A);
    assert_eq!(Foo::B | Foo::C, !Foo::A);

    // Happy path: component type matches flag count and names

    let component = Component::new(
        &engine,
        make_echo_component(r#"(flags "foo-bar-baz" "B" "C")"#, 4),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let func = instance.get_typed_func::<(Foo,), (Foo,)>(&mut store, "echo")?;

    for n in 0..8 {
        let mut input = Foo::default();
        if (n & 1) != 0 {
            input |= Foo::A;
        }
        if (n & 2) != 0 {
            input |= Foo::B;
        }
        if (n & 4) != 0 {
            input |= Foo::C;
        }

        let output = func.call(&mut store, (input,))?;

        assert_eq!((input,), output);
    }

    // Sad path: flag count mismatch (too few)

    let component = Component::new(
        &engine,
        make_echo_component(r#"(flags "foo-bar-baz" "B")"#, 4),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;

    assert!(
        instance
            .get_typed_func::<(Foo,), (Foo,)>(&mut store, "echo")
            .is_err()
    );

    // Sad path: flag count mismatch (too many)

    let component = Component::new(
        &engine,
        make_echo_component(r#"(flags "foo-bar-baz" "B" "C" "D")"#, 4),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;

    assert!(
        instance
            .get_typed_func::<(Foo,), (Foo,)>(&mut store, "echo")
            .is_err()
    );

    // Sad path: flag name mismatch

    let component = Component::new(&engine, make_echo_component(r#"(flags "A" "B" "C")"#, 4))?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;

    assert!(
        instance
            .get_typed_func::<(Foo,), (Foo,)>(&mut store, "echo")
            .is_err()
    );

    // Happy path redux, with large flag count (exactly 8)

    flags_test!(Foo8Exact, 8);

    assert_eq!(
        Foo8Exact::default(),
        (Foo8Exact::F0 | Foo8Exact::F6) & Foo8Exact::F7
    );
    assert_eq!(
        Foo8Exact::F6,
        (Foo8Exact::F0 | Foo8Exact::F6) & Foo8Exact::F6
    );
    assert_eq!(
        Foo8Exact::F0,
        (Foo8Exact::F0 | Foo8Exact::F6) & Foo8Exact::F0
    );
    assert_eq!(Foo8Exact::F0 | Foo8Exact::F6, Foo8Exact::F0 ^ Foo8Exact::F6);
    assert_eq!(Foo8Exact::default(), Foo8Exact::F0 ^ Foo8Exact::F0);
    assert_eq!(
        Foo8Exact::F1
            | Foo8Exact::F2
            | Foo8Exact::F3
            | Foo8Exact::F4
            | Foo8Exact::F5
            | Foo8Exact::F6
            | Foo8Exact::F7,
        !Foo8Exact::F0
    );

    let component = Component::new(
        &engine,
        make_echo_component(
            &format!(
                r#"(flags {})"#,
                (0..8)
                    .map(|index| format!(r#""F{index}""#))
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            4,
        ),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let func = instance.get_typed_func::<(Foo8Exact,), (Foo8Exact,)>(&mut store, "echo")?;

    for &input in &[
        Foo8Exact::F0,
        Foo8Exact::F1,
        Foo8Exact::F5,
        Foo8Exact::F6,
        Foo8Exact::F7,
    ] {
        let output = func.call(&mut store, (input,))?;

        assert_eq!((input,), output);
    }

    // Happy path redux, with large flag count (more than 8)

    flags_test!(Foo16, 9);

    assert_eq!(Foo16::default(), (Foo16::F0 | Foo16::F7) & Foo16::F8);
    assert_eq!(Foo16::F7, (Foo16::F0 | Foo16::F7) & Foo16::F7);
    assert_eq!(Foo16::F0, (Foo16::F0 | Foo16::F7) & Foo16::F0);
    assert_eq!(Foo16::F0 | Foo16::F7, Foo16::F0 ^ Foo16::F7);
    assert_eq!(Foo16::default(), Foo16::F0 ^ Foo16::F0);
    assert_eq!(
        Foo16::F1
            | Foo16::F2
            | Foo16::F3
            | Foo16::F4
            | Foo16::F5
            | Foo16::F6
            | Foo16::F7
            | Foo16::F8,
        !Foo16::F0
    );

    let component = Component::new(
        &engine,
        make_echo_component(
            &format!(
                "(flags {})",
                (0..9)
                    .map(|index| format!(r#""F{index}""#))
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            4,
        ),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let func = instance.get_typed_func::<(Foo16,), (Foo16,)>(&mut store, "echo")?;

    for &input in &[Foo16::F0, Foo16::F1, Foo16::F6, Foo16::F7, Foo16::F8] {
        let output = func.call(&mut store, (input,))?;

        assert_eq!((input,), output);
    }

    // Happy path redux, with large flag count (exactly 16)

    flags_test!(Foo16Exact, 16);

    assert_eq!(
        Foo16Exact::default(),
        (Foo16Exact::F0 | Foo16Exact::F14) & Foo16Exact::F5
    );
    assert_eq!(
        Foo16Exact::F14,
        (Foo16Exact::F0 | Foo16Exact::F14) & Foo16Exact::F14
    );
    assert_eq!(
        Foo16Exact::F0,
        (Foo16Exact::F0 | Foo16Exact::F14) & Foo16Exact::F0
    );
    assert_eq!(
        Foo16Exact::F0 | Foo16Exact::F14,
        Foo16Exact::F0 ^ Foo16Exact::F14
    );
    assert_eq!(Foo16Exact::default(), Foo16Exact::F0 ^ Foo16Exact::F0);
    assert_eq!(
        Foo16Exact::F0 | Foo16Exact::F15,
        !((!Foo16Exact::F0) & (!Foo16Exact::F15))
    );

    let component = Component::new(
        &engine,
        make_echo_component(
            &format!(
                r#"(flags {})"#,
                (0..16)
                    .map(|index| format!(r#""F{index}""#))
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            4,
        ),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let func = instance.get_typed_func::<(Foo16Exact,), (Foo16Exact,)>(&mut store, "echo")?;

    for &input in &[
        Foo16Exact::F0,
        Foo16Exact::F1,
        Foo16Exact::F13,
        Foo16Exact::F14,
        Foo16Exact::F15,
    ] {
        let output = func.call(&mut store, (input,))?;

        assert_eq!((input,), output);
    }

    // Happy path redux, with large flag count (more than 16)

    flags_test!(Foo32, 17);

    assert_eq!(Foo32::default(), (Foo32::F0 | Foo32::F15) & Foo32::F16);
    assert_eq!(Foo32::F15, (Foo32::F0 | Foo32::F15) & Foo32::F15);
    assert_eq!(Foo32::F0, (Foo32::F0 | Foo32::F15) & Foo32::F0);
    assert_eq!(Foo32::F0 | Foo32::F15, Foo32::F0 ^ Foo32::F15);
    assert_eq!(Foo32::default(), Foo32::F0 ^ Foo32::F0);
    assert_eq!(Foo32::F0 | Foo32::F16, !((!Foo32::F0) & (!Foo32::F16)));

    let component = Component::new(
        &engine,
        make_echo_component(
            &format!(
                "(flags {})",
                (0..17)
                    .map(|index| format!(r#""F{index}""#))
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            4,
        ),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let func = instance.get_typed_func::<(Foo32,), (Foo32,)>(&mut store, "echo")?;

    for &input in &[Foo32::F0, Foo32::F1, Foo32::F14, Foo32::F15, Foo32::F16] {
        let output = func.call(&mut store, (input,))?;

        assert_eq!((input,), output);
    }

    // Happy path redux, with large flag count (exactly 32)

    flags_test!(Foo32Exact, 32);

    assert_eq!(
        Foo32Exact::default(),
        (Foo32Exact::F0 | Foo32Exact::F30) & Foo32Exact::F31
    );
    assert_eq!(
        Foo32Exact::F30,
        (Foo32Exact::F0 | Foo32Exact::F30) & Foo32Exact::F30
    );
    assert_eq!(
        Foo32Exact::F0,
        (Foo32Exact::F0 | Foo32Exact::F30) & Foo32Exact::F0
    );
    assert_eq!(
        Foo32Exact::F0 | Foo32Exact::F30,
        Foo32Exact::F0 ^ Foo32Exact::F30
    );
    assert_eq!(Foo32Exact::default(), Foo32Exact::F0 ^ Foo32Exact::F0);
    assert_eq!(
        Foo32Exact::F0 | Foo32Exact::F15,
        !((!Foo32Exact::F0) & (!Foo32Exact::F15))
    );

    let component = Component::new(
        &engine,
        make_echo_component(
            &format!(
                r#"(flags {})"#,
                (0..32)
                    .map(|index| format!(r#""F{index}""#))
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            4,
        ),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let func = instance.get_typed_func::<(Foo32Exact,), (Foo32Exact,)>(&mut store, "echo")?;

    for &input in &[
        Foo32Exact::F0,
        Foo32Exact::F1,
        Foo32Exact::F29,
        Foo32Exact::F30,
        Foo32Exact::F31,
    ] {
        let output = func.call(&mut store, (input,))?;

        assert_eq!((input,), output);
    }

    Ok(())
}

// Compile-time checks that a derive produced the expected marker trait impl.
fn assert_fixed<T: FixedHostHeapUsage>() {}
fn assert_heap_usage<T: HostHeapUsage>() {}

#[test]
#[allow(dead_code, reason = "fields only tested via derive")]
fn host_heap_usage_derive_fixed() {
    #[derive(FixedHostHeapUsage)]
    struct FixedStruct {
        a: u32,
        b: bool,
    }

    // `Option`/`Result` of fixed types are fixed.
    #[derive(FixedHostHeapUsage)]
    struct FixedOptions(Option<u64>, Result<u8, ()>);

    // Arrays and tuples of fixed types are fixed.
    #[derive(FixedHostHeapUsage)]
    struct FixedArrTuple([u8; 16], (u32, u64));

    assert_fixed::<FixedStruct>();
    assert_fixed::<FixedOptions>();
    assert_fixed::<FixedArrTuple>();
    assert_eq!(FixedStruct { a: 1, b: true }.heap_size_of_children(), 0);
}

#[test]
#[allow(dead_code, reason = "fields only tested via derive")]
fn host_heap_usage_derive_struct_and_enum() {
    #[derive(FixedHostHeapUsage)]
    struct FixedStruct {
        a: u32,
    }

    #[derive(HostHeapUsage)]
    struct DynamicStruct {
        fixed: FixedStruct,
        name: String,
        numbers: Vec<u32>,
    }

    // Fixed fields contribute nothing; dynamic fields are summed.
    let s = DynamicStruct {
        fixed: FixedStruct { a: 10 },
        name: String::from("hello"),
        numbers: vec![1, 2, 3, 4],
    };
    assert_eq!(
        s.heap_size_of_children(),
        s.name.heap_size_of_children() + s.numbers.heap_size_of_children()
    );

    #[derive(HostHeapUsage)]
    enum DynamicEnum {
        Empty,
        Data(Vec<u8>),
    }

    assert_eq!(DynamicEnum::Empty.heap_size_of_children(), 0);
    assert_eq!(DynamicEnum::Data(vec![0; 128]).heap_size_of_children(), 128);

    // Uninhabited enums are supported.
    #[derive(HostHeapUsage)]
    enum Void {}
    assert_heap_usage::<Void>();
}

#[test]
#[allow(dead_code, reason = "fields only tested via derive")]
fn host_heap_usage_derive_generics() {
    #[derive(HostHeapUsage)]
    enum Generic<T, U> {
        Named { items: Vec<T> },
        Phantom(core::marker::PhantomData<U>),
    }

    // Bounds are on field types, so `U`, which has no tracked field, is
    // unconstrained and need not implement `HostHeapUsage`.
    struct NotTracked;
    let g = Generic::<u8, NotTracked>::Named { items: vec![0; 16] };
    assert_eq!(g.heap_size_of_children(), 16);
}

#[test]
#[allow(dead_code, reason = "fields only tested via derive")]
fn host_heap_usage_derive_recursive_and_composite() {
    // Recursive types still derive and delegate to their fields.
    #[derive(HostHeapUsage)]
    struct GNode<T> {
        value: T,
        children: Vec<GNode<T>>,
    }
    let tree = GNode {
        value: String::from("root"),
        children: vec![GNode {
            value: String::from("leaf"),
            children: Vec::new(),
        }],
    };
    assert_eq!(
        tree.heap_size_of_children(),
        tree.value.heap_size_of_children() + tree.children.heap_size_of_children()
    );

    // Dynamic types nested in tuples, arrays, and `Vec`s are all counted.
    #[derive(HostHeapUsage)]
    struct DynamicArrTuple {
        pairs: Vec<(String, Vec<u8>)>,
        names: [String; 2],
    }
    let dat = DynamicArrTuple {
        pairs: vec![(String::from("k"), vec![1, 2, 3])],
        names: [String::from("ab"), String::from("cde")],
    };
    assert_eq!(
        dat.heap_size_of_children(),
        dat.pairs.heap_size_of_children() + dat.names.heap_size_of_children()
    );
}
