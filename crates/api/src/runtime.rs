use crate::context::Context;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use wasmtime_environ::{ir, settings};
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

    /// Configures whether the WebAssembly threads proposal will be enabled for
    /// compilation.
    ///
    /// The [WebAssembly threads proposal][threads] is not currently fully
    /// standardized and is undergoing development. Additionally the support in
    /// wasmtime itself is still being worked on. Support for this feature can
    /// be enabled through this method for appropriate wasm modules.
    ///
    /// This feature gates items such as shared memories and atomic
    /// instructions.
    ///
    /// This is `false` by default.
    ///
    /// [threads]: https://github.com/webassembly/threads
    pub fn wasm_threads(&mut self, enable: bool) -> &mut Self {
        self.features.threads = enable;
        self
    }

    /// Configures whether the WebAssembly reference types proposal will be
    /// enabled for compilation.
    ///
    /// The [WebAssembly reference types proposal][proposal] is not currently
    /// fully standardized and is undergoing development. Additionally the
    /// support in wasmtime itself is still being worked on. Support for this
    /// feature can be enabled through this method for appropriate wasm
    /// modules.
    ///
    /// This feature gates items such as the `anyref` type and multiple tables
    /// being in a module.
    ///
    /// This is `false` by default.
    ///
    /// [proposal]: https://github.com/webassembly/reference-types
    pub fn wasm_reference_types(&mut self, enable: bool) -> &mut Self {
        self.features.reference_types = enable;
        self
    }

    /// Configures whether the WebAssembly SIMD proposal will be
    /// enabled for compilation.
    ///
    /// The [WebAssembly SIMD proposal][proposal] is not currently
    /// fully standardized and is undergoing development. Additionally the
    /// support in wasmtime itself is still being worked on. Support for this
    /// feature can be enabled through this method for appropriate wasm
    /// modules.
    ///
    /// This feature gates items such as the `v128` type and all of its
    /// operators being in a module.
    ///
    /// This is `false` by default.
    ///
    /// [proposal]: https://github.com/webassembly/simd
    pub fn wasm_simd(&mut self, enable: bool) -> &mut Self {
        self.features.simd = enable;
        self
    }

    /// Configures whether the WebAssembly bulk memory operations proposal will
    /// be enabled for compilation.
    ///
    /// The [WebAssembly bulk memory operations proposal][proposal] is not
    /// currently fully standardized and is undergoing development.
    /// Additionally the support in wasmtime itself is still being worked on.
    /// Support for this feature can be enabled through this method for
    /// appropriate wasm modules.
    ///
    /// This feature gates items such as the `memory.copy` instruction, passive
    /// data/table segments, etc, being in a module.
    ///
    /// This is `false` by default.
    ///
    /// [proposal]: https://github.com/webassembly/bulk-memory-operations
    pub fn wasm_bulk_memory(&mut self, enable: bool) -> &mut Self {
        self.features.bulk_memory = enable;
        self
    }

    /// Configures whether the WebAssembly multi-value proposal will
    /// be enabled for compilation.
    ///
    /// The [WebAssembly multi-value proposal][proposal] is not
    /// currently fully standardized and is undergoing development.
    /// Additionally the support in wasmtime itself is still being worked on.
    /// Support for this feature can be enabled through this method for
    /// appropriate wasm modules.
    ///
    /// This feature gates functions and blocks returning multiple values in a
    /// module, for example.
    ///
    /// This is `false` by default.
    ///
    /// [proposal]: https://github.com/webassembly/multi-value
    pub fn wasm_multi_value(&mut self, enable: bool) -> &mut Self {
        self.features.multi_value = enable;
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

/// An `Engine` which is a global context for compilation and management of wasm
/// modules.
///
/// An engine can be safely shared across threads and is a cheap cloneable
/// handle to the actual engine. The engine itself will be deallocate once all
/// references to it have gone away.
///
/// Engines store global configuration preferences such as compilation settings,
/// enabled features, etc. You'll likely only need at most one of these for a
/// program.
///
/// ## Engines and `Clone`
///
/// Using `clone` on an `Engine` is a cheap operation. It will not create an
/// entirely new engine, but rather just a new reference to the existing engine.
///
/// ## Engines and `Default`
///
/// You can create an engine with default settings using `Engine::default()`.
/// This engine will not have any unstable wasm features enabled and will use
/// the default compilation backend configured at this crate's compile time.
#[derive(Default, Clone)]
pub struct Engine {
    pub(crate) config: Arc<Config>,
}

impl Engine {
    /// Creates a new [`Engine`] with the specified compilation and
    /// configuration settings.
    pub fn new(config: &Config) -> Engine {
        Engine {
            config: Arc::new(config.clone()),
        }
    }
}

// Store

pub struct Store {
    engine: Engine,
    context: Context,
    global_exports: Rc<RefCell<HashMap<String, Option<wasmtime_runtime::Export>>>>,
    signature_cache: HashMap<wasmtime_runtime::VMSharedSignatureIndex, ir::Signature>,
}

impl Store {
    pub fn new(engine: &Engine) -> Store {
        Store {
            engine: engine.clone(),
            context: Context::new(&engine.config),
            global_exports: Rc::new(RefCell::new(HashMap::new())),
            signature_cache: HashMap::new(),
        }
    }

    pub fn engine(&self) -> &Engine {
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

    pub(crate) fn register_wasmtime_signature(
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

    pub(crate) fn lookup_wasmtime_signature(
        &self,
        type_index: wasmtime_runtime::VMSharedSignatureIndex,
    ) -> Option<&ir::Signature> {
        self.signature_cache.get(&type_index)
    }
}

fn _assert_send_sync() {
    fn _assert<T: Send + Sync>() {}
    _assert::<Engine>();
}
