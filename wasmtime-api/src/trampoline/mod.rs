//! Utility module to create trampolines in/out WebAssembly module.

mod code_memory;
mod create_handle;

use failure::Error;
use std::cell::RefCell;
use std::rc::Rc;

use self::create_handle::create_handle;
use super::externals::Func;

pub fn generate_func_export(f: &Rc<RefCell<Func>>) -> Result<(), Error> {
    let mut instance = create_handle(f)?;
    let export = instance.lookup("trampoline").expect("trampoline export");

    f.borrow_mut().anchor = Some((instance, export));
    Ok(())
}
