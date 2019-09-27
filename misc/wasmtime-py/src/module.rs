//! WebAssembly Module API object.

use pyo3::prelude::*;

use alloc::rc::Rc;

#[pyclass]
pub struct Module {
    pub module: Rc<wasmtime_environ::Module>,
}
