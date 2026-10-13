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

fn check_capacity<R>(
    new: impl Fn(&mut Store<()>) -> Result<R>,
    close: impl Fn(&mut R, &mut Store<()>) -> Result<()>,
) -> Result<()> {
    let engine = engine()?;
    let mut probe = Store::new(&engine, ());
    let mut resource = new(&mut probe)?;
    let required = len(&mut probe);
    assert!(required > 0);
    close(&mut resource, &mut probe)?;
    assert_eq!(len(&mut probe), 0);

    for capacity in 0..required {
        // A fresh store has no previously allocated slots that could be reused
        // even after lowering the capacity.
        let mut store = Store::new(&engine, ());
        set_capacity(&mut store, capacity);
        for _ in 0..2 {
            let error = match new(&mut store) {
                Ok(_) => panic!("allocation succeeded with insufficient capacity"),
                Err(error) => error,
            };
            assert_full(error);
            assert_eq!(len(&mut store), 0);
        }
        set_capacity(&mut store, required);
        for _ in 0..2 {
            let mut resource = new(&mut store)?;
            close(&mut resource, &mut store)?;
            assert_eq!(len(&mut store), 0);
        }
    }
    Ok(())
}

#[test]
fn host_transmits_refuse_without_retaining_partial_allocations() -> Result<()> {
    check_capacity(
        |store| StreamReader::new(store, Vec::<u8>::new()),
        |reader, store| reader.close(store),
    )?;
    check_capacity(
        |store| FutureReader::new(store, async { wasmtime::error::Ok(0_u8) }),
        |reader, store| reader.close(store),
    )
}
