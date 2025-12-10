use wasmtime::component::{Component, FutureAny, FutureReader, Linker, StreamAny, StreamReader};
use wasmtime::{Config, Engine, Result, Store};

#[test]
fn simple_type_conversions() {
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());

    let f = FutureReader::new(&mut store, async { anyhow::Ok(10_u32) });
    let f = f.try_into_future_any(&mut store).unwrap();
    assert!(f.clone().try_into_future_reader::<()>().is_err());
    assert!(f.clone().try_into_future_reader::<u64>().is_err());
    let f = f.try_into_future_reader::<u32>().unwrap();
    f.try_into_future_any(&mut store).unwrap().close(&mut store);

    let s = StreamReader::new(&mut store, vec![10_u32]);
    let s = s.try_into_stream_any(&mut store).unwrap();
    assert!(s.clone().try_into_stream_reader::<()>().is_err());
    assert!(s.clone().try_into_stream_reader::<u64>().is_err());
    let s = s.try_into_stream_reader::<u32>().unwrap();
    s.try_into_stream_any(&mut store).unwrap().close(&mut store);
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
        f_t_t.post_return(&mut *store)?;
        let (f,) = f_t_a.call(&mut *store, (f,))?;
        f_t_a.post_return(&mut *store)?;
        let (f,) = f_a_a.call(&mut *store, (f,))?;
        f_a_a.post_return(&mut *store)?;
        let (mut f,) = f_a_t.call(&mut *store, (f,))?;
        f_a_t.post_return(&mut *store)?;
        f.close(&mut *store);
        Ok(())
    };

    let f = FutureReader::new(&mut store, async { anyhow::Ok(10_u32) });
    roundtrip(&mut store, f)?;

    let (f,) = mk_f_t.call(&mut store, ())?;
    mk_f_t.post_return(&mut store)?;
    roundtrip(&mut store, f)?;

    let (f,) = mk_f_a.call(&mut store, ())?;
    mk_f_a.post_return(&mut store)?;
    let f = f.try_into_future_reader::<u32>()?;
    roundtrip(&mut store, f)?;

    let roundtrip = |store: &mut Store<()>, s: StreamReader<u32>| -> Result<()> {
        let (s,) = s_t_t.call(&mut *store, (s,))?;
        s_t_t.post_return(&mut *store)?;
        let (s,) = s_t_a.call(&mut *store, (s,))?;
        s_t_a.post_return(&mut *store)?;
        let (s,) = s_a_a.call(&mut *store, (s,))?;
        s_a_a.post_return(&mut *store)?;
        let (mut s,) = s_a_t.call(&mut *store, (s,))?;
        s_a_t.post_return(&mut *store)?;
        s.close(&mut *store);
        Ok(())
    };

    let s = StreamReader::new(&mut store, vec![10_u32]);
    roundtrip(&mut store, s)?;

    let (s,) = mk_s_t.call(&mut store, ())?;
    mk_s_t.post_return(&mut store)?;
    roundtrip(&mut store, s)?;

    let (s,) = mk_s_a.call(&mut store, ())?;
    mk_s_a.post_return(&mut store)?;
    let s = s.try_into_stream_reader::<u32>()?;
    roundtrip(&mut store, s)?;

    Ok(())
}
