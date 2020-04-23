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
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use wasmtime::*;
use wasmtime_wast::WastContext;

fn log_wasm(wasm: &[u8]) {
    static CNT: AtomicUsize = AtomicUsize::new(0);
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

/// Instantiate the Wasm buffer, and implicitly fail if we have an unexpected
/// panic or segfault or anything else that can be detected "passively".
///
/// Performs initial validation, and returns early if the Wasm is invalid.
///
/// You can control which compiler is used via passing a `Strategy`.
pub fn instantiate(wasm: &[u8], strategy: Strategy) {
    instantiate_with_config(wasm, crate::fuzz_default_config(strategy).unwrap());
}

/// Instantiate the Wasm buffer, and implicitly fail if we have an unexpected
/// panic or segfault or anything else that can be detected "passively".
///
/// The engine will be configured using provided config.
///
/// See also `instantiate` functions.
pub fn instantiate_with_config(wasm: &[u8], config: Config) {
    crate::init_fuzzing();

    let engine = Engine::new(&config);
    let store = Store::new(&engine);

    log_wasm(wasm);
    let module = match Module::new(&store, wasm) {
        Ok(module) => module,
        Err(_) => return,
    };

    let imports = match dummy_imports(&store, module.imports()) {
        Ok(imps) => imps,
        Err(_) => {
            // There are some value types that we can't synthesize a
            // dummy value for (e.g. anyrefs) and for modules that
            // import things of these types we skip instantiation.
            return;
        }
    };

    // Don't unwrap this: there can be instantiation-/link-time errors that
    // aren't caught during validation or compilation. For example, an imported
    // table might not have room for an element segment that we want to
    // initialize into it.
    let _result = Instance::new(&module, &imports);
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
    let store = Store::new(&engine);
    log_wasm(wasm);
    let _ = Module::new(&store, wasm);
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

        let module = match Module::new(&store, &ttf.wasm) {
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
                // dummy value for (e.g. anyrefs) and for modules that
                // import things of these types we skip instantiation.
                eprintln!("Warning: failed to synthesize dummy imports: {}", e);
                continue;
            }
        };

        // Don't unwrap this: there can be instantiation-/link-time errors that
        // aren't caught during validation or compilation. For example, an imported
        // table might not have room for an element segment that we want to
        // initialize into it.
        let instance = match Instance::new(&module, &imports) {
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
                        (Val::AnyRef(_), Val::AnyRef(_)) | (Val::FuncRef(_), Val::FuncRef(_)) => {
                            continue
                        }
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
                let mut cfg = Config::new();
                cfg.cranelift_debug_verifier(true);
                config = Some(cfg);
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
                let module = match Module::new(store.as_ref().unwrap(), &wasm.wasm) {
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

                let imports = match dummy_imports(store.as_ref().unwrap(), module.imports()) {
                    Ok(imps) => imps,
                    Err(_) => {
                        // There are some value types that we can't synthesize a
                        // dummy value for (e.g. anyrefs) and for modules that
                        // import things of these types we skip instantiation.
                        continue;
                    }
                };

                // Don't unwrap this: there can be instantiation-/link-time errors that
                // aren't caught during validation or compilation. For example, an imported
                // table might not have room for an element segment that we want to
                // initialize into it.
                if let Ok(instance) = Instance::new(&module, &imports) {
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
    let store = Store::new(&Engine::new(&config.to_wasmtime()));
    let mut wast_context = WastContext::new(store);
    wast_context.register_spectest().unwrap();
    wast_context
        .run_buffer(test.file, test.contents.as_bytes())
        .unwrap();
}
