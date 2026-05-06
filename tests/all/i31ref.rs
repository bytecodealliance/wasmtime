use wasmtime::*;

#[test]
fn i31_new_u32() {
    // Values that fit in 31 bits succeed.
    assert_eq!(I31::new_u32(0).unwrap().get_u32(), 0);
    assert_eq!(I31::new_u32(5).unwrap().get_u32(), 5);
    assert_eq!(I31::new_u32(0x7fff_ffff).unwrap().get_u32(), 0x7fff_ffff);

    // Values that do not fit in 31 bits fail.
    assert!(I31::new_u32(0x8000_0000).is_none());
    assert!(I31::new_u32(0xffff_ffff).is_none());
}

#[test]
fn i31_new_i32() {
    // In-range signed values succeed.
    assert_eq!(I31::new_i32(0).unwrap().get_i32(), 0);
    assert_eq!(I31::new_i32(-1).unwrap().get_i32(), -1);
    assert_eq!(I31::new_i32(-5).unwrap().get_i32(), -5);
    // Minimum 31-bit signed value is -(2^30).
    assert_eq!(I31::new_i32(-0x4000_0000).unwrap().get_i32(), -0x4000_0000);

    // Out-of-range signed values fail.
    assert!(I31::new_i32(i32::MIN).is_none());
    assert!(I31::new_i32(-0x4000_0001).is_none());
}

#[test]
fn i31_wrapping_i32() {
    // In-range values are preserved.
    assert_eq!(I31::wrapping_i32(-5).get_i32(), -5);
    assert_eq!(I31::wrapping_i32(0).get_i32(), 0);

    // Out-of-range values are wrapped to fit in 31 bits.
    // -1073741825 (0xbfffffff) wrapped → 0x3fffffff = 1073741823
    assert_eq!(I31::wrapping_i32(-1073741825).get_i32(), 1073741823);
}

#[test]
fn i31_get_i32() {
    // max 31-bit unsigned (0x7fffffff) interpreted as signed = -1.
    assert_eq!(I31::new_u32(0x7fff_ffff).unwrap().get_i32(), -1);
    assert_eq!(I31::new_u32(0).unwrap().get_i32(), 0);
}

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

#[test]
#[cfg_attr(miri, ignore)]
fn owned_rooted_i31ref_through_typed_wasm_func() -> Result<()> {
    // OwnedRooted<AnyRef>::wasm_ty_store should handle i31ref values without
    // requiring a GC heap to be allocated.

    let mut config = Config::new();
    config.wasm_function_references(true);
    config.wasm_gc(true);

    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());

    let module = Module::new(
        &engine,
        r#"(module (func (export "f") (param (ref null any)) (result (ref null any)) local.get 0))"#,
    )?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let f = instance.get_typed_func::<Option<OwnedRooted<AnyRef>>, Option<OwnedRooted<AnyRef>>>(
        &mut store, "f",
    )?;

    // No GC heap objects created; the store has no GcStore allocated yet.
    let anyref = AnyRef::from_i31(&mut store, I31::wrapping_u32(42));
    let owned = anyref.to_owned_rooted(&mut store)?;
    let result = f.call(&mut store, Some(owned))?.unwrap();
    assert_eq!(
        result
            .to_rooted(&mut store)
            .as_i31(&store)?
            .unwrap()
            .get_u32(),
        42
    );

    Ok(())
}
