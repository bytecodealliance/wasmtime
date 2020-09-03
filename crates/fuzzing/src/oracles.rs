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

use dummy::dummy_imports;
use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use std::time::Duration;
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
    if let Ok(s) = wasmprinter::print_bytes(wasm) {
        let name = format!("testcase{}.wat", i);
        std::fs::write(&name, s).expect("failed to write wat file");
    }
}

fn log_wat(wat: &str) {
    if !log::log_enabled!(log::Level::Debug) {
        return;
    }

    let i = CNT.fetch_add(1, SeqCst);
    let name = format!("testcase{}.wat", i);
    std::fs::write(&name, wat).expect("failed to write wat file");
}

/// Instantiate the Wasm buffer, and implicitly fail if we have an unexpected
/// panic or segfault or anything else that can be detected "passively".
///
/// Performs initial validation, and returns early if the Wasm is invalid.
///
/// You can control which compiler is used via passing a `Strategy`.
pub fn instantiate(wasm: &[u8], strategy: Strategy) {
    instantiate_with_config(wasm, crate::fuzz_default_config(strategy).unwrap(), None);
}

/// Instantiate the Wasm buffer, and implicitly fail if we have an unexpected
/// panic or segfault or anything else that can be detected "passively".
///
/// The engine will be configured using provided config.
///
/// See also `instantiate` functions.
pub fn instantiate_with_config(wasm: &[u8], mut config: Config, timeout: Option<Duration>) {
    crate::init_fuzzing();

    let engine = Engine::new(&config);
    let store = Store::new(&engine);

    if let Some(timeout) = timeout {
        config.interruptable(true);
        let handle = store.interrupt_handle().unwrap();
        std::thread::spawn(move || {
            std::thread::sleep(timeout);
            handle.interrupt();
        });
    }

    log_wasm(wasm);
    let module = match Module::new(&engine, wasm) {
        Ok(module) => module,
        Err(_) => return,
    };

    let imports = match dummy_imports(&store, module.imports()) {
        Ok(imps) => imps,
        Err(_) => {
            // There are some value types that we can't synthesize a
            // dummy value for (e.g. externrefs) and for modules that
            // import things of these types we skip instantiation.
            return;
        }
    };

    // Don't unwrap this: there can be instantiation-/link-time errors that
    // aren't caught during validation or compilation. For example, an imported
    // table might not have room for an element segment that we want to
    // initialize into it.
    let _result = Instance::new(&store, &module, &imports);
}

/// Compile the Wasm buffer, and implicitly fail if we have an unexpected
/// panic or segfault or anything else that can be detected "passively".
///
/// Performs initial validation, and returns early if the Wasm is invalid.
///
/// You can control which compiler is used via passing a `Strategy`.
pub fn compile(wasm: &[u8], strategy: Strategy) {
    crate::init_fuzzing();

    let engine = Engine::new(&crate::fuzz_default_config(strategy).unwrap());
    log_wasm(wasm);
    let _ = Module::new(&engine, wasm);
}

/// Instantiate the given Wasm module with each `Config` and call all of its
/// exports. Modulo OOM, non-canonical NaNs, and usage of Wasm features that are
/// or aren't enabled for different configs, we should get the same results when
/// we call the exported functions for all of our different configs.
#[cfg(feature = "binaryen")]
pub fn differential_execution(
    ttf: &crate::generators::WasmOptTtf,
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
    log_wasm(&ttf.wasm);

    for config in &configs {
        let engine = Engine::new(config);
        let store = Store::new(&engine);

        let module = match Module::new(&engine, &ttf.wasm) {
            Ok(module) => module,
            // The module might rely on some feature that our config didn't
            // enable or something like that.
            Err(e) => {
                eprintln!("Warning: failed to compile `wasm-opt -ttf` module: {}", e);
                continue;
            }
        };

        // TODO: we should implement tracing versions of these dummy imports
        // that record a trace of the order that imported functions were called
        // in and with what values. Like the results of exported functions,
        // calls to imports should also yield the same values for each
        // configuration, and we should assert that.
        let imports = match dummy_imports(&store, module.imports()) {
            Ok(imps) => imps,
            Err(e) => {
                // There are some value types that we can't synthesize a
                // dummy value for (e.g. externrefs) and for modules that
                // import things of these types we skip instantiation.
                eprintln!("Warning: failed to synthesize dummy imports: {}", e);
                continue;
            }
        };

        // Don't unwrap this: there can be instantiation-/link-time errors that
        // aren't caught during validation or compilation. For example, an imported
        // table might not have room for an element segment that we want to
        // initialize into it.
        let instance = match Instance::new(&store, &module, &imports) {
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
            let params = match dummy::dummy_values(ty.params()) {
                Ok(p) => p,
                Err(_) => continue,
            };
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
                        (Val::F32(lhs), Val::F32(rhs)) => {
                            let lhs = f32::from_bits(*lhs);
                            let rhs = f32::from_bits(*rhs);
                            if lhs == rhs || (lhs.is_nan() && rhs.is_nan()) {
                                continue;
                            } else {
                                fail()
                            }
                        }
                        (Val::F64(lhs), Val::F64(rhs)) => {
                            let lhs = f64::from_bits(*lhs);
                            let rhs = f64::from_bits(*rhs);
                            if lhs == rhs || (lhs.is_nan() && rhs.is_nan()) {
                                continue;
                            } else {
                                fail()
                            }
                        }
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

/// Invoke the given API calls.
#[cfg(feature = "binaryen")]
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
                engine = Some(Engine::new(config.as_ref().unwrap()));
            }

            ApiCall::StoreNew => {
                log::trace!("creating store");
                assert!(store.is_none());
                store = Some(Store::new(engine.as_ref().unwrap()));
            }

            ApiCall::ModuleNew { id, wasm } => {
                log::debug!("creating module: {}", id);
                log_wasm(&wasm.wasm);
                let module = match Module::new(engine.as_ref().unwrap(), &wasm.wasm) {
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

                let imports = match dummy_imports(store, module.imports()) {
                    Ok(imps) => imps,
                    Err(_) => {
                        // There are some value types that we can't synthesize a
                        // dummy value for (e.g. externrefs) and for modules that
                        // import things of these types we skip instantiation.
                        continue;
                    }
                };

                // Don't unwrap this: there can be instantiation-/link-time errors that
                // aren't caught during validation or compilation. For example, an imported
                // table might not have room for an element segment that we want to
                // initialize into it.
                if let Ok(instance) = Instance::new(store, &module, &imports) {
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
                let params = match dummy::dummy_values(ty.params()) {
                    Ok(p) => p,
                    Err(_) => continue,
                };
                let _ = f.call(&params);
            }
        }
    }
}

/// Executes the wast `test` spectest with the `config` specified.
///
/// Ensures that spec tests pass regardless of the `Config`.
pub fn spectest(config: crate::generators::Config, test: crate::generators::SpecTest) {
    crate::init_fuzzing();
    log::debug!("running {:?} with {:?}", test.file, config);
    let mut config = config.to_wasmtime();
    config.wasm_reference_types(false);
    config.wasm_bulk_memory(false);
    let store = Store::new(&Engine::new(&config));
    let mut wast_context = WastContext::new(store);
    wast_context.register_spectest().unwrap();
    wast_context
        .run_buffer(test.file, test.contents.as_bytes())
        .unwrap();
}

/// Execute a series of `table.get` and `table.set` operations.
pub fn table_ops(config: crate::generators::Config, ops: crate::generators::table_ops::TableOps) {
    let _ = env_logger::try_init();

    let num_dropped = Rc::new(Cell::new(0));

    {
        let mut config = config.to_wasmtime();
        config.wasm_reference_types(true);
        let engine = Engine::new(&config);
        let store = Store::new(&engine);

        let wat = ops.to_wat_string();
        log_wat(&wat);
        let module = match Module::new(&engine, &wat) {
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
