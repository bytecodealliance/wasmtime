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
