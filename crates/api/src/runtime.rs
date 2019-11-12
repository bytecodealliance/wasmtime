use crate::context::Context;
use crate::r#ref::HostRef;
use cranelift_codegen::{ir, settings};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasmtime_jit::{CompilationStrategy, Features};

// Runtime Environment

// Configuration

fn default_flags() -> settings::Flags {
    let flag_builder = settings::builder();
    settings::Flags::new(flag_builder)
}

/// Global configuration options used to create an [`Engine`] and customize its
/// behavior.
///
/// This structure exposed a builder-like interface and is primarily consumed by
/// [`Engine::new()`]
#[derive(Clone)]
pub struct Config {
    pub(crate) flags: settings::Flags,
    pub(crate) features: Features,
    pub(crate) debug_info: bool,
    pub(crate) strategy: CompilationStrategy,
}

impl Config {
    /// Creates a new configuration object with the default configuration
    /// specified.
    pub fn new() -> Config {
        Config {
            debug_info: false,
            features: Default::default(),
            flags: default_flags(),
            strategy: CompilationStrategy::Auto,
        }
    }

    /// Configures whether DWARF debug information will be emitted during
    /// compilation.
    ///
    /// By default this option is `false`.
    pub fn debug_info(&mut self, enable: bool) -> &mut Self {
        self.debug_info = enable;
        self
    }

    /// Configures various flags for compilation such as optimization level and
    /// such.
    ///
    /// For more information on defaults and configuration options, see the
    /// documentation for [`Flags`](settings::Flags)
    pub fn flags(&mut self, flags: settings::Flags) -> &mut Self {
        self.flags = flags;
        self
    }

    /// Indicates which WebAssembly features are enabled for this compilation
    /// session.
    ///
    /// By default only stable features are enabled by default (and none are
    /// fully stabilized yet at this time). If you're loading wasm modules
    /// which may use non-MVP features you'll want to be sure to call this
    /// method and enable the appropriate feature in the [`Features`]
    /// structure.
    pub fn features(&mut self, features: Features) -> &mut Self {
        self.features = features;
        self
    }

    /// Configures the compilation `strategy` provided, indicating which
    /// backend will be used for compiling WebAssembly to native code.
    ///
    /// Currently the primary strategies are with cranelift (an optimizing
    /// compiler) or lightbeam (a fast single-pass JIT which produces code
    /// quickly).
    pub fn strategy(&mut self, strategy: CompilationStrategy) -> &mut Self {
        self.strategy = strategy;
        self
    }
}

impl Default for Config {
    fn default() -> Config {
        Config::new()
    }
}

// Engine

#[derive(Default)]
pub struct Engine {
    config: Config,
}

impl Engine {
    pub fn new(config: &Config) -> Engine {
        Engine {
            config: config.clone(),
        }
    }
}

// Store

pub struct Store {
    engine: HostRef<Engine>,
    context: Context,
    global_exports: Rc<RefCell<HashMap<String, Option<wasmtime_runtime::Export>>>>,
    signature_cache: HashMap<wasmtime_runtime::VMSharedSignatureIndex, ir::Signature>,
}

impl Store {
    pub fn new(engine: &HostRef<Engine>) -> Store {
        Store {
            engine: engine.clone(),
            context: Context::new(&engine.borrow().config),
            global_exports: Rc::new(RefCell::new(HashMap::new())),
            signature_cache: HashMap::new(),
        }
    }

    pub fn engine(&self) -> &HostRef<Engine> {
        &self.engine
    }

    pub(crate) fn context(&mut self) -> &mut Context {
        &mut self.context
    }

    // Specific to wasmtime: hack to pass memory around to wasi
    pub fn global_exports(
        &self,
    ) -> &Rc<RefCell<HashMap<String, Option<wasmtime_runtime::Export>>>> {
        &self.global_exports
    }

    pub(crate) fn register_cranelift_signature(
        &mut self,
        signature: &ir::Signature,
    ) -> wasmtime_runtime::VMSharedSignatureIndex {
        use std::collections::hash_map::Entry;
        let index = self.context().compiler().signatures().register(signature);
        match self.signature_cache.entry(index) {
            Entry::Vacant(v) => {
                v.insert(signature.clone());
            }
            Entry::Occupied(_) => (),
        }
        index
    }

    pub(crate) fn lookup_cranelift_signature(
        &self,
        type_index: wasmtime_runtime::VMSharedSignatureIndex,
    ) -> Option<&ir::Signature> {
        self.signature_cache.get(&type_index)
    }
}
