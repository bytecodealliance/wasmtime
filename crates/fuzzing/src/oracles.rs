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

#[cfg(feature = "fuzz-spec-interpreter")]
pub mod diff_spec;
pub mod diff_wasmi;
pub mod diff_wasmtime;
pub mod dummy;
pub mod engine;
pub mod memory;
mod stacks;

use self::diff_wasmtime::WasmtimeInstance;
use self::engine::{DiffEngine, DiffInstance};
use crate::generators::{self, DiffValue, DiffValueType};
use crate::single_module_fuzzer::KnownValid;
use arbitrary::Arbitrary;
pub use stacks::check_stacks;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst};
use std::sync::{Arc, Condvar, Mutex};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use wasmtime::*;
use wasmtime_wast::WastContext;

#[cfg(not(any(windows, target_arch = "s390x", target_arch = "riscv64")))]
mod diff_v8;

static CNT: AtomicUsize = AtomicUsize::new(0);

/// Logs a wasm file to the filesystem to make it easy to figure out what wasm
/// was used when debugging.
pub fn log_wasm(wasm: &[u8]) {
    super::init_fuzzing();

    if !log::log_enabled!(log::Level::Debug) {
        return;
    }

    let i = CNT.fetch_add(1, SeqCst);
    let name = format!("testcase{i}.wasm");
    std::fs::write(&name, wasm).expect("failed to write wasm file");
    log::debug!("wrote wasm file to `{}`", name);
    let wat = format!("testcase{i}.wat");
    match wasmprinter::print_bytes(wasm) {
        Ok(s) => std::fs::write(&wat, s).expect("failed to write wat file"),
        // If wasmprinter failed remove a `*.wat` file, if any, to avoid
        // confusing a preexisting one with this wasm which failed to get
        // printed.
        Err(_) => drop(std::fs::remove_file(&wat)),
    }
}

/// The `T` in `Store<T>` for fuzzing stores, used to limit resource
/// consumption during fuzzing.
#[derive(Clone)]
pub struct StoreLimits(Arc<LimitsState>);

struct LimitsState {
    /// Remaining memory, in bytes, left to allocate
    remaining_memory: AtomicUsize,
    /// Whether or not an allocation request has been denied
    oom: AtomicBool,
}

impl StoreLimits {
    /// Creates the default set of limits for all fuzzing stores.
    pub fn new() -> StoreLimits {
        StoreLimits(Arc::new(LimitsState {
            // Limits tables/memories within a store to at most 1gb for now to
            // exercise some larger address but not overflow various limits.
            remaining_memory: AtomicUsize::new(1 << 30),
            oom: AtomicBool::new(false),
        }))
    }

    fn alloc(&mut self, amt: usize) -> bool {
        log::trace!("alloc {amt:#x} bytes");
        match self
            .0
            .remaining_memory
            .fetch_update(SeqCst, SeqCst, |remaining| remaining.checked_sub(amt))
        {
            Ok(_) => true,
            Err(_) => {
                self.0.oom.store(true, SeqCst);
                log::debug!("OOM hit");
                false
            }
        }
    }

    fn is_oom(&self) -> bool {
        self.0.oom.load(SeqCst)
    }
}

impl ResourceLimiter for StoreLimits {
    fn memory_growing(
        &mut self,
        current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> Result<bool> {
        Ok(self.alloc(desired - current))
    }

    fn table_growing(
        &mut self,
        current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> Result<bool> {
        let delta = (desired - current).saturating_mul(std::mem::size_of::<usize>());
        Ok(self.alloc(delta))
    }
}

/// Methods of timing out execution of a WebAssembly module
#[derive(Clone, Debug)]
pub enum Timeout {
    /// No timeout is used, it should be guaranteed via some other means that
    /// the input does not infinite loop.
    None,
    /// Fuel-based timeouts are used where the specified fuel is all that the
    /// provided wasm module is allowed to consume.
    Fuel(u64),
    /// An epoch-interruption-based timeout is used with a sleeping
    /// thread bumping the epoch counter after the specified duration.
    Epoch(Duration),
}

/// Instantiate the Wasm buffer, and implicitly fail if we have an unexpected
/// panic or segfault or anything else that can be detected "passively".
///
/// The engine will be configured using provided config.
pub fn instantiate(
    wasm: &[u8],
    known_valid: KnownValid,
    config: &generators::Config,
    timeout: Timeout,
) {
    let mut store = config.to_store();

    let module = match compile_module(store.engine(), wasm, known_valid, config) {
        Some(module) => module,
        None => return,
    };

    let mut timeout_state = HelperThread::default();
    match timeout {
        Timeout::Fuel(fuel) => store.set_fuel(fuel).unwrap(),

        // If a timeout is requested then we spawn a helper thread to wait for
        // the requested time and then send us a signal to get interrupted. We
        // also arrange for the thread's sleep to get interrupted if we return
        // early (or the wasm returns within the time limit), which allows the
        // thread to get torn down.
        //
        // This prevents us from creating a huge number of sleeping threads if
        // this function is executed in a loop, like it does on nightly fuzzing
        // infrastructure.
        Timeout::Epoch(timeout) => {
            let engine = store.engine().clone();
            timeout_state.run_periodically(timeout, move || engine.increment_epoch());
        }
        Timeout::None => {}
    }

    instantiate_with_dummy(&mut store, &module);
}

/// Represents supported commands to the `instantiate_many` function.
#[derive(Arbitrary, Debug)]
pub enum Command {
    /// Instantiates a module.
    ///
    /// The value is the index of the module to instantiate.
    ///
    /// The module instantiated will be this value modulo the number of modules provided to `instantiate_many`.
    Instantiate(usize),
    /// Terminates a "running" instance.
    ///
    /// The value is the index of the instance to terminate.
    ///
    /// The instance terminated will be this value modulo the number of currently running
    /// instances.
    ///
    /// If no instances are running, the command will be ignored.
    Terminate(usize),
}

/// Instantiates many instances from the given modules.
///
/// The engine will be configured using the provided config.
///
/// The modules are expected to *not* have start functions as no timeouts are configured.
pub fn instantiate_many(
    modules: &[Vec<u8>],
    known_valid: KnownValid,
    config: &generators::Config,
    commands: &[Command],
) {
    assert!(!config.module_config.config.allow_start_export);

    let engine = Engine::new(&config.to_wasmtime()).unwrap();

    let modules = modules
        .iter()
        .filter_map(|bytes| compile_module(&engine, bytes, known_valid, config))
        .collect::<Vec<_>>();

    // If no modules were valid, we're done
    if modules.is_empty() {
        return;
    }

    // This stores every `Store` where a successful instantiation takes place
    let mut stores = Vec::new();
    let limits = StoreLimits::new();

    for command in commands {
        match command {
            Command::Instantiate(index) => {
                let index = *index % modules.len();
                log::info!("instantiating {}", index);
                let module = &modules[index];
                let mut store = Store::new(&engine, limits.clone());
                config.configure_store(&mut store);

                if instantiate_with_dummy(&mut store, module).is_some() {
                    stores.push(Some(store));
                } else {
                    log::warn!("instantiation failed");
                }
            }
            Command::Terminate(index) => {
                if stores.is_empty() {
                    continue;
                }
                let index = *index % stores.len();

                log::info!("dropping {}", index);
                stores.swap_remove(index);
            }
        }
    }
}

fn compile_module(
    engine: &Engine,
    bytes: &[u8],
    known_valid: KnownValid,
    config: &generators::Config,
) -> Option<Module> {
    log_wasm(bytes);

    fn is_pcc_error(e: &anyhow::Error) -> bool {
        // NOTE: please keep this predicate in sync with the display format of CodegenError,
        // defined in `wasmtime/cranelift/codegen/src/result.rs`
        e.to_string().to_lowercase().contains("proof-carrying-code")
    }

    match config.compile(engine, bytes) {
        Ok(module) => Some(module),
        Err(e) if is_pcc_error(&e) => {
            panic!("pcc error in input: {e:#?}");
        }
        Err(_) if known_valid == KnownValid::No => None,
        Err(e) => {
            if let generators::InstanceAllocationStrategy::Pooling(c) = &config.wasmtime.strategy {
                // When using the pooling allocator, accept failures to compile
                // when arbitrary table element limits have been exceeded as
                // there is currently no way to constrain the generated module
                // table types.
                let string = e.to_string();
                if string.contains("minimum element size") {
                    return None;
                }

                // Allow modules-failing-to-compile which exceed the requested
                // size for each instance. This is something that is difficult
                // to control and ensure it always succeeds, so we simply have a
                // "random" instance size limit and if a module doesn't fit we
                // move on to the next fuzz input.
                if string.contains("instance allocation for this module requires") {
                    return None;
                }

                // If the pooling allocator is more restrictive on the number of
                // tables and memories than we allowed wasm-smith to generate
                // then allow compilation errors along those lines.
                if c.max_tables_per_module < (config.module_config.config.max_tables as u32)
                    && string.contains("defined tables count")
                    && string.contains("exceeds the per-instance limit")
                {
                    return None;
                }

                if c.max_memories_per_module < (config.module_config.config.max_memories as u32)
                    && string.contains("defined memories count")
                    && string.contains("exceeds the per-instance limit")
                {
                    return None;
                }
            }

            panic!("failed to compile module: {e:?}");
        }
    }
}

/// Create a Wasmtime [`Instance`] from a [`Module`] and fill in all imports
/// with dummy values (e.g., zeroed values, immediately-trapping functions).
/// Also, this function catches certain fuzz-related instantiation failures and
/// returns `None` instead of panicking.
///
/// TODO: we should implement tracing versions of these dummy imports that
/// record a trace of the order that imported functions were called in and with
/// what values. Like the results of exported functions, calls to imports should
/// also yield the same values for each configuration, and we should assert
/// that.
pub fn instantiate_with_dummy(store: &mut Store<StoreLimits>, module: &Module) -> Option<Instance> {
    // Creation of imports can fail due to resource limit constraints, and then
    // instantiation can naturally fail for a number of reasons as well. Bundle
    // the two steps together to match on the error below.
    let instance =
        dummy::dummy_linker(store, module).and_then(|l| l.instantiate(&mut *store, module));
    unwrap_instance(store, instance)
}

fn unwrap_instance(
    store: &Store<StoreLimits>,
    instance: anyhow::Result<Instance>,
) -> Option<Instance> {
    let e = match instance {
        Ok(i) => return Some(i),
        Err(e) => e,
    };

    // If the instantiation hit OOM for some reason then that's ok, it's
    // expected that fuzz-generated programs try to allocate lots of
    // stuff.
    if store.data().is_oom() {
        log::debug!("failed to instantiate: OOM");
        return None;
    }

    // Allow traps which can happen normally with `unreachable` or a
    // timeout or such
    if let Some(trap) = e.downcast_ref::<Trap>() {
        log::debug!("failed to instantiate: {}", trap);
        return None;
    }

    let string = e.to_string();
    // Currently we instantiate with a `Linker` which can't instantiate
    // every single module under the sun due to using name-based resolution
    // rather than positional-based resolution
    if string.contains("incompatible import type") {
        log::debug!("failed to instantiate: {}", string);
        return None;
    }

    // Also allow failures to instantiate as a result of hitting pooling limits.
    if e.is::<wasmtime::PoolConcurrencyLimitError>() {
        log::debug!("failed to instantiate: {}", string);
        return None;
    }

    // Everything else should be a bug in the fuzzer or a bug in wasmtime
    panic!("failed to instantiate: {e:?}");
}

/// Evaluate the function identified by `name` in two different engine
/// instances--`lhs` and `rhs`.
///
/// Returns `Ok(true)` if more evaluations can happen or `Ok(false)` if the
/// instances may have drifted apart and no more evaluations can happen.
///
/// # Panics
///
/// This will panic if the evaluation is different between engines (e.g.,
/// results are different, hashed instance is different, one side traps, etc.).
pub fn differential(
    lhs: &mut dyn DiffInstance,
    lhs_engine: &dyn DiffEngine,
    rhs: &mut WasmtimeInstance,
    name: &str,
    args: &[DiffValue],
    result_tys: &[DiffValueType],
) -> anyhow::Result<bool> {
    log::debug!("Evaluating: `{}` with {:?}", name, args);
    let lhs_results = match lhs.evaluate(name, args, result_tys) {
        Ok(Some(results)) => Ok(results),
        Err(e) => Err(e),
        // this engine couldn't execute this type signature, so discard this
        // execution by returning success.
        Ok(None) => return Ok(true),
    };
    log::debug!(" -> results on {}: {:?}", lhs.name(), &lhs_results);

    let rhs_results = rhs
        .evaluate(name, args, result_tys)
        // wasmtime should be able to invoke any signature, so unwrap this result
        .map(|results| results.unwrap());
    log::debug!(" -> results on {}: {:?}", rhs.name(), &rhs_results);

    // If Wasmtime hit its OOM condition, which is possible since it's set
    // somewhat low while fuzzing, then don't return an error but return
    // `false` indicating that differential fuzzing must stop. There's no
    // guarantee the other engine has the same OOM limits as Wasmtime, and
    // it's assumed that Wasmtime is configured to have a more conservative
    // limit than the other engine.
    if rhs.is_oom() {
        return Ok(false);
    }

    match DiffEqResult::new(lhs_engine, lhs_results, rhs_results) {
        DiffEqResult::Success(lhs, rhs) => assert_eq!(lhs, rhs),
        DiffEqResult::Poisoned => return Ok(false),
        DiffEqResult::Failed => {}
    }

    for (global, ty) in rhs.exported_globals() {
        log::debug!("Comparing global `{global}`");
        let lhs = match lhs.get_global(&global, ty) {
            Some(val) => val,
            None => continue,
        };
        let rhs = rhs.get_global(&global, ty).unwrap();
        assert_eq!(lhs, rhs);
    }
    for (memory, shared) in rhs.exported_memories() {
        log::debug!("Comparing memory `{memory}`");
        let lhs = match lhs.get_memory(&memory, shared) {
            Some(val) => val,
            None => continue,
        };
        let rhs = rhs.get_memory(&memory, shared).unwrap();
        if lhs == rhs {
            continue;
        }
        eprintln!("differential memory is {} bytes long", lhs.len());
        eprintln!("wasmtime memory is     {} bytes long", rhs.len());
        panic!("memories have differing values");
    }

    Ok(true)
}

/// Result of comparing the result of two operations during differential
/// execution.
pub enum DiffEqResult<T, U> {
    /// Both engines succeeded.
    Success(T, U),
    /// The result has reached the state where engines may have diverged and
    /// results can no longer be compared.
    Poisoned,
    /// Both engines failed with the same error message, and internal state
    /// should still match between the two engines.
    Failed,
}

impl<T, U> DiffEqResult<T, U> {
    /// Computes the differential result from executing in two different
    /// engines.
    pub fn new(
        lhs_engine: &dyn DiffEngine,
        lhs_result: Result<T>,
        rhs_result: Result<U>,
    ) -> DiffEqResult<T, U> {
        match (lhs_result, rhs_result) {
            (Ok(lhs_result), Ok(rhs_result)) => DiffEqResult::Success(lhs_result, rhs_result),

            // Both sides failed. If either one hits a stack overflow then that's an
            // engine defined limit which means we can no longer compare the state
            // of the two instances, so `None` is returned and nothing else is
            // compared.
            (Err(lhs), Err(rhs)) => {
                let err = match rhs.downcast::<Trap>() {
                    Ok(trap) => trap,
                    Err(err) => {
                        log::debug!("rhs failed: {err:?}");
                        return DiffEqResult::Failed;
                    }
                };
                let poisoned = err == Trap::StackOverflow || lhs_engine.is_stack_overflow(&lhs);

                if poisoned {
                    return DiffEqResult::Poisoned;
                }
                lhs_engine.assert_error_match(&err, &lhs);
                DiffEqResult::Failed
            }
            // A real bug is found if only one side fails.
            (Ok(_), Err(err)) => panic!("only the `rhs` failed for this input: {err:?}"),
            (Err(err), Ok(_)) => panic!("only the `lhs` failed for this input: {err:?}"),
        }
    }
}

/// Invoke the given API calls.
pub fn make_api_calls(api: generators::api::ApiCalls) {
    use crate::generators::api::ApiCall;
    use std::collections::HashMap;

    let mut store: Option<Store<StoreLimits>> = None;
    let mut modules: HashMap<usize, Module> = Default::default();
    let mut instances: HashMap<usize, Instance> = Default::default();

    for call in api.calls {
        match call {
            ApiCall::StoreNew(config) => {
                log::trace!("creating store");
                assert!(store.is_none());
                store = Some(config.to_store());
            }

            ApiCall::ModuleNew { id, wasm } => {
                log::debug!("creating module: {}", id);
                log_wasm(&wasm);
                let module = match Module::new(store.as_ref().unwrap().engine(), &wasm) {
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
                instances.remove(&id);
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
                        Extern::Func(f) => Some(f),
                        _ => None,
                    })
                    .collect::<Vec<_>>();

                if funcs.is_empty() {
                    continue;
                }

                let nth = nth % funcs.len();
                let f = &funcs[nth];
                let ty = f.ty(&store);
                if let Ok(params) = dummy::dummy_values(ty.params()) {
                    let mut results = vec![Val::I32(0); ty.results().len()];
                    let _ = f.call(store, &params, &mut results);
                }
            }
        }
    }
}

/// Executes the wast `test` with the `config` specified.
///
/// Ensures that wast tests pass regardless of the `Config`.
pub fn wast_test(fuzz_config: generators::Config, test: generators::WastTest) {
    crate::init_fuzzing();
    if !fuzz_config.is_wast_test_compliant() {
        return;
    }

    // Fuel and epochs don't play well with threads right now, so exclude any
    // thread-spawning test if it looks like threads are spawned in that case.
    if fuzz_config.wasmtime.consume_fuel || fuzz_config.wasmtime.epoch_interruption {
        if test.contents.contains("(thread") {
            return;
        }
    }

    log::debug!("running {:?}", test.file);
    let mut wast_context = WastContext::new(fuzz_config.to_store());
    wast_context
        .register_spectest(&wasmtime_wast::SpectestConfig {
            use_shared_memory: false,
            suppress_prints: true,
        })
        .unwrap();
    wast_context
        .run_buffer(test.file, test.contents.as_bytes())
        .unwrap();
}

/// Execute a series of `table.get` and `table.set` operations.
///
/// Returns the number of `gc` operations which occurred throughout the test
/// case -- used to test below that gc happens reasonably soon and eventually.
pub fn table_ops(
    mut fuzz_config: generators::Config,
    ops: generators::table_ops::TableOps,
) -> Result<usize> {
    let expected_drops = Arc::new(AtomicUsize::new(ops.num_params as usize));
    let num_dropped = Arc::new(AtomicUsize::new(0));

    let num_gcs = Arc::new(AtomicUsize::new(0));
    {
        fuzz_config.wasmtime.consume_fuel = true;
        let mut store = fuzz_config.to_store();
        store.set_fuel(1_000).unwrap();

        let wasm = ops.to_wasm_binary();
        log_wasm(&wasm);
        let module = match compile_module(store.engine(), &wasm, KnownValid::No, &fuzz_config) {
            Some(m) => m,
            None => return Ok(0),
        };

        let mut linker = Linker::new(store.engine());

        // To avoid timeouts, limit the number of explicit GCs we perform per
        // test case.
        const MAX_GCS: usize = 5;

        let func_ty = FuncType::new(
            store.engine(),
            vec![],
            vec![ValType::EXTERNREF, ValType::EXTERNREF, ValType::EXTERNREF],
        );
        let func = Func::new(&mut store, func_ty, {
            let num_dropped = num_dropped.clone();
            let expected_drops = expected_drops.clone();
            let num_gcs = num_gcs.clone();
            move |mut caller: Caller<'_, StoreLimits>, _params, results| {
                log::info!("table_ops: GC");
                if num_gcs.fetch_add(1, SeqCst) < MAX_GCS {
                    caller.gc();
                }

                let a = ExternRef::new(&mut caller, CountDrops(num_dropped.clone()))?;
                let b = ExternRef::new(&mut caller, CountDrops(num_dropped.clone()))?;
                let c = ExternRef::new(&mut caller, CountDrops(num_dropped.clone()))?;

                log::info!("table_ops: gc() -> ({:?}, {:?}, {:?})", a, b, c);

                expected_drops.fetch_add(3, SeqCst);
                results[0] = Some(a).into();
                results[1] = Some(b).into();
                results[2] = Some(c).into();
                Ok(())
            }
        });
        linker.define(&store, "", "gc", func).unwrap();

        linker
            .func_wrap("", "take_refs", {
                let expected_drops = expected_drops.clone();
                move |caller: Caller<'_, StoreLimits>,
                      a: Option<Rooted<ExternRef>>,
                      b: Option<Rooted<ExternRef>>,
                      c: Option<Rooted<ExternRef>>|
                      -> Result<()> {
                    log::info!("table_ops: take_refs({a:?}, {b:?}, {c:?})",);

                    // Do the assertion on each ref's inner data, even though it
                    // all points to the same atomic, so that if we happen to
                    // run into a use-after-free bug with one of these refs we
                    // are more likely to trigger a segfault.
                    if let Some(a) = a {
                        let a = a.data(&caller)?.downcast_ref::<CountDrops>().unwrap();
                        assert!(a.0.load(SeqCst) <= expected_drops.load(SeqCst));
                    }
                    if let Some(b) = b {
                        let b = b.data(&caller)?.downcast_ref::<CountDrops>().unwrap();
                        assert!(b.0.load(SeqCst) <= expected_drops.load(SeqCst));
                    }
                    if let Some(c) = c {
                        let c = c.data(&caller)?.downcast_ref::<CountDrops>().unwrap();
                        assert!(c.0.load(SeqCst) <= expected_drops.load(SeqCst));
                    }
                    Ok(())
                }
            })
            .unwrap();

        let func_ty = FuncType::new(
            store.engine(),
            vec![],
            vec![ValType::EXTERNREF, ValType::EXTERNREF, ValType::EXTERNREF],
        );
        let func = Func::new(&mut store, func_ty, {
            let num_dropped = num_dropped.clone();
            let expected_drops = expected_drops.clone();
            move |mut caller, _params, results| {
                log::info!("table_ops: make_refs");

                let a = ExternRef::new(&mut caller, CountDrops(num_dropped.clone()))?;
                let b = ExternRef::new(&mut caller, CountDrops(num_dropped.clone()))?;
                let c = ExternRef::new(&mut caller, CountDrops(num_dropped.clone()))?;
                expected_drops.fetch_add(3, SeqCst);

                log::info!("table_ops: make_refs() -> ({:?}, {:?}, {:?})", a, b, c);

                results[0] = Some(a).into();
                results[1] = Some(b).into();
                results[2] = Some(c).into();

                Ok(())
            }
        });
        linker.define(&store, "", "make_refs", func).unwrap();

        let instance = linker.instantiate(&mut store, &module).unwrap();
        let run = instance.get_func(&mut store, "run").unwrap();

        {
            let mut scope = RootScope::new(&mut store);

            log::info!(
                "table_ops: begin allocating {} externref arguments",
                ops.num_globals
            );
            let args: Vec<_> = (0..ops.num_params)
                .map(|_| {
                    Ok(Val::ExternRef(Some(ExternRef::new(
                        &mut scope,
                        CountDrops(num_dropped.clone()),
                    )?)))
                })
                .collect::<Result<_>>()?;
            log::info!(
                "table_ops: end allocating {} externref arguments",
                ops.num_globals
            );

            // The generated function should always return a trap. The only two
            // valid traps are table-out-of-bounds which happens through `table.get`
            // and `table.set` generated or an out-of-fuel trap. Otherwise any other
            // error is unexpected and should fail fuzzing.
            log::info!("table_ops: calling into Wasm `run` function");
            let trap = run
                .call(&mut scope, &args, &mut [])
                .unwrap_err()
                .downcast::<Trap>()
                .unwrap();

            match trap {
                Trap::TableOutOfBounds | Trap::OutOfFuel => {}
                _ => panic!("unexpected trap: {trap}"),
            }
        }

        // Do a final GC after running the Wasm.
        store.gc();
    }

    assert_eq!(num_dropped.load(SeqCst), expected_drops.load(SeqCst));
    return Ok(num_gcs.load(SeqCst));

    struct CountDrops(Arc<AtomicUsize>);

    impl Drop for CountDrops {
        fn drop(&mut self) {
            self.0.fetch_add(1, SeqCst);
        }
    }
}

// Test that the `table_ops` fuzzer eventually runs the gc function in the host.
// We've historically had issues where this fuzzer accidentally wasn't fuzzing
// anything for a long time so this is an attempt to prevent that from happening
// again.
#[test]
fn table_ops_eventually_gcs() {
    use arbitrary::Unstructured;
    use rand::prelude::*;

    // Skip if we're under emulation because some fuzz configurations will do
    // large address space reservations that QEMU doesn't handle well.
    if std::env::var("WASMTIME_TEST_NO_HOG_MEMORY").is_ok() {
        return;
    }

    let mut rng = SmallRng::seed_from_u64(0);
    let mut buf = vec![0; 2048];
    let n = 100;
    for _ in 0..n {
        rng.fill_bytes(&mut buf);
        let u = Unstructured::new(&buf);

        if let Ok((config, test)) = Arbitrary::arbitrary_take_rest(u) {
            if table_ops(config, test).unwrap() > 0 {
                return;
            }
        }
    }

    panic!("after {n} runs nothing ever gc'd, something is probably wrong");
}

#[derive(Default)]
struct HelperThread {
    state: Arc<HelperThreadState>,
    thread: Option<std::thread::JoinHandle<()>>,
}

#[derive(Default)]
struct HelperThreadState {
    should_exit: Mutex<bool>,
    should_exit_cvar: Condvar,
}

impl HelperThread {
    fn run_periodically(&mut self, dur: Duration, mut closure: impl FnMut() + Send + 'static) {
        let state = self.state.clone();
        self.thread = Some(std::thread::spawn(move || {
            // Using our mutex/condvar we wait here for the first of `dur` to
            // pass or the `HelperThread` instance to get dropped.
            let mut should_exit = state.should_exit.lock().unwrap();
            while !*should_exit {
                let (lock, result) = state
                    .should_exit_cvar
                    .wait_timeout(should_exit, dur)
                    .unwrap();
                should_exit = lock;
                // If we timed out for sure then there's no need to continue
                // since we'll just abort on the next `checked_sub` anyway.
                if result.timed_out() {
                    closure();
                }
            }
        }));
    }
}

impl Drop for HelperThread {
    fn drop(&mut self) {
        let thread = match self.thread.take() {
            Some(thread) => thread,
            None => return,
        };
        // Signal our thread that it should exit and wake it up in case it's
        // sleeping.
        *self.state.should_exit.lock().unwrap() = true;
        self.state.should_exit_cvar.notify_one();

        // ... and then wait for the thread to exit to ensure we clean up
        // after ourselves.
        thread.join().unwrap();
    }
}

/// Generate and execute a `crate::generators::component_types::TestCase` using the specified `input` to create
/// arbitrary types and values.
pub fn dynamic_component_api_target(input: &mut arbitrary::Unstructured) -> arbitrary::Result<()> {
    use crate::generators::component_types;
    use component_fuzz_util::{TestCase, Type, EXPORT_FUNCTION, IMPORT_FUNCTION, MAX_TYPE_DEPTH};
    use component_test_util::FuncExt;
    use wasmtime::component::{Component, Linker, Val};

    crate::init_fuzzing();

    let mut types = Vec::new();
    let mut type_fuel = 500;

    for _ in 0..5 {
        types.push(Type::generate(input, MAX_TYPE_DEPTH, &mut type_fuel)?);
    }
    let params = (0..input.int_in_range(0..=5)?)
        .map(|_| input.choose(&types))
        .collect::<arbitrary::Result<Vec<_>>>()?;
    let results = (0..input.int_in_range(0..=5)?)
        .map(|_| input.choose(&types))
        .collect::<arbitrary::Result<Vec<_>>>()?;

    let case = TestCase {
        params,
        results,
        encoding1: input.arbitrary()?,
        encoding2: input.arbitrary()?,
    };

    let mut config = component_test_util::config();
    if case.results.len() > 1 {
        config.wasm_component_model_multiple_returns(true);
    }
    config.debug_adapter_modules(input.arbitrary()?);
    let engine = Engine::new(&config).unwrap();
    let mut store = Store::new(&engine, (Vec::new(), None));
    let wat = case.declarations().make_component();
    let wat = wat.as_bytes();
    log_wasm(wat);
    let component = Component::new(&engine, wat).unwrap();
    let mut linker = Linker::new(&engine);

    linker
        .root()
        .func_new(IMPORT_FUNCTION, {
            move |mut cx: StoreContextMut<'_, (Vec<Val>, Option<Vec<Val>>)>,
                  params: &[Val],
                  results: &mut [Val]|
                  -> Result<()> {
                log::trace!("received params {params:?}");
                let (expected_args, expected_results) = cx.data_mut();
                assert_eq!(params.len(), expected_args.len());
                for (expected, actual) in expected_args.iter().zip(params) {
                    assert_eq!(expected, actual);
                }
                results.clone_from_slice(&expected_results.take().unwrap());
                log::trace!("returning results {results:?}");
                Ok(())
            }
        })
        .unwrap();

    let instance = linker.instantiate(&mut store, &component).unwrap();
    let func = instance.get_func(&mut store, EXPORT_FUNCTION).unwrap();
    let param_tys = func.params(&store);
    let result_tys = func.results(&store);

    while input.arbitrary()? {
        let params = param_tys
            .iter()
            .map(|ty| component_types::arbitrary_val(ty, input))
            .collect::<arbitrary::Result<Vec<_>>>()?;
        let results = result_tys
            .iter()
            .map(|ty| component_types::arbitrary_val(ty, input))
            .collect::<arbitrary::Result<Vec<_>>>()?;

        *store.data_mut() = (params.clone(), Some(results.clone()));

        log::trace!("passing params {params:?}");
        let mut actual = vec![Val::Bool(false); results.len()];
        func.call_and_post_return(&mut store, &params, &mut actual)
            .unwrap();
        log::trace!("received results {actual:?}");
        assert_eq!(actual, results);
    }

    Ok(())
}

/// Instantiates a wasm module and runs its exports with dummy values, all in
/// an async fashion.
///
/// Attempts to stress yields in host functions to ensure that exiting and
/// resuming a wasm function call works.
pub fn call_async(wasm: &[u8], config: &generators::Config, mut poll_amts: &[u32]) {
    let mut store = config.to_store();
    let module = match compile_module(store.engine(), wasm, KnownValid::Yes, config) {
        Some(module) => module,
        None => return,
    };

    // Configure a helper thread to periodically increment the epoch to
    // forcibly enable yields-via-epochs if epochs are in use. Note that this
    // is required because the wasm isn't otherwise guaranteed to necessarily
    // call any imports which will also increment the epoch.
    let mut helper_thread = HelperThread::default();
    if let generators::AsyncConfig::YieldWithEpochs { dur, .. } = &config.wasmtime.async_config {
        let engine = store.engine().clone();
        helper_thread.run_periodically(*dur, move || engine.increment_epoch());
    }

    // Generate a `Linker` where all function imports are custom-built to yield
    // periodically and additionally increment the epoch.
    let mut imports = Vec::new();
    for import in module.imports() {
        let item = match import.ty() {
            ExternType::Func(ty) => {
                let poll_amt = take_poll_amt(&mut poll_amts);
                Func::new_async(&mut store, ty.clone(), move |caller, _, results| {
                    let ty = ty.clone();
                    Box::new(async move {
                        caller.engine().increment_epoch();
                        log::info!("yielding {} times in import", poll_amt);
                        YieldN(poll_amt).await;
                        for (ret_ty, result) in ty.results().zip(results) {
                            *result = dummy::dummy_value(ret_ty)?;
                        }
                        Ok(())
                    })
                })
                .into()
            }
            other_ty => match dummy::dummy_extern(&mut store, other_ty) {
                Ok(item) => item,
                Err(e) => {
                    log::warn!("couldn't create import: {}", e);
                    return;
                }
            },
        };
        imports.push(item);
    }

    // Run the instantiation process, asynchronously, and if everything
    // succeeds then pull out the instance.
    // log::info!("starting instantiation");
    let instance = run(Timeout {
        future: Instance::new_async(&mut store, &module, &imports),
        polls: take_poll_amt(&mut poll_amts),
        end: Instant::now() + Duration::from_millis(2_000),
    });
    let instance = match instance {
        Ok(instantiation_result) => match unwrap_instance(&store, instantiation_result) {
            Some(instance) => instance,
            None => {
                log::info!("instantiation hit a nominal error");
                return; // resource exhaustion or limits met
            }
        },
        Err(_) => {
            log::info!("instantiation failed to complete");
            return; // Timed out or ran out of polls
        }
    };

    // Run each export of the instance in the same manner as instantiation
    // above. Dummy values are passed in for argument values here:
    //
    // TODO: this should probably be more clever about passing in arguments for
    // example they might be used as pointers or something and always using 0
    // isn't too interesting.
    let funcs = instance
        .exports(&mut store)
        .filter_map(|e| {
            let name = e.name().to_string();
            let func = e.into_extern().into_func()?;
            Some((name, func))
        })
        .collect::<Vec<_>>();
    for (name, func) in funcs {
        let ty = func.ty(&store);
        let params = ty
            .params()
            .map(|ty| dummy::dummy_value(ty).unwrap())
            .collect::<Vec<_>>();
        let mut results = ty
            .results()
            .map(|ty| dummy::dummy_value(ty).unwrap())
            .collect::<Vec<_>>();

        log::info!("invoking export {:?}", name);
        let future = func.call_async(&mut store, &params, &mut results);
        match run(Timeout {
            future,
            polls: take_poll_amt(&mut poll_amts),
            end: Instant::now() + Duration::from_millis(2_000),
        }) {
            // On success or too many polls, try the next export.
            Ok(_) | Err(Exhausted::Polls) => {}

            // If time ran out then stop the current test case as we might have
            // already sucked up a lot of time for this fuzz test case so don't
            // keep it going.
            Err(Exhausted::Time) => return,
        }
    }

    fn take_poll_amt(polls: &mut &[u32]) -> u32 {
        match polls.split_first() {
            Some((a, rest)) => {
                *polls = rest;
                *a
            }
            None => 0,
        }
    }

    /// Helper future to yield N times before resolving.
    struct YieldN(u32);

    impl Future for YieldN {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
            if self.0 == 0 {
                Poll::Ready(())
            } else {
                self.0 -= 1;
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }

    /// Helper future for applying a timeout to `future` up to either when `end`
    /// is the current time or `polls` polls happen.
    ///
    /// Note that this helps to time out infinite loops in wasm, for example.
    struct Timeout<F> {
        future: F,
        /// If the future isn't ready by this time then the `Timeout<F>` future
        /// will return `None`.
        end: Instant,
        /// If the future doesn't resolve itself in this many calls to `poll`
        /// then the `Timeout<F>` future will return `None`.
        polls: u32,
    }

    enum Exhausted {
        Time,
        Polls,
    }

    impl<F: Future> Future for Timeout<F> {
        type Output = Result<F::Output, Exhausted>;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let (end, polls, future) = unsafe {
                let me = self.get_unchecked_mut();
                (me.end, &mut me.polls, Pin::new_unchecked(&mut me.future))
            };
            match future.poll(cx) {
                Poll::Ready(val) => Poll::Ready(Ok(val)),
                Poll::Pending => {
                    if Instant::now() >= end {
                        log::warn!("future operation timed out");
                        return Poll::Ready(Err(Exhausted::Time));
                    }
                    if *polls == 0 {
                        log::warn!("future operation ran out of polls");
                        return Poll::Ready(Err(Exhausted::Polls));
                    }
                    *polls -= 1;
                    Poll::Pending
                }
            }
        }
    }

    fn run<F: Future>(future: F) -> F::Output {
        let mut f = Box::pin(future);
        let mut cx = Context::from_waker(futures::task::noop_waker_ref());
        loop {
            match f.as_mut().poll(&mut cx) {
                Poll::Ready(val) => break val,
                Poll::Pending => {}
            }
        }
    }
}
