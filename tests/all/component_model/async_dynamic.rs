use wasmtime::component::{Component, FutureAny, FutureReader, Linker, StreamAny, StreamReader};
use wasmtime::{Config, Engine, Result, Store};

#[test]
fn simple_type_conversions() -> Result<()> {
    let mut config = Config::new();
    config.wasm_component_model_async(true);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());

    let f = FutureReader::new(&mut store, async { wasmtime::error::Ok(10_u32) })?;
    let f = f.try_into_future_any(&mut store).unwrap();
    assert!(f.clone().try_into_future_reader::<()>().is_err());
    assert!(f.clone().try_into_future_reader::<u64>().is_err());
    let f = f.try_into_future_reader::<u32>().unwrap();
    f.try_into_future_any(&mut store)
        .unwrap()
        .close(&mut store)?;

    let s = StreamReader::new(&mut store, vec![10_u32])?;
    let s = s.try_into_stream_any(&mut store).unwrap();
    assert!(s.clone().try_into_stream_reader::<()>().is_err());
    assert!(s.clone().try_into_stream_reader::<u64>().is_err());
    let s = s.try_into_stream_reader::<u32>().unwrap();
    s.try_into_stream_any(&mut store)
        .unwrap()
        .close(&mut store)?;

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn simple_type_assertions() -> Result<()> {
    let mut config = Config::new();
    config.wasm_component_model_async(true);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());

    let component = Component::new(
        &engine,
        r#"
        (component
            (type $f (future u32))
            (type $s (stream u32))
            (core func $mk-f (canon future.new $f))
            (core func $mk-s (canon stream.new $s))

            (core module $m
                (import "" "mk-f" (func $mk-f (result i64)))
                (import "" "mk-s" (func $mk-s (result i64)))

                (func (export "x") (param i32) (result i32) local.get 0)

                (func (export "mk-f") (result i32)
                    (i32.wrap_i64 (call $mk-f)))
                (func (export "mk-s") (result i32)
                    (i32.wrap_i64 (call $mk-s)))
            )
            (core instance $i (instantiate $m
                (with "" (instance
                    (export "mk-f" (func $mk-f))
                    (export "mk-s" (func $mk-s))
                ))
            ))
            (func (export "f") (param "f" $f) (result $f)
                (canon lift (core func $i "x")))
            (func (export "s") (param "s" $s) (result $s)
                (canon lift (core func $i "x")))
            (func (export "mk-f") (result $f)
                (canon lift (core func $i "mk-f")))
            (func (export "mk-s") (result $s)
                (canon lift (core func $i "mk-s")))
        )
        "#,
    )?;

    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;

    let f_t_t =
        instance.get_typed_func::<(FutureReader<u32>,), (FutureReader<u32>,)>(&mut store, "f")?;
    let f_t_a = instance.get_typed_func::<(FutureReader<u32>,), (FutureAny,)>(&mut store, "f")?;
    let f_a_t = instance.get_typed_func::<(FutureAny,), (FutureReader<u32>,)>(&mut store, "f")?;
    let f_a_a = instance.get_typed_func::<(FutureAny,), (FutureAny,)>(&mut store, "f")?;

    let s_t_t =
        instance.get_typed_func::<(StreamReader<u32>,), (StreamReader<u32>,)>(&mut store, "s")?;
    let s_t_a = instance.get_typed_func::<(StreamReader<u32>,), (StreamAny,)>(&mut store, "s")?;
    let s_a_t = instance.get_typed_func::<(StreamAny,), (StreamReader<u32>,)>(&mut store, "s")?;
    let s_a_a = instance.get_typed_func::<(StreamAny,), (StreamAny,)>(&mut store, "s")?;

    let mk_f_t = instance.get_typed_func::<(), (FutureReader<u32>,)>(&mut store, "mk-f")?;
    let mk_f_a = instance.get_typed_func::<(), (FutureAny,)>(&mut store, "mk-f")?;
    let mk_s_t = instance.get_typed_func::<(), (StreamReader<u32>,)>(&mut store, "mk-s")?;
    let mk_s_a = instance.get_typed_func::<(), (StreamAny,)>(&mut store, "mk-s")?;

    assert!(instance.get_typed_func::<(), ()>(&mut store, "f").is_err());
    assert!(
        instance
            .get_typed_func::<(u32,), (FutureReader<u32>,)>(&mut store, "f")
            .is_err()
    );
    assert!(
        instance
            .get_typed_func::<(FutureReader<u32>,), (u32,)>(&mut store, "f")
            .is_err()
    );
    assert!(
        instance
            .get_typed_func::<(FutureReader<()>,), (FutureReader<u32>,)>(&mut store, "f")
            .is_err()
    );
    assert!(
        instance
            .get_typed_func::<(FutureReader<u64>,), (FutureReader<u32>,)>(&mut store, "f")
            .is_err()
    );

    assert!(instance.get_typed_func::<(), ()>(&mut store, "s").is_err());
    assert!(
        instance
            .get_typed_func::<(u32,), (StreamReader<u32>,)>(&mut store, "s")
            .is_err()
    );
    assert!(
        instance
            .get_typed_func::<(StreamReader<u32>,), (u32,)>(&mut store, "s")
            .is_err()
    );
    assert!(
        instance
            .get_typed_func::<(StreamReader<()>,), (StreamReader<u32>,)>(&mut store, "s")
            .is_err()
    );
    assert!(
        instance
            .get_typed_func::<(StreamReader<u64>,), (StreamReader<u32>,)>(&mut store, "s")
            .is_err()
    );

    let roundtrip = |store: &mut Store<()>, f: FutureReader<u32>| -> Result<()> {
        let (f,) = f_t_t.call(&mut *store, (f,))?;
        let (f,) = f_t_a.call(&mut *store, (f,))?;
        let (f,) = f_a_a.call(&mut *store, (f,))?;
        let (mut f,) = f_a_t.call(&mut *store, (f,))?;
        f.close(&mut *store)?;
        Ok(())
    };

    let f = FutureReader::new(&mut store, async { wasmtime::error::Ok(10_u32) })?;
    roundtrip(&mut store, f)?;

    let (f,) = mk_f_t.call(&mut store, ())?;
    roundtrip(&mut store, f)?;

    let (f,) = mk_f_a.call(&mut store, ())?;
    let f = f.try_into_future_reader::<u32>()?;
    roundtrip(&mut store, f)?;

    let roundtrip = |store: &mut Store<()>, s: StreamReader<u32>| -> Result<()> {
        let (s,) = s_t_t.call(&mut *store, (s,))?;
        let (s,) = s_t_a.call(&mut *store, (s,))?;
        let (s,) = s_a_a.call(&mut *store, (s,))?;
        let (mut s,) = s_a_t.call(&mut *store, (s,))?;
        s.close(&mut *store)?;
        Ok(())
    };

    let s = StreamReader::new(&mut store, vec![10_u32])?;
    roundtrip(&mut store, s)?;

    let (s,) = mk_s_t.call(&mut store, ())?;
    roundtrip(&mut store, s)?;

    let (s,) = mk_s_a.call(&mut store, ())?;
    let s = s.try_into_stream_reader::<u32>()?;
    roundtrip(&mut store, s)?;

    Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn stream_any_smoke() -> Result<()> {
    let mut config = Config::new();
    config.wasm_component_model_async(true);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());
    let component = Component::new(
        &engine,
        r#"
(component
    (type $s (stream u8))

    (core module $libc (memory (export "mem") 1))
    (core instance $libc (instantiate $libc))

    (core module $m
        (import "" "stream.new" (func $stream.new (result i64)))
        (import "" "task.return" (func $task.return))
        (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
        (import "" "waitable.join" (func $waitable.join (param i32 i32)))
        (import "" "waitable-set.wait" (func $waitable-set.wait (param i32 i32) (result i32)))
        (import "" "waitable-set.drop" (func $waitable-set.drop (param i32)))
        (import "" "mem" (memory 1))

        (global $w (mut i32) (i32.const 0))

        (func (export "mk") (result i32)
            (local $r i32) (local $tmp i64)
            (local.set $tmp (call $stream.new))
            (local.set $r (i32.wrap_i64 (local.get $tmp)))
            (global.set $w (i32.wrap_i64 (i64.shr_u (local.get $tmp) (i64.const 32))))
            local.get $r
        )

        (func (export "run") (result i32)
            (local $ws i32)
            (local.set $ws (call $waitable-set.new))
            (call $waitable.join (global.get $w) (local.get $ws))
            (call $waitable-set.wait (local.get $ws) (i32.const 0))
            i32.const 3 ;; EVENT_STREAM_WRITE
            i32.ne
            if unreachable end

            (if (i32.ne (i32.load (i32.const 0)) (global.get $w))
              (then unreachable))
            (if (i32.ne (i32.load (i32.const 4)) (i32.const 1)) ;; DROPPED | (0 << 4)
              (then unreachable))

            call $task.return

            i32.const 0 ;; CALLBACK_CODE_EXIT
        )

        (func (export "cb") (param i32 i32 i32) (result i32) unreachable)
    )
    (core func $stream.new (canon stream.new $s))
    (core func $task.return (canon task.return))
    (core func $waitable-set.new (canon waitable-set.new))
    (core func $waitable.join (canon waitable.join))
    (core func $waitable-set.wait (canon waitable-set.wait (memory $libc "mem")))
    (core func $waitable-set.drop (canon waitable-set.drop))
    (core instance $i (instantiate $m
        (with "" (instance
            (export "stream.new" (func $stream.new))
            (export "task.return" (func $task.return))
            (export "waitable-set.new" (func $waitable-set.new))
            (export "waitable.join" (func $waitable.join))
            (export "waitable-set.wait" (func $waitable-set.wait))
            (export "waitable-set.drop" (func $waitable-set.drop))
            (export "mem" (memory $libc "mem"))
        ))
    ))
    (func (export "mk") (result (stream u8))
        (canon lift (core func $i "mk")))
    (func (export "run") async
        (canon lift (core func $i "run") async (callback (func $i "cb"))))
)
        "#,
    )?;
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let mk = instance.get_typed_func::<(), (StreamAny,)>(&mut store, "mk")?;
    let run = instance.get_typed_func::<(), ()>(&mut store, "run")?;
    store
        .run_concurrent(async |store| {
            let (mut stream,) = mk.call_concurrent(store, ()).await?;
            tokio::try_join! {
                async {
                    run.call_concurrent(store, ()).await?;
                    wasmtime::error::Ok(())
                },
                async {
                    store.with(|store| stream.close(store))?;
                    wasmtime::error::Ok(())
                }
            }?;
            wasmtime::error::Ok(())
        })
        .await??;
    Ok(())
}
