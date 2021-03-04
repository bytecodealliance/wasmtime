use anyhow::Result;
use wasmtime::*;

#[test]
fn successful_instantiation() -> Result<()> {
    let mut config = Config::new();
    config.with_allocation_strategy(InstanceAllocationStrategy::Pooling {
        strategy: PoolingAllocationStrategy::NextAvailable,
        module_limits: ModuleLimits {
            memory_pages: 1,
            table_elements: 10,
            ..Default::default()
        },
        instance_limits: InstanceLimits {
            count: 1,
            memory_reservation_size: 1,
        },
    })?;

    let engine = Engine::new(&config);
    let module = Module::new(&engine, r#"(module (memory 1) (table 10 funcref))"#)?;

    // Module should instantiate
    let store = Store::new(&engine);
    Instance::new(&store, &module, &[])?;

    Ok(())
}

#[test]
fn memory_limit() -> Result<()> {
    let mut config = Config::new();
    config.with_allocation_strategy(InstanceAllocationStrategy::Pooling {
        strategy: PoolingAllocationStrategy::NextAvailable,
        module_limits: ModuleLimits {
            memory_pages: 3,
            table_elements: 10,
            ..Default::default()
        },
        instance_limits: InstanceLimits {
            count: 1,
            memory_reservation_size: 196608,
        },
    })?;

    let engine = Engine::new(&config);

    // Module should fail to validate because the minimum is greater than the configured limit
    match Module::new(&engine, r#"(module (memory 4))"#) {
        Ok(_) => panic!("module compilation should fail"),
        Err(e) => assert_eq!(
            e.to_string(),
            "memory index 0 has a minimum page size of 4 which exceeds the limit of 3"
        ),
    }

    let module = Module::new(
        &engine,
        r#"(module (memory (export "m") 0) (func (export "f") (result i32) (memory.grow (i32.const 1))))"#,
    )?;

    // Instantiate the module and grow the memory via the `f` function
    {
        let store = Store::new(&engine);
        let instance = Instance::new(&store, &module, &[])?;
        let f = instance.get_func("f").unwrap().get0::<i32>().unwrap();

        assert_eq!(f().expect("function should not trap"), 0);
        assert_eq!(f().expect("function should not trap"), 1);
        assert_eq!(f().expect("function should not trap"), 2);
        assert_eq!(f().expect("function should not trap"), -1);
        assert_eq!(f().expect("function should not trap"), -1);
    }

    // Instantiate the module and grow the memory via the Wasmtime API
    let store = Store::new(&engine);
    let instance = Instance::new(&store, &module, &[])?;

    let memory = instance.get_memory("m").unwrap();
    assert_eq!(memory.size(), 0);
    assert_eq!(memory.grow(1).expect("memory should grow"), 0);
    assert_eq!(memory.size(), 1);
    assert_eq!(memory.grow(1).expect("memory should grow"), 1);
    assert_eq!(memory.size(), 2);
    assert_eq!(memory.grow(1).expect("memory should grow"), 2);
    assert_eq!(memory.size(), 3);
    assert!(memory.grow(1).is_err());

    Ok(())
}

#[test]
fn memory_init() -> Result<()> {
    let mut config = Config::new();
    config.with_allocation_strategy(InstanceAllocationStrategy::Pooling {
        strategy: PoolingAllocationStrategy::NextAvailable,
        module_limits: ModuleLimits {
            memory_pages: 2,
            table_elements: 0,
            ..Default::default()
        },
        instance_limits: InstanceLimits {
            count: 1,
            ..Default::default()
        },
    })?;

    let engine = Engine::new(&config);

    let module = Module::new(
        &engine,
        r#"(module (memory (export "m") 2) (data (i32.const 65530) "this data spans multiple pages") (data (i32.const 10) "hello world"))"#,
    )?;

    let store = Store::new(&engine);
    let instance = Instance::new(&store, &module, &[])?;
    let memory = instance.get_memory("m").unwrap();

    unsafe {
        assert_eq!(
            &memory.data_unchecked()[65530..65560],
            b"this data spans multiple pages"
        );
        assert_eq!(&memory.data_unchecked()[10..21], b"hello world");
    }

    Ok(())
}

#[test]
fn memory_guard_page_trap() -> Result<()> {
    let mut config = Config::new();
    config.with_allocation_strategy(InstanceAllocationStrategy::Pooling {
        strategy: PoolingAllocationStrategy::NextAvailable,
        module_limits: ModuleLimits {
            memory_pages: 2,
            table_elements: 0,
            ..Default::default()
        },
        instance_limits: InstanceLimits {
            count: 1,
            ..Default::default()
        },
    })?;

    let engine = Engine::new(&config);

    let module = Module::new(
        &engine,
        r#"(module (memory (export "m") 0) (func (export "f") (param i32) local.get 0 i32.load drop))"#,
    )?;

    // Instantiate the module and check for out of bounds trap
    for _ in 0..10 {
        let store = Store::new(&engine);
        let instance = Instance::new(&store, &module, &[])?;
        let m = instance.get_memory("m").unwrap();
        let f = instance.get_func("f").unwrap().get1::<i32, ()>().unwrap();

        let trap = f(0).expect_err("function should trap");
        assert!(trap.to_string().contains("out of bounds"));

        let trap = f(1).expect_err("function should trap");
        assert!(trap.to_string().contains("out of bounds"));

        m.grow(1).expect("memory should grow");
        f(0).expect("function should not trap");

        let trap = f(65536).expect_err("function should trap");
        assert!(trap.to_string().contains("out of bounds"));

        let trap = f(65537).expect_err("function should trap");
        assert!(trap.to_string().contains("out of bounds"));

        m.grow(1).expect("memory should grow");
        f(65536).expect("function should not trap");

        m.grow(1).expect_err("memory should be at the limit");
    }

    Ok(())
}

#[test]
#[cfg_attr(target_arch = "aarch64", ignore)] // https://github.com/bytecodealliance/wasmtime/pull/2518#issuecomment-747280133
fn memory_zeroed() -> Result<()> {
    let mut config = Config::new();
    config.with_allocation_strategy(InstanceAllocationStrategy::Pooling {
        strategy: PoolingAllocationStrategy::NextAvailable,
        module_limits: ModuleLimits {
            memory_pages: 1,
            table_elements: 0,
            ..Default::default()
        },
        instance_limits: InstanceLimits {
            count: 1,
            memory_reservation_size: 1,
        },
    })?;

    let engine = Engine::new(&config);

    let module = Module::new(&engine, r#"(module (memory (export "m") 1))"#)?;

    // Instantiate the module repeatedly after writing data to the entire memory
    for _ in 0..10 {
        let store = Store::new(&engine);
        let instance = Instance::new(&store, &module, &[])?;
        let memory = instance.get_memory("m").unwrap();

        assert_eq!(memory.size(), 1);
        assert_eq!(memory.data_size(), 65536);

        let ptr = memory.data_ptr();

        unsafe {
            for i in 0..8192 {
                assert_eq!(*ptr.cast::<u64>().offset(i), 0);
            }
            std::ptr::write_bytes(ptr, 0xFE, memory.data_size());
        }
    }

    Ok(())
}

#[test]
fn table_limit() -> Result<()> {
    const TABLE_ELEMENTS: u32 = 10;
    let mut config = Config::new();
    config.with_allocation_strategy(InstanceAllocationStrategy::Pooling {
        strategy: PoolingAllocationStrategy::NextAvailable,
        module_limits: ModuleLimits {
            memory_pages: 1,
            table_elements: TABLE_ELEMENTS,
            ..Default::default()
        },
        instance_limits: InstanceLimits {
            count: 1,
            memory_reservation_size: 1,
        },
    })?;

    let engine = Engine::new(&config);

    // Module should fail to validate because the minimum is greater than the configured limit
    match Module::new(&engine, r#"(module (table 31 funcref))"#) {
        Ok(_) => panic!("module compilation should fail"),
        Err(e) => assert_eq!(
            e.to_string(),
            "table index 0 has a minimum element size of 31 which exceeds the limit of 10"
        ),
    }

    let module = Module::new(
        &engine,
        r#"(module (table (export "t") 0 funcref) (func (export "f") (result i32) (table.grow (ref.null func) (i32.const 1))))"#,
    )?;

    // Instantiate the module and grow the table via the `f` function
    {
        let store = Store::new(&engine);
        let instance = Instance::new(&store, &module, &[])?;
        let f = instance.get_func("f").unwrap().get0::<i32>().unwrap();

        for i in 0..TABLE_ELEMENTS {
            assert_eq!(f().expect("function should not trap"), i as i32);
        }

        assert_eq!(f().expect("function should not trap"), -1);
        assert_eq!(f().expect("function should not trap"), -1);
    }

    // Instantiate the module and grow the table via the Wasmtime API
    let store = Store::new(&engine);
    let instance = Instance::new(&store, &module, &[])?;

    let table = instance.get_table("t").unwrap();

    for i in 0..TABLE_ELEMENTS {
        assert_eq!(table.size(), i);
        assert_eq!(
            table
                .grow(1, Val::FuncRef(None))
                .expect("table should grow"),
            i
        );
    }

    assert_eq!(table.size(), TABLE_ELEMENTS);
    assert!(table.grow(1, Val::FuncRef(None)).is_err());

    Ok(())
}

#[test]
fn table_init() -> Result<()> {
    let mut config = Config::new();
    config.with_allocation_strategy(InstanceAllocationStrategy::Pooling {
        strategy: PoolingAllocationStrategy::NextAvailable,
        module_limits: ModuleLimits {
            memory_pages: 0,
            table_elements: 6,
            ..Default::default()
        },
        instance_limits: InstanceLimits {
            count: 1,
            ..Default::default()
        },
    })?;

    let engine = Engine::new(&config);

    let module = Module::new(
        &engine,
        r#"(module (table (export "t") 6 funcref) (elem (i32.const 1) 1 2 3 4) (elem (i32.const 0) 0) (func) (func (param i32)) (func (param i32 i32)) (func (param i32 i32 i32)) (func (param i32 i32 i32 i32)))"#,
    )?;

    let store = Store::new(&engine);
    let instance = Instance::new(&store, &module, &[])?;
    let table = instance.get_table("t").unwrap();

    for i in 0..5 {
        let v = table.get(i).expect("table should have entry");
        let f = v
            .funcref()
            .expect("expected funcref")
            .expect("expected non-null value");
        assert_eq!(f.ty().params().len(), i as usize);
    }

    assert!(
        table
            .get(5)
            .expect("table should have entry")
            .funcref()
            .expect("expected funcref")
            .is_none(),
        "funcref should be null"
    );

    Ok(())
}

#[test]
#[cfg_attr(target_arch = "aarch64", ignore)] // https://github.com/bytecodealliance/wasmtime/pull/2518#issuecomment-747280133
fn table_zeroed() -> Result<()> {
    let mut config = Config::new();
    config.with_allocation_strategy(InstanceAllocationStrategy::Pooling {
        strategy: PoolingAllocationStrategy::NextAvailable,
        module_limits: ModuleLimits {
            memory_pages: 1,
            table_elements: 10,
            ..Default::default()
        },
        instance_limits: InstanceLimits {
            count: 1,
            memory_reservation_size: 1,
        },
    })?;

    let engine = Engine::new(&config);

    let module = Module::new(&engine, r#"(module (table (export "t") 10 funcref))"#)?;

    // Instantiate the module repeatedly after filling table elements
    for _ in 0..10 {
        let store = Store::new(&engine);
        let instance = Instance::new(&store, &module, &[])?;
        let table = instance.get_table("t").unwrap();
        let f = Func::wrap(&store, || {});

        assert_eq!(table.size(), 10);

        for i in 0..10 {
            match table.get(i).unwrap() {
                Val::FuncRef(r) => assert!(r.is_none()),
                _ => panic!("expected a funcref"),
            }
            table.set(i, Val::FuncRef(Some(f.clone()))).unwrap();
        }
    }

    Ok(())
}

#[test]
fn instantiation_limit() -> Result<()> {
    const INSTANCE_LIMIT: u32 = 10;
    let mut config = Config::new();
    config.with_allocation_strategy(InstanceAllocationStrategy::Pooling {
        strategy: PoolingAllocationStrategy::NextAvailable,
        module_limits: ModuleLimits {
            memory_pages: 1,
            table_elements: 10,
            ..Default::default()
        },
        instance_limits: InstanceLimits {
            count: INSTANCE_LIMIT,
            memory_reservation_size: 1,
        },
    })?;

    let engine = Engine::new(&config);
    let module = Module::new(&engine, r#"(module)"#)?;

    // Instantiate to the limit
    {
        let store = Store::new(&engine);

        for _ in 0..INSTANCE_LIMIT {
            Instance::new(&store, &module, &[])?;
        }

        match Instance::new(&store, &module, &[]) {
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

    let store = Store::new(&engine);

    for _ in 0..INSTANCE_LIMIT {
        Instance::new(&store, &module, &[])?;
    }

    Ok(())
}
