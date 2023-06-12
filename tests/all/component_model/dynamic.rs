#![cfg(not(miri))]

use super::{make_echo_component, make_echo_component_with_params, Param, Type};
use anyhow::Result;
use component_test_util::FuncExt;
use wasmtime::component::{self, Component, Linker, Val};
use wasmtime::Store;

#[test]
fn primitives() -> Result<()> {
    let engine = super::engine();
    let mut store = Store::new(&engine, ());
    let mut output = [Val::Bool(false)];

    for (input, ty, param) in [
        (Val::Bool(true), "bool", Param(Type::U8, Some(0))),
        (Val::S8(-42), "s8", Param(Type::S8, Some(0))),
        (Val::U8(42), "u8", Param(Type::U8, Some(0))),
        (Val::S16(-4242), "s16", Param(Type::S16, Some(0))),
        (Val::U16(4242), "u16", Param(Type::U16, Some(0))),
        (Val::S32(-314159265), "s32", Param(Type::I32, Some(0))),
        (Val::U32(314159265), "u32", Param(Type::I32, Some(0))),
        (Val::S64(-31415926535897), "s64", Param(Type::I64, Some(0))),
        (Val::U64(31415926535897), "u64", Param(Type::I64, Some(0))),
        (
            Val::Float32(3.14159265),
            "float32",
            Param(Type::F32, Some(0)),
        ),
        (
            Val::Float64(3.14159265),
            "float64",
            Param(Type::F64, Some(0)),
        ),
        (Val::Char('🦀'), "char", Param(Type::I32, Some(0))),
    ] {
        let component = Component::new(&engine, make_echo_component_with_params(ty, &[param]))?;
        let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
        let func = instance.get_func(&mut store, "echo").unwrap();
        func.call_and_post_return(&mut store, &[input.clone()], &mut output)?;

        assert_eq!(input, output[0]);
    }

    // Sad path: type mismatch

    let component = Component::new(
        &engine,
        make_echo_component_with_params("float64", &[Param(Type::F64, Some(0))]),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let func = instance.get_func(&mut store, "echo").unwrap();
    let err = func
        .call_and_post_return(&mut store, &[Val::U64(42)], &mut output)
        .unwrap_err();

    assert!(err.to_string().contains("type mismatch"), "{err}");

    // Sad path: arity mismatch (too many)

    let err = func
        .call_and_post_return(
            &mut store,
            &[Val::Float64(3.14159265), Val::Float64(3.14159265)],
            &mut output,
        )
        .unwrap_err();

    assert!(
        err.to_string().contains("expected 1 argument(s), got 2"),
        "{err}"
    );

    // Sad path: arity mismatch (too few)

    let err = func
        .call_and_post_return(&mut store, &[], &mut output)
        .unwrap_err();
    assert!(
        err.to_string().contains("expected 1 argument(s), got 0"),
        "{err}"
    );

    let err = func
        .call_and_post_return(&mut store, &output, &mut [])
        .unwrap_err();
    assert!(
        err.to_string().contains("expected 1 results(s), got 0"),
        "{err}"
    );

    Ok(())
}

#[test]
fn strings() -> Result<()> {
    let engine = super::engine();
    let mut store = Store::new(&engine, ());

    let component = Component::new(&engine, make_echo_component("string", 8))?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let func = instance.get_func(&mut store, "echo").unwrap();
    let input = Val::String(Box::from("hello, component!"));
    let mut output = [Val::Bool(false)];
    func.call_and_post_return(&mut store, &[input.clone()], &mut output)?;
    assert_eq!(input, output[0]);

    Ok(())
}

#[test]
fn lists() -> Result<()> {
    let engine = super::engine();
    let mut store = Store::new(&engine, ());

    let component = Component::new(&engine, make_echo_component("(list u32)", 8))?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let func = instance.get_func(&mut store, "echo").unwrap();
    let ty = &func.params(&store)[0];
    let input = ty.unwrap_list().new_val(Box::new([
        Val::U32(32343),
        Val::U32(79023439),
        Val::U32(2084037802),
    ]))?;
    let mut output = [Val::Bool(false)];
    func.call_and_post_return(&mut store, &[input.clone()], &mut output)?;

    assert_eq!(input, output[0]);

    // Sad path: type mismatch

    let err = ty
        .unwrap_list()
        .new_val(Box::new([
            Val::U32(32343),
            Val::U32(79023439),
            Val::Float32(3.14159265),
        ]))
        .unwrap_err();

    assert!(err.to_string().contains("type mismatch"), "{err}");

    Ok(())
}

#[test]
fn records() -> Result<()> {
    let engine = super::engine();
    let mut store = Store::new(&engine, ());

    let component = Component::new(
        &engine,
        make_echo_component_with_params(
            r#"
                (type $c' (record
                    (field "D" bool)
                    (field "E" u32)
                ))
                (export $c "c" (type $c'))
                (type $Foo' (record
                    (field "A" u32)
                    (field "B" float64)
                    (field "C" $c)
                ))
            "#,
            &[
                Param(Type::I32, Some(0)),
                Param(Type::F64, Some(8)),
                Param(Type::U8, Some(16)),
                Param(Type::I32, Some(20)),
            ],
        ),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let func = instance.get_func(&mut store, "echo").unwrap();
    let ty = &func.params(&store)[0];
    let inner_type = &ty.unwrap_record().fields().nth(2).unwrap().ty;
    let input = ty.unwrap_record().new_val([
        ("A", Val::U32(32343)),
        ("B", Val::Float64(3.14159265)),
        (
            "C",
            inner_type
                .unwrap_record()
                .new_val([("D", Val::Bool(false)), ("E", Val::U32(2084037802))])?,
        ),
    ])?;
    let mut output = [Val::Bool(false)];
    func.call_and_post_return(&mut store, &[input.clone()], &mut output)?;

    assert_eq!(input, output[0]);

    // Sad path: type mismatch

    let err = ty
        .unwrap_record()
        .new_val([
            ("A", Val::S32(32343)),
            ("B", Val::Float64(3.14159265)),
            (
                "C",
                inner_type
                    .unwrap_record()
                    .new_val([("D", Val::Bool(false)), ("E", Val::U32(2084037802))])?,
            ),
        ])
        .unwrap_err();

    assert!(err.to_string().contains("type mismatch"), "{err}");

    // Sad path: too many fields

    let err = ty
        .unwrap_record()
        .new_val([
            ("A", Val::U32(32343)),
            ("B", Val::Float64(3.14159265)),
            (
                "C",
                inner_type
                    .unwrap_record()
                    .new_val([("D", Val::Bool(false)), ("E", Val::U32(2084037802))])?,
            ),
            ("F", Val::Bool(true)),
        ])
        .unwrap_err();

    assert!(
        err.to_string().contains("expected 3 value(s); got 4"),
        "{err}"
    );

    // Sad path: too few fields

    let err = ty
        .unwrap_record()
        .new_val([("A", Val::U32(32343)), ("B", Val::Float64(3.14159265))])
        .unwrap_err();

    assert!(
        err.to_string().contains("expected 3 value(s); got 2"),
        "{err}"
    );

    Ok(())
}

#[test]
fn variants() -> Result<()> {
    let engine = super::engine();
    let mut store = Store::new(&engine, ());

    let fragment = r#"
                (type $c' (record (field "D" bool) (field "E" u32)))
                (export $c "c" (type $c'))
                (type $Foo' (variant
                    (case "A" u32)
                    (case "B" float64)
                    (case "C" $c)
                ))
            "#;

    let component = Component::new(
        &engine,
        make_echo_component_with_params(
            fragment,
            &[
                Param(Type::U8, Some(0)),
                Param(Type::I64, Some(8)),
                Param(Type::I32, None),
            ],
        ),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let func = instance.get_func(&mut store, "echo").unwrap();
    let ty = &func.params(&store)[0];
    let input = ty
        .unwrap_variant()
        .new_val("B", Some(Val::Float64(3.14159265)))?;
    let mut output = [Val::Bool(false)];
    func.call_and_post_return(&mut store, &[input.clone()], &mut output)?;

    assert_eq!(input, output[0]);

    // Do it again, this time using case "C"

    let component = Component::new(
        &engine,
        make_echo_component_with_params(
            fragment,
            &[
                Param(Type::U8, Some(0)),
                Param(Type::I64, Some(8)),
                Param(Type::I32, Some(12)),
            ],
        ),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let func = instance.get_func(&mut store, "echo").unwrap();
    let ty = &func.params(&store)[0];
    let c_type = &ty.unwrap_variant().cases().nth(2).unwrap().ty.unwrap();
    let input = ty.unwrap_variant().new_val(
        "C",
        Some(
            c_type
                .unwrap_record()
                .new_val([("D", Val::Bool(true)), ("E", Val::U32(314159265))])?,
        ),
    )?;
    func.call_and_post_return(&mut store, &[input.clone()], &mut output)?;

    assert_eq!(input, output[0]);

    // Sad path: type mismatch

    let err = ty
        .unwrap_variant()
        .new_val("B", Some(Val::U64(314159265)))
        .unwrap_err();
    assert!(err.to_string().contains("type mismatch"), "{err}");
    let err = ty.unwrap_variant().new_val("B", None).unwrap_err();
    assert!(
        err.to_string().contains("expected a payload for case `B`"),
        "{err}"
    );

    // Sad path: unknown case

    let err = ty
        .unwrap_variant()
        .new_val("D", Some(Val::U64(314159265)))
        .unwrap_err();
    assert!(err.to_string().contains("unknown variant case"), "{err}");
    let err = ty.unwrap_variant().new_val("D", None).unwrap_err();
    assert!(err.to_string().contains("unknown variant case"), "{err}");

    // Make sure we lift variants which have cases of different sizes with the correct alignment

    let component = Component::new(
        &engine,
        make_echo_component_with_params(
            r#"
                (type $c' (record (field "D" bool) (field "E" u32)))
                (export $c "c" (type $c'))
                (type $a' (variant
                    (case "A" u32)
                    (case "B" float64)
                    (case "C" $c)
                ))
                (export $a "a" (type $a'))
                (type $Foo' (record
                    (field "A" $a)
                    (field "B" u32)
                ))
            "#,
            &[
                Param(Type::U8, Some(0)),
                Param(Type::I64, Some(8)),
                Param(Type::I32, None),
                Param(Type::I32, Some(16)),
            ],
        ),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let func = instance.get_func(&mut store, "echo").unwrap();
    let ty = &func.params(&store)[0];
    let a_type = &ty.unwrap_record().fields().nth(0).unwrap().ty;
    let input = ty.unwrap_record().new_val([
        (
            "A",
            a_type
                .unwrap_variant()
                .new_val("A", Some(Val::U32(314159265)))?,
        ),
        ("B", Val::U32(628318530)),
    ])?;
    func.call_and_post_return(&mut store, &[input.clone()], &mut output)?;

    assert_eq!(input, output[0]);

    Ok(())
}

#[test]
fn flags() -> Result<()> {
    let engine = super::engine();
    let mut store = Store::new(&engine, ());

    let component = Component::new(
        &engine,
        make_echo_component_with_params(
            r#"(flags "A" "B" "C" "D" "E")"#,
            &[Param(Type::U8, Some(0))],
        ),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let func = instance.get_func(&mut store, "echo").unwrap();
    let ty = &func.params(&store)[0];
    let input = ty.unwrap_flags().new_val(&["B", "D"])?;
    let mut output = [Val::Bool(false)];
    func.call_and_post_return(&mut store, &[input.clone()], &mut output)?;

    assert_eq!(input, output[0]);

    // Sad path: unknown flags

    let err = ty.unwrap_flags().new_val(&["B", "D", "F"]).unwrap_err();

    assert!(err.to_string().contains("unknown flag"), "{err}");

    Ok(())
}

#[test]
fn everything() -> Result<()> {
    // This serves to test both nested types and storing parameters on the heap (i.e. exceeding `MAX_STACK_PARAMS`)

    let engine = super::engine();
    let mut store = Store::new(&engine, ());

    let component = Component::new(
        &engine,
        make_echo_component_with_params(
            r#"
                (type $b' (enum "a" "b"))
                (export $b "b" (type $b'))
                (type $c' (record (field "D" bool) (field "E" u32)))
                (export $c "c" (type $c'))
                (type $f' (flags "G" "H" "I"))
                (export $f "f" (type $f'))
                (type $m' (record (field "N" bool) (field "O" u32)))
                (export $m "m" (type $m'))
                (type $j' (variant
                    (case "K" u32)
                    (case "L" float64)
                    (case "M" $m)
                ))
                (export $j "j" (type $j'))
                (type $z' (union u32 float64))
                (export $z "z" (type $z'))

                (type $Foo' (record
                    (field "A" u32)
                    (field "B" $b)
                    (field "C" $c)
                    (field "F" (list $f))
                    (field "J" $j)
                    (field "P" s8)
                    (field "Q" s16)
                    (field "R" s32)
                    (field "S" s64)
                    (field "T" float32)
                    (field "U" float64)
                    (field "V" string)
                    (field "W" char)
                    (field "Y" (tuple u32 u32))
                    (field "Z" $z)
                    (field "AA" (option u32))
                    (field "BB" (result string (error string)))
                ))
            "#,
            &[
                Param(Type::I32, Some(0)),
                Param(Type::U8, Some(4)),
                Param(Type::U8, Some(5)),
                Param(Type::I32, Some(8)),
                Param(Type::I32, Some(12)),
                Param(Type::I32, Some(16)),
                Param(Type::U8, Some(20)),
                Param(Type::I64, Some(28)),
                Param(Type::I32, Some(32)),
                Param(Type::S8, Some(36)),
                Param(Type::S16, Some(38)),
                Param(Type::I32, Some(40)),
                Param(Type::I64, Some(48)),
                Param(Type::F32, Some(56)),
                Param(Type::F64, Some(64)),
                Param(Type::I32, Some(72)),
                Param(Type::I32, Some(76)),
                Param(Type::I32, Some(80)),
                Param(Type::I32, Some(84)),
                Param(Type::I32, Some(88)),
                Param(Type::I64, Some(96)),
                Param(Type::U8, Some(104)),
                Param(Type::I32, Some(108)),
                Param(Type::U8, Some(112)),
                Param(Type::I32, Some(116)),
                Param(Type::I32, Some(120)),
            ],
        ),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let func = instance.get_func(&mut store, "echo").unwrap();
    let ty = &func.params(&store)[0];
    let types = ty
        .unwrap_record()
        .fields()
        .map(|field| field.ty)
        .collect::<Box<[component::Type]>>();
    let (b_type, c_type, f_type, j_type, y_type, z_type, aa_type, bb_type) = (
        &types[1], &types[2], &types[3], &types[4], &types[13], &types[14], &types[15], &types[16],
    );
    let f_element_type = &f_type.unwrap_list().ty();
    let input = ty.unwrap_record().new_val([
        ("A", Val::U32(32343)),
        ("B", b_type.unwrap_enum().new_val("b")?),
        (
            "C",
            c_type
                .unwrap_record()
                .new_val([("D", Val::Bool(false)), ("E", Val::U32(2084037802))])?,
        ),
        (
            "F",
            f_type.unwrap_list().new_val(Box::new([f_element_type
                .unwrap_flags()
                .new_val(&["G", "I"])?]))?,
        ),
        (
            "J",
            j_type
                .unwrap_variant()
                .new_val("L", Some(Val::Float64(3.14159265)))?,
        ),
        ("P", Val::S8(42)),
        ("Q", Val::S16(4242)),
        ("R", Val::S32(42424242)),
        ("S", Val::S64(424242424242424242)),
        ("T", Val::Float32(3.14159265)),
        ("U", Val::Float64(3.14159265)),
        ("V", Val::String(Box::from("wow, nice types"))),
        ("W", Val::Char('🦀')),
        (
            "Y",
            y_type
                .unwrap_tuple()
                .new_val(Box::new([Val::U32(42), Val::U32(24)]))?,
        ),
        (
            "Z",
            z_type.unwrap_union().new_val(1, Val::Float64(3.14159265))?,
        ),
        (
            "AA",
            aa_type.unwrap_option().new_val(Some(Val::U32(314159265)))?,
        ),
        (
            "BB",
            bb_type
                .unwrap_result()
                .new_val(Ok(Some(Val::String(Box::from("no problem")))))?,
        ),
    ])?;
    let mut output = [Val::Bool(false)];
    func.call_and_post_return(&mut store, &[input.clone()], &mut output)?;

    assert_eq!(input, output[0]);

    Ok(())
}
