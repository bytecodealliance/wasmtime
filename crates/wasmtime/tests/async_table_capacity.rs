use wasmtime::component::{FutureReader, ResourceTableError, StreamReader};
use wasmtime::{Config, Engine, Result, Store};

fn engine() -> Result<Engine> {
    let mut config = Config::new();
    config.wasm_component_model_async(true);
    Engine::new(&config)
}

fn len<T>(store: &mut Store<T>) -> usize {
    store.concurrent_resource_table().unwrap().len()
}

fn set_capacity<T>(store: &mut Store<T>, capacity: usize) {
    store
        .concurrent_resource_table()
        .unwrap()
        .set_max_capacity(capacity);
}

fn assert_full(error: wasmtime::Error) {
    assert!(
        matches!(
            error.downcast_ref::<ResourceTableError>(),
            Some(ResourceTableError::Full)
        ),
        "{error:?}"
    );
}

#[test]
fn host_transmits_refuse_without_retaining_partial_allocations() -> Result<()> {
    let engine = engine()?;
    for capacity in 0..3 {
        let mut store = Store::new(&engine, ());
        set_capacity(&mut store, capacity);
        for _ in 0..2 {
            assert_full(StreamReader::new(&mut store, Vec::<u8>::new()).unwrap_err());
            assert_eq!(len(&mut store), 0);
            assert_full(
                FutureReader::new(&mut store, async { wasmtime::error::Ok(0_u8) }).unwrap_err(),
            );
            assert_eq!(len(&mut store), 0);
        }
        set_capacity(&mut store, 3);
        for _ in 0..2 {
            let mut stream = StreamReader::new(&mut store, Vec::<u8>::new())?;
            assert_eq!(len(&mut store), 3);
            assert_full(
                FutureReader::new(&mut store, async { wasmtime::error::Ok(0_u8) }).unwrap_err(),
            );
            assert_eq!(len(&mut store), 3);
            stream.close(&mut store)?;
            assert_eq!(len(&mut store), 0);
            let mut future = FutureReader::new(&mut store, async { wasmtime::error::Ok(0_u8) })?;
            assert_eq!(len(&mut store), 3);
            future.close(&mut store)?;
            assert_eq!(len(&mut store), 0);
        }
    }
    Ok(())
}
