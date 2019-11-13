use crate::context::Context;
use crate::r#ref::HostRef;
use crate::HashMap;
use alloc::{rc::Rc, string::String};
use core::cell::RefCell;
use cranelift_codegen::{ir, settings};
use wasmtime_jit::{CompilationStrategy, Features};

// Runtime Environment

// Configuration

fn default_flags() -> settings::Flags {
    let flag_builder = settings::builder();
    settings::Flags::new(flag_builder)
}

pub struct Config {
    flags: settings::Flags,
    features: Features,
    debug_info: bool,
    strategy: CompilationStrategy,
}

impl Config {
    pub fn default() -> Config {
        Config {
            debug_info: false,
            features: Default::default(),
            flags: default_flags(),
            strategy: CompilationStrategy::Auto,
        }
    }

    pub fn new(
        flags: settings::Flags,
        features: Features,
        debug_info: bool,
        strategy: CompilationStrategy,
    ) -> Config {
        Config {
            flags,
            features,
            debug_info,
            strategy,
        }
    }

    pub(crate) fn debug_info(&self) -> bool {
        self.debug_info
    }

    pub(crate) fn flags(&self) -> &settings::Flags {
        &self.flags
    }

    pub(crate) fn features(&self) -> &Features {
        &self.features
    }

    pub(crate) fn strategy(&self) -> CompilationStrategy {
        self.strategy
    }
}

// Engine

pub struct Engine {
    config: Config,
}

impl Engine {
    pub fn new(config: Config) -> Engine {
        Engine { config }
    }

    pub fn default() -> Engine {
        Engine::new(Config::default())
    }

    pub(crate) fn config(&self) -> &Config {
        &self.config
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
        let flags = engine.borrow().config().flags().clone();
        let features = engine.borrow().config().features().clone();
        let debug_info = engine.borrow().config().debug_info();
        let strategy = engine.borrow().config().strategy();
        Store {
            engine: engine.clone(),
            context: Context::create(flags, features, debug_info, strategy),
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
        use crate::hash_map::Entry;
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
