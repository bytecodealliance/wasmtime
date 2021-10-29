//! Oracles.
//!
//! Oracles take a test case and determine whether we have a bug. For example,
//! one of the simplest oracles is to take a Wasm binary as our input test case,
//! validate and instantiate it, and (implicitly) check that no assertions
//! failed or segfaults happened. A more complicated oracle might compare the
//! result of executing a Wasm file with and without optimizations enabled, and
//! make sure that the two executions are observably identical.
//!
//! When an oracle finds a bug, it should report it to the fuzzing engine by
//! panicking.

pub mod dummy;

use anyhow::Context;
use arbitrary::Arbitrary;
use log::{debug, warn};
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};
use wasmtime::*;
use wasmtime_wast::WastContext;

#[cfg(not(any(windows, target_arch = "s390x")))]
pub use self::v8::*;
#[cfg(not(any(windows, target_arch = "s390x")))]
mod v8;

static CNT: AtomicUsize = AtomicUsize::new(0);

fn log_wasm(wasm: &[u8]) {
    if !log::log_enabled!(log::Level::Debug) {
        return;
    }

    let i = CNT.fetch_add(1, SeqCst);
    let name = format!("testcase{}.wasm", i);
    std::fs::write(&name, wasm).expect("failed to write wasm file");
    log::debug!("wrote wasm file to `{}`", name);
    let wat = format!("testcase{}.wat", i);
    match wasmprinter::print_bytes(wasm) {
        Ok(s) => std::fs::write(&wat, s).expect("failed to write wat file"),
        // If wasmprinter failed remove a `*.wat` file, if any, to avoid
        // confusing a preexisting one with this wasm which failed to get
        // printed.
        Err(_) => drop(std::fs::remove_file(&wat)),
    }
}

fn create_store(engine: &Engine) -> Store<StoreLimits> {
    let mut store = Store::new(
        &engine,
        StoreLimits {
            // Limits tables/memories within a store to at most 1gb for now to
            // exercise some larger address but not overflow various limits.
            remaining_memory: 1 << 30,
            oom: false,
        },
    );
    store.limiter(|s| s as &mut dyn ResourceLimiter);
    return store;
}

struct StoreLimits {
    /// Remaining memory, in bytes, left to allocate
    remaining_memory: usize,
    /// Whether or not an allocation request has been denied
    oom: bool,
}

impl StoreLimits {
    fn alloc(&mut self, amt: usize) -> bool {
        match self.remaining_memory.checked_sub(amt) {
            Some(mem) => {
                self.remaining_memory = mem;
                true
            }
            None => {
                self.oom = true;
                false
            }
        }
    }
}

impl ResourceLimiter for StoreLimits {
    fn memory_growing(&mut self, current: usize, desired: usize, _maximum: Option<usize>) -> bool {
        self.alloc(desired - current)
    }

    fn table_growing(&mut self, current: u32, desired: u32, _maximum: Option<u32>) -> bool {
        let delta = (desired - current) as usize * std::mem::size_of::<usize>();
        self.alloc(delta)
    }
}

/// Methods of timing out execution of a WebAssembly module
#[derive(Debug)]
pub enum Timeout {
    /// No timeout is used, it should be guaranteed via some other means that
    /// the input does not infinite loop.
    None,
    /// A time-based timeout is used with a sleeping thread sending a signal
    /// after the specified duration.
    Time(Duration),
    /// Fuel-based timeouts are used where the specified fuel is all that the
    /// provided wasm module is allowed to consume.
    Fuel(u64),
}

/// Instantiate the Wasm buffer, and implicitly fail if we have an unexpected
/// panic or segfault or anything else that can be detected "passively".
///
/// Performs initial validation, and returns early if the Wasm is invalid.
///
/// You can control which compiler is used via passing a `Strategy`.
pub fn instantiate(wasm: &[u8], known_valid: bool, strategy: Strategy) {
    // Explicitly disable module linking for now since it's a breaking change to
    // pre-module-linking modules due to imports
    let mut cfg = crate::fuzz_default_config(strategy).unwrap();
    cfg.wasm_module_linking(false);
    instantiate_with_config(wasm, known_valid, cfg, Timeout::None);
}

/// Instantiate the Wasm buffer, and implicitly fail if we have an unexpected
/// panic or segfault or anything else that can be detected "passively".
///
/// The engine will be configured using provided config.
///
/// See also `instantiate` functions.
pub fn instantiate_with_config(
    wasm: &[u8],
    known_valid: bool,
    mut config: Config,
    timeout: Timeout,
) {
    crate::init_fuzzing();

    config.interruptable(match &timeout {
        Timeout::Time(_) => true,
        _ => false,
    });
    config.consume_fuel(match &timeout {
        Timeout::Fuel(_) => true,
        _ => false,
    });
    let engine = Engine::new(&config).unwrap();
    let mut store = create_store(&engine);

    let mut timeout_state = SignalOnDrop::default();
    match timeout {
        Timeout::Fuel(fuel) => store.add_fuel(fuel).unwrap(),
        // If a timeout is requested then we spawn a helper thread to wait for
        // the requested time and then send us a signal to get interrupted. We
        // also arrange for the thread's sleep to get interrupted if we return
        // early (or the wasm returns within the time limit), which allows the
        // thread to get torn down.
        //
        // This prevents us from creating a huge number of sleeping threads if
        // this function is executed in a loop, like it does on nightly fuzzing
        // infrastructure.
        Timeout::Time(timeout) => {
            let handle = store.interrupt_handle().unwrap();
            timeout_state.spawn_timeout(timeout, move || handle.interrupt());
        }
        Timeout::None => {}
    }

    log_wasm(wasm);
    let module = match Module::new(&engine, wasm) {
        Ok(module) => module,
        Err(_) if !known_valid => return,
        Err(e) => panic!("failed to compile module: {:?}", e),
    };

    instantiate_with_dummy(&mut store, &module);
}

fn instantiate_with_dummy(store: &mut Store<StoreLimits>, module: &Module) -> Option<Instance> {
    // Creation of imports can fail due to resource limit constraints, and then
    // instantiation can naturally fail for a number of reasons as well. Bundle
    // the two steps together to match on the error below.
    let instance =
        dummy::dummy_linker(store, module).and_then(|l| l.instantiate(&mut *store, module));

    let e = match instance {
        Ok(i) => return Some(i),
        Err(e) => e,
    };

    // If the instantiation hit OOM for some reason then that's ok, it's
    // expected that fuzz-generated programs try to allocate lots of
    // stuff.
    if store.data().oom {
        return None;
    }

    // Allow traps which can happen normally with `unreachable` or a
    // timeout or such
    if e.downcast_ref::<Trap>().is_some() {
        return None;
    }

    let string = e.to_string();
    // Also allow errors related to fuel consumption
    if string.contains("all fuel consumed")
        // Currently we instantiate with a `Linker` which can't instantiate
        // every single module under the sun due to using name-based resolution
        // rather than positional-based resolution
        || string.contains("incompatible import type")
    {
        return None;
    }

    // Everything else should be a bug in the fuzzer or a bug in wasmtime
    panic!("failed to instantiate {:?}", e);
}

/// Compile the Wasm buffer, and implicitly fail if we have an unexpected
/// panic or segfault or anything else that can be detected "passively".
///
/// Performs initial validation, and returns early if the Wasm is invalid.
///
/// You can control which compiler is used via passing a `Strategy`.
pub fn compile(wasm: &[u8], strategy: Strategy) {
    crate::init_fuzzing();

    let mut config = crate::fuzz_default_config(strategy).unwrap();
    config.wasm_module_linking(false);
    let engine = Engine::new(&config).unwrap();
    log_wasm(wasm);
    let _ = Module::new(&engine, wasm);
}

/// Instantiate the given Wasm module with each `Config` and call all of its
/// exports. Modulo OOM, non-canonical NaNs, and usage of Wasm features that are
/// or aren't enabled for different configs, we should get the same results when
/// we call the exported functions for all of our different configs.
pub fn differential_execution(
    module: &crate::generators::GeneratedModule,
    configs: &[crate::generators::DifferentialConfig],
) {
    use std::collections::{HashMap, HashSet};

    crate::init_fuzzing();

    // We need at least two configs.
    if configs.len() < 2
        // And all the configs should be unique.
        || configs.iter().collect::<HashSet<_>>().len() != configs.len()
    {
        return;
    }

    let configs: Vec<_> = match configs.iter().map(|c| c.to_wasmtime_config()).collect() {
        Ok(cs) => cs,
        // If the config is trying to use something that was turned off at
        // compile time just continue to the next fuzz input.
        Err(_) => return,
    };

    let mut export_func_results: HashMap<String, Result<Box<[Val]>, Trap>> = Default::default();
    let wasm = module.module.to_bytes();
    log_wasm(&wasm);

    for mut config in configs {
        // Disable module linking since it isn't enabled by default for
        // `GeneratedModule` but is enabled by default for our fuzz config.
        // Since module linking is currently a breaking change this is required
        // to accept modules that would otherwise be broken by module linking.
        config.wasm_module_linking(false);

        let engine = Engine::new(&config).unwrap();
        let mut store = create_store(&engine);

        let module = Module::new(&engine, &wasm).unwrap();

        // TODO: we should implement tracing versions of these dummy imports
        // that record a trace of the order that imported functions were called
        // in and with what values. Like the results of exported functions,
        // calls to imports should also yield the same values for each
        // configuration, and we should assert that.
        let instance = match instantiate_with_dummy(&mut store, &module) {
            Some(instance) => instance,
            None => continue,
        };

        let exports = instance
            .exports(&mut store)
            .filter_map(|e| {
                let name = e.name().to_string();
                e.into_func().map(|f| (name, f))
            })
            .collect::<Vec<_>>();
        for (name, f) in exports {
            // Always call the hang limit initializer first, so that we don't
            // infinite loop when calling another export.
            init_hang_limit(&mut store, instance);

            let ty = f.ty(&store);
            let params = dummy::dummy_values(ty.params());
            let mut results = vec![Val::I32(0); ty.results().len()];
            let this_result = f
                .call(&mut store, &params, &mut results)
                .map(|()| results.into())
                .map_err(|e| e.downcast::<Trap>().unwrap());

            let existing_result = export_func_results
                .entry(name.to_string())
                .or_insert_with(|| this_result.clone());
            assert_same_export_func_result(&existing_result, &this_result, &name);
        }
    }

    fn init_hang_limit<T>(store: &mut Store<T>, instance: Instance) {
        match instance.get_export(&mut *store, "hangLimitInitializer") {
            None => return,
            Some(Extern::Func(f)) => {
                f.call(store, &[], &mut [])
                    .expect("initializing the hang limit should not fail");
            }
            Some(_) => panic!("unexpected hangLimitInitializer export"),
        }
    }

    fn assert_same_export_func_result(
        lhs: &Result<Box<[Val]>, Trap>,
        rhs: &Result<Box<[Val]>, Trap>,
        func_name: &str,
    ) {
        let fail = || {
            panic!(
                "differential fuzzing failed: exported func {} returned two \
                 different results: {:?} != {:?}",
                func_name, lhs, rhs
            )
        };

        match (lhs, rhs) {
            (Err(_), Err(_)) => {}
            (Ok(lhs), Ok(rhs)) => {
                if lhs.len() != rhs.len() {
                    fail();
                }
                for (lhs, rhs) in lhs.iter().zip(rhs.iter()) {
                    match (lhs, rhs) {
                        (Val::I32(lhs), Val::I32(rhs)) if lhs == rhs => continue,
                        (Val::I64(lhs), Val::I64(rhs)) if lhs == rhs => continue,
                        (Val::V128(lhs), Val::V128(rhs)) if lhs == rhs => continue,
                        (Val::F32(lhs), Val::F32(rhs)) if f32_equal(*lhs, *rhs) => continue,
                        (Val::F64(lhs), Val::F64(rhs)) if f64_equal(*lhs, *rhs) => continue,
                        (Val::ExternRef(_), Val::ExternRef(_))
                        | (Val::FuncRef(_), Val::FuncRef(_)) => continue,
                        _ => fail(),
                    }
                }
            }
            _ => fail(),
        }
    }
}

fn f32_equal(a: u32, b: u32) -> bool {
    let a = f32::from_bits(a);
    let b = f32::from_bits(b);
    a == b || (a.is_nan() && b.is_nan())
}

fn f64_equal(a: u64, b: u64) -> bool {
    let a = f64::from_bits(a);
    let b = f64::from_bits(b);
    a == b || (a.is_nan() && b.is_nan())
}

/// Invoke the given API calls.
pub fn make_api_calls(api: crate::generators::api::ApiCalls) {
    use crate::generators::api::ApiCall;
    use std::collections::HashMap;

    crate::init_fuzzing();

    let mut config: Option<Config> = None;
    let mut engine: Option<Engine> = None;
    let mut store: Option<Store<StoreLimits>> = None;
    let mut modules: HashMap<usize, Module> = Default::default();
    let mut instances: HashMap<usize, Instance> = Default::default();

    for call in api.calls {
        match call {
            ApiCall::ConfigNew => {
                log::trace!("creating config");
                assert!(config.is_none());
                config = Some(crate::fuzz_default_config(wasmtime::Strategy::Cranelift).unwrap());
            }

            ApiCall::ConfigDebugInfo(b) => {
                log::trace!("enabling debuginfo");
                config.as_mut().unwrap().debug_info(b);
            }

            ApiCall::ConfigInterruptable(b) => {
                log::trace!("enabling interruption");
                config.as_mut().unwrap().interruptable(b);
            }

            ApiCall::EngineNew => {
                log::trace!("creating engine");
                assert!(engine.is_none());
                engine = Some(Engine::new(config.as_ref().unwrap()).unwrap());
            }

            ApiCall::StoreNew => {
                log::trace!("creating store");
                assert!(store.is_none());
                store = Some(create_store(engine.as_ref().unwrap()));
            }

            ApiCall::ModuleNew { id, wasm } => {
                log::debug!("creating module: {}", id);
                let wasm = wasm.module.to_bytes();
                log_wasm(&wasm);
                let module = match Module::new(engine.as_ref().unwrap(), &wasm) {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                let old = modules.insert(id, module);
                assert!(old.is_none());
            }

            ApiCall::ModuleDrop { id } => {
                log::trace!("dropping module: {}", id);
                drop(modules.remove(&id));
            }

            ApiCall::InstanceNew { id, module } => {
                log::trace!("instantiating module {} as {}", module, id);
                let module = match modules.get(&module) {
                    Some(m) => m,
                    None => continue,
                };

                let store = store.as_mut().unwrap();
                if let Some(instance) = instantiate_with_dummy(store, module) {
                    instances.insert(id, instance);
                }
            }

            ApiCall::InstanceDrop { id } => {
                log::trace!("dropping instance {}", id);
                drop(instances.remove(&id));
            }

            ApiCall::CallExportedFunc { instance, nth } => {
                log::trace!("calling instance export {} / {}", instance, nth);
                let instance = match instances.get(&instance) {
                    Some(i) => i,
                    None => {
                        // Note that we aren't guaranteed to instantiate valid
                        // modules, see comments in `InstanceNew` for details on
                        // that. But the API call generator can't know if
                        // instantiation failed, so we might not actually have
                        // this instance. When that's the case, just skip the
                        // API call and keep going.
                        continue;
                    }
                };
                let store = store.as_mut().unwrap();

                let funcs = instance
                    .exports(&mut *store)
                    .filter_map(|e| match e.into_extern() {
                        Extern::Func(f) => Some(f.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>();

                if funcs.is_empty() {
                    continue;
                }

                let nth = nth % funcs.len();
                let f = &funcs[nth];
                let ty = f.ty(&store);
                let params = dummy::dummy_values(ty.params());
                let mut results = vec![Val::I32(0); ty.results().len()];
                let _ = f.call(store, &params, &mut results);
            }
        }
    }
}

/// Executes the wast `test` spectest with the `config` specified.
///
/// Ensures that spec tests pass regardless of the `Config`.
pub fn spectest(fuzz_config: crate::generators::Config, test: crate::generators::SpecTest) {
    crate::init_fuzzing();
    log::debug!("running {:?} with {:?}", test.file, fuzz_config);
    let mut config = fuzz_config.to_wasmtime();
    config.wasm_memory64(false);
    config.wasm_module_linking(false);
    config.wasm_multi_memory(false);
    let mut store = create_store(&Engine::new(&config).unwrap());
    if fuzz_config.consume_fuel {
        store.add_fuel(u64::max_value()).unwrap();
    }
    let mut wast_context = WastContext::new(store);
    wast_context.register_spectest().unwrap();
    wast_context
        .run_buffer(test.file, test.contents.as_bytes())
        .unwrap();
}

/// Execute a series of `table.get` and `table.set` operations.
pub fn table_ops(
    fuzz_config: crate::generators::Config,
    ops: crate::generators::table_ops::TableOps,
) {
    let _ = env_logger::try_init();

    let expected_drops = Arc::new(AtomicUsize::new(ops.num_params() as usize));
    let num_dropped = Arc::new(AtomicUsize::new(0));

    {
        let mut config = fuzz_config.to_wasmtime();
        config.wasm_reference_types(true);
        config.consume_fuel(true);

        let engine = Engine::new(&config).unwrap();
        let mut store = create_store(&engine);
        store.add_fuel(100).unwrap();

        let wasm = ops.to_wasm_binary();
        log_wasm(&wasm);
        let module = match Module::new(&engine, &wasm) {
            Ok(m) => m,
            Err(_) => return,
        };

        let mut linker = Linker::new(&engine);

        // To avoid timeouts, limit the number of explicit GCs we perform per
        // test case.
        const MAX_GCS: usize = 5;

        let num_gcs = AtomicUsize::new(0);
        linker
            .define(
                "",
                "gc",
                // NB: use `Func::new` so that this can still compile on the old x86
                // backend, where `IntoFunc` isn't implemented for multi-value
                // returns.
                Func::new(
                    &mut store,
                    FuncType::new(
                        vec![],
                        vec![ValType::ExternRef, ValType::ExternRef, ValType::ExternRef],
                    ),
                    {
                        let num_dropped = num_dropped.clone();
                        let expected_drops = expected_drops.clone();
                        move |mut caller: Caller<'_, StoreLimits>, _params, results| {
                            if num_gcs.fetch_add(1, SeqCst) < MAX_GCS {
                                caller.gc();
                            }

                            expected_drops.fetch_add(3, SeqCst);
                            results[0] =
                                Some(ExternRef::new(CountDrops(num_dropped.clone()))).into();
                            results[1] =
                                Some(ExternRef::new(CountDrops(num_dropped.clone()))).into();
                            results[2] =
                                Some(ExternRef::new(CountDrops(num_dropped.clone()))).into();
                            Ok(())
                        }
                    },
                ),
            )
            .unwrap();

        linker
            .func_wrap("", "take_refs", {
                let expected_drops = expected_drops.clone();
                move |a: Option<ExternRef>, b: Option<ExternRef>, c: Option<ExternRef>| {
                    // Do the assertion on each ref's inner data, even though it
                    // all points to the same atomic, so that if we happen to
                    // run into a use-after-free bug with one of these refs we
                    // are more likely to trigger a segfault.
                    if let Some(a) = a {
                        let a = a.data().downcast_ref::<CountDrops>().unwrap();
                        assert!(a.0.load(SeqCst) <= expected_drops.load(SeqCst));
                    }
                    if let Some(b) = b {
                        let b = b.data().downcast_ref::<CountDrops>().unwrap();
                        assert!(b.0.load(SeqCst) <= expected_drops.load(SeqCst));
                    }
                    if let Some(c) = c {
                        let c = c.data().downcast_ref::<CountDrops>().unwrap();
                        assert!(c.0.load(SeqCst) <= expected_drops.load(SeqCst));
                    }
                }
            })
            .unwrap();

        linker
            .define(
                "",
                "make_refs",
                // NB: use `Func::new` so that this can still compile on the old
                // x86 backend, where `IntoFunc` isn't implemented for
                // multi-value returns.
                Func::new(
                    &mut store,
                    FuncType::new(
                        vec![],
                        vec![ValType::ExternRef, ValType::ExternRef, ValType::ExternRef],
                    ),
                    {
                        let num_dropped = num_dropped.clone();
                        let expected_drops = expected_drops.clone();
                        move |_caller, _params, results| {
                            expected_drops.fetch_add(3, SeqCst);
                            results[0] =
                                Some(ExternRef::new(CountDrops(num_dropped.clone()))).into();
                            results[1] =
                                Some(ExternRef::new(CountDrops(num_dropped.clone()))).into();
                            results[2] =
                                Some(ExternRef::new(CountDrops(num_dropped.clone()))).into();
                            Ok(())
                        }
                    },
                ),
            )
            .unwrap();

        let instance = linker.instantiate(&mut store, &module).unwrap();
        let run = instance.get_func(&mut store, "run").unwrap();

        let args: Vec<_> = (0..ops.num_params())
            .map(|_| Val::ExternRef(Some(ExternRef::new(CountDrops(num_dropped.clone())))))
            .collect();
        let _ = run.call(&mut store, &args, &mut []);
    }

    assert_eq!(num_dropped.load(SeqCst), expected_drops.load(SeqCst));
    return;

    struct CountDrops(Arc<AtomicUsize>);

    impl Drop for CountDrops {
        fn drop(&mut self) {
            self.0.fetch_add(1, SeqCst);
        }
    }
}

/// Configuration options for wasm-smith such that generated modules always
/// conform to certain specifications: one exported function, one exported
/// memory.
#[derive(Default, Debug, Arbitrary, Clone)]
pub struct SingleFunctionModuleConfig<const SIMD: bool, const BULK: bool>;

impl<const SIMD: bool, const BULK: bool> wasm_smith::Config
    for SingleFunctionModuleConfig<SIMD, BULK>
{
    fn allow_start_export(&self) -> bool {
        false
    }

    fn min_types(&self) -> usize {
        1
    }

    fn min_funcs(&self) -> usize {
        1
    }

    fn max_funcs(&self) -> usize {
        1
    }

    fn min_memories(&self) -> u32 {
        1
    }

    fn max_memories(&self) -> usize {
        1
    }

    fn max_imports(&self) -> usize {
        0
    }

    fn min_exports(&self) -> usize {
        2
    }

    fn max_memory_pages(&self, _is_64: bool) -> u64 {
        1
    }

    fn memory_max_size_required(&self) -> bool {
        true
    }

    // NaN is canonicalized at the wasm level for differential fuzzing so we
    // can paper over NaN differences between engines.
    fn canonicalize_nans(&self) -> bool {
        true
    }

    fn simd_enabled(&self) -> bool {
        SIMD
    }

    fn bulk_memory_enabled(&self) -> bool {
        BULK
    }
}

/// Perform differential execution between Cranelift and wasmi, diffing the
/// resulting memory image when execution terminates. This relies on the
/// module-under-test to be instrumented to bound the execution time. Invoke
/// with a module generated by `wasm-smith` using the
/// `SingleFunctionModuleConfig` configuration type for best results.
///
/// May return `None` if we early-out due to a rejected fuzz config; these
/// should be rare if modules are generated appropriately.
pub fn differential_wasmi_execution(wasm: &[u8], config: &crate::generators::Config) -> Option<()> {
    crate::init_fuzzing();
    log_wasm(wasm);

    // Instantiate wasmi module and instance.
    let wasmi_module = wasmi::Module::from_buffer(&wasm[..]).ok()?;
    let wasmi_instance =
        wasmi::ModuleInstance::new(&wasmi_module, &wasmi::ImportsBuilder::default()).ok()?;
    let wasmi_instance = wasmi_instance.assert_no_start();

    // If wasmi succeeded then we assert that wasmtime will also succeed.
    let (wasmtime_module, mut wasmtime_store) = differential_store(wasm, config);
    let wasmtime_instance = Instance::new(&mut wasmtime_store, &wasmtime_module, &[])
        .expect("Wasmtime can instantiate module");

    // Introspect wasmtime module to find name of an exported function and of an
    // exported memory.
    let (func_name, ty) = first_exported_function(&wasmtime_module)?;
    let memory_name = first_exported_memory(&wasmtime_module)?;

    let wasmi_mem_export = wasmi_instance.export_by_name(memory_name).unwrap();
    let wasmi_mem = wasmi_mem_export.as_memory().unwrap();
    let wasmi_main_export = wasmi_instance.export_by_name(func_name).unwrap();
    let wasmi_main = wasmi_main_export.as_func().unwrap();
    let wasmi_val = wasmi::FuncInstance::invoke(&wasmi_main, &[], &mut wasmi::NopExternals);

    let wasmtime_mem = wasmtime_instance
        .get_memory(&mut wasmtime_store, memory_name)
        .expect("memory export is present");
    let wasmtime_main = wasmtime_instance
        .get_func(&mut wasmtime_store, func_name)
        .expect("function export is present");
    let mut wasmtime_results = vec![Val::I32(0); ty.results().len()];
    let wasmtime_val = wasmtime_main
        .call(&mut wasmtime_store, &[], &mut wasmtime_results)
        .map(|()| wasmtime_results.get(0).cloned());

    debug!(
        "Successful execution: wasmi returned {:?}, wasmtime returned {:?}",
        wasmi_val, wasmtime_val
    );

    match (&wasmi_val, &wasmtime_val) {
        (&Ok(Some(wasmi::RuntimeValue::I32(a))), &Ok(Some(Val::I32(b)))) if a == b => {}
        (&Ok(Some(wasmi::RuntimeValue::F32(a))), &Ok(Some(Val::F32(b))))
            if f32_equal(a.to_bits(), b) => {}
        (&Ok(Some(wasmi::RuntimeValue::I64(a))), &Ok(Some(Val::I64(b)))) if a == b => {}
        (&Ok(Some(wasmi::RuntimeValue::F64(a))), &Ok(Some(Val::F64(b))))
            if f64_equal(a.to_bits(), b) => {}
        (&Ok(None), &Ok(None)) => {}
        (&Err(_), &Err(_)) => {}
        _ => {
            panic!(
                "Values do not match: wasmi returned {:?}; wasmtime returned {:?}",
                wasmi_val, wasmtime_val
            );
        }
    }

    if wasmi_mem.current_size().0 != wasmtime_mem.size(&wasmtime_store) as usize {
        panic!("resulting memories are not the same size");
    }

    // Wasmi memory may be stored non-contiguously; copy it out to a contiguous chunk.
    let mut wasmi_buf: Vec<u8> = vec![0; wasmtime_mem.data_size(&wasmtime_store)];
    wasmi_mem
        .get_into(0, &mut wasmi_buf[..])
        .expect("can access wasmi memory");

    let wasmtime_slice = wasmtime_mem.data(&wasmtime_store);

    if wasmi_buf.len() >= 64 {
        debug!("-> First 64 bytes of wasmi heap: {:?}", &wasmi_buf[0..64]);
        debug!(
            "-> First 64 bytes of Wasmtime heap: {:?}",
            &wasmtime_slice[0..64]
        );
    }

    if &wasmi_buf[..] != &wasmtime_slice[..] {
        panic!("memory contents are not equal");
    }

    Some(())
}

/// Perform differential execution between Wasmtime and the official WebAssembly
/// specification interpreter.
///
/// May return `None` if we early-out due to a rejected fuzz config.
pub fn differential_spec_execution(wasm: &[u8], config: &crate::generators::Config) -> Option<()> {
    crate::init_fuzzing();
    debug!("config: {:#?}", config);
    log_wasm(wasm);

    // Run the spec interpreter first, then Wasmtime. The order is important
    // because both sides (OCaml runtime and Wasmtime) register signal handlers;
    // Wasmtime uses these signal handlers for catching various WebAssembly
    // failures. On certain OSes (e.g. Linux x86_64), the signal handlers
    // interfere, observable as an uncaught `SIGSEGV`--not even caught by
    // libFuzzer. By running Wasmtime second, its signal handlers are registered
    // most recently and they catch failures appropriately.
    let spec_vals = wasm_spec_interpreter::interpret(wasm, vec![]);
    debug!("spec interpreter returned: {:?}", &spec_vals);
    let wasmtime_vals = run_in_wasmtime(wasm, config, &[]);
    debug!("Wasmtime returned: {:?}", wasmtime_vals);

    // Match a spec interpreter value against a Wasmtime value. Eventually this
    // should support references and `v128` (TODO).
    fn matches(spec_val: &wasm_spec_interpreter::Value, wasmtime_val: &wasmtime::Val) -> bool {
        match (spec_val, wasmtime_val) {
            (wasm_spec_interpreter::Value::I32(a), wasmtime::Val::I32(b)) => a == b,
            (wasm_spec_interpreter::Value::I64(a), wasmtime::Val::I64(b)) => a == b,
            (wasm_spec_interpreter::Value::F32(a), wasmtime::Val::F32(b)) => {
                f32_equal(*a as u32, *b)
            }
            (wasm_spec_interpreter::Value::F64(a), wasmtime::Val::F64(b)) => {
                f64_equal(*a as u64, *b)
            }
            (_, _) => unreachable!("fuzzing non-scalar value types is still TODO"),
        }
    }

    match (&spec_vals, &wasmtime_vals) {
        // Compare the returned values, failing if they do not match.
        (Ok(spec_vals), Ok(wasmtime_vals)) => {
            let all_match = spec_vals
                .iter()
                .zip(wasmtime_vals)
                .all(|(s, w)| matches(s, w));
            if !all_match {
                panic!(
                    "Values do not match: spec returned {:?}; wasmtime returned {:?}",
                    spec_vals, wasmtime_vals
                );
            }
        }
        // If both sides fail, skip this fuzz execution.
        (Err(spec_error), Err(wasmtime_error)) => {
            // The `None` value returned here indicates that both sides
            // failed--if we see too many of these we might be failing too often
            // to check instruction semantics. At some point it would be
            // beneficial to compare the error messages from both sides (TODO).
            // It would also be good to keep track of statistics about the
            // ratios of the kinds of errors the fuzzer sees (TODO).
            warn!(
                "Both sides failed: spec returned '{}'; wasmtime returned {:?}",
                spec_error, wasmtime_error
            );
            return None;
        }
        // If only one side fails, fail the fuzz the test.
        _ => {
            panic!(
                "Only one side failed: spec returned {:?}; wasmtime returned {:?}",
                &spec_vals, &wasmtime_vals
            );
        }
    }

    // TODO Compare memory contents.

    Some(())
}

fn differential_store(
    wasm: &[u8],
    fuzz_config: &crate::generators::Config,
) -> (Module, Store<StoreLimits>) {
    let mut config = fuzz_config.to_wasmtime();
    // forcibly disable NaN canonicalization because wasm-smith has already
    // been configured to canonicalize everything at the wasm level.
    config.cranelift_nan_canonicalization(false);
    let engine = Engine::new(&config).unwrap();
    let mut store = create_store(&engine);
    if fuzz_config.consume_fuel {
        store.add_fuel(u64::max_value()).unwrap();
    }

    let module = Module::new(&engine, &wasm).expect("Wasmtime can compile module");

    (module, store)
}

/// Helper for instantiating and running a Wasm module in Wasmtime and returning
/// its `Val` results.
fn run_in_wasmtime(
    wasm: &[u8],
    config: &crate::generators::Config,
    params: &[Val],
) -> anyhow::Result<Vec<Val>> {
    // Instantiate wasmtime module and instance.
    let (wasmtime_module, mut wasmtime_store) = differential_store(wasm, config);
    let wasmtime_instance = Instance::new(&mut wasmtime_store, &wasmtime_module, &[])
        .context("Wasmtime cannot instantiate module")?;

    // Find the first exported function.
    let (func_name, ty) =
        first_exported_function(&wasmtime_module).context("Cannot find exported function")?;
    let wasmtime_main = wasmtime_instance
        .get_func(&mut wasmtime_store, &func_name[..])
        .expect("function export is present");

    // Execute the function and return the values.
    let mut results = vec![Val::I32(0); ty.results().len()];
    wasmtime_main
        .call(&mut wasmtime_store, params, &mut results)
        .map(|()| results)
}

// Introspect wasmtime module to find the name of the first exported function.
fn first_exported_function(module: &wasmtime::Module) -> Option<(&str, FuncType)> {
    for e in module.exports() {
        match e.ty() {
            wasmtime::ExternType::Func(ty) => return Some((e.name(), ty)),
            _ => {}
        }
    }
    None
}

fn first_exported_memory(module: &Module) -> Option<&str> {
    for e in module.exports() {
        match e.ty() {
            wasmtime::ExternType::Memory(..) => return Some(e.name()),
            _ => {}
        }
    }
    None
}

#[derive(Default)]
struct SignalOnDrop {
    state: Arc<(Mutex<bool>, Condvar)>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl SignalOnDrop {
    fn spawn_timeout(&mut self, dur: Duration, closure: impl FnOnce() + Send + 'static) {
        let state = self.state.clone();
        let start = Instant::now();
        self.thread = Some(std::thread::spawn(move || {
            // Using our mutex/condvar we wait here for the first of `dur` to
            // pass or the `SignalOnDrop` instance to get dropped.
            let (lock, cvar) = &*state;
            let mut signaled = lock.lock().unwrap();
            while !*signaled {
                // Adjust our requested `dur` based on how much time has passed.
                let dur = match dur.checked_sub(start.elapsed()) {
                    Some(dur) => dur,
                    None => break,
                };
                let (lock, result) = cvar.wait_timeout(signaled, dur).unwrap();
                signaled = lock;
                // If we timed out for sure then there's no need to continue
                // since we'll just abort on the next `checked_sub` anyway.
                if result.timed_out() {
                    break;
                }
            }
            drop(signaled);

            closure();
        }));
    }
}

impl Drop for SignalOnDrop {
    fn drop(&mut self) {
        if let Some(thread) = self.thread.take() {
            let (lock, cvar) = &*self.state;
            // Signal our thread that we've been dropped and wake it up if it's
            // blocked.
            let mut g = lock.lock().unwrap();
            *g = true;
            cvar.notify_one();
            drop(g);

            // ... and then wait for the thread to exit to ensure we clean up
            // after ourselves.
            thread.join().unwrap();
        }
    }
}
