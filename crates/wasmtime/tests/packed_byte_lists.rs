#![cfg(all(
    feature = "component-model",
    feature = "cranelift",
    feature = "runtime",
    feature = "wat"
))]

use wasmtime::component::{Component, Linker, Val};
use wasmtime::{Engine, Result, Store};

#[test]
fn packed_byte_lists_lower_to_bytes_and_lift_as_ordinary_lists() -> Result<()> {
    let engine = Engine::default();
    let component = Component::new(
        &engine,
        r#"
        (component
          (type $bytes (list u8))
          (type $reply (result $bytes (error u32)))
          (import "fetch" (func $fetch (result $reply)))
          (core module $m
            (memory (export "memory") 1)
            (func (export "realloc") (param i32 i32 i32 i32) (result i32) i32.const 32)
            (func (export "echo") (param i32 i32) (result i32)
              i32.const 0 local.get 0 i32.store
              i32.const 4 local.get 1 i32.store
              i32.const 0)
            (func (export "take-words") (param i32 i32)))
          (core instance $i (instantiate $m))
          (alias core export $i "memory" (core memory $memory))
          (alias core export $i "realloc" (core func $realloc))
          (core func $fetch-core
            (canon lower (func $fetch) (memory $memory) (realloc $realloc)))
          (core module $relay
            (import "host" "fetch" (func $fetch (param i32)))
            (func (export "fetch") (result i32)
              i32.const 0 call $fetch
              i32.const 0))
          (core instance $relay (instantiate $relay
            (with "host" (instance (export "fetch" (func $fetch-core))))))
          (func (export "fetch") (result $reply)
            (canon lift (core func $relay "fetch") (memory $memory)))
          (func (export "echo") (param "bytes" $bytes) (result $bytes)
            (canon lift (core func $i "echo") (memory $memory) (realloc $realloc)))
          (func (export "take-words") (param "words" (list u32))
            (canon lift (core func $i "take-words") (memory $memory) (realloc $realloc))))
        "#,
    )?;
    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);
    linker.root().func_new("fetch", |_, _, _, results| {
        results[0] = Val::Result(Ok(Some(Box::new(Val::ListU8(vec![13, 42, 199])))));
        Ok(())
    })?;
    let instance = linker.instantiate(&mut store, &component)?;
    let echo = instance.get_func(&mut store, "echo").unwrap();
    for bytes in [vec![], vec![0, 128, 255]] {
        let expected = Val::List(bytes.iter().copied().map(Val::U8).collect());
        let packed = Val::ListU8(bytes);
        let mut results = [Val::Bool(false)];
        echo.call(&mut store, std::slice::from_ref(&packed), &mut results)?;
        assert!(matches!(results[0], Val::List(_)));
        assert_eq!(results[0], expected);
        assert_eq!(expected, packed);
        #[cfg(feature = "wave")]
        assert_eq!(packed.to_wave()?, expected.to_wave()?);
    }

    let fetch = instance.get_func(&mut store, "fetch").unwrap();
    let mut results = [Val::Bool(false)];
    fetch.call(&mut store, &[], &mut results)?;
    let Val::Result(Ok(Some(bytes))) = &results[0] else {
        panic!("expected success with bytes");
    };
    let Val::List(bytes) = bytes.as_ref() else {
        panic!("expected an ordinary list");
    };
    assert_eq!(bytes, &[Val::U8(13), Val::U8(42), Val::U8(199)]);

    let take_words = instance.get_func(&mut store, "take-words").unwrap();
    let error = take_words
        .call(&mut store, &[Val::ListU8(vec![])], &mut [])
        .unwrap_err();
    assert!(error.to_string().contains("type mismatch"), "{error}");
    Ok(())
}
