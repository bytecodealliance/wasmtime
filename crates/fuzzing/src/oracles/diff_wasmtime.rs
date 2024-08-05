//! Evaluate an exported Wasm function using Wasmtime.

use crate::generators::{self, CompilerStrategy, DiffValue, DiffValueType, WasmtimeConfig};
use crate::oracles::dummy;
use crate::oracles::engine::DiffInstance;
use crate::oracles::{compile_module, engine::DiffEngine, StoreLimits};
use crate::single_module_fuzzer::KnownValid;
use anyhow::{Context, Error, Result};
use arbitrary::Unstructured;
use wasmtime::{Extern, FuncType, Instance, Module, Store, Trap, Val};

/// A wrapper for using Wasmtime as a [`DiffEngine`].
pub struct WasmtimeEngine {
    config: generators::Config,
}

impl WasmtimeEngine {
    /// Merely store the configuration; the engine is actually constructed
    /// later. Ideally the store and engine could be built here but
    /// `compile_module` takes a [`generators::Config`]; TODO re-factor this if
    /// that ever changes.
    pub fn new(
        u: &mut Unstructured<'_>,
        config: &mut generators::Config,
        compiler_strategy: CompilerStrategy,
    ) -> arbitrary::Result<Self> {
        if let CompilerStrategy::Winch = compiler_strategy {
            config.disable_unimplemented_winch_proposals();
        }
        let mut new_config = u.arbitrary::<WasmtimeConfig>()?;
        new_config.compiler_strategy = compiler_strategy;
        new_config.make_compatible_with(&config.wasmtime);

        let config = generators::Config {
            wasmtime: new_config,
            module_config: config.module_config.clone(),
        };
        Ok(Self { config })
    }
}

impl DiffEngine for WasmtimeEngine {
    fn name(&self) -> &'static str {
        match self.config.wasmtime.compiler_strategy {
            CompilerStrategy::Cranelift => "wasmtime",
            CompilerStrategy::Winch => "winch",
        }
    }

    fn instantiate(&mut self, wasm: &[u8]) -> Result<Box<dyn DiffInstance>> {
        let store = self.config.to_store();
        let module = compile_module(store.engine(), wasm, KnownValid::Yes, &self.config).unwrap();
        let instance = WasmtimeInstance::new(store, module)?;
        Ok(Box::new(instance))
    }

    fn assert_error_match(&self, trap: &Trap, err: &Error) {
        let trap2 = err
            .downcast_ref::<Trap>()
            .expect(&format!("not a trap: {err:?}"));
        assert_eq!(trap, trap2, "{trap}\nis not equal to\n{trap2}");
    }

    fn is_stack_overflow(&self, err: &Error) -> bool {
        match err.downcast_ref::<Trap>() {
            Some(trap) => *trap == Trap::StackOverflow,
            None => false,
        }
    }
}

/// A wrapper around a Wasmtime instance.
///
/// The Wasmtime engine constructs a new store and compiles an instance of a
/// Wasm module.
pub struct WasmtimeInstance {
    store: Store<StoreLimits>,
    instance: Instance,
}

impl WasmtimeInstance {
    /// Instantiate a new Wasmtime instance.
    pub fn new(mut store: Store<StoreLimits>, module: Module) -> Result<Self> {
        let instance = dummy::dummy_linker(&mut store, &module)
            .and_then(|l| l.instantiate(&mut store, &module))
            .context("unable to instantiate module in wasmtime")?;
        Ok(Self { store, instance })
    }

    /// Retrieve the names and types of all exported functions in the instance.
    ///
    /// This is useful for evaluating each exported function with different
    /// values. The [`DiffInstance`] trait asks for the function name and we
    /// need to know the function signature in order to pass in the right
    /// arguments.
    pub fn exported_functions(&mut self) -> Vec<(String, FuncType)> {
        let exported_functions = self
            .instance
            .exports(&mut self.store)
            .map(|e| (e.name().to_owned(), e.into_func()))
            .filter_map(|(n, f)| f.map(|f| (n, f)))
            .collect::<Vec<_>>();
        exported_functions
            .into_iter()
            .map(|(n, f)| (n, f.ty(&self.store)))
            .collect()
    }

    /// Returns the list of globals and their types exported from this instance.
    pub fn exported_globals(&mut self) -> Vec<(String, DiffValueType)> {
        let globals = self
            .instance
            .exports(&mut self.store)
            .filter_map(|e| {
                let name = e.name();
                e.into_global().map(|g| (name.to_string(), g))
            })
            .collect::<Vec<_>>();

        globals
            .into_iter()
            .map(|(name, global)| {
                (
                    name,
                    global.ty(&self.store).content().clone().try_into().unwrap(),
                )
            })
            .collect()
    }

    /// Returns the list of exported memories and whether or not it's a shared
    /// memory.
    pub fn exported_memories(&mut self) -> Vec<(String, bool)> {
        self.instance
            .exports(&mut self.store)
            .filter_map(|e| {
                let name = e.name();
                match e.into_extern() {
                    Extern::Memory(_) => Some((name.to_string(), false)),
                    Extern::SharedMemory(_) => Some((name.to_string(), true)),
                    _ => None,
                }
            })
            .collect()
    }

    /// Returns whether or not this instance has hit its OOM condition yet.
    pub fn is_oom(&self) -> bool {
        self.store.data().is_oom()
    }
}

impl DiffInstance for WasmtimeInstance {
    fn name(&self) -> &'static str {
        "wasmtime"
    }

    fn evaluate(
        &mut self,
        function_name: &str,
        arguments: &[DiffValue],
        _results: &[DiffValueType],
    ) -> Result<Option<Vec<DiffValue>>> {
        let arguments: Vec<_> = arguments.iter().map(Val::from).collect();

        let function = self
            .instance
            .get_func(&mut self.store, function_name)
            .expect("unable to access exported function");
        let ty = function.ty(&self.store);
        let mut results = vec![Val::I32(0); ty.results().len()];
        function.call(&mut self.store, &arguments, &mut results)?;

        let results = results.into_iter().map(Val::into).collect();
        Ok(Some(results))
    }

    fn get_global(&mut self, name: &str, _ty: DiffValueType) -> Option<DiffValue> {
        Some(
            self.instance
                .get_global(&mut self.store, name)
                .unwrap()
                .get(&mut self.store)
                .into(),
        )
    }

    fn get_memory(&mut self, name: &str, shared: bool) -> Option<Vec<u8>> {
        Some(if shared {
            let memory = self
                .instance
                .get_shared_memory(&mut self.store, name)
                .unwrap();
            memory.data().iter().map(|i| unsafe { *i.get() }).collect()
        } else {
            self.instance
                .get_memory(&mut self.store, name)
                .unwrap()
                .data(&self.store)
                .to_vec()
        })
    }
}

impl From<&DiffValue> for Val {
    fn from(v: &DiffValue) -> Self {
        match *v {
            DiffValue::I32(n) => Val::I32(n),
            DiffValue::I64(n) => Val::I64(n),
            DiffValue::F32(n) => Val::F32(n),
            DiffValue::F64(n) => Val::F64(n),
            DiffValue::V128(n) => Val::V128(n.into()),
            DiffValue::FuncRef { null } => {
                assert!(null);
                Val::FuncRef(None)
            }
            DiffValue::ExternRef { null } => {
                assert!(null);
                Val::ExternRef(None)
            }
            DiffValue::AnyRef { null } => {
                assert!(null);
                Val::AnyRef(None)
            }
        }
    }
}

impl Into<DiffValue> for Val {
    fn into(self) -> DiffValue {
        match self {
            Val::I32(n) => DiffValue::I32(n),
            Val::I64(n) => DiffValue::I64(n),
            Val::F32(n) => DiffValue::F32(n),
            Val::F64(n) => DiffValue::F64(n),
            Val::V128(n) => DiffValue::V128(n.into()),
            Val::ExternRef(r) => DiffValue::ExternRef { null: r.is_none() },
            Val::FuncRef(r) => DiffValue::FuncRef { null: r.is_none() },
            Val::AnyRef(r) => DiffValue::AnyRef { null: r.is_none() },
        }
    }
}

#[test]
fn smoke_cranelift() {
    crate::oracles::engine::smoke_test_engine(|u, config| {
        WasmtimeEngine::new(u, config, CompilerStrategy::Cranelift)
    })
}

#[test]
fn smoke_winch() {
    if !cfg!(target_arch = "x86_64") {
        return;
    }
    crate::oracles::engine::smoke_test_engine(|u, config| {
        WasmtimeEngine::new(u, config, CompilerStrategy::Winch)
    })
}
