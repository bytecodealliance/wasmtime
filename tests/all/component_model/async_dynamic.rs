use wasmtime::component::{
    Component, FutureAny, FutureReader, Linker, StreamAny, StreamReader, Val,
};
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
    (core func $waitable-set.wait (canon waitable-set.wait (memory (core memory $libc "mem"))))
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
        (canon lift (core func $i "run") async (callback (core func $i "cb"))))
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

const TAKE_FUTURES_AND_STREAMS: &str = r#"
(component
    (type $f (future u32))
    (type $s (stream u32))
    (core module $m
        (func (export "take1") (param i32))
        (func (export "take2") (param i32 i32)))
    (core instance $i (instantiate $m))
    (func (export "take1-f") (param "a" $f)
        (canon lift (core func $i "take1")))
    (func (export "take2-f") (param "a" $f) (param "b" $f)
        (canon lift (core func $i "take2")))
    (func (export "take1-s") (param "a" $s)
        (canon lift (core func $i "take1")))
    (func (export "take2-s") (param "a" $s) (param "b" $s)
        (canon lift (core func $i "take2")))
)
"#;

#[test]
#[cfg_attr(miri, ignore)]
fn future_and_stream_lowered_at_most_once() -> Result<()> {
    let mut config = Config::new();
    config.wasm_component_model_async(true);
    let engine = Engine::new(&config)?;
    let component = Component::new(&engine, TAKE_FUTURES_AND_STREAMS)?;
    let linker = Linker::new(&engine);
    let new_store = || Store::new(&engine, ());

    // Two clones of one `FutureAny` passed to the same call.
    {
        let mut store = new_store();
        let instance = linker.instantiate(&mut store, &component)?;
        let take2 = instance.get_typed_func::<(FutureAny, FutureAny), ()>(&mut store, "take2-f")?;
        let f = FutureReader::new(&mut store, async { wasmtime::error::Ok(7_u32) })?
            .try_into_future_any(&mut store)?;
        let mut g = f.clone();
        assert!(take2.call(&mut store, (f, g.clone())).is_err());
        assert!(g.close(&mut store).is_err());
    }

    // Two clones of one `StreamAny` passed to the same call.
    {
        let mut store = new_store();
        let instance = linker.instantiate(&mut store, &component)?;
        let take2 = instance.get_typed_func::<(StreamAny, StreamAny), ()>(&mut store, "take2-s")?;
        let s = StreamReader::new(&mut store, vec![7_u32])?.try_into_stream_any(&mut store)?;
        let mut t = s.clone();
        assert!(take2.call(&mut store, (s, t.clone())).is_err());
        assert!(t.close(&mut store).is_err());
    }

    // The same `Val::Future` lowered in two calls.
    {
        let mut store = new_store();
        let instance = linker.instantiate(&mut store, &component)?;
        let take1 = instance.get_func(&mut store, "take1-f").unwrap();
        let f = FutureReader::new(&mut store, async { wasmtime::error::Ok(7_u32) })?
            .try_into_future_any(&mut store)?;
        let f = Val::Future(f);
        take1.call(&mut store, &[f.clone()], &mut [])?;
        assert!(take1.call(&mut store, &[f.clone()], &mut []).is_err());
        let Val::Future(mut f) = f else {
            unreachable!()
        };
        assert!(f.close(&mut store).is_err());
    }

    // The same `Val::Stream` lowered in two calls.
    {
        let mut store = new_store();
        let instance = linker.instantiate(&mut store, &component)?;
        let take1 = instance.get_func(&mut store, "take1-s").unwrap();
        let s = StreamReader::new(&mut store, vec![7_u32])?.try_into_stream_any(&mut store)?;
        let s = Val::Stream(s);
        take1.call(&mut store, &[s.clone()], &mut [])?;
        assert!(take1.call(&mut store, &[s.clone()], &mut []).is_err());
        let Val::Stream(mut s) = s else {
            unreachable!()
        };
        assert!(s.close(&mut store).is_err());
    }

    // A `FutureReader` lowered twice by reference.
    {
        let mut store = new_store();
        let instance = linker.instantiate(&mut store, &component)?;
        let take1 = instance.get_typed_func::<(&FutureReader<u32>,), ()>(&mut store, "take1-f")?;
        let mut f = FutureReader::new(&mut store, async { wasmtime::error::Ok(7_u32) })?;
        take1.call(&mut store, (&f,))?;
        assert!(take1.call(&mut store, (&f,)).is_err());
        assert!(f.close(&mut store).is_err());
    }

    // A `StreamReader` lowered twice by reference.
    {
        let mut store = new_store();
        let instance = linker.instantiate(&mut store, &component)?;
        let take1 = instance.get_typed_func::<(&StreamReader<u32>,), ()>(&mut store, "take1-s")?;
        let mut s = StreamReader::new(&mut store, vec![7_u32])?;
        take1.call(&mut store, (&s,))?;
        assert!(take1.call(&mut store, (&s,)).is_err());
        assert!(s.close(&mut store).is_err());
    }

    // Closing clones more than once.
    {
        let mut store = new_store();
        let mut f = FutureReader::new(&mut store, async { wasmtime::error::Ok(7_u32) })?
            .try_into_future_any(&mut store)?;
        let mut g = f.clone();
        f.close(&mut store)?;
        assert!(g.close(&mut store).is_err());
        assert!(f.close(&mut store).is_err());
    }

    // A stale clone must not alias a new future which reuses the old one's
    // table slot.
    {
        let mut store = new_store();
        let instance = linker.instantiate(&mut store, &component)?;
        let take1 = instance.get_typed_func::<(&FutureAny,), ()>(&mut store, "take1-f")?;
        let f = FutureReader::new(&mut store, async { wasmtime::error::Ok(7_u32) })?
            .try_into_future_any(&mut store)?;
        f.clone().close(&mut store)?;
        let mut g = FutureReader::new(&mut store, async { wasmtime::error::Ok(8_u32) })?
            .try_into_future_any(&mut store)?;
        assert!(take1.call(&mut store, (&f,)).is_err());
        g.close(&mut store)?;
    }

    Ok(())
}
