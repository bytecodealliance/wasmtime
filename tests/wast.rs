use anyhow::{bail, Context};
use libtest_mimic::{Arguments, FormatSetting, Trial};
use serde_derive::Deserialize;
use std::path::Path;
use std::sync::{Condvar, LazyLock, Mutex};
use wasmtime::{
    Collector, Config, Engine, InstanceAllocationStrategy, MpkEnabled, PoolingAllocationConfig,
    Store, Strategy,
};
use wasmtime_environ::Memory;
use wasmtime_wast::{SpectestConfig, WastContext};

mod support;

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

        let test_uses_gc_types = path.iter().any(|part| {
            part.to_str().map_or(false, |s| {
                s.contains("gc")
                    || s.contains("function-references")
                    || s.contains("reference-types")
                    || s.contains("exception-handling")
            })
        });

        for strategy in [Strategy::Cranelift, Strategy::Winch] {
            for pooling in [true, false] {
                let collectors: &[_] = if !pooling && test_uses_gc_types {
                    &[Collector::DeferredReferenceCounting, Collector::Null]
                } else {
                    &[Collector::Auto]
                };

                for collector in collectors.iter().copied() {
                    let trial = Trial::test(
                        format!(
                            "{strategy:?}/{}{}{}",
                            if pooling { "pooling/" } else { "" },
                            if collector != Collector::Auto {
                                format!("{collector:?}/")
                            } else {
                                String::new()
                            },
                            path.to_str().unwrap()
                        ),
                        {
                            let path = path.clone();
                            move || {
                                run_wast(
                                    &path,
                                    WastConfig {
                                        strategy,
                                        pooling,
                                        collector,
                                    },
                                )
                                .map_err(|e| format!("{e:?}").into())
                            }
                        },
                    );
                    trials.push(trial);
                }
            }
        }
    }
}

fn should_fail(test: &Path, wast_config: &WastConfig, test_config: &TestConfig) -> bool {
    // Winch only supports x86_64 at this time.
    if wast_config.strategy == Strategy::Winch && !cfg!(target_arch = "x86_64") {
        return true;
    }

    // Disable spec tests for proposals that Winch does not implement yet.
    if wast_config.strategy == Strategy::Winch {
        // A few proposals that winch has no support for.
        if test_config.gc == Some(true)
            || test_config.threads == Some(true)
            || test_config.tail_call == Some(true)
            || test_config.function_references == Some(true)
            || test_config.gc == Some(true)
            || test_config.relaxed_simd == Some(true)
        {
            return true;
        }

        let unsupported = [
            // externref/reference-types related
            "component-model/modules.wast",
            "extended-const/elem.wast",
            "extended-const/global.wast",
            "memory64/threads.wast",
            "misc_testsuite/externref-id-function.wast",
            "misc_testsuite/externref-segment.wast",
            "misc_testsuite/externref-segments.wast",
            "misc_testsuite/externref-table-dropped-segment-issue-8281.wast",
            "misc_testsuite/linking-errors.wast",
            "misc_testsuite/many_table_gets_lead_to_gc.wast",
            "misc_testsuite/mutable_externref_globals.wast",
            "misc_testsuite/no-mixup-stack-maps.wast",
            "misc_testsuite/no-panic.wast",
            "misc_testsuite/simple_ref_is_null.wast",
            "misc_testsuite/table_grow_with_funcref.wast",
            "spec_testsuite/br_table.wast",
            "spec_testsuite/data-invalid.wast",
            "spec_testsuite/elem.wast",
            "spec_testsuite/global.wast",
            "spec_testsuite/linking.wast",
            "spec_testsuite/ref_func.wast",
            "spec_testsuite/ref_is_null.wast",
            "spec_testsuite/ref_null.wast",
            "spec_testsuite/select.wast",
            "spec_testsuite/table-sub.wast",
            "spec_testsuite/table_fill.wast",
            "spec_testsuite/table_get.wast",
            "spec_testsuite/table_grow.wast",
            "spec_testsuite/table_set.wast",
            "spec_testsuite/table_size.wast",
            "spec_testsuite/unreached-invalid.wast",
            "spec_testsuite/call_indirect.wast",
            // simd-related failures
            "annotations/simd_lane.wast",
            "memory64/simd.wast",
            "misc_testsuite/int-to-float-splat.wast",
            "misc_testsuite/issue6562.wast",
            "misc_testsuite/simd/almost-extmul.wast",
            "misc_testsuite/simd/canonicalize-nan.wast",
            "misc_testsuite/simd/cvt-from-uint.wast",
            "misc_testsuite/simd/issue4807.wast",
            "misc_testsuite/simd/issue6725-no-egraph-panic.wast",
            "misc_testsuite/simd/issue_3327_bnot_lowering.wast",
            "misc_testsuite/simd/load_splat_out_of_bounds.wast",
            "misc_testsuite/simd/replace-lane-preserve.wast",
            "misc_testsuite/simd/spillslot-size-fuzzbug.wast",
            "misc_testsuite/simd/unaligned-load.wast",
            "multi-memory/simd_memory-multi.wast",
            "spec_testsuite/simd_align.wast",
            "spec_testsuite/simd_bit_shift.wast",
            "spec_testsuite/simd_bitwise.wast",
            "spec_testsuite/simd_boolean.wast",
            "spec_testsuite/simd_const.wast",
            "spec_testsuite/simd_conversions.wast",
            "spec_testsuite/simd_f32x4.wast",
            "spec_testsuite/simd_f32x4_arith.wast",
            "spec_testsuite/simd_f32x4_cmp.wast",
            "spec_testsuite/simd_f32x4_pmin_pmax.wast",
            "spec_testsuite/simd_f32x4_rounding.wast",
            "spec_testsuite/simd_f64x2.wast",
            "spec_testsuite/simd_f64x2_arith.wast",
            "spec_testsuite/simd_f64x2_cmp.wast",
            "spec_testsuite/simd_f64x2_pmin_pmax.wast",
            "spec_testsuite/simd_f64x2_rounding.wast",
            "spec_testsuite/simd_i16x8_arith.wast",
            "spec_testsuite/simd_i16x8_arith2.wast",
            "spec_testsuite/simd_i16x8_cmp.wast",
            "spec_testsuite/simd_i16x8_extadd_pairwise_i8x16.wast",
            "spec_testsuite/simd_i16x8_extmul_i8x16.wast",
            "spec_testsuite/simd_i16x8_q15mulr_sat_s.wast",
            "spec_testsuite/simd_i16x8_sat_arith.wast",
            "spec_testsuite/simd_i32x4_arith.wast",
            "spec_testsuite/simd_i32x4_arith2.wast",
            "spec_testsuite/simd_i32x4_cmp.wast",
            "spec_testsuite/simd_i32x4_dot_i16x8.wast",
            "spec_testsuite/simd_i32x4_extadd_pairwise_i16x8.wast",
            "spec_testsuite/simd_i32x4_extmul_i16x8.wast",
            "spec_testsuite/simd_i32x4_trunc_sat_f32x4.wast",
            "spec_testsuite/simd_i32x4_trunc_sat_f64x2.wast",
            "spec_testsuite/simd_i64x2_arith.wast",
            "spec_testsuite/simd_i64x2_arith2.wast",
            "spec_testsuite/simd_i64x2_cmp.wast",
            "spec_testsuite/simd_i64x2_extmul_i32x4.wast",
            "spec_testsuite/simd_i8x16_arith.wast",
            "spec_testsuite/simd_i8x16_arith2.wast",
            "spec_testsuite/simd_i8x16_cmp.wast",
            "spec_testsuite/simd_i8x16_sat_arith.wast",
            "spec_testsuite/simd_int_to_int_extend.wast",
            "spec_testsuite/simd_lane.wast",
            "spec_testsuite/simd_load.wast",
            "spec_testsuite/simd_load16_lane.wast",
            "spec_testsuite/simd_load32_lane.wast",
            "spec_testsuite/simd_load64_lane.wast",
            "spec_testsuite/simd_load8_lane.wast",
            "spec_testsuite/simd_load_extend.wast",
            "spec_testsuite/simd_load_splat.wast",
            "spec_testsuite/simd_load_zero.wast",
            "spec_testsuite/simd_splat.wast",
            "spec_testsuite/simd_store16_lane.wast",
            "spec_testsuite/simd_store32_lane.wast",
            "spec_testsuite/simd_store64_lane.wast",
            "spec_testsuite/simd_store8_lane.wast",
        ];

        if unsupported.iter().any(|part| test.ends_with(part)) {
            return true;
        }
    }

    for part in test.iter() {
        // Not implemented in Wasmtime yet
        if part == "exception-handling" {
            return !test.ends_with("binary.wast");
        }

        if part == "memory64" {
            if [
                // wasmtime doesn't implement exceptions yet
                "imports.wast",
                "ref_null.wast",
                "exports.wast",
                "throw.wast",
                "throw_ref.wast",
                "try_table.wast",
                "tag.wast",
                "instance.wast",
            ]
            .iter()
            .any(|i| test.ends_with(i))
            {
                return true;
            }
        }
    }

    // Some tests are known to fail with the pooling allocator
    if wast_config.pooling {
        let unsupported = [
            // allocates too much memory for the pooling configuration here
            "misc_testsuite/memory64/more-than-4gb.wast",
            // shared memories + pooling allocator aren't supported yet
            "misc_testsuite/memory-combos.wast",
            "misc_testsuite/threads/LB.wast",
            "misc_testsuite/threads/LB_atomic.wast",
            "misc_testsuite/threads/MP.wast",
            "misc_testsuite/threads/MP_atomic.wast",
            "misc_testsuite/threads/MP_wait.wast",
            "misc_testsuite/threads/SB.wast",
            "misc_testsuite/threads/SB_atomic.wast",
            "misc_testsuite/threads/atomics_notify.wast",
            "misc_testsuite/threads/atomics_wait_address.wast",
            "misc_testsuite/threads/wait_notify.wast",
            "spec_testsuite/proposals/threads/atomic.wast",
            "spec_testsuite/proposals/threads/exports.wast",
            "spec_testsuite/proposals/threads/memory.wast",
        ];

        if unsupported.iter().any(|part| test.ends_with(part)) {
            return true;
        }
    }

    false
}

/// Configuration where the main function will generate a combinatorial
/// matrix of these top-level configurations to run the entire test suite with
/// that configuration.
struct WastConfig {
    strategy: Strategy,
    pooling: bool,
    collector: Collector,
}

/// Per-test configuration which is written down in the test file itself for
/// `misc_testsuite/**/*.wast` or in `spec_test_config` below for spec tests.
#[derive(Debug, PartialEq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct TestConfig {
    memory64: Option<bool>,
    custom_page_sizes: Option<bool>,
    multi_memory: Option<bool>,
    threads: Option<bool>,
    gc: Option<bool>,
    function_references: Option<bool>,
    relaxed_simd: Option<bool>,
    reference_types: Option<bool>,
    tail_call: Option<bool>,
    extended_const: Option<bool>,
    wide_arithmetic: Option<bool>,
    hogs_memory: Option<bool>,
    nan_canonicalization: Option<bool>,
    component_model_more_flags: Option<bool>,
}

fn spec_test_config(wast: &Path) -> TestConfig {
    let mut ret = TestConfig::default();

    match wast.strip_prefix("proposals") {
        // This lists the features require to run the various spec tests suites
        // in their `proposals` folder.
        Ok(rest) => {
            let proposal = rest.iter().next().unwrap().to_str().unwrap();
            match proposal {
                "multi-memory" => {
                    ret.multi_memory = Some(true);
                    ret.reference_types = Some(true);
                }
                "wide-arithmetic" => {
                    ret.wide_arithmetic = Some(true);
                }
                "threads" => {
                    ret.threads = Some(true);
                    ret.reference_types = Some(false);
                }
                "tail-call" => {
                    ret.tail_call = Some(true);
                    ret.reference_types = Some(true);
                }
                "relaxed-simd" => {
                    ret.relaxed_simd = Some(true);
                }
                "memory64" => {
                    ret.memory64 = Some(true);
                    ret.tail_call = Some(true);
                    ret.gc = Some(true);
                    ret.extended_const = Some(true);
                    ret.multi_memory = Some(true);
                    ret.relaxed_simd = Some(true);
                }
                "extended-const" => {
                    ret.extended_const = Some(true);
                    ret.reference_types = Some(true);
                }
                "custom-page-sizes" => {
                    ret.custom_page_sizes = Some(true);
                    ret.multi_memory = Some(true);
                }
                "exception-handling" => {
                    ret.reference_types = Some(true);
                }
                "gc" => {
                    ret.gc = Some(true);
                    ret.tail_call = Some(true);
                }
                "function-references" => {
                    ret.function_references = Some(true);
                    ret.tail_call = Some(true);
                }
                "annotations" => {}
                _ => panic!("unsuported proposal {proposal:?}"),
            }
        }

        // This lists the features required to run the top-level of spec tests
        // outside of the `proposals` directory.
        Err(_) => {
            ret.reference_types = Some(true);
        }
    }
    ret
}

// Each of the tests included from `wast_testsuite_tests` will call this
// function which actually executes the `wast` test suite given the `strategy`
// to compile it.
fn run_wast(wast: &Path, config: WastConfig) -> anyhow::Result<()> {
    let wast_contents = std::fs::read_to_string(wast)
        .with_context(|| format!("failed to read `{}`", wast.display()))?;

    // If this is a spec test then the configuration for it is loaded via
    // `spec_test_config`, but otherwise it's required to be listed in the top
    // of the file as we control the contents of the file.
    let mut test_config = match wast.strip_prefix("tests/spec_testsuite") {
        Ok(test) => spec_test_config(test),
        Err(_) => support::parse_test_config(&wast_contents)?,
    };

    // FIXME: this is a bit of a hack to get Winch working here for now. Winch
    // passes some tests on aarch64 so returning `true` from `should_fail`
    // doesn't work. Winch doesn't pass many tests though as it either panics or
    // segfaults as AArch64 support isn't finished yet. That means that we can't
    // have, for example, an allow-list of tests that should pass and assume
    // everything else fails. In lieu of all of this we feign all tests as
    // requiring references types which Wasmtime understands that Winch doesn't
    // support on aarch64 which means that all tests fail quickly in config
    // validation.
    //
    // Ideally the aarch64 backend for Winch would return a normal error on
    // unsupported opcodes and not segfault, meaning that this would not be
    // needed.
    if cfg!(target_arch = "aarch64") && test_config.reference_types.is_none() {
        test_config.reference_types = Some(true);
    }

    let should_fail = should_fail(wast, &config, &test_config);

    let wast = Path::new(wast);

    // Note that all of these proposals/features are currently default-off to
    // ensure that we annotate all tests accurately with what features they
    // need, even in the future when features are stabilized.
    let memory64 = test_config.memory64.unwrap_or(false);
    let custom_page_sizes = test_config.custom_page_sizes.unwrap_or(false);
    let multi_memory = test_config.multi_memory.unwrap_or(false);
    let threads = test_config.threads.unwrap_or(false);
    let gc = test_config.gc.unwrap_or(false);
    let tail_call = test_config.tail_call.unwrap_or(false);
    let extended_const = test_config.extended_const.unwrap_or(false);
    let wide_arithmetic = test_config.wide_arithmetic.unwrap_or(false);
    let test_hogs_memory = test_config.hogs_memory.unwrap_or(false);
    let component_model_more_flags = test_config.component_model_more_flags.unwrap_or(false);
    let nan_canonicalization = test_config.nan_canonicalization.unwrap_or(false);
    let relaxed_simd = test_config.relaxed_simd.unwrap_or(false);

    // Some proposals in wasm depend on previous proposals. For example the gc
    // proposal depends on function-references which depends on reference-types.
    // To avoid needing to enable all of them at once implicitly enable
    // downstream proposals once the end proposal is enabled (e.g. when enabling
    // gc that also enables function-references and reference-types).
    let function_references = test_config
        .function_references
        .or(test_config.gc)
        .unwrap_or(false);
    let reference_types = test_config
        .reference_types
        .or(test_config.function_references)
        .or(test_config.gc)
        .unwrap_or(false);

    let is_cranelift = match config.strategy {
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
        .wasm_extended_const(extended_const)
        .wasm_wide_arithmetic(wide_arithmetic)
        .wasm_component_model_more_flags(component_model_more_flags)
        .strategy(config.strategy)
        .collector(config.collector)
        .cranelift_nan_canonicalization(nan_canonicalization);

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
    if std::env::var("WASMTIME_TEST_NO_HOG_MEMORY").is_ok() {
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
        cfg.memory_reservation(2 * u64::from(Memory::DEFAULT_PAGE_SIZE));
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
        let max_memory_size = 805 << 16;
        if multi_memory {
            cfg.memory_reservation(max_memory_size as u64);
            cfg.memory_reservation_for_growth(0);
            cfg.memory_guard_size(0);
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
            let mut wast_context = WastContext::new(store);
            wast_context.register_spectest(&SpectestConfig {
                use_shared_memory: true,
                suppress_prints: true,
            })?;
            wast_context
                .run_buffer(wast.to_str().unwrap(), wast_contents.as_bytes())
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
