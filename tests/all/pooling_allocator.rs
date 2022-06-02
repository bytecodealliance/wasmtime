use super::skip_pooling_allocator_tests;
use anyhow::Result;
use wasmtime::*;

#[test]
fn successful_instantiation() -> Result<()> {
    let mut config = Config::new();
    config.allocation_strategy(InstanceAllocationStrategy::Pooling {
        strategy: PoolingAllocationStrategy::NextAvailable,
        instance_limits: InstanceLimits {
            count: 1,
            memory_pages: 1,
            table_elements: 10,
            ..Default::default()
        },
    });
    config.dynamic_memory_guard_size(0);
    config.static_memory_guard_size(0);
    config.static_memory_maximum_size(65536);

    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, r#"(module (memory 1) (table 10 funcref))"#)?;

    // Module should instantiate
    let mut store = Store::new(&engine, ());
    Instance::new(&mut store, &module, &[])?;

    Ok(())
}

#[test]
fn memory_limit() -> Result<()> {
    let mut config = Config::new();
    config.allocation_strategy(InstanceAllocationStrategy::Pooling {
        strategy: PoolingAllocationStrategy::NextAvailable,
        instance_limits: InstanceLimits {
            count: 1,
            memory_pages: 3,
            table_elements: 10,
            ..Default::default()
        },
    });
    config.dynamic_memory_guard_size(0);
    config.static_memory_guard_size(65536);
    config.static_memory_maximum_size(3 * 65536);
    config.wasm_multi_memory(true);

    let engine = Engine::new(&config)?;

    // Module should fail to instantiate because it has too many memories
    match Module::new(&engine, r#"(module (memory 1) (memory 1))"#) {
        Ok(_) => panic!("module instantiation should fail"),
        Err(e) => assert_eq!(
            e.to_string(),
            "defined memories count of 2 exceeds the limit of 1",
        ),
    }

    // Module should fail to instantiate because the minimum is greater than
    // the configured limit
    match Module::new(&engine, r#"(module (memory 4))"#) {
        Ok(_) => panic!("module instantiation should fail"),
        Err(e) => assert_eq!(
            e.to_string(),
            "memory index 0 has a minimum page size of 4 which exceeds the limit of 3",
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
        let f = instance.get_typed_func::<(), i32, _>(&mut store, "f")?;

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
    let mut config = Config::new();
    config.allocation_strategy(InstanceAllocationStrategy::Pooling {
        strategy: PoolingAllocationStrategy::NextAvailable,
        instance_limits: InstanceLimits {
            count: 1,
            memory_pages: 2,
            table_elements: 0,
            ..Default::default()
        },
    });

    let engine = Engine::new(&config)?;

    let module = Module::new(
        &engine,
        r#"(module (memory (export "m") 2) (data (i32.const 65530) "this data spans multiple pages") (data (i32.const 10) "hello world"))"#,
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
fn memory_guard_page_trap() -> Result<()> {
    let mut config = Config::new();
    config.allocation_strategy(InstanceAllocationStrategy::Pooling {
        strategy: PoolingAllocationStrategy::NextAvailable,
        instance_limits: InstanceLimits {
            count: 1,
            memory_pages: 2,
            table_elements: 0,
            ..Default::default()
        },
    });

    let engine = Engine::new(&config)?;

    let module = Module::new(
        &engine,
        r#"(module (memory (export "m") 0) (func (export "f") (param i32) local.get 0 i32.load drop))"#,
    )?;

    // Instantiate the module and check for out of bounds trap
    for _ in 0..10 {
        let mut store = Store::new(&engine, ());
        let instance = Instance::new(&mut store, &module, &[])?;
        let m = instance.get_memory(&mut store, "m").unwrap();
        let f = instance.get_typed_func::<i32, (), _>(&mut store, "f")?;

        let trap = f.call(&mut store, 0).expect_err("function should trap");
        assert!(trap.to_string().contains("out of bounds"));

        let trap = f.call(&mut store, 1).expect_err("function should trap");
        assert!(trap.to_string().contains("out of bounds"));

        m.grow(&mut store, 1).expect("memory should grow");
        f.call(&mut store, 0).expect("function should not trap");

        let trap = f.call(&mut store, 65536).expect_err("function should trap");
        assert!(trap.to_string().contains("out of bounds"));

        let trap = f.call(&mut store, 65537).expect_err("function should trap");
        assert!(trap.to_string().contains("out of bounds"));

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

    let mut config = Config::new();
    config.allocation_strategy(InstanceAllocationStrategy::Pooling {
        strategy: PoolingAllocationStrategy::NextAvailable,
        instance_limits: InstanceLimits {
            count: 1,
            memory_pages: 1,
            table_elements: 0,
            ..Default::default()
        },
    });
    config.dynamic_memory_guard_size(0);
    config.static_memory_guard_size(0);
    config.static_memory_maximum_size(65536);

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
fn table_limit() -> Result<()> {
    const TABLE_ELEMENTS: u32 = 10;
    let mut config = Config::new();
    config.allocation_strategy(InstanceAllocationStrategy::Pooling {
        strategy: PoolingAllocationStrategy::NextAvailable,
        instance_limits: InstanceLimits {
            count: 1,
            memory_pages: 1,
            table_elements: TABLE_ELEMENTS,
            ..Default::default()
        },
    });
    config.dynamic_memory_guard_size(0);
    config.static_memory_guard_size(0);
    config.static_memory_maximum_size(65536);

    let engine = Engine::new(&config)?;

    // Module should fail to instantiate because it has too many tables
    match Module::new(&engine, r#"(module (table 1 funcref) (table 1 funcref))"#) {
        Ok(_) => panic!("module compilation should fail"),
        Err(e) => assert_eq!(
            e.to_string(),
            "defined tables count of 2 exceeds the limit of 1",
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
        let f = instance.get_typed_func::<(), i32, _>(&mut store, "f")?;

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
        assert_eq!(table.size(&store), i);
        assert_eq!(
            table
                .grow(&mut store, 1, Val::FuncRef(None))
                .expect("table should grow"),
            i
        );
    }

    assert_eq!(table.size(&store), TABLE_ELEMENTS);
    assert!(table.grow(&mut store, 1, Val::FuncRef(None)).is_err());

    Ok(())
}

#[test]
fn table_init() -> Result<()> {
    let mut config = Config::new();
    config.allocation_strategy(InstanceAllocationStrategy::Pooling {
        strategy: PoolingAllocationStrategy::NextAvailable,
        instance_limits: InstanceLimits {
            count: 1,
            memory_pages: 0,
            table_elements: 6,
            ..Default::default()
        },
    });

    let engine = Engine::new(&config)?;

    let module = Module::new(
        &engine,
        r#"(module (table (export "t") 6 funcref) (elem (i32.const 1) 1 2 3 4) (elem (i32.const 0) 0) (func) (func (param i32)) (func (param i32 i32)) (func (param i32 i32 i32)) (func (param i32 i32 i32 i32)))"#,
    )?;

    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;
    let table = instance.get_table(&mut store, "t").unwrap();

    for i in 0..5 {
        let v = table.get(&mut store, i).expect("table should have entry");
        let f = v
            .funcref()
            .expect("expected funcref")
            .expect("expected non-null value");
        assert_eq!(f.ty(&store).params().len(), i as usize);
    }

    assert!(
        table
            .get(&mut store, 5)
            .expect("table should have entry")
            .funcref()
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

    let mut config = Config::new();
    config.allocation_strategy(InstanceAllocationStrategy::Pooling {
        strategy: PoolingAllocationStrategy::NextAvailable,
        instance_limits: InstanceLimits {
            count: 1,
            memory_pages: 1,
            table_elements: 10,
            ..Default::default()
        },
    });
    config.dynamic_memory_guard_size(0);
    config.static_memory_guard_size(0);
    config.static_memory_maximum_size(65536);

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
                Val::FuncRef(r) => assert!(r.is_none()),
                _ => panic!("expected a funcref"),
            }
            table
                .set(&mut store, i, Val::FuncRef(Some(f.clone())))
                .unwrap();
        }
    }

    Ok(())
}

#[test]
fn instantiation_limit() -> Result<()> {
    const INSTANCE_LIMIT: u32 = 10;
    let mut config = Config::new();
    config.allocation_strategy(InstanceAllocationStrategy::Pooling {
        strategy: PoolingAllocationStrategy::NextAvailable,
        instance_limits: InstanceLimits {
            count: INSTANCE_LIMIT,
            memory_pages: 1,
            table_elements: 10,
            ..Default::default()
        },
    });
    config.dynamic_memory_guard_size(0);
    config.static_memory_guard_size(0);
    config.static_memory_maximum_size(65536);

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
            Err(e) => assert_eq!(
                e.to_string(),
                format!(
                    "Limit of {} concurrent instances has been reached",
                    INSTANCE_LIMIT
                )
            ),
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
    let mut config = Config::new();
    config.allocation_strategy(InstanceAllocationStrategy::Pooling {
        strategy: PoolingAllocationStrategy::NextAvailable,
        instance_limits: InstanceLimits {
            count: 2,
            memory_pages: 1,
            table_elements: 10,
            ..Default::default()
        },
    });
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
    let mut strings = Vec::new();
    for _ in 0..1000 {
        let mut string = String::new();
        for _ in 0..1000 {
            string.push('g');
        }
        strings.push(string);
    }
    drop(strings);

    let mem = i.get_memory(&mut store, "mem").unwrap();

    // Hopefully it's still `foo`!
    assert!(mem.data(&store).starts_with(b"foo"));

    Ok(())
}

#[test]
fn multi_memory_with_imported_memories() -> Result<()> {
    // This test checks that the base address for the defined memory is correct for the instance
    // despite the presence of an imported memory.

    let mut config = Config::new();
    config.allocation_strategy(InstanceAllocationStrategy::Pooling {
        strategy: PoolingAllocationStrategy::NextAvailable,
        instance_limits: InstanceLimits {
            count: 1,
            memories: 2,
            memory_pages: 1,
            ..Default::default()
        },
    });
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
        fn memory_growing(&mut self, _: usize, _: usize, _: Option<usize>) -> bool {
            false
        }

        fn table_growing(&mut self, _: u32, _: u32, _: Option<u32>) -> bool {
            false
        }
    }

    let mut config = Config::new();
    config.wasm_reference_types(true);
    config.allocation_strategy(InstanceAllocationStrategy::Pooling {
        strategy: PoolingAllocationStrategy::NextAvailable,
        instance_limits: InstanceLimits {
            count: 1,
            ..Default::default()
        },
    });

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
    drop(Instance::new(&mut store, &module, &[])?);
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
#[cfg(target_pointer_width = "64")]
fn instance_too_large() -> Result<()> {
    let mut config = Config::new();
    config.allocation_strategy(InstanceAllocationStrategy::Pooling {
        strategy: PoolingAllocationStrategy::NextAvailable,
        instance_limits: InstanceLimits {
            size: 16,
            count: 1,
            ..Default::default()
        },
    });

    let engine = Engine::new(&config)?;
    let expected = "\
instance allocation for this module requires 304 bytes which exceeds the \
configured maximum of 16 bytes; breakdown of allocation requirement:

 * 78.95% - 240 bytes - instance state management
 * 5.26% - 16 bytes - jit store state
";
    match Module::new(&engine, "(module)") {
        Ok(_) => panic!("should have failed to compile"),
        Err(e) => assert_eq!(e.to_string(), expected),
    }

    let mut lots_of_globals = format!("(module");
    for _ in 0..100 {
        lots_of_globals.push_str("(global i32 i32.const 0)\n");
    }
    lots_of_globals.push_str(")");

    let expected = "\
instance allocation for this module requires 1904 bytes which exceeds the \
configured maximum of 16 bytes; breakdown of allocation requirement:

 * 12.61% - 240 bytes - instance state management
 * 84.03% - 1600 bytes - defined globals
";
    match Module::new(&engine, &lots_of_globals) {
        Ok(_) => panic!("should have failed to compile"),
        Err(e) => assert_eq!(e.to_string(), expected),
    }

    Ok(())
}
