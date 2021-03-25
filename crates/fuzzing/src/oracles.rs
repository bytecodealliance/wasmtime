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

use arbitrary::Arbitrary;
use dummy::dummy_linker;
use log::debug;
use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};
use wasmtime::*;
use wasmtime_wast::WastContext;

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
    let store = Store::new(&engine);

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
    let linker = dummy_linker(&store, &module);

    match linker.instantiate(&module) {
        Ok(_) => {}
        // Allow traps which can happen normally with `unreachable` or a timeout
        Err(e) if e.downcast_ref::<Trap>().is_some() => {}
        // Allow resource exhaustion since this is something that our wasm-smith
        // generator doesn't guarantee is forbidden.
        Err(e) if e.to_string().contains("resource limit exceeded") => {}
        // Also allow errors related to fuel consumption
        Err(e) if e.to_string().contains("all fuel consumed") => {}
        // Everything else should be a bug in the fuzzer
        Err(e) => panic!("failed to instantiate {}", e),
    }
}

/// Compile the Wasm buffer, and implicitly fail if we have an unexpected
/// panic or segfault or anything else that can be detected "passively".
///
/// Performs initial validation, and returns early if the Wasm is invalid.
///
/// You can control which compiler is used via passing a `Strategy`.
pub fn compile(wasm: &[u8], strategy: Strategy) {
    crate::init_fuzzing();

    let engine = Engine::new(&crate::fuzz_default_config(strategy).unwrap()).unwrap();
    log_wasm(wasm);
    let _ = Module::new(&engine, wasm);
}

/// Instantiate the given Wasm module with each `Config` and call all of its
/// exports. Modulo OOM, non-canonical NaNs, and usage of Wasm features that are
/// or aren't enabled for different configs, we should get the same results when
/// we call the exported functions for all of our different configs.
pub fn differential_execution(
    module: &wasm_smith::Module,
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
        // compile time, eg lightbeam, just continue to the next fuzz input.
        Err(_) => return,
    };

    let mut export_func_results: HashMap<String, Result<Box<[Val]>, Trap>> = Default::default();
    let wasm = module.to_bytes();
    log_wasm(&wasm);

    for mut config in configs {
        // Disable module linking since it isn't enabled by default for
        // `wasm_smith::Module` but is enabled by default for our fuzz config.
        // Since module linking is currently a breaking change this is required
        // to accept modules that would otherwise be broken by module linking.
        config.wasm_module_linking(false);

        let engine = Engine::new(&config).unwrap();
        let store = Store::new(&engine);

        let module = Module::new(&engine, &wasm).unwrap();

        // TODO: we should implement tracing versions of these dummy imports
        // that record a trace of the order that imported functions were called
        // in and with what values. Like the results of exported functions,
        // calls to imports should also yield the same values for each
        // configuration, and we should assert that.
        let linker = dummy_linker(&store, &module);

        // Don't unwrap this: there can be instantiation-/link-time errors that
        // aren't caught during validation or compilation. For example, an imported
        // table might not have room for an element segment that we want to
        // initialize into it.
        let instance = match linker.instantiate(&module) {
            Ok(instance) => instance,
            Err(e) => {
                eprintln!(
                    "Warning: failed to instantiate `wasm-opt -ttf` module: {}",
                    e
                );
                continue;
            }
        };

        for (name, f) in instance.exports().filter_map(|e| {
            let name = e.name();
            e.into_func().map(|f| (name, f))
        }) {
            // Always call the hang limit initializer first, so that we don't
            // infinite loop when calling another export.
            init_hang_limit(&instance);

            let ty = f.ty();
            let params = dummy::dummy_values(ty.params());
            let this_result = f.call(&params).map_err(|e| e.downcast::<Trap>().unwrap());

            let existing_result = export_func_results
                .entry(name.to_string())
                .or_insert_with(|| this_result.clone());
            assert_same_export_func_result(&existing_result, &this_result, name);
        }
    }

    fn init_hang_limit(instance: &Instance) {
        match instance.get_export("hangLimitInitializer") {
            None => return,
            Some(Extern::Func(f)) => {
                f.call(&[])
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
    let mut store: Option<Store> = None;
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
                store = Some(Store::new(engine.as_ref().unwrap()));
            }

            ApiCall::ModuleNew { id, wasm } => {
                log::debug!("creating module: {}", id);
                let wasm = wasm.to_bytes();
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

                let store = store.as_ref().unwrap();
                let linker = dummy_linker(store, module);

                // Don't unwrap this: there can be instantiation-/link-time errors that
                // aren't caught during validation or compilation. For example, an imported
                // table might not have room for an element segment that we want to
                // initialize into it.
                if let Ok(instance) = linker.instantiate(&module) {
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

                let funcs = instance
                    .exports()
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
                let ty = f.ty();
                let params = dummy::dummy_values(ty.params());
                let _ = f.call(&params);
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
    config.wasm_reference_types(false);
    config.wasm_bulk_memory(false);
    let store = Store::new(&Engine::new(&config).unwrap());
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

    let num_dropped = Rc::new(Cell::new(0));

    {
        let mut config = fuzz_config.to_wasmtime();
        config.wasm_reference_types(true);
        let engine = Engine::new(&config).unwrap();
        let store = Store::new(&engine);
        if fuzz_config.consume_fuel {
            store.add_fuel(u64::max_value()).unwrap();
        }

        let wasm = ops.to_wasm_binary();
        log_wasm(&wasm);
        let module = match Module::new(&engine, &wasm) {
            Ok(m) => m,
            Err(_) => return,
        };

        // To avoid timeouts, limit the number of explicit GCs we perform per
        // test case.
        const MAX_GCS: usize = 5;

        let num_gcs = Cell::new(0);
        let gc = Func::wrap(&store, move |caller: Caller| {
            if num_gcs.get() < MAX_GCS {
                caller.store().gc();
                num_gcs.set(num_gcs.get() + 1);
            }
        });

        let instance = Instance::new(&store, &module, &[gc.into()]).unwrap();
        let run = instance.get_func("run").unwrap();

        let args: Vec<_> = (0..ops.num_params())
            .map(|_| Val::ExternRef(Some(ExternRef::new(CountDrops(num_dropped.clone())))))
            .collect();
        let _ = run.call(&args);
    }

    assert_eq!(num_dropped.get(), ops.num_params());
    return;

    struct CountDrops(Rc<Cell<u8>>);

    impl Drop for CountDrops {
        fn drop(&mut self) {
            self.0.set(self.0.get().checked_add(1).unwrap());
        }
    }
}

/// Configuration options for wasm-smith such that generated modules always
/// conform to certain specifications.
#[derive(Default, Debug, Arbitrary, Clone)]
pub struct DifferentialWasmiModuleConfig;

impl wasm_smith::Config for DifferentialWasmiModuleConfig {
    fn allow_start_export(&self) -> bool {
        false
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

    fn max_memory_pages(&self) -> u32 {
        1
    }

    fn memory_max_size_required(&self) -> bool {
        true
    }
}

/// Perform differential execution between Cranelift and wasmi, diffing the
/// resulting memory image when execution terminates. This relies on the
/// module-under-test to be instrumented to bound the execution time. Invoke
/// with a module generated by `wasm-smith` using the
/// `DiferentialWasmiModuleConfig` configuration type for best results.
///
/// May return `None` if we early-out due to a rejected fuzz config; these
/// should be rare if modules are generated appropriately.
pub fn differential_wasmi_execution(wasm: &[u8], config: &crate::generators::Config) -> Option<()> {
    crate::init_fuzzing();

    // Instantiate wasmi module and instance.
    let wasmi_module = wasmi::Module::from_buffer(&wasm[..]).ok()?;
    let wasmi_instance =
        wasmi::ModuleInstance::new(&wasmi_module, &wasmi::ImportsBuilder::default()).ok()?;
    let wasmi_instance = wasmi_instance.assert_no_start();

    // TODO(paritytech/wasmi#19): wasmi does not currently canonicalize NaNs. To avoid spurious
    // fuzz failures, for now let's fuzz only integer Wasm programs.
    if wasmi_module.deny_floating_point().is_err() {
        return None;
    }

    // Instantiate wasmtime module and instance.
    let mut wasmtime_config = config.to_wasmtime();
    wasmtime_config.cranelift_nan_canonicalization(true);
    let wasmtime_engine = Engine::new(&wasmtime_config).unwrap();
    let wasmtime_store = Store::new(&wasmtime_engine);
    if config.consume_fuel {
        wasmtime_store.add_fuel(u64::max_value()).unwrap();
    }
    let wasmtime_module =
        Module::new(&wasmtime_engine, &wasm).expect("Wasmtime can compile module");
    let wasmtime_instance = Instance::new(&wasmtime_store, &wasmtime_module, &[])
        .expect("Wasmtime can instantiate module");

    // Introspect wasmtime module to find name of an exported function and of an
    // exported memory. Stop when we have one of each. (According to the config
    // above, there should be at most one of each.)
    let (func_name, memory_name) = {
        let mut func_name = None;
        let mut memory_name = None;
        for e in wasmtime_module.exports() {
            match e.ty() {
                wasmtime::ExternType::Func(..) => func_name = Some(e.name().to_string()),
                wasmtime::ExternType::Memory(..) => memory_name = Some(e.name().to_string()),
                _ => {}
            }
            if func_name.is_some() && memory_name.is_some() {
                break;
            }
        }
        (func_name?, memory_name?)
    };

    let wasmi_mem_export = wasmi_instance.export_by_name(&memory_name[..]).unwrap();
    let wasmi_mem = wasmi_mem_export.as_memory().unwrap();
    let wasmi_main_export = wasmi_instance.export_by_name(&func_name[..]).unwrap();
    let wasmi_main = wasmi_main_export.as_func().unwrap();
    let wasmi_val = wasmi::FuncInstance::invoke(&wasmi_main, &[], &mut wasmi::NopExternals);

    let wasmtime_mem = wasmtime_instance
        .get_memory(&memory_name[..])
        .expect("memory export is present");
    let wasmtime_main = wasmtime_instance
        .get_func(&func_name[..])
        .expect("function export is present");
    let wasmtime_vals = wasmtime_main.call(&[]);
    let wasmtime_val = wasmtime_vals.map(|v| v.iter().next().cloned());

    debug!(
        "Successful execution: wasmi returned {:?}, wasmtime returned {:?}",
        wasmi_val, wasmtime_val
    );

    let show_wat = || {
        if let Ok(s) = wasmprinter::print_bytes(&wasm[..]) {
            eprintln!("wat:\n{}\n", s);
        }
    };

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
            show_wat();
            panic!(
                "Values do not match: wasmi returned {:?}; wasmtime returned {:?}",
                wasmi_val, wasmtime_val
            );
        }
    }

    if wasmi_mem.current_size().0 != wasmtime_mem.size() as usize {
        show_wat();
        panic!("resulting memories are not the same size");
    }

    // Wasmi memory may be stored non-contiguously; copy it out to a contiguous chunk.
    let mut wasmi_buf: Vec<u8> = vec![0; wasmtime_mem.data_size()];
    wasmi_mem
        .get_into(0, &mut wasmi_buf[..])
        .expect("can access wasmi memory");

    let wasmtime_slice = unsafe { wasmtime_mem.data_unchecked() };

    if wasmi_buf.len() >= 64 {
        debug!("-> First 64 bytes of wasmi heap: {:?}", &wasmi_buf[0..64]);
        debug!(
            "-> First 64 bytes of Wasmtime heap: {:?}",
            &wasmtime_slice[0..64]
        );
    }

    if &wasmi_buf[..] != &wasmtime_slice[..] {
        show_wat();
        panic!("memory contents are not equal");
    }

    Some(())
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
