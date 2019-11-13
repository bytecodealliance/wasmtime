use crate::Config;
use alloc::rc::Rc;
use core::cell::{RefCell, RefMut};
use core::hash::{Hash, Hasher};
use wasmtime_jit::{Compiler, Features};

#[derive(Clone)]
pub struct Context {
    compiler: Rc<RefCell<Compiler>>,
    features: Features,
    debug_info: bool,
}

impl Context {
    pub fn new(config: &Config) -> Context {
        let isa_builder =
            cranelift_native::builder().expect("host machine is not a supported target");
        let isa = isa_builder.finish(config.flags.clone());
        Context::new_with_compiler(config, Compiler::new(isa, config.strategy))
    }

    pub fn new_with_compiler(config: &Config, compiler: Compiler) -> Context {
        Context {
            compiler: Rc::new(RefCell::new(compiler)),
            features: config.features.clone(),
            debug_info: config.debug_info,
        }
    }

    pub(crate) fn debug_info(&self) -> bool {
        self.debug_info
    }

    pub(crate) fn compiler(&mut self) -> RefMut<Compiler> {
        self.compiler.borrow_mut()
    }
}

impl Hash for Context {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.compiler.as_ptr().hash(state)
    }
}

impl Eq for Context {}

impl PartialEq for Context {
    fn eq(&self, other: &Context) -> bool {
        Rc::ptr_eq(&self.compiler, &other.compiler)
    }
}
