//! WebAssembly Module API object.

extern crate alloc;

use alloc::rc::Rc;
use pyo3::prelude::*;

#[pyclass]
pub struct Module {
    pub module: Rc<wasmtime_environ::Module>,
}
