#![cfg(not(miri))]

use super::{make_echo_component, make_echo_component_with_params, Param, Type};
use anyhow::Result;
use component_test_util::FuncExt;
use wasmtime::component::types::{self, Case, ComponentItem, Field};
use wasmtime::component::{Component, Linker, ResourceType, Val};
use wasmtime::{Module, Store};
use wasmtime_component_util::REALLOC_AND_FREE;

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
        err.to_string().contains("expected 1 result(s), got 0"),
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
    let input = Val::String("hello, component!".into());
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
    let input = Val::List(vec![
        Val::U32(32343),
        Val::U32(79023439),
        Val::U32(2084037802),
    ]);
    let mut output = [Val::Bool(false)];
    func.call_and_post_return(&mut store, &[input.clone()], &mut output)?;

    assert_eq!(input, output[0]);

    // Sad path: type mismatch

    let err = Val::List(vec![
        Val::U32(32343),
        Val::U32(79023439),
        Val::Float32(3.14159265),
    ]);
    let err = func
        .call_and_post_return(&mut store, &[err], &mut output)
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
    let input = Val::Record(vec![
        ("A".into(), Val::U32(32343)),
        ("B".into(), Val::Float64(3.14159265)),
        (
            "C".into(),
            Val::Record(vec![
                ("D".into(), Val::Bool(false)),
                ("E".into(), Val::U32(2084037802)),
            ]),
        ),
    ]);
    let mut output = [Val::Bool(false)];
    func.call_and_post_return(&mut store, &[input.clone()], &mut output)?;

    assert_eq!(input, output[0]);

    // Sad path: type mismatch

    let err = Val::Record(vec![
        ("A".into(), Val::S32(32343)),
        ("B".into(), Val::Float64(3.14159265)),
        (
            "C".into(),
            Val::Record(vec![
                ("D".into(), Val::Bool(false)),
                ("E".into(), Val::U32(2084037802)),
            ]),
        ),
    ]);
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let func = instance.get_func(&mut store, "echo").unwrap();
    let err = func
        .call_and_post_return(&mut store, &[err], &mut output)
        .unwrap_err();
    assert!(err.to_string().contains("type mismatch"), "{err}");

    // Sad path: too many fields

    let err = Val::Record(vec![
        ("A".into(), Val::U32(32343)),
        ("B".into(), Val::Float64(3.14159265)),
        (
            "C".into(),
            Val::Record(vec![
                ("D".into(), Val::Bool(false)),
                ("E".into(), Val::U32(2084037802)),
            ]),
        ),
        ("F".into(), Val::Bool(true)),
    ]);
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let func = instance.get_func(&mut store, "echo").unwrap();
    let err = func
        .call_and_post_return(&mut store, &[err], &mut output)
        .unwrap_err();
    assert!(
        err.to_string().contains("expected 3 fields, got 4"),
        "{err}"
    );

    // Sad path: too few fields

    let err = Val::Record(vec![
        ("A".into(), Val::U32(32343)),
        ("B".into(), Val::Float64(3.14159265)),
    ]);
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let func = instance.get_func(&mut store, "echo").unwrap();
    let err = func
        .call_and_post_return(&mut store, &[err], &mut output)
        .unwrap_err();
    assert!(
        err.to_string().contains("expected 3 fields, got 2"),
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
    let input = Val::Variant("B".into(), Some(Box::new(Val::Float64(3.14159265))));
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
    let input = Val::Variant(
        "C".into(),
        Some(Box::new(Val::Record(vec![
            ("D".into(), Val::Bool(true)),
            ("E".into(), Val::U32(314159265)),
        ]))),
    );
    func.call_and_post_return(&mut store, &[input.clone()], &mut output)?;

    assert_eq!(input, output[0]);

    // Sad path: type mismatch

    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let func = instance.get_func(&mut store, "echo").unwrap();
    let err = Val::Variant("B".into(), Some(Box::new(Val::U64(314159265))));
    let err = func
        .call_and_post_return(&mut store, &[err], &mut output)
        .unwrap_err();
    assert!(err.to_string().contains("type mismatch"), "{err}");

    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let func = instance.get_func(&mut store, "echo").unwrap();
    let err = Val::Variant("B".into(), None);
    let err = func
        .call_and_post_return(&mut store, &[err], &mut output)
        .unwrap_err();
    assert!(
        err.to_string().contains("expected a payload for case `B`"),
        "{err}"
    );

    // Sad path: unknown case

    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let func = instance.get_func(&mut store, "echo").unwrap();
    let err = Val::Variant("D".into(), Some(Box::new(Val::U64(314159265))));
    let err = func
        .call_and_post_return(&mut store, &[err], &mut output)
        .unwrap_err();
    assert!(err.to_string().contains("unknown variant case"), "{err}");

    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let func = instance.get_func(&mut store, "echo").unwrap();
    let err = Val::Variant("D".into(), None);
    let err = func
        .call_and_post_return(&mut store, &[err], &mut output)
        .unwrap_err();
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
    let input = Val::Record(vec![
        (
            "A".into(),
            Val::Variant("A".into(), Some(Box::new(Val::U32(314159265)))),
        ),
        ("B".into(), Val::U32(628318530)),
    ]);
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
    let input = Val::Flags(vec!["B".into(), "D".into()]);
    let mut output = [Val::Bool(false)];
    func.call_and_post_return(&mut store, &[input.clone()], &mut output)?;

    assert_eq!(input, output[0]);

    // Sad path: unknown flags

    let err = Val::Flags(vec!["B".into(), "D".into(), "F".into()]);
    let err = func
        .call_and_post_return(&mut store, &[err], &mut output)
        .unwrap_err();
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
    let input = Val::Record(vec![
        ("A".into(), Val::U32(32343)),
        ("B".into(), Val::Enum("b".to_string())),
        (
            "C".into(),
            Val::Record(vec![
                ("D".to_string(), Val::Bool(false)),
                ("E".to_string(), Val::U32(2084037802)),
            ]),
        ),
        (
            "F".into(),
            Val::List(vec![Val::Flags(vec!["G".to_string(), "I".to_string()])]),
        ),
        (
            "J".into(),
            Val::Variant("L".to_string(), Some(Box::new(Val::Float64(3.14159265)))),
        ),
        ("P".into(), Val::S8(42)),
        ("Q".into(), Val::S16(4242)),
        ("R".into(), Val::S32(42424242)),
        ("S".into(), Val::S64(424242424242424242)),
        ("T".into(), Val::Float32(3.14159265)),
        ("U".into(), Val::Float64(3.14159265)),
        ("V".into(), Val::String("wow, nice types".to_string())),
        ("W".into(), Val::Char('🦀')),
        ("Y".into(), Val::Tuple(vec![Val::U32(42), Val::U32(24)])),
        (
            "AA".into(),
            Val::Option(Some(Box::new(Val::U32(314159265)))),
        ),
        (
            "BB".into(),
            Val::Result(Ok(Some(Box::new(Val::String("no problem".to_string()))))),
        ),
    ]);
    let mut output = [Val::Bool(false)];
    func.call_and_post_return(&mut store, &[input.clone()], &mut output)?;

    assert_eq!(input, output[0]);

    Ok(())
}

#[test]
fn introspection() -> Result<()> {
    let engine = super::engine();

    let component = Component::new(
        &engine,
        format!(
            r#"
            (component
                (import "res" (type $res (sub resource)))

                (import "ai" (instance $i))
                (import "bi" (instance $i2 (export "m" (core module))))

                (alias export $i2 "m" (core module $m))

                (type $t (func (param "a" u32) (result u32)))
                (component $a
                  (core module $m
                    (func (export "add-five") (param i32) (result i32)
                      local.get 0
                      i32.const 5
                      i32.add)
                  )
                  (core instance $m (instantiate $m))
                  (func (export "add-five") (type $t) (canon lift (core func $m "add-five")))
                )
                (component $b
                  (import "interface-v1" (instance $i
                    (export "add-five" (func (type $t)))))
                  (core module $m
                    (func $add-five (import "interface-0.1.0" "add-five") (param i32) (result i32))
                    (func) ;; causes index out of bounds
                    (func (export "run") (result i32) i32.const 0 call $add-five)
                  )
                  (core func $add-five (canon lower (func $i "add-five")))
                  (core instance $i (instantiate 0
                    (with "interface-0.1.0" (instance
                      (export "add-five" (func $add-five))
                    ))
                  ))
                  (func (result u32) (canon lift (core func $i "run")))
                  (export "run" (func 1))
                )
                (instance $a (instantiate $a))
                (instance $b (instantiate $b (with "interface-v1" (instance $a))))
                (export "run" (func $b "run"))

                (component $c
                    (component $c
                        (export "m" (core module $m))
                    )
                    (instance $c (instantiate $c))
                    (export "i" (instance $c))
                )
                (instance $c (instantiate $c))
                (export "i" (instance $c))
                (export "r" (instance $i))
                (export "r2" (instance $i2))

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
                    (field "AA" (option u32))
                    (field "BB" (result string (error string)))
                    (field "CC" (own $res))
                ))
                (export $Foo "foo" (type $Foo'))

                (core module $m2
                    (func (export "f") (param i32) (result i32)
                        local.get 0
                    )
                    (memory (export "memory") 1)
                    {REALLOC_AND_FREE}
                )
                (core instance $i3 (instantiate $m2))

                (func (export "fn") (param "x" (option $Foo)) (result (option (tuple u32 u32)))
                    (canon lift
                        (core func $i3 "f")
                        (memory $i3 "memory")
                        (realloc (func $i3 "realloc"))
                    )
                )
            )
        "#
        ),
    )?;

    struct MyType;

    let mut linker = Linker::<()>::new(&engine);
    linker
        .root()
        .resource("res", ResourceType::host::<MyType>(), |_, _| Ok(()))?;
    linker.instance("ai")?;
    linker
        .instance("bi")?
        .module("m", &Module::new(&engine, "(module)")?)?;

    let component_ty = linker.substituted_component_type(&component)?;

    let mut imports = component_ty.imports(linker.engine());
    assert_eq!(imports.len(), 3);
    let (name, res_ty) = imports.next().unwrap();
    assert_eq!(name, "res");
    let ComponentItem::Resource(res_ty) = res_ty else {
        panic!("`res` import item of wrong type")
    };
    assert_eq!(res_ty, ResourceType::host::<MyType>());

    let (name, ai_ty) = imports.next().unwrap();
    assert_eq!(name, "ai");
    let ComponentItem::ComponentInstance(ai_ty) = ai_ty else {
        panic!("`ai` import item of wrong type")
    };
    assert_eq!(ai_ty.exports(linker.engine()).len(), 0);

    let (name, bi_ty) = imports.next().unwrap();
    assert_eq!(name, "bi");
    let ComponentItem::ComponentInstance(bi_ty) = bi_ty else {
        panic!("`bi` import item of wrong type")
    };
    let mut bi_exports = bi_ty.exports(linker.engine());
    assert_eq!(bi_exports.len(), 1);
    let (name, bi_m_ty) = bi_exports.next().unwrap();
    assert_eq!(name, "m");
    let ComponentItem::Module(bi_m_ty) = bi_m_ty else {
        panic!("`bi.m` import item of wrong type")
    };
    assert_eq!(bi_m_ty.imports(linker.engine()).len(), 0);
    assert_eq!(bi_m_ty.exports(linker.engine()).len(), 0);

    let mut exports = component_ty.exports(linker.engine());
    assert_eq!(exports.len(), 11);

    let (name, run_ty) = exports.next().unwrap();
    assert_eq!(name, "run");
    let ComponentItem::ComponentFunc(run_ty) = run_ty else {
        panic!("`run` export item of wrong type")
    };
    assert_eq!(run_ty.params().len(), 0);

    let mut run_results = run_ty.results();
    assert_eq!(run_results.len(), 1);
    assert_eq!(run_results.next().unwrap(), types::Type::U32);

    let (name, i_ty) = exports.next().unwrap();
    assert_eq!(name, "i");
    let ComponentItem::ComponentInstance(i_ty) = i_ty else {
        panic!("`i` export item of wrong type")
    };
    let mut i_ty_exports = i_ty.exports(linker.engine());
    assert_eq!(i_ty_exports.len(), 1);
    let (name, i_i_ty) = i_ty_exports.next().unwrap();
    assert_eq!(name, "i");
    let ComponentItem::ComponentInstance(i_i_ty) = i_i_ty else {
        panic!("`i.i` import item of wrong type")
    };
    let mut i_i_ty_exports = i_i_ty.exports(linker.engine());
    assert_eq!(i_i_ty_exports.len(), 1);
    let (name, i_i_m_ty) = i_i_ty_exports.next().unwrap();
    assert_eq!(name, "m");
    let ComponentItem::Module(i_i_m_ty) = i_i_m_ty else {
        panic!("`i.i.m` import item of wrong type")
    };
    assert_eq!(i_i_m_ty.imports(linker.engine()).len(), 0);
    assert_eq!(i_i_m_ty.exports(linker.engine()).len(), 0);

    let (name, r_ty) = exports.next().unwrap();
    assert_eq!(name, "r");
    let ComponentItem::ComponentInstance(r_ty) = r_ty else {
        panic!("`r` export item of wrong type")
    };
    assert_eq!(r_ty.exports(linker.engine()).len(), 0);

    let (name, r2_ty) = exports.next().unwrap();
    assert_eq!(name, "r2");
    let ComponentItem::ComponentInstance(r2_ty) = r2_ty else {
        panic!("`r2` export item of wrong type")
    };
    let mut r2_exports = r2_ty.exports(linker.engine());
    assert_eq!(r2_exports.len(), 1);
    let (name, r2_m_ty) = r2_exports.next().unwrap();
    assert_eq!(name, "m");
    let ComponentItem::Module(r2_m_ty) = r2_m_ty else {
        panic!("`r2.m` export item of wrong type")
    };
    assert_eq!(r2_m_ty.imports(linker.engine()).len(), 0);
    assert_eq!(r2_m_ty.exports(linker.engine()).len(), 0);

    let (name, b_ty) = exports.next().unwrap();
    assert_eq!(name, "b");
    let ComponentItem::Type(b_ty) = b_ty else {
        panic!("`b` export item of wrong type")
    };
    assert_eq!(b_ty.unwrap_enum().names().collect::<Vec<_>>(), ["a", "b"]);

    let (name, c_ty) = exports.next().unwrap();
    assert_eq!(name, "c");
    let ComponentItem::Type(c_ty) = c_ty else {
        panic!("`c` export item of wrong type")
    };
    let mut fields = c_ty.unwrap_record().fields();
    {
        let Field { name, ty } = fields.next().unwrap();
        assert_eq!(name, "D");
        assert_eq!(ty, types::Type::Bool);
        let Field { name, ty } = fields.next().unwrap();
        assert_eq!(name, "E");
        assert_eq!(ty, types::Type::U32);
    }

    let (name, f_ty) = exports.next().unwrap();
    assert_eq!(name, "f");
    let ComponentItem::Type(f_ty) = f_ty else {
        panic!("`f` export item of wrong type")
    };
    assert_eq!(
        f_ty.unwrap_flags().names().collect::<Vec<_>>(),
        ["G", "H", "I"]
    );

    let (name, m_ty) = exports.next().unwrap();
    assert_eq!(name, "m");
    let ComponentItem::Type(m_ty) = m_ty else {
        panic!("`m` export item of wrong type")
    };
    {
        let mut fields = m_ty.unwrap_record().fields();
        let Field { name, ty } = fields.next().unwrap();
        assert_eq!(name, "N");
        assert_eq!(ty, types::Type::Bool);
        let Field { name, ty } = fields.next().unwrap();
        assert_eq!(name, "O");
        assert_eq!(ty, types::Type::U32);
    }

    let (name, j_ty) = exports.next().unwrap();
    assert_eq!(name, "j");
    let ComponentItem::Type(j_ty) = j_ty else {
        panic!("`j` export item of wrong type")
    };
    let mut cases = j_ty.unwrap_variant().cases();
    {
        let Case { name, ty } = cases.next().unwrap();
        assert_eq!(name, "K");
        assert_eq!(ty, Some(types::Type::U32));
        let Case { name, ty } = cases.next().unwrap();
        assert_eq!(name, "L");
        assert_eq!(ty, Some(types::Type::Float64));
        let Case { name, ty } = cases.next().unwrap();
        assert_eq!(name, "M");
        assert_eq!(ty, Some(m_ty));
    }

    let (name, foo_ty) = exports.next().unwrap();
    assert_eq!(name, "foo");
    let ComponentItem::Type(foo_ty) = foo_ty else {
        panic!("`foo` export item of wrong type")
    };
    {
        let mut fields = foo_ty.unwrap_record().fields();
        assert_eq!(fields.len(), 17);
        let Field { name, ty } = fields.next().unwrap();
        assert_eq!(name, "A");
        assert_eq!(ty, types::Type::U32);
        let Field { name, ty } = fields.next().unwrap();
        assert_eq!(name, "B");
        assert_eq!(ty, b_ty);
        let Field { name, ty } = fields.next().unwrap();
        assert_eq!(name, "C");
        assert_eq!(ty, c_ty);
        let Field { name, ty } = fields.next().unwrap();
        assert_eq!(name, "F");
        let ty = ty.unwrap_list();
        assert_eq!(ty.ty(), f_ty);
        let Field { name, ty } = fields.next().unwrap();
        assert_eq!(name, "J");
        assert_eq!(ty, j_ty);
        let Field { name, ty } = fields.next().unwrap();
        assert_eq!(name, "P");
        assert_eq!(ty, types::Type::S8);
        let Field { name, ty } = fields.next().unwrap();
        assert_eq!(name, "Q");
        assert_eq!(ty, types::Type::S16);
        let Field { name, ty } = fields.next().unwrap();
        assert_eq!(name, "R");
        assert_eq!(ty, types::Type::S32);
        let Field { name, ty } = fields.next().unwrap();
        assert_eq!(name, "S");
        assert_eq!(ty, types::Type::S64);
        let Field { name, ty } = fields.next().unwrap();
        assert_eq!(name, "T");
        assert_eq!(ty, types::Type::Float32);
        let Field { name, ty } = fields.next().unwrap();
        assert_eq!(name, "U");
        assert_eq!(ty, types::Type::Float64);
        let Field { name, ty } = fields.next().unwrap();
        assert_eq!(name, "V");
        assert_eq!(ty, types::Type::String);
        let Field { name, ty } = fields.next().unwrap();
        assert_eq!(name, "W");
        assert_eq!(ty, types::Type::Char);
        let Field { name, ty } = fields.next().unwrap();
        assert_eq!(name, "Y");
        assert_eq!(
            ty.unwrap_tuple().types().collect::<Vec<_>>(),
            [types::Type::U32, types::Type::U32]
        );
        let Field { name, ty } = fields.next().unwrap();
        assert_eq!(name, "AA");
        assert_eq!(ty.unwrap_option().ty(), types::Type::U32);
        let Field { name, ty } = fields.next().unwrap();
        assert_eq!(name, "BB");
        let ty = ty.unwrap_result();
        assert_eq!(ty.ok(), Some(types::Type::String));
        assert_eq!(ty.err(), Some(types::Type::String));
        let Field { name, ty } = fields.next().unwrap();
        assert_eq!(name, "CC");
        assert_eq!(*ty.unwrap_own(), res_ty);
    }

    let (name, fn_ty) = exports.next().unwrap();
    assert_eq!(name, "fn");
    let ComponentItem::ComponentFunc(fn_ty) = fn_ty else {
        panic!("`fn` export item of wrong type")
    };
    let mut params = fn_ty.params();
    assert_eq!(params.len(), 1);
    let (name, param) = params.next().unwrap();
    assert_eq!(name, "x");
    assert_eq!(param.unwrap_option().ty(), foo_ty);

    let mut results = fn_ty.results();
    assert_eq!(results.len(), 1);
    assert_eq!(
        results
            .next()
            .unwrap()
            .unwrap_option()
            .ty()
            .unwrap_tuple()
            .types()
            .collect::<Vec<_>>(),
        [types::Type::U32, types::Type::U32]
    );
    Ok(())
}
