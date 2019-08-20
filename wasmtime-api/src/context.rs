use std::cell::{RefCell, RefMut};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use wasmtime_jit::{Compiler, Features};

use cranelift_codegen::settings;

#[derive(Clone)]
pub struct Context {
    compiler: Rc<RefCell<Compiler>>,
    features: Features,
    debug_info: bool,
}

impl Context {
    pub fn new(compiler: Compiler, features: Features, debug_info: bool) -> Context {
        Context {
            compiler: Rc::new(RefCell::new(compiler)),
            features,
            debug_info,
        }
    }

    pub fn create(flags: settings::Flags, features: Features, debug_info: bool) -> Context {
        Context::new(create_compiler(flags), features, debug_info)
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

pub(crate) fn create_compiler(flags: settings::Flags) -> Compiler {
    let isa = {
        let isa_builder =
            cranelift_native::builder().expect("host machine is not a supported target");
        isa_builder.finish(flags)
    };

    Compiler::new(isa)
}
