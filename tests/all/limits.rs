use anyhow::Result;
use wasmtime::*;

const WASM_PAGE_SIZE: usize = wasmtime_environ::WASM_PAGE_SIZE as usize;

#[test]
fn test_limits() -> Result<()> {
    let engine = Engine::default();
    let module = Module::new(
        &engine,
        r#"(module (memory (export "m") 0) (table (export "t") 0 anyfunc))"#,
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
    for memory in std::array::IntoIter::new([
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
    for table in std::array::IntoIter::new([
        instance.get_table(&mut store, "t").unwrap(),
        Table::new(
            &mut store,
            TableType::new(ValType::FuncRef, 0, None),
            Val::FuncRef(None),
        )?,
    ]) {
        table.grow(&mut store, 2, Val::FuncRef(None))?;
        table.grow(&mut store, 1, Val::FuncRef(None))?;
        table.grow(&mut store, 2, Val::FuncRef(None))?;

        assert_eq!(
            table
                .grow(&mut store, 1, Val::FuncRef(None))
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
        r#"(module (memory (export "m") 0) (table (export "t") 0 anyfunc))"#,
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
    for memory in std::array::IntoIter::new([
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
    for table in std::array::IntoIter::new([
        instance.get_table(&mut store, "t").unwrap(),
        Table::new(
            &mut store,
            TableType::new(ValType::FuncRef, 0, None),
            Val::FuncRef(None),
        )?,
    ]) {
        table.grow(&mut store, 2, Val::FuncRef(None))?;
        table.grow(&mut store, 1, Val::FuncRef(None))?;
        table.grow(&mut store, 2, Val::FuncRef(None))?;
        table.grow(&mut store, 1, Val::FuncRef(None))?;
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
            "Insufficient resources: memory minimum size of 11 pages exceeds memory limits"
        ),
    }

    match Memory::new(&mut store, MemoryType::new(25, None)) {
        Ok(_) => unreachable!(),
        Err(e) => assert_eq!(
            e.to_string(),
            "Insufficient resources: memory minimum size of 25 pages exceeds memory limits"
        ),
    }

    Ok(())
}

#[test]
fn test_limits_table_only() -> Result<()> {
    let engine = Engine::default();
    let module = Module::new(
        &engine,
        r#"(module (memory (export "m") 0) (table (export "t") 0 anyfunc))"#,
    )?;

    let mut store = Store::new(&engine, StoreLimitsBuilder::new().table_elements(5).build());
    store.limiter(|s| s as &mut dyn ResourceLimiter);

    let instance = Instance::new(&mut store, &module, &[])?;

    // Test instance exports and host objects *not* hitting the limit
    for memory in std::array::IntoIter::new([
        instance.get_memory(&mut store, "m").unwrap(),
        Memory::new(&mut store, MemoryType::new(0, None))?,
    ]) {
        memory.grow(&mut store, 3)?;
        memory.grow(&mut store, 5)?;
        memory.grow(&mut store, 2)?;
        memory.grow(&mut store, 1)?;
    }

    // Test instance exports and host objects hitting the limit
    for table in std::array::IntoIter::new([
        instance.get_table(&mut store, "t").unwrap(),
        Table::new(
            &mut store,
            TableType::new(ValType::FuncRef, 0, None),
            Val::FuncRef(None),
        )?,
    ]) {
        table.grow(&mut store, 2, Val::FuncRef(None))?;
        table.grow(&mut store, 1, Val::FuncRef(None))?;
        table.grow(&mut store, 2, Val::FuncRef(None))?;

        assert_eq!(
            table
                .grow(&mut store, 1, Val::FuncRef(None))
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
    let module = Module::new(&engine, r#"(module (table (export "t") 23 anyfunc))"#)?;

    let mut store = Store::new(&engine, StoreLimitsBuilder::new().table_elements(4).build());
    store.limiter(|s| s as &mut dyn ResourceLimiter);

    match Instance::new(&mut store, &module, &[]) {
        Ok(_) => unreachable!(),
        Err(e) => assert_eq!(
            e.to_string(),
            "Insufficient resources: table minimum size of 23 elements exceeds table limits"
        ),
    }

    match Table::new(
        &mut store,
        TableType::new(ValType::FuncRef, 99, None),
        Val::FuncRef(None),
    ) {
        Ok(_) => unreachable!(),
        Err(e) => assert_eq!(
            e.to_string(),
            "Insufficient resources: table minimum size of 99 elements exceeds table limits"
        ),
    }

    Ok(())
}

#[test]
fn test_pooling_allocator_initial_limits_exceeded() -> Result<()> {
    let mut config = Config::new();
    config.wasm_multi_memory(true);
    config.allocation_strategy(InstanceAllocationStrategy::Pooling {
        strategy: PoolingAllocationStrategy::NextAvailable,
        module_limits: ModuleLimits {
            memories: 2,
            ..Default::default()
        },
        instance_limits: InstanceLimits {
            count: 1,
            ..Default::default()
        },
    });

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
            "Insufficient resources: memory minimum size of 5 pages exceeds memory limits"
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
    fn memory_growing(&mut self, current: usize, desired: usize, maximum: Option<usize>) -> bool {
        // Check if the desired exceeds a maximum (either from Wasm or from the host)
        assert!(desired < maximum.unwrap_or(usize::MAX));

        assert_eq!(current as usize, self.wasm_memory_used);
        let desired = desired as usize;

        if desired + self.host_memory_used > self.memory_limit {
            self.limit_exceeded = true;
            return false;
        }

        self.wasm_memory_used = desired;
        true
    }

    fn table_growing(&mut self, _current: u32, _desired: u32, _maximum: Option<u32>) -> bool {
        true
    }
}

#[test]
fn test_custom_limiter() -> Result<()> {
    let engine = Engine::default();
    let mut linker = Linker::new(&engine);

    // This approximates a function that would "allocate" resources that the host tracks.
    // Here this is a simple function that increments the current host memory "used".
    linker.func_wrap(
        "",
        "alloc",
        |mut caller: Caller<'_, MemoryContext>, size: u32| -> u32 {
            let mut ctx = caller.data_mut();
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
    let f = instance.get_typed_func::<u32, u32, _>(&mut store, "f")?;

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

#[derive(Default)]
struct MemoryGrowFailureDetector {
    /// Arguments of most recent call to memory_growing
    current: usize,
    desired: usize,
    /// Display impl of most recent call to memory_grow_failed
    error: Option<String>,
}

impl ResourceLimiter for MemoryGrowFailureDetector {
    fn memory_growing(&mut self, current: usize, desired: usize, _maximum: Option<usize>) -> bool {
        self.current = current;
        self.desired = desired;
        true
    }

    fn memory_grow_failed(&mut self, err: &anyhow::Error) {
        self.error = Some(err.to_string());
    }

    fn table_growing(&mut self, _current: u32, _desired: u32, _maximum: Option<u32>) -> bool {
        true
    }
}

#[test]
fn custom_limiter_detect_grow_failure() -> Result<()> {
    if std::env::var("WASMTIME_TEST_NO_HOG_MEMORY").is_ok() {
        return Ok(());
    }
    let mut config = Config::new();
    config.allocation_strategy(InstanceAllocationStrategy::Pooling {
        strategy: PoolingAllocationStrategy::NextAvailable,
        module_limits: ModuleLimits {
            memory_pages: 10,
            ..Default::default()
        },
        instance_limits: InstanceLimits {
            ..Default::default()
        },
    });
    let engine = Engine::new(&config).unwrap();
    let linker = Linker::new(&engine);

    let module = Module::new(&engine, r#"(module (memory (export "m") 0))"#)?;

    let context = MemoryGrowFailureDetector::default();

    let mut store = Store::new(&engine, context);
    store.limiter(|s| s as &mut dyn ResourceLimiter);
    let instance = linker.instantiate(&mut store, &module)?;
    let memory = instance.get_memory(&mut store, "m").unwrap();

    // Grow the memory by 640 KiB (10 pages)
    memory.grow(&mut store, 10)?;

    assert!(store.data().error.is_none());
    assert_eq!(store.data().current, 0);
    assert_eq!(store.data().desired, 10 * 64 * 1024);

    // Grow past the static limit set by ModuleLimits.
    // The ResourcLimiter will permit this, but the grow will fail.
    assert_eq!(
        memory.grow(&mut store, 1).unwrap_err().to_string(),
        "failed to grow memory by `1`"
    );

    assert_eq!(store.data().current, 10 * 64 * 1024);
    assert_eq!(store.data().desired, 11 * 64 * 1024);
    assert_eq!(
        store.data().error.as_ref().unwrap(),
        "Memory maximum size exceeded"
    );

    drop(store);

    Ok(())
}

// This test only works on Linux. It may be portable to MacOS as well,
// but the original author did not have a machine available to test it.
#[cfg(target_os = "linux")]
#[test]
fn custom_limiter_detect_os_oom_failure() -> Result<()> {
    if std::env::var("WASMTIME_TEST_NO_HOG_MEMORY").is_ok() {
        return Ok(());
    }

    let pid = unsafe { libc::fork() };
    if pid == 0 {
        // Child process
        let r = std::panic::catch_unwind(|| {
            // Ask Linux to limit this process to 128MiB of memory
            let process_max_memory: usize = 128 * 1024 * 1024;
            unsafe {
                // limit process to 128MiB memory
                let rlimit = libc::rlimit {
                    rlim_cur: 0,
                    rlim_max: process_max_memory as u64,
                };
                let res = libc::setrlimit(libc::RLIMIT_DATA, &rlimit);
                assert_eq!(res, 0, "setrlimit failed: {}", res);
            };

            // Default behavior of on-demand memory allocation so that a
            // memory grow will hit Linux for a larger mmap.
            let engine = Engine::default();
            let linker = Linker::new(&engine);
            let module = Module::new(&engine, r#"(module (memory (export "m") 0))"#).unwrap();

            let context = MemoryGrowFailureDetector::default();

            let mut store = Store::new(&engine, context);
            store.limiter(|s| s as &mut dyn ResourceLimiter);
            let instance = linker.instantiate(&mut store, &module).unwrap();
            let memory = instance.get_memory(&mut store, "m").unwrap();

            // Small (640KiB) grow should succeed
            memory.grow(&mut store, 10).unwrap();
            assert!(store.data().error.is_none());
            assert_eq!(store.data().current, 0);
            assert_eq!(store.data().desired, 10 * 64 * 1024);

            // Try to grow past the process's memory limit.
            // This should fail.
            let pages_exceeding_limit = process_max_memory / (64 * 1024);
            let err_msg = memory
                .grow(&mut store, pages_exceeding_limit as u64)
                .unwrap_err()
                .to_string();
            assert!(
                err_msg.starts_with("failed to grow memory"),
                "unexpected error: {}",
                err_msg
            );

            assert_eq!(store.data().current, 10 * 64 * 1024);
            assert_eq!(
                store.data().desired,
                (pages_exceeding_limit + 10) * 64 * 1024
            );
            // The memory_grow_failed hook should show Linux gave OOM:
            let err_msg = store.data().error.as_ref().unwrap();
            assert!(
                err_msg.starts_with("System call failed: Cannot allocate memory"),
                "unexpected error: {}",
                err_msg
            );
        });
        // on assertion failure, exit 1 so parent process can fail test.
        std::process::exit(if r.is_err() { 1 } else { 0 });
    } else {
        // Parent process
        let mut wstatus: libc::c_int = 0;
        unsafe { libc::wait(&mut wstatus) };
        if libc::WIFEXITED(wstatus) {
            if libc::WEXITSTATUS(wstatus) == 0 {
                Ok(())
            } else {
                anyhow::bail!("child exited with failure");
            }
        } else {
            anyhow::bail!("child didnt exit??")
        }
    }
}
