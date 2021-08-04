use std::path::Path;
use std::sync::{Condvar, Mutex};
use wasmtime::{
    Config, Engine, InstanceAllocationStrategy, InstanceLimits, ModuleLimits,
    PoolingAllocationStrategy, Store, Strategy,
};
use wasmtime_wast::WastContext;

include!(concat!(env!("OUT_DIR"), "/wast_testsuite_tests.rs"));

// Each of the tests included from `wast_testsuite_tests` will call this
// function which actually executes the `wast` test suite given the `strategy`
// to compile it.
fn run_wast(wast: &str, strategy: Strategy, pooling: bool) -> anyhow::Result<()> {
    let wast = Path::new(wast);

    let simd = wast.iter().any(|s| s == "simd");

    let multi_memory = wast.iter().any(|s| s == "multi-memory");
    let module_linking = wast.iter().any(|s| s == "module-linking");
    let threads = wast.iter().any(|s| s == "threads");
    let bulk_mem = multi_memory || wast.iter().any(|s| s == "bulk-memory-operations");

    // Some simd tests assume support for multiple tables, which are introduced
    // by reference types.
    let reftypes = simd || wast.iter().any(|s| s == "reference-types");

    // Threads aren't implemented in the old backend, so skip those tests.
    if threads && cfg!(feature = "old-x86-backend") {
        return Ok(());
    }

    let mut cfg = Config::new();
    cfg.wasm_simd(simd)
        .wasm_bulk_memory(bulk_mem)
        .wasm_reference_types(reftypes || module_linking)
        .wasm_multi_memory(multi_memory || module_linking)
        .wasm_module_linking(module_linking)
        .wasm_threads(threads)
        .strategy(strategy)?
        .cranelift_debug_verifier(true);

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
        cfg.static_memory_maximum_size(0);
    }

    let _pooling_lock = if pooling {
        // The limits here are crafted such that the wast tests should pass.
        // However, these limits may become insufficient in the future as the wast tests change.
        // If a wast test fails because of a limit being "exceeded" or if memory/table
        // fails to grow, the values here will need to be adjusted.
        cfg.allocation_strategy(InstanceAllocationStrategy::Pooling {
            strategy: PoolingAllocationStrategy::NextAvailable,
            module_limits: ModuleLimits {
                imported_memories: 2,
                imported_tables: 2,
                imported_globals: 11,
                memories: 2,
                tables: 4,
                globals: 11,
                memory_pages: 805,
                ..Default::default()
            },
            instance_limits: InstanceLimits {
                count: 450,
                ..Default::default()
            },
        });
        Some(lock_pooling())
    } else {
        None
    };

    let store = Store::new(&Engine::new(&cfg)?, ());
    let mut wast_context = WastContext::new(store);
    wast_context.register_spectest()?;
    wast_context.run_file(wast)?;
    Ok(())
}

// The pooling tests make about 6TB of address space reservation which means
// that we shouldn't let too many of them run concurrently at once. On
// high-cpu-count systems (e.g. 80 threads) this leads to mmap failures because
// presumably too much of the address space has been reserved with our limits
// specified above. By keeping the number of active pooling-related tests to a
// specified maximum we can put a cap on the virtual address space reservations
// made.
fn lock_pooling() -> impl Drop {
    const MAX_CONCURRENT_POOLING: u32 = 8;

    lazy_static::lazy_static! {
        static ref ACTIVE: MyState = MyState::default();
    }

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
