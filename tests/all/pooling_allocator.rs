use super::skip_pooling_allocator_tests;
use wasmtime::*;

#[test]
fn successful_instantiation() -> Result<()> {
    let pool = crate::small_pool_config();
    let mut config = Config::new();
    config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));
    config.dynamic_memory_guard_size(0);
    config.static_memory_guard_size(0);
    config.static_memory_maximum_size(1 << 16);

    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, r#"(module (memory 1) (table 10 funcref))"#)?;

    // Module should instantiate
    let mut store = Store::new(&engine, ());
    Instance::new(&mut store, &module, &[])?;

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn memory_limit() -> Result<()> {
    let mut pool = crate::small_pool_config();
    pool.max_memory_size(3 << 16);
    let mut config = Config::new();
    config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));
    config.dynamic_memory_guard_size(0);
    config.static_memory_guard_size(1 << 16);
    config.static_memory_maximum_size(3 << 16);
    config.wasm_multi_memory(true);

    let engine = Engine::new(&config)?;

    // Module should fail to instantiate because it has too many memories
    match Module::new(&engine, r#"(module (memory 1) (memory 1))"#) {
        Ok(_) => panic!("module instantiation should fail"),
        Err(e) => assert_eq!(
            e.to_string(),
            "defined memories count of 2 exceeds the per-instance limit of 1",
        ),
    }

    // Module should fail to instantiate because the minimum is greater than
    // the configured limit
    match Module::new(&engine, r#"(module (memory 4))"#) {
        Ok(_) => panic!("module instantiation should fail"),
        Err(e) => assert_eq!(
            e.to_string(),
            "memory index 0 has a minimum byte size of 262144 which exceeds the limit of 196608 bytes",
        ),
    }

    let module = Module::new(
        &engine,
        r#"(module (memory (export "m") 0) (func (export "f") (result i32) (memory.grow (i32.const 1))))"#,
    )?;

    // Instantiate the module and grow the memory via the `f` function
    {
        let mut store = Store::new(&engine, ());
        let instance = Instance::new(&mut store, &module, &[])?;
        let f = instance.get_typed_func::<(), i32>(&mut store, "f")?;

        assert_eq!(f.call(&mut store, ()).expect("function should not trap"), 0);
        assert_eq!(f.call(&mut store, ()).expect("function should not trap"), 1);
        assert_eq!(f.call(&mut store, ()).expect("function should not trap"), 2);
        assert_eq!(
            f.call(&mut store, ()).expect("function should not trap"),
            -1
        );
        assert_eq!(
            f.call(&mut store, ()).expect("function should not trap"),
            -1
        );
    }

    // Instantiate the module and grow the memory via the Wasmtime API
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;

    let memory = instance.get_memory(&mut store, "m").unwrap();
    assert_eq!(memory.size(&store), 0);
    assert_eq!(memory.grow(&mut store, 1).expect("memory should grow"), 0);
    assert_eq!(memory.size(&store), 1);
    assert_eq!(memory.grow(&mut store, 1).expect("memory should grow"), 1);
    assert_eq!(memory.size(&store), 2);
    assert_eq!(memory.grow(&mut store, 1).expect("memory should grow"), 2);
    assert_eq!(memory.size(&store), 3);
    assert!(memory.grow(&mut store, 1).is_err());

    Ok(())
}

#[test]
fn memory_init() -> Result<()> {
    let mut pool = crate::small_pool_config();
    pool.max_memory_size(2 << 16).table_elements(0);
    let mut config = Config::new();
    config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));

    let engine = Engine::new(&config)?;

    let module = Module::new(
        &engine,
        r#"
            (module
                (memory (export "m") 2)
                (data (i32.const 65530) "this data spans multiple pages")
                (data (i32.const 10) "hello world")
            )
        "#,
    )?;

    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;
    let memory = instance.get_memory(&mut store, "m").unwrap();

    assert_eq!(
        &memory.data(&store)[65530..65560],
        b"this data spans multiple pages"
    );
    assert_eq!(&memory.data(&store)[10..21], b"hello world");

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn memory_guard_page_trap() -> Result<()> {
    let mut pool = crate::small_pool_config();
    pool.max_memory_size(2 << 16).table_elements(0);
    let mut config = Config::new();
    config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));

    let engine = Engine::new(&config)?;

    let module = Module::new(
        &engine,
        r#"
            (module
                (memory (export "m") 0)
                (func (export "f") (param i32) local.get 0 i32.load drop)
            )
        "#,
    )?;

    // Instantiate the module and check for out of bounds trap
    for _ in 0..10 {
        let mut store = Store::new(&engine, ());
        let instance = Instance::new(&mut store, &module, &[])?;
        let m = instance.get_memory(&mut store, "m").unwrap();
        let f = instance.get_typed_func::<i32, ()>(&mut store, "f")?;

        let trap = f
            .call(&mut store, 0)
            .expect_err("function should trap")
            .downcast::<Trap>()?;
        assert_eq!(trap, Trap::MemoryOutOfBounds);

        let trap = f
            .call(&mut store, 1)
            .expect_err("function should trap")
            .downcast::<Trap>()?;
        assert_eq!(trap, Trap::MemoryOutOfBounds);

        m.grow(&mut store, 1).expect("memory should grow");
        f.call(&mut store, 0).expect("function should not trap");

        let trap = f
            .call(&mut store, 65536)
            .expect_err("function should trap")
            .downcast::<Trap>()?;
        assert_eq!(trap, Trap::MemoryOutOfBounds);

        let trap = f
            .call(&mut store, 65537)
            .expect_err("function should trap")
            .downcast::<Trap>()?;
        assert_eq!(trap, Trap::MemoryOutOfBounds);

        m.grow(&mut store, 1).expect("memory should grow");
        f.call(&mut store, 65536).expect("function should not trap");

        m.grow(&mut store, 1)
            .expect_err("memory should be at the limit");
    }

    Ok(())
}

#[test]
fn memory_zeroed() -> Result<()> {
    if skip_pooling_allocator_tests() {
        return Ok(());
    }

    let mut pool = crate::small_pool_config();
    pool.max_memory_size(1 << 16).table_elements(0);
    let mut config = Config::new();
    config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));
    config.dynamic_memory_guard_size(0);
    config.static_memory_guard_size(0);
    config.static_memory_maximum_size(1 << 16);

    let engine = Engine::new(&config)?;

    let module = Module::new(&engine, r#"(module (memory (export "m") 1))"#)?;

    // Instantiate the module repeatedly after writing data to the entire memory
    for _ in 0..10 {
        let mut store = Store::new(&engine, ());
        let instance = Instance::new(&mut store, &module, &[])?;
        let memory = instance.get_memory(&mut store, "m").unwrap();

        assert_eq!(memory.size(&store,), 1);
        assert_eq!(memory.data_size(&store), 65536);

        let ptr = memory.data_mut(&mut store).as_mut_ptr();

        unsafe {
            for i in 0..8192 {
                assert_eq!(*ptr.cast::<u64>().offset(i), 0);
            }
            std::ptr::write_bytes(ptr, 0xFE, memory.data_size(&store));
        }
    }

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn table_limit() -> Result<()> {
    const TABLE_ELEMENTS: usize = 10;
    let mut pool = crate::small_pool_config();
    pool.table_elements(TABLE_ELEMENTS);
    let mut config = Config::new();
    config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));
    config.dynamic_memory_guard_size(0);
    config.static_memory_guard_size(0);
    config.static_memory_maximum_size(1 << 16);

    let engine = Engine::new(&config)?;

    // Module should fail to instantiate because it has too many tables
    match Module::new(&engine, r#"(module (table 1 funcref) (table 1 funcref))"#) {
        Ok(_) => panic!("module compilation should fail"),
        Err(e) => assert_eq!(
            e.to_string(),
            "defined tables count of 2 exceeds the per-instance limit of 1",
        ),
    }

    // Module should fail to instantiate because the minimum is greater than
    // the configured limit
    match Module::new(&engine, r#"(module (table 31 funcref))"#) {
        Ok(_) => panic!("module compilation should fail"),
        Err(e) => assert_eq!(
            e.to_string(),
            "table index 0 has a minimum element size of 31 which exceeds the limit of 10",
        ),
    }

    let module = Module::new(
        &engine,
        r#"(module (table (export "t") 0 funcref) (func (export "f") (result i32) (table.grow (ref.null func) (i32.const 1))))"#,
    )?;

    // Instantiate the module and grow the table via the `f` function
    {
        let mut store = Store::new(&engine, ());
        let instance = Instance::new(&mut store, &module, &[])?;
        let f = instance.get_typed_func::<(), i32>(&mut store, "f")?;

        for i in 0..TABLE_ELEMENTS {
            assert_eq!(
                f.call(&mut store, ()).expect("function should not trap"),
                i as i32
            );
        }

        assert_eq!(
            f.call(&mut store, ()).expect("function should not trap"),
            -1
        );
        assert_eq!(
            f.call(&mut store, ()).expect("function should not trap"),
            -1
        );
    }

    // Instantiate the module and grow the table via the Wasmtime API
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;

    let table = instance.get_table(&mut store, "t").unwrap();

    for i in 0..TABLE_ELEMENTS {
        assert_eq!(table.size(&store), i as u64);
        assert_eq!(
            table
                .grow(&mut store, 1, Ref::Func(None))
                .expect("table should grow"),
            i as u64
        );
    }

    assert_eq!(table.size(&store), TABLE_ELEMENTS as u64);
    assert!(table.grow(&mut store, 1, Ref::Func(None)).is_err());

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn table_init() -> Result<()> {
    let mut pool = crate::small_pool_config();
    pool.max_memory_size(0).table_elements(6);
    let mut config = Config::new();
    config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));

    let engine = Engine::new(&config)?;

    let module = Module::new(
        &engine,
        r#"
            (module
                (table (export "t") 6 funcref)
                (elem (i32.const 1) 1 2 3 4)
                (elem (i32.const 0) 0)
                (func)
                (func (param i32))
                (func (param i32 i32))
                (func (param i32 i32 i32))
                (func (param i32 i32 i32 i32))
            )
        "#,
    )?;

    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;
    let table = instance.get_table(&mut store, "t").unwrap();

    for i in 0..5 {
        let v = table.get(&mut store, i).expect("table should have entry");
        let f = v
            .as_func()
            .expect("expected funcref")
            .expect("expected non-null value");
        assert_eq!(f.ty(&store).params().len(), i as usize);
    }

    assert!(
        table
            .get(&mut store, 5)
            .expect("table should have entry")
            .as_func()
            .expect("expected funcref")
            .is_none(),
        "funcref should be null"
    );

    Ok(())
}

#[test]
fn table_zeroed() -> Result<()> {
    if skip_pooling_allocator_tests() {
        return Ok(());
    }

    let pool = crate::small_pool_config();
    let mut config = Config::new();
    config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));
    config.dynamic_memory_guard_size(0);
    config.static_memory_guard_size(0);
    config.static_memory_maximum_size(1 << 16);

    let engine = Engine::new(&config)?;

    let module = Module::new(&engine, r#"(module (table (export "t") 10 funcref))"#)?;

    // Instantiate the module repeatedly after filling table elements
    for _ in 0..10 {
        let mut store = Store::new(&engine, ());
        let instance = Instance::new(&mut store, &module, &[])?;
        let table = instance.get_table(&mut store, "t").unwrap();
        let f = Func::wrap(&mut store, || {});

        assert_eq!(table.size(&store), 10);

        for i in 0..10 {
            match table.get(&mut store, i).unwrap() {
                Ref::Func(r) => assert!(r.is_none()),
                _ => panic!("expected a funcref"),
            }
            table.set(&mut store, i, Ref::Func(Some(f))).unwrap();
        }
    }

    Ok(())
}

#[test]
fn total_core_instances_limit() -> Result<()> {
    const INSTANCE_LIMIT: u32 = 10;
    let mut pool = crate::small_pool_config();
    pool.total_core_instances(INSTANCE_LIMIT);
    let mut config = Config::new();
    config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));
    config.dynamic_memory_guard_size(0);
    config.static_memory_guard_size(0);
    config.static_memory_maximum_size(1 << 16);

    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, r#"(module)"#)?;

    // Instantiate to the limit
    {
        let mut store = Store::new(&engine, ());

        for _ in 0..INSTANCE_LIMIT {
            Instance::new(&mut store, &module, &[])?;
        }

        match Instance::new(&mut store, &module, &[]) {
            Ok(_) => panic!("instantiation should fail"),
            Err(e) => assert!(e.is::<PoolConcurrencyLimitError>()),
        }
    }

    // With the above store dropped, ensure instantiations can be made

    let mut store = Store::new(&engine, ());

    for _ in 0..INSTANCE_LIMIT {
        Instance::new(&mut store, &module, &[])?;
    }

    Ok(())
}

#[test]
fn preserve_data_segments() -> Result<()> {
    let mut pool = crate::small_pool_config();
    pool.total_memories(2);
    let mut config = Config::new();
    config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));
    let engine = Engine::new(&config)?;
    let m = Module::new(
        &engine,
        r#"
            (module
                (memory (export "mem") 1 1)
                (data (i32.const 0) "foo"))
        "#,
    )?;
    let mut store = Store::new(&engine, ());
    let i = Instance::new(&mut store, &m, &[])?;

    // Drop the module. This should *not* drop the actual data referenced by the
    // module.
    drop(m);

    // Spray some stuff on the heap. If wasm data lived on the heap this should
    // paper over things and help us catch use-after-free here if it would
    // otherwise happen.
    if !cfg!(miri) {
        let mut strings = Vec::new();
        for _ in 0..1000 {
            let mut string = String::new();
            for _ in 0..1000 {
                string.push('g');
            }
            strings.push(string);
        }
        drop(strings);
    }

    let mem = i.get_memory(&mut store, "mem").unwrap();

    // Hopefully it's still `foo`!
    assert!(mem.data(&store).starts_with(b"foo"));

    Ok(())
}

#[test]
fn multi_memory_with_imported_memories() -> Result<()> {
    // This test checks that the base address for the defined memory is correct for the instance
    // despite the presence of an imported memory.

    let mut pool = crate::small_pool_config();
    pool.total_memories(2).max_memories_per_module(2);
    let mut config = Config::new();
    config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));
    config.wasm_multi_memory(true);

    let engine = Engine::new(&config)?;
    let module = Module::new(
        &engine,
        r#"(module (import "" "m1" (memory 0)) (memory (export "m2") 1))"#,
    )?;

    let mut store = Store::new(&engine, ());

    let m1 = Memory::new(&mut store, MemoryType::new(0, None))?;
    let instance = Instance::new(&mut store, &module, &[m1.into()])?;

    let m2 = instance.get_memory(&mut store, "m2").unwrap();

    m2.data_mut(&mut store)[0] = 0x42;
    assert_eq!(m2.data(&store)[0], 0x42);

    Ok(())
}

#[test]
fn drop_externref_global_during_module_init() -> Result<()> {
    struct Limiter;

    impl ResourceLimiter for Limiter {
        fn memory_growing(&mut self, _: usize, _: usize, _: Option<usize>) -> Result<bool> {
            Ok(false)
        }

        fn table_growing(&mut self, _: usize, _: usize, _: Option<usize>) -> Result<bool> {
            Ok(false)
        }
    }

    let pool = crate::small_pool_config();
    let mut config = Config::new();
    config.wasm_reference_types(true);
    config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));

    let engine = Engine::new(&config)?;

    let module = Module::new(
        &engine,
        r#"
            (module
                (global i32 (i32.const 1))
                (global i32 (i32.const 2))
                (global i32 (i32.const 3))
                (global i32 (i32.const 4))
                (global i32 (i32.const 5))
            )
        "#,
    )?;

    let mut store = Store::new(&engine, Limiter);
    Instance::new(&mut store, &module, &[])?;
    drop(store);

    let module = Module::new(
        &engine,
        r#"
            (module
                (memory 1)
                (global (mut externref) (ref.null extern))
            )
        "#,
    )?;

    let mut store = Store::new(&engine, Limiter);
    store.limiter(|s| s);
    assert!(Instance::new(&mut store, &module, &[]).is_err());

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn switch_image_and_non_image() -> Result<()> {
    let pool = crate::small_pool_config();
    let mut c = Config::new();
    c.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));
    let engine = Engine::new(&c)?;
    let module1 = Module::new(
        &engine,
        r#"
            (module
                (memory 1)
                (func (export "load") (param i32) (result i32)
                    local.get 0
                    i32.load
                )
            )
        "#,
    )?;
    let module2 = Module::new(
        &engine,
        r#"
            (module
                (memory (export "memory") 1)
                (data (i32.const 0) "1234")
            )
        "#,
    )?;

    let assert_zero = || -> Result<()> {
        let mut store = Store::new(&engine, ());
        let instance = Instance::new(&mut store, &module1, &[])?;
        let func = instance.get_typed_func::<i32, i32>(&mut store, "load")?;
        assert_eq!(func.call(&mut store, 0)?, 0);
        Ok(())
    };

    // Initialize with a heap image and make sure the next instance, without an
    // image, is zeroed
    Instance::new(&mut Store::new(&engine, ()), &module2, &[])?;
    assert_zero()?;

    // ... transition back to heap image and do this again
    Instance::new(&mut Store::new(&engine, ()), &module2, &[])?;
    assert_zero()?;

    // And go back to an image and make sure it's read/write on the host.
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module2, &[])?;
    let memory = instance.get_memory(&mut store, "memory").unwrap();
    let mem = memory.data_mut(&mut store);
    assert!(mem.starts_with(b"1234"));
    mem[..6].copy_from_slice(b"567890");

    Ok(())
}

#[test]
#[cfg(target_pointer_width = "64")]
#[cfg_attr(miri, ignore)]
fn instance_too_large() -> Result<()> {
    let mut pool = crate::small_pool_config();
    pool.max_core_instance_size(16);
    let mut config = Config::new();
    config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));

    let engine = Engine::new(&config)?;
    let expected = if cfg!(feature = "wmemcheck") {
        "\
        instance allocation for this module requires 336 bytes which exceeds the \
configured maximum of 16 bytes; breakdown of allocation requirement:

 * 71.43% - 240 bytes - instance state management
 * 26.19% - 88 bytes - static vmctx data
"
    } else {
        "\
        instance allocation for this module requires 240 bytes which exceeds the \
configured maximum of 16 bytes; breakdown of allocation requirement:

 * 60.00% - 144 bytes - instance state management
 * 36.67% - 88 bytes - static vmctx data
"
    };
    match Module::new(&engine, "(module)") {
        Ok(_) => panic!("should have failed to compile"),
        Err(e) => assert_eq!(e.to_string(), expected),
    }

    let mut lots_of_globals = format!("(module");
    for _ in 0..100 {
        lots_of_globals.push_str("(global i32 i32.const 0)\n");
    }
    lots_of_globals.push_str(")");

    let expected = if cfg!(feature = "wmemcheck") {
        "\
instance allocation for this module requires 1936 bytes which exceeds the \
configured maximum of 16 bytes; breakdown of allocation requirement:

 * 12.40% - 240 bytes - instance state management
 * 82.64% - 1600 bytes - defined globals
"
    } else {
        "\
instance allocation for this module requires 1840 bytes which exceeds the \
configured maximum of 16 bytes; breakdown of allocation requirement:

 * 7.83% - 144 bytes - instance state management
 * 86.96% - 1600 bytes - defined globals
"
    };
    match Module::new(&engine, &lots_of_globals) {
        Ok(_) => panic!("should have failed to compile"),
        Err(e) => assert_eq!(e.to_string(), expected),
    }

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn dynamic_memory_pooling_allocator() -> Result<()> {
    for guard_size in [0, 1 << 16] {
        let max_size = 128 << 20;
        let mut pool = crate::small_pool_config();
        pool.max_memory_size(max_size as usize);
        let mut config = Config::new();
        config.static_memory_maximum_size(max_size);
        config.dynamic_memory_guard_size(guard_size);
        config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));

        let engine = Engine::new(&config)?;

        let module = Module::new(
            &engine,
            r#"
            (module
                (memory (export "memory") 1)

                (func (export "grow") (param i32) (result i32)
                    local.get 0
                    memory.grow)

                (func (export "size") (result i32)
                    memory.size)

                (func (export "i32.load") (param i32) (result i32)
                    local.get 0
                    i32.load)

                (func (export "i32.store") (param i32 i32)
                    local.get 0
                    local.get 1
                    i32.store)

                (data (i32.const 100) "x")
            )
         "#,
        )?;

        let mut store = Store::new(&engine, ());
        let instance = Instance::new(&mut store, &module, &[])?;

        let grow = instance.get_typed_func::<u32, i32>(&mut store, "grow")?;
        let size = instance.get_typed_func::<(), u32>(&mut store, "size")?;
        let i32_load = instance.get_typed_func::<u32, i32>(&mut store, "i32.load")?;
        let i32_store = instance.get_typed_func::<(u32, i32), ()>(&mut store, "i32.store")?;
        let memory = instance.get_memory(&mut store, "memory").unwrap();

        // basic length 1 tests
        // assert_eq!(memory.grow(&mut store, 1)?, 0);
        assert_eq!(memory.size(&store), 1);
        assert_eq!(size.call(&mut store, ())?, 1);
        assert_eq!(i32_load.call(&mut store, 0)?, 0);
        assert_eq!(i32_load.call(&mut store, 100)?, i32::from(b'x'));
        i32_store.call(&mut store, (0, 0))?;
        i32_store.call(&mut store, (100, i32::from(b'y')))?;
        assert_eq!(i32_load.call(&mut store, 100)?, i32::from(b'y'));

        // basic length 2 tests
        let page = 64 * 1024;
        assert_eq!(grow.call(&mut store, 1)?, 1);
        assert_eq!(memory.size(&store), 2);
        assert_eq!(size.call(&mut store, ())?, 2);
        i32_store.call(&mut store, (page, 200))?;
        assert_eq!(i32_load.call(&mut store, page)?, 200);

        // test writes are visible
        i32_store.call(&mut store, (2, 100))?;
        assert_eq!(i32_load.call(&mut store, 2)?, 100);

        // test growth can't exceed maximum
        let too_many = max_size / (64 * 1024);
        assert_eq!(grow.call(&mut store, too_many as u32)?, -1);
        assert!(memory.grow(&mut store, too_many).is_err());

        assert_eq!(memory.data(&store)[page as usize], 200);

        // Re-instantiate in another store.
        store = Store::new(&engine, ());
        let instance = Instance::new(&mut store, &module, &[])?;
        let i32_load = instance.get_typed_func::<u32, i32>(&mut store, "i32.load")?;
        let memory = instance.get_memory(&mut store, "memory").unwrap();

        // This is out of bounds...
        assert!(i32_load.call(&mut store, page).is_err());
        assert_eq!(memory.data_size(&store), page as usize);

        // ... but implementation-wise it should still be mapped memory from
        // before if we don't have any guard pages.
        //
        // Note though that prior writes should all appear as zeros and we can't see
        // data from the prior instance.
        //
        // Note that this part is only implemented on Linux which has
        // `MADV_DONTNEED`.
        if cfg!(target_os = "linux") && guard_size == 0 {
            unsafe {
                let ptr = memory.data_ptr(&store);
                assert_eq!(*ptr.offset(page as isize), 0);
            }
        }
    }

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn zero_memory_pages_disallows_oob() -> Result<()> {
    let mut pool = crate::small_pool_config();
    pool.max_memory_size(0);
    let mut config = Config::new();
    config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));

    let engine = Engine::new(&config)?;
    let module = Module::new(
        &engine,
        r#"
            (module
                (memory 0)

                (func (export "load") (param i32) (result i32)
                    local.get 0
                    i32.load)

                (func (export "store") (param i32 )
                    local.get 0
                    local.get 0
                    i32.store)
            )
        "#,
    )?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;
    let load32 = instance.get_typed_func::<i32, i32>(&mut store, "load")?;
    let store32 = instance.get_typed_func::<i32, ()>(&mut store, "store")?;
    for i in 0..31 {
        assert!(load32.call(&mut store, 1 << i).is_err());
        assert!(store32.call(&mut store, 1 << i).is_err());
    }
    Ok(())
}

#[test]
#[cfg(feature = "component-model")]
fn total_component_instances_limit() -> Result<()> {
    const TOTAL_COMPONENT_INSTANCES: u32 = 5;

    let mut pool = crate::small_pool_config();
    pool.total_component_instances(TOTAL_COMPONENT_INSTANCES);
    let mut config = Config::new();
    config.wasm_component_model(true);
    config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));

    let engine = Engine::new(&config)?;
    let linker = wasmtime::component::Linker::new(&engine);
    let component = wasmtime::component::Component::new(&engine, "(component)")?;

    let mut store = Store::new(&engine, ());
    for _ in 0..TOTAL_COMPONENT_INSTANCES {
        linker.instantiate(&mut store, &component)?;
    }

    match linker.instantiate(&mut store, &component) {
        Ok(_) => panic!("should have hit component instance limit"),
        Err(e) => assert!(e.is::<PoolConcurrencyLimitError>()),
    }

    drop(store);
    let mut store = Store::new(&engine, ());
    for _ in 0..TOTAL_COMPONENT_INSTANCES {
        linker.instantiate(&mut store, &component)?;
    }

    Ok(())
}

#[test]
#[cfg(feature = "component-model")]
fn component_instance_size_limit() -> Result<()> {
    let mut pool = crate::small_pool_config();
    pool.max_component_instance_size(1);
    let mut config = Config::new();
    config.wasm_component_model(true);
    config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));
    let engine = Engine::new(&config)?;

    match wasmtime::component::Component::new(&engine, "(component)") {
        Ok(_) => panic!("should have hit limit"),
        Err(e) => assert_eq!(
            e.to_string(),
            "instance allocation for this component requires 64 bytes of `VMComponentContext` space \
             which exceeds the configured maximum of 1 bytes"
        ),
    }

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn total_tables_limit() -> Result<()> {
    const TOTAL_TABLES: u32 = 5;

    let mut pool = crate::small_pool_config();
    pool.total_tables(TOTAL_TABLES)
        .total_core_instances(TOTAL_TABLES + 1);
    let mut config = Config::new();
    config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));

    let engine = Engine::new(&config)?;
    let linker = Linker::new(&engine);
    let module = Module::new(&engine, "(module (table 0 1 funcref))")?;

    let mut store = Store::new(&engine, ());
    for _ in 0..TOTAL_TABLES {
        linker.instantiate(&mut store, &module)?;
    }

    match linker.instantiate(&mut store, &module) {
        Ok(_) => panic!("should have hit table limit"),
        Err(e) => assert!(e.is::<PoolConcurrencyLimitError>()),
    }

    drop(store);
    let mut store = Store::new(&engine, ());
    for _ in 0..TOTAL_TABLES {
        linker.instantiate(&mut store, &module)?;
    }

    Ok(())
}

#[tokio::test]
#[cfg(not(miri))]
async fn total_stacks_limit() -> Result<()> {
    use super::async_functions::PollOnce;

    const TOTAL_STACKS: u32 = 2;

    let mut pool = crate::small_pool_config();
    pool.total_stacks(TOTAL_STACKS)
        .total_core_instances(TOTAL_STACKS + 1);
    let mut config = Config::new();
    config.async_support(true);
    config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));

    let engine = Engine::new(&config)?;

    let mut linker = Linker::new(&engine);
    linker.func_new_async(
        "async",
        "yield",
        FuncType::new(&engine, [], []),
        |_caller, _params, _results| {
            Box::new(async {
                tokio::task::yield_now().await;
                Ok(())
            })
        },
    )?;

    let module = Module::new(
        &engine,
        r#"
        (module
            (import "async" "yield" (func $yield))
            (func (export "run")
                call $yield
            )
        )
    "#,
    )?;

    // Allocate stacks up to the limit. (Poll the futures once to make sure we
    // actually enter Wasm and force a stack allocation.)

    let mut store1 = Store::new(&engine, ());
    let instance1 = linker.instantiate_async(&mut store1, &module).await?;
    let run1 = instance1.get_func(&mut store1, "run").unwrap();
    let future1 = PollOnce::new(Box::pin(run1.call_async(store1, &[], &mut [])))
        .await
        .unwrap_err();

    let mut store2 = Store::new(&engine, ());
    let instance2 = linker.instantiate_async(&mut store2, &module).await?;
    let run2 = instance2.get_func(&mut store2, "run").unwrap();
    let future2 = PollOnce::new(Box::pin(run2.call_async(store2, &[], &mut [])))
        .await
        .unwrap_err();

    // Allocating more should fail.
    let mut store3 = Store::new(&engine, ());
    match linker.instantiate_async(&mut store3, &module).await {
        Ok(_) => panic!("should have hit stack limit"),
        Err(e) => assert!(e.is::<PoolConcurrencyLimitError>()),
    }

    // Finish the futures and return their Wasm stacks to the pool.
    future1.await?;
    future2.await?;

    // Should be able to allocate new stacks again.
    let mut store1 = Store::new(&engine, ());
    let instance1 = linker.instantiate_async(&mut store1, &module).await?;
    let run1 = instance1.get_func(&mut store1, "run").unwrap();
    let future1 = run1.call_async(store1, &[], &mut []);

    let mut store2 = Store::new(&engine, ());
    let instance2 = linker.instantiate_async(&mut store2, &module).await?;
    let run2 = instance2.get_func(&mut store2, "run").unwrap();
    let future2 = run2.call_async(store2, &[], &mut []);

    future1.await?;
    future2.await?;

    Ok(())
}

#[test]
#[cfg(feature = "component-model")]
fn component_core_instances_limit() -> Result<()> {
    let mut pool = crate::small_pool_config();
    pool.max_core_instances_per_component(1);
    let mut config = Config::new();
    config.wasm_component_model(true);
    config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));
    let engine = Engine::new(&config)?;

    // One core instance works.
    wasmtime::component::Component::new(
        &engine,
        r#"
            (component
                (core module $m)
                (core instance $a (instantiate $m))
            )
        "#,
    )?;

    // Two core instances doesn't.
    match wasmtime::component::Component::new(
        &engine,
        r#"
            (component
                (core module $m)
                (core instance $a (instantiate $m))
                (core instance $b (instantiate $m))
            )
        "#,
    ) {
        Ok(_) => panic!("should have hit limit"),
        Err(e) => assert_eq!(
            e.to_string(),
            "The component transitively contains 2 core module instances, which exceeds the \
             configured maximum of 1"
        ),
    }

    Ok(())
}

#[test]
#[cfg(feature = "component-model")]
fn component_memories_limit() -> Result<()> {
    let mut pool = crate::small_pool_config();
    pool.max_memories_per_component(1).total_memories(2);
    let mut config = Config::new();
    config.wasm_component_model(true);
    config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));
    let engine = Engine::new(&config)?;

    // One memory works.
    wasmtime::component::Component::new(
        &engine,
        r#"
            (component
                (core module $m (memory 1 1))
                (core instance $a (instantiate $m))
            )
        "#,
    )?;

    // Two memories doesn't.
    match wasmtime::component::Component::new(
        &engine,
        r#"
            (component
                (core module $m (memory 1 1))
                (core instance $a (instantiate $m))
                (core instance $b (instantiate $m))
            )
        "#,
    ) {
        Ok(_) => panic!("should have hit limit"),
        Err(e) => assert_eq!(
            e.to_string(),
            "The component transitively contains 2 Wasm linear memories, which exceeds the \
             configured maximum of 1"
        ),
    }

    Ok(())
}

#[test]
#[cfg(feature = "component-model")]
fn component_tables_limit() -> Result<()> {
    let mut pool = crate::small_pool_config();
    pool.max_tables_per_component(1).total_tables(2);
    let mut config = Config::new();
    config.wasm_component_model(true);
    config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));
    let engine = Engine::new(&config)?;

    // One table works.
    wasmtime::component::Component::new(
        &engine,
        r#"
            (component
                (core module $m (table 1 1 funcref))
                (core instance $a (instantiate $m))
            )
        "#,
    )?;

    // Two tables doesn't.
    match wasmtime::component::Component::new(
        &engine,
        r#"
            (component
                (core module $m (table 1 1 funcref))
                (core instance $a (instantiate $m))
                (core instance $b (instantiate $m))
            )
        "#,
    ) {
        Ok(_) => panic!("should have hit limit"),
        Err(e) => assert_eq!(
            e.to_string(),
            "The component transitively contains 2 tables, which exceeds the \
             configured maximum of 1"
        ),
    }

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn total_memories_limit() -> Result<()> {
    const TOTAL_MEMORIES: u32 = 5;

    let mut pool = crate::small_pool_config();
    pool.total_memories(TOTAL_MEMORIES)
        .total_core_instances(TOTAL_MEMORIES + 1)
        .memory_protection_keys(MpkEnabled::Disable);
    let mut config = Config::new();
    config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));

    let engine = Engine::new(&config)?;
    let linker = Linker::new(&engine);
    let module = Module::new(&engine, "(module (memory 1 1))")?;

    let mut store = Store::new(&engine, ());
    for _ in 0..TOTAL_MEMORIES {
        linker.instantiate(&mut store, &module)?;
    }

    match linker.instantiate(&mut store, &module) {
        Ok(_) => panic!("should have hit memory limit"),
        Err(e) => assert!(e.is::<PoolConcurrencyLimitError>()),
    }

    drop(store);
    let mut store = Store::new(&engine, ());
    for _ in 0..TOTAL_MEMORIES {
        linker.instantiate(&mut store, &module)?;
    }

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn decommit_batching() -> Result<()> {
    for (capacity, batch_size) in [
        // A reasonable batch size.
        (10, 5),
        // Batch sizes of zero and one should effectively disable batching.
        (10, 1),
        (10, 0),
        // A bigger batch size than capacity, which forces the allocation path
        // to flush the decommit queue.
        (10, 99),
    ] {
        let mut pool = crate::small_pool_config();
        pool.total_memories(capacity)
            .total_core_instances(capacity)
            .decommit_batch_size(batch_size)
            .memory_protection_keys(MpkEnabled::Disable);
        let mut config = Config::new();
        config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));

        let engine = Engine::new(&config)?;
        let linker = Linker::new(&engine);
        let module = Module::new(&engine, "(module (memory 1 1))")?;

        // Just make sure that we can instantiate all slots a few times and the
        // pooling allocator must be flushing the decommit queue as necessary.
        for _ in 0..3 {
            let mut store = Store::new(&engine, ());
            for _ in 0..capacity {
                linker.instantiate(&mut store, &module)?;
            }
        }
    }

    Ok(())
}
