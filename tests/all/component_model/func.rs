#![cfg(not(miri))]

use super::{ApiStyle, REALLOC_AND_FREE};
use bytes::{Bytes, BytesMut};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering::SeqCst},
};
use wasmtime::Result;
use wasmtime::component::*;
use wasmtime::{Config, Engine, Store, StoreContextMut, Trap};

const CANON_32BIT_NAN: u32 = 0b01111111110000000000000000000000;

#[test]
fn typecheck() -> Result<()> {
    let component = r#"
        (component
            (core module $m
                (func (export "thunk"))
                (func (export "take-string") (param i32 i32))
                (func (export "two-args") (param i32 i32 i32))
                (func (export "ret-one") (result i32) unreachable)

                (memory (export "memory") 1)
                (func (export "realloc") (param i32 i32 i32 i32) (result i32)
                    unreachable)
            )
            (core instance $i (instantiate (module $m)))
            (func (export "thunk")
                (canon lift (core func $i "thunk"))
            )
            (func (export "take-string") (param "a" string)
                (canon lift (core func $i "take-string") (memory (core memory $i "memory")) (realloc (core func $i "realloc")))
            )
            (func (export "take-two-args") (param "a" s32) (param "b" (list u8))
                (canon lift (core func $i "two-args") (memory (core memory $i "memory")) (realloc (core func $i "realloc")))
            )
            (func (export "ret-tuple") (result (tuple u8 s8))
                (canon lift (core func $i "ret-one") (memory (core memory $i "memory")) (realloc (core func $i "realloc")))
            )
            (func (export "ret-tuple1") (result (tuple u32))
                (canon lift (core func $i "ret-one") (memory (core memory $i "memory")) (realloc (core func $i "realloc")))
            )
            (func (export "ret-string") (result string)
                (canon lift (core func $i "ret-one") (memory (core memory $i "memory")) (realloc (core func $i "realloc")))
            )
            (func (export "ret-list-u8") (result (list u8))
                (canon lift (core func $i "ret-one") (memory (core memory $i "memory")) (realloc (core func $i "realloc")))
            )
        )
    "#;

    let engine = Engine::default();
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, ());
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let thunk = instance.get_func(&mut store, "thunk").unwrap();
    let take_string = instance.get_func(&mut store, "take-string").unwrap();
    let take_two_args = instance.get_func(&mut store, "take-two-args").unwrap();
    let ret_tuple = instance.get_func(&mut store, "ret-tuple").unwrap();
    let ret_tuple1 = instance.get_func(&mut store, "ret-tuple1").unwrap();
    let ret_string = instance.get_func(&mut store, "ret-string").unwrap();
    let ret_list_u8 = instance.get_func(&mut store, "ret-list-u8").unwrap();
    assert!(thunk.typed::<(), (u32,)>(&store).is_err());
    assert!(thunk.typed::<(u32,), ()>(&store).is_err());
    assert!(thunk.typed::<(), ()>(&store).is_ok());
    assert!(take_string.typed::<(), ()>(&store).is_err());
    assert!(take_string.typed::<(String,), ()>(&store).is_ok());
    assert!(take_string.typed::<(&str,), ()>(&store).is_ok());
    assert!(take_string.typed::<(&[u8],), ()>(&store).is_err());
    assert!(take_two_args.typed::<(), ()>(&store).is_err());
    assert!(take_two_args.typed::<(i32, &[u8]), (u32,)>(&store).is_err());
    assert!(take_two_args.typed::<(u32, &[u8]), ()>(&store).is_err());
    assert!(take_two_args.typed::<(i32, &[u8]), ()>(&store).is_ok());
    assert!(take_two_args.typed::<(i32, Bytes), ()>(&store).is_ok());
    assert!(take_two_args.typed::<(i32, BytesMut), ()>(&store).is_ok());
    assert!(ret_tuple.typed::<(), ()>(&store).is_err());
    assert!(ret_tuple.typed::<(), (u8,)>(&store).is_err());
    assert!(ret_tuple.typed::<(), ((u8, i8),)>(&store).is_ok());
    assert!(ret_tuple1.typed::<(), ((u32,),)>(&store).is_ok());
    assert!(ret_tuple1.typed::<(), (u32,)>(&store).is_err());
    assert!(ret_string.typed::<(), ()>(&store).is_err());
    assert!(ret_string.typed::<(), (WasmStr,)>(&store).is_ok());
    assert!(ret_list_u8.typed::<(), (WasmList<u16>,)>(&store).is_err());
    assert!(ret_list_u8.typed::<(), (WasmList<i8>,)>(&store).is_err());
    assert!(ret_list_u8.typed::<(), (WasmList<u8>,)>(&store).is_ok());
    assert!(ret_list_u8.typed::<(), (Bytes,)>(&store).is_ok());
    assert!(ret_list_u8.typed::<(), (BytesMut,)>(&store).is_ok());

    Ok(())
}

#[test]
fn type_layers() -> Result<()> {
    let component = r#"
        (component
            (core module $m
                (func (export "take-i32-100") (param i32)
                    local.get 0
                    i32.const 2
                    i32.eq
                    br_if 0
                    unreachable
                )
            )
            (core instance $i (instantiate $m))
            (func (export "take-u32") (param "a" u32) (canon lift (core func $i "take-i32-100")))
        )
    "#;

    let engine = super::engine();
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, ());
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;

    instance
        .get_typed_func::<(Box<u32>,), ()>(&mut store, "take-u32")?
        .call(&mut store, (Box::new(2),))?;
    instance
        .get_typed_func::<(&u32,), ()>(&mut store, "take-u32")?
        .call(&mut store, (&2,))?;
    instance
        .get_typed_func::<(Arc<u32>,), ()>(&mut store, "take-u32")?
        .call(&mut store, (Arc::new(2),))?;
    instance
        .get_typed_func::<(&Box<Arc<Box<u32>>>,), ()>(&mut store, "take-u32")?
        .call(&mut store, (&Box::new(Arc::new(Box::new(2))),))?;

    Ok(())
}

#[test]
fn strings() -> Result<()> {
    let component = format!(
        r#"(component
            (core module $m
                (memory (export "memory") 1)
                (func (export "roundtrip") (param i32 i32) (result i32)
                    (local $base i32)
                    (local.set $base
                        (call $realloc
                            (i32.const 0)
                            (i32.const 0)
                            (i32.const 4)
                            (i32.const 8)))
                    (i32.store offset=0
                        (local.get $base)
                        (local.get 0))
                    (i32.store offset=4
                        (local.get $base)
                        (local.get 1))
                    (local.get $base)
                )

                {REALLOC_AND_FREE}
            )
            (core instance $i (instantiate $m))

            (func (export "list8-to-str") (param "a" (list u8)) (result string)
                (canon lift
                    (core func $i "roundtrip")
                    (memory (core memory $i "memory"))
                    (realloc (core func $i "realloc"))
                )
            )
            (func (export "str-to-list8") (param "a" string) (result (list u8))
                (canon lift
                    (core func $i "roundtrip")
                    (memory (core memory $i "memory"))
                    (realloc (core func $i "realloc"))
                )
            )
            (func (export "list16-to-str") (param "a" (list u16)) (result string)
                (canon lift
                    (core func $i "roundtrip")
                    string-encoding=utf16
                    (memory (core memory $i "memory"))
                    (realloc (core func $i "realloc"))
                )
            )
            (func (export "str-to-list16") (param "a" string) (result (list u16))
                (canon lift
                    (core func $i "roundtrip")
                    string-encoding=utf16
                    (memory (core memory $i "memory"))
                    (realloc (core func $i "realloc"))
                )
            )
        )"#
    );

    let engine = super::engine();
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, ());
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let list8_to_str =
        instance.get_typed_func::<(&[u8],), (WasmStr,)>(&mut store, "list8-to-str")?;
    let str_to_list8 =
        instance.get_typed_func::<(&str,), (WasmList<u8>,)>(&mut store, "str-to-list8")?;
    let list16_to_str =
        instance.get_typed_func::<(&[u16],), (WasmStr,)>(&mut store, "list16-to-str")?;
    let str_to_list16 =
        instance.get_typed_func::<(&str,), (WasmList<u16>,)>(&mut store, "str-to-list16")?;

    let mut roundtrip = |x: &str| -> Result<()> {
        let ret = list8_to_str.call(&mut store, (x.as_bytes(),))?.0;
        assert_eq!(ret.to_str(&store)?, x);

        let utf16 = x.encode_utf16().collect::<Vec<_>>();
        let ret = list16_to_str.call(&mut store, (&utf16[..],))?.0;
        assert_eq!(ret.to_str(&store)?, x);

        let ret = str_to_list8.call(&mut store, (x,))?.0;
        assert_eq!(
            ret.iter(&mut store)?.collect::<Result<Vec<_>>>()?,
            x.as_bytes()
        );

        let ret = str_to_list16.call(&mut store, (x,))?.0;
        assert_eq!(ret.iter(&mut store)?.collect::<Result<Vec<_>>>()?, utf16,);

        Ok(())
    };

    roundtrip("")?;
    roundtrip("foo")?;
    roundtrip("hello there")?;
    roundtrip("💝")?;
    roundtrip("Löwe 老虎 Léopard")?;

    let ret = list8_to_str.call(&mut store, (b"\xff",))?.0;
    let err = ret.to_str(&store).unwrap_err();
    assert!(err.to_string().contains("invalid utf-8"), "{}", err);

    let ret = list8_to_str
        .call(&mut store, (b"hello there \xff invalid",))?
        .0;
    let err = ret.to_str(&store).unwrap_err();
    assert!(err.to_string().contains("invalid utf-8"), "{}", err);

    let ret = list16_to_str.call(&mut store, (&[0xd800],))?.0;
    let err = ret.to_str(&store).unwrap_err();
    assert!(err.to_string().contains("unpaired surrogate"), "{}", err);

    let ret = list16_to_str.call(&mut store, (&[0xdfff],))?.0;
    let err = ret.to_str(&store).unwrap_err();
    assert!(err.to_string().contains("unpaired surrogate"), "{}", err);

    let ret = list16_to_str.call(&mut store, (&[0xd800, 0xff00],))?.0;
    let err = ret.to_str(&store).unwrap_err();
    assert!(err.to_string().contains("unpaired surrogate"), "{}", err);

    Ok(())
}

#[tokio::test]
async fn many_parameters() -> Result<()> {
    test_many_parameters(false, false).await
}

#[tokio::test]
async fn many_parameters_concurrent() -> Result<()> {
    test_many_parameters(false, true).await
}

#[tokio::test]
async fn many_parameters_dynamic_concurrent() -> Result<()> {
    test_many_parameters(true, true).await
}

async fn test_many_parameters(dynamic: bool, concurrent: bool) -> Result<()> {
    let (body, async_opts) = if concurrent {
        (
            r#"
                    (call $task-return
                        (i32.const 0)
                        (i32.mul
                            (memory.size)
                            (i32.const 65536)
                        )
                        (local.get 0)
                    )

                    (i32.const 0)
            "#,
            r#"async (callback (core func $i "callback"))"#,
        )
    } else {
        (
            r#"
                    (local $base i32)

                    ;; Allocate space for the return
                    (local.set $base
                        (call $realloc
                            (i32.const 0)
                            (i32.const 0)
                            (i32.const 4)
                            (i32.const 12)))

                    ;; Store the pointer/length of the entire linear memory
                    ;; so we have access to everything.
                    (i32.store offset=0
                        (local.get $base)
                        (i32.const 0))
                    (i32.store offset=4
                        (local.get $base)
                        (i32.mul
                            (memory.size)
                            (i32.const 65536)))

                    ;; And also store our pointer parameter
                    (i32.store offset=8
                        (local.get $base)
                        (local.get 0))

                    (local.get $base)
            "#,
            "",
        )
    };

    let component = format!(
        r#"(component
            (core module $libc
                (memory (export "memory") 1)

                {REALLOC_AND_FREE}
            )
            (core instance $libc (instantiate $libc))
            (core module $m
                (import "libc" "memory" (memory 1))
                (import "libc" "realloc" (func $realloc (param i32 i32 i32 i32) (result i32)))
                (import "" "task.return" (func $task-return (param i32 i32 i32)))
                (func (export "foo") (param i32) (result i32)
                    {body}
                )
                (func (export "callback") (param i32 i32 i32) (result i32) unreachable)
            )
            (type $tuple (tuple (list u8) u32))
            (core func $task-return (canon task.return
                (result $tuple)
                (memory (core memory $libc "memory"))
            ))
            (core instance $i (instantiate $m
                (with "" (instance (export "task.return" (func $task-return))))
                (with "libc" (instance $libc))
            ))

            (type $t (func async
                (param "p1" s8)              ;; offset  0, size 1
                (param "p2" u64)             ;; offset  8, size 8
                (param "p3" float32)         ;; offset 16, size 4
                (param "p4" u8)              ;; offset 20, size 1
                (param "p5" s16)             ;; offset 22, size 2
                (param "p6" string)          ;; offset 24, size 8
                (param "p7" (list u32))      ;; offset 32, size 8
                (param "p8" bool)            ;; offset 40, size 1
                (param "p9" bool)            ;; offset 41, size 1
                (param "p0" char)            ;; offset 44, size 4
                (param "pa" (list bool))     ;; offset 48, size 8
                (param "pb" (list char))     ;; offset 56, size 8
                (param "pc" (list string))   ;; offset 64, size 8

                (result $tuple)
            ))
            (func (export "many-param") (type $t)
                (canon lift
                    (core func $i "foo")
                    (memory (core memory $libc "memory"))
                    (realloc (core func $libc "realloc"))
                    {async_opts}
                )
            )
        )"#
    );

    let mut config = Config::new();
    config.wasm_component_model_async(true);
    let engine = &Engine::new(&config)?;
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, ());

    let instance = Linker::new(&engine)
        .instantiate_async(&mut store, &component)
        .await?;

    let input = (
        -100,
        u64::MAX / 2,
        f32::from_bits(CANON_32BIT_NAN | 1),
        38,
        18831,
        "this is the first string",
        [1, 2, 3, 4, 5, 6, 7, 8].as_slice(),
        true,
        false,
        '🚩',
        [false, true, false, true, true].as_slice(),
        ['🍌', '🥐', '🍗', '🍙', '🍡'].as_slice(),
        [
            "the quick",
            "brown fox",
            "was too lazy",
            "to jump over the dog",
            "what a demanding dog",
        ]
        .as_slice(),
    );

    let (memory, pointer) = if dynamic {
        let input = vec![
            Val::S8(input.0),
            Val::U64(input.1),
            Val::Float32(input.2),
            Val::U8(input.3),
            Val::S16(input.4),
            Val::String(input.5.into()),
            Val::List(input.6.iter().copied().map(Val::U32).collect()),
            Val::Bool(input.7),
            Val::Bool(input.8),
            Val::Char(input.9),
            Val::List(input.10.iter().copied().map(Val::Bool).collect()),
            Val::List(input.11.iter().copied().map(Val::Char).collect()),
            Val::List(input.12.iter().map(|&s| Val::String(s.into())).collect()),
        ];
        let func = instance.get_func(&mut store, "many-param").unwrap();

        let mut results = vec![Val::Bool(false)];
        if concurrent {
            store
                .run_concurrent(async |store| {
                    func.call_concurrent(store, &input, &mut results).await?;
                    wasmtime::error::Ok(())
                })
                .await??;
        } else {
            func.call_async(&mut store, &input, &mut results).await?;
        };
        let mut results = results.into_iter();
        let Some(Val::Tuple(results)) = results.next() else {
            panic!()
        };
        let mut results = results.into_iter();
        let Some(Val::List(memory)) = results.next() else {
            panic!()
        };
        let Some(Val::U32(pointer)) = results.next() else {
            panic!()
        };
        (
            memory
                .into_iter()
                .map(|v| if let Val::U8(v) = v { v } else { panic!() })
                .collect(),
            pointer,
        )
    } else {
        let func = instance.get_typed_func::<(
            i8,
            u64,
            f32,
            u8,
            i16,
            &str,
            &[u32],
            bool,
            bool,
            char,
            &[bool],
            &[char],
            &[&str],
        ), ((Vec<u8>, u32),)>(&mut store, "many-param")?;

        if concurrent {
            store
                .run_concurrent(async move |accessor| {
                    wasmtime::error::Ok(func.call_concurrent(accessor, input).await?)
                })
                .await??
                .0
        } else {
            func.call_async(&mut store, input).await?.0
        }
    };
    let memory = &memory[..];

    let mut actual = &memory[pointer as usize..][..72];
    assert_eq!(i8::from_le_bytes(*actual.take_n::<1>()), input.0);
    actual.skip::<7>();
    assert_eq!(u64::from_le_bytes(*actual.take_n::<8>()), input.1);
    assert_eq!(
        u32::from_le_bytes(*actual.take_n::<4>()),
        CANON_32BIT_NAN | 1
    );
    assert_eq!(u8::from_le_bytes(*actual.take_n::<1>()), input.3);
    actual.skip::<1>();
    assert_eq!(i16::from_le_bytes(*actual.take_n::<2>()), input.4);
    assert_eq!(actual.ptr_len(memory, 1), input.5.as_bytes());
    let mut mem = actual.ptr_len(memory, 4);
    for expected in input.6.iter() {
        assert_eq!(u32::from_le_bytes(*mem.take_n::<4>()), *expected);
    }
    assert!(mem.is_empty());
    assert_eq!(actual.take_n::<1>(), &[input.7 as u8]);
    assert_eq!(actual.take_n::<1>(), &[input.8 as u8]);
    actual.skip::<2>();
    assert_eq!(u32::from_le_bytes(*actual.take_n::<4>()), input.9 as u32);

    // (list bool)
    mem = actual.ptr_len(memory, 1);
    for expected in input.10.iter() {
        assert_eq!(mem.take_n::<1>(), &[*expected as u8]);
    }
    assert!(mem.is_empty());

    // (list char)
    mem = actual.ptr_len(memory, 4);
    for expected in input.11.iter() {
        assert_eq!(u32::from_le_bytes(*mem.take_n::<4>()), *expected as u32);
    }
    assert!(mem.is_empty());

    // (list string)
    mem = actual.ptr_len(memory, 8);
    for expected in input.12.iter() {
        let actual = mem.ptr_len(memory, 1);
        assert_eq!(actual, expected.as_bytes());
    }
    assert!(mem.is_empty());
    assert!(actual.is_empty());

    Ok(())
}

#[tokio::test]
async fn many_results() -> Result<()> {
    test_many_results(false, false).await
}

#[tokio::test]
async fn many_results_concurrent() -> Result<()> {
    test_many_results(false, true).await
}

#[tokio::test]
async fn many_results_dynamic_concurrent() -> Result<()> {
    test_many_results(true, true).await
}

async fn test_many_results(dynamic: bool, concurrent: bool) -> Result<()> {
    let (ret, async_opts) = if concurrent {
        (
            r#"
                   call $task-return
                   i32.const 0
            "#,
            r#"async (callback (core func $i "callback"))"#,
        )
    } else {
        ("", "")
    };

    let my_nan = CANON_32BIT_NAN | 1;

    let component = format!(
        r#"(component
            (core module $libc
                (memory (export "memory") 1)

                {REALLOC_AND_FREE}
            )
            (core instance $libc (instantiate $libc))
            (core module $m
                (import "libc" "memory" (memory 1))
                (import "libc" "realloc" (func $realloc (param i32 i32 i32 i32) (result i32)))
                (import "" "task.return" (func $task-return (param i32)))
                (func (export "foo") (result i32)
                    (local $base i32)
                    (local $string i32)
                    (local $list i32)

                    (local.set $base
                        (call $realloc
                            (i32.const 0)
                            (i32.const 0)
                            (i32.const 8)
                            (i32.const 72)))

                    (i32.store8 offset=0
                        (local.get $base)
                        (i32.const -100))

                    (i64.store offset=8
                        (local.get $base)
                        (i64.const 9223372036854775807))

                    (f32.store offset=16
                        (local.get $base)
                        (f32.reinterpret_i32 (i32.const {my_nan})))

                    (i32.store8 offset=20
                        (local.get $base)
                        (i32.const 38))

                    (i32.store16 offset=22
                        (local.get $base)
                        (i32.const 18831))

                    (local.set $string
                        (call $realloc
                            (i32.const 0)
                            (i32.const 0)
                            (i32.const 1)
                            (i32.const 6)))

                    (i32.store8 offset=0
                        (local.get $string)
                        (i32.const 97)) ;; 'a'
                    (i32.store8 offset=1
                        (local.get $string)
                        (i32.const 98)) ;; 'b'
                    (i32.store8 offset=2
                        (local.get $string)
                        (i32.const 99)) ;; 'c'
                    (i32.store8 offset=3
                        (local.get $string)
                        (i32.const 100)) ;; 'd'
                    (i32.store8 offset=4
                        (local.get $string)
                        (i32.const 101)) ;; 'e'
                    (i32.store8 offset=5
                        (local.get $string)
                        (i32.const 102)) ;; 'f'

                    (i32.store offset=24
                        (local.get $base)
                        (local.get $string))

                    (i32.store offset=28
                        (local.get $base)
                        (i32.const 2))

                    (local.set $list
                        (call $realloc
                            (i32.const 0)
                            (i32.const 0)
                            (i32.const 4)
                            (i32.const 32)))

                    (i32.store offset=0
                        (local.get $list)
                        (i32.const 1))
                    (i32.store offset=4
                        (local.get $list)
                        (i32.const 2))
                    (i32.store offset=8
                        (local.get $list)
                        (i32.const 3))
                    (i32.store offset=12
                        (local.get $list)
                        (i32.const 4))
                    (i32.store offset=16
                        (local.get $list)
                        (i32.const 5))
                    (i32.store offset=20
                        (local.get $list)
                        (i32.const 6))
                    (i32.store offset=24
                        (local.get $list)
                        (i32.const 7))
                    (i32.store offset=28
                        (local.get $list)
                        (i32.const 8))

                    (i32.store offset=32
                        (local.get $base)
                        (local.get $list))

                    (i32.store offset=36
                        (local.get $base)
                        (i32.const 8))

                    (i32.store8 offset=40
                        (local.get $base)
                        (i32.const 1))

                    (i32.store8 offset=41
                        (local.get $base)
                        (i32.const 0))

                    (i32.store offset=44
                        (local.get $base)
                        (i32.const 128681)) ;; '🚩'

                    (local.set $list
                        (call $realloc
                            (i32.const 0)
                            (i32.const 0)
                            (i32.const 1)
                            (i32.const 5)))

                    (i32.store8 offset=0
                        (local.get $list)
                        (i32.const 0))
                    (i32.store8 offset=1
                        (local.get $list)
                        (i32.const 1))
                    (i32.store8 offset=2
                        (local.get $list)
                        (i32.const 0))
                    (i32.store8 offset=3
                        (local.get $list)
                        (i32.const 1))
                    (i32.store8 offset=4
                        (local.get $list)
                        (i32.const 1))

                    (i32.store offset=48
                        (local.get $base)
                        (local.get $list))

                    (i32.store offset=52
                        (local.get $base)
                        (i32.const 5))

                    (local.set $list
                        (call $realloc
                            (i32.const 0)
                            (i32.const 0)
                            (i32.const 4)
                            (i32.const 20)))

                    (i32.store offset=0
                        (local.get $list)
                        (i32.const 127820)) ;; '🍌'
                    (i32.store offset=4
                        (local.get $list)
                        (i32.const 129360)) ;; '🥐'
                    (i32.store offset=8
                        (local.get $list)
                        (i32.const 127831)) ;; '🍗'
                    (i32.store offset=12
                        (local.get $list)
                        (i32.const 127833)) ;; '🍙'
                    (i32.store offset=16
                        (local.get $list)
                        (i32.const 127841)) ;; '🍡'

                    (i32.store offset=56
                        (local.get $base)
                        (local.get $list))

                    (i32.store offset=60
                        (local.get $base)
                        (i32.const 5))

                    (local.set $list
                        (call $realloc
                            (i32.const 0)
                            (i32.const 0)
                            (i32.const 4)
                            (i32.const 16)))

                    (i32.store offset=0
                        (local.get $list)
                        (i32.add (local.get $string) (i32.const 2)))
                    (i32.store offset=4
                        (local.get $list)
                        (i32.const 2))
                    (i32.store offset=8
                        (local.get $list)
                        (i32.add (local.get $string) (i32.const 4)))
                    (i32.store offset=12
                        (local.get $list)
                        (i32.const 2))

                    (i32.store offset=64
                        (local.get $base)
                        (local.get $list))

                    (i32.store offset=68
                        (local.get $base)
                        (i32.const 2))

                    local.get $base

                    {ret}
                )
                (func (export "callback") (param i32 i32 i32) (result i32) unreachable)
            )
            (type $tuple (tuple
                s8
                u64
                float32
                u8
                s16
                string
                (list u32)
                bool
                bool
                char
                (list bool)
                (list char)
                (list string)
            ))
            (core func $task-return (canon task.return
                (result $tuple)
                (memory (core memory $libc "memory"))
            ))
            (core instance $i (instantiate $m
                (with "" (instance (export "task.return" (func $task-return))))
                (with "libc" (instance $libc))
            ))

            (type $t (func async (result $tuple)))
            (func (export "many-results") (type $t)
                (canon lift
                    (core func $i "foo")
                    (memory (core memory $libc "memory"))
                    (realloc (core func $libc "realloc"))
                    {async_opts}
                )
            )
        )"#
    );

    let mut config = Config::new();
    config.wasm_component_model_async(true);
    let engine = &Engine::new(&config)?;
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, ());

    let instance = Linker::new(&engine)
        .instantiate_async(&mut store, &component)
        .await?;

    let expected = (
        -100i8,
        u64::MAX / 2,
        f32::from_bits(CANON_32BIT_NAN | 1),
        38u8,
        18831i16,
        "ab".to_string(),
        vec![1u32, 2, 3, 4, 5, 6, 7, 8],
        true,
        false,
        '🚩',
        vec![false, true, false, true, true],
        vec!['🍌', '🥐', '🍗', '🍙', '🍡'],
        vec!["cd".to_string(), "ef".to_string()],
    );

    let actual = if dynamic {
        let func = instance.get_func(&mut store, "many-results").unwrap();

        let mut results = vec![Val::Bool(false)];
        if concurrent {
            store
                .run_concurrent(async |store| {
                    func.call_concurrent(store, &[], &mut results).await?;
                    wasmtime::error::Ok(())
                })
                .await??;
        } else {
            func.call_async(&mut store, &[], &mut results).await?;
        };
        let mut results = results.into_iter();

        let Some(Val::Tuple(results)) = results.next() else {
            panic!()
        };
        let mut results = results.into_iter();
        let Some(Val::S8(p1)) = results.next() else {
            panic!()
        };
        let Some(Val::U64(p2)) = results.next() else {
            panic!()
        };
        let Some(Val::Float32(p3)) = results.next() else {
            panic!()
        };
        let Some(Val::U8(p4)) = results.next() else {
            panic!()
        };
        let Some(Val::S16(p5)) = results.next() else {
            panic!()
        };
        let Some(Val::String(p6)) = results.next() else {
            panic!()
        };
        let Some(Val::List(p7)) = results.next() else {
            panic!()
        };
        let p7 = p7
            .into_iter()
            .map(|v| if let Val::U32(v) = v { v } else { panic!() })
            .collect();
        let Some(Val::Bool(p8)) = results.next() else {
            panic!()
        };
        let Some(Val::Bool(p9)) = results.next() else {
            panic!()
        };
        let Some(Val::Char(p0)) = results.next() else {
            panic!()
        };
        let Some(Val::List(pa)) = results.next() else {
            panic!()
        };
        let pa = pa
            .into_iter()
            .map(|v| if let Val::Bool(v) = v { v } else { panic!() })
            .collect();
        let Some(Val::List(pb)) = results.next() else {
            panic!()
        };
        let pb = pb
            .into_iter()
            .map(|v| if let Val::Char(v) = v { v } else { panic!() })
            .collect();
        let Some(Val::List(pc)) = results.next() else {
            panic!()
        };
        let pc = pc
            .into_iter()
            .map(|v| if let Val::String(v) = v { v } else { panic!() })
            .collect();

        (p1, p2, p3, p4, p5, p6, p7, p8, p9, p0, pa, pb, pc)
    } else {
        let func = instance.get_typed_func::<(), ((
            i8,
            u64,
            f32,
            u8,
            i16,
            String,
            Vec<u32>,
            bool,
            bool,
            char,
            Vec<bool>,
            Vec<char>,
            Vec<String>,
        ),)>(&mut store, "many-results")?;

        if concurrent {
            store
                .run_concurrent(async move |accessor| {
                    wasmtime::error::Ok(func.call_concurrent(accessor, ()).await?)
                })
                .await??
                .0
        } else {
            func.call_async(&mut store, ()).await?.0
        }
    };

    assert_eq!(expected.0, actual.0);
    assert_eq!(expected.1, actual.1);
    assert!(expected.2.is_nan());
    assert!(actual.2.is_nan());
    assert_eq!(expected.3, actual.3);
    assert_eq!(expected.4, actual.4);
    assert_eq!(expected.5, actual.5);
    assert_eq!(expected.6, actual.6);
    assert_eq!(expected.7, actual.7);
    assert_eq!(expected.8, actual.8);
    assert_eq!(expected.9, actual.9);
    assert_eq!(expected.10, actual.10);
    assert_eq!(expected.11, actual.11);
    assert_eq!(expected.12, actual.12);

    Ok(())
}

trait SliceExt<'a> {
    fn take_n<const N: usize>(&mut self) -> &'a [u8; N];

    fn skip<const N: usize>(&mut self) {
        self.take_n::<N>();
    }

    fn ptr_len<'b>(&mut self, all_memory: &'b [u8], size: usize) -> &'b [u8] {
        let ptr = u32::from_le_bytes(*self.take_n::<4>());
        let len = u32::from_le_bytes(*self.take_n::<4>());
        let ptr = usize::try_from(ptr).unwrap();
        let len = usize::try_from(len).unwrap();
        &all_memory[ptr..][..len * size]
    }
}

impl<'a> SliceExt<'a> for &'a [u8] {
    fn take_n<const N: usize>(&mut self) -> &'a [u8; N] {
        let (a, b) = self.split_at(N);
        *self = b;
        a.try_into().unwrap()
    }
}

#[test]
fn drop_component_still_works() -> Result<()> {
    let component = r#"
        (component
            (import "f" (func $f))

            (core func $f_lower
                (canon lower (func $f))
            )
            (core module $m
                (import "" "" (func $f))

                (func $f2
                    call $f
                    call $f
                )

                (export "f" (func $f2))
            )
            (core instance $i (instantiate $m
                (with "" (instance
                    (export "" (func $f_lower))
                ))
            ))
            (func (export "g")
                (canon lift
                    (core func $i "f")
                )
            )
        )
    "#;

    let (mut store, instance) = {
        let engine = super::engine();
        let component = Component::new(&engine, component)?;
        let mut store = Store::new(&engine, 0);
        let mut linker = Linker::new(&engine);
        linker.root().func_wrap(
            "f",
            |mut store: StoreContextMut<'_, u32>, _: ()| -> Result<()> {
                *store.data_mut() += 1;
                Ok(())
            },
        )?;
        let instance = linker.instantiate(&mut store, &component)?;
        (store, instance)
    };

    let f = instance.get_typed_func::<(), ()>(&mut store, "g")?;
    assert_eq!(*store.data(), 0);
    f.call(&mut store, ())?;
    assert_eq!(*store.data(), 2);

    Ok(())
}

#[test]
fn raw_slice_of_various_types() -> Result<()> {
    let component = r#"
        (component
            (core module $m
                (memory (export "memory") 1)

                (func (export "list8") (result i32)
                    (call $setup_list (i32.const 16))
                )
                (func (export "list16") (result i32)
                    (call $setup_list (i32.const 8))
                )
                (func (export "list32") (result i32)
                    (call $setup_list (i32.const 4))
                )
                (func (export "list64") (result i32)
                    (call $setup_list (i32.const 2))
                )

                (func $setup_list (param i32) (result i32)
                    (i32.store offset=0 (i32.const 100) (i32.const 8))
                    (i32.store offset=4 (i32.const 100) (local.get 0))
                    i32.const 100
                )

                (data (i32.const 8) "\00\01\02\03\04\05\06\07\08\09\0a\0b\0c\0d\0e\0f")
            )
            (core instance $i (instantiate $m))
            (func (export "list-u8") (result (list u8))
                (canon lift (core func $i "list8") (memory (core memory $i "memory")))
            )
            (func (export "list-i8") (result (list s8))
                (canon lift (core func $i "list8") (memory (core memory $i "memory")))
            )
            (func (export "list-u16") (result (list u16))
                (canon lift (core func $i "list16") (memory (core memory $i "memory")))
            )
            (func (export "list-i16") (result (list s16))
                (canon lift (core func $i "list16") (memory (core memory $i "memory")))
            )
            (func (export "list-u32") (result (list u32))
                (canon lift (core func $i "list32") (memory (core memory $i "memory")))
            )
            (func (export "list-i32") (result (list s32))
                (canon lift (core func $i "list32") (memory (core memory $i "memory")))
            )
            (func (export "list-u64") (result (list u64))
                (canon lift (core func $i "list64") (memory (core memory $i "memory")))
            )
            (func (export "list-i64") (result (list s64))
                (canon lift (core func $i "list64") (memory (core memory $i "memory")))
            )
        )
    "#;

    let (mut store, instance) = {
        let engine = super::engine();
        let component = Component::new(&engine, component)?;
        let mut store = Store::new(&engine, ());
        let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
        (store, instance)
    };

    let list = instance
        .get_typed_func::<(), (WasmList<u8>,)>(&mut store, "list-u8")?
        .call(&mut store, ())?
        .0;
    assert_eq!(
        list.as_le_slice(&store),
        [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f,
        ]
    );
    let list = instance
        .get_typed_func::<(), (WasmList<i8>,)>(&mut store, "list-i8")?
        .call(&mut store, ())?
        .0;
    assert_eq!(
        list.as_le_slice(&store),
        [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f,
        ]
    );

    let list = instance
        .get_typed_func::<(), (WasmList<u16>,)>(&mut store, "list-u16")?
        .call(&mut store, ())?
        .0;
    assert_eq!(
        list.as_le_slice(&store),
        [
            u16::to_le(0x01_00),
            u16::to_le(0x03_02),
            u16::to_le(0x05_04),
            u16::to_le(0x07_06),
            u16::to_le(0x09_08),
            u16::to_le(0x0b_0a),
            u16::to_le(0x0d_0c),
            u16::to_le(0x0f_0e),
        ]
    );
    let list = instance
        .get_typed_func::<(), (WasmList<i16>,)>(&mut store, "list-i16")?
        .call(&mut store, ())?
        .0;
    assert_eq!(
        list.as_le_slice(&store),
        [
            i16::to_le(0x01_00),
            i16::to_le(0x03_02),
            i16::to_le(0x05_04),
            i16::to_le(0x07_06),
            i16::to_le(0x09_08),
            i16::to_le(0x0b_0a),
            i16::to_le(0x0d_0c),
            i16::to_le(0x0f_0e),
        ]
    );
    let list = instance
        .get_typed_func::<(), (WasmList<u32>,)>(&mut store, "list-u32")?
        .call(&mut store, ())?
        .0;
    assert_eq!(
        list.as_le_slice(&store),
        [
            u32::to_le(0x03_02_01_00),
            u32::to_le(0x07_06_05_04),
            u32::to_le(0x0b_0a_09_08),
            u32::to_le(0x0f_0e_0d_0c),
        ]
    );
    let list = instance
        .get_typed_func::<(), (WasmList<i32>,)>(&mut store, "list-i32")?
        .call(&mut store, ())?
        .0;
    assert_eq!(
        list.as_le_slice(&store),
        [
            i32::to_le(0x03_02_01_00),
            i32::to_le(0x07_06_05_04),
            i32::to_le(0x0b_0a_09_08),
            i32::to_le(0x0f_0e_0d_0c),
        ]
    );
    let list = instance
        .get_typed_func::<(), (WasmList<u64>,)>(&mut store, "list-u64")?
        .call(&mut store, ())?
        .0;
    assert_eq!(
        list.as_le_slice(&store),
        [
            u64::to_le(0x07_06_05_04_03_02_01_00),
            u64::to_le(0x0f_0e_0d_0c_0b_0a_09_08),
        ]
    );
    let list = instance
        .get_typed_func::<(), (WasmList<i64>,)>(&mut store, "list-i64")?
        .call(&mut store, ())?
        .0;
    assert_eq!(
        list.as_le_slice(&store),
        [
            i64::to_le(0x07_06_05_04_03_02_01_00),
            i64::to_le(0x0f_0e_0d_0c_0b_0a_09_08),
        ]
    );

    Ok(())
}

#[test]
fn lower_then_lift() -> Result<()> {
    // First test simple integers when the import/export ABI happen to line up
    let component = r#"
(component $c
  (import "f" (func $f (result u32)))

  (core func $f_lower
    (canon lower (func $f))
  )
  (func $f2 (result s32)
    (canon lift (core func $f_lower))
  )
  (export "f2" (func $f2))
)
    "#;

    let engine = super::engine();
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);
    linker.root().func_wrap("f", |_, _: ()| Ok((2u32,)))?;
    let instance = linker.instantiate(&mut store, &component)?;

    let f = instance.get_typed_func::<(), (i32,)>(&mut store, "f2")?;
    assert_eq!(f.call(&mut store, ())?, (2,));

    // First test strings when the import/export ABI happen to line up
    let component = format!(
        r#"
(component $c
  (import "s" (func $f (param "a" string)))

  (core module $libc
    (memory (export "memory") 1)
    {REALLOC_AND_FREE}
  )
  (core instance $libc (instantiate $libc))

  (core func $f_lower
    (canon lower (func $f) (memory (core memory $libc "memory")))
  )
  (func $f2 (param "a" string)
    (canon lift (core func $f_lower)
        (memory (core memory $libc "memory"))
        (realloc (core func $libc "realloc"))
    )
  )
  (export "f" (func $f2))
)
    "#
    );

    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, ());
    linker
        .root()
        .func_wrap("s", |store: StoreContextMut<'_, ()>, (x,): (WasmStr,)| {
            assert_eq!(x.to_str(&store)?, "hello");
            Ok(())
        })?;
    let instance = linker.instantiate(&mut store, &component)?;

    let f = instance.get_typed_func::<(&str,), ()>(&mut store, "f")?;
    f.call(&mut store, ("hello",))?;

    // Next test "type punning" where return values are reinterpreted just
    // because the return ABI happens to line up.
    let component = format!(
        r#"
(component $c
  (import "s2" (func $f (param "a" string) (result u32)))

  (core module $libc
    (memory (export "memory") 1)
    {REALLOC_AND_FREE}
  )
  (core instance $libc (instantiate $libc))

  (core func $f_lower
    (canon lower (func $f) (memory (core memory $libc "memory")))
  )
  (func $f2 (param "a" string) (result string)
    (canon lift (core func $f_lower)
        (memory (core memory $libc "memory"))
        (realloc (core func $libc "realloc"))
    )
  )
  (export "f" (func $f2))
)
    "#
    );

    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, ());
    linker
        .root()
        .func_wrap("s2", |store: StoreContextMut<'_, ()>, (x,): (WasmStr,)| {
            assert_eq!(x.to_str(&store)?, "hello");
            Ok((u32::MAX,))
        })?;
    let instance = linker.instantiate(&mut store, &component)?;

    let f = instance.get_typed_func::<(&str,), (WasmStr,)>(&mut store, "f")?;
    let err = f.call(&mut store, ("hello",)).err().unwrap();
    assert!(
        err.to_string().contains("return pointer not aligned"),
        "{}",
        err
    );

    Ok(())
}

// The dynamic `Val` API is covered by wast tests, but `TypedFunc` has its own
// lifting and lowering for options, results, strings, and lists.
#[test]
fn typed_lift_errors() -> Result<()> {
    let component = r#"
(component
  (import "host-option" (func $host (param "a" (option u8))))
  (core func $host (canon lower (func $host)))
  (core module $m
    (import "" "host" (func $host (param i32 i32)))
    (memory (export "memory") 1)
    (func (export "id") (param i32) (result i32) local.get 0)
    (func (export "second") (param i32 i32) (result i32) local.get 1)
    (func (export "store") (param i32 i32) (result i32)
      (i32.store (i32.const 8) (local.get 0))
      (i32.store (i32.const 12) (local.get 1))
      i32.const 8)
    (func (export "call-host") (param i32)
      (call $host (local.get 0) (i32.const 3)))
  )
  (core instance $i (instantiate $m
    (with "" (instance (export "host" (func $host))))))
  (alias core export $i "memory" (core memory $mem))

  (func (export "call-host") (param "a" u32)
    (canon lift (core func $i "call-host")))
  (func (export "result-unit") (param "a" u32) (result (result))
    (canon lift (core func $i "id")))
  (func (export "option-u32") (param "a" u32) (param "b" u32)
    (result (option u32))
    (canon lift (core func $i "store") (memory $mem)))
  (func (export "result-s16-f32") (param "a" u32) (param "b" u32)
    (result (result s16 (error f32)))
    (canon lift (core func $i "store") (memory $mem)))
  (func (export "list-u8") (param "a" u32) (param "b" u32)
    (result (list u8))
    (canon lift (core func $i "store") (memory $mem)))
  (func (export "list-u32") (param "a" u32) (param "b" u32)
    (result (list u32))
    (canon lift (core func $i "store") (memory $mem)))
  (func (export "string") (param "a" u32) (param "b" u32) (result string)
    (canon lift (core func $i "store") (memory $mem)))
  (func (export "result-u8-f32-payload") (param "a" (result u8 (error f32)))
    (result u32)
    (canon lift (core func $i "second")))
  (func (export "ret-ptr") (param "a" u32) (result (tuple u32 u32))
    (canon lift (core func $i "id") (memory $mem)))
)
"#;

    let engine = super::engine();
    let component = Component::new(&engine, component)?;
    let mut linker = Linker::new(&engine);
    linker
        .root()
        .func_wrap("host-option", |_, (a,): (Option<u8>,)| {
            assert_eq!(a, Some(3));
            Ok(())
        })?;
    let mut store = Store::new(&engine, ());
    let instance = linker.instantiate(&mut store, &component)?;

    // Each error poisons the instance, so use a fresh one for each.
    fn expect_err<P, R>(
        linker: &Linker<()>,
        component: &Component,
        name: &str,
        params: P,
        msg: &str,
    ) where
        P: ComponentNamedList + Lower + 'static,
        R: ComponentNamedList + Lift + 'static,
    {
        let mut store = Store::new(linker.engine(), ());
        let instance = linker.instantiate(&mut store, component).unwrap();
        let f = instance.get_typed_func::<P, R>(&mut store, name).unwrap();
        let err = f.call(&mut store, params).err().unwrap();
        assert!(format!("{err:?}").contains(msg), "{err:?}");
    }

    // Options and results lifted from flat values.
    let f = instance.get_typed_func::<(u32,), ()>(&mut store, "call-host")?;
    f.call(&mut store, (1,))?;
    let f = instance.get_typed_func::<(u32,), (Result<(), ()>,)>(&mut store, "result-unit")?;
    assert_eq!(f.call(&mut store, (0,))?, (Ok(()),));
    assert_eq!(f.call(&mut store, (1,))?, (Err(()),));
    expect_err::<(u32,), ()>(
        &linker,
        &component,
        "call-host",
        (2,),
        "invalid option discriminant",
    );
    expect_err::<(u32,), (Result<(), ()>,)>(
        &linker,
        &component,
        "result-unit",
        (2,),
        "invalid expected discriminant",
    );

    // Options and results lifted from memory.
    let f = instance.get_typed_func::<(u32, u32), (Option<u32>,)>(&mut store, "option-u32")?;
    assert_eq!(f.call(&mut store, (0, 0))?, (None,));
    assert_eq!(f.call(&mut store, (1, 0x1234fead))?, (Some(0x1234fead),));
    let f =
        instance.get_typed_func::<(u32, u32), (Result<i16, f32>,)>(&mut store, "result-s16-f32")?;
    assert_eq!(f.call(&mut store, (0, 100))?, (Ok(100),));
    let (ret,) = f.call(&mut store, (1, CANON_32BIT_NAN | 1))?;
    assert_eq!(ret.unwrap_err().to_bits(), CANON_32BIT_NAN | 1);
    expect_err::<(u32, u32), (Option<u32>,)>(
        &linker,
        &component,
        "option-u32",
        (2, 0),
        "invalid option discriminant",
    );
    expect_err::<(u32, u32), (Result<i16, f32>,)>(
        &linker,
        &component,
        "result-s16-f32",
        (2, 0),
        "invalid expected discriminant",
    );

    // Strings and lists with invalid pointers.
    expect_err::<(u32, u32), (WasmList<u8>,)>(
        &linker,
        &component,
        "list-u8",
        (100_000, 1),
        "list pointer/length out of bounds",
    );
    expect_err::<(u32, u32), (WasmList<u32>,)>(
        &linker,
        &component,
        "list-u32",
        (1, 1),
        "list pointer is not aligned",
    );
    expect_err::<(u32, u32), (WasmStr,)>(
        &linker,
        &component,
        "string",
        (100_000, 1),
        "string pointer/length out of bounds",
    );

    // A return pointer that is aligned but out of bounds.
    expect_err::<(u32,), ((u32, u32),)>(
        &linker,
        &component,
        "ret-ptr",
        (0xffff_fff8,),
        "pointer out of bounds of memory",
    );

    // A result whose `u8` and `f32` payloads share one flat `i32` slot.
    let f = instance
        .get_typed_func::<(Result<u8, f32>,), (u32,)>(&mut store, "result-u8-f32-payload")?;
    assert_eq!(f.call(&mut store, (Ok(7),))?, (7,));
    assert_eq!(f.call(&mut store, (Err(1.5),))?, (1.5f32.to_bits(),));
    let nan = f32::from_bits(CANON_32BIT_NAN | 1);
    assert_eq!(f.call(&mut store, (Err(nan),))?, (CANON_32BIT_NAN | 1,));

    Ok(())
}

#[test]
fn errors_that_poison_instance() -> Result<()> {
    let component = format!(
        r#"
(component $c
  (core module $m1
    (func (export "f1") unreachable)
    (func (export "f2"))
  )
  (core instance $m1 (instantiate $m1))
  (func (export "f1") (canon lift (core func $m1 "f1")))
  (func (export "f2") (canon lift (core func $m1 "f2")))

  (core module $m2
    (func (export "f") (param i32 i32))
    (func (export "r") (param i32 i32 i32 i32) (result i32) unreachable)
    (memory (export "m") 1)
  )
  (core instance $m2 (instantiate $m2))
  (func (export "f3") (param "a" string)
    (canon lift (core func $m2 "f") (realloc (core func $m2 "r")) (memory (core memory $m2 "m")))
  )

  (core module $m3
    (func (export "f") (result i32) i32.const 1)
    (memory (export "m") 1)
  )
  (core instance $m3 (instantiate $m3))
  (func (export "f4") (result string)
    (canon lift (core func $m3 "f") (memory (core memory $m3 "m")))
  )
)
    "#
    );

    let engine = super::engine();
    let component = Component::new(&engine, component)?;
    let linker = Linker::new(&engine);

    {
        let mut store = Store::new(&engine, ());
        let instance = linker.instantiate(&mut store, &component)?;
        let f1 = instance.get_typed_func::<(), ()>(&mut store, "f1")?;
        let f2 = instance.get_typed_func::<(), ()>(&mut store, "f2")?;
        assert_unreachable(f1.call(&mut store, ()));
        assert_poisoned(f1.call(&mut store, ()));
        assert_poisoned(f2.call(&mut store, ()));
    }

    {
        let mut store = Store::new(&engine, ());
        let instance = linker.instantiate(&mut store, &component)?;
        let f3 = instance.get_typed_func::<(&str,), ()>(&mut store, "f3")?;
        assert_unreachable(f3.call(&mut store, ("x",)));
        assert_poisoned(f3.call(&mut store, ("x",)));

        // Since we actually poison the store, even an unrelated instance will
        // be considered poisoned:
        let instance = linker.instantiate(&mut store, &component)?;
        let f3 = instance.get_typed_func::<(&str,), ()>(&mut store, "f3")?;
        assert_poisoned(f3.call(&mut store, ("x",)));
    }

    {
        let mut store = Store::new(&engine, ());
        let instance = linker.instantiate(&mut store, &component)?;
        let f4 = instance.get_typed_func::<(), (WasmStr,)>(&mut store, "f4")?;
        assert!(f4.call(&mut store, ()).is_err());
        assert_poisoned(f4.call(&mut store, ()));
    }

    return Ok(());

    #[track_caller]
    fn assert_unreachable<T>(err: Result<T>) {
        let err = match err {
            Ok(_) => panic!("expected an error"),
            Err(e) => e,
        };
        assert_eq!(
            err.downcast::<Trap>().unwrap(),
            Trap::UnreachableCodeReached
        );
    }

    #[track_caller]
    fn assert_poisoned<T>(err: Result<T>) {
        let err = match err {
            Ok(_) => panic!("expected an error"),
            Err(e) => e,
        };
        assert_eq!(
            err.downcast_ref::<Trap>(),
            Some(&Trap::CannotEnterComponent),
            "{err}",
        );
    }
}

enum RecurseKind {
    AThenA,
    AThenB,
    AThenBThenA,
}

#[test]
fn recurse_a_then_b() -> Result<()> {
    test_recurse(RecurseKind::AThenB)
}

#[test]
fn recurse_a_then_a() -> Result<()> {
    test_recurse(RecurseKind::AThenA)
}

#[test]
fn recurse_a_then_b_then_a() -> Result<()> {
    test_recurse(RecurseKind::AThenBThenA)
}

fn test_recurse(kind: RecurseKind) -> Result<()> {
    #[derive(Default)]
    struct Ctx {
        instances: Vec<Arc<Instance>>,
    }

    let component = r#"
(component
  (import "import" (func $import))
  (core func $import (canon lower (func $import)))
  (core module $m
    (func $import (import "" "import"))
    (func (export "export") call $import)
  )
  (core instance $m (instantiate $m (with "" (instance
    (export "import" (func $import))
  ))))
  (func (export "export") (canon lift (core func $m "export")))
)
"#;
    let engine = super::engine();
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, Ctx::default());
    let mut linker = Linker::<Ctx>::new(&engine);
    linker.root().func_wrap("import", |mut store, (): ()| {
        if let Some(instance) = store.data_mut().instances.pop() {
            let run = instance.get_typed_func::<(), ()>(&mut store, "export")?;
            run.call(&mut store, ())?;
            store.data_mut().instances.push(instance);
        }
        Ok(())
    })?;
    let instance = Arc::new(linker.instantiate(&mut store, &component)?);
    let instance = match kind {
        RecurseKind::AThenA => {
            store.data_mut().instances.push(instance.clone());
            instance
        }
        RecurseKind::AThenB => {
            let other = Arc::new(linker.instantiate(&mut store, &component)?);
            store.data_mut().instances.push(other);
            instance
        }
        RecurseKind::AThenBThenA => {
            store.data_mut().instances.push(instance.clone());
            let other = Arc::new(linker.instantiate(&mut store, &component)?);
            store.data_mut().instances.push(other);
            instance
        }
    };

    let export = instance.get_typed_func::<(), ()>(&mut store, "export")?;
    export.call(&mut store, ())?;
    Ok(())
}

#[tokio::test]
async fn thread_index_via_instantiation_sync() -> Result<()> {
    thread_index_via_instantiation(ApiStyle::Sync).await
}

async fn thread_index_via_instantiation(style: ApiStyle) -> Result<()> {
    let component = r#"
(component
  (core module $m
    (import "" "thread.index" (func $thread-index (result i32)))
    (func $start
       (if (i32.eqz (call $thread-index)) (then unreachable))
    )
    (start $start)
  )
  (core func $thread-index (canon thread.index))
  (core instance $m (instantiate $m (with "" (instance
    (export "thread.index" (func $thread-index))
  ))))
)
"#;
    let engine = Engine::new(&style.config())?;
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, ());
    let linker = Linker::new(&engine);
    style.instantiate(&mut store, &linker, &component).await?;
    Ok(())
}

#[tokio::test]
async fn thread_index_via_call_sync() -> Result<()> {
    thread_index_via_call(ApiStyle::Sync).await
}

#[tokio::test]
async fn thread_index_via_call_concurrent() -> Result<()> {
    thread_index_via_call(ApiStyle::Concurrent).await
}

async fn thread_index_via_call(style: ApiStyle) -> Result<()> {
    let component = r#"
(component
  (core module $m
    (import "" "thread.index" (func $thread-index (result i32)))
    (func (export "run")
       (if (i32.eqz (call $thread-index)) (then unreachable))
    )
  )
  (core func $thread-index (canon thread.index))
  (core instance $m (instantiate $m (with "" (instance
    (export "thread.index" (func $thread-index))
  ))))
  (func (export "run") (canon lift (core func $m "run")))
)
"#;
    let engine = Engine::new(&style.config())?;
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, ());
    let linker = Linker::new(&engine);
    let instance = style.instantiate(&mut store, &linker, &component).await?;
    let run = instance.get_typed_func::<(), ()>(&mut store, "run")?;
    style.call(&mut store, run, ()).await?;
    Ok(())
}

#[tokio::test]
async fn thread_index_via_resource_drop_sync() -> Result<()> {
    thread_index_via_resource_drop(ApiStyle::Sync).await
}

#[tokio::test]
async fn thread_index_via_resource_drop_async() -> Result<()> {
    thread_index_via_resource_drop(ApiStyle::Async).await
}

#[tokio::test]
async fn thread_index_via_resource_drop_concurrent() -> Result<()> {
    thread_index_via_resource_drop(ApiStyle::Concurrent).await
}

/// A component exporting a resource whose destructor traps unless it runs on a
/// non-zero `thread.index`, i.e. within a guest thread context.
const THREAD_INDEX_VIA_RESOURCE_DROP_COMPONENT: &str = r#"
(component
  (core module $m
    (import "" "thread.index" (func $thread-index (result i32)))
    (func (export "dtor") (param i32)
       (if (i32.eqz (call $thread-index)) (then unreachable))
    )
  )
  (core func $thread-index (canon thread.index))
  (core instance $m (instantiate $m (with "" (instance
    (export "thread.index" (func $thread-index))
  ))))
  (type $r (resource (rep i32) (dtor (core func $m "dtor"))))
  (core func $new (canon resource.new $r))
  (core module $m2
    (import "" "new" (func $new (param i32) (result i32)))
    (func (export "new") (result i32)
       (call $new (i32.const 100))
    )
  )
  (core instance $m2 (instantiate $m2 (with "" (instance
    (export "new" (func $new))
  ))))
  (func $new (result (own $r)) (canon lift (core func $m2 "new")))
  (component $c
    (import "r" (type $r (sub resource)))
    (import "new" (func $new (result (own $r))))
    (export $r-export "r" (type $r))
    (export "new" (func $new) (func (result (own $r-export))))
  )
  (instance $c (instantiate $c
    (with "r" (type $r))
    (with "new" (func $new))
  ))
  (export "i" (instance $c))
)
"#;

async fn thread_index_via_resource_drop(style: ApiStyle) -> Result<()> {
    let engine = Engine::new(&style.config())?;
    let component = Component::new(&engine, THREAD_INDEX_VIA_RESOURCE_DROP_COMPONENT)?;
    let mut store = Store::new(&engine, ());
    let linker = Linker::new(&engine);
    let instance = style.instantiate(&mut store, &linker, &component).await?;
    let instance_index = instance.get_export_index(&mut store, None, "i").unwrap();
    let func_index = instance
        .get_export_index(&mut store, Some(&instance_index), "new")
        .unwrap();
    let run = instance.get_typed_func::<(), (ResourceAny,)>(&mut store, &func_index)?;
    let (resource,) = style.call(&mut store, run, ()).await?;
    style.resource_drop(&mut store, resource).await?;
    Ok(())
}

/// Test that a guest-exported `ResourceAny` can be dropped with
/// `resource_drop_concurrent` inside the same `Store::run_concurrent` call
/// that produced it, i.e. while the store's event loop is already running.
/// This is the scenario from issue #14291.
#[tokio::test]
async fn resource_drop_concurrent_in_run_concurrent() -> Result<()> {
    let style = ApiStyle::Concurrent;
    let engine = Engine::new(&style.config())?;
    let component = Component::new(&engine, THREAD_INDEX_VIA_RESOURCE_DROP_COMPONENT)?;
    let mut store = Store::new(&engine, ());
    let linker = Linker::new(&engine);
    let instance = style.instantiate(&mut store, &linker, &component).await?;
    let instance_index = instance.get_export_index(&mut store, None, "i").unwrap();
    let func_index = instance
        .get_export_index(&mut store, Some(&instance_index), "new")
        .unwrap();
    let run = instance.get_typed_func::<(), (ResourceAny,)>(&mut store, &func_index)?;
    store
        .run_concurrent(async |accessor| {
            let (resource,) = run.call_concurrent(accessor, ()).await?;
            resource.resource_drop_concurrent(accessor).await
        })
        .await??;
    store.assert_concurrent_state_empty();
    Ok(())
}

#[tokio::test]
async fn thread_index_via_guest_call_sync() -> Result<()> {
    thread_index_via_guest_call(ApiStyle::Sync).await
}

#[tokio::test]
async fn thread_index_via_guest_call_concurrent() -> Result<()> {
    thread_index_via_guest_call(ApiStyle::Concurrent).await
}

async fn thread_index_via_guest_call(style: ApiStyle) -> Result<()> {
    let component = r#"
(component
  (component $c
    (core module $m
      (import "" "thread.index" (func $thread-index (result i32)))
      (func (export "run") (result i32)
         (call $thread-index)
      )
    )
    (core func $thread-index (canon thread.index))
    (core instance $m (instantiate $m (with "" (instance
      (export "thread.index" (func $thread-index))
    ))))
    (func (export "run") (result u32) (canon lift (core func $m "run")))
  )
  (instance $c (instantiate $c))

  (component $d
    (import "c" (instance $c
      (export "run" (func (result u32)))
    ))
    (core func $run (canon lower (func $c "run")))
    (core module $m
      (import "" "thread.index" (func $thread-index (result i32)))
      (import "" "run" (func $run (result i32)))
      (func (export "run")
         (local $mine i32)
         (local $theirs i32)
         (local.set $mine (call $thread-index))
         (if (i32.eqz (local.get $mine)) (then unreachable))
         (local.set $theirs (call $run))
         (if (i32.eqz (local.get $theirs)) (then unreachable))
      )
    )
    (core func $thread-index (canon thread.index))
    (core instance $m (instantiate $m (with "" (instance
      (export "thread.index" (func $thread-index))
      (export "run" (func $run))
    ))))
    (func (export "run") (canon lift (core func $m "run")))
  )
  (instance $d (instantiate $d (with "c" (instance $c))))
  (func (export "run") (alias export $d "run"))
)
"#;
    let engine = Engine::new(&style.config())?;
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, ());
    let linker = Linker::new(&engine);
    let instance = style.instantiate(&mut store, &linker, &component).await?;
    let run = instance.get_typed_func::<(), ()>(&mut store, "run")?;
    style.call(&mut store, run, ()).await?;
    Ok(())
}

#[tokio::test]
async fn drop_call_async_future() -> Result<()> {
    let component = r#"
(component
  (import "foo" (func $f))
  (core module $m
    (func $f (import "" "foo"))
    (func (export "foo") call $f)
  )
  (core func $f (canon lower (func $f)))
  (core instance $m (instantiate $m (with "" (instance
     (export "foo" (func $f))
  ))))
  (func (export "foo") (canon lift (core func $m "foo")))
)
"#;

    let engine = &Engine::new(&Config::new())?;
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);
    linker.root().func_wrap_async("foo", |_, _: ()| {
        Box::new(async {
            tokio::task::yield_now().await;
            Ok(())
        })
    })?;
    let instance = linker.instantiate_async(&mut store, &component).await?;
    let foo = instance.get_typed_func::<(), ()>(&mut store, "foo")?;
    // Here we'll use `call_async` a few times but only poll each returned
    // future once.  This will put the instance in a weird state but shouldn't
    // cause a panic.
    for _ in 0..5 {
        let mut future = std::pin::pin!(foo.call_async(&mut store, ()));
        if let std::task::Poll::Ready(result) =
            std::future::poll_fn(|cx| std::task::Poll::Ready(future.as_mut().poll(cx))).await
        {
            _ = result;
        }
    }

    Ok(())
}

#[test]
fn host_call_with_concurrency_disabled() -> Result<()> {
    let mut config = Config::default();
    config.concurrency_support(false);

    struct MyResource;

    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());
    let mut linker = Linker::<()>::new(&engine);

    linker
        .root()
        .resource("r", ResourceType::host::<MyResource>(), |_, _| Ok(()))?;

    let f_called = Arc::new(AtomicBool::new(false));
    linker.root().func_wrap("f", {
        let f_called = f_called.clone();
        move |_ctx, _: (Resource<MyResource>,)| -> Result<()> {
            f_called.store(true, SeqCst);
            Ok(())
        }
    })?;

    let component = Component::new(
        &engine,
        r#"
            (component
                (import "r" (type $r (sub resource)))
                (import "f" (func $f (param "r" (borrow $r))))

                (core func $f' (canon lower (func $f)))
                (core func $drop (canon resource.drop $r))

                (core module $m
                    (import "" "f" (func $f (param i32)))
                    (import "" "drop" (func $drop (param i32)))
                    (func (export "g") (param i32)
                        (call $f (local.get 0))
                        (call $drop (local.get 0))
                    )
                )

                (core instance $i (instantiate $m (with
                    "" (instance
                           (export "f" (func $f'))
                           (export "drop" (func $drop))
                       )
                )))

                (func (export "g") (param "r" (borrow $r))
                    (canon lift (core func $i "g"))
                )
            )
        "#
        .as_bytes(),
    )?;

    let instance = linker.instantiate(&mut store, &component)?;
    let g = instance.get_typed_func::<(&Resource<MyResource>,), ()>(&mut store, "g")?;

    let resource = Resource::new_own(100);
    g.call(&mut store, (&resource,))?;

    assert!(f_called.load(SeqCst));

    Ok(())
}

/// Tests map types with misaligned key/value combinations through the adapter
/// trampoline (component-to-component translation).
///
/// This specifically tests the alignment bug where the value offset was
/// calculated as `key_size` instead of `align(key_size, value_align)`.
/// For map<u8, u64>, the value should be at offset 8 (not 1).
///
#[test]
fn map_trampoline_alignment() -> Result<()> {
    // Test map<u8, u64> - key_size=1, value_align=8
    // With the alignment bug, value would be read/written at offset 1 instead of 8
    let component = format!(
        r#"
(component
    (import "host" (func $host (param "m" (map u8 u64)) (result (map u8 u64))))

    ;; Component A: the "destination" that receives and echoes back
    (component $dst
        (import "echo" (func $echo (param "m" (map u8 u64)) (result (map u8 u64))))
        (core module $libc
            (memory (export "memory") 1)
            {REALLOC_AND_FREE}
        )
        (core module $echo_mod
            (import "" "echo" (func $echo (param i32 i32 i32)))
            (import "libc" "memory" (memory 0))
            (import "libc" "realloc" (func $realloc (param i32 i32 i32 i32) (result i32)))

            (func (export "echo") (param i32 i32) (result i32)
                (local $retptr i32)
                (local.set $retptr
                    (call $realloc (i32.const 0) (i32.const 0) (i32.const 4) (i32.const 8)))
                (call $echo (local.get 0) (local.get 1) (local.get $retptr))
                local.get $retptr
            )
        )
        (core instance $libc (instantiate $libc))
        (core func $echo_lower (canon lower (func $echo)
            (memory (core memory $libc "memory"))
            (realloc (core func $libc "realloc"))
        ))
        (core instance $echo_inst (instantiate $echo_mod
            (with "libc" (instance $libc))
            (with "" (instance (export "echo" (func $echo_lower))))
        ))
        (func (export "echo2") (param "m" (map u8 u64)) (result (map u8 u64))
            (canon lift
                (core func $echo_inst "echo")
                (memory (core memory $libc "memory"))
                (realloc (core func $libc "realloc"))
            )
        )
    )

    ;; Component B: the "source" that calls dst
    (component $src
        (import "echo" (func $echo (param "m" (map u8 u64)) (result (map u8 u64))))
        (core module $libc
            (memory (export "memory") 1)
            {REALLOC_AND_FREE}
        )
        (core module $echo_mod
            (import "" "echo" (func $echo (param i32 i32 i32)))
            (import "libc" "memory" (memory 0))
            (import "libc" "realloc" (func $realloc (param i32 i32 i32 i32) (result i32)))

            (func (export "echo") (param i32 i32) (result i32)
                (local $retptr i32)
                (local.set $retptr
                    (call $realloc (i32.const 0) (i32.const 0) (i32.const 4) (i32.const 8)))
                (call $echo (local.get 0) (local.get 1) (local.get $retptr))
                local.get $retptr
            )
        )
        (core instance $libc (instantiate $libc))
        (core func $echo_lower (canon lower (func $echo)
            (memory (core memory $libc "memory"))
            (realloc (core func $libc "realloc"))
        ))
        (core instance $echo_inst (instantiate $echo_mod
            (with "libc" (instance $libc))
            (with "" (instance (export "echo" (func $echo_lower))))
        ))
        (func (export "echo2") (param "m" (map u8 u64)) (result (map u8 u64))
            (canon lift
                (core func $echo_inst "echo")
                (memory (core memory $libc "memory"))
                (realloc (core func $libc "realloc"))
            )
        )
    )

    ;; Wire: host -> dst -> src creates adapter trampolines between components
    (instance $dst (instantiate $dst (with "echo" (func $host))))
    (instance $src (instantiate $src (with "echo" (func $dst "echo2"))))
    (export "echo" (func $src "echo2"))
)
"#
    );

    let mut config = Config::new();
    config.wasm_component_model(true);
    config.wasm_component_model_map(true);
    let engine = Engine::new(&config)?;
    let component = Component::new(&engine, component)?;

    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);

    linker.root().func_new("host", |_cx, _ty, args, results| {
        results[0] = args[0].clone();
        Ok(())
    })?;

    let instance = linker.instantiate(&mut store, &component)?;
    let func = instance.get_func(&mut store, "echo").unwrap();

    let test_data = vec![
        (Val::U8(1), Val::U64(0x0102030405060708)),
        (Val::U8(2), Val::U64(0x1112131415161718)),
        (Val::U8(255), Val::U64(0xFFFFFFFFFFFFFFFF)),
    ];
    let input = Val::Map(test_data.clone());

    let mut results = [Val::Bool(false)];
    func.call(&mut store, &[input], &mut results)?;

    match &results[0] {
        Val::Map(output) => {
            assert_eq!(output.len(), 3);
            for (key, value) in &test_data {
                assert!(
                    output.iter().any(|(k, v)| k == key && v == value),
                    "Missing or corrupted entry"
                );
            }
        }
        _ => panic!("expected map"),
    }

    Ok(())
}

/// Tests map<u32, u64> alignment through trampoline
#[test]
fn map_trampoline_alignment_u32_u64() -> Result<()> {
    // Test map<u32, u64> - key_size=4, value_align=8
    // With the alignment bug, value would be read/written at offset 4 instead of 8
    let component = format!(
        r#"
(component
    (import "host" (func $host (param "m" (map u32 u64)) (result (map u32 u64))))

    (component $dst
        (import "echo" (func $echo (param "m" (map u32 u64)) (result (map u32 u64))))
        (core module $libc
            (memory (export "memory") 1)
            {REALLOC_AND_FREE}
        )
        (core module $echo_mod
            (import "" "echo" (func $echo (param i32 i32 i32)))
            (import "libc" "memory" (memory 0))
            (import "libc" "realloc" (func $realloc (param i32 i32 i32 i32) (result i32)))

            (func (export "echo") (param i32 i32) (result i32)
                (local $retptr i32)
                (local.set $retptr
                    (call $realloc (i32.const 0) (i32.const 0) (i32.const 4) (i32.const 8)))
                (call $echo (local.get 0) (local.get 1) (local.get $retptr))
                local.get $retptr
            )
        )
        (core instance $libc (instantiate $libc))
        (core func $echo_lower (canon lower (func $echo)
            (memory (core memory $libc "memory"))
            (realloc (core func $libc "realloc"))
        ))
        (core instance $echo_inst (instantiate $echo_mod
            (with "libc" (instance $libc))
            (with "" (instance (export "echo" (func $echo_lower))))
        ))
        (func (export "echo2") (param "m" (map u32 u64)) (result (map u32 u64))
            (canon lift
                (core func $echo_inst "echo")
                (memory (core memory $libc "memory"))
                (realloc (core func $libc "realloc"))
            )
        )
    )

    (component $src
        (import "echo" (func $echo (param "m" (map u32 u64)) (result (map u32 u64))))
        (core module $libc
            (memory (export "memory") 1)
            {REALLOC_AND_FREE}
        )
        (core module $echo_mod
            (import "" "echo" (func $echo (param i32 i32 i32)))
            (import "libc" "memory" (memory 0))
            (import "libc" "realloc" (func $realloc (param i32 i32 i32 i32) (result i32)))

            (func (export "echo") (param i32 i32) (result i32)
                (local $retptr i32)
                (local.set $retptr
                    (call $realloc (i32.const 0) (i32.const 0) (i32.const 4) (i32.const 8)))
                (call $echo (local.get 0) (local.get 1) (local.get $retptr))
                local.get $retptr
            )
        )
        (core instance $libc (instantiate $libc))
        (core func $echo_lower (canon lower (func $echo)
            (memory (core memory $libc "memory"))
            (realloc (core func $libc "realloc"))
        ))
        (core instance $echo_inst (instantiate $echo_mod
            (with "libc" (instance $libc))
            (with "" (instance (export "echo" (func $echo_lower))))
        ))
        (func (export "echo2") (param "m" (map u32 u64)) (result (map u32 u64))
            (canon lift
                (core func $echo_inst "echo")
                (memory (core memory $libc "memory"))
                (realloc (core func $libc "realloc"))
            )
        )
    )

    (instance $dst (instantiate $dst (with "echo" (func $host))))
    (instance $src (instantiate $src (with "echo" (func $dst "echo2"))))
    (export "echo" (func $src "echo2"))
)
"#
    );

    let mut config = Config::new();
    config.wasm_component_model(true);
    config.wasm_component_model_map(true);
    let engine = Engine::new(&config)?;
    let component = Component::new(&engine, component)?;

    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);

    linker.root().func_new("host", |_cx, _ty, args, results| {
        results[0] = args[0].clone();
        Ok(())
    })?;

    let instance = linker.instantiate(&mut store, &component)?;
    let func = instance.get_func(&mut store, "echo").unwrap();

    let test_data = vec![
        (Val::U32(1), Val::U64(0x0102030405060708)),
        (Val::U32(2), Val::U64(0x1112131415161718)),
    ];
    let input = Val::Map(test_data.clone());

    let mut results = [Val::Bool(false)];
    func.call(&mut store, &[input], &mut results)?;

    match &results[0] {
        Val::Map(output) => {
            assert_eq!(output.len(), 2);
            for (key, value) in &test_data {
                assert!(
                    output.iter().any(|(k, v)| k == key && v == value),
                    "Missing or corrupted entry"
                );
            }
        }
        _ => panic!("expected map"),
    }

    Ok(())
}

/// Tests map<u8, u32> alignment through trampoline
#[test]
fn map_trampoline_alignment_u8_u32() -> Result<()> {
    let component = format!(
        r#"
(component
    (import "host" (func $host (param "m" (map u8 u32)) (result (map u8 u32))))

    (component $dst
        (import "echo" (func $echo (param "m" (map u8 u32)) (result (map u8 u32))))
        (core module $libc
            (memory (export "memory") 1)
            {REALLOC_AND_FREE}
        )
        (core module $echo_mod
            (import "" "echo" (func $echo (param i32 i32 i32)))
            (import "libc" "memory" (memory 0))
            (import "libc" "realloc" (func $realloc (param i32 i32 i32 i32) (result i32)))

            (func (export "echo") (param i32 i32) (result i32)
                (local $retptr i32)
                (local.set $retptr
                    (call $realloc (i32.const 0) (i32.const 0) (i32.const 4) (i32.const 8)))
                (call $echo (local.get 0) (local.get 1) (local.get $retptr))
                local.get $retptr
            )
        )
        (core instance $libc (instantiate $libc))
        (core func $echo_lower (canon lower (func $echo)
            (memory (core memory $libc "memory"))
            (realloc (core func $libc "realloc"))
        ))
        (core instance $echo_inst (instantiate $echo_mod
            (with "libc" (instance $libc))
            (with "" (instance (export "echo" (func $echo_lower))))
        ))
        (func (export "echo2") (param "m" (map u8 u32)) (result (map u8 u32))
            (canon lift
                (core func $echo_inst "echo")
                (memory (core memory $libc "memory"))
                (realloc (core func $libc "realloc"))
            )
        )
    )

    (component $src
        (import "echo" (func $echo (param "m" (map u8 u32)) (result (map u8 u32))))
        (core module $libc
            (memory (export "memory") 1)
            {REALLOC_AND_FREE}
        )
        (core module $echo_mod
            (import "" "echo" (func $echo (param i32 i32 i32)))
            (import "libc" "memory" (memory 0))
            (import "libc" "realloc" (func $realloc (param i32 i32 i32 i32) (result i32)))

            (func (export "echo") (param i32 i32) (result i32)
                (local $retptr i32)
                (local.set $retptr
                    (call $realloc (i32.const 0) (i32.const 0) (i32.const 4) (i32.const 8)))
                (call $echo (local.get 0) (local.get 1) (local.get $retptr))
                local.get $retptr
            )
        )
        (core instance $libc (instantiate $libc))
        (core func $echo_lower (canon lower (func $echo)
            (memory (core memory $libc "memory"))
            (realloc (core func $libc "realloc"))
        ))
        (core instance $echo_inst (instantiate $echo_mod
            (with "libc" (instance $libc))
            (with "" (instance (export "echo" (func $echo_lower))))
        ))
        (func (export "echo2") (param "m" (map u8 u32)) (result (map u8 u32))
            (canon lift
                (core func $echo_inst "echo")
                (memory (core memory $libc "memory"))
                (realloc (core func $libc "realloc"))
            )
        )
    )

    (instance $dst (instantiate $dst (with "echo" (func $host))))
    (instance $src (instantiate $src (with "echo" (func $dst "echo2"))))
    (export "echo" (func $src "echo2"))
)
"#
    );

    let mut config = Config::new();
    config.wasm_component_model(true);
    config.wasm_component_model_map(true);
    let engine = Engine::new(&config)?;
    let component = Component::new(&engine, component)?;

    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);

    linker.root().func_new("host", |_cx, _ty, args, results| {
        results[0] = args[0].clone();
        Ok(())
    })?;

    let instance = linker.instantiate(&mut store, &component)?;
    let func = instance.get_func(&mut store, "echo").unwrap();

    let test_data = vec![
        (Val::U8(1), Val::U32(0x01020304)),
        (Val::U8(2), Val::U32(0x11121314)),
    ];
    let input = Val::Map(test_data.clone());

    let mut results = [Val::Bool(false)];
    func.call(&mut store, &[input], &mut results)?;

    match &results[0] {
        Val::Map(output) => {
            assert_eq!(output.len(), 2);
            for (key, value) in &test_data {
                assert!(
                    output.iter().any(|(k, v)| k == key && v == value),
                    "Missing or corrupted entry"
                );
            }
        }
        _ => panic!("expected map"),
    }

    Ok(())
}

/// Tests map<u16, u64> alignment through trampoline
#[test]
fn map_trampoline_alignment_u16_u64() -> Result<()> {
    let component = format!(
        r#"
(component
    (import "host" (func $host (param "m" (map u16 u64)) (result (map u16 u64))))

    (component $dst
        (import "echo" (func $echo (param "m" (map u16 u64)) (result (map u16 u64))))
        (core module $libc
            (memory (export "memory") 1)
            {REALLOC_AND_FREE}
        )
        (core module $echo_mod
            (import "" "echo" (func $echo (param i32 i32 i32)))
            (import "libc" "memory" (memory 0))
            (import "libc" "realloc" (func $realloc (param i32 i32 i32 i32) (result i32)))

            (func (export "echo") (param i32 i32) (result i32)
                (local $retptr i32)
                (local.set $retptr
                    (call $realloc (i32.const 0) (i32.const 0) (i32.const 4) (i32.const 8)))
                (call $echo (local.get 0) (local.get 1) (local.get $retptr))
                local.get $retptr
            )
        )
        (core instance $libc (instantiate $libc))
        (core func $echo_lower (canon lower (func $echo)
            (memory (core memory $libc "memory"))
            (realloc (core func $libc "realloc"))
        ))
        (core instance $echo_inst (instantiate $echo_mod
            (with "libc" (instance $libc))
            (with "" (instance (export "echo" (func $echo_lower))))
        ))
        (func (export "echo2") (param "m" (map u16 u64)) (result (map u16 u64))
            (canon lift
                (core func $echo_inst "echo")
                (memory (core memory $libc "memory"))
                (realloc (core func $libc "realloc"))
            )
        )
    )

    (component $src
        (import "echo" (func $echo (param "m" (map u16 u64)) (result (map u16 u64))))
        (core module $libc
            (memory (export "memory") 1)
            {REALLOC_AND_FREE}
        )
        (core module $echo_mod
            (import "" "echo" (func $echo (param i32 i32 i32)))
            (import "libc" "memory" (memory 0))
            (import "libc" "realloc" (func $realloc (param i32 i32 i32 i32) (result i32)))

            (func (export "echo") (param i32 i32) (result i32)
                (local $retptr i32)
                (local.set $retptr
                    (call $realloc (i32.const 0) (i32.const 0) (i32.const 4) (i32.const 8)))
                (call $echo (local.get 0) (local.get 1) (local.get $retptr))
                local.get $retptr
            )
        )
        (core instance $libc (instantiate $libc))
        (core func $echo_lower (canon lower (func $echo)
            (memory (core memory $libc "memory"))
            (realloc (core func $libc "realloc"))
        ))
        (core instance $echo_inst (instantiate $echo_mod
            (with "libc" (instance $libc))
            (with "" (instance (export "echo" (func $echo_lower))))
        ))
        (func (export "echo2") (param "m" (map u16 u64)) (result (map u16 u64))
            (canon lift
                (core func $echo_inst "echo")
                (memory (core memory $libc "memory"))
                (realloc (core func $libc "realloc"))
            )
        )
    )

    (instance $dst (instantiate $dst (with "echo" (func $host))))
    (instance $src (instantiate $src (with "echo" (func $dst "echo2"))))
    (export "echo" (func $src "echo2"))
)
"#
    );

    let mut config = Config::new();
    config.wasm_component_model(true);
    config.wasm_component_model_map(true);
    let engine = Engine::new(&config)?;
    let component = Component::new(&engine, component)?;

    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);

    linker.root().func_new("host", |_cx, _ty, args, results| {
        results[0] = args[0].clone();
        Ok(())
    })?;

    let instance = linker.instantiate(&mut store, &component)?;
    let func = instance.get_func(&mut store, "echo").unwrap();

    let test_data = vec![
        (Val::U16(1), Val::U64(0x0102030405060708)),
        (Val::U16(2), Val::U64(0x1112131415161718)),
    ];
    let input = Val::Map(test_data.clone());

    let mut results = [Val::Bool(false)];
    func.call(&mut store, &[input], &mut results)?;

    match &results[0] {
        Val::Map(output) => {
            assert_eq!(output.len(), 2);
            for (key, value) in &test_data {
                assert!(
                    output.iter().any(|(k, v)| k == key && v == value),
                    "Missing or corrupted entry"
                );
            }
        }
        _ => panic!("expected map"),
    }

    Ok(())
}

/// Tests map<u8, u16> alignment through trampoline
#[test]
fn map_trampoline_alignment_u8_u16() -> Result<()> {
    let component = format!(
        r#"
(component
    (import "host" (func $host (param "m" (map u8 u16)) (result (map u8 u16))))

    (component $dst
        (import "echo" (func $echo (param "m" (map u8 u16)) (result (map u8 u16))))
        (core module $libc
            (memory (export "memory") 1)
            {REALLOC_AND_FREE}
        )
        (core module $echo_mod
            (import "" "echo" (func $echo (param i32 i32 i32)))
            (import "libc" "memory" (memory 0))
            (import "libc" "realloc" (func $realloc (param i32 i32 i32 i32) (result i32)))

            (func (export "echo") (param i32 i32) (result i32)
                (local $retptr i32)
                (local.set $retptr
                    (call $realloc (i32.const 0) (i32.const 0) (i32.const 4) (i32.const 8)))
                (call $echo (local.get 0) (local.get 1) (local.get $retptr))
                local.get $retptr
            )
        )
        (core instance $libc (instantiate $libc))
        (core func $echo_lower (canon lower (func $echo)
            (memory (core memory $libc "memory"))
            (realloc (core func $libc "realloc"))
        ))
        (core instance $echo_inst (instantiate $echo_mod
            (with "libc" (instance $libc))
            (with "" (instance (export "echo" (func $echo_lower))))
        ))
        (func (export "echo2") (param "m" (map u8 u16)) (result (map u8 u16))
            (canon lift
                (core func $echo_inst "echo")
                (memory (core memory $libc "memory"))
                (realloc (core func $libc "realloc"))
            )
        )
    )

    (component $src
        (import "echo" (func $echo (param "m" (map u8 u16)) (result (map u8 u16))))
        (core module $libc
            (memory (export "memory") 1)
            {REALLOC_AND_FREE}
        )
        (core module $echo_mod
            (import "" "echo" (func $echo (param i32 i32 i32)))
            (import "libc" "memory" (memory 0))
            (import "libc" "realloc" (func $realloc (param i32 i32 i32 i32) (result i32)))

            (func (export "echo") (param i32 i32) (result i32)
                (local $retptr i32)
                (local.set $retptr
                    (call $realloc (i32.const 0) (i32.const 0) (i32.const 4) (i32.const 8)))
                (call $echo (local.get 0) (local.get 1) (local.get $retptr))
                local.get $retptr
            )
        )
        (core instance $libc (instantiate $libc))
        (core func $echo_lower (canon lower (func $echo)
            (memory (core memory $libc "memory"))
            (realloc (core func $libc "realloc"))
        ))
        (core instance $echo_inst (instantiate $echo_mod
            (with "libc" (instance $libc))
            (with "" (instance (export "echo" (func $echo_lower))))
        ))
        (func (export "echo2") (param "m" (map u8 u16)) (result (map u8 u16))
            (canon lift
                (core func $echo_inst "echo")
                (memory (core memory $libc "memory"))
                (realloc (core func $libc "realloc"))
            )
        )
    )

    (instance $dst (instantiate $dst (with "echo" (func $host))))
    (instance $src (instantiate $src (with "echo" (func $dst "echo2"))))
    (export "echo" (func $src "echo2"))
)
"#
    );

    let mut config = Config::new();
    config.wasm_component_model(true);
    config.wasm_component_model_map(true);
    let engine = Engine::new(&config)?;
    let component = Component::new(&engine, component)?;

    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);

    linker.root().func_new("host", |_cx, _ty, args, results| {
        results[0] = args[0].clone();
        Ok(())
    })?;

    let instance = linker.instantiate(&mut store, &component)?;
    let func = instance.get_func(&mut store, "echo").unwrap();

    let test_data = vec![
        (Val::U8(1), Val::U16(0x0102)),
        (Val::U8(2), Val::U16(0x1112)),
    ];
    let input = Val::Map(test_data.clone());

    let mut results = [Val::Bool(false)];
    func.call(&mut store, &[input], &mut results)?;

    match &results[0] {
        Val::Map(output) => {
            assert_eq!(output.len(), 2);
            for (key, value) in &test_data {
                assert!(
                    output.iter().any(|(k, v)| k == key && v == value),
                    "Missing or corrupted entry"
                );
            }
        }
        _ => panic!("expected map"),
    }

    Ok(())
}

/// Tests map<u64, u8> alignment through trampoline (reverse case: key larger than value)
#[test]
fn map_trampoline_alignment_u64_u8() -> Result<()> {
    let component = format!(
        r#"
(component
    (import "host" (func $host (param "m" (map u64 u8)) (result (map u64 u8))))

    (component $dst
        (import "echo" (func $echo (param "m" (map u64 u8)) (result (map u64 u8))))
        (core module $libc
            (memory (export "memory") 1)
            {REALLOC_AND_FREE}
        )
        (core module $echo_mod
            (import "" "echo" (func $echo (param i32 i32 i32)))
            (import "libc" "memory" (memory 0))
            (import "libc" "realloc" (func $realloc (param i32 i32 i32 i32) (result i32)))

            (func (export "echo") (param i32 i32) (result i32)
                (local $retptr i32)
                (local.set $retptr
                    (call $realloc (i32.const 0) (i32.const 0) (i32.const 4) (i32.const 8)))
                (call $echo (local.get 0) (local.get 1) (local.get $retptr))
                local.get $retptr
            )
        )
        (core instance $libc (instantiate $libc))
        (core func $echo_lower (canon lower (func $echo)
            (memory (core memory $libc "memory"))
            (realloc (core func $libc "realloc"))
        ))
        (core instance $echo_inst (instantiate $echo_mod
            (with "libc" (instance $libc))
            (with "" (instance (export "echo" (func $echo_lower))))
        ))
        (func (export "echo2") (param "m" (map u64 u8)) (result (map u64 u8))
            (canon lift
                (core func $echo_inst "echo")
                (memory (core memory $libc "memory"))
                (realloc (core func $libc "realloc"))
            )
        )
    )

    (component $src
        (import "echo" (func $echo (param "m" (map u64 u8)) (result (map u64 u8))))
        (core module $libc
            (memory (export "memory") 1)
            {REALLOC_AND_FREE}
        )
        (core module $echo_mod
            (import "" "echo" (func $echo (param i32 i32 i32)))
            (import "libc" "memory" (memory 0))
            (import "libc" "realloc" (func $realloc (param i32 i32 i32 i32) (result i32)))

            (func (export "echo") (param i32 i32) (result i32)
                (local $retptr i32)
                (local.set $retptr
                    (call $realloc (i32.const 0) (i32.const 0) (i32.const 4) (i32.const 8)))
                (call $echo (local.get 0) (local.get 1) (local.get $retptr))
                local.get $retptr
            )
        )
        (core instance $libc (instantiate $libc))
        (core func $echo_lower (canon lower (func $echo)
            (memory (core memory $libc "memory"))
            (realloc (core func $libc "realloc"))
        ))
        (core instance $echo_inst (instantiate $echo_mod
            (with "libc" (instance $libc))
            (with "" (instance (export "echo" (func $echo_lower))))
        ))
        (func (export "echo2") (param "m" (map u64 u8)) (result (map u64 u8))
            (canon lift
                (core func $echo_inst "echo")
                (memory (core memory $libc "memory"))
                (realloc (core func $libc "realloc"))
            )
        )
    )

    (instance $dst (instantiate $dst (with "echo" (func $host))))
    (instance $src (instantiate $src (with "echo" (func $dst "echo2"))))
    (export "echo" (func $src "echo2"))
)
"#
    );

    let mut config = Config::new();
    config.wasm_component_model(true);
    config.wasm_component_model_map(true);
    let engine = Engine::new(&config)?;
    let component = Component::new(&engine, component)?;

    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);

    linker.root().func_new("host", |_cx, _ty, args, results| {
        results[0] = args[0].clone();
        Ok(())
    })?;

    let instance = linker.instantiate(&mut store, &component)?;
    let func = instance.get_func(&mut store, "echo").unwrap();

    let test_data = vec![
        (Val::U64(0x0102030405060708), Val::U8(42)),
        (Val::U64(0x1112131415161718), Val::U8(99)),
    ];
    let input = Val::Map(test_data.clone());

    let mut results = [Val::Bool(false)];
    func.call(&mut store, &[input], &mut results)?;

    match &results[0] {
        Val::Map(output) => {
            assert_eq!(output.len(), 2);
            for (key, value) in &test_data {
                assert!(
                    output.iter().any(|(k, v)| k == key && v == value),
                    "Missing or corrupted entry"
                );
            }
        }
        _ => panic!("expected map"),
    }

    Ok(())
}

#[test]
fn typed_map_roundtrip() -> Result<()> {
    use std::collections::HashMap;

    let engine = super::map_engine();
    let mut store = Store::new(&engine, ());
    let component = Component::new(&engine, super::make_echo_component("(map string u32)", 8))?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let echo = instance
        .get_typed_func::<(&HashMap<String, u32>,), (HashMap<String, u32>,)>(&mut store, "echo")?;

    for input in [
        HashMap::new(),
        HashMap::from([("a".to_string(), 1)]),
        (0..100).map(|i| (format!("key{i}"), i)).collect(),
    ] {
        let (output,) = echo.call(&mut store, (&input,))?;
        assert_eq!(output, input);
    }

    let component = Component::new(
        &engine,
        super::make_echo_component("(map string (map u8 (list string)))", 8),
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    type Nested = HashMap<String, HashMap<u8, Vec<String>>>;
    let echo = instance.get_typed_func::<(&Nested,), (Nested,)>(&mut store, "echo")?;

    let input: Nested = HashMap::from([
        ("empty".to_string(), HashMap::new()),
        (
            "full".to_string(),
            HashMap::from([(1, vec!["a".to_string(), "b".to_string()]), (2, vec![])]),
        ),
    ]);
    let (output,) = echo.call(&mut store, (&input,))?;
    assert_eq!(output, input);
    Ok(())
}

#[test]
fn typed_map_host_function() -> Result<()> {
    use std::collections::HashMap;

    let component = format!(
        r#"
(component
    (import "host" (func $host (param "m" (map string u32)) (result (map string u32))))
    (core module $libc
        (memory (export "memory") 1)
        {REALLOC_AND_FREE}
    )
    (core instance $libc (instantiate $libc))
    (core func $host_lower (canon lower (func $host)
        (memory (core memory $libc "memory"))
        (realloc (core func $libc "realloc"))
    ))
    (core module $m
        (import "" "host" (func $host (param i32 i32 i32)))
        (import "libc" "realloc" (func $realloc (param i32 i32 i32 i32) (result i32)))
        (func (export "echo") (param i32 i32) (result i32)
            (local $retptr i32)
            (local.set $retptr
                (call $realloc (i32.const 0) (i32.const 0) (i32.const 4) (i32.const 8)))
            (call $host (local.get 0) (local.get 1) (local.get $retptr))
            local.get $retptr
        )
    )
    (core instance $i (instantiate $m
        (with "libc" (instance $libc))
        (with "" (instance (export "host" (func $host_lower))))
    ))
    (func (export "echo") (param "m" (map string u32)) (result (map string u32))
        (canon lift
            (core func $i "echo")
            (memory (core memory $libc "memory"))
            (realloc (core func $libc "realloc"))
        )
    )
)
"#
    );

    let engine = super::map_engine();
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);
    linker.root().func_wrap(
        "host",
        |_cx, (mut m,): (HashMap<String, u32>,)| -> Result<(HashMap<String, u32>,)> {
            m.insert("from-host".to_string(), 42);
            Ok((m,))
        },
    )?;
    let instance = linker.instantiate(&mut store, &component)?;
    let echo = instance
        .get_typed_func::<(&HashMap<String, u32>,), (HashMap<String, u32>,)>(&mut store, "echo")?;

    let input = HashMap::from([("a".to_string(), 1), ("b".to_string(), 2)]);
    let (output,) = echo.call(&mut store, (&input,))?;
    let mut expected = input.clone();
    expected.insert("from-host".to_string(), 42);
    assert_eq!(output, expected);
    Ok(())
}
