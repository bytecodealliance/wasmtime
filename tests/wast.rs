use anyhow::Context;
use bstr::ByteSlice;
use libtest_mimic::{Arguments, FormatSetting, Trial};
use once_cell::sync::Lazy;
use std::path::Path;
use std::sync::{Condvar, Mutex};
use wasmtime::{
    Config, Engine, InstanceAllocationStrategy, MpkEnabled, PoolingAllocationConfig, Store,
    Strategy,
};
use wasmtime_environ::Memory;
use wasmtime_wast::{SpectestConfig, WastContext};

fn main() {
    env_logger::init();

    let mut trials = Vec::new();
    if !cfg!(miri) {
        add_tests(&mut trials, "tests/spec_testsuite".as_ref());
        add_tests(&mut trials, "tests/misc_testsuite".as_ref());
    }

    // There's a lot of tests so print only a `.` to keep the output a
    // bit more terse by default.
    let mut args = Arguments::from_args();
    if args.format.is_none() {
        args.format = Some(FormatSetting::Terse);
    }
    libtest_mimic::run(&args, trials).exit()
}

fn add_tests(trials: &mut Vec<Trial>, path: &Path) {
    for entry in path.read_dir().unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if entry.file_type().unwrap().is_dir() {
            add_tests(trials, &path);
            continue;
        }

        if path.extension().and_then(|s| s.to_str()) != Some("wast") {
            continue;
        }

        for strategy in [Strategy::Cranelift, Strategy::Winch] {
            for pooling in [true, false] {
                let trial = Trial::test(
                    format!(
                        "{strategy:?}/{}{}",
                        if pooling { "pooling/" } else { "" },
                        path.to_str().unwrap()
                    ),
                    {
                        let path = path.clone();
                        move || {
                            run_wast(&path, strategy, pooling).map_err(|e| format!("{e:?}").into())
                        }
                    },
                );
                trials.push(trial.with_ignored_flag(ignore(&path, strategy)));
            }
        }
    }
}

fn ignore(test: &Path, strategy: Strategy) -> bool {
    // Winch only supports x86_64 at this time.
    if strategy == Strategy::Winch && !cfg!(target_arch = "x86_64") {
        return true;
    }

    for part in test.iter() {
        // Not implemented in Wasmtime yet
        if part == "exception-handling" {
            return true;
        }
        // Not implemented in Wasmtime yet
        if part == "extended-const" {
            return true;
        }
        // Wasmtime doesn't implement the table64 extension yet.
        if part == "memory64" {
            if [
                "call_indirect.wast",
                "table_copy.wast",
                "table_get.wast",
                "table_set.wast",
                "table_fill.wast",
                "table.wast",
                "table_init.wast",
                "table_copy_mixed.wast",
                "table_grow.wast",
                "table_size.wast",
            ]
            .iter()
            .any(|i| test.ends_with(i))
            {
                return true;
            }
        }

        // TODO(#6530): These tests require tail calls, but s390x doesn't
        // support them yet.
        if cfg!(target_arch = "s390x") {
            if part == "function-references" || part == "tail-call" {
                return true;
            }
        }

        // Disable spec tests for proposals that Winch does not implement yet.
        if strategy == Strategy::Winch {
            let part = part.to_str().unwrap();
            let unsupported = [
                // wasm proposals that Winch doesn't support,
                "references",
                "tail-call",
                "gc",
                "threads",
                "multi-memory",
                "relaxed-simd",
                "function-references",
                // tests in misc_testsuite that Winch doesn't support
                "no-panic.wast",
                "externref-id-function.wast",
                "int-to-float-splat.wast",
                "issue6562.wast",
                "many_table_gets_lead_to_gc.wast",
                "mutable_externref_globals.wast",
                "no-mixup-stack-maps.wast",
                "simple_ref_is_null.wast",
                "table_grow_with_funcref.wast",
                // Tests in the spec test suite Winch doesn't support
                "threads.wast",
                "br_table.wast",
                "global.wast",
                "table_fill.wast",
                "table_get.wast",
                "table_set.wast",
                "table_grow.wast",
                "table_size.wast",
                "elem.wast",
                "select.wast",
                "unreached-invalid.wast",
                "linking.wast",
            ];

            if unsupported.contains(&part) || part.starts_with("simd") || part.starts_with("ref_") {
                return true;
            }
        }

        // Implementation of the GC proposal is a work-in-progress, this is
        // a list of all currently known-to-fail tests.
        if part == "gc" {
            return [
                "array_copy.wast",
                "array_fill.wast",
                "array_init_data.wast",
                "array_init_elem.wast",
                "array.wast",
                "binary_gc.wast",
                "binary.wast",
                "br_on_cast_fail.wast",
                "br_on_cast.wast",
                "br_on_non_null.wast",
                "br_on_null.wast",
                "br_table.wast",
                "call_ref.wast",
                "data.wast",
                "elem.wast",
                "extern.wast",
                "func.wast",
                "global.wast",
                "if.wast",
                "linking.wast",
                "local_get.wast",
                "local_init.wast",
                "ref_as_non_null.wast",
                "ref_cast.wast",
                "ref_eq.wast",
                "ref_is_null.wast",
                "ref_null.wast",
                "ref_test.wast",
                "ref.wast",
                "return_call_indirect.wast",
                "return_call_ref.wast",
                "return_call.wast",
                "select.wast",
                "struct.wast",
                "table_sub.wast",
                "table.wast",
                "type_canon.wast",
                "type_equivalence.wast",
                "type-rec.wast",
                "type-subtyping.wast",
                "unreached-invalid.wast",
                "unreached_valid.wast",
                "i31.wast",
            ]
            .iter()
            .any(|i| test.ends_with(i));
        }
    }

    false
}

// Each of the tests included from `wast_testsuite_tests` will call this
// function which actually executes the `wast` test suite given the `strategy`
// to compile it.
fn run_wast(wast: &Path, strategy: Strategy, pooling: bool) -> anyhow::Result<()> {
    let wast_bytes =
        std::fs::read(wast).with_context(|| format!("failed to read `{}`", wast.display()))?;

    let wast = Path::new(wast);

    let memory64 = feature_found(wast, "memory64");
    let custom_page_sizes = feature_found(wast, "custom-page-sizes");
    let multi_memory = feature_found(wast, "multi-memory")
        || feature_found(wast, "component-model")
        || custom_page_sizes;
    let threads = feature_found(wast, "threads");
    let gc = feature_found(wast, "gc");
    let function_references = gc || feature_found(wast, "function-references");
    let reference_types = !(threads && feature_found(wast, "proposals"));
    let relaxed_simd = feature_found(wast, "relaxed-simd");
    let tail_call = feature_found(wast, "tail-call") || feature_found(wast, "function-references");
    let use_shared_memory = feature_found_src(&wast_bytes, "shared_memory")
        || feature_found_src(&wast_bytes, "shared)");

    if pooling && use_shared_memory {
        log::warn!("skipping pooling test with shared memory");
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
        .wasm_custom_page_sizes(custom_page_sizes)
        .strategy(strategy);

    if is_cranelift {
        cfg.cranelift_debug_verifier(true);
    }

    let component_model = feature_found(wast, "component-model");
    cfg.wasm_component_model(component_model)
        .wasm_component_model_more_flags(component_model);

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
            cfg.static_memory_maximum_size(2 * u64::from(Memory::DEFAULT_PAGE_SIZE));
        } else {
            cfg.static_memory_maximum_size(0);
        }
        cfg.dynamic_memory_reserved_for_growth(0);

        let small_guard = 64 * 1024;
        cfg.static_memory_guard_size(small_guard);
        cfg.dynamic_memory_guard_size(small_guard);
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
        let max_memory_size = 805 << 16;
        if multi_memory {
            cfg.static_memory_maximum_size(max_memory_size as u64);
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
            .max_memory_size(max_memory_size)
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
            suppress_prints: true,
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
