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

use dummy::{dummy_imports, dummy_value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasmtime::*;
use wasmtime_environ::{isa, settings};
use wasmtime_jit::{native, CompilationStrategy, CompiledModule, Compiler, NullResolver};

fn host_isa() -> Box<dyn isa::TargetIsa> {
    let flag_builder = settings::builder();
    let isa_builder = native::builder();
    isa_builder.finish(settings::Flags::new(flag_builder))
}

/// Instantiate the Wasm buffer, and implicitly fail if we have an unexpected
/// panic or segfault or anything else that can be detected "passively".
///
/// Performs initial validation, and returns early if the Wasm is invalid.
///
/// You can control which compiler is used via passing a `CompilationStrategy`.
pub fn instantiate(wasm: &[u8], strategy: Strategy) {
    if wasmparser::validate(wasm, None).is_err() {
        return;
    }

    let mut config = Config::new();
    config
        .strategy(strategy)
        .expect("failed to enable lightbeam");
    let engine = Engine::new(&config);
    let store = Store::new(&engine);

    let module = Module::new(&store, wasm).expect("Failed to compile a valid Wasm module!");

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
    let _result = Instance::new(&store, &module, &imports);
}

/// Compile the Wasm buffer, and implicitly fail if we have an unexpected
/// panic or segfault or anything else that can be detected "passively".
///
/// Performs initial validation, and returns early if the Wasm is invalid.
///
/// You can control which compiler is used via passing a `CompilationStrategy`.
pub fn compile(wasm: &[u8], compilation_strategy: CompilationStrategy) {
    if wasmparser::validate(wasm, None).is_err() {
        return;
    }

    let isa = host_isa();
    let mut compiler = Compiler::new(isa, compilation_strategy);
    let mut resolver = NullResolver {};
    let global_exports = Rc::new(RefCell::new(HashMap::new()));
    let _ = CompiledModule::new(&mut compiler, wasm, &mut resolver, global_exports, false);
}

/// Invoke the given API calls.
pub fn make_api_calls(api: crate::generators::api::ApiCalls) {
    use crate::generators::api::ApiCall;

    let mut config: Option<Config> = None;
    let mut engine: Option<Engine> = None;
    let mut store: Option<Store> = None;
    let mut modules: HashMap<usize, Module> = Default::default();
    let mut instances: HashMap<usize, HostRef<Instance>> = Default::default();

    for call in api.calls {
        match call {
            ApiCall::ConfigNew => {
                assert!(config.is_none());
                config = Some(Config::new());
            }

            ApiCall::ConfigDebugInfo(b) => {
                config.as_mut().unwrap().debug_info(b);
            }

            ApiCall::EngineNew => {
                assert!(engine.is_none());
                engine = Some(Engine::new(config.as_ref().unwrap()));
            }

            ApiCall::StoreNew => {
                assert!(store.is_none());
                store = Some(Store::new(engine.as_ref().unwrap()));
            }

            ApiCall::ModuleNew { id, wasm } => {
                let module = match Module::new(store.as_ref().unwrap(), &wasm.wasm) {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                let old = modules.insert(id, module);
                assert!(old.is_none());
            }

            ApiCall::ModuleDrop { id } => {
                drop(modules.remove(&id));
            }

            ApiCall::InstanceNew { id, module } => {
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
                if let Ok(instance) = Instance::new(store.as_ref().unwrap(), &module, &imports) {
                    instances.insert(id, HostRef::new(instance));
                }
            }

            ApiCall::InstanceDrop { id } => {
                drop(instances.remove(&id));
            }

            ApiCall::CallExportedFunc { instance, nth } => {
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

                let funcs = {
                    let instance = instance.borrow();
                    instance
                        .exports()
                        .iter()
                        .filter_map(|e| match e {
                            Extern::Func(f) => Some(f.clone()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                };

                if funcs.is_empty() {
                    continue;
                }

                let nth = nth % funcs.len();
                let f = funcs[nth].borrow();
                let ty = f.r#type();
                let params = match ty
                    .params()
                    .iter()
                    .map(|valty| dummy_value(valty))
                    .collect::<Result<Vec<_>, _>>()
                {
                    Ok(p) => p,
                    Err(_) => continue,
                };
                let _ = f.call(&params);
            }
        }
    }
}
