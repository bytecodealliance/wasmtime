use anyhow::{Context, bail};
use libtest_mimic::{Arguments, FormatSetting, Trial};
use std::sync::{Condvar, LazyLock, Mutex};
use wasmtime::{
    Config, Engine, InstanceAllocationStrategy, MpkEnabled, PoolingAllocationConfig, Store,
};
use wasmtime_test_util::wast::{Collector, Compiler, WastConfig, WastTest, limits};
use wasmtime_wast::{Async, SpectestConfig, WastContext};

fn main() {
    env_logger::init();

    let tests = if cfg!(miri) {
        Vec::new()
    } else {
        wasmtime_test_util::wast::find_tests(".".as_ref()).unwrap()
    };

    let mut trials = Vec::new();

    let mut add_trial = |test: &WastTest, config: WastConfig| {
        let trial = Trial::test(
            format!(
                "{:?}/{}{}{}",
                config.compiler,
                if config.pooling { "pooling/" } else { "" },
                if config.collector != Collector::Auto {
                    format!("{:?}/", config.collector)
                } else {
                    String::new()
                },
                test.path.to_str().unwrap()
            ),
            {
                let test = test.clone();
                move || run_wast(&test, config).map_err(|e| format!("{e:?}").into())
            },
        );

        trials.push(trial);
    };

    // List of supported compilers, filtered by what our current host supports.
    let mut compilers = vec![
        Compiler::CraneliftNative,
        Compiler::Winch,
        Compiler::CraneliftPulley,
    ];
    compilers.retain(|c| c.supports_host());

    // Run each wast test in a few interesting configuration combinations, but
    // leave the full combinatorial matrix and such to fuzz testing which
    // configures many more settings than those configured here.
    for test in tests {
        let collector = if test.test_uses_gc_types() {
            Collector::DeferredReferenceCounting
        } else {
            Collector::Auto
        };

        // Run this test in all supported compilers.
        for compiler in compilers.iter().copied() {
            add_trial(
                &test,
                WastConfig {
                    compiler,
                    pooling: false,
                    collector,
                },
            );
        }

        let compiler = compilers[0];

        // Run this test with the pooling allocator under the default compiler.
        add_trial(
            &test,
            WastConfig {
                compiler,
                pooling: true,
                collector,
            },
        );

        // If applicable, also run with the null collector in addition to the
        // default collector.
        if test.test_uses_gc_types() {
            add_trial(
                &test,
                WastConfig {
                    compiler,
                    pooling: false,
                    collector: Collector::Null,
                },
            );
        }
    }

    // There's a lot of tests so print only a `.` to keep the output a
    // bit more terse by default.
    let mut args = Arguments::from_args();
    if args.format.is_none() {
        args.format = Some(FormatSetting::Terse);
    }
    libtest_mimic::run(&args, trials).exit()
}

// Each of the tests included from `wast_testsuite_tests` will call this
// function which actually executes the `wast` test suite given the `strategy`
// to compile it.
fn run_wast(test: &WastTest, config: WastConfig) -> anyhow::Result<()> {
    let test_config = test.config.clone();

    if test.ignore(&config) {
        return Ok(());
    }

    // Determine whether this test is expected to fail or pass. Regardless the
    // test is executed and the result of the execution is asserted to match
    // this expectation. Note that this means that the test can't, for example,
    // panic or segfault as a result.
    //
    // Updates to whether a test should pass or fail should be done in the
    // `crates/wast-util/src/lib.rs` file.
    let should_fail = test.should_fail(&config);

    let multi_memory = test_config.multi_memory();
    let test_hogs_memory = test_config.hogs_memory();
    let relaxed_simd = test_config.relaxed_simd();

    let is_cranelift = match config.compiler {
        Compiler::CraneliftNative | Compiler::CraneliftPulley => true,
        _ => false,
    };

    let mut cfg = Config::new();
    cfg.async_support(true);
    wasmtime_test_util::wasmtime_wast::apply_test_config(&mut cfg, &test_config);
    wasmtime_test_util::wasmtime_wast::apply_wast_config(&mut cfg, &config);

    if is_cranelift {
        cfg.cranelift_debug_verifier(true);
    }

    // By default we'll allocate huge chunks (6gb) of the address space for each
    // linear memory. This is typically fine but when we emulate tests with QEMU
    // it turns out that it causes memory usage to balloon massively. Leave a
    // knob here so on CI we can cut down the memory usage of QEMU and avoid the
    // OOM killer.
    //
    // Locally testing this out this drops QEMU's memory usage running this
    // tests suite from 10GiB to 600MiB. Previously we saw that crossing the
    // 10GiB threshold caused our processes to get OOM killed on CI.
    //
    // Note that this branch is also taken for 32-bit platforms which generally
    // can't test much of the pooling allocator as the virtual address space is
    // so limited.
    if cfg!(target_pointer_width = "32") || std::env::var("WASMTIME_TEST_NO_HOG_MEMORY").is_ok() {
        // The pooling allocator hogs ~6TB of virtual address space for each
        // store, so if we don't to hog memory then ignore pooling tests.
        if config.pooling {
            return Ok(());
        }

        // If the test allocates a lot of memory, that's considered "hogging"
        // memory, so skip it.
        if test_hogs_memory {
            return Ok(());
        }

        // Don't use 4gb address space reservations when not hogging memory, and
        // also don't reserve lots of memory after dynamic memories for growth
        // (makes growth slower).
        cfg.memory_reservation(2 * u64::from(wasmtime_environ::Memory::DEFAULT_PAGE_SIZE));
        cfg.memory_reservation_for_growth(0);

        let small_guard = 64 * 1024;
        cfg.memory_guard_size(small_guard);
    }

    let _pooling_lock = if config.pooling {
        // Some memory64 tests take more than 4gb of resident memory to test,
        // but we don't want to configure the pooling allocator to allow that
        // (that's a ton of memory to reserve), so we skip those tests.
        if test_hogs_memory {
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
        let max_memory_size = limits::MEMORY_SIZE;
        if multi_memory {
            cfg.memory_reservation(max_memory_size as u64);
            cfg.memory_reservation_for_growth(0);
            cfg.memory_guard_size(0);
        }

        let mut pool = PoolingAllocationConfig::default();
        pool.total_memories(limits::MEMORIES * 2)
            .max_memory_protection_keys(2)
            .max_memory_size(max_memory_size)
            .max_memories_per_module(if multi_memory {
                limits::MEMORIES_PER_MODULE
            } else {
                1
            })
            .max_tables_per_module(limits::TABLES_PER_MODULE);

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

    let mut engines = vec![(Engine::new(&cfg), "default")];

    // For tests that use relaxed-simd test both the default engine and the
    // guaranteed-deterministic engine to ensure that both the 'native'
    // semantics of the instructions plus the canonical semantics work.
    if relaxed_simd {
        engines.push((
            Engine::new(cfg.relaxed_simd_deterministic(true)),
            "deterministic",
        ));
    }

    for (engine, desc) in engines {
        let result = engine.and_then(|engine| {
            let store = Store::new(&engine, ());
            let mut wast_context = WastContext::new(store, Async::Yes);
            wast_context.generate_dwarf(true);
            wast_context.register_spectest(&SpectestConfig {
                use_shared_memory: true,
                suppress_prints: true,
            })?;
            wast_context
                .run_buffer(test.path.to_str().unwrap(), test.contents.as_bytes())
                .with_context(|| format!("failed to run spec test with {desc} engine"))
        });

        if should_fail {
            if result.is_ok() {
                bail!("this test is flagged as should-fail but it succeeded")
            }
        } else {
            result?;
        }
    }

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
    const MAX_CONCURRENT_POOLING: u32 = 4;

    static ACTIVE: LazyLock<MyState> = LazyLock::new(MyState::default);

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
