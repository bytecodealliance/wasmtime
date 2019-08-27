use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::context::{create_compiler, Context};

use cranelift_codegen::{ir, settings};
use wasmtime_jit::Features;

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
}

impl Config {
    pub fn default() -> Config {
        Config {
            debug_info: false,
            features: Default::default(),
            flags: default_flags(),
        }
    }

    pub fn new(flags: settings::Flags, features: Features, debug_info: bool) -> Config {
        Config {
            flags,
            features,
            debug_info,
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

    pub fn create_wasmtime_context(&self) -> wasmtime_jit::Context {
        let flags = self.config.flags().clone();
        wasmtime_jit::Context::new(Box::new(create_compiler(flags)))
    }
}

// Store

pub(crate) trait SignatureRegistry {
    fn register_cranelift_signature(
        &mut self,
        signature: &ir::Signature,
    ) -> wasmtime_runtime::VMSharedSignatureIndex;
    fn lookup_cranelift_signature(
        &self,
        type_index: wasmtime_runtime::VMSharedSignatureIndex,
    ) -> Option<&ir::Signature>;
}

pub struct Store {
    engine: Rc<RefCell<Engine>>,
    context: Context,
    global_exports: Rc<RefCell<HashMap<String, Option<wasmtime_runtime::Export>>>>,
    signature_cache: HashMap<wasmtime_runtime::VMSharedSignatureIndex, ir::Signature>,
}

impl Store {
    pub fn new(engine: Rc<RefCell<Engine>>) -> Store {
        let flags = engine.borrow().config().flags().clone();
        let features = engine.borrow().config().features().clone();
        let debug_info = engine.borrow().config().debug_info();
        Store {
            engine,
            context: Context::create(flags, features, debug_info),
            global_exports: Rc::new(RefCell::new(HashMap::new())),
            signature_cache: HashMap::new(),
        }
    }

    pub fn engine(&self) -> &Rc<RefCell<Engine>> {
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
}

impl SignatureRegistry for Store {
    fn register_cranelift_signature(
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
    fn lookup_cranelift_signature(
        &self,
        type_index: wasmtime_runtime::VMSharedSignatureIndex,
    ) -> Option<&ir::Signature> {
        self.signature_cache.get(&type_index)
    }
}
