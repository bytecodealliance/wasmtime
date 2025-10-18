use wasmtime::*;

#[test]
fn eqref_from_i31() -> Result<()> {
    let mut config = Config::new();
    config.wasm_function_references(true);
    config.wasm_gc(true);

    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());

    let i31 = I31::wrapping_u32(31);

    // without EqRef::from_i31
    let any_ref = AnyRef::from_i31(&mut store, i31);
    let eq_ref1 = any_ref.unwrap_eqref(&mut store)?;

    // with EqRef::from_i31
    let eq_ref2 = EqRef::from_i31(&mut store, i31);

    // reference to same i31
    assert_eq!(Rooted::ref_eq(&store, &eq_ref1, &eq_ref2)?, true);

    Ok(())
}
