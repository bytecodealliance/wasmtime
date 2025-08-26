use wasmtime::*;

#[test]
fn always_pop_i31ref_lifo_roots() -> Result<()> {
    let mut config = Config::new();
    config.wasm_function_references(true);
    config.wasm_gc(true);

    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());

    let anyref = {
        let mut scope = RootScope::new(&mut store);
        AnyRef::from_i31(&mut scope, I31::wrapping_u32(42))
    };

    // The anyref has left its rooting scope and been unrooted.
    assert!(anyref.is_i31(&store).is_err());

    Ok(())
}

#[test]
fn i31ref_to_raw_round_trip() -> Result<()> {
    let mut config = Config::new();
    config.wasm_function_references(true);
    config.wasm_gc(true);

    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());

    // Should be able to round trip an `i31ref` to its raw representation and
    // back again even though we have not forced the allocation of the `GcStore`
    // yet.
    let anyref = AnyRef::from_i31(&mut store, I31::wrapping_u32(42));
    let raw = anyref.to_raw(&mut store)?;
    let anyref = AnyRef::from_raw(&mut store, raw).expect("should be non-null");
    assert!(anyref.is_i31(&store)?);
    assert_eq!(anyref.as_i31(&store)?.unwrap().get_u32(), 42);

    Ok(())
}
