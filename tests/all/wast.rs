use anyhow::Context;
use bstr::ByteSlice;
use once_cell::sync::Lazy;
use std::path::Path;
use std::sync::{Condvar, Mutex};
use wasmtime::{
    Config, Engine, InstanceAllocationStrategy, PoolingAllocationConfig, Store, Strategy,
};
use wasmtime_environ::WASM_PAGE_SIZE;
use wasmtime_runtime::MpkEnabled;
use wasmtime_wast::{SpectestConfig, WastContext};

include!(concat!(env!("OUT_DIR"), "/wast_testsuite_tests.rs"));

// Each of the tests included from `wast_testsuite_tests` will call this
// function which actually executes the `wast` test suite given the `strategy`
// to compile it.
fn run_wast(wast: &str, strategy: Strategy, pooling: bool) -> anyhow::Result<()> {
    drop(env_logger::try_init());

    let wast_bytes = std::fs::read(wast).with_context(|| format!("failed to read `{}`", wast))?;

    let wast = Path::new(wast);

    let memory64 = feature_found(wast, "memory64");
    let multi_memory = feature_found(wast, "multi-memory");
    let threads = feature_found(wast, "threads");
    let gc = feature_found(wast, "gc");
    let function_references = gc || feature_found(wast, "function-references");
    let reference_types = !(threads && feature_found(wast, "proposals"));
    let relaxed_simd = feature_found(wast, "relaxed-simd");
    let tail_call = feature_found(wast, "tail-call") || feature_found(wast, "function-references");
    let use_shared_memory = feature_found_src(&wast_bytes, "shared_memory")
        || feature_found_src(&wast_bytes, "shared)");

    if pooling && use_shared_memory {
        eprintln!("skipping pooling test with shared memory");
        return Ok(());
    }

    let is_cranelift = match strategy {
        Strategy::Cranelift => true,
        _ => false,
    };

    let mut cfg = Config::new();
    cfg.wasm_multi_memory(multi_memory)
        .wasm_threads(threads)
        .wasm_memory64(memory64)
        .wasm_function_references(function_references)
        .wasm_gc(gc)
        .wasm_reference_types(reference_types)
        .wasm_relaxed_simd(relaxed_simd)
        .wasm_tail_call(tail_call)
        .strategy(strategy);

    if is_cranelift {
        cfg.cranelift_debug_verifier(true);
    }

    cfg.wasm_component_model(feature_found(wast, "component-model"));

    if feature_found(wast, "canonicalize-nan") && is_cranelift {
        cfg.cranelift_nan_canonicalization(true);
    }
    let test_allocates_lots_of_memory = wast.ends_with("more-than-4gb.wast");

    // By default we'll allocate huge chunks (6gb) of the address space for each
    // linear memory. This is typically fine but when we emulate tests with QEMU
    // it turns out that it causes memory usage to balloon massively. Leave a
    // knob here so on CI we can cut down the memory usage of QEMU and avoid the
    // OOM killer.
    //
    // Locally testing this out this drops QEMU's memory usage running this
    // tests suite from 10GiB to 600MiB. Previously we saw that crossing the
    // 10GiB threshold caused our processes to get OOM killed on CI.
    if std::env::var("WASMTIME_TEST_NO_HOG_MEMORY").is_ok() {
        // The pooling allocator hogs ~6TB of virtual address space for each
        // store, so if we don't to hog memory then ignore pooling tests.
        if pooling {
            return Ok(());
        }

        // If the test allocates a lot of memory, that's considered "hogging"
        // memory, so skip it.
        if test_allocates_lots_of_memory {
            return Ok(());
        }

        // Don't use 4gb address space reservations when not hogging memory, and
        // also don't reserve lots of memory after dynamic memories for growth
        // (makes growth slower).
        if use_shared_memory {
            cfg.static_memory_maximum_size(2 * WASM_PAGE_SIZE as u64);
        } else {
            cfg.static_memory_maximum_size(0);
        }
        cfg.dynamic_memory_reserved_for_growth(0);
        cfg.static_memory_guard_size(0);
        cfg.dynamic_memory_guard_size(0);
    }

    let _pooling_lock = if pooling {
        // Some memory64 tests take more than 4gb of resident memory to test,
        // but we don't want to configure the pooling allocator to allow that
        // (that's a ton of memory to reserve), so we skip those tests.
        if test_allocates_lots_of_memory {
            return Ok(());
        }

        // Reduce the virtual memory required to run multi-memory-based tests.
        //
        // The configuration parameters below require that a bare minimum
        // virtual address space reservation of 450*9*805*65536 == 200G be made
        // to support each test. If 6G reservations are made for each linear
        // memory then not that many tests can run concurrently with much else.
        //
        // When multiple memories are used and are configured in the pool then
        // force the usage of static memories without guards to reduce the VM
        // impact.
        if multi_memory {
            cfg.static_memory_maximum_size(0);
            cfg.dynamic_memory_reserved_for_growth(0);
            cfg.static_memory_guard_size(0);
            cfg.dynamic_memory_guard_size(0);
        }

        // The limits here are crafted such that the wast tests should pass.
        // However, these limits may become insufficient in the future as the
        // wast tests change. If a wast test fails because of a limit being
        // "exceeded" or if memory/table fails to grow, the values here will
        // need to be adjusted.
        let mut pool = PoolingAllocationConfig::default();
        pool.total_memories(450 * 2)
            .max_memory_protection_keys(2)
            .memory_pages(805)
            .max_memories_per_module(if multi_memory { 9 } else { 1 })
            .max_tables_per_module(5);

        // When testing, we may choose to start with MPK force-enabled to ensure
        // we use that functionality.
        if std::env::var("WASMTIME_TEST_FORCE_MPK").is_ok() {
            pool.memory_protection_keys(MpkEnabled::Enable);
        }

        cfg.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));
        Some(lock_pooling())
    } else {
        None
    };

    let mut engines = vec![(Engine::new(&cfg)?, "default")];

    // For tests that use relaxed-simd test both the default engine and the
    // guaranteed-deterministic engine to ensure that both the 'native'
    // semantics of the instructions plus the canonical semantics work.
    if relaxed_simd {
        engines.push((
            Engine::new(cfg.relaxed_simd_deterministic(true))?,
            "deterministic",
        ));
    }

    for (engine, desc) in engines {
        let store = Store::new(&engine, ());
        let mut wast_context = WastContext::new(store);
        wast_context.register_spectest(&SpectestConfig {
            use_shared_memory,
            suppress_prints: false,
        })?;
        wast_context
            .run_buffer(wast.to_str().unwrap(), &wast_bytes)
            .with_context(|| format!("failed to run spec test with {desc} engine"))?;
    }

    Ok(())
}

fn feature_found(path: &Path, name: &str) -> bool {
    path.iter().any(|part| match part.to_str() {
        Some(s) => s.contains(name),
        None => false,
    })
}

fn feature_found_src(bytes: &[u8], name: &str) -> bool {
    bytes.contains_str(name)
}

// The pooling tests make about 6TB of address space reservation which means
// that we shouldn't let too many of them run concurrently at once. On
// high-cpu-count systems (e.g. 80 threads) this leads to mmap failures because
// presumably too much of the address space has been reserved with our limits
// specified above. By keeping the number of active pooling-related tests to a
// specified maximum we can put a cap on the virtual address space reservations
// made.
fn lock_pooling() -> impl Drop {
    const MAX_CONCURRENT_POOLING: u32 = 4;

    static ACTIVE: Lazy<MyState> = Lazy::new(MyState::default);

    #[derive(Default)]
    struct MyState {
        lock: Mutex<u32>,
        waiters: Condvar,
    }

    impl MyState {
        fn lock(&self) -> impl Drop + '_ {
            let state = self.lock.lock().unwrap();
            let mut state = self
                .waiters
                .wait_while(state, |cnt| *cnt >= MAX_CONCURRENT_POOLING)
                .unwrap();
            *state += 1;
            LockGuard { state: self }
        }
    }

    struct LockGuard<'a> {
        state: &'a MyState,
    }

    impl Drop for LockGuard<'_> {
        fn drop(&mut self) {
            *self.state.lock.lock().unwrap() -= 1;
            self.state.waiters.notify_one();
        }
    }

    ACTIVE.lock()
}
