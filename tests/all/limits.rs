use wasmtime::*;

const WASM_PAGE_SIZE: usize = wasmtime_environ::WASM_PAGE_SIZE as usize;

#[test]
#[cfg_attr(miri, ignore)]
fn test_limits() -> Result<()> {
    let engine = Engine::default();
    let module = Module::new(
        &engine,
        r#"(module
            (memory $m (export "m") 0)
            (table (export "t") 0 funcref)
            (func (export "grow") (param i32) (result i32)
              (memory.grow $m (local.get 0)))
           )"#,
    )?;

    let mut store = Store::new(
        &engine,
        StoreLimitsBuilder::new()
            .memory_size(10 * WASM_PAGE_SIZE)
            .table_elements(5)
            .build(),
    );
    store.limiter(|s| s as &mut dyn ResourceLimiter);

    let instance = Instance::new(&mut store, &module, &[])?;

    // Test instance exports and host objects hitting the limit
    for memory in IntoIterator::into_iter([
        instance.get_memory(&mut store, "m").unwrap(),
        Memory::new(&mut store, MemoryType::new(0, None))?,
    ]) {
        memory.grow(&mut store, 3)?;
        memory.grow(&mut store, 5)?;
        memory.grow(&mut store, 2)?;

        assert_eq!(
            memory
                .grow(&mut store, 1)
                .map_err(|e| e.to_string())
                .unwrap_err(),
            "failed to grow memory by `1`"
        );
    }

    // Test instance exports and host objects hitting the limit
    for table in IntoIterator::into_iter([
        instance.get_table(&mut store, "t").unwrap(),
        Table::new(
            &mut store,
            TableType::new(RefType::FUNCREF, 0, None),
            Ref::Func(None),
        )?,
    ]) {
        table.grow(&mut store, 2, Ref::Func(None))?;
        table.grow(&mut store, 1, Ref::Func(None))?;
        table.grow(&mut store, 2, Ref::Func(None))?;

        assert_eq!(
            table
                .grow(&mut store, 1, Ref::Func(None))
                .map_err(|e| e.to_string())
                .unwrap_err(),
            "failed to grow table by `1`"
        );
    }

    // Make a new store and instance to test memory grow through wasm
    let mut store = Store::new(
        &engine,
        StoreLimitsBuilder::new()
            .memory_size(10 * WASM_PAGE_SIZE)
            .table_elements(5)
            .build(),
    );
    store.limiter(|s| s as &mut dyn ResourceLimiter);
    let instance = Instance::new(&mut store, &module, &[])?;
    let grow = instance.get_func(&mut store, "grow").unwrap();
    let grow = grow.typed::<i32, i32>(&store).unwrap();

    grow.call(&mut store, 3).unwrap();
    grow.call(&mut store, 5).unwrap();
    grow.call(&mut store, 2).unwrap();

    // Wasm grow failure returns -1.
    assert_eq!(grow.call(&mut store, 1).unwrap(), -1);

    Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_limits_async() -> Result<()> {
    let mut config = Config::new();
    config.async_support(true);
    let engine = Engine::new(&config).unwrap();
    let module = Module::new(
        &engine,
        r#"(module (memory (export "m") 0) (table (export "t") 0 funcref))"#,
    )?;

    struct LimitsAsync {
        memory_size: usize,
        table_elements: u32,
    }
    #[async_trait::async_trait]
    impl ResourceLimiterAsync for LimitsAsync {
        async fn memory_growing(
            &mut self,
            _current: usize,
            desired: usize,
            _maximum: Option<usize>,
        ) -> Result<bool> {
            Ok(desired <= self.memory_size)
        }
        async fn table_growing(
            &mut self,
            _current: u32,
            desired: u32,
            _maximum: Option<u32>,
        ) -> Result<bool> {
            Ok(desired <= self.table_elements)
        }
    }

    let mut store = Store::new(
        &engine,
        LimitsAsync {
            memory_size: 10 * WASM_PAGE_SIZE,
            table_elements: 5,
        },
    );

    store.limiter_async(|s| s as &mut dyn ResourceLimiterAsync);

    let instance = Instance::new_async(&mut store, &module, &[]).await?;

    // Test instance exports and host objects hitting the limit
    for memory in IntoIterator::into_iter([
        instance.get_memory(&mut store, "m").unwrap(),
        Memory::new_async(&mut store, MemoryType::new(0, None)).await?,
    ]) {
        memory.grow_async(&mut store, 3).await?;
        memory.grow_async(&mut store, 5).await?;
        memory.grow_async(&mut store, 2).await?;

        assert_eq!(
            memory
                .grow_async(&mut store, 1)
                .await
                .map_err(|e| e.to_string())
                .unwrap_err(),
            "failed to grow memory by `1`"
        );
    }

    // Test instance exports and host objects hitting the limit
    for table in IntoIterator::into_iter([
        instance.get_table(&mut store, "t").unwrap(),
        Table::new_async(
            &mut store,
            TableType::new(RefType::FUNCREF, 0, None),
            Ref::Func(None),
        )
        .await?,
    ]) {
        table.grow_async(&mut store, 2, Ref::Func(None)).await?;
        table.grow_async(&mut store, 1, Ref::Func(None)).await?;
        table.grow_async(&mut store, 2, Ref::Func(None)).await?;

        assert_eq!(
            table
                .grow_async(&mut store, 1, Ref::Func(None))
                .await
                .map_err(|e| e.to_string())
                .unwrap_err(),
            "failed to grow table by `1`"
        );
    }

    Ok(())
}

#[test]
fn test_limits_memory_only() -> Result<()> {
    let engine = Engine::default();
    let module = Module::new(
        &engine,
        r#"(module (memory (export "m") 0) (table (export "t") 0 funcref))"#,
    )?;

    let mut store = Store::new(
        &engine,
        StoreLimitsBuilder::new()
            .memory_size(10 * WASM_PAGE_SIZE)
            .build(),
    );
    store.limiter(|s| s as &mut dyn ResourceLimiter);

    let instance = Instance::new(&mut store, &module, &[])?;

    // Test instance exports and host objects hitting the limit
    for memory in IntoIterator::into_iter([
        instance.get_memory(&mut store, "m").unwrap(),
        Memory::new(&mut store, MemoryType::new(0, None))?,
    ]) {
        memory.grow(&mut store, 3)?;
        memory.grow(&mut store, 5)?;
        memory.grow(&mut store, 2)?;

        assert_eq!(
            memory
                .grow(&mut store, 1)
                .map_err(|e| e.to_string())
                .unwrap_err(),
            "failed to grow memory by `1`"
        );
    }

    // Test instance exports and host objects *not* hitting the limit
    for table in IntoIterator::into_iter([
        instance.get_table(&mut store, "t").unwrap(),
        Table::new(
            &mut store,
            TableType::new(RefType::FUNCREF, 0, None),
            Ref::Func(None),
        )?,
    ]) {
        table.grow(&mut store, 2, Ref::Func(None))?;
        table.grow(&mut store, 1, Ref::Func(None))?;
        table.grow(&mut store, 2, Ref::Func(None))?;
        table.grow(&mut store, 1, Ref::Func(None))?;
    }

    Ok(())
}

#[test]
fn test_initial_memory_limits_exceeded() -> Result<()> {
    let engine = Engine::default();
    let module = Module::new(&engine, r#"(module (memory (export "m") 11))"#)?;

    let mut store = Store::new(
        &engine,
        StoreLimitsBuilder::new()
            .memory_size(10 * WASM_PAGE_SIZE)
            .build(),
    );
    store.limiter(|s| s as &mut dyn ResourceLimiter);

    match Instance::new(&mut store, &module, &[]) {
        Ok(_) => unreachable!(),
        Err(e) => assert_eq!(
            e.to_string(),
            "memory minimum size of 11 pages exceeds memory limits"
        ),
    }

    match Memory::new(&mut store, MemoryType::new(25, None)) {
        Ok(_) => unreachable!(),
        Err(e) => assert_eq!(
            e.to_string(),
            "memory minimum size of 25 pages exceeds memory limits"
        ),
    }

    Ok(())
}

#[test]
fn test_limits_table_only() -> Result<()> {
    let engine = Engine::default();
    let module = Module::new(
        &engine,
        r#"(module (memory (export "m") 0) (table (export "t") 0 funcref))"#,
    )?;

    let mut store = Store::new(&engine, StoreLimitsBuilder::new().table_elements(5).build());
    store.limiter(|s| s as &mut dyn ResourceLimiter);

    let instance = Instance::new(&mut store, &module, &[])?;

    // Test instance exports and host objects *not* hitting the limit
    for memory in IntoIterator::into_iter([
        instance.get_memory(&mut store, "m").unwrap(),
        Memory::new(&mut store, MemoryType::new(0, None))?,
    ]) {
        memory.grow(&mut store, 3)?;
        memory.grow(&mut store, 5)?;
        memory.grow(&mut store, 2)?;
        memory.grow(&mut store, 1)?;
    }

    // Test instance exports and host objects hitting the limit
    for table in IntoIterator::into_iter([
        instance.get_table(&mut store, "t").unwrap(),
        Table::new(
            &mut store,
            TableType::new(RefType::FUNCREF, 0, None),
            Ref::Func(None),
        )?,
    ]) {
        table.grow(&mut store, 2, Ref::Func(None))?;
        table.grow(&mut store, 1, Ref::Func(None))?;
        table.grow(&mut store, 2, Ref::Func(None))?;

        assert_eq!(
            table
                .grow(&mut store, 1, Ref::Func(None))
                .map_err(|e| e.to_string())
                .unwrap_err(),
            "failed to grow table by `1`"
        );
    }

    Ok(())
}

#[test]
fn test_initial_table_limits_exceeded() -> Result<()> {
    let engine = Engine::default();
    let module = Module::new(&engine, r#"(module (table (export "t") 23 funcref))"#)?;

    let mut store = Store::new(&engine, StoreLimitsBuilder::new().table_elements(4).build());
    store.limiter(|s| s as &mut dyn ResourceLimiter);

    match Instance::new(&mut store, &module, &[]) {
        Ok(_) => unreachable!(),
        Err(e) => assert_eq!(
            e.to_string(),
            "table minimum size of 23 elements exceeds table limits"
        ),
    }

    match Table::new(
        &mut store,
        TableType::new(RefType::FUNCREF, 99, None),
        Ref::Func(None),
    ) {
        Ok(_) => unreachable!(),
        Err(e) => assert_eq!(
            e.to_string(),
            "table minimum size of 99 elements exceeds table limits"
        ),
    }

    Ok(())
}

#[test]
fn test_pooling_allocator_initial_limits_exceeded() -> Result<()> {
    let mut pool = crate::small_pool_config();
    pool.total_memories(2)
        .max_memories_per_module(2)
        .max_memory_size(5 << 16)
        .memory_protection_keys(MpkEnabled::Disable);
    let mut config = Config::new();
    config.wasm_multi_memory(true);
    config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));

    let engine = Engine::new(&config)?;
    let module = Module::new(
        &engine,
        r#"(module (memory (export "m1") 2) (memory (export "m2") 5))"#,
    )?;

    let mut store = Store::new(
        &engine,
        StoreLimitsBuilder::new()
            .memory_size(3 * WASM_PAGE_SIZE)
            .build(),
    );
    store.limiter(|s| s as &mut dyn ResourceLimiter);

    match Instance::new(&mut store, &module, &[]) {
        Ok(_) => unreachable!(),
        Err(e) => assert_eq!(
            e.to_string(),
            "memory minimum size of 5 pages exceeds memory limits"
        ),
    }

    // An instance should still be able to be created after the failure above
    let module = Module::new(&engine, r#"(module (memory (export "m") 2))"#)?;

    Instance::new(&mut store, &module, &[])?;

    Ok(())
}

struct MemoryContext {
    host_memory_used: usize,
    wasm_memory_used: usize,
    memory_limit: usize,
    limit_exceeded: bool,
}

impl ResourceLimiter for MemoryContext {
    fn memory_growing(
        &mut self,
        current: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> Result<bool> {
        // Check if the desired exceeds a maximum (either from Wasm or from the host)
        assert!(desired < maximum.unwrap_or(usize::MAX));

        assert_eq!(current, self.wasm_memory_used);

        if desired + self.host_memory_used > self.memory_limit {
            self.limit_exceeded = true;
            return Ok(false);
        }

        self.wasm_memory_used = desired;
        Ok(true)
    }
    fn table_growing(
        &mut self,
        _current: u32,
        _desired: u32,
        _maximum: Option<u32>,
    ) -> Result<bool> {
        Ok(true)
    }
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_custom_memory_limiter() -> Result<()> {
    let engine = Engine::default();
    let mut linker = Linker::new(&engine);

    // This approximates a function that would "allocate" resources that the host tracks.
    // Here this is a simple function that increments the current host memory "used".
    linker.func_wrap(
        "",
        "alloc",
        |mut caller: Caller<'_, MemoryContext>, size: u32| -> u32 {
            let ctx = caller.data_mut();
            let size = size as usize;

            if size + ctx.host_memory_used + ctx.wasm_memory_used <= ctx.memory_limit {
                ctx.host_memory_used += size;
                return 1;
            }

            ctx.limit_exceeded = true;

            0
        },
    )?;

    let module = Module::new(
        &engine,
        r#"(module (import "" "alloc" (func $alloc (param i32) (result i32))) (memory (export "m") 0) (func (export "f") (param i32) (result i32) local.get 0 call $alloc))"#,
    )?;

    let context = MemoryContext {
        host_memory_used: 0,
        wasm_memory_used: 0,
        memory_limit: 1 << 20, // 16 wasm pages is the limit for both wasm + host memory
        limit_exceeded: false,
    };

    let mut store = Store::new(&engine, context);
    store.limiter(|s| s as &mut dyn ResourceLimiter);
    let instance = linker.instantiate(&mut store, &module)?;
    let memory = instance.get_memory(&mut store, "m").unwrap();

    // Grow the memory by 640 KiB
    memory.grow(&mut store, 3)?;
    memory.grow(&mut store, 5)?;
    memory.grow(&mut store, 2)?;

    assert!(!store.data().limit_exceeded);

    // Grow the host "memory" by 384 KiB
    let f = instance.get_typed_func::<u32, u32>(&mut store, "f")?;

    assert_eq!(f.call(&mut store, 1 * 0x10000)?, 1);
    assert_eq!(f.call(&mut store, 3 * 0x10000)?, 1);
    assert_eq!(f.call(&mut store, 2 * 0x10000)?, 1);

    // Memory is at the maximum, but the limit hasn't been exceeded
    assert!(!store.data().limit_exceeded);

    // Try to grow the memory again
    assert_eq!(
        memory
            .grow(&mut store, 1)
            .map_err(|e| e.to_string())
            .unwrap_err(),
        "failed to grow memory by `1`"
    );

    assert!(store.data().limit_exceeded);

    // Try to grow the host "memory" again
    assert_eq!(f.call(&mut store, 1)?, 0);

    assert!(store.data().limit_exceeded);

    drop(store);

    Ok(())
}

#[async_trait::async_trait]
impl ResourceLimiterAsync for MemoryContext {
    async fn memory_growing(
        &mut self,
        current: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> Result<bool> {
        // Show we can await in this async context:
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        // Check if the desired exceeds a maximum (either from Wasm or from the host)
        assert!(desired < maximum.unwrap_or(usize::MAX));

        assert_eq!(current, self.wasm_memory_used);

        if desired + self.host_memory_used > self.memory_limit {
            self.limit_exceeded = true;
            return Ok(false);
        }

        self.wasm_memory_used = desired;
        Ok(true)
    }
    async fn table_growing(
        &mut self,
        _current: u32,
        _desired: u32,
        _maximum: Option<u32>,
    ) -> Result<bool> {
        Ok(true)
    }
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_custom_memory_limiter_async() -> Result<()> {
    let mut config = Config::new();
    config.async_support(true);
    let engine = Engine::new(&config).unwrap();
    let mut linker = Linker::new(&engine);

    // This approximates a function that would "allocate" resources that the host tracks.
    // Here this is a simple function that increments the current host memory "used".
    linker.func_wrap(
        "",
        "alloc",
        |mut caller: Caller<'_, MemoryContext>, size: u32| -> u32 {
            let ctx = caller.data_mut();
            let size = size as usize;

            if size + ctx.host_memory_used + ctx.wasm_memory_used <= ctx.memory_limit {
                ctx.host_memory_used += size;
                return 1;
            }

            ctx.limit_exceeded = true;

            0
        },
    )?;

    let module = Module::new(
        &engine,
        r#"(module (import "" "alloc" (func $alloc (param i32) (result i32))) (memory (export "m") 0) (func (export "f") (param i32) (result i32) local.get 0 call $alloc))"#,
    )?;

    let context = MemoryContext {
        host_memory_used: 0,
        wasm_memory_used: 0,
        memory_limit: 1 << 20, // 16 wasm pages is the limit for both wasm + host memory
        limit_exceeded: false,
    };

    let mut store = Store::new(&engine, context);
    store.limiter_async(|s| s as &mut dyn ResourceLimiterAsync);
    let instance = linker.instantiate_async(&mut store, &module).await?;
    let memory = instance.get_memory(&mut store, "m").unwrap();

    // Grow the memory by 640 KiB
    memory.grow_async(&mut store, 3).await?;
    memory.grow_async(&mut store, 5).await?;
    memory.grow_async(&mut store, 2).await?;

    assert!(!store.data().limit_exceeded);

    // Grow the host "memory" by 384 KiB
    let f = instance.get_typed_func::<u32, u32>(&mut store, "f")?;

    assert_eq!(f.call_async(&mut store, 1 * 0x10000).await?, 1);
    assert_eq!(f.call_async(&mut store, 3 * 0x10000).await?, 1);
    assert_eq!(f.call_async(&mut store, 2 * 0x10000).await?, 1);

    // Memory is at the maximum, but the limit hasn't been exceeded
    assert!(!store.data().limit_exceeded);

    // Try to grow the memory again
    assert_eq!(
        memory
            .grow_async(&mut store, 1)
            .await
            .map_err(|e| e.to_string())
            .unwrap_err(),
        "failed to grow memory by `1`"
    );

    assert!(store.data().limit_exceeded);

    // Try to grow the host "memory" again
    assert_eq!(f.call_async(&mut store, 1).await?, 0);

    assert!(store.data().limit_exceeded);

    drop(store);

    Ok(())
}

struct TableContext {
    elements_used: u32,
    element_limit: u32,
    limit_exceeded: bool,
}

impl ResourceLimiter for TableContext {
    fn memory_growing(
        &mut self,
        _current: usize,
        _desired: usize,
        _maximum: Option<usize>,
    ) -> Result<bool> {
        Ok(true)
    }
    fn table_growing(&mut self, current: u32, desired: u32, maximum: Option<u32>) -> Result<bool> {
        // Check if the desired exceeds a maximum (either from Wasm or from the host)
        assert!(desired < maximum.unwrap_or(u32::MAX));
        assert_eq!(current, self.elements_used);
        Ok(if desired > self.element_limit {
            self.limit_exceeded = true;
            false
        } else {
            self.elements_used = desired;
            true
        })
    }
}

#[test]
fn test_custom_table_limiter() -> Result<()> {
    let engine = Engine::default();
    let linker = Linker::new(&engine);

    let module = Module::new(&engine, r#"(module (table (export "t") 0 funcref))"#)?;

    let context = TableContext {
        elements_used: 0,
        element_limit: 10,
        limit_exceeded: false,
    };

    let mut store = Store::new(&engine, context);
    store.limiter(|s| s as &mut dyn ResourceLimiter);
    let instance = linker.instantiate(&mut store, &module)?;
    let table = instance.get_table(&mut store, "t").unwrap();

    // Grow the table by 10 elements
    table.grow(&mut store, 3, Ref::Func(None))?;
    table.grow(&mut store, 5, Ref::Func(None))?;
    table.grow(&mut store, 2, Ref::Func(None))?;

    assert!(!store.data().limit_exceeded);

    // Table is at the maximum, but the limit hasn't been exceeded
    assert!(!store.data().limit_exceeded);

    // Try to grow the memory again
    assert_eq!(
        table
            .grow(&mut store, 1, Ref::Func(None))
            .map_err(|e| e.to_string())
            .unwrap_err(),
        "failed to grow table by `1`"
    );

    assert!(store.data().limit_exceeded);

    Ok(())
}

#[derive(Default)]
struct FailureDetector {
    /// Arguments of most recent call to memory_growing
    memory_current: usize,
    memory_desired: usize,
    /// Display impl of most recent call to memory_grow_failed
    memory_error: Option<String>,
    /// Arguments of most recent call to table_growing
    table_current: u32,
    table_desired: u32,
    /// Display impl of most recent call to table_grow_failed
    table_error: Option<String>,
}

impl ResourceLimiter for FailureDetector {
    fn memory_growing(
        &mut self,
        current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> Result<bool> {
        self.memory_current = current;
        self.memory_desired = desired;
        Ok(true)
    }
    fn memory_grow_failed(&mut self, err: anyhow::Error) -> Result<()> {
        self.memory_error = Some(err.to_string());
        Ok(())
    }
    fn table_growing(&mut self, current: u32, desired: u32, _maximum: Option<u32>) -> Result<bool> {
        self.table_current = current;
        self.table_desired = desired;
        Ok(true)
    }
    fn table_grow_failed(&mut self, err: anyhow::Error) -> Result<()> {
        self.table_error = Some(err.to_string());
        Ok(())
    }
}

#[test]
fn custom_limiter_detect_grow_failure() -> Result<()> {
    if std::env::var("WASMTIME_TEST_NO_HOG_MEMORY").is_ok() {
        return Ok(());
    }
    let mut pool = crate::small_pool_config();
    pool.max_memory_size(10 << 16).table_elements(10);
    let mut config = Config::new();
    config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));
    let engine = Engine::new(&config).unwrap();
    let linker = Linker::new(&engine);

    let module = Module::new(
        &engine,
        r#"(module (memory (export "m") 0) (table (export "t") 0 funcref))"#,
    )?;

    let context = FailureDetector::default();

    let mut store = Store::new(&engine, context);
    store.limiter(|s| s as &mut dyn ResourceLimiter);
    let instance = linker.instantiate(&mut store, &module)?;
    let memory = instance.get_memory(&mut store, "m").unwrap();

    // Grow the memory by 640 KiB (10 pages)
    memory.grow(&mut store, 10)?;

    assert!(store.data().memory_error.is_none());
    assert_eq!(store.data().memory_current, 0);
    assert_eq!(store.data().memory_desired, 10 * 64 * 1024);

    // Grow past the static limit set by ModuleLimits.
    // The ResourceLimiter will permit this, but the grow will fail.
    assert_eq!(
        memory.grow(&mut store, 1).unwrap_err().to_string(),
        "failed to grow memory by `1`"
    );

    assert_eq!(store.data().memory_current, 10 * 64 * 1024);
    assert_eq!(store.data().memory_desired, 11 * 64 * 1024);
    assert_eq!(
        store.data().memory_error.as_ref().unwrap(),
        "Memory maximum size exceeded"
    );

    let table = instance.get_table(&mut store, "t").unwrap();
    // Grow the table 10 elements
    table.grow(&mut store, 10, Ref::Func(None))?;

    assert!(store.data().table_error.is_none());
    assert_eq!(store.data().table_current, 0);
    assert_eq!(store.data().table_desired, 10);

    // Grow past the static limit set by ModuleLimits.
    // The ResourceLimiter will permit this, but the grow will fail.
    assert_eq!(
        table
            .grow(&mut store, 1, Ref::Func(None))
            .unwrap_err()
            .to_string(),
        "failed to grow table by `1`"
    );

    assert_eq!(store.data().table_current, 10);
    assert_eq!(store.data().table_desired, 11);
    assert_eq!(
        store.data().table_error.as_ref().unwrap(),
        "Table maximum size exceeded"
    );

    drop(store);

    Ok(())
}

#[async_trait::async_trait]
impl ResourceLimiterAsync for FailureDetector {
    async fn memory_growing(
        &mut self,
        current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> Result<bool> {
        // Show we can await in this async context:
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        self.memory_current = current;
        self.memory_desired = desired;
        Ok(true)
    }
    fn memory_grow_failed(&mut self, err: anyhow::Error) -> Result<()> {
        self.memory_error = Some(err.to_string());
        Ok(())
    }

    async fn table_growing(
        &mut self,
        current: u32,
        desired: u32,
        _maximum: Option<u32>,
    ) -> Result<bool> {
        self.table_current = current;
        self.table_desired = desired;
        Ok(true)
    }
    fn table_grow_failed(&mut self, err: anyhow::Error) -> Result<()> {
        self.table_error = Some(err.to_string());
        Ok(())
    }
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn custom_limiter_async_detect_grow_failure() -> Result<()> {
    if std::env::var("WASMTIME_TEST_NO_HOG_MEMORY").is_ok() {
        return Ok(());
    }
    let mut pool = crate::small_pool_config();
    pool.max_memory_size(10 << 16).table_elements(10);
    let mut config = Config::new();
    config.async_support(true);
    config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));
    let engine = Engine::new(&config).unwrap();
    let linker = Linker::new(&engine);

    let module = Module::new(
        &engine,
        r#"(module (memory (export "m") 0) (table (export "t") 0 funcref))"#,
    )?;

    let context = FailureDetector::default();

    let mut store = Store::new(&engine, context);
    store.limiter_async(|s| s as &mut dyn ResourceLimiterAsync);
    let instance = linker.instantiate_async(&mut store, &module).await?;
    let memory = instance.get_memory(&mut store, "m").unwrap();

    // Grow the memory by 640 KiB (10 pages)
    memory.grow_async(&mut store, 10).await?;

    assert!(store.data().memory_error.is_none());
    assert_eq!(store.data().memory_current, 0);
    assert_eq!(store.data().memory_desired, 10 * 64 * 1024);

    // Grow past the static limit set by ModuleLimits.
    // The ResourcLimiterAsync will permit this, but the grow will fail.
    assert_eq!(
        memory
            .grow_async(&mut store, 1)
            .await
            .unwrap_err()
            .to_string(),
        "failed to grow memory by `1`"
    );

    assert_eq!(store.data().memory_current, 10 * 64 * 1024);
    assert_eq!(store.data().memory_desired, 11 * 64 * 1024);
    assert_eq!(
        store.data().memory_error.as_ref().unwrap(),
        "Memory maximum size exceeded"
    );

    let table = instance.get_table(&mut store, "t").unwrap();
    // Grow the table 10 elements
    table.grow_async(&mut store, 10, Ref::Func(None)).await?;

    assert!(store.data().table_error.is_none());
    assert_eq!(store.data().table_current, 0);
    assert_eq!(store.data().table_desired, 10);

    // Grow past the static limit set by ModuleLimits.
    // The ResourceLimiter will permit this, but the grow will fail.
    assert_eq!(
        table
            .grow_async(&mut store, 1, Ref::Func(None))
            .await
            .unwrap_err()
            .to_string(),
        "failed to grow table by `1`"
    );

    assert_eq!(store.data().table_current, 10);
    assert_eq!(store.data().table_desired, 11);
    assert_eq!(
        store.data().table_error.as_ref().unwrap(),
        "Table maximum size exceeded"
    );

    drop(store);

    Ok(())
}

struct Panic;

impl ResourceLimiter for Panic {
    fn memory_growing(
        &mut self,
        _current: usize,
        _desired: usize,
        _maximum: Option<usize>,
    ) -> Result<bool> {
        panic!("resource limiter memory growing");
    }
    fn table_growing(
        &mut self,
        _current: u32,
        _desired: u32,
        _maximum: Option<u32>,
    ) -> Result<bool> {
        panic!("resource limiter table growing");
    }
}
#[async_trait::async_trait]
impl ResourceLimiterAsync for Panic {
    async fn memory_growing(
        &mut self,
        _current: usize,
        _desired: usize,
        _maximum: Option<usize>,
    ) -> Result<bool> {
        panic!("async resource limiter memory growing");
    }
    async fn table_growing(
        &mut self,
        _current: u32,
        _desired: u32,
        _maximum: Option<u32>,
    ) -> Result<bool> {
        panic!("async resource limiter table growing");
    }
}

#[test]
#[should_panic(expected = "resource limiter memory growing")]
fn panic_in_memory_limiter() {
    let engine = Engine::default();
    let linker = Linker::new(&engine);

    let module = Module::new(&engine, r#"(module (memory (export "m") 0))"#).unwrap();

    let mut store = Store::new(&engine, Panic);
    store.limiter(|s| s as &mut dyn ResourceLimiter);
    let instance = linker.instantiate(&mut store, &module).unwrap();
    let memory = instance.get_memory(&mut store, "m").unwrap();

    // Grow the memory, which should panic
    memory.grow(&mut store, 3).unwrap();
}

#[test]
#[should_panic(expected = "resource limiter memory growing")]
#[cfg_attr(miri, ignore)]
fn panic_in_memory_limiter_wasm_stack() {
    // Like the test above, except the memory.grow happens in wasm code
    // instead of a host function call.
    let engine = Engine::default();
    let linker = Linker::new(&engine);

    let module = Module::new(
        &engine,
        r#"
    (module
      (memory $m (export "m") 0)
      (func (export "grow") (param i32) (result i32)
        (memory.grow $m (local.get 0)))
    )"#,
    )
    .unwrap();

    let mut store = Store::new(&engine, Panic);
    store.limiter(|s| s as &mut dyn ResourceLimiter);
    let instance = linker.instantiate(&mut store, &module).unwrap();
    let grow = instance.get_func(&mut store, "grow").unwrap();
    let grow = grow.typed::<i32, i32>(&store).unwrap();

    // Grow the memory, which should panic
    grow.call(&mut store, 3).unwrap();
}

#[test]
#[should_panic(expected = "resource limiter table growing")]
fn panic_in_table_limiter() {
    let engine = Engine::default();
    let linker = Linker::new(&engine);

    let module = Module::new(&engine, r#"(module (table (export "t") 0 funcref))"#).unwrap();

    let mut store = Store::new(&engine, Panic);
    store.limiter(|s| s as &mut dyn ResourceLimiter);
    let instance = linker.instantiate(&mut store, &module).unwrap();
    let table = instance.get_table(&mut store, "t").unwrap();

    // Grow the table, which should panic
    table.grow(&mut store, 3, Ref::Func(None)).unwrap();
}

#[tokio::test]
#[should_panic(expected = "async resource limiter memory growing")]
#[cfg_attr(miri, ignore)]
async fn panic_in_async_memory_limiter() {
    let mut config = Config::new();
    config.async_support(true);
    let engine = Engine::new(&config).unwrap();
    let linker = Linker::new(&engine);

    let module = Module::new(&engine, r#"(module (memory (export "m") 0))"#).unwrap();

    let mut store = Store::new(&engine, Panic);
    store.limiter_async(|s| s as &mut dyn ResourceLimiterAsync);
    let instance = linker.instantiate_async(&mut store, &module).await.unwrap();
    let memory = instance.get_memory(&mut store, "m").unwrap();

    // Grow the memory, which should panic
    memory.grow_async(&mut store, 3).await.unwrap();
}

#[tokio::test]
#[should_panic(expected = "async resource limiter memory growing")]
#[cfg_attr(miri, ignore)]
async fn panic_in_async_memory_limiter_wasm_stack() {
    // Like the test above, except the memory.grow happens in
    // wasm code instead of a host function call.
    let mut config = Config::new();
    config.async_support(true);
    let engine = Engine::new(&config).unwrap();
    let linker = Linker::new(&engine);

    let module = Module::new(
        &engine,
        r#"
    (module
      (memory $m (export "m") 0)
      (func (export "grow") (param i32) (result i32)
        (memory.grow $m (local.get 0)))
    )"#,
    )
    .unwrap();

    let mut store = Store::new(&engine, Panic);
    store.limiter_async(|s| s as &mut dyn ResourceLimiterAsync);
    let instance = linker.instantiate_async(&mut store, &module).await.unwrap();
    let grow = instance.get_func(&mut store, "grow").unwrap();
    let grow = grow.typed::<i32, i32>(&store).unwrap();

    // Grow the memory, which should panic
    grow.call_async(&mut store, 3).await.unwrap();
}

#[tokio::test]
#[should_panic(expected = "async resource limiter table growing")]
#[cfg_attr(miri, ignore)]
async fn panic_in_async_table_limiter() {
    let mut config = Config::new();
    config.async_support(true);
    let engine = Engine::new(&config).unwrap();
    let linker = Linker::new(&engine);

    let module = Module::new(&engine, r#"(module (table (export "t") 0 funcref))"#).unwrap();

    let mut store = Store::new(&engine, Panic);
    store.limiter_async(|s| s as &mut dyn ResourceLimiterAsync);
    let instance = linker.instantiate_async(&mut store, &module).await.unwrap();
    let table = instance.get_table(&mut store, "t").unwrap();

    // Grow the table, which should panic
    table
        .grow_async(&mut store, 3, Ref::Func(None))
        .await
        .unwrap();
}

#[test]
#[cfg_attr(miri, ignore)]
fn growth_trap() -> Result<()> {
    let engine = Engine::default();
    let module = Module::new(
        &engine,
        r#"(module
            (memory $m (export "m") 0)
            (table (export "t") 0 funcref)
            (func (export "grow") (param i32) (result i32)
              (memory.grow $m (local.get 0)))
           )"#,
    )?;

    let mut store = Store::new(
        &engine,
        StoreLimitsBuilder::new()
            .memory_size(WASM_PAGE_SIZE)
            .table_elements(1)
            .trap_on_grow_failure(true)
            .build(),
    );
    store.limiter(|s| s as &mut dyn ResourceLimiter);

    let instance = Instance::new(&mut store, &module, &[])?;

    // Test instance exports and host objects hitting the limit
    for memory in [
        instance.get_memory(&mut store, "m").unwrap(),
        Memory::new(&mut store, MemoryType::new(0, None))?,
    ] {
        memory.grow(&mut store, 1)?;
        assert!(memory.grow(&mut store, 1).is_err());
    }

    // Test instance exports and host objects hitting the limit
    for table in [
        instance.get_table(&mut store, "t").unwrap(),
        Table::new(
            &mut store,
            TableType::new(RefType::FUNCREF, 0, None),
            Ref::Func(None),
        )?,
    ] {
        table.grow(&mut store, 1, Ref::Func(None))?;
        assert!(table.grow(&mut store, 1, Ref::Func(None)).is_err());
    }

    let mut store = Store::new(&engine, store.data().clone());
    store.limiter(|s| s as &mut dyn ResourceLimiter);
    let instance = Instance::new(&mut store, &module, &[])?;
    let grow = instance.get_func(&mut store, "grow").unwrap();
    let grow = grow.typed::<i32, i32>(&store).unwrap();
    grow.call(&mut store, 1)?;
    assert!(grow.call(&mut store, 1).is_err());

    Ok(())
}
