use wasmtime::*;

#[test]
fn wrong_import_numbers() -> Result<()> {
    let mut store = Store::<()>::default();
    let module = Module::new(store.engine(), r#"(module (import "" "" (func)))"#)?;

    assert!(Instance::new(&mut store, &module, &[]).is_err());
    let func = Func::wrap(&mut store, || {});
    assert!(Instance::new(&mut store, &module, &[func.into(), func.into()]).is_err());
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn initializes_linear_memory() -> Result<()> {
    // Test for https://github.com/bytecodealliance/wasmtime/issues/2784
    let wat = r#"
        (module
            (memory (export "memory") 2)
            (data (i32.const 0) "Hello World!")
        )"#;
    let module = Module::new(&Engine::default(), wat)?;

    let mut store = Store::new(module.engine(), ());
    let instance = Instance::new(&mut store, &module, &[])?;
    let memory = instance.get_memory(&mut store, "memory").unwrap();

    let mut bytes = [0; 12];
    memory.read(&store, 0, &mut bytes)?;
    assert_eq!(bytes, "Hello World!".as_bytes());
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn linear_memory_limits() -> Result<()> {
    // this test will allocate 4GB of virtual memory space, and may not work in
    // situations like CI QEMU emulation where it triggers SIGKILL.
    if std::env::var("WASMTIME_TEST_NO_HOG_MEMORY").is_ok() {
        return Ok(());
    }
    test(&Engine::default())?;
    let mut pool = crate::small_pool_config();
    pool.max_memory_size(1 << 32);
    test(&Engine::new(Config::new().allocation_strategy(
        InstanceAllocationStrategy::Pooling(pool),
    ))?)?;
    return Ok(());

    fn test(engine: &Engine) -> Result<()> {
        let wat = r#"
        (module
            (memory 65534)

            (func (export "grow")  (result i32)
                i32.const 1
                memory.grow)
            (func (export "size")  (result i32)
                memory.size)
        )
    "#;
        let module = Module::new(engine, wat)?;

        let mut store = Store::new(engine, ());
        let instance = Instance::new(&mut store, &module, &[])?;
        let size = instance.get_typed_func::<(), i32>(&mut store, "size")?;
        let grow = instance.get_typed_func::<(), i32>(&mut store, "grow")?;

        assert_eq!(size.call(&mut store, ())?, 65534);
        assert_eq!(grow.call(&mut store, ())?, 65534);
        assert_eq!(size.call(&mut store, ())?, 65535);
        assert_eq!(grow.call(&mut store, ())?, 65535);
        assert_eq!(size.call(&mut store, ())?, 65536);
        assert_eq!(grow.call(&mut store, ())?, -1);
        assert_eq!(size.call(&mut store, ())?, 65536);
        Ok(())
    }
}
